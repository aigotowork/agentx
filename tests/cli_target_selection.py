#!/usr/bin/env python3
"""Finite lifecycle-picker acceptance with owned Qoder/tmux substitutes only.

Usage: python3 tests/cli_target_selection.py /absolute/path/to/candidate
The existing management fixture owns the disposable scope and process cleanup.
"""

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

from cli_selection import interactive

sys.path.insert(0, str(Path(__file__).parent / "agent_management"))
from fixture import Fixture, wait_for


def main():
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    evidence = Path(tempfile.mkdtemp(prefix="agentx-target-acceptance-")) / "run"
    fixture = Fixture(binary, evidence)
    tmux = shutil.which("tmux")
    if tmux:
        (fixture.root / "bin/tmux").symlink_to(tmux)
    passed = []
    try:
        http = fixture.prepare("primary")
        env = fixture.environment("primary")
        cwd = fixture.root / "primary/project"
        (cwd / ".git").mkdir()
        nested = cwd / "src"
        nested.mkdir()

        for command in ["attach", "kill", "resume", "fork"]:
            result = subprocess.run([binary, command], input=b"", capture_output=True, env=env, cwd=cwd, timeout=10)
            assert result.returncode != 0 and b"explicit target" in result.stderr, (command, result.stderr)
            assert b"\x1b[?1049h" not in result.stderr
        passed.append("missing non-TTY targets reject without prompting")
        for command in ["kill", "resume", "fork"]:
            result = subprocess.run([binary, command, "--json"], input=b"", capture_output=True, env=env, cwd=cwd, timeout=10)
            assert result.returncode != 0 and b"explicit target" in result.stderr, (command, result.stderr)
        passed.append("JSON requires an explicit target")
        for argv in [
            ["kill", "all", "--dir", "."],
            ["kill", "12345", "--label", "fixture=kill"],
            ["recover", "--all", "--workspace", "global"],
        ]:
            result = subprocess.run([binary, *argv], input=b"", capture_output=True, env=env, cwd=nested, timeout=10)
            assert result.returncode != 0 and b"cannot be used with" in result.stderr, (argv, result.stderr)
        for command in ["kill", "recover"]:
            result = subprocess.run([binary, command, "--dir", ".", "--label", "fixture=kill"], input=b"", capture_output=True, env=env, cwd=nested, timeout=10)
            assert result.returncode != 0 and b"noninteractive" in result.stderr, (command, result.stderr)
        passed.append("selection filters never imply bulk or noninteractive execution")

        fixture.cli("primary", "launch", ["launch", "--tool", "qoder", "--headless", "--no-run-here", "--title", "picker-live", "--json", "--agentx-prompt", "selection fixture"])
        def live():
            return next((view for view in http.agents() if view["title"] == "picker-live" and view["session_id"] and view["can_send"]), None)
        view = wait_for("selection fixture session bound", live)
        name = view["agent"]["name"]

        preview_env = {**env, "AGENTX_TOOL": "codex"}
        code, stdout, screen = interactive(binary, ["fork", "--title", "picker-preview"], [(b"Select session to fork", name.encode() + b"\r")], cwd, preview_env)
        assert code == 0 and b"qoder" in screen, (code, stdout, screen)
        assert len(http.agents()) == 1, "Fork preview unexpectedly launched a Provider"
        passed.append("active fork selection reaches existing preview without launch")

        # Review changes a previous frame incrementally, so wait for its
        # unique new key hint rather than a title split across ANSI updates.
        code, stdout, _ = interactive(binary, ["kill"], [(b"Select Agents to kill", name.encode() + b" \r"), (b"Backspace edit", b"\x1b")], cwd, env)
        assert code == 130 and not stdout and len(http.agents()) == 1
        passed.append("kill selection review can cancel without stopping the Agent")

        code, stdout, _ = interactive(binary, ["kill"], [(b"Select Agents to kill", name.encode() + b" \r"), (b"Backspace edit", b"\r")], cwd, env)
        assert code == 0 and name in stdout, (code, stdout)
        wait_for("selected Agent stopped", lambda: next((item for item in http.agents() if item["agent"]["name"] == name and item["status"] == "stopped"), None))
        passed.append("confirmed kill applies only the selected native target")

        code, stdout, _ = interactive(binary, ["resume", "--title", "picker-preview"], [(b"Select session to resume", name.encode() + b"\r")], cwd, preview_env)
        assert code == 0, (code, stdout)
        assert all(item["status"] == "stopped" for item in http.agents()), "Resume preview unexpectedly started a Provider"
        passed.append("stopped resume selection reaches existing preview")
        code, stdout, _ = interactive(binary, ["fork"], [(b"Select session to fork", b"\x03")], cwd, env)
        assert code == 130 and not stdout and all(item["status"] == "stopped" for item in http.agents())
        passed.append("stopped fork cancellation has no launch effect")

        first_title, second_title = "picker-kill-first", "picker-kill-second"
        titles = (first_title, second_title, "picker-kill-remaining")
        for title in titles:
            fixture.cli("primary", title, ["launch", "--tool", "qoder", "--headless", "--no-run-here", "--title", title, "--label", "fixture=kill", "--json", "--agentx-prompt", "multiple selection fixture"])

        def multiple_ready():
            agents = {item["title"]: item["agent"]["name"] for item in http.agents() if item["can_send"]}
            return [agents[title] for title in titles] if all(title in agents for title in titles) else None

        first, second, remaining = wait_for("three selection fixture sessions bound", multiple_ready)
        other_title = "picker-kill-other-workspace"
        fixture.cli("primary", "launch-other-workspace", ["launch", "--tool", "qoder", "--headless", "--no-run-here", "--workspace", str(nested), "--title", other_title, "--label", "fixture=kill", "--json", "--agentx-prompt", "different workspace fixture"])
        other = wait_for("other workspace fixture", lambda: next((item for item in http.agents() if item["title"] == other_title and item["can_send"]), None))["agent"]["name"]
        # The picker searches the whole row. A generated name like "nage" can
        # match every row's "agentx-management-..." directory; unique titles cannot.
        steps = [
            (b"Select Agents to kill", other_title.encode()),
            (b"No matches", b"\x15" + first_title.encode() + b" \x15" + second_title.encode() + b" \r"),
            (b"Backspace edit", b"\r"),
        ]
        code, stdout, _ = interactive(binary, ["k", "--dir", ".", "--label", "fixture=kill"], steps, nested, env)
        assert code == 0 and first in stdout and second in stdout, (code, stdout)
        wait_for("multi-selected Agents stopped", lambda: all(any(item["agent"]["name"] == name and item["status"] == "stopped" for item in http.agents()) for name in [first, second]) and any(item["agent"]["name"] == remaining and item["status"] != "stopped" for item in http.agents()))
        assert any(item["agent"]["name"] == other and item["status"] != "stopped" for item in http.agents())
        passed.append("filtered kill preserves selection and spares unselected Agents and other workspaces")

        if tmux:
            fixture.cli("primary", "launch-tmux", ["launch", "--tool", "qoder", "--headless", "--terminal", "tmux", "--title", "picker-tmux", "--label", "fixture=attach", "--json", "--agentx-prompt", "attach fixture"])
            tmux_view = wait_for("tmux selection fixture session bound", lambda: next((item for item in http.agents() if item["title"] == "picker-tmux" and item["can_send"]), None))
            tmux_name = tmux_view["agent"]["name"]
            listing = subprocess.run([binary, "list", "--dir", ".", "--label", "fixture=attach", "--names"], capture_output=True, text=True, env=env, cwd=nested, timeout=10, check=True)
            assert listing.stdout.splitlines() == [tmux_name], listing.stdout
            wrong_workspace = subprocess.run([binary, "attach", tmux_name, "--workspace", "."], capture_output=True, text=True, env=env, cwd=nested, timeout=10)
            assert wrong_workspace.returncode != 0 and "does not match the filters" in wrong_workspace.stderr
            code, stdout, screen = interactive(binary, ["a", "--dir", ".", "--label", "fixture=attach"], [(tmux_name.encode(), b"\x1b")], nested, env)
            assert code == 130 and not stdout
            assert b"Select Agent to attach" in screen
            assert any(item["agent"] == tmux_view["agent"] for item in http.agents())
            passed.append("attach and list share nested directory discovery and label filters; exact workspace never widens")
        else:
            passed.append("attach PTY unavailable: tmux is not installed")
    finally:
        fixture.close()
    result = {"binary": binary, "passed": passed, "evidence": str(evidence)}
    (evidence / "selection-result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
