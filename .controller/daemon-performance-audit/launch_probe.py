#!/usr/bin/env python3
"""Measure fresh local `agentx l` in disposable Linux scopes without model turns."""

import argparse
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import time


WRAPPER = r'''#!/usr/bin/python3
import hashlib, json, os, subprocess, sys, time, tomllib, uuid
from pathlib import Path

root = Path(__file__).parent.parent
args = sys.argv[1:]
phase = 'version' if '--version' in args or '-V' in args else 'preflight' if 'app-server' in args else 'provider'
def record(phase):
    with (root / 'markers.jsonl').open('a') as log:
        log.write(json.dumps([phase, time.monotonic()]) + '\n')
record(phase)
if REAL:
    os.execv(REAL, [REAL, *args])
if phase == 'version':
    print('codex-cli 0.171.0')
    sys.exit(0)
hooks = {}
for option, value in zip(args, args[1:]):
    if option == '-c' and value.startswith('hooks.') and not value.startswith('hooks.state='):
        hooks.update(tomllib.loads(value)['hooks'])
if phase == 'preflight':
    entries = [{'key': f'fixture:{event}:{i}:{j}', 'command': hook['command'], 'source': 'sessionFlags',
                'currentHash': hashlib.sha256(hook['command'].encode()).hexdigest()}
               for event, groups in hooks.items() for i, group in enumerate(groups) for j, hook in enumerate(group['hooks'])]
    for line in sys.stdin:
        request = json.loads(line)
        if 'id' in request:
            result = {'data': [{'hooks': entries}]} if request['method'] == 'hooks/list' else {}
            print(json.dumps({'id': request['id'], 'result': result}), flush=True)
    sys.exit(0)
if (root / 'suppress-ready').exists():
    time.sleep(22)
    sys.exit(0)
for group in hooks['SessionStart']:
    for hook in group['hooks']:
        subprocess.run(['/bin/sh', '-c', hook['command']], input=json.dumps({
            'source': 'startup', 'session_id': str(uuid.uuid4()), 'cwd': os.getcwd()
        }), text=True, capture_output=True, check=True, timeout=10)
record('hook-complete')
print('› \x1b[2mAsk Codex to do anything\x1b[0m', flush=True)
time.sleep(6)
'''


def probe(binary: Path, tmux: Path, codex: Path | None, cases: list[tuple], history: int) -> None:
    with tempfile.TemporaryDirectory(prefix="ax-launch-") as tmp:
        root = Path(tmp)
        scope, workspace, home, tools = (root / name for name in ("scope", "workspace", "home", "bin"))
        for path in (scope, workspace, home, tools, home / ".codex", root / "xdg"):
            path.mkdir(mode=0o700)
        provider = tools / "codex"
        provider.write_text(WRAPPER.replace("import hashlib", f"REAL = {str(codex) if codex else ''!r}\nimport hashlib", 1))
        provider.chmod(0o700)
        (tools / "agentx").symlink_to(binary)
        (tools / "tmux").symlink_to(tmux)
        (scope / "config.toml").write_text("[launch]\nauto_trust_workspace = true\n[preferences]\nauto_approve = false\n")
        (home / ".codex/config.toml").write_text('check_for_update_on_startup = false\nforced_login_method = "api"\n')
        path = f"{tools}:{codex.parent if codex else '/usr/bin'}:/usr/bin:/bin"
        # Zsh uses this private file only in the nested launch cases. Keep it
        # deterministic, with no personal startup scripts or Provider settings.
        (home / ".zshrc").write_text(f'export PATH="{path}"\n')
        env = {"PATH": path, "HOME": str(home), "CODEX_HOME": str(home / ".codex"),
               "AGENTX_DIR": str(scope), "TERM": "xterm-256color", "SHELL": "/bin/zsh",
               "XDG_RUNTIME_DIR": str(root / "xdg"), "OPENAI_API_KEY": "fixture-invalid-key",
               "OPENAI_BASE_URL": "http://127.0.0.1:9/v1"}
        socket = scope / ".tmp/tmux/socket"
        marker = root / "markers.jsonl"
        db = scope / "agentx.db"

        def run(args, environment=env):
            return subprocess.run(args, cwd=workspace, env=environment, stdin=subprocess.DEVNULL,
                                  capture_output=True, text=True, timeout=40)

        def read_ready(batch):
            with sqlite3.connect(f"{db.as_uri()}?mode=ro", uri=True) as conn:
                return conn.execute("SELECT 1 FROM events WHERE type='life' AND json_extract(data,'$.action')='ready' AND json_extract(data,'$.batch_id')=?", (batch,)).fetchone()

        def ms(t):
            return round((t - start) * 1000, 1)

        def daemon_status():
            return run([binary, "daemon", "status", "--scope", scope]).stdout

        def emit(row):
            print(json.dumps(row, ensure_ascii=False), flush=True)

        def stop_daemon(daemon):
            if daemon.poll() is None:
                daemon.terminate()
                try:
                    daemon.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    daemon.kill()
                    daemon.wait(timeout=5)

        def start_daemon(label):
            started = time.monotonic()
            daemon = subprocess.Popen([binary, "daemon", "serve", "--scope", scope], cwd=workspace, env=env,
                                      stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                deadline = started + 30
                while "Daemon: running" not in daemon_status():
                    assert daemon.poll() is None and time.monotonic() < deadline, "disposable daemon did not start"
                    time.sleep(0.025)
                emit({"case": label, "ms": round((time.monotonic() - started) * 1000, 1)})
                return daemon
            except BaseException:
                stop_daemon(daemon)
                raise

        def load_history():
            with sqlite3.connect(db) as conn:
                # Start over from the same existing schema without these optional
                # indexes, including for the candidate's one-time installation.
                conn.execute("DROP INDEX IF EXISTS agentx_sub_owner_v1")
                conn.execute("DROP INDEX IF EXISTS agentx_stopped_name_v1")
                ids = "WITH RECURSIVE ids(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM ids WHERE i<?)"
                conn.execute(ids + " INSERT INTO kv SELECT 'fixture:history:' || i, json_object('text',hex(zeroblob(256))) FROM ids", (history,))
                conn.execute(ids + " INSERT INTO events(timestamp,type,instance,data) SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now'), 'life', 'fixture', json_object('action','created','additional',hex(zeroblob(256))) FROM ids", (history,))
                conn.execute("INSERT INTO events(timestamp,type,instance,data) VALUES(strftime('%Y-%m-%dT%H:%M:%SZ','now'),'life','luna','{\"action\":\"stopped\"}')")

        daemon = None
        try:
            for name, mode, terminal, nested, suppress in cases:
                if not name.startswith("dry") and daemon is None:
                    daemon = start_daemon("daemon-start-until-status")
                    if history:
                        stop_daemon(daemon)
                        load_history()
                        daemon = start_daemon("history-daemon-start-until-status")
                marker.unlink(missing_ok=True)
                (root / "suppress-ready").unlink(missing_ok=True)
                if suppress:
                    (root / "suppress-ready").touch()
                launch_env = env | {"AGENTX_CODEX_TERMINAL": mode}
                if nested:
                    launch_env["AGENTX_LAUNCHED"] = "1"
                args = [binary, "l", "--tool", "codex", "--headless", "--terminal", terminal, "--go", "--json"]
                if name.startswith("dry"):
                    args.append("--dry-run")
                start = time.monotonic()
                result = run(args, launch_env)
                returned = time.monotonic()
                payload = json.loads(result.stdout) if result.stdout.strip() else {}
                batch = payload.get("batch_id")
                ready = None
                if batch and not suppress and codex:
                    end = returned + 6
                    while time.monotonic() < end:
                        if read_ready(batch):
                            ready = ms(time.monotonic())
                            break
                        time.sleep(0.01)
                if batch and not codex and not suppress:
                    deadline = returned + 5
                    while (not marker.exists() or '"hook-complete"' not in marker.read_text()) and time.monotonic() < deadline:
                        time.sleep(0.01)
                phases = [(phase, ms(t)) for phase, t in (json.loads(line) for line in marker.read_text().splitlines())] if marker.exists() else []
                readiness = payload.get("readiness") or {}
                emit({"case": name, "cli_ms": ms(returned), "ready_observed_ms": ready, "phases": phases,
                      "cli_readiness": readiness.get("status"), "exit": result.returncode,
                      "error": result.stderr.strip()[:450] if result.returncode and not readiness else None})
                assert result.returncode == (2 if suppress else 0), (name, result.stdout, result.stderr)
                if batch:
                    assert payload.get("launched") == 1 and any(phase == "provider" for phase, _ in phases), (name, phases)
                    if not codex and not suppress:
                        assert any(phase == "hook-complete" for phase, _ in phases), (name, phases)
                    if history:
                        assert "luna" not in payload["names"], "launch reused a historically reserved name"
        finally:
            if socket.exists():
                run([tmux, "-S", socket, "kill-server"])
            if daemon is not None:
                stop_daemon(daemon)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--codex", type=Path, help="use this real executable instead of the controlled Provider")
    parser.add_argument("--history", type=int, default=0, help="seed this many synthetic KV rows and life events before launch")
    args = parser.parse_args()
    if args.history < 0:
        parser.error("--history cannot be negative")
    tmux = Path(shutil.which("tmux")).resolve()
    print(json.dumps({"binary": str(args.binary.resolve()), "codex": str(args.codex) if args.codex else "fixture", "history": args.history}), flush=True)
    cases = [("dry-run", "pty", "tmux", False, False),
             ("warm-daemon/cold-hooks", "pty", "tmux", False, False),
             *[(f"warm/{i}", "pty", "tmux", False, False) for i in range(1, 4)],
             ("auto", "auto", "tmux", False, False)]
    if not args.codex and not args.history:
        cases += [("nested/ready-suppressed", "pty", "tmux", True, True)]
    probe(args.binary.resolve(), tmux, args.codex, cases, args.history)


if __name__ == "__main__":
    main()
