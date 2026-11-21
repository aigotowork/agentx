"""Disposable deployment, subprocess lifetime and redacted wire evidence."""

import hashlib
import http.client
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import socket
import subprocess
import threading
import time
from urllib.parse import urlencode


def wait_for(label, observe, timeout=15, interval=0.1):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = observe()
        if result:
            return result
        time.sleep(interval)
    raise AssertionError(f"Timed out: {label}")


def require(condition, message):
    if not condition:
        raise AssertionError(message)


class Deployment:
    """Own one native server and its private environments; never use ambient credentials."""

    admin = "/api/v1/admin"

    def __init__(self, candidate, evidence):
        self.directory = evidence.absolute()
        self.directory.mkdir(mode=0o700)
        self.candidate = candidate.resolve(strict=True)
        with self.candidate.open("rb") as stream:
            self.candidate_sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
        self.tokens = []
        self.admin_token = self.token()
        self.runtime = self.directory / "runtime"
        self.runtime.mkdir(mode=0o700)
        self.process = None
        self.children = []
        self.boots = 0
        self.sequence = 0
        self.journal = threading.Lock()
        with socket.socket() as selected:
            selected.bind(("127.0.0.1", 0))
            self.port = selected.getsockname()[1]
        self.url = f"http://127.0.0.1:{self.port}"
        self.connect = lambda: http.client.HTTPConnection("127.0.0.1", self.port, timeout=5)
        self.public_url = "https://agentx-acceptance.example.invalid"
        self.result = {"candidate": str(self.candidate), "candidate_sha256": self.candidate_sha256,
                       "listen_url": self.url, "published_url": self.public_url, "cases": {}}

    def token(self):
        token = secrets.token_urlsafe(32)
        self.tokens.append(token)
        return token

    def redact(self, text):
        for token in self.tokens:
            text = text.replace(token, "<redacted>")
        return text

    def write(self, name, value):
        (self.directory / name).write_text(self.redact(json.dumps(value, indent=2, ensure_ascii=False)) + "\n")

    def environment(self, name):
        root = self.runtime / name
        for child in ("home", "workspace", "codex", "tmp", "scope"):
            (root / child).mkdir(parents=True, exist_ok=True)
        return {"HOME": str(root / "home"), "CODEX_HOME": str(root / "codex"),
                "TMPDIR": str(root / "tmp"), "AGENTX_DIR": str(root / "scope"),
                "PATH": os.environ["PATH"], "TERM": "xterm-256color", "LANG": "C.UTF-8"}

    def spawn(self, label, args, env, cwd=None):
        with (self.directory / f"{label}.log").open("xb") as log:
            process = subprocess.Popen(args, env=env, cwd=cwd or self.runtime,
                                       stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                                       start_new_session=True)
        self.children.append((label, process))
        return process

    def command(self, label, args, env, cwd=None, check=True, timeout=20):
        result = subprocess.run(args, env=env, cwd=cwd or self.runtime, text=True,
                                capture_output=True, timeout=timeout)
        self.write(f"{label}.json", {"argv": [str(arg) for arg in args], "exit_code": result.returncode,
                                    "stdout": result.stdout, "stderr": result.stderr})
        if check and result.returncode:
            raise AssertionError(f"{label} exited {result.returncode}; see {label}.json")
        return result

    def start(self):
        self.boots += 1
        argv = [self.candidate, "daemon", "cloud", "--root", self.runtime / "deployment",
                "--listen", f"127.0.0.1:{self.port}", "--public-url", self.public_url]
        label = f"deployment-{self.boots}"
        self.process = self.spawn(label, argv,
                                  {**self.environment("server"), "AGENTX_ADMIN_TOKEN": self.admin_token})

        def ready():
            if self.process.poll() is not None:
                raise AssertionError(f"Deployment exited {self.process.returncode}; see {label}.log")
            # A ready HTTP response alone would not prove we own the port.
            if f"Cloud daemon listening on 127.0.0.1:{self.port}" not in (self.directory / f"{label}.log").read_text():
                return False
            try:
                # Unauthenticated readiness is separate from the authenticated case assertion.
                status, _ = self.request("GET", self.admin + "/overview", record=False)
                return status in (200, 401, 403, 404)
            except (OSError, http.client.HTTPException):
                return False

        wait_for("deployment HTTP listener", ready)

    @staticmethod
    def stop(process):
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)

    def request(self, method, path, token=None, payload=None, query=None, record=True):
        if query:
            path += "?" + urlencode(query)
        connection = self.connect()
        headers = {"Authorization": f"Bearer {token}"} if token else {}
        body = None
        if payload is not None:
            body = json.dumps(payload).encode()
            headers["Content-Type"] = "application/json"
        try:
            connection.request(method, path, body, headers)
            response = connection.getresponse()
            raw = response.read(2_000_001)
            require(len(raw) <= 2_000_000, f"Unbounded response: {method} {path}")
            response_text = raw.decode()
            if record:
                with self.journal:
                    self.sequence += 1
                    self.write(f"http-{self.sequence:03}.json", {"method": method, "path": path,
                               "authenticated": token is not None, "request": payload, "status": response.status,
                               "response": response_text})
            require(all(secret not in response_text for secret in self.tokens),
                    f"Credential disclosed in response: {method} {path}")
            require(all(secret not in value for _, value in response.getheaders() for secret in self.tokens),
                    f"Credential disclosed in response headers: {method} {path}")
            parsed = json.loads(response_text) if response_text else None
            return response.status, parsed
        finally:
            connection.close()

    def expect(self, method, path, token=None, payload=None, *, query=None, statuses=(200,)):
        status, result = self.request(method, path, token, payload, query)
        require(status in statuses, f"{method} {path}: expected {statuses}, got {status}; see http-{self.sequence:03}.json")
        if status >= 400:
            require(isinstance(result, dict) and isinstance(result.get("code"), str) and result["code"]
                    and isinstance(result.get("message"), str) and result["message"],
                    f"{method} {path}: expected a typed error; see http-{self.sequence:03}.json")
        return result

    def close(self):
        errors = []
        for label, child in reversed(self.children):
            try:
                self.stop(child)
            except (OSError, subprocess.TimeoutExpired) as error:
                errors.append(f"{label}: {error}")
        # Runtime is explicitly owned and may contain credentials. Keep only the redacted evidence.
        if not errors:
            shutil.rmtree(self.runtime)
        for log in self.directory.glob("*.log"):
            contents = log.read_text(errors="replace")
            redacted = self.redact(contents)
            if contents != redacted:
                log.write_text(redacted)
        self.result["cleanup"] = {"processes": [{"label": label, "exit_code": child.poll()} for label, child in self.children],
                                  "runtime_removed": not self.runtime.exists(), "errors": errors}
        self.write("result.json", self.result)
        if errors:
            raise RuntimeError("Owned deployment cleanup failed; see result.json")
