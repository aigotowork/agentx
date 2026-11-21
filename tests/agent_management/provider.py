#!/usr/bin/python3
"""Finite Qoder substitute: real generated hooks, local transcript, no model/API."""

import codecs
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import sys
import termios
import time
import tty
import uuid


def process_identity(pid):
    path = Path(f"/proc/{pid}")
    return {"pid": pid, "start_ticks": (path / "stat").read_text().rsplit(")", 1)[1].split()[19],
            "executable": str((path / "exe").resolve()),
            "argv": (path / "cmdline").read_bytes().rstrip(b"\0").decode().split("\0")}


def main():
    if "--version" in sys.argv or "-V" in sys.argv:
        print("Qoder CLI 1.1.65 (local management acceptance fixture)")
        return
    root = Path(os.environ["AGENTX_MANAGEMENT_FIXTURE"]).resolve(strict=True)
    scope = Path(os.environ["AGENTX_DIR"]).resolve(strict=True)
    assert root.name.startswith("agentx-management-") and scope.is_relative_to(root)
    arguments = sys.argv[1:]
    settings_paths = [value.split("=", 1)[1] for value in arguments if value.startswith("--settings=")]
    settings_paths += [arguments[index + 1] for index, value in enumerate(arguments) if value == "--settings"]
    assert len(settings_paths) == 1, arguments
    settings_path = Path(settings_paths[0]).resolve(strict=True)
    assert settings_path.is_relative_to(scope), settings_path
    settings = json.loads(settings_path.read_text())
    resumed = "--resume" in arguments
    session = arguments[arguments.index("--resume") + 1] if resumed else str(uuid.uuid4())
    transcript = Path(os.environ["HOME"]) / ".qoder/projects/management-fixture" / f"{session}.jsonl"
    transcript.parent.mkdir(parents=True, exist_ok=True)
    logfile = root / "providers" / f"{os.getpid()}.jsonl"
    base = {"session_id": session, "cwd": os.getcwd(), "transcript_path": str(transcript)}
    seen = set()

    def record(kind, **values):
        with logfile.open("a") as stream:
            stream.write(json.dumps({"kind": kind, "at": time.time(), **values}) + "\n")

    def exchange(prompt, answer):
        parent = None
        with transcript.open("a") as stream:
            for role, content in [("user", prompt), ("assistant", [{"type": "text", "text": answer}])]:
                identity = str(uuid.uuid4())
                stream.write(json.dumps({"type": role, "uuid": identity, "parentUuid": parent,
                                         "timestamp": datetime.now(timezone.utc).isoformat(),
                                         "isSidechain": False, "cwd": os.getcwd(), "sessionId": session,
                                         "message": {"role": role, "content": content}}) + "\n")
                parent = identity

    def hook(event, **values):
        outputs = []
        commands = [item["command"] for group in settings["hooks"][event]
                    for item in group["hooks"] if item.get("type") == "command"]
        assert commands, event
        for command in commands:
            result = subprocess.run(["/bin/sh", "-c", command], input=json.dumps({**base, **values}),
                                    text=True, capture_output=True, timeout=15)
            record("hook", event=event, code=result.returncode, stdout=result.stdout, stderr=result.stderr)
            assert result.returncode == 0, result.stderr
            if result.stdout.strip():
                outputs.append(json.loads(result.stdout))
        return outputs

    def reply(outputs):
        for output in outputs:
            context = output.get("reason", "") + "\n" + output.get("hookSpecificOutput", {}).get("additionalContext", "")
            # Only actual messages addressed from the management sender. No
            # bootstrap prose is interpreted as a command or forwarded.
            pattern = r"\[(?:request|inform)(?::[^\]\s]+)? #(\d+)\] bigboss → [^:\n]+: ([^\n]+)"
            for message_id, text in re.findall(pattern, context):
                if message_id in seen:
                    continue
                seen.add(message_id)
                answer = f"fixture reply #{message_id}: {text}"
                exchange(text, answer)
                command = [os.environ["AGENTX_MANAGEMENT_CANDIDATE"], "send", "@bigboss", "--intent", "ack",
                           "--reply-to", message_id, "--", answer]
                result = subprocess.run(command, text=True, capture_output=True, timeout=15)
                record("reply", message_id=message_id, text=text, code=result.returncode,
                       stdout=result.stdout, stderr=result.stderr)
                assert result.returncode == 0, result.stderr

    record("started", process=process_identity(os.getpid()), parent=process_identity(os.getppid()),
           name=os.environ.get("AGENTX_INSTANCE_NAME"), session=session, resumed=resumed,
           cwd=os.getcwd(), scope=str(scope), transcript=str(transcript), args=arguments)
    initial = arguments[arguments.index("--prompt-interactive") + 1] if "--prompt-interactive" in arguments else "fixture initial prompt"
    if not resumed:
        exchange(initial, "fixture ready")
    hook("SessionStart", source="resume" if resumed else "startup")
    reply(hook("Stop", stop_hook_active=False))
    running = True

    def finish(_signal, _frame):
        nonlocal running
        running = False

    signal.signal(signal.SIGTERM, finish)
    signal.signal(signal.SIGINT, finish)
    original = termios.tcgetattr(0)
    tty.setraw(0)
    decoder = codecs.getincrementaldecoder("utf-8")()
    pending, composer = "", ""

    def render():
        columns, rows = os.get_terminal_size(1)
        rule = "─" * columns
        content = ["? for shortcuts", rule, " Shift+Tab to Accept Edits", rule,
                   " > " + (composer or "  Type your message or @path/to/file"), rule,
                   " Qwen3.8-Flash Model · ctx ░░░░░░░░░░ 0% · /management-fixture"]
        screen = "\x1b[2J\x1b[H" + f"\x1b[{max(1, rows - len(content) + 1)};1H"
        os.write(1, (screen + "\r\n".join(content)).encode())

    try:
        render()
        record("ready")
        deadline = time.monotonic() + 900
        while running and time.monotonic() < deadline:
            # Optional consolidated acceptance supply stays in its owning harness.
            # Import only a sibling explicitly staged inside this private fixture.
            if (root / "bin/contract_actor.py").is_file():
                from contract_actor import poll
                poll(root, record)
            if not select.select([0], [], [], 0.5)[0]:
                # Models can complete turns without a PTY wake. This consumes
                # the supported Stop-hook delivery channel, never DB state.
                reply(hook("Stop", stop_hook_active=False))
                continue
            data = os.read(0, 4096)
            if not data:
                break
            pending += decoder.decode(data)
            while pending:
                wrapper = next((token for token in ["\x1b[200~", "\x1b[201~"] if pending.startswith(token)), None)
                if wrapper:
                    pending = pending[len(wrapper):]
                    continue
                if any(token.startswith(pending) for token in ["\x1b[200~", "\x1b[201~"]):
                    break
                character, pending = pending[0], pending[1:]
                if character in "\r\n":
                    if composer:
                        prompt, composer = composer, ""
                        record("input", prompt=prompt)
                        reply(hook("UserPromptSubmit", prompt=prompt))
                        reply(hook("Stop", stop_hook_active=False))
                elif character in ["\x7f", "\b"]:
                    composer = composer[:-1]
                elif character >= " ":
                    composer += character
                elif character in ["\x03", "\x04"]:
                    running = False
            render()
    finally:
        termios.tcsetattr(0, termios.TCSANOW, original)
        record("finished")


if __name__ == "__main__":
    main()
