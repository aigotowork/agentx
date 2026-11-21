#!/usr/bin/python3
"""Independent public HTTP and actual iframe acceptance; all resources disposable."""
import argparse
import base64
import copy
import functools
import hashlib
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from http.client import HTTPConnection
import json
from pathlib import Path
import shutil
import subprocess
import sys
import threading
import traceback
from urllib.parse import urlencode, urlsplit
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "live_ui_integration"))
from live_fixture import LiveFixture, wait_for

PROTECTED = {18959, 18967, 18971, 38023}
PNG = base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=")
def uid(): return str(uuid.uuid4())
def digest(data): return hashlib.sha256(data).hexdigest()

class Cases:
    def __init__(self, fixture):
        self.fixture, self.ledger = fixture, []
        self.http = fixture.prepare("primary")
        self.other = fixture.prepare("other")
        for http in [self.http, self.other]:
            assert http.origin.port not in PROTECTED
        self.a, self.b = self.launch("context-a"), self.launch("context-b")

    def record(self, case, **facts):
        self.ledger.append({"case": case, "status": "pass", **facts})
        (self.fixture.evidence / "native-cases.json").write_text(json.dumps(self.ledger, indent=2) + "\n")
        print(case + " passed", flush=True)

    def launch(self, title):
        body = {"request_id": uid(), "generation": self.http.generation,
                "launch": {"tool": "qoder", "profile": None, "directory": None,
                           "prompt": "local web context acceptance", "title": title}}
        self.http.post("/api/v1/agents", body)
        outcome = self.http.operation(body["request_id"])["result"]["Launched"]
        assert outcome["failed"] == 0 and len(outcome["agents"]) == 1, outcome
        return wait_for("exact sendable Provider", lambda: next((v["agent"] for v in self.http.agents()
            if v["agent"]["name"] == outcome["agents"][0] and v["session_id"] and v["can_send"]), None))

    def binary(self, method, path, data=None, query=None, expected=200, http=None):
        http = http or self.http
        if query: path += "?" + urlencode(query)
        # Binary evidence stays separate from the legacy text-only Http decoder.
        connection = HTTPConnection(http.origin.hostname, http.origin.port, timeout=25)
        try:
            connection.request(method, path, body=data, headers={"Content-Type": "application/octet-stream"})
            response = connection.getresponse()
            body, headers, status = response.read(), dict(response.getheaders()), response.status
            row = {"method": method, "path": path, "input_size": len(data or b""), "input_sha256": digest(data or b""),
                   "status": status, "headers": headers, "size": len(body), "sha256": digest(body)}
            if headers.get("content-type", "").startswith("application/json"):
                row["json"] = json.loads(body)
            log = self.fixture.evidence / "binary-http.jsonl"
            with log.open("a") as stream: stream.write(json.dumps(row) + "\n")
            assert status == expected, row
            return body, headers
        finally: connection.close()

    def upload(self, name, media, data, expected=201, **query):
        body, _ = self.binary("POST", "/api/v1/attachments", data,
            {"generation": self.http.generation, "name": name, "media_type": media, **query}, expected)
        return json.loads(body)

    def messages(self, reference=None):
        ref = reference or self.a
        return self.http.get(f"/api/v1/agents/{ref['name']}/messages", **{k:v for k,v in ref.items() if k != 'name'})["messages"]

    def send(self, text, attachments=None, expected=202, ref=None, body=None):
        ref = ref or self.a
        body = body or {"request_id": uid(), "generation": self.http.generation, "agent": ref,
                       "message": {"text": text, "intent": "request", "reply_to": None, "thread": None,
                                   **({"attachments": attachments} if attachments is not None else {})}}
        status, result = self.http.call("POST", f"/api/v1/agents/{ref['name']}/messages", body)
        assert status == expected, (status, result)
        return (self.http.operation(body["request_id"]) if status == 202 else result), body

    def native(self):
        image = self.upload("屏幕.png", "image/png", PNG)
        file = self.upload("../../same.bin", "application/octet-stream", b"durable\x00file\xff")
        duplicate = self.upload("../../same.bin", "application/octet-stream", b"second")
        zero = self.upload("empty", "", b"")
        assert file["id"] != duplicate["id"] and zero["size"] == 0
        self.upload("x" * 255, "text/plain", b"name-boundary")
        self.upload("x" * 256, "text/plain", b"bad", expected=400)
        self.upload("bad\nname", "text/plain", b"bad", expected=400)
        self.upload("file", "text/plain;injected", b"bad", expected=400)
        boundary = self.upload("limit.bin", "application/octet-stream", bytes(10*1024*1024))
        self.upload("too-big", "application/octet-stream", bytes(10*1024*1024+1), expected=400)
        self.upload("old", "text/plain", b"bad", expected=409, generation=uid())
        storage = self.fixture.root / "primary/scope/management-attachments" / self.http.generation
        for item, data in [(image, PNG), (file, b"durable\x00file\xff"), (duplicate, b"second"), (zero, b"")]:
            path = (storage / item["id"]).resolve(strict=True)
            assert path.parent == storage and path.is_relative_to(self.fixture.root)
            assert path.read_bytes() == data
            downloaded, _ = self.binary("GET", item["url"])
            assert downloaded == data
        for media in ["text/html", "image/svg+xml"]:
            item = self.upload("unsafe.svg", media, b"<svg>test</svg>")
            _, headers = self.binary("GET", item["url"])
            assert headers["content-type"] == "application/octet-stream" and headers["content-disposition"].startswith("attachment")
        self.binary("GET", image["url"], expected=409, http=self.other)
        self.binary("GET", "/api/v1/attachments/" + image["id"], query={"generation": self.other.generation}, expected=400, http=self.other)
        self.binary("GET", "/api/v1/attachments/" + uid(), query={"generation": self.http.generation}, expected=400)
        self.record("WC14-native-WC11-limits", image=image, file=file, zero=zero, boundary=boundary, storage=str(storage))
        self.deliveries(image, file, zero, storage)

    def continue_delivery(self):
        image = self.upload("屏幕.png", "image/png", PNG)
        file = self.upload("../../same.bin", "application/octet-stream", b"durable\x00file\xff")
        zero = self.upload("empty", "", b"")
        self.binary("GET", "/api/v1/attachments/" + image["id"], query={"generation": self.other.generation}, expected=400, http=self.other)
        self.binary("GET", "/api/v1/attachments/" + uid(), query={"generation": self.http.generation}, expected=400)
        self.record("WC12-foreign-ID-WC14-unknown-ID", basis="Earlier upload boundary evidence retained in r1-a; 400 InvalidInput is the native unknown-reference contract, not invented 404.")
        self.deliveries(image, file, zero, self.fixture.root / "primary/scope/management-attachments" / self.http.generation)

    def deliveries(self, image, file, zero, storage):
        sent, body = self.send("native durable context", [image["id"], file["id"]])
        event_id = sent["result"]["Sent"]["message_id"]
        event = next(row for row in self.messages() if row["id"] == event_id)
        assert [item["id"] for item in event["attachments"]] == [image["id"], file["id"]], event
        paths = [str(storage / item["id"]) for item in [image,file]]
        assert all(path in event["text"] for path in paths), event
        wait_for("actual generated hook contains full readable references", lambda: any(
            all(path in json.dumps(row, ensure_ascii=False) for path in paths)
            for row in self.fixture.provider_records() if row["kind"] == "hook"))
        replay, _ = self.send("", body=body)
        assert replay == sent
        changed = copy.deepcopy(body); changed["message"]["attachments"] = [image["id"]]
        self.send("", body=changed, expected=409)
        self.send("", [uid()], expected=400)
        self.send("", [image["id"]]*9, expected=400)
        only, _ = self.send("", [zero["id"]])
        plain, _ = self.send("ordinary omitted-attachments text")
        assert sum(row["id"] == event_id for row in self.messages()) == 1
        self.record("WC06-WC07-WC13-native", sent=sent, event=event, replay=replay, attachment_only=only, text_only=plain)
        self.fixture.stop_daemon("primary")
        self.http = self.fixture.start("primary")
        assert self.http.generation == body["generation"]
        restored = next(row for row in self.messages() if row["id"] == event_id)
        assert restored["attachments"] == event["attachments"]
        assert self.binary("GET", image["url"])[0] == PNG
        self.record("WC08-native-restart", restored=restored)
        self.image, self.file = image, file

    def browser(self, start=None):
        if start:
            image = self.upload("截屏.png", "image/png", PNG)
            file = self.upload("picker.bin", "application/octet-stream", b"picker bytes")
            self.send("retained history prerequisite", [image["id"], file["id"]])
        directory = Path(__file__).resolve().parents[2] / ".docs/agent-management/examples"
        server = ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(SimpleHTTPRequestHandler, directory=str(directory)))
        assert server.server_port not in PROTECTED
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        config = {"url": self.http.url, "parent": f"http://127.0.0.1:{server.server_port}/context-parent.html",
                  "scope": str(self.fixture.root / "primary/scope"), "generation": self.http.generation,
                  "a": self.a, "b": self.b, "evidence": str(self.fixture.evidence / "browser"),
                  "png": base64.b64encode(PNG).decode(), "start": start}
        path = self.fixture.evidence / "browser-input.json"; path.write_text(json.dumps(config, indent=2))
        try:
            result = subprocess.run(["node", str(Path(__file__).with_name("browser.mjs")), str(path)],
                                    capture_output=True, text=True, timeout=350)
            (self.fixture.evidence / "browser.stdout").write_text(result.stdout)
            (self.fixture.evidence / "browser.stderr").write_text(result.stderr)
            assert result.returncode == 0, (result.stdout[-1000:], result.stderr[-2000:])
        finally:
            server.shutdown(); server.server_close(); thread.join(timeout=5)

    def generation_fence(self):
        missing = self.upload("missing.bin", "text/plain", b"owned disposable bytes")
        path = self.fixture.root / "primary/scope/management-attachments" / self.http.generation / missing["id"]
        assert path.is_relative_to(self.fixture.root) and path.is_file()
        path.unlink()
        body = {"request_id": uid(), "generation": self.http.generation, "agent": self.a,
                "message": {"text": "must not send missing host bytes", "attachments": [missing["id"]], "intent": "request", "reply_to": None, "thread": None}}
        self.http.post(f"/api/v1/agents/{self.a['name']}/messages", body)
        failed = self.http.operation(body["request_id"], success=False)
        assert failed["status"] == "Failed" and "unavailable" in failed["error"], failed
        assert not any("must not send missing host bytes" in row["text"] for row in self.messages())
        self.record("WC14-missing-bytes", operation=failed, no_message_effect=True)
        before = self.http.generation
        self.fixture.cli("primary", "acceptance-reset", ["reset"])
        descriptor = self.http.get("/api/v1/management")
        assert descriptor["generation"] != before
        self.upload("stale-generation.bin", "text/plain", b"reject", expected=409)
        status, result = self.http.call("POST", f"/api/v1/agents/{self.a['name']}/messages", {
            "request_id": uid(), "generation": before, "agent": self.a,
            "message": {"text": "stale generation cannot mutate", "attachments": [], "intent": "request", "reply_to": None, "thread": None}})
        assert status == 409, (status, result)
        self.record("WC12-generation-fence", previous=before, current=descriptor["generation"], rejected=result)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", required=True); parser.add_argument("--sha256", required=True)
    parser.add_argument("--evidence", required=True); parser.add_argument("--browser-only", action="store_true")
    parser.add_argument("--native-start", choices=["delivery"])
    parser.add_argument("--browser-start", choices=["remaining", "recovery", "narrow-recovery"])
    parser.add_argument("--fences-only", action="store_true")
    args = parser.parse_args()
    candidate = Path(args.candidate).resolve(strict=True)
    assert digest(candidate.read_bytes()) == args.sha256, "candidate hash differs"
    fixture = LiveFixture(candidate, args.evidence)
    failure = None
    try:
        for suffix in [".source.json", ".receipt.json"]:
            shutil.copyfile(str(candidate)+suffix, fixture.evidence / ("candidate"+suffix))
        cases = Cases(fixture)
        if not args.browser_only and not args.fences_only:
            if args.native_start == "delivery": cases.continue_delivery()
            else: cases.native()
        if not args.fences_only: cases.browser(args.browser_start)
        if not args.browser_only: cases.generation_fence()
    except Exception:
        failure = traceback.format_exc(); (fixture.evidence / "failure.txt").write_text(failure)
    finally:
        try: fixture.close()
        except Exception: failure = (failure or "") + traceback.format_exc()
        if not fixture.manifest.get("cleanup_errors"):
            storage = fixture.root / "primary/scope/management-attachments"
            if storage.exists(): shutil.copytree(storage, fixture.evidence / "disk-attachments")
            shutil.rmtree(fixture.root)
            fixture.manifest["fixture_root_removed"] = not fixture.root.exists()
            fixture.persist()
        (fixture.evidence / "result.json").write_text(json.dumps({"status": "fail" if failure else "pass", "failure": failure,
            "limits": "Local generated-hook Provider; no paid model, OS clipboard or extension runtime evidence."}, indent=2))
    if failure: print(failure, file=sys.stderr); return 1
    return 0

if __name__ == "__main__": sys.exit(main())
