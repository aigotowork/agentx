#!/usr/bin/env python3
"""Drive the built native dashboard through a real Unix PTY and, when available, tmux."""

import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shlex
import shutil
import struct
import subprocess
import tempfile
import termios
import time
import unicodedata


EMPTY = b"No agents running"
CPR_QUERY = b"\x1b[6n"
MODE = re.compile(rb"\x1b\[\?([\d;]+)([hl])")
MOUSE_MODES = {1000, 1002, 1003, 1005, 1006, 1015, 1016}
ERASE_SAVED_LINES = re.compile(rb"\x1b\[0*3J")
SHELL_HISTORY = "PTY_PREEXISTING_SHELL_HISTORY"
TIMEOUT = 10


def eventually(description, probe):
    deadline = time.monotonic() + TIMEOUT
    while time.monotonic() < deadline:
        if result := probe():
            return result
        time.sleep(0.025)
    raise AssertionError(f"Timed out waiting for {description}")


class Pty:
    """A controlling Unix PTY, with only an opt-in synthetic cursor reply."""

    def __init__(self, argv, cwd, env, output, *, reply_to_cursor=False):
        self.master, self.slave = pty.openpty()
        self.original = termios.tcgetattr(self.slave)
        self.resize(110, 24)
        self.output = output
        self.raw = bytearray()
        self.replies = 0
        self.reply_to_cursor = reply_to_cursor

        def controlling_terminal():
            os.setsid()
            fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)

        try:
            self.process = subprocess.Popen(
                argv, cwd=cwd, env=env, stdin=self.slave, stdout=self.slave,
                stderr=self.slave, preexec_fn=controlling_terminal,
            )
        except BaseException:
            os.close(self.master)
            os.close(self.slave)
            raise

    def __enter__(self):
        return self

    def __exit__(self, *_):
        try:
            if self.process.poll() is None:
                self.process.kill()
                self.process.wait(timeout=TIMEOUT)
            self.pump()
            self.output.write_bytes(self.raw)
        finally:
            os.close(self.master)
            os.close(self.slave)

    def resize(self, cols, rows):
        # TIOCSWINSZ signals the controlling foreground group's SIGWINCH.
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))

    def send(self, data):
        assert os.write(self.master, data) == len(data)

    def pump(self):
        while select.select([self.master], [], [], 0)[0]:
            self.raw.extend(os.read(self.master, 65536))
        if self.reply_to_cursor and self.process.poll() is None:
            queries = self.raw.count(CPR_QUERY)
            while self.replies < queries:
                self.send(b"\x1b[1;1R")
                self.replies += 1

    def wait_for(self, marker, since=0):
        def observed():
            self.pump()
            assert self.process.poll() is None, f"{self.output.name}: child exited before {marker!r}"
            return marker in self.raw[since:]

        eventually(f"{marker!r} in {self.output.name}", observed)

    def modes(self):
        enabled = set()
        for numbers, direction in MODE.findall(self.raw):
            for number in map(int, numbers.split(b";")):
                if direction == b"h":
                    enabled.add(number)
                else:
                    enabled.discard(number)
        return enabled

    def exit_cleanly(self):
        def exited():
            self.pump()
            return self.process.poll() is not None

        eventually(f"process exit for {self.output.name}", exited)
        self.pump()
        assert self.process.returncode == 0, self.process.returncode
        assert termios.tcgetattr(self.slave) == self.original, "TTY attributes were not restored"


def check_modes(terminal, fullscreen, check_inline_mouse):
    enabled = terminal.modes()
    assert (1049 in enabled) == fullscreen, enabled
    assert 2004 in enabled, "bracketed paste was not enabled"
    if fullscreen or check_inline_mouse:
        assert {1000, 1006} <= enabled, f"SGR mouse capture was not enabled: {enabled}"


def direct(binary, root, env, evidence, *, fullscreen, check_inline_mouse):
    name = "fullscreen" if fullscreen else "inline"
    if fullscreen:
        env = {**env, "AGENTX_TUI_FULLSCREEN": "1"}
    with Pty([binary], root, env, evidence / f"direct-{name}.ansi", reply_to_cursor=True) as terminal:
        terminal.wait_for(EMPTY)
        assert bool(terminal.replies) != fullscreen, "unexpected cursor query policy at startup"
        assert not termios.tcgetattr(terminal.slave)[3] & (termios.ECHO | termios.ICANON), "TTY did not enter raw mode"
        check_modes(terminal, fullscreen, check_inline_mouse)

        terminal.send(b"\t")
        terminal.wait_for(b"initial task for agent")
        terminal.send(b"\x1b[200~PTY_PASTE\x1b[201~")
        terminal.wait_for(b"PTY_PASTE")
        offset = len(terminal.raw)
        terminal.send(b"\x1b")
        terminal.wait_for(b"initial task for agent", offset)
        offset = len(terminal.raw)
        terminal.send(b"\x1b")
        terminal.wait_for(EMPTY, offset)

        for current_mode, size in [(fullscreen, (96, 21)), (not fullscreen, (83, 19))]:
            offset = len(terminal.raw)
            terminal.resize(*size)
            terminal.wait_for(EMPTY, offset)
            if not current_mode:
                assert CPR_QUERY in terminal.raw[offset:], "inline resize did not rebuild the viewport"
                if check_inline_mouse:
                    assert not ERASE_SAVED_LINES.search(terminal.raw, offset), "inline resize erased saved terminal lines"
            check_modes(terminal, current_mode, check_inline_mouse)
            if current_mode == fullscreen:
                offset = len(terminal.raw)
                terminal.send(b"\\")
                terminal.wait_for(EMPTY, offset)
                check_modes(terminal, not fullscreen, check_inline_mouse)

        terminal.send(b"\x04" if fullscreen else b"\x03")
        terminal.exit_cleanly()
        assert not terminal.modes() & (MOUSE_MODES | {1049, 2004}), terminal.modes()


def cell_of(screen, label):
    for y, line in enumerate(screen.splitlines(), start=1):
        if label in line:
            prefix = line[:line.index(label)]
            x = 1 + sum(0 if unicodedata.combining(c) else 2 if unicodedata.east_asian_width(c) in "WF" else 1 for c in prefix)
            return x, y
    raise AssertionError(f"Unable to find {label!r} in pane:\n{screen}")


def sgr(button, cell, suffix="M"):
    x, y = cell
    return f"\x1b[<{button};{x};{y}{suffix}".encode()


def tmux_smoke(tmux, binary, root, env, evidence, *, keyboard_only):
    config = root / "tmux.conf"
    config.write_text("set -g status off\nset -g mouse on\nset-window-option -g remain-on-exit on\n")
    prefix = [tmux, "-S", str(root / "tmux.sock"), "-f", str(config)]
    target = "smoke:0.0"
    pane_output = evidence / "tmux-pane-output.ansi"

    def command(*argv, check=True):
        result = subprocess.run([*prefix, *argv], cwd=root, env=env, capture_output=True, text=True, timeout=TIMEOUT)
        assert not check or result.returncode == 0, (argv, result.stderr)
        return result.stdout

    def pane(*flags):
        return command("capture-pane", "-p", "-t", target, *flags)

    def capture(name):
        (evidence / f"tmux-{name}.txt").write_text(pane())
        (evidence / f"tmux-{name}.ansi").write_text(pane("-e"))

    def info(fmt):
        return command("display-message", "-p", "-t", target, fmt).rstrip("\n")

    def resize(client, cols, rows, *, inline=False):
        offset = pane_output.stat().st_size
        client.resize(cols, rows)

        def app_painted():
            client.pump()
            with pane_output.open("rb") as stream:
                stream.seek(offset)
                raw = stream.read()
            start = raw.find(EMPTY)
            return start >= 0 and (not inline or (CPR_QUERY in raw[:start] and b"\x1b[?2026l" in raw[start:]))

        # tmux can repaint its pane before AgentX has processed SIGWINCH and
        # rebuilt the clickable regions. Wait for new bytes from AgentX itself.
        eventually("attached tmux resize", lambda: info("#{pane_width}x#{pane_height}") == f"{cols}x{rows}")
        eventually("dashboard repaint after tmux resize", app_painted)

    try:
        # Push a known shell line into the pane's saved history before exec.
        prelude = f"printf '%s\\n' {SHELL_HISTORY}; i=0; while [ \"$i\" -lt 30 ]; do printf '\\n'; i=$((i+1)); done; "
        command("new-session", "-d", "-s", "smoke", "-x", "110", "-y", "24", prelude + "exec " + shlex.quote(str(binary)))
        with Pty([*prefix, "attach-session", "-t", "smoke"], root, env, evidence / "tmux-client.ansi") as client:
            client.wait_for(EMPTY)
            command("pipe-pane", "-O", "-t", target, "cat > " + shlex.quote(str(pane_output)))
            eventually("tmux pipe-pane capture", pane_output.is_file)

            def await_pane(description, predicate):
                def observe():
                    client.pump()
                    screen = pane()
                    return screen if predicate(screen) else None

                return eventually(description, observe)

            def click(screen, label):
                cell = cell_of(screen, label)
                client.send(sgr(0, cell) + sgr(0, cell, "m"))

            assert info("#{alternate_on}") == "0"
            assert SHELL_HISTORY in pane("-S", "-") and SHELL_HISTORY not in pane(), "fixture did not seed saved shell lines"
            capture("inline")
            if not keyboard_only:
                resize(client, 96, 22, inline=True)
            for fullscreen in [False, True]:
                if fullscreen:
                    resize(client, 96, 15)
                screen = await_pane("dashboard footer", lambda screen: "? help" in screen and "Keyboard" not in screen)
                client.send(b"?") if keyboard_only else click(screen, "? help")
                help_screen = await_pane("help overlay", lambda screen: "Keyboard" in screen and "select all" in screen)
                if not keyboard_only:
                    assert "cancel" not in help_screen, help_screen
                    x, y = cell_of(help_screen, "select all")
                    client.send(sgr(65, (x, y)) * 2)
                    help_screen = await_pane("wheel reveals help content", lambda screen: "cancel" in screen)
                capture("help-fullscreen" if fullscreen else "help-inline")
                client.send(b"\x1b") if keyboard_only else click(help_screen, "[×]")
                screen = await_pane("help closed", lambda screen: "? help" in screen and "Keyboard" not in screen)
                if not fullscreen:
                    if not keyboard_only:
                        saved = pane("-S", "-")
                        (evidence / "tmux-inline-resized-history.txt").write_text(saved)
                        assert SHELL_HISTORY in saved, "inline resize lost saved shell lines in tmux"
                    client.send(b"\\") if keyboard_only else click(screen, "\\ view")
                    eventually("tmux alternate buffer", lambda: info("#{alternate_on}") == "1")
                    await_pane("fullscreen dashboard", lambda screen: "No agents running" in screen and "? help" in screen)
            capture("fullscreen-resized")

            client.send(b"\x04")
            eventually("dashboard exit in tmux", lambda: info("#{pane_dead}:#{pane_dead_status}") == "1:0")
            # Detach the owned client; the retained dead pane is evidence of app exit.
            client.send(b"\x02d")
            client.exit_cleanly()
    except BaseException:
        (evidence / "tmux-failure.txt").write_text(command("capture-pane", "-p", "-t", target, check=False))
        raise
    finally:
        command("kill-server", check=False)


@contextmanager
def fixture(binary, evidence):
    with tempfile.TemporaryDirectory(prefix="axtui-", dir="/tmp") as directory:
        root = Path(directory)
        scope, home, tmp = (root / name for name in ("scope", "home", "tmp"))
        for path in (scope, home, tmp):
            path.mkdir(mode=0o700)
        env = {
            "PATH": os.defpath, "TERM": "xterm-256color", "LANG": "C.UTF-8", "SHELL": "/bin/sh",
            "HOME": str(home), "TMPDIR": str(tmp), "CI": "1", "AGENTX_DIR": str(scope),
        }
        # Own the only daemon process in this scope. Wait for its authenticated
        # status before opening the dashboard so it never needs to auto-spawn one.
        with (evidence / "daemon.log").open("wb") as log:
            daemon = subprocess.Popen([binary, "daemon", "serve", "--scope", scope], cwd=root, env=env, stdin=subprocess.DEVNULL, stdout=log, stderr=log)
            try:
                def ready():
                    assert daemon.poll() is None, "isolated daemon exited; see daemon.log"
                    status = subprocess.run([binary, "daemon", "status"], cwd=root, env=env, capture_output=True, text=True, timeout=TIMEOUT)
                    return status.returncode == 0 and "Daemon: running\n" in status.stdout and f"Peer: pid={daemon.pid} " in status.stdout

                eventually("verified isolated daemon", ready)
                yield root, env
            finally:
                if daemon.poll() is None:
                    daemon.terminate()
                    try:
                        daemon.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        daemon.kill()
                        daemon.wait(timeout=TIMEOUT)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path, help="exact built agentx executable")
    parser.add_argument("--output", type=Path, required=True, help="new evidence directory (outside the repo recommended)")
    parser.add_argument("--require-tmux", action="store_true", help="fail instead of skipping if tmux is missing")
    parser.add_argument("--keyboard-only", action="store_true", help="check shared lifecycle against pre-inline-mouse builds")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    evidence = args.output.resolve()
    evidence.mkdir(parents=True)
    tmux = shutil.which("tmux")
    if args.require_tmux and not tmux:
        parser.error("tmux not found in PATH")
    with binary.open("rb") as candidate:
        digest = hashlib.file_digest(candidate, "sha256").hexdigest()
    result = {
        "binary": str(binary), "sha256": digest,
        "mode": "keyboard-baseline" if args.keyboard_only else "mouse-smoke",
        "tmux": subprocess.check_output([tmux, "-V"], text=True).strip() if tmux else "skipped: not installed",
        "passed": [],
    }
    try:
        with fixture(binary, evidence) as (root, env):
            for fullscreen in [False, True]:
                direct(binary, root, env, evidence, fullscreen=fullscreen, check_inline_mouse=not args.keyboard_only)
                result["passed"].append("direct fullscreen" if fullscreen else "direct inline")
            if tmux:
                tmux_smoke(tmux, binary, root, env, evidence, keyboard_only=args.keyboard_only)
                result["passed"].append("attached tmux")
    finally:
        receipt = json.dumps(result, indent=2) + "\n"
        (evidence / "result.json").write_text(receipt)
        print(receipt, end="")
        print(f"Evidence: {evidence}")


if __name__ == "__main__":
    main()
