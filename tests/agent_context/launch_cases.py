#!/usr/bin/env python3
"""Candidate-only black-box checks. A stub is not native Codex acceptance."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from startup_usability.fixture import Fixture


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", type=Path, default=Path("/tmp/agentx-context-candidate"))
    parser.add_argument("--handoff", action="store_true", required=True)
    args = parser.parse_args()
    artifact = args.artifact.resolve(strict=True)
    root = Path(tempfile.mkdtemp(prefix="agentx-startup-fixture-context-"))
    evidence = root / "evidence"
    evidence.mkdir()
    fixture = Fixture(root, evidence)
    binary = root / "bin"
    binary.mkdir()
    shutil.copy2(Path(__file__).parents[1] / "environment_loading/codex_stub.py", binary / "codex")
    (binary / "codex").chmod(0o755)
    fixture.env.update({"PATH": f"{binary}:/usr/bin:/bin", "SHELL": "/bin/sh", "AGENTX_UPDATE_CHECK": "0"})
    peers, results = [], []
    sources = lambda text: [{"id": "custom", "text": text}]

    def launch(label, selected=None, selector=None, native=False, bad=False, notes=False,
               native_config=None, native_args=None):
        scope, capture = root / label / "scope", root / label / "capture"
        scope.mkdir(parents=True)
        capture.mkdir()
        home = root / label / "codex"
        home.mkdir()
        if native_config is not None or native:
            (home / "config.toml").write_text(native_config if native_config is not None else 'developer_instructions = "NATIVE_CONFIG_SENTINEL"\n')
        body = '[launch]\nnotes = "NOTES_SENTINEL"\n' if notes else ''
        if selected is not None:
            entries = ["{ " + ", ".join(f"{k} = {json.dumps(v)}" for k, v in source.items()) + " }" for source in selected]
            body += "[context]\nsources = [" + ", ".join(entries) + "]\n"
        (scope / "config.toml").write_text(body)
        fixture.env.update({"CODEX_HOME": str(home), "AGENTX_ENV_CAPTURE_DIR": str(capture)})
        fixture.env.pop("AGENTX_CONTEXT_SOURCES", None)
        if selector is not None:
            fixture.env["AGENTX_CONTEXT_SOURCES"] = selector
        command = ["--env-file=", "launch", "--tool", "codex", "--stdio", "--go", "--", "exec"]
        if native == "argv":
            command += ["-c", 'developer_instructions="NATIVE_ARGV_SENTINEL"']
        command += native_args or []
        result = fixture.run(label, artifact, scope, command)
        try:
            peers.append(fixture.peer(scope, artifact))
        except (FileNotFoundError, ConnectionRefusedError):
            pass  # Early config failure may legitimately create no daemon.
        children = list(capture.glob("child-*.json"))
        if bad:
            assert result.returncode != 0 and not children, (label, result.returncode, result.stderr)
            assert any(term in result.stderr.lower() for term in ["context", "source", "65536"]), result.stderr
            return ""
        assert result.returncode == 0 and len(children) == 1, (label, result.stderr, children)
        child = json.loads(children[0].read_text())
        values = []
        for item in child["argv"]:
            if item.startswith("developer_instructions="):
                values.append(tomllib.loads(item)["developer_instructions"])
        assert len(values) <= 1, values
        return values[0] if values else ""

    def case(label, check):
        try:
            check()
            results.append({"case": label, "passed": True})
        except Exception as error:
            results.append({"case": label, "passed": False, "error": repr(error)})

    def equal(actual, expected):
        assert actual == expected, (actual, expected)

    def default_case():
        text = launch("default", notes=True)
        assert "agentx" in text.lower() and text.count("NOTES_SENTINEL") == 1

    def empty_override():
        text = launch("empty-env", sources("SHADOWED"), "")
        assert "agentx" in text.lower() and "SHADOWED" not in text

    def native_case():
        text = launch("native-disabled", [], native="argv")
        equal(text, "NATIVE_ARGV_SENTINEL")

    def native_config_case():
        text = launch("native-config", sources("CUSTOM_ONLY"), native="config")
        assert "CUSTOM_ONLY" in text and "NATIVE_CONFIG_SENTINEL" in text, text

    profile_config = ('developer_instructions = "HOME_SENTINEL"\n'
                      'profile = "default_native"\n'
                      '[profiles.default_native]\ndeveloper_instructions = "DEFAULT_PROFILE_SENTINEL"\n'
                      '[profiles.selected_native]\ndeveloper_instructions = "SELECTED_PROFILE_SENTINEL"\n')

    def native_precedence_case(label, native_args, expected):
        text = launch(label, sources("CUSTOM_ONLY"), native_config=profile_config, native_args=native_args)
        equal(text, "CUSTOM_ONLY\n---\n" + expected)

    try:
        case("C01-legacy-notes", default_case)
        case("C01-disabled", lambda: equal(launch("disabled", [], notes=True), ""))
        case("C02-replacement", lambda: equal(launch("replace", sources("CUSTOM_ONLY"), notes=True), "CUSTOM_ONLY"))
        case("C02-env-empty", empty_override)
        case("C02-env-json", lambda: equal(launch("env-json", sources("SHADOWED"), json.dumps(sources("ENV_ONLY"))), "ENV_ONLY"))
        case("C02-env-disable", lambda: equal(launch("env-disable", sources("SHADOWED"), "[]"), ""))
        case("C03-order", lambda: equal(launch("order", [{"id":"first","text":"ONE"},{"id":"empty","text":""},{"id":"last","text":"TWO"}]), "ONE\n\nTWO"))
        (root / "project/guide.txt").write_text("CALLER_CWD_SENTINEL")
        case("C04-relative", lambda: equal(launch("relative", [{"id":"file","file":"guide.txt"}]), "CALLER_CWD_SENTINEL"))
        case("C05-missing", lambda: launch("missing", [{"id":"missing","file":"absent"}], bad=True))
        case("C05-unknown", lambda: launch("unknown", [{"id":"unknown","builtin":"missing"}], bad=True))
        case("C05-invalid", lambda: launch("invalid", selector='[{"id":"bad"}]', bad=True))
        case("C06-overflow", lambda: launch("overflow", sources("x" * 65537), bad=True))
        case("C07-native-argv-disabled", native_case)
        case("C07-native-config-retention", native_config_case)
        native_cases = [
            ("native-default-profile", [], "DEFAULT_PROFILE_SENTINEL"),
            ("native-argv-profile", ["--profile", "selected_native"], "SELECTED_PROFILE_SENTINEL"),
            ("native-equals-profile", ["--profile=selected_native"], "SELECTED_PROFILE_SENTINEL"),
            ("native-short-profile", ["-p", "selected_native"], "SELECTED_PROFILE_SENTINEL"),
            ("native-explicit-wins", ["--profile", "selected_native", "-c", 'developer_instructions="EXPLICIT_SENTINEL"'], "EXPLICIT_SENTINEL"),
            ("native-attached-override", ['-cdeveloper_instructions="ATTACHED_SENTINEL"'], "ATTACHED_SENTINEL"),
        ]
        for label, native_args, expected in native_cases:
            case("C07-" + label, lambda label=label, native_args=native_args, expected=expected:
                 native_precedence_case(label, native_args, expected))
    finally:
        cleanup = []
        for peer in peers:
            cleanup.append(Fixture.terminate(peer))
        report = {"artifact":str(artifact), "sha256":hashlib.sha256(artifact.read_bytes()).hexdigest(),
                  "root":str(root), "results":results, "cleanup":cleanup,
                  "limits":"Finite recipient captures argv; no native model or compact evidence."}
        (evidence / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))
    return int(any(not row["passed"] for row in results))


if __name__ == "__main__":
    raise SystemExit(main())
