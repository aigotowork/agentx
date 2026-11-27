#!/usr/bin/env python3
"""Convention roles through real CLI/native launch and HTTP, using disposable fake Providers.

Run: python3 tests/convention_roles.py --binary /absolute/agentx --output /new/evidence
No installed programs, user scopes or running services are changed.
"""
import argparse
import json
from pathlib import Path
import sys
import traceback
import uuid

from roles_workspaces import Fixture


def markdown(project, name):
    path = project / f".agent-guides/roles/{name}.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"---\nschema: role\nrole: {name}\ndescription: Shared {name}\n---\nLiteral $HOME {{{{name}}}}\n")
    return path


def cli_cases(binary, report):
    f = Fixture(binary, report)
    try:
        project = f.root / "published"
        source = markdown(project, "coder")
        markdown(project, "coder.test")
        (source.parent / "README.md").write_text("# Reference")
        config = f.scope / "config.toml"
        config.write_text(config.read_text().replace('default_profile = "ambient"', ''))
        catalog = f.parsed("roles", "--dir", project, "--json")
        assert catalog["workspace"] == str(project)
        assert [r["name"] for r in catalog["roles"]] == ["coder", "reviewer"]
        role = catalog["roles"][0]
        assert role["profile"] is None and role["path"] == str(source)
        assert "No default profile" in f.run("launch", "--dir", project, "--role", "coder", "--dry-run", ok=False)
        preview = f.parsed("launch", "--dir", project, "--role", "coder", "--profile", "override", "--dry-run", "--json")
        assert preview["collaboration"]["role"]["profile"] == "override"
        assert not (f.scope / "agentx.db").exists()
        override = project / ".agentx/roles/coder.toml"
        override.parent.mkdir(parents=True)
        override.write_text('profile = "deep"\n')
        f.launch("--dir", project, "--role", "coder", "--profile", "override")
        provider = f.providers()[0]
        adopted = json.loads(f.query("SELECT launch_context FROM instances WHERE name=?", (provider["name"],))[0]["launch_context"])["collaboration"]["role"]
        assert adopted["profile"] == "override" and adopted["path"] == str(override)
        source.write_text("---\nschema: role\n---\nEdited")
        discovery = f.parsed("roles", "--json", extra={"AGENTX_PROCESS_ID": provider["process"]})
        assert "coder.test" in [role["name"] for role in discovery["roles"]]
        assert adopted["prompt"].endswith("Literal $HOME {{name}}\n")
        f.role(f.scope / "roles", "coder", "Far fallback")
        override.write_text("disabled = true\n")
        assert "coder" not in [role["name"] for role in f.parsed("roles", "--dir", project, "--all", "--json")["roles"]]
        assert "disabled" in f.run("roles", "show", "coder", "--dir", project, ok=False)
        markdown(project, "controller")
        (override.parent / "controller.toml").write_text("default = true\n")
        # Directory routing must select the caller environment profile when the thin role omits one.
        routes = [f.parsed("send", "--dir", project, "--intent", "inform", "--json", "--", text,
                           extra={"AGENTX_PROFILE": "ambient"})["directory_route"] for text in ["first", "second"]]
        f.wait_providers(2)
        assert routes[0]["role"]["profile"] == "ambient"
        assert routes[0]["recipient"] == routes[1]["recipient"]
        assert [route["created"] for route in routes] == [True, False]
        help_text = f.run("roles", "--help")
        capabilities = f.run("config", "bootstrap", "show", "capabilities")
        reference = f.run("docs", "--cli")
        for text in [help_text, capabilities, reference]:
            assert ".agent-guides/roles/" in text and "disabled = true" in text
        assert "[--role NAME] [--profile NAME]" in f.run("--help")
        assert "[--role ROLE] [--profile PROFILE]" in capabilities
        report["status"] = "passed"
    finally:
        f.close()


def http_cases(binary, output):
    sys.path.insert(0, str(Path(__file__).parent / "agent_management"))
    from fixture import Fixture as HttpFixture, wait_for
    f = HttpFixture(binary, output)
    try:
        http = f.prepare("convention")
        project = f.root / "convention/project"
        source = markdown(project, "coder")
        role = http.get("/api/v1/roles", workspace=str(project))["roles"][0]
        assert role["name"] == "coder" and role["profile"] is None
        assert "prompt" not in role
        offices = http.get("/api/v1/workspaces")
        assert str(project) in [row["path"] for row in offices["workspaces"]]
        def launch():
            payload = {"request_id": str(uuid.uuid4()), "generation": http.generation, "launch": {"role": "coder", "directory": str(project)}}
            http.post("/api/v1/agents", payload)
            return http.operation(payload["request_id"], success=False)
        failed = launch()
        assert failed["status"] == "Failed" and "No default profile" in json.dumps(failed)
        config = f.root / "convention/scope/config.toml"
        config.write_text(config.read_text().replace('[launch.profiles.management-fixture]', '[launch]\ndefault_profile = "management-fixture"\n[launch.profiles.management-fixture]'))
        result = launch()
        assert result["status"] == "Succeeded", result
        name = result["result"]["Launched"]["agents"][0]
        view = wait_for("adopted Markdown role", lambda: next((row for row in http.agents() if row["agent"]["name"] == name), None))
        adopted = view["collaboration"]["role"]
        assert adopted["path"] == str(source) and adopted["profile"] == "management-fixture"
        assert view["collaboration"]["workspace"] == str(project)
        markdown(project, "controller")
        override = project / ".agentx/roles/controller.toml"
        override.parent.mkdir(parents=True)
        override.write_text("default = true\n")
        preview = http.get("/api/v1/directory-entry", directory=str(project))
        assert preview["role"]["path"] == str(override) and preview["role"]["profile"] == "management-fixture"
        (output / "result.json").write_text(json.dumps({"status": "passed", "adopted": adopted, "preview": preview}, indent=2) + "\n")
    finally:
        f.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    report = {}
    try:
        cli_cases(args.binary.resolve(), report)
        http_cases(args.binary.resolve(), args.output / "http")
        report["status"] = "passed"
    except Exception:
        report.update(status="failed", error=traceback.format_exc())
        print(report["error"])
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print("convention roles: " + report["status"])
    raise SystemExit(report["status"] != "passed")


if __name__ == "__main__":
    main()
