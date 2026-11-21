#!/usr/bin/env python3
"""Execute the independent environment scenarios against one fixed artifact.

Execution authority belongs to the original acceptance Task and requires a
coherent native release. Every product command uses an explicit disposable
environment; only recorded fixture process identities may be cleaned up.
"""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import http.client
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
from threading import RLock
import time
from urllib.parse import urlsplit

from prepare import write

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from startup_usability.fixture import Fixture, process_record


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


class Outcome:
    def __init__(self, artifact: Path, evidence: Path) -> None:
        self.artifact = artifact.resolve(strict=True)
        self.artifact_sha256 = digest(self.artifact)
        self.evidence = evidence.resolve()
        self.evidence.mkdir(parents=True, exist_ok=False)
        self.root = Path(tempfile.mkdtemp(prefix="agentx-environment-fixture-"))
        generated = subprocess.run(
            [sys.executable, str(Path(__file__).with_name("prepare.py")), "--root", str(self.root)],
            text=True, capture_output=True, check=True,
        )
        self.manifest = json.loads((self.root / "manifest.json").read_text())
        (self.evidence / "fixture-preparation.json").write_text(generated.stdout)
        (self.evidence / "fixture-manifest.json").write_text(json.dumps(self.manifest, indent=2) + "\n")
        for name in ["home", "tmp", "captures"]:
            (self.root / name).mkdir()
        self.shell_marker = self.root / "login-shell-called"
        self.shell = self.root / "bin/fixture-login-shell"
        write(self.shell, f"#!/bin/sh\nprintf 'called\\n' >> '{self.shell_marker}'\nexec /bin/bash --noprofile --norc \"$@\"\n", 0o755)
        self.base = {
            "PATH": f"{self.root / 'bin'}:{self.artifact.parent}:/usr/bin:/bin",
            "HOME": str(self.root / "home"),
            "TMPDIR": str(self.root / "tmp"),
            "XDG_CONFIG_HOME": str(self.root / "home/config"),
            "XDG_CACHE_HOME": str(self.root / "home/cache"),
            "XDG_DATA_HOME": str(self.root / "home/share"),
            "XDG_STATE_HOME": str(self.root / "home/state"),
            "LANG": "C.UTF-8", "NO_COLOR": "1", "SHELL": str(self.shell),
            "CODEX_HOME": str(self.root / "home/codex"),
        }
        self.lock = RLock()
        self.peers: dict[str, dict] = {}
        self.hosts: list[tuple[subprocess.Popen, dict]] = []
        self.ports: list[int] = []
        self.commands: list[dict] = []
        self.results: list[dict] = []
        self.skip_stdio_bytes = False
        self.launch_from = "bytes"
        self.scopes = {name: Path(path) for name, path in self.manifest["scopes"].items()}
        self.projects = {name: Path(item["project"]) for name, item in self.manifest["projects"].items()}
        self.save()

    def save(self) -> None:
        with self.lock:
            record = {
                "artifact": str(self.artifact), "sha256": self.artifact_sha256,
                "root": str(self.root), "commands": self.commands,
                "daemon_peers": dict(self.peers), "results": self.results,
                "hosts": [record for _process, record in self.hosts], "ports": self.ports,
            }
            (self.evidence / "resources.json").write_text(json.dumps(record, indent=2) + "\n")

    def run(self, label: str, arguments: list[str], *, scope: Path | None = None,
            cwd: Path | None = None, env: dict[str, str | None] | None = None,
            expected: int | None = 0, stdin: bytes = b"", timeout: int = 45) -> subprocess.CompletedProcess:
        receipt = self.evidence / f"{label}.json"
        if receipt.exists():
            raise FileExistsError(receipt)
        environment = dict(self.base)
        if scope is not None:
            if not scope.is_relative_to(self.root):
                raise ValueError("scope outside owned fixture")
            environment["AGENTX_DIR"] = str(scope)
        for key, value in (env or {}).items():
            if value is None:
                environment.pop(key, None)
            else:
                environment[key] = value
        command = [str(self.artifact), *arguments]
        started = time.monotonic()
        child = subprocess.Popen(command, cwd=cwd or self.projects["a"], env=environment,
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, start_new_session=True)
        try:
            identity = process_record(child.pid)
        except FileNotFoundError:
            identity = {"pid": child.pid, "exited_before_identity": True}
        record = {"label": label, "command": command, "cwd": str(cwd or self.projects["a"]),
                  "scope_env": environment.get("AGENTX_DIR"), "process": identity}
        with self.lock:
            self.commands.append(record)
        try:
            stdout, stderr = child.communicate(stdin, timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGTERM)
            stdout, stderr = child.communicate(timeout=10)
            record.update(timeout=True, exit_code=child.returncode,
                          stdout=stdout.decode(errors="replace"), stderr=stderr.decode(errors="replace"),
                          elapsed_seconds=time.monotonic() - started)
            receipt.write_text(json.dumps(record, indent=2) + "\n")
            self.save()
            raise
        record.update(exit_code=child.returncode, stdout=stdout.decode(errors="replace"),
                      stderr=stderr.decode(errors="replace"), elapsed_seconds=time.monotonic() - started,
                      stdin_bytes=len(stdin), stdin_sha256=hashlib.sha256(stdin).hexdigest(),
                      stdout_bytes=len(stdout), stdout_sha256=hashlib.sha256(stdout).hexdigest())
        receipt.write_text(json.dumps(record, indent=2) + "\n")
        self.save()
        if expected is not None and child.returncode != expected:
            raise AssertionError(f"{label}: expected exit {expected}, got {child.returncode}; see {receipt}")
        return subprocess.CompletedProcess(command, child.returncode, stdout, stderr)

    def scope_config(self, scope: Path, files: list[str] | None = None, override: bool = False) -> None:
        body = "[environment]\n" + (f"files = {json.dumps(files)}\n" if files is not None else "")
        write(scope / "config.toml", body + f"override = {str(override).lower()}\n")

    def inspect(self, label: str, *, scope: Path, args: list[str] | None = None,
                env: dict[str, str | None] | None = None, cwd: Path | None = None) -> dict:
        result = self.run(label, [*(args or []), "config", "environment", "--json"], scope=scope, env=env, cwd=cwd)
        decoded = result.stdout.decode()
        for forbidden in ["fixture-value", "should-not-leak", "must-not-be-read"]:
            assert forbidden not in decoded, f"{label}: diagnostic leaked a value"
        return json.loads(decoded)

    def probe(self, label: str, *, scope: Path, expected_case: str | None,
              args: list[str] | None = None, env: dict[str, str | None] | None = None,
              cwd: Path | None = None, expected_empty: str | None = None) -> dict:
        capture = self.root / "captures" / label
        capture.mkdir()
        self.run(label, [*(args or []), "run", "envprobe"], scope=scope, cwd=cwd,
                 env={**(env or {}), "AGENTX_ENV_CAPTURE_DIR": str(capture)})
        files = list(capture.glob("child-*.json"))
        assert len(files) == 1, f"{label}: expected one finite workflow child, got {files}"
        actual = json.loads(files[0].read_text())
        shutil.copyfile(files[0], self.evidence / f"{label}-child.json")
        assert actual["environment"]["AGENTX_ENV_CASE"] == expected_case, label
        if expected_empty is not None:
            assert actual["environment"]["AGENTX_ENV_EMPTY"] == expected_empty, label
        with self.lock:
            self.results.append({"label": label, "passed": True, "child": actual["pid"]})
        self.save()
        return actual

    def peer(self, scope: Path) -> dict:
        scope = scope.resolve(strict=True)
        endpoint = f"/tmp/agentx-{os.geteuid()}/{hashlib.sha256(os.fsencode(scope)).hexdigest()}.sock"
        with socket.socket(socket.AF_UNIX) as client:
            client.settimeout(2)
            client.connect(endpoint)
            pid, uid, _gid = struct.unpack("3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        record = process_record(pid)
        assert scope.is_relative_to(self.root) and uid == os.geteuid()
        assert record["executable"] == str(self.artifact), record
        assert record["argv"][1:5] == ["daemon", "serve", "--scope", str(scope)], record
        record.update(scope=str(scope), endpoint=endpoint)
        with self.lock:
            self.peers[str(scope)] = record
        self.save()
        return record

    def launcher(self, label: str, *, scope: Path, project: Path, files: list[Path],
                 expected_case: str, mode: str = "stdio", env: dict[str, str | None] | None = None,
                 extra: list[str] | None = None, status: int = 0, stdin: bytes = b"") -> dict:
        capture = self.root / "captures" / label
        capture.mkdir()
        launch_file = self.root / "outside" / f"{label}-launch.env"
        write(launch_file, f"PATH={self.base['PATH']}\nAGENTX_ENV_CAPTURE_DIR={capture}\nCODEX_HOME={self.base['CODEX_HOME']}\n")
        flags = [item for file in [launch_file, *files] for item in ["--env-file", str(file)]]
        launch_flags = ["--stdio"] if mode == "stdio" else (["--headless", "--json"] if mode == "background" else ["--run-here"])
        arguments = [*flags, *(extra or []), "codex", *launch_flags, "--go", "--dir", str(project), "--"]
        if mode == "stdio":
            arguments.append("exec")
        result = self.run(label, arguments, scope=scope, cwd=self.root / "outside", env=env,
                          expected=status, stdin=stdin)
        if stdin:
            assert result.stdout == stdin, f"{label}: stdio bytes changed"
        self.peer(scope)
        deadline = time.monotonic() + 10
        while not list(capture.glob("child-*.json")) and time.monotonic() < deadline:
            time.sleep(0.025)
        children = list(capture.glob("child-*.json"))
        assert len(children) == 1, f"{label}: missing finite launched child"
        actual = json.loads(children[0].read_text())
        shutil.copyfile(children[0], self.evidence / f"{label}-child.json")
        for preflight in capture.glob("preflight-*.json"):
            shutil.copyfile(preflight, self.evidence / f"{label}-{preflight.name}")
        assert actual["environment"]["AGENTX_ENV_CASE"] == expected_case, label
        assert actual["cwd"] == str(project), label
        with self.lock:
            self.results.append({"label": label, "passed": True, "mode": mode, "child": actual["pid"]})
        self.save()
        return actual

    def selection_cases(self) -> None:
        a = self.scopes["a"]
        project = self.projects["a"]
        files = self.manifest["projects"]["a"]
        selected = ",".join(files[name] for name in ["base", "project_file", "local"])
        self.scope_config(a, ["not-selected.env"])
        report = self.inspect("ENV01-ordered-sources", scope=a, env={"AGENTX_ENV_FILES": selected})
        assert [source["path"] for source in report["sources"]] == selected.split(",")
        assert report["variables"]["AGENTX_ENV_CASE"] == {"source": "file", "index": 3}
        report = self.inspect("INS01-inherited-provenance", scope=a,
                              env={"AGENTX_ENV_FILES": selected, "AGENTX_ENV_CASE": "caller"})
        assert report["variables"]["AGENTX_ENV_CASE"] == {"source": "inherited"}
        self.probe("ENV01-ordered-values", scope=a, expected_case="project-a-local", env={"AGENTX_ENV_FILES": selected})
        self.probe("ENV02-cli-replaces", scope=a, expected_case="project-a-project",
                   args=["--env-file", files["base"], "--env-file=" + files["project_file"]],
                   env={"AGENTX_ENV_FILES": self.manifest["missing"]}, expected_empty="")
        for name in ["a", "b"]:
            self.scope_config(self.scopes[name], ["env/base.env", "env/project.env"])
            self.probe(f"ENV03-toml-project-{name}", scope=self.scopes[name], cwd=self.projects[name],
                       expected_case=f"project-{name}-project", expected_empty="")
        self.scope_config(a, ["not-selected.env"])
        for label, args, env in [
            ("ENV02-empty-env", [], {"AGENTX_ENV_FILES": ""}),
            ("ENV02-empty-cli", ["--env-file="], {"AGENTX_ENV_FILES": self.manifest["missing"]}),
        ]:
            report = self.inspect(label, scope=a, args=args, env=env)
            assert report["sources"] == [], label
        self.scope_config(a, ["env/project.env"])
        caller = {"AGENTX_ENV_CASE": "caller", "AGENTX_ENV_EMPTY": "caller-empty"}
        self.probe("ENV04-caller-default", scope=a, expected_case="caller", env=caller, expected_empty="caller-empty")
        self.probe("ENV04-cli-override", scope=a, expected_case="project-a-project", args=["--env-override"], env=caller, expected_empty="")
        self.probe("ENV04-env-override-reversed", scope=a, expected_case="caller", args=["--no-env-override"], env={**caller, "AGENTX_ENV_OVERRIDE": "true"})
        self.probe("ENV04-last-cli-control", scope=a, expected_case="caller", args=["--env-override", "--no-env-override"], env=caller)
        self.scope_config(a, override=True)
        self.probe("ENV04-control-toml-external", scope=a, expected_case="project-a-project", args=["--env-file", files["project_file"]], env=caller, expected_empty="")
        self.scope_config(a)
        selector = self.manifest["nested_selector"]
        actual = self.probe("ENV07-selector-frozen", scope=a, expected_case="nested-selector", args=["--env-file", selector])
        assert actual["environment"]["AGENTX_ENV_NESTED_MARKER"] is None
        self.probe("ENV07-no-shell", scope=a, expected_case=f"$(touch {self.manifest['shell_marker']})", args=["--env-file", self.manifest["shell_data"]], expected_empty="")
        assert not Path(self.manifest["shell_marker"]).exists()
        assert not (a / "agentx.db").exists(), "local workflow/inspection unexpectedly acquired a daemon"

    def bootstrap_and_errors(self) -> None:
        a, b = self.scopes["a"], self.scopes["b"]
        self.scope_config(a)
        self.scope_config(b, [self.manifest["missing"]])
        report = self.inspect("ENV05-external-bootstrap", scope=a,
                              args=["--env-file", self.manifest["bootstrap"]], env={"AGENTX_DIR": None})
        assert report["scope"] == str(a)
        report = self.inspect("ENV05-inherited-scope-wins", scope=b,
                              args=["--env-file", self.manifest["bootstrap"]])
        assert report["scope"] == str(b)
        report = self.inspect("ENV05-override-scope", scope=b,
                              args=["--env-file", self.manifest["bootstrap"], "--env-override"])
        assert report["scope"] == str(a)
        assert report["selector_scope"] == str(b)
        self.scope_config(a, [self.manifest["redirect"]])
        result = self.run("ENV06-scoped-redirect", ["config", "environment", "--json"], scope=a, expected=None)
        assert result.returncode != 0 and b"AGENTX_DIR" in result.stderr
        assert not (a / "agentx.db").exists() and not (b / "agentx.db").exists()
        self.scope_config(a)
        for kind in ["missing", "unreadable", "invalid", "envrc"]:
            absent_scope = self.root / f"must-not-create-{kind}"
            result = self.run(f"ENV08-{kind}", ["--env-file", self.manifest[kind], "list"], scope=absent_scope, expected=None)
            assert result.returncode != 0, kind
            assert not absent_scope.exists(), kind
            assert b"should-not-leak" not in result.stdout + result.stderr, kind
            assert not Path(self.manifest["envrc_marker"]).exists(), kind
    def metadata_cases(self) -> None:
        poison = {"AGENTX_ENV_FILES": self.manifest["missing"], "AGENTX_ENV_OVERRIDE": "poison"}
        for label, args in [
            ("root-help", ["--help"]), ("version", ["--version"]),
            ("topic-tool-help", ["help", "codex"]), ("topic-config-help", ["help", "config"]),
            ("topic-daemon-help", ["help", "daemon"]), ("config-help", ["config", "--help"]),
            ("tool-help", ["launch", "--tool", "codex", "--help"]), ("daemon-help", ["daemon", "--help"]),
            ("observe-help", ["observe", "--help"]), ("workbench-help", ["workbench", "--help"]),
            ("explicit-missing-file-help", ["--env-file", self.manifest["missing"], "--help"]),
        ]:
            absent_scope = self.root / f"must-not-create-{label}"
            self.run(f"ENV09-{label}", args, scope=absent_scope, env=poison)
            assert not absent_scope.exists(), label
        absent_scope = self.root / "must-not-create-config-environment"
        report = self.inspect("ENV09-config-environment", scope=absent_scope,
                              env={"AGENTX_ENV_FILES": "", "AGENTX_ENV_OVERRIDE": "false"})
        assert report["sources"] == [] and report["scope"] == str(absent_scope)
        assert not absent_scope.exists()
        capture = self.root / "captures/ENV09-provider-version"
        capture.mkdir()
        absent_scope = self.root / "must-not-create-provider-version"
        result = self.run("ENV09-provider-version", ["launch", "--tool", "codex", "--stdio", "--go", "--", "--version"],
                          scope=absent_scope, env={**poison, "AGENTX_ENV_CAPTURE_DIR": str(capture)})
        assert result.stdout == b"codex-cli 0.155.0\n" and not absent_scope.exists()
        self.results.append({"label": "ENV09-passive-metadata", "passed": True, "metadata_shapes": 13})

    def launch_cases(self) -> None:
        a, b = self.scopes["a"], self.scopes["b"]
        self.scope_config(a)
        self.scope_config(b)
        source = Path(self.manifest["projects"]["a"]["project_file"])
        stages = ["bytes", "nested", "stdio", "foreground", "background", "contaminated", "isolation"]
        needed = lambda stage: stages.index(stage) >= stages.index(self.launch_from)
        if needed("bytes") and not self.skip_stdio_bytes:
            self.launcher("ENV10-stdio-bytes-exit", scope=a, project=self.projects["a"], files=[source],
                          expected_case="project-a-project", env={"AGENTX_ENV_ECHO_STDIN": "1", "AGENTX_ENV_CHILD_STATUS": "37"},
                          status=37, stdin=bytes(range(256)) * 64)
        if needed("nested"):
            nested_capture = self.root / "captures/ENV10-nested-stdio-inner"
            nested_capture.mkdir()
            nested = [str(self.artifact), "--env-file=", "codex", "--stdio", "--go", "--dir", str(self.projects["a"]), "--", "exec"]
            self.launcher("ENV10-nested-stdio-outer", scope=a, project=self.projects["a"], files=[source],
                          expected_case="project-a-project", env={"AGENTX_ENV_NEST_COMMAND": json.dumps(nested),
                                                                  "AGENTX_ENV_NEST_CAPTURE": str(nested_capture)})
            inner_files = list(nested_capture.glob("child-*.json"))
            assert len(inner_files) == 1
            inner = json.loads(inner_files[0].read_text())
            assert inner["environment"]["AGENTX_ENV_CASE"] == "project-a-project"
            assert inner["environment"]["AGENTX_ENV_EMPTY"] == ""
            shutil.copyfile(inner_files[0], self.evidence / "ENV10-nested-stdio-inner-child.json")
            self.results.append({"label": "ENV10-nested-stdio-inner", "passed": True, "child": inner["pid"]})
        for mode in ["stdio", "foreground", "background"]:
            if needed(mode):
                self.launcher(f"ENV10-{mode}", scope=a, project=self.projects["a"], files=[source], expected_case="project-a-project", mode=mode)
        if self.shell_marker.exists():
            self.shell_marker.unlink()
        self.launcher("ENV10-contaminated-shell", scope=a, project=self.projects["a"], files=[source], expected_case="project-a-project", mode="background", env={"CI": "1"})
        assert self.shell_marker.exists(), "clean login-shell reconstruction was not exercised"
        before = self.peer(a)
        before_environment = self.daemon_fixture_environment(before)
        first = Path(self.manifest["projects"]["a"]["project_file"])
        second = Path(self.manifest["projects"]["b"]["project_file"])
        with ThreadPoolExecutor(max_workers=2) as workers:
            futures = [workers.submit(self.launcher, f"ENV12-shared-context-{name}", scope=a, project=self.projects[name], files=[file], expected_case=f"project-{name}-project") for name, file in [("a", first), ("b", second)]]
            for future in futures:
                future.result()
        after = self.peer(a)
        assert before["pid"] == after["pid"] and before["start_ticks"] == after["start_ticks"]
        assert self.daemon_fixture_environment(after) == before_environment
        (self.evidence / "ENV12-daemon-environment.json").write_text(json.dumps({
            "pid": after["pid"], "start_ticks": after["start_ticks"],
            "before": before_environment, "after": self.daemon_fixture_environment(after),
            "limits": "Only fixture AGENTX_ENV_CASE/AGENTX_ENV_OVERLAY keys read from the exact owned daemon.",
        }, indent=2) + "\n")
        self.launcher("ENV12-second-scope", scope=b, project=self.projects["b"], files=[second], expected_case="project-b-project")

    @staticmethod
    def daemon_fixture_environment(peer: dict) -> dict[str, str]:
        record = process_record(peer["pid"])
        assert record["start_ticks"] == peer["start_ticks"] and record["executable"] == peer["executable"]
        allowed = {b"AGENTX_ENV_CASE", b"AGENTX_ENV_OVERLAY"}
        result = {}
        for entry in Path(f"/proc/{peer['pid']}/environ").read_bytes().split(b"\0"):
            key, separator, value = entry.partition(b"=")
            if separator and key in allowed:
                result[key.decode()] = value.decode()
        return result

    def existing_source_cases(self) -> None:
        source = Path(self.manifest["projects"]["a"]["project_file"])
        legacy = Path(self.manifest["legacy_scope"])
        self.scope_config(legacy)
        child = self.launcher("ENV11-legacy-child-last", scope=legacy, project=self.projects["a"], files=[source], expected_case="project-a-project")
        assert child["environment"]["ENV_FIXTURE_LEGACY"] == "legacy"
        assert child["environment"]["ENV_FIXTURE_EMPTY"] == "dotenv-nonempty"
        self.launcher("ENV11-no-new-setting", scope=self.scopes["a"], project=self.projects["a"], files=[],
                      expected_case="caller", env={"AGENTX_ENV_CASE": "caller"})
        native_direnv = Path("/home/long/.local/bin/direnv")
        if not native_direnv.is_file():
            raise FileNotFoundError("existing authorized real direnv fixture prerequisite missing")
        fixture_direnv = self.root / "bin/direnv"
        shutil.copyfile(native_direnv, fixture_direnv)
        fixture_direnv.chmod(0o755)
        project = Path(self.manifest["envrc_project"])
        setup = subprocess.run([str(fixture_direnv), "allow", str(project)], cwd=project,
                               env=self.base, text=True, capture_output=True, timeout=10)
        (self.evidence / "ENV11-direnv-allow.json").write_text(json.dumps({
            "command": [str(fixture_direnv), "allow", str(project)], "exit_code": setup.returncode,
            "stdout": setup.stdout, "stderr": setup.stderr, "scope": "disposable fixture only",
        }, indent=2) + "\n")
        assert setup.returncode == 0
        marker = Path(self.manifest["envrc_marker"])
        for label, extra, expected in [
            ("ENV11-direnv-caller-default", [], "envrc"),
            ("ENV11-direnv-file-override", ["--env-override"], "project-a-project"),
        ]:
            before = marker.read_text().splitlines() if marker.exists() else []
            self.launcher(label, scope=self.scopes["a"], project=project, files=[source], expected_case=expected, extra=extra)
            after = marker.read_text().splitlines()
            assert len(after) == len(before) + 1, f"{label}: envrc imported more than once"

    def baseline_case(self) -> None:
        """No CLI/ENV/TOML selector or override declaration is present."""
        scope = self.scopes["a"]
        write(scope / "config.toml", "")
        capture = self.root / "captures/ENV11-no-selector-baseline"
        capture.mkdir()
        self.run("ENV11-no-selector-baseline", ["launch", "--tool", "codex", "--stdio", "--go", "--dir", str(self.projects["a"]), "--", "exec"],
                 scope=scope, env={"AGENTX_ENV_CAPTURE_DIR": str(capture),
                                   "AGENTX_ENV_CASE": "caller", "AGENTX_ENV_EMPTY": ""})
        self.peer(scope)
        children = list(capture.glob("child-*.json"))
        assert len(children) == 1
        actual = json.loads(children[0].read_text())
        assert actual["environment"]["AGENTX_ENV_CASE"] == "caller"
        assert actual["environment"]["AGENTX_ENV_EMPTY"] == ""
        assert actual["cwd"] == str(self.projects["a"])
        shutil.copyfile(children[0], self.evidence / "ENV11-no-selector-baseline-child.json")
        self.results.append({"label": "ENV11-no-selector-baseline", "passed": True, "child": actual["pid"]})
        self.save()

    def workbench_case(self) -> None:
        """A selected project's relative file bootstraps the fixed host scope."""
        scope = self.root / "workbench-scope"
        self.scopes["workbench"] = scope
        project = self.projects["b"]
        source = project / "env/workbench.env"
        write(source, f"AGENTX_DIR={scope}\nAGENTX_ENV_CASE=workbench-selected\n")
        command = [str(self.artifact), "--env-file", "env/workbench.env", "workbench",
                   "--project", str(project), "--port", "0"]
        output, error = self.evidence / "ENV10-workbench.stdout", self.evidence / "ENV10-workbench.stderr"
        with output.open("x") as out, error.open("x") as err:
            process = subprocess.Popen(command, cwd=self.root / "outside", env=self.base,
                                       stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
        record = {**process_record(process.pid), "command": command, "cwd": str(self.root / "outside")}
        self.hosts.append((process, record))
        self.save()
        deadline = time.monotonic() + 30
        while True:
            text = output.read_text().strip()
            if text.startswith("http://"):
                origin = text.splitlines()[0].rstrip("/")
                break
            assert process.poll() is None, (process.returncode, error.read_text())
            if time.monotonic() > deadline:
                raise TimeoutError(error.read_text())
            time.sleep(0.05)
        endpoint = urlsplit(origin)
        self.ports.append(endpoint.port)
        self.save()
        self.peer(scope)
        source.unlink()
        connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=10)
        try:
            connection.request("POST", "/api/v1/connection", body="{}",
                               headers={"Origin": origin, "Content-Type": "application/json"})
            response = connection.getresponse()
            descriptor = json.loads(response.read())
            (self.evidence / "ENV10-workbench-connection.json").write_text(json.dumps({
                "origin": origin, "status": response.status, "descriptor": descriptor,
                "source_removed_after_startup": not source.exists(),
            }, indent=2) + "\n")
            assert response.status == 200 and descriptor["scope"] == str(scope)
            assert descriptor["project"] == str(project) and descriptor["available"]
            assert descriptor["collaboration"] is False
        finally:
            connection.close()
        self.results.append({"label": "ENV10-workbench-selected-scope-project", "passed": True})
        self.save()

    def cleanup(self) -> None:
        stopped = []
        for process, record in reversed(self.hosts):
            if process.poll() is None:
                stopped.append(Fixture.terminate(record))
            process.wait(timeout=10)
        for port in self.ports:
            with socket.socket() as probe:
                assert probe.connect_ex(("127.0.0.1", port)) != 0, f"owned port still open: {port}"
        for scope in [*self.scopes.values(), Path(self.manifest["legacy_scope"])]:
            if str(scope) in self.peers or not scope.exists():
                continue
            try:
                self.peer(scope)
            except (FileNotFoundError, ConnectionRefusedError):
                pass
        for peer in self.peers.values():
            stopped.append(Fixture.terminate(peer))
        retained_captures = []
        for capture in sorted((self.root / "captures").glob("*/*.json")):
            actual = json.loads(capture.read_text())
            retained_captures.append(actual)
            target = self.evidence / "retained-captures" / capture.parent.name / capture.name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(capture, target)
        observed_children = []
        for capture in retained_captures:
            record = capture.get("process")
            if record is None:
                continue
            try:
                current = process_record(record["pid"])
                still_running = current["start_ticks"] == record["start_ticks"]
            except FileNotFoundError:
                still_running = False
            observed_children.append({"process": record, "same_process_still_executing": still_running})
            if still_running:
                raise RuntimeError(f"finite fixture child still executing: {record}")
        (self.evidence / "child-process-audit.json").write_text(json.dumps(observed_children, indent=2) + "\n")
        self.save()
        report = {"stopped_peers": stopped, "root": str(self.root), "root_removed": False,
                  "limits": "Only recorded, kernel-authenticated disposable daemon identities were signalled."}
        (self.evidence / "cleanup.json").write_text(json.dumps(report, indent=2) + "\n")
        shutil.rmtree(self.root)
        report["root_removed"] = not self.root.exists()
        (self.evidence / "cleanup.json").write_text(json.dumps(report, indent=2) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--phase", action="append", choices=["selection", "bootstrap", "metadata", "launch", "existing", "baseline", "workbench"])
    parser.add_argument("--skip-stdio-bytes", action="store_true", help="Continue after a retained passing stdio byte/status receipt")
    parser.add_argument("--launch-from", choices=["bytes", "nested", "stdio", "foreground", "background", "contaminated"], default="bytes")
    args = parser.parse_args()
    fixture = Outcome(args.artifact, args.evidence)
    fixture.skip_stdio_bytes = args.skip_stdio_bytes
    fixture.launch_from = args.launch_from
    try:
        for phase in args.phase or ["selection", "bootstrap", "metadata", "launch", "existing"]:
            {"selection": fixture.selection_cases, "bootstrap": fixture.bootstrap_and_errors,
             "metadata": fixture.metadata_cases, "launch": fixture.launch_cases,
             "existing": fixture.existing_source_cases, "baseline": fixture.baseline_case,
             "workbench": fixture.workbench_case}[phase]()
        (fixture.evidence / "result.json").write_text(json.dumps({"passed": True, "cases": fixture.results}, indent=2) + "\n")
    finally:
        fixture.cleanup()


if __name__ == "__main__":
    main()
