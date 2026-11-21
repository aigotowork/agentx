#!/usr/bin/env python3
"""Bounded auxiliary discovery checks; all runtime state uses the disposable role fixture."""
import argparse
import json
from pathlib import Path
import traceback

from roles_workspaces import Fixture


def check(f):
    local = f.child / ".agentx/roles"
    for directory in [local, f.b / ".agentx/roles", f.scope / "roles"]:
        for name in ["fronter", "fronter.e2e", "reviewer.audit"]:
            f.role(directory, name, str(directory) + ":" + name)
    public = f.parsed("roles", "--dir", f.child, "--json")
    assert all("." not in row["name"] for row in public["roles"])
    everything = f.parsed("roles", "--dir", f.child, "--all", "--json")
    assert len([row for row in everything["roles"] if row["name"] == "fronter.e2e"]) == 2
    assert f.parsed("roles", "show", "fronter.e2e", "--dir", f.child, "--json")["role"]["name"] == "fronter.e2e"
    preview = f.parsed("launch", "--role", "fronter.e2e", "--dir", f.child, "--dry-run", "--json")
    assert preview["collaboration"]["role"]["name"] == "fronter.e2e"
    assert not (f.scope / "agentx.db").exists(), "passive commands must not start a daemon"
    f.parsed("roles", "--dir", f.child, "--json", extra={"AGENTX_PROCESS_ID": "missing"})
    assert not (f.scope / "agentx.db").exists(), "implicit identity probe must not acquire a daemon"

    # Native launch creates the adopted snapshot and real process binding.
    f.launch("--role", "fronter", "--dir", f.child)
    provider = f.providers()[0]
    identity = {"AGENTX_PROCESS_ID": provider["process"], "AGENTX_WORKSPACE": str(f.b)}
    adopted = f.parsed("roles", "--json", cwd=f.b, extra=identity)
    assert adopted["workspace"] == str(f.child), adopted
    helpers = [row for row in adopted["roles"] if "." in row["name"]]
    assert [row["name"] for row in helpers] == ["fronter.e2e"], helpers
    assert Path(helpers[0]["path"]).parent == local
    explicit = f.parsed("roles", "--iam", provider["name"], "--json", cwd=f.b)
    assert explicit == adopted
    elsewhere = f.parsed("roles", "--dir", f.b, "--json", extra=identity)
    assert all("." not in row["name"] for row in elsewhere["roles"])
    assert "[helper of fronter]" in f.run("roles", extra=identity)
    f.run("roles", "--iam", "missing-agent", ok=False)

    f.launch("--role", "fronter.e2e", "--dir", f.child)
    helper = next(row for row in f.providers() if row["name"] != provider["name"])
    helper_view = f.parsed("roles", "--json", extra={"AGENTX_PROCESS_ID": helper["process"]})
    assert all("." not in row["name"] for row in helper_view["roles"])
    f.launch("--role", "global:fronter", "--dir", f.child)
    global_owner = next(row for row in f.providers() if row["name"] not in [provider["name"], helper["name"]])
    global_view = f.parsed("roles", "--json", extra={"AGENTX_PROCESS_ID": global_owner["process"]})
    assert all("." not in row["name"] for row in global_view["roles"]), "shadowed global helpers must not resolve to local namesakes"
    global_preview = f.parsed("launch", "--role", "global:fronter.e2e", "--dir", f.child, "--dry-run", "--json")
    assert Path(global_preview["collaboration"]["role"]["path"]).parent == f.scope / "roles"
    assert "OWNER.HELPER.md" in f.run("roles", "--help")
    assert "roles --all" in f.run("config", "bootstrap", "show", "capabilities")
    f.result["checks"] = ["public discovery", "all helpers and shadows", "explicit show", "launch dry-run", "passive human and missing identity", "native adopted-owner discovery", "persisted workspace", "explicit caller", "namespace isolation", "helper annotation", "explicit missing identity error", "actual helper launch", "helper caller isolation", "shadowed-owner discovery and explicit global launch", "shipped help and startup discovery"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    result = {"binary": str(args.binary.resolve())}
    fixture = None
    try:
        fixture = Fixture(args.binary.resolve(), result)
        check(fixture)
        result["status"] = "passed"
    except Exception:
        result["status"] = "failed"
        result["error"] = traceback.format_exc()
        raise
    finally:
        if fixture:
            fixture.close()
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"status": result["status"], "checks": len(result["checks"]), "evidence": str(args.output)}))


if __name__ == "__main__":
    main()
