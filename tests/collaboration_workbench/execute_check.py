#!/usr/bin/env python3
"""Run one explicitly selected fixed-candidate check with task-local tooling."""
import argparse
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    info = json.loads((args.evidence / "tooling/tooling.json").read_text())
    env = {key: value for key, value in os.environ.items() if not key.startswith(("AGENTX_", "PASEO_"))}
    env.update(PATH=str(Path(info["node"]).parent) + os.pathsep + env["PATH"],
               AGENTX_DIR=str(args.evidence / "check-scope"), TMPDIR=str(args.evidence / "check-tmp"))
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    return subprocess.call(["python3", "scripts/run-acceptance-check.py", "--evidence", str(args.evidence),
                            "--name", args.name, "--", *command], cwd=args.source, env=env)


if __name__ == "__main__":
    raise SystemExit(main())
