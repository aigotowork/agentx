#!/usr/bin/env python3
"""Exercise the public profile CLI in a disposable scope without any Provider."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--report", type=Path)
    options = parser.parse_args()
    binary = options.binary.resolve(strict=True)
    checks = []
    with tempfile.TemporaryDirectory(prefix="agentx-profile-cli-") as directory:
        root = Path(directory)
        scope = root / "state"
        scope.mkdir()
        home = root / "home"
        home.mkdir()
        tools = root / "bin"
        tools.mkdir()
        marker = root / "unexpected-provider"
        for name in ["codex", "direnv"]:
            executable = tools / name
            executable.write_text("#!/bin/sh\n: > \"$PROFILE_TEST_MARKER\"\nexit 99\n")
            executable.chmod(0o755)
        (root / ".envrc").write_text("exit 99\n")
        environment = {
            "PATH": str(tools) + os.pathsep + os.environ.get("PATH", ""),
            "HOME": str(home), "USERPROFILE": str(home),
            "AGENTX_DIR": str(scope), "PROFILE_TEST_MARKER": str(marker),
            "XDG_CONFIG_HOME": str(home / "config"),
            "XDG_DATA_HOME": str(home / "data"),
            "XDG_CACHE_HOME": str(home / "cache"),
            "XDG_STATE_HOME": str(home / "state"),
            "XDG_RUNTIME_DIR": str(home / "runtime"),
        }
        for key in ["SYSTEMROOT", "WINDIR", "TMP", "TEMP"]:
            if key in os.environ:
                environment[key] = os.environ[key]
        config = scope / "config.toml"
        config.write_text("# preserve this comment\n[launch.codex]\nargs='--model legacy'\n")

        def run(*args, success=True, extra=None):
            child_env = dict(environment)
            child_env.update(extra or {})
            result = subprocess.run([str(binary), *args], cwd=root, env=child_env, capture_output=True, text=True, timeout=15)
            assert (result.returncode == 0) == success, (args, result.returncode, result.stdout, result.stderr)
            return result.stdout

        def preview(*args, extra=None):
            return json.loads(run("launch", *args, "--dry-run", "--json", extra=extra))["configuration"]

        def passed(name):
            checks.append(name)

        run("launch", "--dry-run", success=False)
        run("profile", "create", "daily", "--tool", "codex", "--", "--model", "daily-model")
        run("profile", "create", "deep", "--tool", "codex", "--description", "复杂问题", "--", "--model", "deep-model", "--add-dir", "a path", "--future", "")
        assert json.loads(run("profile", "show", "deep", "--json"))["definition"]["args"][-1] == ""
        assert set(json.loads(run("profile", "list", "--json"))["profiles"]) == {"daily", "deep"}
        passed("CRUD preserves empty argv and paths containing spaces")
        run("profile", "default", "daily")
        run("profile", "default", "deep", "--tool", "codex")
        assert preview()["profile"]["name"] == "daily"
        assert preview("--tool", "codex")["profile"]["name"] == "deep"
        passed("scope and tool defaults are independent")
        assert preview(extra={"AGENTX_PROFILE": "deep"})["profile"]["name"] == "deep"
        assert preview("--tool", "codex", extra={"AGENTX_PROFILE": "missing"})["profile"]["name"] == "deep"
        assert preview("--profile", "daily", extra={"AGENTX_PROFILE": "missing"})["profile"]["name"] == "daily"
        passed("explicit selections override ENV; ENV overrides scope default")
        saved = config.read_bytes()
        value = json.loads(run("launch", "--profile", "deep", "--dry-run", "--json", "--", "-m", "temporary", extra={"AGENTX_CODEX_ARGS": "--model env-legacy"}))["configuration"]
        assert value["args"] == ["--add-dir", "a path", "--future", "", "-m", "temporary"]
        assert config.read_bytes() == saved
        passed("temporary singleton override excludes legacy defaults without saving")
        assert preview("--profile", "deep", "--replace-args")["args"] == []
        assert preview("--tool", "codex", "--no-profile")["args"] == []
        passed("replace clears inherited argv; no-profile bypasses all defaults")
        for args in [
            ("launch", "--profile", "deep", "--tool", "claude", "--dry-run"),
            ("launch", "--profile", "missing", "--dry-run"),
            ("profile", "create", "daily", "--tool", "codex"),
            ("profile", "default", "deep", "--tool", "claude"),
            ("profile", "delete", "deep"),
            ("profile", "update", "deep", "--", "--model", "unintended"),
        ]:
            run(*args, success=False)
            assert config.read_bytes() == saved
        passed("invalid selections, duplicate names, default deletion and ambiguous updates fail atomically")
        run("profile", "update", "deep", "--replace-args", "--", "--model", "updated")
        assert preview("--profile", "deep")["args"] == ["--model", "updated"]
        passed("explicit saved update replaces the full argv")
        value = json.loads(run("launch", "--profile", "deep", "--stdio", "--dry-run", "--json"))
        assert value["executed"] is False
        assert not marker.exists()
        assert not (scope / "agentx.db").exists()
        passed("stdio dry-run invokes neither direnv nor Provider and creates no database")
        run("profile", "create", "opaque", "--tool", "codex", "--", "--help-all")
        assert json.loads(run("profile", "show", "opaque", "--json"))["definition"]["args"] == ["--help-all"]
        passed("native help-looking argv remains opaque")
        good = config.read_bytes()
        config.write_text("malformed [")
        run("launch", "--profile", "deep", "--dry-run", success=False)
        run("profile", "default", "--unset", success=False)
        assert config.read_text() == "malformed ["
        assert "profile create" in run("profile", "--help-all")
        assert "--replace-args" in run("profile", "update", "--help")
        config.write_bytes(good)
        passed("malformed config fails without rewrite; help remains configuration-independent")
        run("profile", "default", "--unset", "--tool", "codex")
        run("profile", "delete", "deep")
        assert preview("--tool", "codex")["args"] == ["--model", "legacy"]
        run("profile", "default", "--unset")
        run("profile", "delete", "daily")
        run("profile", "delete", "opaque")
        assert json.loads(run("profile", "list", "--json"))["profiles"] == {}
        assert "# preserve this comment" in config.read_text()
        assert not marker.exists() and not (scope / "agentx.db").exists()
        passed("clearing defaults permits deletion and retains legacy config and comments")

    with binary.open("rb") as executable:
        digest = hashlib.file_digest(executable, "sha256").hexdigest()
    report = {"binary": str(binary), "sha256": digest, "passed": len(checks), "checks": checks, "scope": "disposable Linux local CLI; no real model or cross-device Relay execution"}
    if options.report:
        options.report.parent.mkdir(parents=True, exist_ok=True)
        options.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report, ensure_ascii=False))


if __name__ == "__main__":
    main()
