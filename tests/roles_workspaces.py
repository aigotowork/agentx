#!/usr/bin/env python3
"""Independent local roles/workspaces acceptance against an explicitly supplied binary.

Run only after the controller releases the artifact. All writable state and fake
Provider children live below disposable roots. No Cargo or production DB writes.
Canonical cases: .controller/roles-workspaces/tasks/acceptance.md.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import tempfile
import threading
import time
import tomllib
import traceback


FAKE_CODEX = r'''#!/usr/bin/env python3
import json, os, pathlib, signal, subprocess, sys, time, tomllib, uuid
if any(arg in ("--version", "-V") for arg in sys.argv[1:]):
    print("codex-cli 0.114.0")
    sys.exit(0)
if "app-server" in sys.argv:
    # The real launcher inspects its per-run hooks before starting a Provider.
    # Echo the declared hook metadata through that native JSON-RPC preflight.
    hooks = []
    for arg in sys.argv[1:]:
        if arg.startswith("hooks."):
            document = tomllib.loads(arg)
            for event, groups in document["hooks"].items():
                for group in groups:
                    for hook in group.get("hooks", []):
                        hooks.append({"key": "fixture-" + event, "command": hook["command"], "source": "sessionFlags", "currentHash": "fixture-hash"})
    for line in sys.stdin:
        request = json.loads(line)
        if "id" in request:
            result = {"data": [{"hooks": hooks}]} if request["method"] == "hooks/list" else {}
            print(json.dumps({"id": request["id"], "result": result}), flush=True)
    sys.exit(0)
root = pathlib.Path(os.environ["RW_FIXTURE"])
record = {"pid": os.getpid(), "argv": sys.argv[1:], "cwd": os.getcwd(),
          "workspace": os.environ.get("AGENTX_WORKSPACE"),
          "name": os.environ.get("AGENTX_INSTANCE_NAME"),
          "process": os.environ.get("AGENTX_PROCESS_ID")}
path = root / "providers" / (str(os.getpid()) + ".json")
gate = os.environ.get("RW_STARTUP_GATE")
record["startup_gate"] = gate
def persist():
    temporary = path.with_suffix(".pending")
    temporary.write_text(json.dumps(record))
    temporary.replace(path)
persist()
if gate:
    deadline = time.monotonic() + 15
    while not pathlib.Path(gate).exists() and time.monotonic() < deadline:
        time.sleep(0.02)
started = subprocess.run([os.environ["RW_BINARY"], "start"], capture_output=True, text=True)
record["start"] = {"code": started.returncode, "stdout": started.stdout, "stderr": started.stderr}
args = sys.argv[1:]
session = args[args.index("resume") + 1] if "resume" in args else str(uuid.uuid4())
transcript = pathlib.Path(os.environ["CODEX_HOME"]) / "sessions" / ("rollout-2026-11-15-" + session + ".jsonl")
transcript.parent.mkdir(parents=True, exist_ok=True)
transcript.write_text(json.dumps({"type": "session_meta", "payload": {"id": session, "cwd": os.getcwd()}}) + "\n")
payload = {"session_id": session, "cwd": os.getcwd(), "transcript_path": str(transcript), "source": "resume" if "resume" in args else "startup"}
hook = subprocess.run([os.environ["RW_BINARY"], "codex-sessionstart"], input=json.dumps(payload), capture_output=True, text=True)
record["hook"] = {"code": hook.returncode, "stdout": hook.stdout, "stderr": hook.stderr}
record["session"] = session
persist()
print("RW_PROVIDER_READY", flush=True)
deadline = time.monotonic() + 180
while time.monotonic() < deadline:
    signal.pause()
'''


class Fixture:
    def __init__(self, binary, result):
        self.binary, self.result = binary, result
        self.temporary = tempfile.TemporaryDirectory(prefix="agentx-roles-acceptance-")
        self.root = Path(self.temporary.name)
        self.scope = self.root / "scope"
        self.a = self.root / "project"
        self.child = self.a / "child"
        self.b = self.root / "other"
        for path in [self.scope / "roles", self.scope / ".tmp/flags", self.root / "home", self.root / "bin", self.root / "providers", self.root / "tmp", self.a / ".agentx/roles", self.child / ".agentx/roles", self.child / "src", self.b / ".agentx/roles"]:
            path.mkdir(parents=True, exist_ok=True)
        (self.scope / ".tmp/flags/update_check").touch()
        (self.root / "bin/codex").write_text(FAKE_CODEX)
        (self.root / "bin/codex").chmod(0o755)
        (self.root / "bin/agentx").symlink_to(binary)
        self.env = {
            "PATH": str(self.root / "bin") + ":/usr/local/bin:/usr/bin:/bin",
            "HOME": str(self.root / "home"), "SHELL": "/bin/sh", "TERM": "xterm-256color",
            "LANG": "C.UTF-8", "NO_COLOR": "1", "AGENTX_DIR": str(self.scope),
            "CODEX_HOME": str(self.root / "home/.codex"), "TMPDIR": str(self.root / "tmp"),
            "RW_FIXTURE": str(self.root), "RW_BINARY": str(binary),
            "AGENTX_TEST_ROOT": str(self.root), "AGENTX_TEST_DB_NOSYNC": "1",
        }
        for field in ["CONFIG", "CACHE", "DATA", "STATE", "RUNTIME"]:
            self.env["XDG_" + field + "_HOME" if field != "RUNTIME" else "XDG_RUNTIME_DIR"] = str(self.root / ("xdg-" + field.lower()))
        # The launch environment file retains the exact fake Provider and binary
        # even if native launch chooses clean-shell environment reconstruction.
        (self.scope / "env").write_text("\n".join(f"{key}={json.dumps(value)}" for key, value in self.env.items()) + "\n")
        (self.scope / "config.toml").write_text('''[launch]
default_profile = "ambient"
[launch.profiles.deep]
tool = "codex"
args = ["--model", "role-model"]
[launch.profiles.ambient]
tool = "codex"
args = ["--model", "ambient-model"]
[launch.profiles.override]
tool = "codex"
args = ["--model", "override-model"]
''')
        self.role(self.scope / "roles", "reviewer", "GLOBAL_ROLE")
        self.role(self.a / ".agentx/roles", "controller", "PARENT_ROLE", default=True)
        self.role(self.a / ".agentx/roles", "reviewer", "PARENT_REVIEWER")
        self.role(self.child / ".agentx/roles", "controller", "CHILD_SHADOW")
        self.role(self.child / ".agentx/roles", "reviewer", "CHILD_REVIEWER")
        self.role(self.b / ".agentx/roles", "controller", "OTHER_ROLE", default=True)
        self.result["fixture"] = str(self.root)
        assert self.run("config", "AGENTX_DIR").strip() == str(self.scope)

    def role(self, directory, name, prompt, default=False, profile="deep", prompt_file=None):
        definition = f"profile = {json.dumps(profile)}\ndefault = {str(default).lower()}\n"
        definition += f"prompt_file = {json.dumps(prompt_file)}\n" if prompt_file else f"prompt = {json.dumps(prompt)}\n"
        (directory / (name + ".toml")).write_text(definition)

    def run(self, *args, ok=True, extra=None, cwd=None, timeout=35, input_text=None):
        command = [str(self.binary), *map(str, args)]
        env = dict(self.env, **(extra or {}))
        completed = subprocess.run(command, cwd=cwd or self.root, env=env, text=True, capture_output=True, timeout=timeout, input=input_text)
        receipt = {"args": list(map(str, args)), "cwd": str(cwd or self.root), "code": completed.returncode, "stdout": completed.stdout, "stderr": completed.stderr}
        self.result.setdefault("commands", []).append(receipt)
        if ok is not None:
            assert (completed.returncode == 0) == ok, receipt
        return completed.stdout if ok is not False else completed.stderr + completed.stdout

    def parsed(self, *args, **kwargs):
        return json.loads(self.run(*args, **kwargs))

    def rows(self, *args, **kwargs):
        return self.parsed("list", *args, "--json", **kwargs)

    def query(self, query, parameters=()):
        # Read-only independent effect observation; never seed or mutate DB state.
        with sqlite3.connect(self.scope.joinpath("agentx.db").as_uri() + "?mode=ro", uri=True) as connection:
            connection.row_factory = sqlite3.Row
            return [dict(row) for row in connection.execute(query, parameters)]

    def providers(self):
        return [json.loads(path.read_text()) for path in (self.root / "providers").glob("*.json")]

    def wait_providers(self, count):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            values = self.providers()
            if len(values) >= count and all("hook" in value for value in values):
                assert all(value["start"]["code"] == 0 for value in values), values
                assert all(value["hook"]["code"] == 0 for value in values), values
                return values
            time.sleep(0.05)
        raise AssertionError(("fake Provider startup not observed", self.providers()))

    def launch(self, *args):
        previous = len(self.providers())
        self.run("launch", *args, "--headless", "--go", "--json")
        self.wait_providers(previous + 1)
        return self.rows("--all")

    def close(self):
        # Only processes carrying this exact disposable scope are ours. Verify
        # /proc immediately before signaling; never signal process groups.
        owned = []
        expected = ("AGENTX_DIR=" + str(self.scope)).encode()
        for proc in Path("/proc").iterdir():
            if not proc.name.isdigit() or int(proc.name) == os.getpid():
                continue
            try:
                if expected in (proc / "environ").read_bytes().split(b"\0"):
                    owned.append(int(proc.name))
            except (OSError, PermissionError):
                pass
        for pid in owned:
            try:
                os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        # Give the scoped daemon time to reap children and release its socket.
        time.sleep(0.1)
        for pid in owned:
            try:
                proc = Path("/proc") / str(pid)
                if expected in (proc / "environ").read_bytes().split(b"\0"):
                    os.kill(pid, signal.SIGKILL)
            except (OSError, ProcessLookupError):
                pass
        self.result["cleanup_pids"] = owned
        self.temporary.cleanup()


def discovery(f):
    shown = f.parsed("roles", "show", "reviewer", "--dir", f.child / "src", "--json")
    assert shown["workspace"] == str(f.child) and shown["role"]["prompt"] == "CHILD_REVIEWER", shown
    all_roles = f.parsed("roles", "--dir", f.child, "--all", "--json")["roles"]
    reviewers = [role for role in all_roles if role["name"] == "reviewer"]
    assert [role["shadowed"] for role in reviewers] == [False, True, True], reviewers
    assert f.parsed("roles", "show", "global:reviewer", "--dir", f.child, "--json")["role"]["prompt"] == "GLOBAL_ROLE"
    assert f.parsed("roles", "show", "local:reviewer", "--dir", f.child, "--json")["role"]["prompt"] == "CHILD_REVIEWER"
    (f.child / ".agentx/prompts").mkdir()
    (f.child / ".agentx/prompts/literal.md").write_text('literal $HOME `unchanged` 中文\n')
    f.role(f.child / ".agentx/roles", "literal", "", prompt_file="../prompts/literal.md")
    prompt = f.parsed("roles", "show", "literal", "--dir", f.child, "--json")["role"]["prompt"]
    source = (f.child / ".agentx/prompts/literal.md").resolve()
    assert prompt == f"[Role prompt file: {source}]\nResolve relative paths in this role prompt from this file's parent directory.\n\nliteral $HOME `unchanged` 中文\n"
    empty = f.child / "empty"
    (empty / ".agentx/roles").mkdir(parents=True)
    assert f.parsed("roles", "--dir", empty, "--json")["workspace"] == str(empty)
    assert not (f.scope / "agentx.db").exists() and not f.providers()


def invalid_definitions(f):
    path = f.child / ".agentx/roles/reviewer.toml"
    path.write_text("profile = [broken")
    assert "reviewer.toml" in f.run("roles", "show", "reviewer", "--dir", f.child, ok=False)
    f.role(path.parent, "reviewer", "missing", profile="absent")
    assert f.parsed("roles", "show", "reviewer", "--dir", f.child, "--json")["role"]["profile"] == "absent"
    assert "absent" in f.run("launch", "--role", "reviewer", "--dir", f.child, "--dry-run", ok=False)
    f.role(path.parent, "reviewer", "fixed", default=True)
    f.role(path.parent, "controller", "duplicate", default=True)
    assert "default" in f.run("roles", "--dir", f.child, ok=False).lower()
    assert not (f.scope / "agentx.db").exists() and not f.providers()


def composition(f):
    value = f.parsed("launch", "--role", "global:reviewer", "--workspace", f.a, "--dir", f.b, "--agentx-system-prompt", "EXPLICIT_ADDITION", "--dry-run", "--json", extra={"AGENTX_PROFILE": "ambient"})
    assert value["configuration"]["profile"]["name"] == "deep", value
    assert value["configuration"]["selected_by"] == "role_profile", value
    assert value["collaboration"]["workspace"] == str(f.a) and value["cwd"] == str(f.b), value
    assert value["instructions"]["role_prompt"] == "GLOBAL_ROLE" and value["instructions"]["additional_prompt"] == "EXPLICIT_ADDITION", value
    override = f.parsed("launch", "--role", "global:reviewer", "--workspace", f.a, "--profile", "override", "--dry-run", "--json")
    assert override["configuration"]["profile"]["name"] == "override" and override["collaboration"]["role"]["profile"] == "override", override
    assert "no-profile" in f.run("launch", "--role", "global:reviewer", "--no-profile", "--dry-run", ok=False)
    assert f.run("launch", "--role", "global:reviewer", "--tool", "claude", "--dry-run", ok=False)
    assert not (f.scope / "agentx.db").exists() and not f.providers()


def unsupported(f):
    for args in [("--role", "global:reviewer"), ("--workspace", str(f.a))]:
        error = f.run("launch", *args, "--device", "ZZZZ", "--dry-run", ok=False)
        assert "Remote launch does not support" in error, error
        error = f.run("launch", *args, "--tool", "codex", "--stdio", "--dry-run", "--", "app-server", ok=False)
        assert "stdio" in error and "support" in error, error
    assert not (f.scope / "agentx.db").exists() and not f.providers()


def membership(f):
    f.launch("--role", "global:reviewer", "--workspace", f.a, "--dir", f.b, "--agentx-system-prompt", "EXPLICIT_ADDITION")
    first = f.rows("--all")[0]
    assert first["workspace"] == str(f.a) and first["role"]["namespace"].startswith("global:"), first
    provider = f.providers()[0]
    assert provider["workspace"] == str(f.a) and provider["cwd"] == str(f.b), provider
    native_prompt = "\n".join(provider["argv"])
    assert "GLOBAL_ROLE" in native_prompt and "EXPLICIT_ADDITION" in native_prompt and "agentx" in native_prompt.lower(), provider
    f.launch("--tool", "codex", "--workspace", f.b)
    f.launch("--tool", "codex", "--workspace", "global")
    rows = f.rows("--all")
    assert len(rows) == 3, rows
    assert len(f.rows("--dir", f.a)) == 2 and len(f.rows("--dir", f.b)) == 2
    assert len(f.rows("--workspace", "global")) == 1
    own = f.rows(extra={"AGENTX_PROCESS_ID": provider["process"], "AGENTX_WORKSPACE": str(f.b)}, cwd=f.b)
    assert {row["name"] for row in own} == {row["name"] for row in f.rows("--dir", f.a)}, own
    f.result["provider_receipts"] = f.providers()


def delegation(f):
    preview = f.parsed("send", "--dir", f.child, "--dry-run", "--", "preview")
    assert preview["requested_workspace"] == str(f.child) and preview["resolved_workspace"] == str(f.a), preview
    assert preview["role"]["prompt"] == "PARENT_ROLE", preview
    assert not f.providers()
    receipts = [f.parsed("send", "--dir", f.child, "--intent", "inform", "--json", "--", text) for text in ["child-one", "child-two"]]
    f.wait_providers(1)
    routes = [receipt["directory_route"] for receipt in receipts]
    assert routes[0]["recipient"] == routes[1]["recipient"] and [r["created"] for r in routes] == [True, False], routes
    assert receipts[0]["event_id"] != receipts[1]["event_id"], receipts
    rows = f.rows("--all")
    assert len(rows) == 1 and rows[0]["workspace"] == str(f.a) and rows[0]["role"]["prompt"] == "PARENT_ROLE", rows
    events = f.query("SELECT instance, data FROM events WHERE type='message'")
    assert len(events) == 2 and all(json.loads(event["data"])["from"] == "bigboss" and json.loads(event["data"])["sender_kind"] == "external" for event in events), events
    assert all(json.loads(event["data"])["requested_directory"] == str(f.child) for event in events), events
    assert {json.loads(event["data"])["text"] for event in events} == {"child-one", "child-two"}, events
    provider = f.providers()[0]
    hook = f.parsed("codex-userpromptsubmit", input_text=json.dumps({"session_id": provider["session"], "prompt": "<agentx>", "cwd": str(f.a)}), extra={"AGENTX_PROCESS_ID": provider["process"], "AGENTX_INSTANCE_NAME": rows[0]["name"], "AGENTX_TOOL": "codex", "AGENTX_LAUNCHED": "1"}, cwd=f.a)
    formatted = hook["hookSpecificOutput"]["additionalContext"]
    assert "[Requested directory: " + str(f.child) + "]" in formatted and "child-two" in formatted, hook
    f.result["delivered_hook"] = hook
    other = f.parsed("send", "@" + rows[0]["name"], "--from", "bigboss", "--json", "--", "explicit-cross-workspace")
    assert other["delivered_to"] and other["directory_route"] is None, other
    error = f.run("send", "@" + rows[0]["name"], "--dir", f.b, "--", "mismatch", ok=False)
    assert "workspace" in error, error
    f.result["messages"] = events


def concurrent(f):
    gate = f.root / "release-provider-startup"
    f.env["RW_STARTUP_GATE"] = str(gate)
    with (f.scope / "env").open("a") as environment_file:
        environment_file.write("RW_STARTUP_GATE=" + json.dumps(str(gate)) + "\n")
    start = threading.Barrier(2)
    def send(index):
        start.wait(timeout=5)
        return f.parsed("send", "--dir", f.a, "--intent", "inform", "--json", "--", f"race-{index}")
    with ThreadPoolExecutor(max_workers=2) as pool:
        pending = [pool.submit(send, index) for index in range(2)]
        observed = []
        try:
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                providers = f.providers()
                if providers and providers[0].get("startup_gate") == str(gate):
                    observed = f.query("SELECT id, data FROM events WHERE type='message'")
                    if observed:
                        break
                time.sleep(0.02)
            f.result["pending_startup_messages"] = observed
            f.result["pending_startup_instances"] = f.query("SELECT name, status, status_context, session_id FROM instances")
        finally:
            gate.touch()
        receipts = [future.result() for future in pending]
    assert observed and all(row["session_id"] is None or row["session_id"] == "" for row in f.result["pending_startup_instances"]), f.result["pending_startup_instances"]
    f.wait_providers(1)
    assert len(f.providers()) == 1 and len(f.rows("--all")) == 1, f.providers()
    assert len(f.query("SELECT key FROM kv WHERE key LIKE 'workspace_endpoint:%'")) == 1
    assert len({receipt["directory_route"]["recipient"] for receipt in receipts}) == 1, receipts
    assert len({receipt["event_id"] for receipt in receipts}) == 2, receipts
    assert len(f.query("SELECT * FROM events WHERE type='message'")) == 2


def conflicts(f):
    f.launch("--role", "controller", "--workspace", f.a)
    f.launch("--role", "controller", "--workspace", f.a)
    error = f.run("send", "--dir", f.a, "--", "ambiguous", ok=False)
    assert "ambiguous" in error.lower(), error
    assert len(f.providers()) == 2 and not f.query("SELECT * FROM events WHERE type='message'")
    f.parsed("send", "--dir", f.b, "--json", "--", "bind")
    f.wait_providers(3)
    f.role(f.b / ".agentx/roles", "controller", "OLD_ROLE", default=False)
    f.role(f.b / ".agentx/roles", "replacement", "NEW_ROLE", default=True)
    error = f.run("send", "--dir", f.b, "--", "role-drift", ok=False)
    assert "role changed" in error.lower(), error
    assert len(f.providers()) == 3 and len(f.query("SELECT * FROM events WHERE type='message'")) == 1


def history(f):
    f.launch("--role", "global:reviewer", "--workspace", f.a, "--agentx-system-prompt", "ADOPTED_ADDITION")
    original = f.rows("--all")[0]
    name = original["name"]
    (f.scope / "roles/reviewer.toml").unlink()
    config = f.scope / "config.toml"
    config.write_text(config.read_text().replace("role-model", "EDITED_MODEL"))
    f.run("fork", name, "--headless", "--go", "--json", cwd=f.b)
    f.wait_providers(2)
    forked = next(row for row in f.rows("--all") if row["name"] != name)
    assert forked["workspace"] == str(f.a) and forked["role"] == original["role"], forked
    assert forked["session_id"] != original["session_id"], (original, forked)
    fork_context = json.loads(f.query("SELECT launch_context FROM instances WHERE name=?", [forked["name"]])[0]["launch_context"])
    assert fork_context["collaboration"]["endpoint"] is False, fork_context
    f.run("kill", name, "--json")
    stopped = f.query("SELECT data FROM events WHERE type='life' AND instance=? AND json_extract(data, '$.action')='stopped' ORDER BY id DESC LIMIT 1", [name])
    assert stopped, name
    saved = json.loads(json.loads(stopped[0]["data"])["snapshot"]["launch_context"])
    assert saved["collaboration"]["role"] == original["role"], saved
    f.run("resume", name, "--headless", "--go", "--json", extra={"AGENTX_WORKSPACE": str(f.b)})
    f.wait_providers(3)
    resumed = next(row for row in f.rows("--all") if row["name"] == name)
    assert resumed["workspace"] == str(f.a) and resumed["role"] == original["role"], resumed
    assert resumed["session_id"] == original["session_id"], (original, resumed)
    for provider in f.providers():
        argv = "\n".join(provider["argv"])
        assert "GLOBAL_ROLE" in argv and "ADOPTED_ADDITION" in argv and "role-model" in argv and "EDITED_MODEL" not in argv, provider
        encoded = next(value for value in provider["argv"] if value.startswith("developer_instructions="))
        instructions = tomllib.loads(encoded)["developer_instructions"]
        assert "Your name: " + provider["name"] in instructions, provider
        if provider["name"] != name:
            assert "Your name: " + name not in instructions, provider
    resumed_provider = next(provider for provider in f.providers() if "resume" in provider["argv"])
    assert "ADOPTED_ADDITION" in "\n".join(resumed_provider["argv"]), resumed_provider
    f.result["provider_receipts"] = f.providers()
    f.result["stopped_snapshot"] = json.loads(stopped[0]["data"])


CASES = {function.__name__: function for function in [discovery, invalid_definitions, composition, unsupported, membership, delegation, concurrent, conflicts, history]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--only", choices=list(CASES), action="append")
    options = parser.parse_args()
    binary = options.binary.resolve(strict=True)
    report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "scope": "disposable Linux native CLI + fake Provider; no real model/remote execution", "cases": []}
    for name in options.only or CASES:
        result = {"name": name}
        fixture = None
        try:
            fixture = Fixture(binary, result)
            CASES[name](fixture)
            result["status"] = "passed"
        except Exception:
            result["status"] = "failed"
            result["error"] = traceback.format_exc()
        finally:
            if fixture:
                result["provider_receipts"] = fixture.providers()
                if (fixture.scope / "agentx.db").exists():
                    result["durable_messages"] = fixture.query("SELECT id, instance, data FROM events WHERE type='message'")
                    result["durable_instances"] = fixture.query("SELECT name, created_at, status, session_id, launch_args, launch_profile, launch_context FROM instances")
                    result["durable_endpoints"] = fixture.query("SELECT key, value FROM kv WHERE key LIKE 'workspace_endpoint:%'")
                fixture.close()
            report["cases"].append(result)
            options.report.parent.mkdir(parents=True, exist_ok=True)
            options.report.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
        print(name + ": " + result["status"], flush=True)
        if result["status"] == "failed":
            print(result["error"], flush=True)
    raise SystemExit(any(result["status"] == "failed" for result in report["cases"]))


if __name__ == "__main__":
    main()
