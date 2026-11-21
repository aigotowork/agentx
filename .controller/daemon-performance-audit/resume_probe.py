#!/usr/bin/env python3
"""Time direct Provider startup and AgentX resume against disposable state."""

import argparse
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import time
import uuid


def probe(binary: Path, baggage: int, history: int) -> dict:
    with tempfile.TemporaryDirectory(prefix="ax-resume-") as directory:
        root = Path(directory)
        scope, workspace, home, tools = (root / name for name in ("scope", "workspace", "home", "bin"))
        for path in (scope, workspace, home, tools):
            path.mkdir(mode=0o700)
        marker = root / "started"
        provider = tools / "codex"
        provider.write_text("""#!/usr/bin/python3
import hashlib, json, os, subprocess, sys, time, tomllib
from pathlib import Path
if '-V' in sys.argv or '--version' in sys.argv:
    print('codex-cli 0.170.0')
    sys.exit(0)
hooks = {}
for option, value in zip(sys.argv, sys.argv[1:]):
    if option == '-c' and value.startswith('hooks.') and not value.startswith('hooks.state='):
        hooks.update(tomllib.loads(value).get('hooks', {}))
if 'app-server' in sys.argv:
    entries = [{'key': f'fixture:{event}:{i}:{j}', 'command': hook['command'], 'source': 'sessionFlags',
                'currentHash': hashlib.sha256(hook['command'].encode()).hexdigest()}
               for event, groups in hooks.items() for i, group in enumerate(groups) for j, hook in enumerate(group['hooks'])]
    for line in sys.stdin:
        request = json.loads(line)
        if 'id' in request:
            result = {'data': [{'hooks': entries}]} if request['method'] == 'hooks/list' else {}
            print(json.dumps({'id': request['id'], 'result': result}), flush=True)
    sys.exit(0)
marker = Path(os.environ['AX_PROBE_MARKER'])
# Codex flags may precede resume. Find the exact command and saved session.
resume = sys.argv.index('resume')
session = sys.argv[resume + 1]
marker.write_text(json.dumps([time.monotonic(), ['resume', session]]))
if hooks:
    for group in hooks['SessionStart']:
        for hook in group['hooks']:
            result = subprocess.run(['/bin/sh', '-c', hook['command']],
                input=json.dumps({'source': 'resume', 'session_id': session, 'cwd': os.getcwd()}),
                capture_output=True, text=True, timeout=10)
            if result.returncode != 0:
                raise RuntimeError(result.stderr)
    marker.with_suffix('.hook').write_text(json.dumps(time.monotonic()))
time.sleep(2)
""")
        provider.chmod(0o700)
        (tools / "agentx").symlink_to(binary)
        (tools / "tmux").symlink_to(Path(shutil.which("tmux")).resolve())
        (scope / "config.toml").write_text("[launch]\nauto_trust_workspace = false\n[preferences]\nauto_approve = false\n")
        env = {
            "PATH": f"{tools}:/usr/bin:/bin",
            "HOME": str(home),
            "CODEX_HOME": str(home / ".codex"),
            "AGENTX_DIR": str(scope),
            "AX_PROBE_MARKER": str(marker),
            "TERM": "xterm-256color",
        }

        def spawn(args):
            return subprocess.Popen(args, cwd=workspace, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

        def read_marker(deadline, process=None):
            while True:
                try:
                    return json.loads(marker.read_text())
                except (FileNotFoundError, json.JSONDecodeError):
                    if time.monotonic() >= deadline or (process is not None and process.poll() is not None):
                        return None
                    time.sleep(0.02)

        session = str(uuid.uuid4())
        before = time.monotonic()
        with spawn([provider, "resume", session]) as direct:
            evidence = read_marker(before + 20, direct)
            if evidence is None:
                direct.terminate()
                raise RuntimeError(f"controlled Provider never started: {direct.communicate(timeout=5)}")
            direct_started, direct_args = evidence
            assert direct_args == ["resume", session]
            direct.terminate()
            direct.communicate(timeout=5)
        marker.unlink()
        daemon = spawn([binary, "daemon", "serve", "--scope", scope])
        socket = scope / ".tmp/tmux/socket"
        try:
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                status = subprocess.run([binary, "daemon", "status", "--scope", scope], env=env, cwd=workspace, capture_output=True, text=True, timeout=5)
                if "Daemon: running" in status.stdout:
                    break
                assert daemon.poll() is None, daemon.communicate()
                time.sleep(0.05)
            else:
                raise RuntimeError("fixture daemon never became ready")
            with sqlite3.connect(scope / "agentx.db") as conn:
                conn.executemany("INSERT INTO kv(key,value) VALUES (?, '{}')", ((f"fixture-history:{i}",) for i in range(baggage)))
                stopped = {"action":"stopped", "snapshot":{
                    "name":"luna", "tool":"codex", "session_id":session, "directory":str(workspace),
                    "created_at":1, "last_event_id":0, "launch_args":"[]", "subscriptions":[{
                        "id":"fixture-collision", "caller":"luna", "sql":"0", "filters":{"collision":["1"]}, "last_id":0
                    }]
                }}
                conn.execute("INSERT INTO events(timestamp,type,instance,data) VALUES (datetime('now'),'life','luna',?)", (json.dumps(stopped),))
                conn.executemany("INSERT INTO events(timestamp,type,instance,data) VALUES (datetime('now'),?,?,?)", (("status", "nava", '{"status":"active","context":"tool:Bash"}') if i % 2 else ("message", "nava", '{"from":"nava","scope":"broadcast","text":"fixture"}') for i in range(history)))
            start = time.monotonic()
            managed = subprocess.run([binary, "resume", "luna", "--headless", "--terminal", "tmux", "--json", "--go"],
                                     cwd=workspace, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=75)
            returned = time.monotonic()
            first = read_marker(returned + 5) if managed.returncode == 0 else None
            evidence = {
                "binary": str(binary), "unrelated_kv": baggage, "events": history,
                "direct_provider_ms": round((direct_started - before) * 1000, 1),
                "resume_return_ms": round((returned - start) * 1000, 1),
                "resume_provider_ms": round((first[0] - start) * 1000, 1) if first else None,
                "exit": managed.returncode, "stderr": managed.stderr.strip(),
            }
            if managed.returncode == 0:
                assert first is not None, f"Provider did not start: {evidence}"
                assert first[1] == ["resume", session], first
                assert json.loads(managed.stdout)["launched"] == 1, managed.stdout
                ready = marker.with_suffix(".hook")
                deadline = time.monotonic() + 15
                while not ready.exists() and time.monotonic() < deadline:
                    time.sleep(0.02)
                assert ready.exists(), f"SessionStart did not complete: {evidence}"
                evidence["session_hook_ms"] = round((json.loads(ready.read_text()) - start) * 1000, 1)
            log = scope / ".tmp/logs/agentx.log"
            if log.is_file():
                rows = map(json.loads, log.read_text().splitlines())
                evidence["slow_dispatch"] = [
                    {key: row[key] for key in ("operation", "queue_wait_ms", "duration_ms", "post_commit_work") if key in row}
                    for row in rows if row["event"] == "dispatch.slow"
                ]
            return evidence
        finally:
            if socket.exists():
                subprocess.run(["tmux", "-S", socket, "kill-server"], env=env, capture_output=True, timeout=5)
            if daemon.poll() is None:
                daemon.terminate()
                try:
                    daemon.communicate(timeout=5)
                except subprocess.TimeoutExpired:
                    daemon.kill()
                    daemon.communicate()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binaries", type=lambda path: Path(path).resolve(strict=True), nargs="+")
    parser.add_argument("--kv", type=int, default=105_000)
    parser.add_argument("--events", type=int, default=745)
    args = parser.parse_args()
    if not shutil.which("tmux"):
        parser.error("tmux must be installed")
    for binary in args.binaries:
        print(json.dumps(probe(binary, args.kv, args.events)), flush=True)


if __name__ == "__main__":
    main()
