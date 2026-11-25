"""Use external management clients against a disposable native deployment."""

from contextlib import contextmanager
import base64
import hashlib
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import re
import shutil
import socket
import ssl
import stat
import subprocess
import tempfile
import threading
import time

from fixture import CLOUD_PROTOCOL, COMPACT_ID, require, wait_for


def certificates(fixture, ip="127.0.0.1"):
    """Generate a separate private CA and IP-verified leaf for one fixture."""
    ca_key, ca, key, cert, csr, extensions = (fixture.runtime / name for name in
        ("ca.key", "ca.crt", "tls.key", "tls.crt", "tls.csr", "tls.ext"))
    env = fixture.environment("tls")
    fixture.command("test-ca", ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", ca_key,
                    "-out", ca, "-days", "1", "-subj", "/CN=AgentX acceptance CA", "-addext", "basicConstraints=critical,CA:TRUE",
                    "-addext", "keyUsage=critical,keyCertSign,cRLSign"], env)
    fixture.command("test-leaf-request", ["openssl", "req", "-new", "-newkey", "rsa:2048", "-nodes",
                    "-keyout", key, "-out", csr, "-subj", f"/CN={ip}"], env)
    extensions.write_text("basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\n"
                          f"extendedKeyUsage=serverAuth\nsubjectAltName=IP:{ip}\n")
    fixture.command("test-leaf-certificate", ["openssl", "x509", "-req", "-in", csr, "-CA", ca, "-CAkey", ca_key,
                    "-CAcreateserial", "-out", cert, "-days", "1", "-extfile", extensions], env)
    return ca, cert, key


def certificate_pin(cert):
    """SPKI pin for a private, fixture-only Chromium profile."""
    pubkey = subprocess.check_output(["openssl", "x509", "-in", cert, "-pubkey", "-noout"])
    der = subprocess.check_output(["openssl", "pkey", "-pubin", "-outform", "DER"], input=pubkey)
    return base64.b64encode(hashlib.sha256(der).digest()).decode()


@contextmanager
def tls_front(fixture, drop_method=None, drop_path=None, observed_polls=None, protocol_overrides=None):
    """Use a private CA and leaf; inject a protocol version or lose one response when requested."""
    ca, cert, key = certificates(fixture)
    pin = certificate_pin(cert)
    attempts = []
    overridden = []
    lock = threading.Lock()

    class Proxy(BaseHTTPRequestHandler):
        def handle_request(self):
            require(self.headers.get("Transfer-Encoding") is None, "Fixture only admits bounded HTTP bodies")
            size = int(self.headers.get("Content-Length", 0))
            require(0 <= size <= 1_000_000, "Fixture request exceeded the body bound")
            omit = {"connection", "host", "transfer-encoding", "content-length"}
            headers = {key: value for key, value in self.headers.items() if key.lower() not in omit}
            observe = observed_polls is not None and self.path.endswith("/control/poll")
            began = time.monotonic()
            connection = http.client.HTTPConnection("127.0.0.1", fixture.port, timeout=35 if observe or self.path.partition("?")[0].endswith("/changes") else 10)
            try:
                connection.request(self.command, self.path, self.rfile.read(size) if size else None, headers)
                response = connection.getresponse()
                data = response.read(8_000_001)
                require(len(data) <= 8_000_000, "Fixture response exceeded the body bound")
                protocol = (protocol_overrides or {}).get(self.path)
                if self.command == "GET" and protocol is not None and response.status == 200:
                    descriptor = json.loads(data)
                    original = descriptor["protocol_version"]
                    data = json.dumps({**descriptor, "protocol_version": protocol}).encode()
                    with lock:
                        overridden.append({"path": self.path, "original": original, "advertised": protocol})
                if observe:
                    with lock:
                        observed_polls.append({"path": self.path, "started": began, "seconds": time.monotonic() - began,
                                               "status": response.status,
                                               "empty": response.status == 200 and json.loads(data).get("operation") is None})
                if (self.command, self.path) == (drop_method, drop_path):
                    with lock:
                        drop = 200 <= response.status < 300 and not any(entry["dropped"] for entry in attempts)
                        attempts.append({"status": response.status, "dropped": drop})
                    if drop:
                        self.connection.shutdown(socket.SHUT_RDWR)
                        return
                self.send_response(response.status)
                for key, value in response.getheaders():
                    if key.lower() not in omit | {"server", "date"}:
                        self.send_header(key, value)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            finally:
                connection.close()

        do_GET = do_POST = do_PUT = do_HEAD = handle_request

        def log_message(self, *_args):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain(cert, key)
    server.socket = tls.wrap_socket(server.socket, server_side=True)
    srv = threading.Thread(target=server.serve_forever)
    fixture.public_url = f"https://127.0.0.1:{server.server_port}"
    fixture.result["published_url"] = fixture.public_url
    srv.start()
    try:
        yield ca, pin, attempts
    finally:
        server.shutdown()
        server.server_close()
        srv.join(5)
        require(not srv.is_alive(), "Owned HTTPS terminator failed to stop")
        fixture.write("https-proxy.json", {"url": fixture.public_url, "attempts": attempts, "protocol_overrides": overridden,
                                           "discarded_route": drop_path, "closed": True})


def cli(fixture, consumer, cert, attempts):
    package = consumer.resolve(strict=True) / "node_modules/@agentx/client"
    executable = consumer / "node_modules/.bin/agentx-http"
    require(executable.resolve(strict=True).is_relative_to(package), "Select an externally installed @agentx/client")
    pkg = json.loads((package / "package.json").read_text())
    require(pkg["name"] == "@agentx/client", "Unexpected consumer package")
    env = {**fixture.environment("cli"), "AGENTX_ADMIN_URL": fixture.public_url,
           "AGENTX_ADMIN_TOKEN": fixture.admin_token, "NODE_EXTRA_CA_CERTS": str(cert)}
    for surface in ("admin", "namespace"):
        export = fixture.command(f"openapi-{surface}", [fixture.candidate, "cloud-openapi", surface], env)
        packaged = json.loads((package / f"cloud-{surface}.openapi.json").read_text())
        require(json.loads(export.stdout) == packaged, f"Packaged {surface} OpenAPI differs from the native candidate export")
    pending = fixture.runtime / "cli" / "pending.json"
    token = fixture.token()
    token_file = pending.with_name("new-token")
    token_file.write_text(token)
    token_file.chmod(0o600)

    def command(label, *args, check=True):
        return fixture.command(label, [executable, "admin", *args], env, check=check)

    lost = command("cli-create-lost", "namespaces", "create", "cli-managed", "--pending", str(pending),
                   "--new-token-file", str(token_file), check=False)
    require(lost.returncode != 0 and token not in lost.stdout + lost.stderr
            and fixture.admin_token not in lost.stdout + lost.stderr, "Ambiguous result must fail without exposing either credential")
    require(len(attempts) == 1 and attempts[0]["dropped"], "Exactly one committed response must be lost without automatic retry")
    require(stat.S_IMODE(pending.stat().st_mode) == 0o600, "Pending credentials must be stored in a private file")
    before = fixture.expect("GET", fixture.admin + "/namespaces/cli-managed", fixture.admin_token)
    require(before["credential_version"] == 1, "Deployment must record the change before CLI reconciliation")
    replay = json.loads(command("cli-pending-replay", "pending", "replay", str(pending)).stdout)
    require(replay["generation"] == before["generation"] and replay["credential_version"] == 1
            and len(attempts) == 2 and not attempts[-1]["dropped"], "Explicit replay must recover the original receipt")
    require(json.loads(command("cli-pending-reveal", "pending", "reveal", str(pending)).stdout) == {"token": token},
            "Explicit reveal must recover the original exact credential")

    sdk = Path(__file__).with_name("sdk.mjs")
    # Load the package through the actual external consumer's module resolver.
    result = fixture.command("sdk-read", ["node", "--input-type=module", "--eval", sdk.read_text()],
                             {**env, "AGENTX_NAMESPACE_TOKEN": token}, cwd=consumer)
    values = json.loads(result.stdout)
    require(values["detail"]["generation"] == before["generation"] and values["page"]["items"][0]["namespace_id"] == "cli-managed",
            "Public SDK reads must agree with committed CLI receipt and raw HTTP")
    require(values["hosts"] == values["agents"] == {"generation": before["generation"], "items": [], "next": None}, "New scope inventory must be empty through the SDK")
    require(values["workspaces"] == values["adminWorkspaces"] == {"generation": before["generation"], "items": [], "next": None},
            "A host without a project lease must read as zero Workspaces through both SDK surfaces")
    require(values["overview"]["protocol_version"] == CLOUD_PROTOCOL
            and values["descriptor"] == {"namespace_id": "cli-managed", "generation": before["generation"], "protocol_version": CLOUD_PROTOCOL}
            and values["host"]["online"] and values["scoped"] == values["cross"] == {"generation": before["generation"], "items": [values["agent"]], "next": None},
            "SDK scope enrollment must appear under the same exact identity in scope and deployment reads")

    replacement = fixture.token()
    token_file.write_text(replacement)
    rotated = json.loads(command("cli-rotate", "namespaces", "rotate-token", "cli-managed", "--expected-version", "1",
                                 "--pending", str(pending.with_name("rotation.json")), "--new-token-file", str(token_file)).stdout)
    require(rotated == {**replay, "credential_version": 2}, "CLI must rotate the exact scope without changing generation")
    scope_api = "/namespaces/cli-managed/api/v1/descriptor"
    fixture.expect("GET", scope_api, token, statuses=(401, 403))
    fixture.expect("GET", scope_api, replacement)
    return {"package_version": pkg["version"], "package_integrity": json.loads((consumer / "package-lock.json").read_text())
            ["packages"]["node_modules/@agentx/client"].get("integrity"), "initial_receipt": replay, "rotation_receipt": rotated}


def tui(fixture):
    tmux = shutil.which("tmux")
    require(tmux, "TUI acceptance requires tmux")
    env = {**fixture.environment("tui"), "AGENTX_ADMIN_TOKEN": fixture.admin_token}
    with tempfile.TemporaryDirectory(prefix="agentx-cloud-tui-") as root:
        private_socket = Path(root) / "tmux.sock"

        def terminal(*args, check=True):
            return subprocess.run([tmux, "-f", "/dev/null", "-S", private_socket, *args], env=env,
                                  text=True, capture_output=True, check=check, timeout=10).stdout

        def screen(label, *strings, row=None, absent=()):
            captured = ""
            def read():
                nonlocal captured
                captured = terminal("capture-pane", "-pt", "deployment")
                revealed = re.search(rf"New communication token[^\n]*\n[^A-Za-z0-9_-]*({COMPACT_ID})(?![A-Za-z0-9_-])", captured)
                if revealed and revealed[1] not in fixture.tokens:
                    fixture.tokens.append(revealed[1])
                if (all(string in captured for string in strings)
                        and not any(string in captured for string in (*absent, "Working…"))
                        and (row is None or re.search(row, captured, re.MULTILINE))):
                    (fixture.directory / (label + ".txt")).write_text(fixture.redact(captured))
                    return captured
                return False
            try:
                return wait_for(label, read)
            except AssertionError:
                (fixture.directory / (label + "-failure.txt")).write_text(fixture.redact(captured))
                raise

        def keys(*args):
            terminal("send-keys", "-t", "deployment", *args)

        try:
            terminal("new-session", "-d", "-s", "deployment", "-x", "125", "-y", "31",
                     str(fixture.candidate), "tui", "--admin-url", fixture.url)
            screen("tui-overview", "AgentX / Deployment", "n Create", fixture.url)
            keys("n")
            screen("tui-create-dialog", "Create namespace", "Namespace ID")
            keys("-l", "tui-managed")
            keys("Tab")
            screen("tui-create-revealed", "Save the token before submitting.", "New communication token")
            create_token = fixture.tokens[-1]
            keys("Enter")
            screen("tui-created", "Saved", "credential v1")
            detail_path = fixture.admin + "/namespaces/tui-managed"
            v1 = fixture.expect("GET", detail_path, fixture.admin_token)
            require(v1["credential_version"] == 1, "TUI create must reach admin inventory")
            keys("Enter")
            screen("tui-namespace-list", "1 namespace ·", "n Create",
                   row=r"^│›[ \t]+\d+/\d+[ \t]+\d+[ \t]+tui-managed[ \t]*│$", absent=("Create namespace", "Saved"))
            keys("Right")
            screen("tui-hosts", "[Hosts]", "tui-managed", "token v1", v1["namespace_url"])
            keys("Tab")
            screen("tui-agents", "[Agents]", "tui-managed", "r Rotate")
            keys("r")
            screen("tui-rotate-dialog", "Rotate namespace token")
            keys("Tab")
            screen("tui-rotation-revealed", "Save the token before submitting.", "New communication token")
            rotation_token = fixture.tokens[-1]
            require(rotation_token != create_token and create_token != fixture.admin_token,
                    "TUI must reveal a distinct private credential for each change")
            keys("Enter")
            screen("tui-rotated", "Saved", "credential v2")
            v2 = fixture.expect("GET", detail_path, fixture.admin_token)
            require(v2["credential_version"] == 2 and v2["generation"] == v1["generation"],
                    "TUI rotation must update the authoritative version in place")
            descriptor = "/namespaces/tui-managed/api/v1/descriptor"
            fixture.expect("GET", descriptor, create_token, statuses=(401, 403))
            fixture.expect("GET", descriptor, rotation_token)
            return {"scope": v2["namespace_id"], "credential_versions": [1, 2], "generation": v2["generation"]}
        finally:
            terminal("kill-server", check=False)


def browser(fixture, chromium, pin):
    initial, rotated = fixture.token(), fixture.token()
    env = {**fixture.environment("browser"), "CLOUD_BROWSER": str(chromium),
           "CLOUD_BROWSER_URL": fixture.public_url, "CLOUD_CERT_PIN": pin, "CLOUD_ADMIN_TOKEN": fixture.admin_token,
           "CLOUD_CREATE_TOKEN": initial, "CLOUD_ROTATED_TOKEN": rotated, "CLOUD_EVIDENCE": str(fixture.directory)}
    root = Path(__file__).resolve().parents[2]
    # Chromium's profile uses a Unix socket; keep the owned path short even under a deep evidence directory.
    with tempfile.TemporaryDirectory(prefix="agentx-cloud-chrome-", dir="/tmp") as short_path:
        result = fixture.command("browser", ["node", "--input-type=module", "--eval", Path(__file__).with_name("browser.mjs").read_text()],
                                 {**env, "TMPDIR": short_path, "CLOUD_CHROME_PROFILE": short_path + "/profile"}, cwd=root, timeout=100)
    observed = json.loads(result.stdout)
    scope = fixture.expect("GET", fixture.admin + "/namespaces/web-managed", fixture.admin_token)
    require(scope["credential_version"] == 2, "Browser rotation must agree with the authoritative Scope detail")
    descriptor = "/namespaces/web-managed/api/v1/descriptor"
    fixture.expect("GET", descriptor, initial, statuses=(401, 403))
    fixture.expect("GET", descriptor, rotated)
    return {**observed, "scope": scope["namespace_id"], "generation": scope["generation"]}
