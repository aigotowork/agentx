#!/usr/bin/env python3
"""Run one owned acceptance command with its exact durable log and receipt.

This runner does not select checks, retry failures or grant process scope.
The integration Task owns execution authorization and the evidence directory.
"""

import argparse
import json
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or Path(args.name).name != args.name:
        parser.error("requires a simple receipt name and command after --")
    args.evidence.mkdir(parents=True, exist_ok=True)
    log_path = args.evidence / (args.name + ".log")
    receipt_path = args.evidence / (args.name + "-receipt.json")
    if log_path.exists() or receipt_path.exists():
        parser.error("receipt exists; choose a new attempt name and preserve history")
    started = time.monotonic()
    with log_path.open("x") as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    receipt = {"command": command, "cwd": str(Path.cwd()), "exit_code": result.returncode,
               "elapsed_seconds": time.monotonic() - started, "log": str(log_path)}
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))
    print(log_path.read_text(errors="replace")[-(18000 if result.returncode else 5000):])
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
