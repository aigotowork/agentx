#!/usr/bin/env python3
"""One real management-worker launch using the existing isolated local Qoder fixture."""
import argparse
import json
from pathlib import Path
import sqlite3
import sys
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "agent_management"))
from fixture import Fixture, wait_for


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    owner = Fixture(args.candidate, args.evidence)
    receipt = {"boundary": "One separate fresh local Qoder mock scope; no backup-derived mutations, paid Provider or live service changes."}
    try:
        http = owner.prepare("worker-launch")
        request_id = str(uuid.uuid4())
        request = {"generation": http.generation, "request_id": request_id,
                   "launch": {"profile": "management-fixture", "tool": None, "role": None, "workspace": None,
                              "directory": str(owner.root / "worker-launch/project"), "prompt": "one finite worker acceptance launch",
                              "title": "Independent worker acceptance", "labels": {}}}
        accepted = http.post("/api/v1/agents", request)
        operation = http.operation(request_id)
        outcome = operation["result"]["Launched"]
        assert outcome["failed"] == 0 and len(outcome["agents"]) == 1, operation
        name = outcome["agents"][0]
        row = wait_for("local Qoder exact current reference", lambda: next((row for row in http.agents(current_only="true") if row["agent"]["name"] == name and row["session_id"]), None))
        assert row["agent"]["incarnation"] and row["agent"]["stopped_event"] is None
        exact = http.get("/api/v1/agents/" + name, incarnation=row["agent"]["incarnation"])
        assert exact["agent"] == row["agent"]
        scope = owner.root / "worker-launch/scope"
        with sqlite3.connect((scope / "agentx.db").as_uri() + "?mode=ro", uri=True) as connection:
            stored = json.loads(connection.execute("SELECT value FROM kv WHERE key=?", (f"management:operation:{http.generation}:{request_id}",)).fetchone()[0])
        worker_pid, worker_incarnation = stored["worker"]
        assert worker_pid > 0 and worker_incarnation
        assert stored["operation"]["status"] == "Succeeded"
        correlated = []
        for path in sorted((scope / ".tmp/logs").glob("agentx.log*")):
            if path.name.endswith(".lock"):
                continue
            for line in path.read_text().splitlines():
                try:
                    value = json.loads(line)
                    if value.get("operation_id") == request_id:
                        correlated.append(value)
                except ValueError:
                    pass
        events = {value["event"] for value in correlated}
        assert {"worker.started", "operation.finished"} <= events, correlated
        assert all(value["generation"] == http.generation for value in correlated)
        assert any(value.get("worker_pid") == str(worker_pid) for value in correlated)
        assert any(value["event"] == "operation.finished" and value["pid"] == worker_pid for value in correlated)
        providers = [value for value in owner.provider_records() if value.get("kind") == "started"]
        assert len(providers) == 1, providers
        receipt.update(status="passed", accepted=accepted, operation=operation, exact_agent=exact["agent"],
                       worker={"pid": worker_pid, "incarnation": worker_incarnation}, correlated_logs=correlated,
                       local_provider_starts=len(providers))
    except BaseException as error:
        receipt.update(status="failed", error=str(error))
        raise
    finally:
        try:
            owner.close()
            receipt["cleanup"] = "Existing fixture exact-process cleanup passed; no owned services left running."
        finally:
            (owner.evidence / "result.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    main()
