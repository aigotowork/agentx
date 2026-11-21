#!/usr/bin/env python3
"""Create a disposable, non-secret environment-loading acceptance fixture."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil


def write(path: Path, content: str, mode: int = 0o644) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    path.chmod(mode)


def env_file(path: Path, *, value: str, empty: str = "") -> None:
    write(
        path,
        "\n".join(
            [
                f"AGENTX_ENV_CASE={value}",
                f"AGENTX_ENV_OVERLAY={value}",
                f"AGENTX_ENV_EMPTY={empty}",
                "AGENTX_ENV_SELECTOR=from-file-selector-must-not-recurse",
                "AGENTX_ENV_TEST_VALUE=fixture-value",
                "ENV_FIXTURE_LEGACY=dotenv",
                "ENV_FIXTURE_EMPTY=dotenv-nonempty",
                "",
            ]
        ),
    )


def project(root: Path, name: str, scope: Path) -> dict[str, str]:
    directory = root / name
    files = directory / "env"
    env_file(files / "base.env", value=f"{name}-base", empty="base-empty")
    env_file(files / "project.env", value=f"{name}-project", empty="")
    env_file(files / "local.env", value=f"{name}-local", empty="local-empty")
    write(
        scope / "config.toml",
        "[environment]\n"
        'files = ["env/base.env", "env/project.env", "env/local.env"]\n'
        "override = false\n",
    )
    return {
        "project": str(directory),
        "config": str(scope / "config.toml"),
        "base": str(files / "base.env"),
        "project_file": str(files / "project.env"),
        "local": str(files / "local.env"),
        "scope": str(scope),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.absolute()
    if root.exists() and any(root.iterdir()):
        raise SystemExit(f"fixture root must be new or empty: {root}")
    root.mkdir(parents=True, exist_ok=True)
    fixture_bin = root / "bin"
    fixture_bin.mkdir()
    stub = fixture_bin / "codex"
    shutil.copyfile(Path(__file__).with_name("codex_stub.py"), stub)
    stub.chmod(0o755)
    capture_dir = root / "provider-capture"
    capture_dir.mkdir()
    scope_a = root / "scope-a"
    scope_b = root / "scope-b"
    scope_a.mkdir()
    scope_b.mkdir()
    project_a = project(root, "project-a", scope_a)
    project_b = project(root, "project-b", scope_b)
    for scope in [scope_a, scope_b]:
        scripts = scope / "scripts"
        scripts.mkdir()
        shutil.copyfile(Path(__file__).with_name("codex_stub.py"), scripts / "envprobe.py")
        write(scope / ".tmp/flags/update_check", "")
    outside = root / "outside"
    outside.mkdir()

    bootstrap = outside / "bootstrap.env"
    write(
        bootstrap,
        f"AGENTX_DIR={scope_a}\nAGENTX_ENV_CASE=bootstrap\nAGENTX_ENV_TEST_VALUE=fixture-value\n",
    )
    redirect = outside / "redirect.env"
    write(redirect, f"AGENTX_DIR={scope_b}\nAGENTX_ENV_CASE=redirect\n")
    nested_selector = outside / "nested-selector.env"
    write(nested_selector, f"AGENTX_ENV_FILES={outside / 'nested.env'}\nAGENTX_ENV_CASE=nested-selector\n")
    nested = outside / "nested.env"
    write(nested, "AGENTX_ENV_NESTED_MARKER=must-not-be-read\n")
    invalid = outside / "invalid.env"
    write(invalid, "AGENTX_ENV_SECRET_MARKER=should-not-leak\nthis is not dotenv\n")
    missing = outside / "missing.env"
    unreadable = outside / "unreadable.env"
    write(unreadable, "AGENTX_ENV_CASE=unreadable\n", 0o000)
    envrc_project = root / "direnv-project"
    envrc_marker = envrc_project / "envrc-executed"
    envrc = envrc_project / ".envrc"
    write(envrc, f"printf 'load\\n' >> {envrc_marker}\nexport AGENTX_ENV_CASE=envrc\n")
    legacy_scope = root / "legacy-scope"
    legacy = legacy_scope / "env"
    write(legacy, "ENV_FIXTURE_LEGACY=legacy\nENV_FIXTURE_EMPTY=\nAGENTX_ENV_CASE=ignored-legacy-agentx-key\n")
    write(legacy_scope / ".tmp/flags/update_check", "")
    shell_marker = root / "unexpected-shell-execution"
    shell_data = outside / "shell-data.env"
    write(shell_data, f"AGENTX_ENV_CASE='$(touch {shell_marker})'\nAGENTX_ENV_EMPTY=\n")
    child_capture = root / "child-capture"

    manifest = {
        "schema": "agentx-environment-loading-fixture-v1",
        "root": str(root),
        "projects": {"a": project_a, "b": project_b},
        "scopes": {"a": str(scope_a), "b": str(scope_b)},
        "outside": str(outside),
        "bin": str(fixture_bin),
        "codex_stub": str(stub),
        "provider_capture": str(capture_dir),
        "bootstrap": str(bootstrap),
        "redirect": str(redirect),
        "nested_selector": str(nested_selector),
        "nested": str(nested),
        "invalid": str(invalid),
        "missing": str(missing),
        "unreadable": str(unreadable),
        "envrc": str(envrc),
        "envrc_project": str(envrc_project),
        "envrc_marker": str(envrc_marker),
        "legacy": str(legacy),
        "legacy_scope": str(legacy_scope),
        "shell_data": str(shell_data),
        "shell_marker": str(shell_marker),
        "child_capture": str(child_capture),
        "caller_values": {"AGENTX_ENV_CASE": "caller", "AGENTX_ENV_EMPTY": "caller-empty"},
        "secret_marker": "should-not-leak",
        "created_by_pid": os.getpid(),
    }
    manifest_path = root / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"manifest": str(manifest_path), "root": str(root)}))


if __name__ == "__main__":
    main()
