#!/usr/bin/python3
"""Finite terminal Codex substitute: generated hooks only, no model or real app-server."""
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import time
import tomllib
import uuid


def main():
    arguments = sys.argv[1:]
    if any(value in arguments for value in ["--version", "-V"]):
        print("codex-cli 0.134.0 (local generated-hook acceptance substitute)")
        return
    root = Path(os.environ["AGENTX_MANAGEMENT_FIXTURE"]).resolve(strict=True)
    scope = Path(os.environ["AGENTX_DIR"]).resolve(strict=True)
    assert root.name.startswith("agentx-management-") and scope.is_relative_to(root)
    hooks = {}
    for index, value in enumerate(arguments):
        if value not in ["-c", "--config"] or index + 1 >= len(arguments):
            continue
        raw = arguments[index + 1]
        if raw.startswith("hooks.") and not raw.startswith("hooks.state="):
            parsed = tomllib.loads(raw)
            hooks.update(parsed.get("hooks", {}))
    assert "SessionStart" in hooks and "PostModelRequest" in hooks, arguments
    if "app-server" in arguments:
        # Native launch performs a trust-discovery subprocess. This finite local
        # responder admits only discovery, never session/model execution.
        entries = []
        for event, groups in hooks.items():
            for group_index, group in enumerate(groups):
                for index, handler in enumerate(group["hooks"]):
                    command = handler["command"]
                    entries.append({"key": f"fixture:{event}:{group_index}:{index}", "command": command,
                                    "source": "sessionFlags", "currentHash": hashlib.sha256(command.encode()).hexdigest()})
        for line in sys.stdin:
            message = json.loads(line)
            if message["method"] == "initialized":
                continue
            assert message["method"] in ["initialize", "hooks/list"], message
            result = {"data": [{"hooks": entries}]} if message["method"] == "hooks/list" else {}
            print(json.dumps({"id": message["id"], "result": result}), flush=True)
        return
    session = str(uuid.uuid4())
    transcript = Path(os.environ["CODEX_HOME"]) / "sessions" / f"rollout-{session}.jsonl"
    transcript.parent.mkdir(parents=True, exist_ok=True)
    transcript.write_text(json.dumps({"type": "session_meta", "payload": {"id": session, "cwd": os.getcwd()}}) + "\n")
    control = root / "providers" / f"codex-{session}.control.json"
    log = root / "providers" / f"codex-{session}.jsonl"

    def record(kind, **fields):
        with log.open("a") as stream:
            stream.write(json.dumps({"kind": kind, "at": time.time(), **fields}) + "\n")

    def hook(event, **fields):
        for group in hooks[event]:
            for handler in group["hooks"]:
                result = subprocess.run(["/bin/sh", "-c", handler["command"]],
                    input=json.dumps({"session_id": session, "cwd": os.getcwd(), "transcript_path": str(transcript), **fields}),
                    capture_output=True, text=True, timeout=15)
                record("codex-hook", event=event, code=result.returncode, stdout=result.stdout, stderr=result.stderr)
                assert result.returncode == 0, result.stderr

    record("codex-started", name=os.environ["AGENTX_INSTANCE_NAME"], pid=os.getpid(),
           session=session, control=str(control), transcript=str(transcript), scope=str(scope))
    hook("SessionStart", source="startup")
    running = True

    def stop(_signal, _frame):
        nonlocal running
        running = False

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    last = None
    deadline = time.monotonic() + 900
    record("codex-ready", session=session)
    while running and time.monotonic() < deadline:
        try:
            command = json.loads(control.read_text())
        except (FileNotFoundError, ValueError):
            command = None
        if command and command != last:
            assert command["phase"] in ["failed", "completed", "started"]
            event = "PreModelRequest" if command["phase"] == "started" else "PostModelRequest"
            hook(event, turn_id=command["turn"], model="fixture-no-model", status=command["phase"],
                 error="controlled local transport failure" if command["phase"] == "failed" else None)
            record("codex-observed", command=command)
            last = command
        # Do not synthesize Stop/listening or read/complete the continuation.
        # Mailbox acceptance alone must leave the model error unresolved.
        select.select([], [], [], 0.1)
    record("codex-finished", session=session)


if __name__ == "__main__":
    main()
