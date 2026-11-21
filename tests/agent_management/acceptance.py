#!/usr/bin/python3
"""Public-contract acceptance against an explicit, integration-owned candidate."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import traceback
import uuid

from fixture import Fixture, wait_for


def request_id():
    return str(uuid.uuid4())


def target(http, reference):
    return {"request_id": request_id(), "generation": http.generation, "agent": reference}


def exact_query(reference):
    return {key: value for key, value in reference.items() if key != "name" and value is not None}


def public_shape(value):
    if isinstance(value, dict):
        forbidden = {"pid", "environment", "env", "ticket", "claim_ticket", "invocation", "process_id", "argv"}
        assert not forbidden.intersection(value), ("internal capability leaked", value)
        for key, child in value.items():
            if key in {"generation", "incarnation", "stopped_event", "request_id", "message_id", "reply_to", "next_after", "position"} and child is not None:
                assert isinstance(child, str), (key, child)
            public_shape(child)
    elif isinstance(value, list):
        for child in value:
            public_shape(child)


class Cases:
    def __init__(self, fixture):
        self.fixture, self.http, self.steps = fixture, None, []

    def record(self, name, **facts):
        public_shape(facts)
        self.steps.append({"case": name, "status": "pass", **facts})
        (self.fixture.evidence / "cases.json").write_text(json.dumps(self.steps, indent=2) + "\n")
        print(name + " passed", flush=True)

    def reject(self, method, path, expected=(400,), **arguments):
        status, response = self.http.call(method, path, **arguments)
        assert status in expected, (method, path, status, response)
        assert isinstance(response, dict) and isinstance(response.get("error"), dict), response
        assert isinstance(response["error"].get("code"), str) and isinstance(response["error"].get("message"), str)
        public_shape(response)
        return response

    def launch_payload(self, title):
        return {"request_id": request_id(), "generation": self.http.generation,
                "launch": {"tool": "qoder", "profile": None, "directory": None,
                           "prompt": "management fixture initial prompt", "title": title}}

    def submit_target(self, action, reference, success=True):
        payload = target(self.http, reference)
        self.http.post(f"/api/v1/agents/{reference['name']}/{action}", payload)
        return self.http.operation(payload["request_id"], success=success), payload

    def live(self, name):
        def ready():
            return next((view for view in self.http.agents() if view["agent"]["name"] == name
                         and view["agent"].get("incarnation") and view["session_id"] and view["can_send"]), None)
        return wait_for("live Provider with bound session", ready)

    def messages(self, reference, **query):
        page = self.http.get(f"/api/v1/agents/{reference['name']}/messages", **exact_query(reference), **query)
        public_shape(page)
        assert isinstance(page["has_more"], bool)
        assert all(isinstance(message["id"], str) for message in page["messages"])
        return page

    def startup(self):
        self.http = self.fixture.prepare("primary")
        descriptor = self.http.get("/api/v1/management")
        assert "qoder" in descriptor["tools"] and descriptor["max_page"] == 100 and descriptor["max_text"] == 65536
        assert self.http.agents() == []
        status, html = self.http.call("GET", "/")
        assert status == 200 and isinstance(html, str) and "<html" in html, (status, html)
        assets = re.findall(r'(?:src|href)=["\'](/[^"\']+\.(?:js|css))["\']', html)
        assert assets, "embedded entry lacks JS/CSS"
        for path in assets:
            code, body = self.http.call("GET", path)
            assert code == 200 and body, path
        self.fixture.cli("primary", "daemon-status", ["daemon", "status"])
        self.record("M01", descriptor=descriptor, embedded_assets=assets)

    def lifecycle(self):
        launch = self.launch_payload("http-lifecycle")
        admitted = self.http.post("/api/v1/agents", launch)
        operation = self.http.operation(launch["request_id"])
        outcome = operation["result"]["Launched"]
        assert outcome["failed"] == 0 and len(outcome["agents"]) == 1, operation
        view = self.live(outcome["agents"][0])
        reference = view["agent"]
        assert view["tool"] == "qoder" and view["can_stop"] and view["has_transcript"]
        assert view["directory"] == str(self.fixture.root / "primary/project") and view["title"] == "http-lifecycle"
        self.record("M02", admission=admitted, operation=operation, agent=view)

        message = target(self.http, reference)
        message["message"] = {"text": "management public request one", "intent": "request", "reply_to": None, "thread": "acceptance"}
        path = f"/api/v1/agents/{reference['name']}/messages"
        self.http.post(path, message)
        sent = self.http.operation(message["request_id"])
        message_id = sent["result"]["Sent"]["message_id"]
        replay = self.http.post(path, message)
        assert replay == sent, (replay, sent)
        changed = copy.deepcopy(message)
        changed["message"]["text"] = "changed body must not be admitted"
        conflict = self.reject("POST", path, (409,), payload=changed)

        def answered():
            page = self.messages(reference)
            return page if any(item["reply_to"] == message_id and item["from"] == reference["name"] for item in page["messages"]) else None

        page = wait_for("Provider reply through generated hooks", answered)
        originals = [item for item in page["messages"] if item["id"] == message_id]
        assert len(originals) == 1 and originals[0]["from"] == "bigboss" and originals[0]["text"] == message["message"]["text"]
        replies = [item for item in page["messages"] if item["reply_to"] == message_id]
        assert len(replies) == 1 and "fixture reply" in replies[0]["text"]
        self.record("M03", sent=sent, replay=replay, conflict=conflict, messages=page)

        transcript, _ = self.submit_target("transcript", reference)
        transcript_page = transcript["result"]["Transcript"]
        assert transcript_page["available"] and transcript_page["exchanges"], transcript
        assert any(message["message"]["text"] in exchange["user"] for exchange in transcript_page["exchanges"]), transcript
        self.record("M03-transcript", operation=transcript)
        self.reference, self.launch_receipt, self.message_receipt = reference, operation, sent

    def duplicate_and_disconnect(self):
        launch = self.launch_payload("duplicate-launch")
        with ThreadPoolExecutor(max_workers=2) as executor:
            attempts = list(executor.map(lambda _: self.http.post("/api/v1/agents", launch), range(2)))
        complete = self.http.operation(launch["request_id"])
        assert complete["result"]["Launched"]["failed"] == 0
        name = complete["result"]["Launched"]["agents"][0]
        reference = self.live(name)["agent"]
        assert len([item for item in self.http.agents() if item["title"] == "duplicate-launch"]) == 1
        changed = copy.deepcopy(launch)
        changed["launch"]["title"] = "conflicting-launch"
        self.reject("POST", "/api/v1/agents", (409,), payload=changed)

        lost = target(self.http, reference)
        lost["message"] = {"text": "lost HTTP response fixture", "intent": "request", "reply_to": None, "thread": None}
        path = f"/api/v1/agents/{name}/messages"
        self.http.call("POST", path, lost, disconnect=True)
        found = wait_for("lost reply operation lookup", lambda: self._find_operation(lost["request_id"]))
        if found["status"] not in ["Succeeded", "Failed", "Unknown"]:
            found = self.http.operation(lost["request_id"])
        assert found["status"] == "Succeeded", found
        assert self.http.post(path, lost) == found
        message_id = found["result"]["Sent"]["message_id"]
        wait_for("lost response message effect", lambda: any(item["id"] == message_id for item in self.messages(reference)["messages"]))
        self.record("M04", duplicate_admissions=attempts, completed=complete, lost_response_receipt=found)
        self.submit_target("stop", reference)

    def _find_operation(self, identity):
        code, value = self.http.call("GET", "/api/v1/operations/" + identity, query={"generation": self.http.generation})
        if code == 404:
            return None
        assert code == 200, (code, value)
        return value

    def bounds(self):
        generation = self.http.generation
        base = self.launch_payload("reject-invalid")
        cases = [({**base, "unexpected": True}, None),
                 ({**base, "request_id": "invalid"}, None),
                 ({**base, "launch": {**base["launch"], "argv": ["arbitrary"]}}, None),
                 ({**base, "launch": {**base["launch"], "environment": {"X": "Y"}}}, None),
                 ({**base, "launch": {**base["launch"], "prompt": "x" * 65537}}, None),
                 ({**base, "launch": {**base["launch"], "prompt": "界" * 21846}}, None),
                 (None, '{"request_id":"one","request_id":"two","generation":"' + generation + '","launch":{}}'),
                 (None, "{malformed"), (None, "["), (None, "x" * 131073)]
        for payload, raw in cases:
            self.reject("POST", "/api/v1/agents", payload=payload, raw=raw)
        # Public browser metadata and DNS aliases must preserve the selected
        # native generation, without using a mutation as an admission probe.
        for headers in [{"Origin": "https://foreign.invalid"}, {"Origin": "null"},
                        {"Sec-Fetch-Site": "cross-site"}, {"Host": "foreign.invalid"}]:
            status, descriptor = self.http.call("GET", "/api/v1/management", headers=headers)
            assert status == 200 and descriptor["generation"] == generation, (status, descriptor)
        self.reject("GET", "/api/v1/management", (403,), headers={"Host": "invalid host"})
        self.reject("POST", "/api/v1/agents", payload=base, headers={"Content-Type": "text/plain"})
        self.reject("GET", "/api/v1/agents", query={})
        for query in [{"limit": 0}, {"limit": 101}, {"limit": -1}, {"after": "x" * 513}, {"unknown": "field"}]:
            self.reject("GET", "/api/v1/agents", query={"generation": generation, **query})
        self.reject("GET", "/api/v1/agents?generation=" + generation + "&generation=" + generation)
        self.reject("POST", "/api/v1/native", (404,), payload={"Request": "arbitrary"})
        self.reject("GET", "/api/v1/operations/" + request_id(), (404,), query={"generation": generation})
        reference = self.reference
        path = f"/api/v1/agents/{reference['name']}"
        for invalid_ref in [{"name": reference["name"], "incarnation": None, "stopped_event": None},
                            {**reference, "stopped_event": "1"}, {**reference, "incarnation": "bad"}]:
            self.reject("POST", path + "/transcript", payload=target(self.http, invalid_ref))
        wrong_path = target(self.http, reference)
        self.reject("POST", "/api/v1/agents/wrong/transcript", payload=wrong_path)
        for text in ["", " ", "x" * 65537, "has\0control"]:
            body = target(self.http, reference)
            body["message"] = {"text": text, "intent": "request", "reply_to": None, "thread": None}
            self.reject("POST", path + "/messages", payload=body)
        boundary = target(self.http, reference)
        boundary["message"] = {"text": "界" * 21845 + "x", "intent": "request", "reply_to": None, "thread": None}
        assert len(boundary["message"]["text"].encode()) == 65536
        self.http.post(path + "/messages", boundary)
        boundary_result = self.http.operation(boundary["request_id"])
        assert boundary_result["status"] == "Succeeded", boundary_result
        self.reject("GET", path + "/messages", query={"generation": generation, **exact_query(reference), "after": -1})
        assert not any(item["title"] in ["reject-invalid", "conflicting-launch"] for item in self.http.agents())
        self.record("M07", accepted_utf8_bytes=65536, rejected_groups=["JSON", "fields", "text bytes", "body", "malformed Host", "MIME", "query", "references", "native proxy"])

    def pagination(self):
        seen, cursor = [], None
        for _ in range(20):
            page = self.http.get("/api/v1/agents", limit=1, after=cursor)
            public_shape(page)
            assert len(page["agents"]) <= 1
            seen.extend(view["agent"] for view in page["agents"])
            if page["next"] is None:
                break
            assert isinstance(page["next"], str) and page["next"] != cursor
            cursor = page["next"]
        else:
            raise AssertionError("agent pagination does not terminate")
        assert len(seen) >= 2 and len({json.dumps(ref, sort_keys=True) for ref in seen}) == len(seen)
        messages, cursor = [], "0"
        for _ in range(30):
            page = self.messages(self.reference, limit=1, after=cursor)
            assert len(page["messages"]) <= 1
            messages.extend(page["messages"])
            if not page["has_more"]:
                break
            assert int(page["next_after"]) > int(cursor)
            cursor = page["next_after"]
        else:
            raise AssertionError("message pagination does not terminate")
        assert len(messages) >= 2 and len({message["id"] for message in messages}) == len(messages)
        self.record("M08", agent_references=seen, paginated_messages=messages)

    def stop_history_resume(self):
        reference = self.reference
        stop, payload = self.submit_target("stop", reference)
        assert stop["result"]["Stopped"]["name"] == reference["name"]
        assert self.http.post(f"/api/v1/agents/{reference['name']}/stop", payload) == stop
        stopped = wait_for("stopped exact history", lambda: next((view for view in self.http.agents()
                           if view["agent"]["name"] == reference["name"] and view["agent"].get("stopped_event")), None))
        history = stopped["agent"]
        assert stopped["status"] == "stopped" and not stopped["can_send"] and not stopped["can_stop"] and stopped["can_resume"]
        self.reject("GET", f"/api/v1/agents/{reference['name']}", (409,), query={"generation": self.http.generation, **exact_query(reference)})
        self.reject("POST", f"/api/v1/agents/{reference['name']}/stop", (409,), payload=target(self.http, reference))
        old_messages = self.messages(history)
        assert any(item["text"] == "management public request one" for item in old_messages["messages"])
        transcript, _ = self.submit_target("transcript", history)
        assert transcript["result"]["Transcript"]["available"]
        resumed, resume_payload = self.submit_target("resume", history)
        names = resumed["result"]["Launched"]["agents"]
        assert len(names) == 1 and resumed["result"]["Launched"]["failed"] == 0, resumed
        successor = self.live(names[0])
        assert successor["agent"] != reference and successor["session_id"] == stopped["session_id"], (successor, stopped)
        assert self.http.post(f"/api/v1/agents/{reference['name']}/resume", resume_payload) == resumed
        stale = target(self.http, reference)
        stale["message"] = {"text": "must not reach successor", "intent": "request", "reply_to": None, "thread": None}
        self.reject("POST", f"/api/v1/agents/{reference['name']}/messages", (409,), payload=stale)
        old_detail = self.http.get(f"/api/v1/agents/{history['name']}", **exact_query(history))
        assert old_detail["agent"] == history and old_detail["status"] == "stopped"
        successor_body = target(self.http, successor["agent"])
        successor_body["message"] = {"text": "successor-only message must not enter stopped history", "intent": "request", "reply_to": None, "thread": None}
        successor_path = f"/api/v1/agents/{successor['agent']['name']}/messages"
        self.http.post(successor_path, successor_body)
        successor_receipt = self.http.operation(successor_body["request_id"])
        successor_message_id = successor_receipt["result"]["Sent"]["message_id"]
        successor_messages = wait_for("successor reply", lambda: self._reply_page(successor["agent"], successor_message_id))
        previous_after_successor = self.messages(history)
        assert not any(item["id"] == successor_message_id or item["reply_to"] == successor_message_id
                       or item["text"] == successor_body["message"]["text"] for item in previous_after_successor["messages"]), previous_after_successor
        old_ids = {item["id"] for item in old_messages["messages"]}
        assert old_ids == {item["id"] for item in previous_after_successor["messages"]}, (old_messages, previous_after_successor)
        assert not any(item["id"] in old_ids for item in successor_messages["messages"]), successor_messages
        self.record("M05", stop=stop, stopped=stopped, resume=resumed, successor=successor,
                    history_after_resume=old_detail, successor_messages=successor_messages,
                    stopped_messages_after_successor=previous_after_successor)
        self.submit_target("stop", successor["agent"])
        self.history = history

    def _reply_page(self, reference, message_id):
        page = self.messages(reference)
        return page if any(item["reply_to"] == message_id for item in page["messages"]) else None

    def isolation_restart_reset(self):
        other = self.fixture.prepare("other")
        assert other.generation != self.http.generation and other.agents() == []
        code, response = other.call("GET", "/api/v1/operations/" + self.launch_receipt["request_id"], query={"generation": other.generation})
        assert code == 404, response
        stale_generation = self.launch_payload("wrong-generation")
        stale_generation["generation"] = other.generation
        self.reject("POST", "/api/v1/agents", (409,), payload=stale_generation)
        self.fixture.stop_daemon("other")
        old_generation = self.http.generation
        self.fixture.stop_daemon("primary")
        self.http = self.fixture.start("primary")
        assert self.http.generation == old_generation
        assert self.http.get("/api/v1/operations/" + self.launch_receipt["request_id"]) == self.launch_receipt
        assert self.http.get(f"/api/v1/agents/{self.history['name']}", **exact_query(self.history))["status"] == "stopped"
        self.record("M09", retained_receipt=self.launch_receipt, generation=old_generation)
        self.fixture.cli("primary", "reset", ["reset"])
        descriptor = wait_for("changed database generation", lambda: self._changed_generation(old_generation))
        self.reject("GET", "/api/v1/agents", (409,), query={"generation": old_generation})
        self.reject("GET", "/api/v1/operations/" + self.launch_receipt["request_id"], (409,), query={"generation": old_generation})
        self.reject("POST", "/api/v1/agents", (409,), payload={**stale_generation, "generation": old_generation})
        self.http.generation = descriptor["generation"]
        assert self.http.agents() == []
        self.record("M06", old_generation=old_generation, descriptor=descriptor, other_generation=other.generation)

    def _changed_generation(self, old):
        descriptor = self.http.get("/api/v1/management")
        return descriptor if descriptor["generation"] != old else None

    def cli_enable_reconfigure_replay(self):
        primary = self.http
        self.http = self.fixture.prepare("cli", via_cli=True)
        scope = self.fixture.root / "cli/scope"
        first_endpoint = json.loads((scope / ".daemon/management.json").read_text())
        process = next(item for item in self.fixture.processes if item["pid"] == self.fixture.servers["cli"].pid)
        old_generation = self.http.generation
        removed_directory = self.fixture.root / "cli/project/temporary-launch"
        removed_directory.mkdir()
        launch = self.launch_payload("removed-directory-replay")
        launch["launch"]["directory"] = str(removed_directory)
        self.http.post("/api/v1/agents", launch)
        original = self.http.operation(launch["request_id"])
        name = original["result"]["Launched"]["agents"][0]
        view = self.live(name)
        self.submit_target("stop", view["agent"])
        shutil.rmtree(removed_directory)
        assert self.http.post("/api/v1/agents", launch) == original

        changed_project = self.fixture.root / "cli/changed-project"
        changed_project.mkdir()
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as reservation:
            reservation.bind(("127.0.0.2", 0))
            changed_port = reservation.getsockname()[1]
        self.http = self.fixture.web_cli("cli", "reconfigure", changed_project, "127.0.0.2", changed_port)
        changed_endpoint = json.loads((scope / ".daemon/management.json").read_text())
        assert changed_endpoint["daemon_instance"] == first_endpoint["daemon_instance"]
        assert self.http.generation == old_generation and self.fixture.servers["cli"].poll() is None
        assert changed_endpoint["url"] != first_endpoint["url"]
        assert self.http.post("/api/v1/agents", launch) == original
        changed = copy.deepcopy(launch)
        changed["launch"]["prompt"] = "changed command must conflict even when original directory is missing"
        self.reject("POST", "/api/v1/agents", (409,), payload=changed)
        assert len([item for item in self.http.agents() if item["title"] == "removed-directory-replay"]) == 1
        self.record("M11", first_endpoint=first_endpoint, changed_endpoint=changed_endpoint,
                    generation=old_generation, replay=original,
                    daemon_identity_preserved=self.fixture.servers["cli"].record == process)
        self.fixture.stop_daemon("cli")
        self.http = primary

    def browser(self):
        config = {"url": self.http.url, "generation": self.http.generation,
                  "project": str(self.fixture.root / "primary/project"), "evidence": str(self.fixture.evidence / "browser")}
        config_path = self.fixture.evidence / "browser-input.json"
        config_path.write_text(json.dumps(config, indent=2) + "\n")
        env = {**os.environ, "AGENTX_DIR": str(self.fixture.root / "primary/scope")}
        result = subprocess.run(["node", str(Path(__file__).with_name("browser.mjs")), str(config_path)],
                                capture_output=True, text=True, env=env, timeout=180)
        (self.fixture.evidence / "browser-output.json").write_text(json.dumps({"code": result.returncode,
                                    "stdout": result.stdout, "stderr": result.stderr}, indent=2) + "\n")
        assert result.returncode == 0, result.stderr
        self.record("B01-B03", report="browser/report.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--evidence", required=True)
    parser.add_argument("--browser", action="store_true")
    parser.add_argument("--browser-only", action="store_true", help="Run actual browser on a new fixture after unchanged candidate HTTP cases passed")
    args = parser.parse_args()
    fixture = Fixture(args.candidate, args.evidence)
    cases = Cases(fixture)
    failure = None
    try:
        if args.browser_only:
            cases.http = fixture.prepare("primary")
        else:
            cases.startup()
            cases.lifecycle()
            cases.duplicate_and_disconnect()
            cases.bounds()
            cases.pagination()
            cases.stop_history_resume()
            cases.isolation_restart_reset()
            cases.cli_enable_reconfigure_replay()
        if args.browser or args.browser_only:
            cases.browser()
    except Exception:
        failure = traceback.format_exc()
        print(failure, flush=True)
    finally:
        try:
            fixture.close()
            cases.record("M10", cleanup="manifest.json")
        except Exception:
            failure = (failure or "") + "\nCleanup: " + traceback.format_exc()
        report = {"status": "fail" if failure else "pass", "failure": failure,
                  "browser_requested": args.browser or args.browser_only, "browser_only": args.browser_only, "cases": cases.steps,
                  "fixture": str(fixture.root), "candidate": str(fixture.candidate)}
        (fixture.evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    raise SystemExit(1 if failure else 0)


if __name__ == "__main__":
    main()
