"""Owned daemon/HTTP fixtures. No managed storage or ambient AgentX access."""

import hashlib
import http.client
import ipaddress
import json
import os
from pathlib import Path
import select
import shutil
import signal
import socket
import struct
import subprocess
import tempfile
import time
from urllib.parse import urlencode, urlsplit

from provider import process_identity


def wait_for(description, observe, timeout=25):
    deadline = time.monotonic() + timeout
    while True:
        result = observe()
        if result:
            return result
        if time.monotonic() >= deadline:
            raise TimeoutError(description)
        select.select([], [], [], 0.1)


def terminate_exact(record):
    """Only signal a recorded Linux process incarnation using its pinned pidfd."""
    try:
        descriptor = os.pidfd_open(record["pid"])
    except ProcessLookupError:
        return {**record, "already_exited": True}
    try:
        current = process_identity(record["pid"])
        if any(current[key] != record[key] for key in ["pid", "start_ticks", "executable"]):
            return {**record, "incarnation_gone": True}
        signal.pidfd_send_signal(descriptor, signal.SIGTERM)
        poller = select.poll()
        poller.register(descriptor, select.POLLIN)
        if not poller.poll(5000):
            signal.pidfd_send_signal(descriptor, signal.SIGKILL)
            if not poller.poll(3000):
                raise TimeoutError(f"owned fixture process {record['pid']} did not exit")
        return {**record, "terminated": True}
    except FileNotFoundError:
        return {**record, "already_exited": True}
    finally:
        os.close(descriptor)


class AdoptedProcess:
    """Track a daemon autostarted by our CLI through its verified kernel peer."""

    def __init__(self, record):
        self.record, self.pid = record, record["pid"]

    def poll(self):
        try:
            current = process_identity(self.pid)
            return None if all(current[key] == self.record[key] for key in ["pid", "start_ticks", "executable"]) else 0
        except (OSError, ValueError):
            return 0

    def wait(self, timeout):
        wait_for("autostarted owned daemon exit", lambda: self.poll() is not None, timeout)


class Http:
    def __init__(self, url, evidence, prefix, expected_host=None):
        self.origin = urlsplit(url)
        host = ipaddress.ip_address(self.origin.hostname)
        assert self.origin.scheme == "http" and (host.is_loopback if expected_host is None else host == expected_host), url
        assert self.origin.port and self.origin.port > 1024, url
        self.url = f"http://{self.origin.hostname}:{self.origin.port}"
        self.evidence, self.prefix, self.sequence = evidence, prefix, 0
        self.generation = None

    def call(self, method, path, payload=None, query=None, raw=None, headers=None, disconnect=False):
        if query:
            path += "?" + urlencode({key: value for key, value in query.items() if value is not None})
        body = raw if raw is not None else json.dumps(payload, ensure_ascii=False).encode() if payload is not None else None
        if isinstance(body, str):
            body = body.encode()
        request_headers = {"Content-Type": "application/json", **(headers or {})}
        connection = http.client.HTTPConnection(self.origin.hostname, self.origin.port, timeout=15)
        self.sequence += 1
        sequence = self.sequence
        record = {"method": method, "path": path, "headers": request_headers,
                  "body": body.decode(errors="replace") if body else None}
        try:
            connection.request(method, path, body=body, headers=request_headers)
            if disconnect:
                # Admit the request before discarding its body. Closing as soon
                # as send() returns may cancel it before the server even reads
                # headers, which cannot establish lost-admitted-reply recovery.
                response = connection.getresponse()
                assert response.status == 202, response.status
                record["response_intentionally_lost"] = True
                record["transport_status_before_drop"] = response.status
                return None, None
            response = connection.getresponse()
            data = response.read(2 * 1024 * 1024 + 1)
            assert len(data) <= 2 * 1024 * 1024, "unbounded HTTP response"
            record.update(status=response.status, response_headers=dict(response.getheaders()))
            try:
                parsed = json.loads(data)
            except (ValueError, UnicodeError):
                parsed = data.decode(errors="replace")
            record["response"] = parsed
            return response.status, parsed
        finally:
            connection.close()
            (self.evidence / f"{self.prefix}-http-{sequence:04d}.json").write_text(json.dumps(record, indent=2) + "\n")

    def get(self, path, **query):
        status, response = self.call("GET", path, query={"generation": self.generation, **query})
        assert status == 200, (path, status, response)
        return response

    def post(self, path, payload):
        status, response = self.call("POST", path, payload)
        assert status == 202, (path, status, response)
        assert response["request_id"] == payload["request_id"] and response["generation"] == payload["generation"]
        return response

    def operation(self, request_id, success=True):
        result = wait_for("operation completion " + request_id, lambda: self._terminal(request_id))
        if success:
            assert result["status"] == "Succeeded", result
        return result

    def _terminal(self, request_id):
        result = self.get("/api/v1/operations/" + request_id)
        return result if result["status"] in ["Succeeded", "Failed", "Unknown"] else None

    def agents(self, **query):
        return self.get("/api/v1/agents", **query)["agents"]


class Fixture:
    @classmethod
    def reopen_owned(cls, candidate, evidence, previous):
        """Explicitly continue a cleaned acceptance scope from its retained receipt."""
        manifest = json.loads((Path(previous) / "manifest.json").read_text())
        assert manifest.get("cleanup_errors") == [] and manifest.get("candidate_unchanged") is True
        owner = cls.__new__(cls)
        owner.candidate = Path(candidate).resolve(strict=True)
        assert str(owner.candidate) == manifest["candidate"]
        owner.evidence = Path(evidence).absolute()
        owner.evidence.mkdir(parents=True, exist_ok=False)
        owner.root = Path(manifest["fixture_root"]).resolve(strict=True)
        assert owner.root.name.startswith("agentx-management-")
        owner.processes, owner.clients, owner.servers, owner.connections = [], {}, {}, 0
        owner.manifest = {**manifest, "processes": [], "cleanup": [], "resumed_from": str(Path(previous).absolute())}
        owner.persist()
        return owner

    def __init__(self, candidate, evidence, root=None):
        self.candidate = Path(candidate).resolve(strict=True)
        assert self.candidate.is_file() and os.access(self.candidate, os.X_OK)
        self.evidence = Path(evidence).absolute()
        self.evidence.mkdir(parents=True, exist_ok=False)
        if root is None:
            self.root = Path(tempfile.mkdtemp(prefix="agentx-management-"))
        else:
            # A consolidated consumer owns one root inside its retained workspace.
            # Never adopt or overwrite an existing service/root implicitly.
            self.root = Path(root).absolute()
            if not self.root.name.startswith("agentx-management-"):
                raise ValueError("explicit fixture root must retain the private management prefix")
            self.root.mkdir(mode=0o700)
        self.processes, self.clients, self.servers = [], {}, {}
        self.connections = 0
        for name in ["bin", "providers", "tmp"]:
            (self.root / name).mkdir()
        shutil.copyfile(Path(__file__).with_name("provider.py"), self.root / "bin/qoder")
        (self.root / "bin/qoder").chmod(0o755)
        (self.root / "bin/qodercli").symlink_to(self.root / "bin/qoder")
        (self.root / "bin/agentx").symlink_to(self.candidate)
        # A wrong Provider choice fails locally instead of reaching an installed model.
        for name in ["claude", "codex", "gemini", "cursor-agent", "copilot", "pi", "omp", "opencode", "kimi", "grok", "hermes", "kilo"]:
            path = self.root / "bin" / name
            path.write_text("#!/bin/sh\necho 'Only the local Qoder acceptance fixture is allowed' >&2\nexit 96\n")
            path.chmod(0o755)
        self.manifest = {"candidate": str(self.candidate), "sha256": hashlib.sha256(self.candidate.read_bytes()).hexdigest(),
                         "fixture_root": str(self.root), "fixture_kind": "local Qoder hooks; no paid model", "processes": [], "cleanup": []}
        self.persist()

    def persist(self):
        (self.evidence / "manifest.json").write_text(json.dumps(self.manifest, indent=2) + "\n")

    def environment(self, name):
        home = self.root / name / "home"
        home.mkdir(parents=True, exist_ok=True)
        env = {"HOME": str(home), "PATH": str(self.root / "bin") + ":/usr/bin:/bin", "SHELL": "/bin/sh",
               "LANG": "C.UTF-8", "TERM": "xterm-256color", "NO_COLOR": "1", "CI": "1",
               "TMPDIR": str(self.root / "tmp"), "AGENTX_DIR": str(self.root / name / "scope"),
               "AGENTX_MANAGEMENT_FIXTURE": str(self.root), "AGENTX_MANAGEMENT_CANDIDATE": str(self.candidate),
               "AGENTX_TEST_ROOT": str(self.root), "AGENTX_UPDATE_VERIFY": "1", "AGENTX_TEST_DB_NOSYNC": "1"}
        for field in ["CONFIG", "CACHE", "DATA", "STATE"]:
            env[f"XDG_{field}_HOME"] = str(home / field.lower())
        return env

    def prepare(self, name, via_cli=False):
        self.configure(name, enabled=not via_cli)
        project = self.root / name / "project"
        if via_cli:
            return self.web_cli(name, "enable", project, "127.0.0.1", 0)
        return self.start(name)

    def configure(self, name, enabled=True):
        """Prepare this owned scope without acquiring a daemon (passive observer setup)."""
        env = self.environment(name)
        scope = Path(env["AGENTX_DIR"])
        project = scope.parent / "project"
        project.mkdir(parents=True)
        scope.mkdir(parents=True, exist_ok=True)
        scope.chmod(0o700)
        (scope / ".tmp/flags").mkdir(parents=True)
        (scope / ".tmp/flags/update_check").touch()
        flag = "true" if enabled else "false"
        config = ('[management]\nenabled = ' + flag + '\nhost = "127.0.0.1"\nport = 0\nproject = ' + json.dumps(str(project)) + '\n'
                  '[launch.profiles.management-fixture]\ntool = "qoder"\nargs = []\n')
        (scope / "config.toml").write_text(config)
        (scope / "env").write_text("\n".join(key + "=" + json.dumps(value) for key, value in env.items()) + "\n")
        return env

    def web_cli(self, name, label, project, host, port):
        scope = self.root / name / "scope"
        try:
            result = self.cli(name, "web-" + label, ["daemon", "web", "--scope", str(scope), "--project", str(project), "--host", host, "--port", str(port)])
        finally:
            if name not in self.servers:
                endpoint = f"/tmp/agentx-{os.geteuid()}/{hashlib.sha256(os.fsencode(scope)).hexdigest()}.sock"
                with socket.socket(socket.AF_UNIX) as client:
                    client.connect(endpoint)
                    pid, uid, _gid = struct.unpack("3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                record = process_identity(pid)
                assert uid == os.geteuid() and record["executable"] == str(self.candidate), record
                assert record["argv"][1:] == ["daemon", "serve", "--scope", str(scope)], record
                self.processes.append(record)
                self.manifest["processes"].append(record)
                self.servers[name] = AdoptedProcess(record)
                self.persist()
        http = self.connect(name, project)
        descriptor = json.loads((scope / ".daemon/management.json").read_text())
        assert result.stdout.strip() == descriptor["url"], (result.stdout, descriptor)
        assert descriptor["bind_host"] == host and descriptor["bind_port"] == port, descriptor
        return http

    def start(self, name):
        env = self.environment(name)
        scope = Path(env["AGENTX_DIR"])
        index = len(self.processes)
        with (self.evidence / f"{name}-daemon-{index}.stdout").open("x") as stdout, (self.evidence / f"{name}-daemon-{index}.stderr").open("x") as stderr:
            process = subprocess.Popen([str(self.candidate), "daemon", "serve", "--scope", str(scope)],
                                       cwd=scope.parent / "project", env=env, stdin=subprocess.DEVNULL,
                                       stdout=stdout, stderr=stderr, start_new_session=True)
        record = process_identity(process.pid)
        self.processes.append(record)
        self.manifest["processes"].append(record)
        self.servers[name] = process
        self.persist()
        return self.connect(name)

    def connect(self, name, project=None, *, expected_host=None):
        scope = self.root / name / "scope"
        process = self.servers[name]
        self.connections += 1
        index = self.connections
        descriptor_path = scope / ".daemon/management.json"

        def descriptor_ready():
            assert process.poll() is None, f"daemon exited: {name}; see stderr"
            try:
                return json.loads(descriptor_path.read_text())
            except (FileNotFoundError, ValueError):
                return None

        descriptor = wait_for("native management descriptor", descriptor_ready)
        (self.evidence / f"{name}-descriptor-{index}.json").write_text(json.dumps(descriptor, indent=2) + "\n")
        http = Http(descriptor["url"], self.evidence, f"{name}-{index}", expected_host)
        status, view = http.call("GET", "/api/v1/management")
        assert status == 200, view
        assert view["scope"] == str(scope) and view["project"] == str(project or scope.parent / "project"), view
        assert isinstance(view["generation"], str) and view["generation"], view
        http.generation = view["generation"]
        self.clients[name] = http
        return http

    def cli(self, name, label, args):
        result = subprocess.run([str(self.candidate), *args], env=self.environment(name),
                                cwd=self.root / name / "project", capture_output=True, text=True, timeout=30)
        (self.evidence / f"{name}-cli-{label}.json").write_text(json.dumps({"args": args, "code": result.returncode,
                            "stdout": result.stdout, "stderr": result.stderr}, indent=2) + "\n")
        assert result.returncode == 0, (args, result.stderr)
        return result

    def stop_daemon(self, name):
        process = self.servers[name]
        record = next(item for item in self.processes if item["pid"] == process.pid)
        self.manifest["cleanup"].append(terminate_exact(record))
        process.wait(timeout=5)
        self.persist()
        descriptor = self.root / name / "scope/.daemon/management.json"
        wait_for("owned descriptor removed", lambda: not descriptor.exists(), timeout=5)

    def provider_records(self):
        records = []
        for path in sorted((self.root / "providers").glob("*.jsonl")):
            for line in path.read_text().splitlines():
                try:
                    records.append(json.loads(line))
                except ValueError:
                    pass  # A provider may be appending its final record.
        return records

    def close(self):
        failures = []
        # Stop via the public CLI first. It is scoped explicitly and does not
        # depend on possibly reset/stale HTTP references retained by a test.
        for name, process in self.servers.items():
            if process.poll() is None:
                try:
                    self.cli(name, "cleanup-stop", ["stop", "all"])
                except Exception as error:
                    failures.append(str(error))
        # Include a worker/PTY host that failed before the Provider could log.
        # Admission is the exact private run marker plus private scope, never a
        # process name, shared executable, port scan or ambient AgentX PID.
        scopes = {str(self.root / name / "scope") for name in self.servers}
        owned = []
        for path in Path("/proc").iterdir():
            if not path.name.isdigit():
                continue
            try:
                environment = dict(item.split(b"=", 1) for item in (path / "environ").read_bytes().split(b"\0") if b"=" in item)
                if environment.get(b"AGENTX_MANAGEMENT_FIXTURE") == os.fsencode(self.root) and environment.get(b"AGENTX_DIR", b"").decode() in scopes:
                    owned.append(process_identity(int(path.name)))
            except (OSError, ValueError):
                continue
        daemon_pids = {process.pid for process in self.servers.values()}
        for record in owned:
            if record["pid"] not in daemon_pids:
                try:
                    self.manifest["cleanup"].append(terminate_exact(record))
                except Exception as error:
                    failures.append(str(error))
        for name, process in self.servers.items():
            if process.poll() is None:
                try:
                    self.stop_daemon(name)
                except Exception as error:
                    failures.append(str(error))
        shutil.copytree(self.root / "providers", self.evidence / "providers")
        self.manifest["cleanup_errors"] = failures
        self.manifest["candidate_unchanged"] = self.manifest["sha256"] == hashlib.sha256(self.candidate.read_bytes()).hexdigest()
        self.persist()
        assert not failures and self.manifest["candidate_unchanged"], self.manifest
