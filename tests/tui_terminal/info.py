#!/usr/bin/env python3
"""Exercise Agent Info through a real dashboard, scoped daemon and mock provider."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from roles_workspaces import Fixture


def inspect(binary, output, daemon_binary=None):
    tmux = shutil.which("tmux")
    if not tmux:
        raise RuntimeError("tmux is required")
    fixture = Fixture(daemon_binary or binary, {})
    socket = fixture.root / "info-tmux"
    output.mkdir(parents=True, exist_ok=True)

    def terminal(*args, check=True):
        return subprocess.run([tmux, "-S", socket, *args], cwd=fixture.a, env=fixture.env,
                              text=True, capture_output=True, timeout=10, check=check).stdout

    def capture():
        return terminal("capture-pane", "-pt", "reader")

    def wait(label, predicate):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            screen = capture()
            if predicate(screen):
                (output / (label + ".txt")).write_text(screen)
                return screen
            time.sleep(0.05)
        raise AssertionError(f"Timed out at {label}\n{screen}")

    def open_row(label):
        for _ in range(40):
            screen = capture()
            rows = [line.partition("│")[2] for line in screen.splitlines()]
            focused = next((i for i, line in enumerate(rows) if "❯" in line), -1)
            target = next((i for i, line in enumerate(rows) if f"› {label}" in line), None)
            if focused == target:
                terminal("send-keys", "-t", "reader", "Enter")
                return
            terminal("send-keys", "-t", "reader", "Up" if target is not None and target < focused else "Down")
            time.sleep(0.08)
        raise AssertionError(f"Cannot focus {label}\n{screen}")

    try:
        (fixture.root / "bin/tmux").symlink_to(tmux)
        task = fixture.a / ".controller/demo.task.md"
        task.parent.mkdir()
        task.write_text("---\nschema: task\nstatus: ongoing\ndescription: Native reader task\n---\nInspect the assigned task body.\n")
        fixture.launch("--dir", fixture.a, "--role", "controller", "--bind", task, "--title", "Native Info")
        name = fixture.providers()[0]["name"]
        if daemon_binary:
            before = fixture.run("daemon", "status")
            (output / "daemon-before.txt").write_text(before)
            fixture.role(fixture.a / ".agentx/roles", "controller", "CHANGED_ON_DISK", default=True)
            config = fixture.scope / "config.toml"
            config.write_text(config.read_text().replace("role-model", "changed-model"))
            # Same name alone, or even the same incarnation bits in another
            # generation, must not add a link to this Agent's Info.
            task_source = task.read_text()
            agents_line = next(line for line in task_source.splitlines() if line.startswith("agents: "))
            assignments = json.loads(agents_line.removeprefix("agents: "))
            assignments[0]["generation"] = "00000000-0000-0000-0000-000000000001"
            (task.parent / "namesake.task.md").write_text(
                task_source.replace(agents_line, "agents: " + json.dumps(assignments))
                .replace("Native reader task", "Another generation's task"))
        fixture.parsed("bindings", "--project", fixture.a, "--agent", name, "--refresh")
        fixture.env["AGENTX_TUI_FULLSCREEN"] = "1"
        terminal("new-session", "-d", "-s", "reader", "-x", "128", "-y", "32", str(binary))
        wait("dashboard", lambda s: name in s)
        terminal("send-keys", "-t", "reader", "i")
        wait("info", lambda s: "controller" in s and "deep" in s and "Native reader task" in s)
        open_row("Role")
        wait("role", lambda s: "Instructions" in s and "deep" in s)
        open_row("Instructions")
        wait("instructions", lambda s: "PARENT_ROLE" in s)
        terminal("send-keys", "-t", "reader", "Escape")
        wait("role-return", lambda s: "Instructions" in s and "deep" in s)
        open_row("Profile")
        wait("profile", lambda s: "codex" in s and "Arguments" in s)
        open_row("Arguments")
        wait("arguments", lambda s: '"role-model"' in s and "changed-model" not in s)
        terminal("send-keys", "-t", "reader", "Escape")
        wait("arguments-return", lambda s: "codex" in s and "Arguments" in s)
        terminal("send-keys", "-t", "reader", "Escape")
        wait("profile-return", lambda s: "Instructions" in s and "deep" in s)
        terminal("send-keys", "-t", "reader", "Escape")
        wait("back", lambda s: "Native reader task" in s)
        open_row("Bootstrap")
        if daemon_binary:
            wait("bootstrap", lambda s: "full frozen Bootstrap" in s and "PARENT_ROLE" in s)
        else:
            wait("bootstrap", lambda s: "Role: controller" in s or "AGENTX SESSION" in s)
        terminal("send-keys", "-t", "reader", "Escape")
        wait("root-return", lambda s: "Native reader task" in s)
        open_row("Task")
        wait("task", lambda s: "demo.task.md" in s and "Body" in s)
        open_row("Body")
        wait("body", lambda s: "Inspect the assigned task body." in s)
        terminal("send-keys", "-t", "reader", "i")
        wait("returned", lambda s: name in s and "i Exit" not in s)

        actor = fixture.providers()[0]
        fixture.run("launch", "--tool", "codex", "--title", "Related child",
                    "--headless", "--go", "--json",
                    extra={"AGENTX_PROCESS_ID": actor["process"], "AGENTX_INSTANCE_NAME": name})
        child = next(p["name"] for p in fixture.wait_providers(2) if p["name"] != name)
        wait("updated-list", lambda s: child in s)
        terminal("send-keys", "-t", "reader", "i")
        wait("parent", lambda s: "Child" in s and "Native reader task" in s)
        open_row("Child")
        wait("child", lambda s: "Related child" in s and "From" in s)
        open_row("From")
        wait("from-child", lambda s: "Native reader task" in s and "Child" in s)
        terminal("send-keys", "-t", "reader", "Escape")
        wait("back-to-child", lambda s: "Related child" in s and "From" in s)

        if daemon_binary:
            fixture.run("kill", child)
            terminal("send-keys", "-t", "reader", "i")
            wait("child-stopped-list", lambda s: "1 stopped" in s and "i Exit" not in s)
            terminal("send-keys", "-t", "reader", "i")
            wait("child-stopped-relation", lambda s: "Child" in s and "Native reader task" in s)
            open_row("Child")
            wait("child-stopped-exact", lambda s: "Related child" in s and "› From" in s)
            open_row("From")
            wait("from-stopped-child", lambda s: "Native reader task" in s and "Child" in s)
            terminal("send-keys", "-t", "reader", "i")
            terminal("send-keys", "-t", "reader", "C-s")
            wait("stops-expanded", lambda s: "▾ 1 stopped" in s and f"{child}  stopped" in s)
            terminal("send-keys", "-t", "reader", "Down", "Down", "i")
            wait("child-stopped-snapshot", lambda s: "Related child" in s and "› From" in s)
            after = fixture.run("daemon", "status")
            (output / "daemon-after.txt").write_text(after)
            assert before == after, (before, after)
            return

        # Reuse the live fixture on the ordinary inline dashboard. The reader
        # should borrow the event area, then return to the original main buffer.
        terminal("send-keys", "-t", "reader", "i")
        wait("relations-closed", lambda s: "i Exit" not in s)
        terminal("send-keys", "-t", "reader", "\\")
        wait("inline-events", lambda s: "│" not in s and name in s and child in s)
        terminal("send-keys", "-t", "reader", "i")

        def reader_above_agents(screen):
            lines = screen.splitlines()
            header = next((n for n, line in enumerate(lines) if "Info ·" in line), None)
            agents = next((n for n, line in enumerate(lines) if name in line and child in line), None)
            return header == 0 and agents is not None and agents > len(lines) // 2 and "Bootstrap" in screen

        wait("inline-upper-info", reader_above_agents)
        terminal("send-keys", "-t", "reader", "i")
        wait("inline-events-restored", lambda s: "i Exit" not in s and name in s and child in s)
    finally:
        terminal("kill-server", check=False)
        fixture.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--daemon-binary", type=Path, help="Use an older binary to launch and retain a disposable daemon")
    args = parser.parse_args()
    inspect(args.binary.resolve(strict=True), args.output, args.daemon_binary.resolve(strict=True) if args.daemon_binary else None)
