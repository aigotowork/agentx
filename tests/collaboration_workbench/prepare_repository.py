#!/usr/bin/env python3
"""Create only the bounded actual repository; do not launch Agents or checks."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def prepare(destination):
    destination.mkdir(mode=0o700, parents=True, exist_ok=False)
    fixture = Path(__file__).resolve().parent / "fixtures"
    files = {}
    for source in sorted(fixture.iterdir()):
        if source.is_file():
            target = destination / source.name
            shutil.copyfile(source, target)
            files[source.name] = hashlib.sha256(target.read_bytes()).hexdigest()
    (destination / ".gitignore").write_text(".checks/\n__pycache__/\n*.csv\n")
    subprocess.run(["git", "init", "--initial-branch=workbench-fixture", str(destination)], check=True, capture_output=True)
    for key, value in [("user.name", "Workbench Acceptance Fixture"), ("user.email", "fixture@agentx.invalid")]:
        subprocess.run(["git", "-C", str(destination), "config", "--local", key, value], check=True)
    subprocess.run(["git", "-C", str(destination), "add", "."], check=True)
    subprocess.run(["git", "-C", str(destination), "commit", "-m", "Retain deficient CSV baseline for independent checking"], check=True, capture_output=True)
    commit = subprocess.run(["git", "-C", str(destination), "rev-parse", "HEAD"], check=True, text=True, capture_output=True).stdout.strip()
    return {"project": str(destination), "baseline_commit": commit, "fixture_sha256": files,
            "limits": "Repository preparation only. No Provider, service, check or product execution."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    with args.receipt.open("x") as receipt:
        value = prepare(args.project.resolve())
        receipt.write(json.dumps(value, indent=2) + "\n")
    print(json.dumps(value))
