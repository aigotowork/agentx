#!/usr/bin/env python3
"""Finite Linux correctness check for the managed Codex command and local IPC."""

import argparse
import contextlib
import json
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time


def check(binary, root):
    scope, home = root / "scope", root / "home"
    scope.mkdir(mode=0o700)
    home.mkdir(mode=0o700)
    env = {"PATH": "/usr/bin:/bin", "HOME": str(home), "AGENTX_DIR": str(scope), "AGENTX_TEST_DB_NOSYNC": "1"}
    database = scope / "agentx.db"
    token = "hook-fixture-binding"

    def run(*args, input=None, hook=False):
        extra = {} if not hook else {
            "AGENTX_LAUNCHED": "1", "AGENTX_PROCESS_ID": token, "AGENTX_TOOL": "codex",
            "AGENTX_ENV_FILES": str(root / "intentionally-missing.env"), "AGENTX_HOOK_TRACE_SAMPLE_PERCENT": "100",
        }
        return subprocess.run([binary, *args], env={**env, **extra}, cwd=root, input=input, capture_output=True, text=True, timeout=10, check=True).stdout

    @contextlib.contextmanager
    def daemon():
        with subprocess.Popen([binary, "daemon", "serve", "--scope", str(scope)], cwd=root, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as proc:
            try:
                until = time.monotonic() + 10
                while time.monotonic() < until and proc.poll() is None:
                    result = subprocess.run([binary, "observe", "--json"], cwd=root, env=env, capture_output=True, timeout=5, text=True)
                    if result.returncode == 0 and '"Serving"' in result.stdout:
                        break
                    time.sleep(0.02)
                else:
                    raise AssertionError("disposable daemon did not become ready")
                yield
            finally:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=5)

    def post_tool(**fields):
        payload = {"session_id": "fixture-session", "cwd": str(root), "tool_name": "Bash", **fields}
        return run("__codex-hook", "codex-posttooluse", input=json.dumps(payload), hook=True)

    with daemon():
        pass
    with sqlite3.connect(database) as db:
        db.execute("INSERT INTO instances(name,tool,session_id,created_at,directory) VALUES ('luna','codex','fixture-session',1,?)", (str(root),))
        db.execute("INSERT INTO process_bindings(process_id,session_id,instance_name,updated_at) VALUES (?,'fixture-session','luna',0)", (token,))
    with daemon():
        assert post_tool() == ""
        assert post_tool(agent_id="provider-subagent") == ""
    with sqlite3.connect(database) as db:
        assert db.execute("SELECT COUNT(*) FROM kv WHERE key GLOB 'output_receipt:*'").fetchone() == (0,)
        event_id = db.execute("INSERT INTO events(type,timestamp,instance,data) VALUES ('message','2026-11-20T00:00:00Z','fixture',?)", (json.dumps({"scope": "mentions", "mentions": ["luna"], "from": "fixture", "text": "only this message should appear"}),)).lastrowid
    with daemon():
        output = json.loads(post_tool(tool_response={"unconsumed": "do-not-retain-fixture"}))
        assert "only this message should appear" in output["hookSpecificOutput"]["additionalContext"]
        assert post_tool() == ""
    with sqlite3.connect(database) as db:
        assert db.execute("SELECT last_event_id FROM instances WHERE name='luna'").fetchone() == (event_id,)
    entries = [json.loads(line) for line in (scope / ".tmp/logs/agentx.log").read_text().splitlines()]
    samples = [entry["trace"] for entry in entries if entry.get("event") == "hook.trace" and entry["trace"]["hook"] == "codex-posttooluse"]
    assert [len(trace["attempts"]) for trace in samples] == [1, 0, 2, 1], samples
    assert all(sum(span["stage"] == "Connect" for span in attempt["stages"]) == 1 for trace in samples for attempt in trace["attempts"])
    assert all(phase["phase"] not in {"environment", "dev_root"} for trace in samples for phase in trace["phases"])
    assert "do-not-retain-fixture" not in json.dumps(samples)
    print("Verified managed Hook: one connection for empty, none for subagent, two for delivered output plus confirmation.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="agentx-hook-fast-") as base:
        check(args.binary.resolve(), Path(base))
