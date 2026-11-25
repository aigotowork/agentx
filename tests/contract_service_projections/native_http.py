#!/usr/bin/env python3
"""Independent five-operation native HTTP conformance, before SDK supply."""

import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys
import traceback
import uuid
from urllib.parse import quote

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "agent_management"))
from fixture import Fixture, wait_for


def run(fixture, spec_path):
    spec_bytes = spec_path.read_bytes()
    spec = json.loads(spec_bytes)
    expected = {
        ("get", "/api/v1/management"): "managementDescribe",
        ("get", "/api/v1/agents"): "managementListAgents",
        ("get", "/api/v1/agents/{name}"): "managementGetAgent",
        ("post", "/api/v1/agents/{name}/messages"): "managementSubmitMessage",
        ("get", "/api/v1/operations/{request_id}"): "managementGetOperation",
    }
    actual = {(method, path): operation["operationId"] for path, item in spec["paths"].items()
              for method, operation in item.items() if method in {"get", "post", "put", "patch", "delete"}}
    assert all(actual.get(key) == operation for key, operation in expected.items()), actual
    (fixture.evidence / "spec.json").write_bytes(spec_bytes)
    cases = []

    def record(case, **facts):
        cases.append({"case": case, "status": "pass", **facts})
        (fixture.evidence / "native-cases.json").write_text(json.dumps(cases, indent=2) + "\n")

    http = fixture.clients.get("primary") or fixture.prepare("primary")
    descriptor = http.get("/api/v1/management")
    assert descriptor["generation"] == http.generation
    empty = http.get("/api/v1/agents", limit=1, current_only="false")
    assert empty["agents"] == [] and empty["next"] is None, empty
    record("CS-01-native-describe-empty-list", descriptor=descriptor, page=empty)

    # Launch is setup through an existing public capability, not pilot coverage.
    launch = {"request_id": str(uuid.uuid4()), "generation": http.generation,
              "launch": {"tool": "qoder", "profile": None, "directory": None,
                         "prompt": "independent contract acceptance fixture", "title": "contract-pilot"}}
    http.post("/api/v1/agents", launch)
    created = http.operation(launch["request_id"])["result"]["Launched"]
    assert created["failed"] == 0 and len(created["agents"]) == 1, created

    def ready():
        return next((view for view in http.agents() if view["agent"]["name"] == created["agents"][0]
                     and view["can_send"] and view["session_id"]), None)

    view = wait_for("finite fixture Agent session bound", ready)
    ref = view["agent"]
    query = {key: value for key, value in ref.items() if key != "name" and value is not None}
    agent_path = "/api/v1/agents/" + ref["name"]
    exact = http.get(agent_path, **query)
    assert exact["agent"] == ref, exact
    scoped_status, scoped = http.call("GET", agent_path, query={"generation": http.generation, **query},
                                      headers={"X-AgentX-Expected-Scope": quote(descriptor["scope"], safe="")})
    assert scoped_status == 200 and scoped["agent"] == ref, scoped
    page = http.get("/api/v1/agents", limit=1, current_only="true")
    assert page["agents"][0]["agent"] == ref and len(page["agents"]) == 1, page
    request = {"request_id": str(uuid.uuid4()), "generation": http.generation, "agent": ref,
               "message": {"text": "contract native composed message", "intent": "request",
                           "reply_to": None, "thread": "contract-acceptance", "attachments": []}}
    accepted = http.post(agent_path + "/messages", request)
    sent = http.operation(request["request_id"])
    message_id = sent["result"]["Sent"]["message_id"]
    assert isinstance(message_id, str) and sent["request_id"] == request["request_id"]
    replay = http.post(agent_path + "/messages", request)
    assert replay == sent, (replay, sent)

    def replied():
        page = http.get(agent_path + "/messages", **query)
        return page if any(message["reply_to"] == message_id for message in page["messages"]) else None

    messages = wait_for("finite Provider hook reply", replied)
    assert sum(message["id"] == message_id for message in messages["messages"]) == 1, messages
    original = next(message for message in messages["messages"] if message["id"] == message_id)
    assert original["text"] == request["message"]["text"] and original["from"] == "bigboss", original
    record("CS-01-native-five-operations", exact=exact, list_page=page, request=request,
           admission=accepted, receipt=sent, messages=messages)
    record("CS-03-native-identical-replay", receipt=replay, effect_message_id=message_id)

    changed = copy.deepcopy(request)
    changed["message"]["text"] = "conflicting intent must not execute"
    invalid = {**request, "request_id": str(uuid.uuid4()), "unknown": True}
    stale = {**request, "request_id": str(uuid.uuid4()), "generation": str(uuid.uuid4())}
    failures = [("CS-03-native-conflicting-id", changed, 409, "Conflict"),
                ("CS-06-native-strict-body", invalid, 400, "InvalidInput"),
                ("CS-02-native-wrong-generation", stale, 409, "StaleGeneration")]
    for case, payload, status, code in failures:
        observed, body = http.call("POST", agent_path + "/messages", payload)
        assert observed == status and body["error"]["code"] == code, (case, observed, body)
        record(case, status=observed, error=body)
    wrong_scope = {**request, "request_id": str(uuid.uuid4())}
    observed, body = http.call("POST", agent_path + "/messages", wrong_scope,
                               headers={"X-AgentX-Expected-Scope": quote(descriptor["scope"] + "/另一个 scope", safe="")})
    assert observed == 409 and body["error"]["code"] == "StaleGeneration", (observed, body)
    missing, receipt = http.call("GET", "/api/v1/operations/" + wrong_scope["request_id"], query={"generation": http.generation})
    assert missing == 404 and receipt["error"]["code"] == "NotFound", (missing, receipt)
    record("CS-02-native-wrong-scope-same-generation", status=observed, error=body, receipt=receipt)
    assert http.get("/api/v1/operations/" + request["request_id"]) == sent
    record("native-run-basis", spec_sha256=hashlib.sha256(spec_bytes).hexdigest(),
           boundary="real HTTP/native/worker with finite Qoder fixture; no SDK/CLI or actual model evidence")
    return ref, launch


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--spec", required=True, type=Path)
    args = parser.parse_args()
    fixture = Fixture(args.candidate, args.evidence)
    result = {"status": "failed", "boundary": "native HTTP only; public package acceptance pending"}
    try:
        run(fixture, args.spec)
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
            (fixture.evidence / "native-result.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
