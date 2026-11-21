#!/usr/bin/env python3
"""Linux smoke of the real hook, CLI, daemon and log boundaries in a disposable scope."""

import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import tempfile
import time


def check(binary, root):
    scope = root / "scope"
    home = root / "home"
    for path in (scope, home):
        path.mkdir(mode=0o700)
    scope.joinpath("config.toml").write_text("[management]\nenabled = false\n")
    env = {
        "PATH": "/usr/bin:/bin",
        "HOME": str(home),
        "AGENTX_DIR": str(scope),
        "AGENTX_HOOK_TRACE_SAMPLE_PERCENT": "100",
    }
    log = scope / ".tmp/logs/agentx.log"
    secret = "fixture-do-not-collect-4747"

    def run(*args, hook_env=None, stdin=None, codes=(0,)):
        result = subprocess.run([binary, *args], env={**env, **(hook_env or {})}, cwd=root, input=stdin, capture_output=True, timeout=20, text=True)
        assert result.returncode in codes, (args, result.stderr, result.stdout)
        assert secret not in result.stdout + result.stderr
        return result.stdout

    def observe(*args):
        return json.loads(run("observe", *args, "--json"))

    def snapshot():
        # Native reads intentionally report transient registry contention
        # while admission is registering the fixture's held connections.
        for _ in range(8):
            result = json.loads(run("observe", "--json", codes=(0, 1)))["result"]
            runtime = result.get("Inspection", {}).get("response", {}).get("Runtime", {})
            if runtime.get("Err") != "Busy":
                assert "Ok" in runtime, result
                return runtime["Ok"]["Snapshot"]
            time.sleep(0.01)
        raise AssertionError("observation registry remained busy")

    def traces():
        return [row["trace"] for row in observe("trace")["logs"]]

    @contextlib.contextmanager
    def daemon():
        child = subprocess.Popen([binary, "daemon", "serve", "--scope", str(scope)], cwd=root, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            for _ in range(50):
                assert child.poll() is None
                try:
                    if snapshot()["lifecycle"] == "Serving":
                        break
                except AssertionError:
                    pass
                time.sleep(0.02)
            else:
                raise AssertionError("disposable daemon never became ready")
            yield
        finally:
            child.terminate()
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=5)

    hook_env = {"AGENTX_PROCESS_ID": "hook-fixture-token", "AGENTX_TOOL": "codex"}
    # Bad input reports a local failure without requiring a daemon or copying
    # payloads, including when normal success sampling is disabled.
    run("codex-pretooluse", hook_env={**hook_env, "AGENTX_HOOK_TRACE_SAMPLE_PERCENT": "0"}, stdin=secret)
    early, = traces()
    assert early["selection"] == "failed" and early["failure_phase"] == "read_input" and early["attempts"] == []
    assert observe("trace", early["trace_id"], "--local")["native"] is None
    local_text = run("observe", "trace", early["trace_id"], "--local")
    assert all(field in local_text for field in (early["trace_id"], "failed", "read_input"))
    assert "Err" in observe("trace", early["trace_id"])["native"]
    assert not (scope / "agentx.db").exists()
    run("userpromptsubmit", hook_env={**hook_env, "AGENTX_TOOL": "claude"}, stdin=secret)
    claude, = (trace for trace in traces() if trace["provider"] == "claude")
    assert claude["failure_phase"] == "read_input" and claude["attempts"] == []

    with daemon():
        pass
    # Seed only this closed fixture. The tested hook then uses ordinary native
    # request/identity handling while the sole daemon owns the database.
    with contextlib.closing(sqlite3.connect(scope / "agentx.db")) as db:
        with db:
            db.execute("INSERT INTO instances(name,tool,session_id,created_at) VALUES ('luna','codex','codex-fixture',1)")
            db.execute("INSERT INTO process_bindings(process_id,session_id,instance_name,updated_at) VALUES ('hook-fixture-token','codex-fixture','luna',0)")
    payload = json.dumps({"session_id": "codex-fixture", "transcript_path": str(root / "fixture.jsonl"), "cwd": str(root), "tool_name": "Bash", "tool_input": {"command": secret}})

    socket_path = f"/tmp/agentx-{os.geteuid()}/{hashlib.sha256(os.fsencode(scope.resolve())).hexdigest()}.sock"
    with daemon():
        assert run("codex-pretooluse", hook_env=hook_env, stdin=payload) == ""
        success, = (trace for trace in traces() if trace["selection"] in ("sampled", "slow"))
        joined = observe("trace", success["trace_id"])
        rows = joined["native"]["Ok"]["response"]["Runtime"]["Ok"]["Requests"]["rows"]
        assert {row["operation"]["variant"] for row in rows} >= {"Resolve", "ObserveHookPosition", "ToolActivity"}
        assert {row["origin"]["invocation"]["invocation"] for row in rows} == {success["trace_id"]}
        client_request_ids = {attempt["request_id"] for attempt in success["attempts"]}
        assert {row["request_id"] for row in rows} <= client_request_ids
        assert all("Returned" in row["execution"] and row["enqueued_at"] and row["dequeued_at"] for row in rows)
        before = log.stat().st_size
        for _ in range(4):
            run("codex-pretooluse", hook_env={**hook_env, "AGENTX_HOOK_TRACE_SAMPLE_PERCENT": "0"}, stdin=payload)
        skipped_sample = [t for t in traces() if t["selection"] == "sampled"]
        assert len(skipped_sample) == int(success["selection"] == "sampled")
        # A highly loaded test machine may legitimately promote an invocation
        # to slow. Report this separately from the 0% success sampling check.
        unsampled_log_bytes = log.stat().st_size - before
        with subprocess.Popen(
            [binary, "codex-pretooluse"], cwd=root,
            env={**env, **hook_env, "AGENTX_HOOK_TRACE_SAMPLE_PERCENT": "0"},
            text=True, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        ) as slow_child:
            # Deliver only half a JSON object first so stdin parsing blocks
            # inside AgentX, independent of provider or daemon turnaround.
            slow_child.stdin.write(payload[: len(payload) // 2])
            slow_child.stdin.flush()
            time.sleep(0.75)
            stdout, stderr = slow_child.communicate(payload[len(payload) // 2 :], timeout=20)
            assert slow_child.returncode == 0 and stdout == "" and secret not in stderr
        latest_slow = next(trace for trace in traces() if trace["selection"] == "slow")
        assert latest_slow["duration_us"] >= 500_000

        # Keep precisely the business connection pool occupied without sending
        # any calls. The separate passive lane still exposes pressure.
        with contextlib.ExitStack() as connections:
            for _ in range(32):
                stream = connections.enter_context(socket.socket(socket.AF_UNIX))
                stream.settimeout(2)
                stream.connect(socket_path)
            for _ in range(30):
                if snapshot()["gauges"]["business_connections"] == 32:
                    break
                time.sleep(0.01)
            else:
                raise AssertionError("fixture did not occupy the full business pool")
            previous = snapshot()["lifetime"]["admission_busy"]
            started = time.monotonic()
            run("codex-pretooluse", hook_env=hook_env, stdin=payload)
            busy_ms = (time.monotonic() - started) * 1000
            assert snapshot()["lifetime"]["admission_busy"] > previous
            busy, = (trace for trace in traces() if trace["failure_phase"] == "daemon_acquire")
            assert any(attempt["failure"] == "Busy" for attempt in busy["attempts"]), busy
            assert not any(stage["stage"] in ("Spawn", "StartupLock") for attempt in busy["attempts"] for stage in attempt["stages"]), busy["attempts"]

    assert observe("trace", success["trace_id"], "--local")["logs"]
    assert secret not in log.read_text()
    return {
        "joined_native_requests": len(rows),
        "busy_hook_wall_ms": round(busy_ms, 2),
        "four_zero_percent_hooks_log_bytes": unsampled_log_bytes,
        "slow_hook_ms": round(latest_slow["duration_us"] / 1000, 2),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="agentx-hook-trace-") as temp:
        print(json.dumps(check(str(args.binary.resolve()), Path(temp))))
