#!/usr/bin/python3
"""Finite Codex recipient for local environment acceptance, without a model.

The hooks-list handshake mirrors the existing direct-stdio fixture contract.
All observations are restricted to an explicit fixture variable allowlist.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys


KEYS = [
    "AGENTX_ENV_CASE",
    "AGENTX_ENV_OVERLAY",
    "AGENTX_ENV_EMPTY",
    "AGENTX_ENV_TEST_VALUE",
    "AGENTX_ENV_NESTED_MARKER",
    "ENV_FIXTURE_LEGACY",
    "ENV_FIXTURE_EMPTY",
    "AGENTX_DIR",
    "CODEX_HOME",
]


def process_identity(pid: int) -> dict | None:
    proc = Path("/proc") / str(pid)
    try:
        return {
            "pid": pid,
            "start_ticks": (proc / "stat").read_text().rsplit(")", 1)[1].split()[19],
            "executable": str((proc / "exe").resolve(strict=True)),
            "argv": (proc / "cmdline").read_bytes().rstrip(b"\0").decode().split("\0"),
        }
    except FileNotFoundError:
        return None


def record(capture: Path, kind: str) -> None:
    capture.mkdir(parents=True, exist_ok=True)
    result = {
        "kind": kind,
        "pid": os.getpid(),
        "ppid": os.getppid(),
        "process": process_identity(os.getpid()),
        "parent_process": process_identity(os.getppid()),
        "start_ticks": Path("/proc/self/stat").read_text().rsplit(")", 1)[1].split()[19],
        "cwd": os.getcwd(),
        "argv": sys.argv[1:],
        "environment": {key: os.environ.get(key) for key in KEYS},
        "streams": {str(fd): os.isatty(fd) for fd in range(3)},
    }
    (capture / f"{kind}-{os.getpid()}.json").write_text(json.dumps(result, indent=2) + "\n")


def hooks_preflight() -> None:
    hooks = [
        {
            "key": f"environment-fixture-{index}",
            "command": f"agentx codex-{hook}",
            "source": "sessionFlags",
            "currentHash": f"environment-fixture-hash-{index}",
        }
        for index, hook in enumerate(
            ["sessionstart", "userpromptsubmit", "pretooluse", "posttooluse", "stop", "interrupt"]
        )
    ]
    initialize = json.loads(sys.stdin.readline())
    print(json.dumps({"id": initialize["id"], "result": {}}), flush=True)
    json.loads(sys.stdin.readline())
    listing = json.loads(sys.stdin.readline())
    print(json.dumps({"id": listing["id"], "result": {"data": [{"hooks": hooks}]}}), flush=True)


def main() -> None:
    capture = Path(os.environ["AGENTX_ENV_CAPTURE_DIR"]).resolve(strict=True)
    if sys.argv[1:3] == ["-c", "features.hooks=true"] and "app-server" in sys.argv[1:]:
        record(capture, "preflight")
        hooks_preflight()
        return
    if sys.argv[1:] in [["--version"], ["-V"]]:
        record(capture, "version")
        print("codex-cli 0.155.0")
        return
    record(capture, "child")
    if os.environ.get("AGENTX_ENV_ECHO_STDIN") == "1":
        sys.stdout.buffer.write(sys.stdin.buffer.read())
        sys.stdout.buffer.flush()
    if command := os.environ.get("AGENTX_ENV_NEST_COMMAND"):
        environment = dict(os.environ)
        environment.pop("AGENTX_ENV_NEST_COMMAND")
        environment["AGENTX_ENV_CAPTURE_DIR"] = environment.pop("AGENTX_ENV_NEST_CAPTURE")
        nested = subprocess.run(json.loads(command), env=environment, capture_output=True, timeout=30)
        (capture / "nested-result.json").write_text(json.dumps({
            "command": json.loads(command), "exit_code": nested.returncode,
            "stdout": nested.stdout.decode(errors="replace"), "stderr": nested.stderr.decode(errors="replace"),
        }, indent=2) + "\n")
        if nested.returncode != 0:
            raise SystemExit(nested.returncode)
    raise SystemExit(int(os.environ.get("AGENTX_ENV_CHILD_STATUS", "0")))


if __name__ == "__main__":
    main()
