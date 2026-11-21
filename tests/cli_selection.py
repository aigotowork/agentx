#!/usr/bin/env python3
"""Finite Linux/macOS CLI selection checks; pass an exact built agentx binary.

Usage: python3 tests/cli_selection.py /absolute/path/to/agentx
All invocations use disposable scopes and dry-run; no Provider or user daemon runs.
"""

import fcntl
import json
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time


def interactive(binary, argv, steps, cwd, env):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 110, 0, 0))
    original = termios.tcgetattr(slave)
    def attach_controlling_terminal():
        os.setsid()
        fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

    process = subprocess.Popen([binary, *argv], stdin=slave, stdout=subprocess.PIPE, stderr=slave, cwd=cwd, env=env, preexec_fn=attach_controlling_terminal)
    screen = bytearray()
    cursor_replies = 0

    def read_available(timeout):
        nonlocal cursor_replies
        if select.select([master], [], [], timeout)[0]:
            screen.extend(os.read(master, 65536))
            # A PTY carries bytes but does not emulate a terminal. Ratatui's
            # initialization asks for the cursor position; answer each DSR,
            # including a query split across reads, while retaining raw output.
            queries = screen.count(b"\x1b[6n")
            while cursor_replies < queries:
                os.write(master, b"\x1b[1;1R")
                cursor_replies += 1

    try:
        offset = 0
        for marker, keys in steps:
            deadline = time.monotonic() + 10
            while marker not in screen[offset:]:
                if time.monotonic() >= deadline or process.poll() is not None:
                    raise AssertionError(f"Missing {marker!r}: argv={argv!r}, screen={screen.decode(errors='replace')}")
                read_available(0.1)
            offset = len(screen)
            os.write(master, keys)
        deadline = time.monotonic() + 10
        while process.poll() is None and time.monotonic() < deadline:
            read_available(0.1)
        stdout, _ = process.communicate(timeout=1)
        while select.select([master], [], [], 0)[0]:
            read_available(0)
        assert termios.tcgetattr(slave) == original, "Terminal attributes were not restored"
        assert b"\x1b[?1049h" in screen and b"\x1b[?1049l" in screen, "Alternate screen was not restored"
        assert b"\x1b" not in stdout, "Picker output leaked to stdout"
        if process.returncode == 130:
            assert b"Tool:" not in stdout, "Cancelled selection continued into launch"
        return process.returncode, stdout.decode(), bytes(screen)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=2)
        os.close(master)
        os.close(slave)


def main():
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    passed = []
    with tempfile.TemporaryDirectory(prefix="agentx-selection-") as temporary:
        root = Path(temporary)
        scope = root / "scope"
        workspace = root / "workspace"
        roles = workspace / ".agent-guides" / "roles"
        for path in [scope, roles, root / "home", root / "tmp"]:
            path.mkdir(parents=True)
        (scope / "config.toml").write_text('[launch]\ndefault_profile = "fast"\n[launch.profiles.fast]\ntool = "codex"\ndescription = "Quick coding"\n[launch.profiles.other]\ntool = "claude"\ndescription = "Review work"\n')
        overrides = workspace / ".agentx" / "roles"
        overrides.mkdir(parents=True)
        for name, profile in [("coder", "fast"), ("reviewer", "other")]:
            (roles / f"{name}.md").write_text(f"---\nschema: role\ndescription: {name} responsibility\n---\nAct as {name}.\n")
            (overrides / f"{name}.toml").write_text(f'profile = "{profile}"\n')
        env = {"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "LANG": "C.UTF-8", "TERM": "xterm-256color", "CI": "1", "HOME": str(root / "home"), "TMPDIR": str(root / "tmp"), "AGENTX_DIR": str(scope)}

        def plain(argv, expected=0):
            result = subprocess.run([binary, *argv], input=b"", capture_output=True, cwd=workspace, env=env, timeout=10)
            assert (result.returncode == 0) == (expected == 0), (argv, result.returncode, result.stderr.decode())
            assert b"\x1b[?1049h" not in result.stderr, "Unexpected picker"
            return result

        for option in ["--role", "--profile", "--tool"]:
            result = plain(["l", option, "--dry-run"], expected=1)
            assert b"explicit value" in result.stderr, result.stderr
        passed.append("noninteractive missing values fail without prompting")
        for control in ["--help", "--help-all"]:
            result = plain(["l", "--role", control])
            assert b"--role [<ROLE>]" in result.stdout, result.stdout
        passed.append("help remains passive after bare selectors")
        result = plain(["l", "--dry-run", "--json"])
        assert json.loads(result.stdout)["configuration"]["profile"]["name"] == "fast", result.stdout
        result = plain(["l", "--role", "missing-role", "--dry-run", "--json"], expected=1)
        assert b"Unknown role" in result.stderr, result.stderr
        plain(["l", "--agentx-prompt", "--role", "--dry-run", "--json"])
        passed.append("defaults, explicit errors and opaque prompt values retain semantics")

        cases = [
            (["l", "--role", "--dry-run"], [(b"Select role", b"reviewer\r")], "Tool: claude", "Profile: other"),
            (["l", "--profile", "--tool", "codex", "--dry-run"], [(b"Select profile", b"Quick\r")], "Tool: codex", "Profile: fast"),
            (["l", "--tool", "--dry-run"], [(b"Select tool", b"codex\r")], "Tool: codex", "Profile:"),
            (["l", "--role", "--profile", "--tool", "--dry-run"], [(b"Select role", b"reviewer\r"), (b"Select profile", b"fast\r"), (b"Select tool", b"codex\r")], "Tool: codex", "Profile: fast"),
            (["l", "--role", "--dry-run"], [(b"Select role", b"no-such-choice\r"), (b"No matches", b"\x15coder\r")], "Tool: codex", "Profile: fast"),
        ]
        for argv, steps, tool, profile in cases:
            code, stdout, screen = interactive(binary, argv, steps, workspace, env)
            assert code == 0 and tool in stdout and profile in stdout, (argv, code, stdout, screen.decode(errors="replace"))
            if "--tool" in argv and "codex" in argv:
                assert b"other" not in screen, "Incompatible profile was offered"
            passed.append("PTY " + " ".join(argv))
        for option, cancel in [("--role", b"\x1b"), ("--profile", b"\x03")]:
            code, stdout, _ = interactive(binary, ["l", option, "--dry-run"], [(b"Select ", cancel)], workspace, env)
            assert code == 130 and not stdout, (code, stdout)
        passed.append("Escape and Ctrl+C cancel and restore terminal")
        assert not (scope / "agentx.db").exists(), "Selection/dry-run unexpectedly acquired a daemon database"
        passed.append("no Provider or daemon acquisition")
    print(json.dumps({"binary": binary, "passed": passed, "count": len(passed)}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
