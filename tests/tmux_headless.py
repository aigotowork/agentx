#!/usr/bin/env python3
"""Exercise real tmux lifecycle using isolated AgentX scopes and a local provider fixture."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import signal
import sqlite3
import subprocess
import tempfile
import termios
import time


PROVIDER = r'''#!/usr/bin/env python3
import json, os, select, sys, time
from pathlib import Path
if "--version" in sys.argv:
    print("qodercli 1.1.65")
    raise SystemExit
probe = Path(sys.argv[sys.argv.index("--fixture-probe") + 1])
probe.write_text(json.dumps({"pid": os.getpid(), "parent_pid": os.getppid(), "name": os.environ["AGENTX_INSTANCE_NAME"], "tty": os.isatty(0), "host": {k: os.environ.get(k) for k in ["TMUX", "TMUX_PANE", "AGENTX_LAUNCHED_PRESET"]}}))
print("TMUX_FIXTURE_READY", flush=True)
counter = 0
while True:
    probe.with_suffix(".next").write_text(str(counter))
    probe.with_suffix(".next").replace(probe.with_suffix(".tick"))
    counter += 1
    if select.select([sys.stdin], [], [], 0.1)[0]:
        line = sys.stdin.readline()
        if not line or line.strip() == "exit":
            break
        print("ECHO:" + line.rstrip(), flush=True)
'''


def eventually(fn, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = fn()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError("condition was not observed before timeout")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    binary = str(args.binary.resolve())
    tmux = shutil.which("tmux")
    assert tmux, "real tmux is required"
    with tempfile.TemporaryDirectory(prefix="axtm-") as directory:
        root = Path(directory)
        bindir = root / "bin"
        bindir.mkdir()
        fixture = bindir / "qodercli"
        fixture.write_text(PROVIDER)
        fixture.chmod(0o755)
        scopes = []
        children = []
        hosts = []

        def record_process(info):
            children.append(info["pid"])
            hosts.append(info["parent_pid"])

        def running(pid):
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                return False
            try:
                return Path(f"/proc/{pid}/stat").read_text().split(") ", 1)[1].split()[0] != "Z"
            except (FileNotFoundError, IndexError):
                return False


        def command(env, *words, ok=True):
            result = subprocess.run([binary, *words], env=env, cwd=root, text=True, capture_output=True, timeout=30)
            if ok:
                assert result.returncode == 0, (words, result.returncode, result.stdout, result.stderr)
            return result

        def native(socket, *words):
            return subprocess.run([tmux, "-S", str(socket), *words], text=True, capture_output=True, timeout=5)

        try:
            for index in range(2):
                scope = root / f"scope{index}"
                home = root / f"home{index}"
                scope.mkdir()
                home.mkdir()
                env = {
                    "PATH": str(bindir) + os.pathsep + os.environ["PATH"], "HOME": str(home),
                    "SHELL": "/bin/bash", "TERM": "xterm-256color", "LANG": "C.UTF-8", "CI": "1",
                    "AGENTX_DIR": str(scope), "AGENTX_TEST_DB_NOSYNC": "1", "TMUX_FIXTURE_PROBE": str(scope / "probe.json"),
                    "XDG_CONFIG_HOME": str(home / ".config"), "XDG_CACHE_HOME": str(home / ".cache"),
                }
                socket = scope / ("server dir/socket" if index else ".tmp/tmux/socket")
                if index:
                    command(env, "config", "tmux_socket", "unused.sock")
                    env["AGENTX_TMUX_SOCKET"] = "server dir/socket"
                else:
                    env["TMUX"] = str(root / "parent.sock") + ",123,0"
                    env["TMUX_PANE"] = "%99"
                scopes.append((scope, env, socket))
                command(env, "config", "terminal", "wezterm")
                command(env, "config", "headless_terminal", "tmux")
                (home / ".tmux.conf").write_text('set -g @fixture_user_config "loaded"\n')
                if not index:
                    (scope / "tmux.conf").write_text("set -g history-limit 23456\n")
                    command(env, "config", "tmux_config", "tmux.conf")
                assert "tmux" in command(env, "config", "headless_terminal").stdout
                bad = command(env, "launch", "--tool", "qoder", "--headless", "--terminal", "iterm", "--go", ok=False)
                assert bad.returncode and "headless terminal" in bad.stderr, bad
                bad = command(env, "launch", "--tool", "qoder", "--headless", "--run-here", "--go", ok=False)
                assert bad.returncode and "--run-here" in bad.stderr, bad
                bad = command(env, "launch", "--tool", "qoder", "--terminal", "tmux", "--run-here", "--go", ok=False)
                assert bad.returncode and "--run-here" in bad.stderr, bad
                command(env, "launch", "--tool", "qoder", "--headless", "--go", "--json", "--", "--fixture-probe", str(scope / "probe.json"))
                probe = scope / "probe.json"
                eventually(lambda: probe.exists())
                info = json.loads(probe.read_text())
                record_process(info)
                assert info["tty"], info
                assert info["host"]["TMUX"].rsplit(",", 2)[0] == str(socket), info
                assert socket.exists()
                assert socket.parent.stat().st_mode & 0o777 == 0o700
                if not index:
                    assert native(socket, "show-option", "-gv", "history-limit").stdout.strip() == "23456"
                else:
                    assert native(socket, "show-option", "-gv", "history-limit").stdout.strip() == "100000"
                    assert native(socket, "show-option", "-gv", "mouse").stdout.strip() == "on"
                assert native(socket, "show-option", "-gv", "@fixture_user_config").returncode != 0
                assert native(socket, "list-sessions", "-F", "#{session_name}").stdout.startswith("agentx-")

            scope, env, socket = scopes[0]
            info = json.loads((scope / "probe.json").read_text())
            # Attachment follows retained host metadata, even after defaults change.
            attach_env = dict(env, AGENTX_TMUX_SOCKET="not-the-running-server.sock")
            attach_env.pop("TMUX", None)
            attach_env.pop("TMUX_PANE", None)
            def terminal_exchange(words, terminal_env, text, detached=False, current=False):
                # Real controlling TTY: exercise foreground launch and manual attach.
                master, slave = pty.openpty()

                def terminal_child():
                    os.setsid()
                    fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

                client = subprocess.Popen([binary, *words], env=terminal_env, cwd=root, stdin=slave, stdout=slave, stderr=slave, preexec_fn=terminal_child)
                os.close(slave)
                screen = bytearray()
                try:
                    if detached:
                        assert client.wait(timeout=10) == 0
                        while select.select([master], [], [], 0)[0]:
                            try:
                                screen.extend(os.read(master, 65536))
                            except OSError:
                                break
                        return screen.decode(errors="replace")

                    def received(marker=b"TMUX_FIXTURE_READY"):
                        if select.select([master], [], [], 0.05)[0]:
                            try:
                                screen.extend(os.read(master, 65536))
                            except OSError:
                                raise AssertionError(screen.decode(errors="replace"))
                        assert client.poll() is None, screen.decode(errors="replace")
                        return marker in screen

                    eventually(received)
                    os.write(master, (text + "\r").encode())
                    eventually(lambda: received(("ECHO:" + text).encode()))
                    os.write(master, b"exit\r" if current else b"\x02d")
                    assert client.wait(timeout=10) == 0, screen.decode(errors="replace")
                finally:
                    if client.poll() is None:
                        client.terminate()
                        client.wait(timeout=10)
                    os.close(master)

            for attempt in range(2):
                tick_before = int((scope / "probe.tick").read_text())
                terminal_exchange(["attach", info["name"]], attach_env, f"attach-input-{attempt}")
                eventually(lambda: int((scope / "probe.tick").read_text()) > tick_before + 2)
                os.kill(info["pid"], 0)

            # Explicit terminal overrides the single-human default. Both paths
            # use the same scoped socket, including no-run-here and JSON calls.
            foreground_probe = scope / "foreground.json"
            foreground_env = dict(attach_env, AGENTX_TMUX_SOCKET=str(socket), TMUX_FIXTURE_PROBE=str(foreground_probe))
            terminal_exchange(["launch", "--tool", "qoder", "--terminal", "tmux", "--go", "--", "--fixture-probe", str(foreground_probe)], foreground_env, "foreground-input")
            foreground = json.loads(foreground_probe.read_text())
            record_process(foreground)
            assert foreground["host"]["TMUX"].rsplit(",", 2)[0] == str(socket)
            tick_before = int(foreground_probe.with_suffix(".tick").read_text())
            eventually(lambda: int(foreground_probe.with_suffix(".tick").read_text()) > tick_before + 2)
            terminal_exchange(["attach", foreground["name"]], foreground_env, "foreground-reattach")
            os.kill(foreground["pid"], 0)
            assert native(socket, "kill-pane", "-t", foreground["host"]["TMUX_PANE"]).returncode == 0

            for suffix, extra in [("detached", ["--no-run-here"]), ("json", ["--json"]), ("pipe", [])]:
                detached_probe = scope / f"{suffix}.json"
                detached_env = dict(foreground_env, TMUX_FIXTURE_PROBE=str(detached_probe))
                words = ["launch", "--tool", "qoder", "--terminal", "tmux", "--go", *extra, "--", "--fixture-probe", str(detached_probe)]
                if suffix in ("detached", "json"):
                    output = terminal_exchange(words, detached_env, "", detached=True)
                else:
                    output = command(detached_env, *words).stdout
                if suffix == "json":
                    report_lines = [line for line in output.splitlines() if line.startswith("{")]
                    assert len(report_lines) == 1, output
                    assert json.loads(report_lines[0])["launched"] == 1, output
                eventually(lambda: detached_probe.exists())
                detached = json.loads(detached_probe.read_text())
                record_process(detached)
                assert detached["host"]["TMUX"].rsplit(",", 2)[0] == str(socket)
                assert native(socket, "kill-pane", "-t", detached["host"]["TMUX_PANE"]).returncode == 0

            # Configured new-window terminal (wezterm) must not override the
            # ordinary single-human default, which reuses this controlling TTY.
            current_probe = scope / "current.json"
            current_env = dict(foreground_env, TMUX_FIXTURE_PROBE=str(current_probe))
            terminal_exchange(["launch", "--tool", "qoder", "--go", "--", "--fixture-probe", str(current_probe)], current_env, "current-input", current=True)
            current_info = json.loads(current_probe.read_text())
            assert current_info["tty"] and not current_info["host"]["TMUX"], current_info

            # The second scope is an independent live tmux server/session.
            assert native(scopes[1][2], "list-sessions").returncode == 0
            assert native(socket, "new-session", "-d", "-s", "fixture-neighbor", "--", "/bin/bash", "--noprofile", "--norc").returncode == 0
            assert native(socket, "kill-pane", "-t", info["host"]["TMUX_PANE"]).returncode == 0
            eventually(lambda: native(socket, "list-sessions", "-F", "#{session_name}").stdout.strip() == "fixture-neighbor")
            assert native(scopes[1][2], "list-sessions").returncode == 0
            # CLI override retains native PTY even when tmux is configured.
            native_probe = scope / "native.json"
            native_env = dict(env, TMUX_FIXTURE_PROBE=str(native_probe))
            command(native_env, "launch", "--tool", "qoder", "--headless", "--terminal", "pty", "--go", "--json", "--", "--fixture-probe", str(native_probe))
            eventually(lambda: native_probe.exists())
            native_info = json.loads(native_probe.read_text())
            record_process(native_info)
            # Stop only the fixture provider. Public operator kill/screen fencing
            # is a separate regression; no user process or server is involved.
            os.kill(native_info["pid"], signal.SIGTERM)
            print("PASS: foreground launch/reattach, detached variants, config, conflicts, scoped servers, retained PID/socket, neighbor, native override")
        except Exception:
            for scope, env, socket in scopes:
                probe = scope / "probe.json"
                if probe.exists():
                    print("PROBE", probe.read_text(), flush=True)
                database = scope / "agentx.db"
                if database.exists():
                    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as db:
                        print("HOST", db.execute("SELECT name, terminal_preset_effective, launch_context FROM instances").fetchall(), flush=True)
            raise
        finally:
            for scope, env, socket in scopes:
                native(socket, "kill-server")
            for child in children:
                try:
                    os.kill(child, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            # PTY shutdown finalizes through the daemon. Let hosts exit first;
            # stopping the daemon earlier can make late finalizers start it again.
            eventually(lambda: all(not running(host) for host in hosts))
            for scope, env, socket in scopes:
                status = command(env, "daemon", "status", "--scope", str(scope), ok=False)
                match = re.search(r"^Peer: pid=(\d+) ", status.stdout, re.MULTILINE)
                if match:
                    daemon = int(match[1])
                    try:
                        os.kill(daemon, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    eventually(lambda: not running(daemon))



if __name__ == "__main__":
    main()
