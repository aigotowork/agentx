#!/usr/bin/env python3
"""Independent real adapter boundary cases; requires released product and actual Web bundle.

No service starts on import. Bootstrap/bearer material stays in process memory and
is never included in receipts. Native/browser cases supplement this bounded lane.
"""
import argparse
import hashlib
import http.client
import json
from pathlib import Path
import queue
import signal
import socket
import time
import subprocess
import tempfile
import threading
from urllib.parse import urlsplit, parse_qs


def transport_cases(origin, authority, port, attempts, evidence):
    """Actual finite connection/header/body checks; no native read or credentials."""
    held = []
    started = time.monotonic()
    try:
        for _ in range(8):
            connection = socket.create_connection(("127.0.0.1", port), timeout=2)
            connection.sendall(b"GET / HTTP/1.1\r\nHost:")
            held.append(connection)
        excess = socket.create_connection(("127.0.0.1", port), timeout=2)
        try:
            try:
                closed = excess.recv(1) == b""
            except ConnectionResetError:
                closed = True
            assert closed, "ninth connection admitted before parsing"
        finally:
            excess.close()
        attempts.append({"case": "eight-connections-excess-closed", "held": len(held), "excess_closed": closed})
        for connection in held:
            connection.settimeout(7)
            try:
                reply = connection.recv(1024)
            except ConnectionResetError:
                reply = b""
            assert not reply or reply.startswith(b"HTTP/1.1 408"), "partial header stayed live"
        elapsed = time.monotonic() - started
        assert 4 <= elapsed < 8, "header lifetime outside bounded allowance"
        attempts.append({"case": "partial-header-deadline", "connections": 8, "elapsed_seconds": elapsed})
    finally:
        for connection in held:
            connection.close()
        (evidence / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    try:
        started = time.monotonic()
        connection.putrequest("POST", "/api/v1/bootstrap", skip_host=True)
        for key, value in {"Host": authority, "Origin": origin, "Content-Type": "application/json", "Content-Length": "100"}.items():
            connection.putheader(key, value)
        connection.endheaders(b"{")
        response = connection.getresponse()
        response.read()
        elapsed = time.monotonic() - started
        assert response.status == 408 and 1.5 <= elapsed < 4
        attempts.append({"case": "partial-body-deadline", "status": response.status, "elapsed_seconds": elapsed})
    finally:
        connection.close()
        (evidence / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
    for label, extra in [("duplicate-host", [("Host", authority)]), ("duplicate-origin", [("Origin", origin)]), ("null-origin", [("Origin", "null")])]:
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
        try:
            connection.putrequest("POST", "/api/v1/read", skip_host=True)
            connection.putheader("Host", authority)
            if label != "null-origin": connection.putheader("Origin", origin)
            connection.putheader("Content-Type", "application/json")
            connection.putheader("Content-Length", "2")
            for key, value in extra: connection.putheader(key, value)
            connection.endheaders(b"{}")
            response = connection.getresponse()
            response.read()
            # Hyper may reject duplicate Host at protocol parsing before route admission.
            assert response.status in ({400, 403} if label == "duplicate-host" else {403})
            attempts.append({"case": label, "status": response.status})
        finally:
            connection.close()
            (evidence / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")


def run(binary, assets, evidence, resource_only=False, package_only=False):
    evidence.mkdir(parents=True, exist_ok=False)
    attempts = []
    with tempfile.TemporaryDirectory(prefix="ax-consumer-") as directory:
        root = Path(directory)
        scope = root / "scope"
        scope.mkdir(mode=0o700)
        home = root / "home"
        home.mkdir()
        env = {"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8", "CI": "1", "NO_COLOR": "1", "HOME": str(home), "AGENTX_DIR": str(scope), "TMPDIR": str(root), "XDG_CONFIG_HOME": str(home / "config"), "XDG_CACHE_HOME": str(home / "cache"), "XDG_DATA_HOME": str(home / "data"), "CODEX_HOME": str(home / "codex"), "CLAUDE_CONFIG_DIR": str(home / "claude")}
        before = sorted(str(p.relative_to(scope)) for p in scope.rglob("*"))
        child = subprocess.Popen([str(binary), "observe-web", "--scope", str(scope), "--assets", str(assets)], env=env, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        handoff = queue.Queue(maxsize=1)
        reader = threading.Thread(target=lambda: handoff.put(child.stdout.readline()), daemon=True)
        reader.start()
        session = None
        token = None
        try:
            line = handoff.get(timeout=10)
            url = urlsplit(line.strip())
            assert url.scheme == "http" and url.hostname == "127.0.0.1" and url.port >= 1024, "invalid loopback handoff"
            token = parse_qs(url.fragment)["bootstrap"][0]
            assert len(token) == 43
            origin = f"http://127.0.0.1:{url.port}"
            authority = f"127.0.0.1:{url.port}"

            def request(label, method, path, body=None, overrides=None, auth=False, expected=200, raw_body=None):
                headers = {"Host": authority, "Origin": origin, "Content-Type": "application/json"}
                if auth:
                    headers["Authorization"] = "Bearer " + session
                headers.update(overrides or {})
                connection = http.client.HTTPConnection("127.0.0.1", url.port, timeout=5)
                try:
                    encoded = raw_body if raw_body is not None else (None if body is None else json.dumps(body).encode())
                    connection.request(method, path, body=encoded, headers=headers)
                    response = connection.getresponse()
                    data = response.read(2 * 1024 * 1024 + 1)
                    attempts.append({"case": label, "method": method, "path": path, "status": response.status, "expected_status": expected, "response_bytes": len(data)})
                    (evidence / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
                    assert len(data) <= 2 * 1024 * 1024
                    assert response.status == expected, f"{label}: expected {expected}, got {response.status}"
                    assert response.getheader("Cache-Control") == "no-store"
                    assert response.getheader("Referrer-Policy") == "no-referrer"
                    assert response.getheader("X-Content-Type-Options") == "nosniff"
                    # Never persist actual credential-bearing response/input bytes.
                    value = json.loads(data) if response.getheader("Content-Type", "").startswith("application/json") else None
                    if label != "bootstrap":
                        assert token.encode() not in data and (not session or session.encode() not in data), "credential reflected"
                    return value
                finally:
                    connection.close()

            if resource_only:
                transport_cases(origin, authority, url.port, attempts, evidence)
            elif package_only:
                request("packaged-real-asset", "GET", "/")
                boot = request("bootstrap", "POST", "/api/v1/bootstrap", {"bootstrap": token})
                session = boot["bearer"]
                assert len(session) == 43 and boot["scope"]["canonical_scope"] == str(scope)
                value = request("packaged-native-passive-read", "POST", "/api/v1/read", {"expected_daemon": None, "read": "Snapshot"}, auth=True)
                assert "LocalFailure" in value and child.poll() is None
                request("packaged-logout", "POST", "/api/v1/logout", {}, auth=True)
            else:
                request("asset", "GET", "/")
                request("wrong-host", "GET", "/", overrides={"Host": "localhost:" + str(url.port)}, expected=403)
                request("wrong-origin", "POST", "/api/v1/read", {}, overrides={"Origin": "http://example.invalid"}, expected=403)
                request("unauthorized", "POST", "/api/v1/read", {"expected_daemon": None, "read": "Snapshot"}, expected=401)
                request("unknown-route", "POST", "/api/v1/business", {}, expected=404)
                request("invalid-bootstrap", "POST", "/api/v1/bootstrap", {"bootstrap": "A" * 43}, expected=401)
                boot = request("bootstrap", "POST", "/api/v1/bootstrap", {"bootstrap": token})
                session = boot["bearer"]
                assert len(session) == 43 and boot["scope"]["canonical_scope"] == str(scope)
                assert boot["idle_ms"] == {"$integer": "900000"}
                request("replayed-bootstrap", "POST", "/api/v1/bootstrap", {"bootstrap": token}, expected=401)
                missing = request("native-daemon-unavailable", "POST", "/api/v1/read", {"expected_daemon": None, "read": "Snapshot"}, auth=True)
                assert "LocalFailure" in missing and child.poll() is None
                request("mutation-rejected", "POST", "/api/v1/read", {"expected_daemon": None, "read": {"Report": {}}}, auth=True, expected=400)
                request("raw-integer-rejected", "POST", "/api/v1/read", {"expected_daemon": None, "read": {"Connections": {"before": None, "limit": 1}}}, auth=True, expected=400)
                request("canonical-integer-to-native", "POST", "/api/v1/read", {"expected_daemon": None, "read": {"Connections": {"before": None, "limit": {"$integer": "1"}}}}, auth=True)
                request("noncanonical-integer", "POST", "/api/v1/read", {"expected_daemon": None, "read": {"Connections": {"before": None, "limit": {"$integer": "01"}}}}, auth=True, expected=400)
                request("duplicate-field", "POST", "/api/v1/read", auth=True, expected=400, raw_body=b'{"expected_daemon":null,"read":"Snapshot","read":"Snapshot"}')
                request("unknown-field", "POST", "/api/v1/read", {"expected_daemon": None, "read": "Snapshot", "scope": str(root)}, auth=True, expected=400)
                request("cross-scope", "POST", "/api/v1/read", {"expected_daemon": {"scope": {"canonical_scope": str(root)}, "daemon_instance": "fixture"}, "read": "Snapshot"}, auth=True, expected=400)
                request("query-rejected", "POST", "/api/v1/read?scope=other", {"expected_daemon": None, "read": "Snapshot"}, auth=True, expected=403)
                request("media-type-rejected", "POST", "/api/v1/read", {}, overrides={"Content-Type": "text/plain"}, auth=True, expected=415)
                request("oversized-body", "POST", "/api/v1/read", auth=True, expected=413, raw_body=b" " * (32 * 1024 + 1))
                request("logout", "POST", "/api/v1/logout", {}, auth=True)
                request("revoked", "POST", "/api/v1/read", {"expected_daemon": None, "read": "Snapshot"}, auth=True, expected=401)
            assert sorted(str(p.relative_to(scope)) for p in scope.rglob("*")) == before, "passive adapter changed scope"
            child.send_signal(signal.SIGINT)
            code = child.wait(timeout=10)
            stderr = child.stderr.read()
            assert token not in stderr and (not session or session not in stderr), "credential in stderr"
            assert code == 0, f"adapter shutdown {code}"
        finally:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=10)
            child.stdout.close()
            child.stderr.close()
            reader.join(timeout=1)
            (evidence / "cleanup.json").write_text(json.dumps({"root": str(root), "pid": child.pid, "exit_code": child.returncode, "child_reaped": child.poll() is not None, "scope_changed": sorted(str(p.relative_to(scope)) for p in scope.rglob("*")) != before}, indent=2) + "\n")
        receipt = {"artifact": str(binary), "artifact_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "assets": str(assets), "scope": str(scope), "pid": child.pid, "exit_code": child.returncode, "attempts": attempts, "secrets_retained": False, "scope_changed": False, "child_reaped": True, "limits": "Real HTTP boundary with actual selected bundle and no daemon; not full API/store/browser acceptance."}
    receipt["root_removed"] = not root.exists()
    (evidence / "boundary.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"cases": len(attempts), "root_removed": receipt["root_removed"], "child_reaped": True}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--resource-only", action="store_true")
    parser.add_argument("--package-only", action="store_true")
    args = parser.parse_args()
    try:
        run(args.binary.resolve(strict=True), args.assets.resolve(strict=True), args.evidence.resolve(), args.resource_only, args.package_only)
    except Exception as error:
        # Exception text may contain handoff credentials; retain only safe type and cleanup facts.
        cleanup_path = args.evidence / "cleanup.json"
        cleanup = json.loads(cleanup_path.read_text()) if cleanup_path.exists() else {}
        args.evidence.mkdir(parents=True, exist_ok=True)
        (args.evidence / "failure.json").write_text(json.dumps({"failure_type": type(error).__name__, "root_removed": not Path(cleanup["root"]).exists() if "root" in cleanup else None, "child_reaped": cleanup.get("child_reaped"), "secrets_retained": False}, indent=2) + "\n")
        raise SystemExit("Consumer boundary failed; sanitized attempts/cleanup retained") from None


if __name__ == "__main__":
    main()
