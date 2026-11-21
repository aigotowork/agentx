#!/usr/bin/env python3
"""Finite consumer driver, composed over accepted generated-hook native fixtures."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import threading
import traceback
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/live_ui_integration"))
from live_fixture import LiveFixture, wait_for

PROTECTED = {18959, 18967, 18971, 38023, 38024}
PIN = "4f175b5d39ed02852dca6f493a336a5bb1d7f6e596f1e3c826eee7f4adff005a"


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    assert port not in PROTECTED
    return port


def launch(http):
    body = {"request_id": str(uuid.uuid4()), "generation": http.generation,
            "launch": {"tool": "qoder", "profile": None, "directory": None,
                       "prompt": "local context client acceptance", "title": "context-client"}}
    http.post("/api/v1/agents", body)
    result = http.operation(body["request_id"])["result"]["Launched"]
    assert result["failed"] == 0 and len(result["agents"]) == 1, result
    return wait_for("sendable generated-hook recipient", lambda: next((row["agent"] for row in http.agents()
                    if row["agent"]["name"] == result["agents"][0] and row["can_send"] and row["session_id"]), None))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("client", choices=["host", "demo", "demo-fences", "extension", "space-full", "demo-full", "extension-full"])
    parser.add_argument("--evidence", required=True)
    parser.add_argument("--extension")
    parser.add_argument("--candidate", default=str(ROOT / ".bin/agentx-web-context-r3"))
    parser.add_argument("--sha256", default=PIN)
    parser.add_argument("--historical-compact", action="store_true")
    args = parser.parse_args()
    if args.client in {"demo", "demo-fences", "extension"} and not args.historical_compact:
        parser.error("Compact consumer runners are superseded. Use the full-Space runners; --historical-compact is for explicit historical reproduction only.")
    if args.client.endswith("-full") and args.sha256 == PIN:
        parser.error("Corrected full-Space execution requires the owner's new --candidate and exact --sha256 handoff.")
    candidate = Path(args.candidate).resolve(strict=True)
    assert hashlib.sha256(candidate.read_bytes()).hexdigest() == args.sha256
    fixture = LiveFixture(candidate, args.evidence)
    extra = []
    parent_server = None
    failure = None
    try:
        http = fixture.prepare("primary")
        assert http.origin.port not in PROTECTED
        a, b = launch(http), launch(http)
        config = {"url": http.url, "scope": str(fixture.root / "primary/scope"),
                  "generation": http.generation, "a": a, "b": b,
                  "evidence": str(fixture.evidence), "profile": str(fixture.root / "browser-profile"),
                  "client": args.client}
        if args.extension:
            config["extension"] = str(Path(args.extension).resolve(strict=True))
        if args.client.startswith("demo"):
            port = free_port()
            config["demo"] = f"http://127.0.0.1:{port}"
            env = {**os.environ, "HOST": "127.0.0.1", "PORT": str(port), "AGENTX_ORIGIN": http.url}
            log = (fixture.evidence / "demo-server.log").open("x")
            entry = "web/context-demo/server.mjs"
            config["demoEntry"] = entry
            proc = subprocess.Popen(["node", entry], cwd=ROOT, env=env, stdout=log, stderr=log)
            extra.append((proc, log))
            fixture.manifest["consumer_server"] = {"pid": proc.pid, "url": config["demo"], "retained": False}
            from urllib.request import urlopen
            def ready():
                assert proc.poll() is None, "demo exited"
                try:
                    with urlopen(config["demo"] + "/api/config", timeout=2) as response:
                        return response.status == 200
                except OSError:
                    return False
            wait_for("isolated demo readiness", ready)
        if args.client == "space-full":
            class Parent(BaseHTTPRequestHandler):
                def do_GET(self):
                    if self.path != "/":
                        self.send_error(404)
                        return
                    data = (ROOT / "tests/context_clients/space-parent.html").read_bytes()
                    self.send_response(200)
                    self.send_header("Content-Type", "text/html; charset=utf-8")
                    self.send_header("Content-Length", str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)

                def log_message(self, *_args):
                    pass
            parent_server = ThreadingHTTPServer(("127.0.0.1", 0), Parent)
            assert parent_server.server_port not in PROTECTED
            threading.Thread(target=parent_server.serve_forever, daemon=True).start()
            config["parent"] = f"http://127.0.0.1:{parent_server.server_port}/"
        config_path = fixture.evidence / "config.json"
        config_path.write_text(json.dumps(config, indent=2) + "\n")
        fixture.persist()
        command = ["node", str(Path(__file__).with_name(args.client + ".mjs")), str(config_path)]
        if args.client in {"host", "extension", "extension-full"}:
            command = ["xvfb-run", "-a", "--server-args=-screen 0 1440x1000x24", *command]
        (fixture.evidence / "command.json").write_text(json.dumps(command))
        with (fixture.evidence / "browser.log").open("x") as log:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, timeout=240)
        assert result.returncode == 0, f"browser exited {result.returncode}; see browser.log"
        native = {"agents": http.agents(), "provider_records": fixture.provider_records(), "messages": {}, "attachments": []}
        for ref in [a, b]:
            status, body = http.call("GET", f"/api/v1/agents/{ref['name']}/messages", query={"generation": http.generation, **{k: v for k, v in ref.items() if k != "name"}})
            native["messages"][ref["name"]] = {"status": status, "body": body}
        storage = fixture.root / "primary/scope/management-attachments" / http.generation
        if storage.exists():
            for path in storage.iterdir():
                assert path.is_file() and path.parent == storage
                data = path.read_bytes()
                native["attachments"].append({"id": path.name, "path": str(path), "size": len(data), "sha256": hashlib.sha256(data).hexdigest()})
        (fixture.evidence / "native-outcome.json").write_text(json.dumps(native, indent=2) + "\n")
    except Exception:
        failure = traceback.format_exc()
        (fixture.evidence / "failure.txt").write_text(failure)
    finally:
        if parent_server:
            parent_server.shutdown()
            parent_server.server_close()
        for proc, log in reversed(extra):
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=5)
            log.close()
        fixture.close()
        shutil.rmtree(fixture.root)
        fixture.manifest["disposable_root_removed"] = not fixture.root.exists()
        fixture.persist()
    if failure:
        raise SystemExit(failure)


if __name__ == "__main__":
    main()
