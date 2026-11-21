#!/usr/bin/env python3
"""Project workspace priority through a supplied native binary and disposable fake Provider.

Run: python3 tests/workspace_discovery.py --binary /absolute/agentx --report /new/report.json
Design: .docs/proposals/workspace-discovery.md. Reuses the existing scoped launch fixture.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import traceback

from roles_workspaces import Fixture


def preview(f, directory=None, workspace=None, cwd=None, extra=None):
    args = ["launch", "--dry-run", "--json"]
    if directory is not None:
        args += ["--dir", str(directory)]
    if workspace is not None:
        args += ["--workspace", str(workspace)]
    return f.parsed(*args, cwd=cwd, extra=extra)


def project_roots(f):
    repo = f.root / "git-project"
    sub = repo / "sub"
    sub.mkdir(parents=True)
    subprocess.run(["git", "init", "--quiet", str(repo)], check=True, env=f.env)
    (sub / ".agentx").mkdir()
    value = preview(f, cwd=sub)
    assert value["collaboration"]["workspace"] == str(repo), value
    assert value["cwd"] == str(sub), value
    assert f.parsed("roles", "--dir", sub, "--json")["workspace"] == str(repo)
    override = preview(f, directory=sub, workspace=f.b, cwd=f.a)
    assert override["collaboration"]["workspace"] == str(f.b) and override["cwd"] == str(sub), override
    subprocess.run(["git", "-C", str(repo), "-c", "user.name=Fixture", "-c", "user.email=fixture@invalid", "commit", "--allow-empty", "--quiet", "-m", "fixture"], check=True, env=f.env)
    worktree = f.root / "linked-worktree"
    subprocess.run(["git", "-C", str(repo), "worktree", "add", "--quiet", "--detach", str(worktree)], check=True, env=f.env)
    linked = preview(f, directory=worktree, extra={"AGENTX_WORKSPACE": str(f.a)})
    assert (worktree / ".git").is_file()
    assert linked["collaboration"]["workspace"] == str(worktree), linked
    non_git = f.root / "project-config/sub"
    (non_git.parent / ".agentx").mkdir(parents=True)
    non_git.mkdir()
    assert preview(f, directory=non_git)["collaboration"]["workspace"] == str(non_git.parent)
    assert not (f.scope / "agentx.db").exists(), "passive project discovery started a daemon"


def defaults_and_continuation(f):
    plain = f.root / "plain"
    plain.mkdir()
    inherited = {"AGENTX_WORKSPACE": str(f.a)}
    assert preview(f, directory=plain, extra=inherited)["collaboration"]["workspace"] == str(f.a)
    assert preview(f, directory=plain, cwd=f.b)["collaboration"]["workspace"] == str(f.b)
    personal = Path(f.env["HOME"]) / ".agentx"
    fallback = preview(f, cwd=plain)
    assert fallback["collaboration"]["workspace"] == str(personal), fallback
    assert fallback["cwd"] == str(plain), fallback
    assert not personal.exists(), "passive personal workspace fallback wrote a directory"
    inherited_personal = preview(f, cwd=plain, extra={"AGENTX_WORKSPACE": str(personal)})
    assert inherited_personal["collaboration"]["workspace"] == str(personal), inherited_personal
    role_catalog = f.parsed("roles", "--json", cwd=plain)
    assert role_catalog["workspace"] == str(personal), role_catalog
    assert not personal.exists() and not (f.scope / "agentx.db").exists()
    personal.mkdir()
    home_sub = personal.parent / "unconfigured/sub"
    home_sub.mkdir(parents=True)
    assert preview(f, cwd=home_sub)["collaboration"]["workspace"] == str(personal)
    assert preview(f, cwd=f.a, extra={"AGENTX_WORKSPACE": "global"})["collaboration"]["workspace"] is None
    assert preview(f, directory=f.b, extra={"AGENTX_WORKSPACE": "global"})["collaboration"]["workspace"] == str(f.b)


def launch_inheritance(f):
    repo_a = f.root / "native-a"
    sub_a = repo_a / "src"
    sub_a.mkdir(parents=True)
    (repo_a / ".git").mkdir()
    repo_b = f.root / "native-b"
    sub_b = repo_b / "src"
    sub_b.mkdir(parents=True)
    (repo_b / ".git").mkdir()
    f.run("launch", "--headless", "--go", "--json", cwd=sub_a)
    parent = f.wait_providers(1)[0]
    assert parent["workspace"] == str(repo_a) and parent["cwd"] == str(sub_a), parent
    # A bound caller's native membership wins over edited ambient environment,
    # even when it has changed cwd into another repository.
    caller = {"AGENTX_PROCESS_ID": parent["process"], "AGENTX_WORKSPACE": str(repo_b)}
    inherited = preview(f, cwd=sub_b, extra=caller)
    assert inherited["collaboration"]["workspace"] == str(repo_a), inherited
    assert inherited["cwd"] == str(sub_b), inherited
    targeted = preview(f, directory=sub_b, cwd=sub_a, extra=caller)
    assert targeted["collaboration"]["workspace"] == str(repo_b), targeted
    f.run("launch", "--dir", sub_b, "--headless", "--go", "--json", cwd=sub_a, extra=caller)
    providers = f.wait_providers(2)
    child = next(value for value in providers if value["name"] != parent["name"])
    assert child["workspace"] == str(repo_b) and child["cwd"] == str(sub_b), child
    records = f.rows("--all")
    assert {row["name"]: row["workspace"] for row in records} == {parent["name"]: str(repo_a), child["name"]: str(repo_b)}, records
    for provider in providers:
        saved = json.loads(f.query("SELECT launch_context FROM instances WHERE name=?", [provider["name"]])[0]["launch_context"])
        assert saved["collaboration"]["workspace"] == provider["workspace"], (saved, provider)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "cases": [], "scope": "Linux local CLI/native + fake Provider; no real model or live user-service adoption"}
    for case in [project_roots, defaults_and_continuation, launch_inheritance]:
        result = {"name": case.__name__}
        fixture = None
        try:
            fixture = Fixture(binary, result)
            case(fixture)
            result["status"] = "passed"
        except Exception:
            result.update(status="failed", error=traceback.format_exc())
        finally:
            if fixture:
                result["providers"] = fixture.providers()
                fixture.close()
            report["cases"].append(result)
            args.report.parent.mkdir(parents=True, exist_ok=True)
            args.report.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
        print(f"{case.__name__}: {result['status']}", flush=True)
        if result["status"] == "failed":
            print(result["error"], flush=True)
    raise SystemExit(any(case["status"] != "passed" for case in report["cases"]))


if __name__ == "__main__":
    main()
