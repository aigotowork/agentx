#!/usr/bin/env python3
"""Capture pre-dispatch UI/journal/network state for one exact Space message send."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/live_ui_integration"))
from live_fixture import LiveFixture, wait_for


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--driver", default="send-diagnostic.mjs")
    args = parser.parse_args()
    candidate = args.candidate.resolve(strict=True)
    assert not args.candidate.is_symlink()
    assert hashlib.sha256(candidate.read_bytes()).hexdigest() == args.sha256
    args.evidence.mkdir(parents=True, exist_ok=False)
    fixture = LiveFixture(candidate, args.evidence / "fixture")
    try:
        http = fixture.prepare("primary")
        body = {"generation": http.generation, "request_id": str(uuid.uuid4()),
                "launch": {"tool": "qoder", "profile": None, "directory": None,
                           "prompt": "send diagnostic fixture", "title": "send-diagnostic"}}
        http.post("/api/v1/agents", body)
        receipt = http.operation(body["request_id"])
        name = receipt["result"]["Launched"]["agents"][0]
        row = wait_for("send diagnostic fixture Agent", lambda: next((view for view in http.agents()
            if view["agent"]["name"] == name and view["can_send"] and view["session_id"]), None))
        config = {"url": http.url, "agent": row["agent"], "evidence": str(args.evidence / "browser")}
        config_path = args.evidence / "browser-config.json"
        config_path.write_text(json.dumps(config, indent=2) + "\n")
        subprocess.run(["node", str(Path(__file__).with_name(args.driver)), str(config_path)],
                       cwd=ROOT, check=True, timeout=150)
        (args.evidence / "fixture-state.json").write_text(json.dumps({"launch": receipt, "agent": row,
            "messages": http.get(f"/api/v1/agents/{name}/messages", generation=http.generation,
                                 incarnation=row["agent"]["incarnation"]),
            "candidate_sha256": hashlib.sha256(candidate.read_bytes()).hexdigest()}, indent=2) + "\n")
    finally:
        fixture.close()


if __name__ == "__main__":
    main()
