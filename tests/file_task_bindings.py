#!/usr/bin/env python3
"""Scoped file-binding acceptance using existing disposable launch fixtures.

Supply an explicit newly built candidate. No installed binary or live service is
modified. Native file cases remain in src/file_bindings/tests.rs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import time
import tomllib
import traceback
from urllib.request import urlopen
from urllib.parse import urlencode

from roles_workspaces import Fixture, FAKE_CODEX


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    result = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "cases": []}
    fixture = Fixture(binary, result)
    f = fixture
    task = f.root / ".controller/feature/unusual-name.md"
    task.parent.mkdir(parents=True)
    delivery = task.with_name("outcome.md")
    with (f.scope / "config.toml").open("a") as config:
        config.write("\n[management]\nenabled = true\nhost = \"127.0.0.1\"\nport = 0\nproject = " + json.dumps(str(f.root)) + "\n")
    original = "---\nschema: task\nstatus: planning\nagent: legacy-owner # retain\ncustom: {hello: world}\n---\nBody is preserved.\n"
    task.write_text(original)
    delivery.write_text("---\nschema: delivery\nstatus: planned\nstages: {implementation: {tasks: [./unusual-name.md]}}\n---\nDelivery body\n")

    def passed(name):
        result["cases"].append({"case": name, "status": "passed"})
        print(name + " passed", flush=True)

    def bindings(*extra):
        return f.parsed("bindings", "--project", f.root, *extra)

    def observed(path, projection):
        return next(entry for entry in projection["files"] if entry["path"] == str(path))

    def stop_disposable_daemon():
        # Positive exact scope ownership, not user daemon management or name lookup.
        candidates = []
        for process in Path("/proc").iterdir():
            if not process.name.isdigit():
                continue
            try:
                argv = (process / "cmdline").read_bytes().split(b"\0")
                if b"daemon" in argv and b"serve" in argv and str(f.scope).encode() in argv:
                    candidates.append(int(process.name))
            except OSError:
                pass
        assert len(candidates) == 1, candidates
        os.kill(candidates[0], signal.SIGTERM)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            try:
                command = Path(f"/proc/{candidates[0]}/cmdline").read_bytes()
            except OSError:
                command = b""
            if not command:
                return
            time.sleep(0.05)
        raise AssertionError("disposable daemon did not stop")

    try:
        preview = f.parsed("launch", "--tool", "codex", "--bind", task.relative_to(f.root), "--bind", delivery, "--dir", f.b, "--dry-run", "--json")
        assert preview["bindings"] == [str(task), str(delivery)], preview
        assert task.read_text() == original and not f.providers()
        passed("B01 caller-relative paths and child dir independence, dry-run no assignment")

        for extra in [("--device", "missing"), ("--stdio",), ("--stdio", "--", "--version")]:
            error = f.run("launch", "--tool", "codex", "--bind", task, *extra, ok=False)
            assert "--bind" in error and "unsupported" in error, error
        opaque_error = f.run("launch", "--tool", "codex", "--bind", "--env-file", "--dry-run", ok=False)
        assert "unexpected argument '--env-file'" in opaque_error, opaque_error
        error = f.run("resume", "missing", "--bind", task, ok=False)
        assert "--bind" in error, error
        missing = task.with_name("missing.md")
        error = f.run("launch", "--tool", "codex", "--bind", missing, "--headless", "--go", "--json", ok=False)
        assert "binding source" in error and not f.providers(), error
        passed("B02 unsupported contracts and missing source fail before Provider effects")

        launch_prompt = "Implement the specific assignment."
        f.launch("--bind", task.relative_to(f.root), "--bind", delivery, "--bind", task, "--dir", f.b,
                 "--workspace", f.a, "--role", "controller", "--title", "Implement assignment", "--agentx-prompt", launch_prompt)
        providers = f.providers()
        first = providers[-1]["name"]
        argv = providers[-1]["argv"]
        message, = (arg for arg in argv if arg.startswith(launch_prompt))
        assert original in message and "Delivery body\n" in message
        assert message.count(str(task)) == 1 and str(delivery) in message
        instructions, = (tomllib.loads(arg)["developer_instructions"] for arg in argv if arg.startswith("developer_instructions="))
        assert f"Workspace: {f.a}" in instructions and "Role: controller" in instructions
        assert f"Initial working directory: {f.b}" in instructions
        assert "Launch title: Implement assignment" in instructions
        assert "Body is preserved." not in instructions and "list self workspace" not in instructions
        projection = bindings("--refresh", "--agent", first)
        assert len(projection["files"]) == 2, projection
        document = observed(task, projection)["document"]
        assert document["status"] == "ongoing" and len(document["assignments"]) == 1, document
        assert document["agent"] == "legacy-owner"
        assert document["assignments"][0]["participant"]["canonical_name"] == first
        assert providers[-1]["cwd"] == str(f.b)
        assert "custom: {hello: world}" in task.read_text() and task.read_text().endswith("---\nBody is preserved.\n")
        passed("B03 assignment snapshots reach the native first message; stable facts reach developer instructions")

        f.launch("--tool", "codex", "--bind", task)
        current = bindings("--refresh")
        assignments = observed(task, current)["document"]["assignments"]
        assert len(assignments) == 2 and len({a["participant"]["canonical_name"] for a in assignments}) == 2
        second = next(a["participant"]["canonical_name"] for a in assignments if a["participant"]["canonical_name"] != first)
        second_provider = next(provider for provider in f.providers() if provider["name"] == second)
        assert any("Bound file " in arg and "Body is preserved.\n" in arg for arg in second_provider["argv"])
        assert len(bindings("--agent", second)["files"]) == 1
        passed("B04 shared file with two native participants and deduplicated repeated paths")

        descriptor_path = f.scope / ".daemon/management.json"
        deadline = time.monotonic() + 10
        while not descriptor_path.exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        endpoint = json.loads(descriptor_path.read_text())["url"]
        generation = assignments[0]["generation"]
        with urlopen(endpoint + "api/v1/agents?" + urlencode({"generation": generation}), timeout=10) as response:
            management = json.load(response)
        view = next(view for view in management["agents"] if view["agent"]["name"] == first)
        assert {entry["path"] for entry in view["file_bindings"]} == {str(task), str(delivery)}, view
        assert next(entry for entry in view["file_bindings"] if entry["path"] == str(task))["status"] == "ongoing"
        passed("B04a management HTTP exposes exact-participant matching file states")

        # Trigger the real lifecycle callback after editing work state; no explicit refresh.
        task.write_text(task.read_text().replace("status: planning", "status: paused"))
        time.sleep(1.1)
        first_provider = next(p for p in f.providers() if p["name"] == first)
        actor_env = dict(f.env, AGENTX_PROCESS_ID=first_provider["process"], AGENTX_INSTANCE_NAME=first)
        status = subprocess.run([str(binary), "codex-stop"], cwd=f.root, env=actor_env, text=True, capture_output=True, input=json.dumps({"session_id": first_provider["session"], "cwd": str(f.b)}), timeout=15)
        result["lifecycle_status"] = {"code": status.returncode, "stdout": status.stdout, "stderr": status.stderr}
        assert status.returncode == 0, result["lifecycle_status"]
        assert observed(task, bindings())["document"]["status"] == "paused"
        passed("B05 selected lifecycle refresh observes direct file work-state edit")

        stop_disposable_daemon()
        rebuilt = bindings("--refresh")
        assert observed(task, rebuilt)["document"]["assignments"] == assignments
        assert observed(task, rebuilt)["document"]["status"] == "paused"
        passed("B06 daemon memory discarded and associations rebuilt from .controller files")

        saved = task.read_text()
        task.write_text("---\nschema: task\nstatus: [\n---\n")
        stale = observed(task, bindings("--refresh"))
        assert stale["diagnostic"] and stale["last_success_at"] and stale["document"]["status"] == "paused"
        task.unlink()
        absent = observed(task, bindings("--refresh"))
        assert absent["diagnostic"] and absent["last_success_at"]
        task.write_text(saved)
        assert observed(task, bindings("--refresh"))["diagnostic"] is None
        passed("B07 malformed/missing/recovered reads retain distinguishable last success")

        # Synchronous terminal selection fails after reservation/binding, before child starts.
        before = len(f.providers())
        error = f.run("launch", "--tool", "codex", "--bind", task, "--terminal", "file-binding-terminal-does-not-exist", "--no-run-here", "--go", "--json", ok=False)
        assert error and len(f.providers()) == before
        assert observed(task, bindings("--refresh"))["document"]["assignments"] == assignments
        passed("B08 synchronous launch failure compensates its own assignment")

        # Change a validated source during Provider version preflight. Binding must
        # fail, compensate the earlier file, and never spawn a new Provider.
        breaker = delivery.with_name("break-during-preflight.md")
        breaker.write_text(original)
        fake = f.root / "bin/codex"
        injected = "import pathlib\np = pathlib.Path(" + repr(str(breaker)) + ")\nif any(a in ('--version', '-V') for a in sys.argv[1:]) and p.is_file():\n    p.unlink()\n    p.mkdir()\n"
        fake.write_text(FAKE_CODEX.replace("if any(arg in", injected + "if any(arg in", 1))
        error = f.run("launch", "--tool", "codex", "--bind", task, "--bind", breaker, "--headless", "--go", "--json", ok=False)
        assert "assignment failed" in error and len(f.providers()) == before, error
        assert observed(task, bindings("--refresh"))["document"]["assignments"] == assignments
        fake.write_text(FAKE_CODEX)
        passed("B09 launch file-write failure and partial multi-file compensation")

        with sqlite3.connect(f.scope.joinpath("agentx.db").as_uri() + "?mode=ro", uri=True) as connection:
            names = [name for (name,) in connection.execute("SELECT name FROM sqlite_master WHERE type='table'")]
            result["database_tables"] = names
            assert not any("file_bind" in name or "task_bind" in name for name in names)
        assert not list(task.parent.glob("*.lock"))
        passed("B10 no file-binding tables or sidecar index/locks")
        previous = len(f.providers())
        f.run("kill", first, "--json")
        f.run("resume", first, "--headless", "--go", "--json")
        f.wait_providers(previous + 1)
        reused = next(row for row in f.rows("--all") if row["name"] == first)
        original_bits = next(a["participant"]["incarnation_bits"] for a in assignments if a["participant"]["canonical_name"] == first)
        import struct
        resumed_bits = struct.unpack("Q", struct.pack("d", reused["created_at"]))[0]
        assert resumed_bits != original_bits, (reused, original_bits)
        assert bindings("--refresh", "--agent", first)["files"] == []
        assert observed(task, bindings())["document"]["assignments"] == assignments
        passed("B11 same-name new native incarnation does not inherit prior file assignments")
        result["status"] = "passed"
    except BaseException:
        result["status"] = "failed"
        result["traceback"] = traceback.format_exc()
        raise
    finally:
        f.close()
        result["cleanup"] = "Only exact disposable-scope processes signaled; fixture removed. No live service restarted."
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
