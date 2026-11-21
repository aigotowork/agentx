#!/usr/bin/env python3
"""Public installed tarball/CLI against actual candidate and explicit finite failures."""

import argparse
import copy
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import shutil
import signal
import subprocess
import threading
import traceback
import uuid

from native_http import Fixture, run as native_run, wait_for
from fault_proxy import ResponseLossProxy


class WireFixture:
    """Finite HTTP responses for unreachable extremes and receipt-failure branches."""

    def __init__(self, body):
        self.body, self.requests = body, []
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                length = int(self.headers.get("Content-Length", "0"))
                assert 0 <= length <= 512 * 1024
                body = self.rfile.read(length).decode()
                owner.requests.append({"method": "POST", "path": self.path, "body": body})
                self.respond()

            def do_GET(self):
                owner.requests.append({"method": "GET", "path": self.path})
                self.respond()

            def respond(self):
                body = json.dumps(owner.body).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_args):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever)
        self.url = "http://127.0.0.1:" + str(self.server.server_port)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *_args):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)
        assert not self.thread.is_alive()


class PublicCases:
    def __init__(self, fixture, project):
        self.fixture, self.project = fixture, project.resolve(strict=True)
        self.node = shutil.which("node")
        self.bin = str(self.project / "node_modules/.bin/agentx-http")
        self.sequence, self.cases = 0, []
        self.env = fixture.environment("primary")
        # The actual Node runtime is explicit; fixture PATH still blocks paid Providers.
        self.env["PATH"] = str(Path(self.node).parent) + ":" + self.env["PATH"]

    def record(self, case, boundary="actual installed package → HTTP/native/worker, finite Qoder", **facts):
        self.cases.append({"case": case, "status": "pass", "boundary": boundary, **facts})
        (self.fixture.evidence / "package-cases.json").write_text(json.dumps(self.cases, indent=2) + "\n")

    def execute(self, label, command, stdin=None):
        self.sequence += 1
        result = subprocess.run(command, input=stdin, text=True, capture_output=True,
                                cwd=self.project, env=self.env, timeout=40)
        (self.fixture.evidence / f"consumer-{self.sequence:03d}-{label}.json").write_text(json.dumps({
            "command": command, "stdin": stdin, "exit": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}, indent=2) + "\n")
        return result

    def sdk(self, action, input=None, url=None, **options):
        request = {"baseUrl": url or self.fixture.clients["primary"].url, "action": action,
                   "input": input, **options}
        path = self.fixture.evidence / f"sdk-input-{self.sequence + 1:03d}.json"
        path.write_text(json.dumps(request) + "\n")
        result = self.execute(action, [self.node, str(self.project / "sdk-driver.mjs"), str(path)])
        assert result.returncode == 0, result.stderr
        return json.loads(result.stdout)

    def sdk_ok(self, action, input=None, **options):
        result = self.sdk(action, input, **options)
        assert result["ok"], result
        return result["value"]

    def cli(self, args, stdin=None, code=0, url=None):
        result = self.execute("cli", [self.node, self.bin, *args, "--management-url",
                                    url or self.fixture.clients["primary"].url], stdin)
        assert result.returncode == code, (args, result.returncode, result.stdout, result.stderr)
        return result

    def message(self, ref, text):
        return {"request_id": str(uuid.uuid4()), "generation": self.fixture.clients["primary"].generation,
                "agent": ref, "message": {"text": text, "attachments": [], "intent": "request",
                                         "reply_to": None, "thread": "public-package"}}

    def effect(self, ref, message_id):
        http = self.fixture.clients["primary"]
        query = {key: value for key, value in ref.items() if key != "name" and value is not None}

        def observe():
            page = http.get("/api/v1/agents/" + ref["name"] + "/messages", **query)
            return page if any(item["reply_to"] == message_id for item in page["messages"]) else None

        page = wait_for("public package finite hook reply", observe)
        assert sum(item["id"] == message_id for item in page["messages"]) == 1, page
        return page

    def actual(self, ref, launch):
        http = self.fixture.clients["primary"]
        identity = {"generation": http.generation, "agent": ref}
        descriptor = self.sdk_ok("describe")
        assert descriptor["generation"] == http.generation
        page = self.sdk_ok("listAgents", {"generation": http.generation, "limit": 1, "current_only": False})
        assert page["agents"][0]["agent"] == ref
        view = self.sdk_ok("getAgent", identity)
        assert view["agent"] == ref
        wrong_result = self.sdk("waitForSent", launch)
        assert wrong_result["error"]["code"] == "ProtocolError", wrong_result
        assert "Launched" in wrong_result["error"]["operation"]["result"]
        cli_wrong = self.cli(["operations", "wait", launch["request_id"], "--generation", http.generation, "--sent"], code=2)
        assert json.loads(cli_wrong.stderr.splitlines()[-1])["operation"] == wrong_result["error"]["operation"]
        self.record("CS-05-actual-non-Sent-result", outcome=wrong_result, cli_error=cli_wrong.stderr)
        missing = self.sdk("getOperation", {"generation": http.generation, "request_id": str(uuid.uuid4())})
        assert missing["error"]["code"] == "NotFound", missing
        self.record("CS-04-actual-absent-receipt-is-not-nonexecution", outcome=missing)
        request = self.message(ref, "installed SDK exact send")
        operation = self.sdk_ok("send", request)
        assert operation["request_id"] == request["request_id"]
        sent = self.sdk_ok("waitForSent", request, options={"timeoutMs": 15000, "pollIntervalMs": 20})
        original = self.sdk_ok("getOperation", request)
        assert original["result"]["Sent"] == sent
        self.effect(ref, sent["message_id"])
        self.record("CS-01-SDK", request=request, receipt=original)
        assert self.sdk_ok("send", request) == original
        changed = copy.deepcopy(request)
        changed["message"]["text"] = "different body same ID"
        refusal = self.sdk("send", changed)
        assert not refusal["ok"] and refusal["error"]["code"] == "Conflict", refusal
        self.record("CS-03-SDK-replay-conflict", original=original, refusal=refusal)

        assert json.loads(self.cli(["management", "show"]).stdout)["generation"] == http.generation
        assert json.loads(self.cli(["agents", "list", "--limit", "1", "--generation", http.generation]).stdout)["agents"]
        exact = ["--incarnation", ref["incarnation"]] if ref["incarnation"] else ["--stopped-event", ref["stopped_event"]]
        assert json.loads(self.cli(["agents", "show", ref["name"], *exact, "--generation", http.generation]).stdout)["agent"] == ref
        cli_request = self.message(ref, "installed CLI stdin send")
        pending = self.fixture.root / "cli-pending.json"
        cli = self.cli(["agents", "messages", "send", "--input", "-", "--pending", str(pending)], json.dumps(cli_request))
        assert json.loads(cli.stdout)["request_id"] == cli_request["request_id"]
        assert json.loads(pending.read_text()) == cli_request
        assert json.loads(cli.stderr.splitlines()[0])["request_id"] == cli_request["request_id"]
        result = json.loads(self.cli(["operations", "wait", cli_request["request_id"], "--generation", http.generation,
                                     "--sent", "--poll-seconds", "0.02", "--wait-timeout-seconds", "15"]).stdout)
        self.effect(ref, result["sent"]["message_id"])
        self.cli(["operations", "show", cli_request["request_id"], "--generation", http.generation])
        self.record("CS-01-CLI", request=cli_request, receipt=result, pending=str(pending))

        for surface in ["SDK", "CLI"]:
            lost = self.message(ref, "lost response from installed " + surface)
            with ResponseLossProxy(http.url, self.fixture.evidence / (surface.lower() + "-loss-proxy")) as proxy:
                if surface == "SDK":
                    unknown = self.sdk("send", lost, url=proxy.url)
                    assert not unknown["ok"] and unknown["error"]["code"] == "OutcomeUnknown", unknown
                    assert unknown["error"]["input"] == lost, unknown
                else:
                    pending = self.fixture.root / "cli-lost.json"
                    unknown = self.cli(["agents", "messages", "send", "--input", "-", "--pending", str(pending)],
                                       json.dumps(lost), code=3, url=proxy.url)
                    assert json.loads(unknown.stderr.splitlines()[-1])["input"] == lost
                    assert json.loads(pending.read_text()) == lost
                assert proxy.dropped.is_set()
            records = [json.loads(path.read_text()) for path in (self.fixture.evidence / (surface.lower() + "-loss-proxy")).glob("*.json")]
            assert sum(item["method"] == "POST" for item in records) == 1, records
            recovered = self.sdk_ok("getOperation", lost) if surface == "SDK" else json.loads(self.cli(
                ["operations", "show", lost["request_id"], "--generation", http.generation]).stdout)
            completed = self.sdk_ok("waitForSent", lost, options={"timeoutMs": 15000, "pollIntervalMs": 20})
            self.effect(ref, completed["message_id"])
            self.record("CS-04-" + surface, input=lost, recovered=recovered, sent=completed)

        # Ending local observation of a completed actual receipt still cannot cancel native work.
        stopped = self.sdk("waitForOperation", request, cancelBeforeWait=True)
        assert stopped["error"]["code"] == "Canceled", stopped
        assert self.sdk_ok("getOperation", request) == original
        self.record("CS-05-actual-local-cancel", stop=stopped, receipt=original)

        stale = {**identity, "generation": str(uuid.uuid4())}
        refusal = self.sdk("getAgent", stale)
        assert refusal["error"]["code"] == "StaleReference", refusal
        self.cli(["agents", "show", ref["name"], *exact, "--generation", stale["generation"]], code=1)
        # Real lifecycle replacement creates a new exact incarnation, not a client name fallback.
        stop = {"request_id": str(uuid.uuid4()), **identity}
        http.post("/api/v1/agents/" + ref["name"] + "/stop", stop)
        http.operation(stop["request_id"])
        history = wait_for("owned stopped reference", lambda: next((v["agent"] for v in http.agents()
                           if v["agent"]["name"] == ref["name"] and v["agent"]["stopped_event"]), None))
        resume = {"request_id": str(uuid.uuid4()), "generation": http.generation, "agent": history}
        http.post("/api/v1/agents/" + ref["name"] + "/resume", resume)
        resumed = http.operation(resume["request_id"])
        successor_name = resumed["result"]["Launched"]["agents"][0]
        successor = wait_for("owned successor", lambda: next((v["agent"] for v in http.agents()
                             if v["agent"]["name"] == successor_name and v["can_send"]), None))
        assert successor != ref
        stale_send = self.message(ref, "must never reach successor")
        assert self.sdk("send", stale_send)["error"]["code"] == "StaleReference"
        cli_stale = self.cli(["agents", "messages", "send", "--input", "-", "--pending", str(self.fixture.root / "cli-stale.json")],
                             json.dumps(stale_send), code=1)
        assert json.loads(cli_stale.stderr.splitlines()[-1])["code"] == "StaleReference"
        assert self.sdk_ok("send", request) == original  # replay precedes current target eligibility
        self.record("CS-02-actual-stale-reference-generation", old=ref, successor=successor,
                    same_name=successor["name"] == ref["name"], wrong_generation=refusal)
        self.record("CS-03-actual-replay-after-retirement", receipt=original)
        return view, original

    def finite(self, view, original):
        boundary = "installed public package over finite HTTP; no native transition or extreme-value supply claim"
        identity = {key: original[key] for key in ["generation", "request_id"]}
        for value in [0, 2**53 - 1, 2**53, 2**64 - 1]:
            data = copy.deepcopy(view)
            data["pending_messages"] = value
            data["request_error"] = {"turn_id": "001234", "source": "acceptance", "reason": "finite wire",
                                     "error_since": -(2**63), "last_error_at": 2**63 - 1,
                                     "consecutive_failures": 2**64 - 1}
            with WireFixture(data) as wire:
                result = self.sdk("getAgent", {"generation": identity["generation"], "agent": view["agent"]}, url=wire.url)
            assert result["ok"] and result["value"]["pending_messages"] == value, result
            types = {item["path"]: item for item in result["integerTypes"]}
            assert types["$.pending_messages"]["value"] == str(value), result
            assert types["$.request_error.error_since"]["value"] == str(-(2**63))
            assert result["value"]["request_error"]["turn_id"] == "001234"
            self.record("CS-06-finite-precision-" + str(value), boundary, response=data, decoded=result)

        for status, result, expected in [
            ("Succeeded", {"Launched": {"agents": [], "failed": 0, "errors": []}}, "ProtocolError"),
            ("Failed", None, "Failed"), ("Unknown", None, "OutcomeUnknown")]:
            receipt = {**original, "status": status, "result": result, "error": "finite failure" if status == "Failed" else None}
            with WireFixture(receipt) as wire:
                outcome = self.sdk("waitForSent", identity, url=wire.url)
            assert not outcome["ok"] and outcome["error"]["code"] == expected, outcome
            assert outcome["error"]["operation"] == receipt
            with WireFixture(receipt) as wire:
                cli = self.cli(["operations", "wait", identity["request_id"], "--generation", identity["generation"], "--sent"],
                               code={"ProtocolError": 2, "Failed": 1, "OutcomeUnknown": 3}[expected], url=wire.url)
                assert json.loads(cli.stderr.splitlines()[-1])["operation"] == receipt
            self.record("CS-05-finite-" + status, boundary, response=receipt, outcome=outcome)
        wrong = {**original, "request_id": str(uuid.uuid4())}
        with WireFixture(wrong) as wire:
            outcome = self.sdk("getOperation", identity, url=wire.url)
        assert outcome["error"]["code"] == "ProtocolError", outcome
        self.record("CS-05-finite-wrong-identity", boundary, outcome=outcome)
        pending = {**original, "status": "Running", "result": None, "error": None}
        with WireFixture(pending) as wire:
            outcome = self.sdk("waitForOperation", identity, url=wire.url, options={"timeoutMs": 100, "pollIntervalMs": 10})
            assert outcome["error"]["code"] == "DeadlineExceeded", outcome
            assert wire.requests and all(item["method"] == "GET" for item in wire.requests)
            wire.body = original
            assert self.sdk_ok("getOperation", identity, url=wire.url) == original
            requests = wire.requests
        self.record("CS-05-finite-deadline-recovery", boundary, outcome=outcome, requests=requests)
        with WireFixture(pending) as wire:
            cli = self.cli(["operations", "wait", identity["request_id"], "--generation", identity["generation"],
                            "--poll-seconds", "0.01", "--wait-timeout-seconds", "0.1"], code=124, url=wire.url)
            assert json.loads(cli.stderr.splitlines()[-1])["identity"] == identity
            command = [self.node, self.bin, "operations", "wait", identity["request_id"], "--generation", identity["generation"],
                       "--management-url", wire.url, "--poll-seconds", "0.02", "--wait-timeout-seconds", "20"]
            prior_requests = len(wire.requests)
            process = subprocess.Popen(command, cwd=self.project, env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                wait_for("installed CLI observed first receipt and began second poll", lambda: len(wire.requests) >= prior_requests + 2, timeout=10)
                process.send_signal(signal.SIGINT)
                stdout, stderr = process.communicate(timeout=10)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.communicate()
            cancellation = {"command": command, "exit": process.returncode, "stdout": stdout, "stderr": stderr}
            error = json.loads(stderr.splitlines()[-1])
            assert process.returncode == 130 and error["identity"] == identity, cancellation
            assert error["lastOperation"] == pending, cancellation
        self.record("CS-05-finite-CLI-local-stop", boundary, cancellation=cancellation)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["candidate", "spec", "project", "evidence"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    fixture = Fixture(args.candidate, args.evidence, root=args.project.parent / "agentx-management-system")
    result = {"status": "failed", "package_project": str(args.project)}
    try:
        ref, launch = native_run(fixture, args.spec)
        cases = PublicCases(fixture, args.project)
        for flag in ["--help", "--version"]:
            output = cases.execute("offline", [cases.node, cases.bin, flag])
            assert output.returncode == 0 and output.stdout and not output.stderr, output
        cases.record("CS-10-installed-offline-entry", "installed CLI artifact; no service URL supplied")
        view, original = cases.actual(ref, launch)
        cases.finite(view, original)
        result["status"] = "passed"
    except Exception:
        result["failure"] = traceback.format_exc()
        raise
    finally:
        try:
            fixture.close()
        except Exception:
            result["status"] = "failed"
            result["cleanup_failure"] = traceback.format_exc()
            raise
        finally:
            (fixture.evidence / "package-result.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
