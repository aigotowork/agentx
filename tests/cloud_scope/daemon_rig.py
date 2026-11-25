"""Own disposable local or SSH roots and construct the cloud daemon acceptance topology."""

from contextlib import contextmanager
from concurrent.futures import ThreadPoolExecutor
import http.client
import ipaddress
import json
import os
from pathlib import Path
import shlex
import ssl
import subprocess
import tempfile
from urllib.parse import urlparse

from fixture import require, wait_for
from management import certificates, tls_front


SCOPE = "native-daemon"
STATION_SCRIPT = Path(__file__).with_name("daemon_station.py")
SSH = ["ssh", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes", "-o", "ConnectTimeout=8"]
PORT_CODE = "import socket; s=socket.socket(); s.bind(('127.0.0.1',0)); print(s.getsockname()[1])"


class Station:
    """A private machine root, with explicit candidate and environment for every command."""

    def __init__(self, fixture, label, ssh=None, jump=None):
        self.fixture, self.label, self.ssh = fixture, label, ssh
        self.ssh_dir = tempfile.TemporaryDirectory(prefix="axd-ssh-", dir="/tmp") if ssh else None
        self.ssh_argv = [*SSH, *(["-J", jump] if jump else [])]
        if self.ssh_dir:
            self.ssh_argv.extend(["-o", "ControlMaster=auto", "-o", "ControlPersist=120",
                                  "-o", f"ControlPath={self.ssh_dir.name}/control",
                                  "-o", "ServerAliveInterval=10", "-o", "ServerAliveCountMax=3"])
        try:
            self.root = Path(self.raw(["mktemp", "-d", "/tmp/axd-XXXXXXXX"]).stdout.strip())
        except Exception:
            self.close_ssh()
            raise
        require(self.root.parent == Path("/tmp") and self.root.name.startswith("axd-"),
                "Refusing unexpected temporary path")
        self.candidate = self.root / "bin/agentx" if ssh else fixture.candidate
        self.workspace = self.root / "workspace"
        self.env = {}

    def initialize(self):
        self.raw(["mkdir", "-m", "700", "--", *(self.root / name for name in
                 ("home", "codex", "tmp", "scope", "workspace", "bin"))])
        path = self.raw_shell('printf "%s" "$PATH"').stdout
        self.env = {"HOME": str(self.root / "home"), "CODEX_HOME": str(self.root / "codex"),
                    "TMPDIR": str(self.root / "tmp"), "AGENTX_DIR": str(self.root / "scope"),
                    "PATH": f"{self.root / 'bin'}:{path}", "LANG": "C.UTF-8", "TERM": "xterm-256color"}
        if self.ssh:
            self.transfer(self.fixture.candidate, self.candidate)
            self.raw(["chmod", "700", self.candidate])
            digest = self.raw(["sha256sum", self.candidate]).stdout.split()[0]
            require(digest == self.fixture.candidate_sha256, f"{self.label} candidate SHA-256 mismatch")
        else:
            self.raw(["ln", "-s", self.candidate, self.root / "bin/agentx"])
        self.write(self.root / STATION_SCRIPT.name, STATION_SCRIPT.read_text())
        self.fixture.write(f"{self.label}-station.json",
                           {"hostname": self.raw(["hostname"]).stdout.strip(), "address": self.ssh or "local",
                            "candidate_sha256": self.fixture.candidate_sha256, "root": str(self.root)})

    def argv(self, command):
        return [*self.ssh_argv, "--", self.ssh, command] if self.ssh else ["/bin/sh", "-c", command]

    def raw_shell(self, command, input_text=None, timeout=30, check=True):
        result = subprocess.run(self.argv(command), input=input_text, capture_output=True, text=True, timeout=timeout)
        if check and result.returncode:
            raise RuntimeError(f"{self.label} command exit {result.returncode}: {self.fixture.redact(result.stderr.strip())}")
        return result

    def raw(self, args, **kwargs):
        return self.raw_shell(shlex.join(map(str, args)), **kwargs)

    def write(self, target, contents):
        self.raw_shell(f"umask 077; cat > {shlex.quote(str(target))}", input_text=contents)

    def transfer(self, source, target):
        if self.ssh and source.stat().st_size >= 1024 * 1024:
            # Compress the payload itself: flags on a later SSH session do not
            # renegotiate compression on an already established master.
            with tempfile.TemporaryFile() as archive:
                subprocess.run(["zstd", "-q", "-3", "-c", source], stdout=archive, check=True, timeout=25)
                archive.seek(0)
                command = f"umask 077; zstd -q -d -c > {shlex.quote(str(target))}"
                subprocess.run(self.argv(command), stdin=archive, check=True, timeout=420)
        elif self.ssh:
            with source.open("rb") as data:
                command = f"umask 077; cat > {shlex.quote(str(target))}"
                subprocess.run(self.argv(command), stdin=data, check=True, timeout=30)
        else:
            self.write(target, source.read_text())

    def command(self, label, *arguments, check=True, timeout=75, cwd=None):
        result = self.raw_shell(self.invocation([self.candidate, *arguments], cwd=cwd), timeout=timeout, check=False)
        self.fixture.write(f"{self.label}-{label}.json",
                           {"argv": list(map(str, arguments)), "exit_code": result.returncode,
                            "stdout": result.stdout, "stderr": result.stderr})
        require(all(secret not in result.stdout + result.stderr for secret in self.fixture.tokens),
                f"{self.label} {label} disclosed a fixture credential")
        if check:
            require(result.returncode == 0, f"{self.label} {label}: exit {result.returncode}; see command receipt")
        return result

    def invocation(self, args, *, private_admin_token=None, cwd=None):
        environment = [f"{key}={value}" for key, value in self.env.items()]
        command = shlex.join(map(str, [*environment, *args]))
        # The cloud server alone expands its own private file into an environment variable.
        private = f'AGENTX_ADMIN_TOKEN="$(cat {shlex.quote(str(private_admin_token))})" ' if private_admin_token else ""
        return f"cd {shlex.quote(str(cwd or self.workspace))} && exec env -i {private}{command}"

    def process(self, action):
        result = self.raw(["python3", self.root / STATION_SCRIPT.name, action, self.root, self.candidate], timeout=45)
        return json.loads(result.stdout)

    def prepare_worker(self, url, cert, credential, host, model, model_port=None, codex_bin=None, *,
                       project=None, role="acceptance"):
        roles = (project or self.workspace) / ".agentx/roles"
        self.raw(["mkdir", "-p", "--", roles])
        for name, profile in ((role, "acceptance-codex"), ("unavailable", "missing-profile")):
            self.write(roles / f"{name}.toml",
                       f'profile = "{profile}"\nprompt = "Cloud daemon acceptance role"\n')
        self.write(self.root / "scope-token", credential + "\n")
        self.transfer(cert, self.root / "ca.crt")
        self.env.update({"SSL_CERT_FILE": str(self.root / "ca.crt"),
                         "CLOUD_ACCEPTANCE_MODEL_KEY": "local-model-no-real-credential"})
        self.write(self.root / "codex/config.toml", model.configuration(model_port, restrict_network=True))
        self.command("binary-version", "--version")
        if self.ssh and codex_bin:
            for name in ("codex", "codex-code-mode-host"):
                source = (codex_bin / name).resolve(strict=True)
                target = self.root / "bin" / name
                self.transfer(source, target)
                self.raw(["chmod", "700", target])
        provider = self.raw_shell(self.invocation(["codex", "--version"]), check=False)
        self.fixture.write(f"{self.label}-codex.json", {"version": provider.stdout.strip(),
                           "exit_code": provider.returncode, "stderr": provider.stderr})
        require(provider.returncode == 0 and provider.stdout.strip(), f"{self.label} needs a working Codex CLI")
        self.command("profile-create", "profile", "create", "acceptance-codex", "--tool", "codex")
        self.command("profile-default", "profile", "default", "acceptance-codex")
        self.command("connect", "daemon", "connect", "--configure-only", "--scope", self.root / "scope",
                     "--namespace-url", url, "--host", host, "--project-root", f"work={self.workspace}",
                     "--token-file", self.root / "scope-token")
        mode = self.raw(["stat", "-c", "%a", self.root / "scope/cloud.toml"]).stdout.strip()
        require(mode == "600", f"{host} cloud.toml must be mode 0600, got {mode}")

    def close(self):
        if self.raw(["test", "-f", self.root / STATION_SCRIPT.name], check=False).returncode == 0:
            stopped = self.process("cleanup")
            self.fixture.write(f"{self.label}-stopped.json", stopped)
        log_dirs = [self.root / "scope/.daemon", self.root / "scope/cloud-client/logs"]
        result = self.raw(["find", *log_dirs, "-maxdepth", "1", "-type", "f", "-name", "*.log", "-print"], check=False)
        for line in result.stdout.splitlines():
            log = Path(line)
            if log.parent in log_dirs:
                content = self.raw(["tail", "-c", "200000", log], check=False).stdout
                (self.fixture.directory / f"{self.label}-{log.name}").write_text(self.fixture.redact(content))
        self.raw(["rm", "-rf", "--", self.root])
        require(self.raw(["test", "!", "-e", self.root], check=False).returncode == 0,
                f"{self.label} scratch root was not removed")

    def close_ssh(self):
        if self.ssh_dir:
            try:
                if (Path(self.ssh_dir.name) / "control").exists():
                    subprocess.run([*self.ssh_argv, "-O", "exit", "--", self.ssh],
                                   capture_output=True, timeout=8, check=False)
            finally:
                self.ssh_dir.cleanup()


def use_https(fixture, ca):
    origin = urlparse(fixture.public_url)
    context = ssl.create_default_context(cafile=ca)
    fixture.connect = lambda: http.client.HTTPSConnection(origin.hostname, origin.port, context=context, timeout=15)


def serve_physical(fixture, cloud, server_ip):
    ca, cert, key = certificates(fixture, server_ip)
    for local in (cert, key):
        cloud.transfer(local, cloud.root / local.name)
    cloud.write(cloud.root / "admin-token", fixture.admin_token + "\n")
    # Reserve two distinct unused ports together; the owned listeners subsequently bind them.
    backend_port, tls_port = map(int, cloud.raw(["python3", "-c",
        "import socket,sys; a=socket.socket(); b=socket.socket(); a.bind(('127.0.0.1',0)); "
        "b.bind((sys.argv[1],0)); print(a.getsockname()[1],b.getsockname()[1])", server_ip]).stdout.split())
    fixture.public_url = f"https://{server_ip}:{tls_port}"
    fixture.result.update({"published_url": fixture.public_url, "listen_url": f"http://127.0.0.1:{backend_port}"})
    require(ipaddress.ip_address(server_ip).is_private, "Physical fixture must bind a private LAN address")
    command = cloud.invocation([cloud.candidate, "daemon", "cloud", "--root", cloud.root / "deployment",
                                "--listen", f"127.0.0.1:{backend_port}", "--public-url", fixture.public_url],
                               private_admin_token=cloud.root / "admin-token")
    daemon = fixture.spawn("physical-cloud", cloud.argv(command), os.environ.copy())
    cloud.write(cloud.root / "Caddyfile", f"""{{
    admin off
    auto_https disable_redirects
}}
{fixture.public_url} {{
    bind {server_ip}
    tls {cloud.root / cert.name} {cloud.root / key.name}
    reverse_proxy 127.0.0.1:{backend_port}
}}
""")
    caddy = fixture.spawn("physical-tls", cloud.argv(cloud.invocation(
        ["caddy", "run", "--config", cloud.root / "Caddyfile", "--adapter", "caddyfile"])), os.environ.copy())
    use_https(fixture, ca)

    def ready():
        require(daemon.poll() is None and caddy.poll() is None, "Physical cloud service exited; see service logs")
        if f"Cloud daemon listening on 127.0.0.1:{backend_port}" not in (fixture.directory / "physical-cloud.log").read_text():
            return False
        try:
            status, _ = fixture.request("GET", fixture.admin + "/overview", record=False)
            return status in (401, 403)
        except (OSError, http.client.HTTPException):
            return False

    wait_for("physical cloud HTTPS terminator", ready, timeout=20)
    return ca


def model_forward(fixture, worker, local_port):
    port = int(worker.raw(["python3", "-c", PORT_CODE]).stdout)
    forward = subprocess.run([*worker.ssh_argv, "-O", "forward", "-R", f"127.0.0.1:{port}:127.0.0.1:{local_port}",
                              "--", worker.ssh], text=True, capture_output=True, timeout=15, check=False)
    fixture.write("fixture-model-ssh.json", {"status": forward.returncode, "stderr": forward.stderr})
    require(forward.returncode == 0, "Fixture-only scripted model SSH forwarding failed")

    def ready():
        response = worker.raw(["python3", "-c",
            "import socket,sys; socket.create_connection(('127.0.0.1',int(sys.argv[1])),1).close()", port],
            check=False, timeout=10)
        return response.returncode == 0

    wait_for("remote access to scripted model", ready, timeout=12)
    return port


@contextmanager
def topology(fixture, physical=None, east_jump=None):
    stations = []

    def station(name, ssh=None, jump=None):
        selected = Station(fixture, name, ssh, jump)
        stations.append(selected)
        selected.initialize()
        return selected

    try:
        west = station("west")
        if physical:
            cloud_ssh, server_ip, worker_ssh = physical
            with ThreadPoolExecutor(max_workers=2) as pool:
                hosts = [pool.submit(station, "cloud", cloud_ssh), pool.submit(station, "east", worker_ssh, east_jump)]
                cloud, east = (host.result() for host in hosts)
            names = [fixture.directory / f"{selected.label}-station.json" for selected in (west, east, cloud)]
            hostnames = {json.loads(name.read_text())["hostname"] for name in names}
            require(len(hostnames) == 3, "Physical acceptance requires three distinct observed hosts")
            yield east, west, serve_physical(fixture, cloud, server_ip), None, None
        else:
            east = station("east")
            observations = []
            drop = f"{fixture.admin}/namespaces/{SCOPE}/operations"
            with tls_front(fixture, "POST", drop, observations) as (ca, _pin, attempts):
                fixture.start()
                use_https(fixture, ca)
                yield east, west, ca, attempts, observations
    finally:
        errors = []
        for selected in reversed(stations):
            try:
                selected.close()
            except Exception as error:
                errors.append(f"{selected.label}: {error}")
            finally:
                selected.close_ssh()
        fixture.result["station_cleanup"] = {"roots": [str(selected.root) for selected in stations], "errors": errors}
        require(not errors, "Fixture station cleanup failed; see result.json")
