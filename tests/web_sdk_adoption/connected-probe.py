#!/usr/bin/env python3
"""Run browser CORS checks against one disposable candidate Management listener."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import traceback

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/agent_management"))
from fixture import Fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--evidence", required=True, type=Path)
    args = parser.parse_args()
    candidate = args.candidate.resolve(strict=True)
    assert candidate.is_absolute() and not args.candidate.is_symlink()
    assert hashlib.sha256(candidate.read_bytes()).hexdigest() == args.sha256, "candidate hash mismatch"
    args.evidence.mkdir(parents=True, exist_ok=False)
    fixture = Fixture(candidate, args.evidence / "fixture")
    failure = None
    try:
        http = fixture.prepare("primary")
        config = {
            "requests": [
                {"name": "descriptor-read", "url": http.url + "/api/v1/management",
                 "expectedStatuses": [200], "expectedContentType": "application/json"},
                {"name": "invalid-json-write", "url": http.url + "/api/v1/agents", "method": "POST",
                 "headers": {"Content-Type": "application/json"}, "body": "{}",
                 "expectedStatuses": [400], "expectedContentType": "application/json"},
            ]
        }
        config_path = args.evidence / "cross-origin.json"
        config_path.write_text(json.dumps(config, indent=2) + "\n")
        command = ["node", str(ROOT / "tests/web_sdk_adoption/cross-origin.browser.mjs"),
                   str(config_path), str(args.evidence / "browser")]
        (args.evidence / "command.json").write_text(json.dumps(command) + "\n")
        subprocess.run(command, cwd=ROOT, check=True)
    except Exception:
        failure = traceback.format_exc()
        (args.evidence / "failure.txt").write_text(failure)
    finally:
        fixture.close()
    if failure:
        raise SystemExit(failure)


if __name__ == "__main__":
    main()
