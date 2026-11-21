#!/usr/bin/env python3
"""Direct LAN/standalone acceptance; no credentials and no daemon acquisition."""
import argparse
import concurrent.futures
import hashlib
import http.client
import json
import os
from pathlib import Path
import queue
import signal
import subprocess
import tempfile
import threading
from urllib.parse import urlsplit


def run(binary, address, evidence):
    evidence.mkdir(parents=True, exist_ok=False)
    attempts = []
    with tempfile.TemporaryDirectory(prefix="ax-lan-") as directory:
        root = Path(directory)
        scope = root / "scope"
        scope.mkdir(mode=0o700)
        # Only copied executable exists here: no assets, checkout or current-directory dependency.
        import shutil
        image = root / "agentx"
        shutil.copy2(binary, image)
        child = subprocess.Popen([str(image), "observe-web", "--scope", str(scope), "--host", "0.0.0.0", "--iam", "nano"], cwd=root, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        lines = queue.Queue()
        threading.Thread(target=lambda: lines.put(child.stdout.readline()), daemon=True).start()
        try:
            launch = urlsplit(lines.get(timeout=15).strip())
            assert launch.scheme == "http" and launch.hostname == "0.0.0.0" and launch.port
            assert not launch.fragment and not launch.query
            origin = f"http://{address}:{launch.port}"
            def request(label, path, value=None, origin_override=None, host_override=None, expected=200):
                client = http.client.HTTPConnection(address, launch.port, timeout=5)
                headers = {"Origin": origin if origin_override is None else origin_override, "Content-Type": "application/json"}
                if host_override is not None:
                    headers["Host"] = host_override
                try:
                    client.request("GET" if value is None else "POST", path, body=None if value is None else json.dumps(value), headers=headers)
                    reply = client.getresponse()
                    data = reply.read()
                    assert reply.status == expected, (label, reply.status, data[:200])
                    assert reply.getheader("Cache-Control") == "no-store"
                    attempts.append({"case": label, "status": reply.status, "bytes": len(data)})
                    return data
                finally:
                    client.close()
            html = request("embedded-page", "/")
            assert b"/entry.js" in html
            js = request("embedded-entry", "/entry.js")
            assert b"ObservationApi.connect()" in js and b"#bootstrap" not in js
            metadata = json.loads(request("direct-scope", "/api/v1/scope", {}))
            assert metadata["canonical_scope"] == str(scope)
            # Separate requests are separate clients, without cookies, bearer or previous handoff.
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                replies = list(pool.map(lambda n: request(f"independent-read-{n}", "/api/v1/read", {"read": "Snapshot"}), range(2)))
            assert all("LocalFailure" in json.loads(reply) for reply in replies)
            request("reload-page", "/")
            request("reconnect-scope", "/api/v1/scope", {})
            request("cross-origin", "/api/v1/read", {"read": "Snapshot"}, origin_override="http://example.com", expected=403)
            request("forged-host", "/", host_override=f"192.0.2.1:{launch.port}", expected=403)
            request("no-bootstrap", "/api/v1/bootstrap", {}, expected=404)
            request("no-mutation", "/api/v1/read", {"read": "Submit"}, expected=400)
            request("cross-scope", "/api/v1/read", {"read": "Snapshot", "scope": "/elsewhere"}, expected=400)
            assert not list(scope.iterdir()), "passive server acquired or mutated selected scope"
            (evidence / "acceptance.json").write_text(json.dumps({"sha256": hashlib.sha256(image.read_bytes()).hexdigest(), "address": address, "standalone": True, "credentials": False, "attempts": attempts}, indent=2)+"\n")
        finally:
            if child.poll() is None:
                child.send_signal(signal.SIGINT)
            try:
                _, stderr = child.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                _, stderr = child.communicate()
                raise AssertionError("adapter did not stop")
            (evidence / "cleanup.json").write_text(json.dumps({"pid": child.pid, "exit": child.returncode, "stderr": stderr, "reaped": child.poll() is not None}, indent=2)+"\n")
            assert child.returncode == 0, stderr
    print(f"PASS: {len(attempts)} direct LAN/embedded checks")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--address", required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.address, args.evidence.resolve())
