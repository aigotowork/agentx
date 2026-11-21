#!/usr/bin/python3
"""Independent public office acceptance; explicit immutable candidate only."""
import argparse
import copy
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import traceback

from office_fixture import OfficeFixture, wait_for
from acceptance import Cases as ManagementCases, exact_query, public_shape, request_id


class OfficeCases(ManagementCases):
    def record(self, name, **facts):
        public_shape(facts)
        assert "PRIVATE_OFFICE_PROMPT_NEVER_PUBLIC" not in json.dumps(facts), "Role prompt leaked"
        super().record(name, **facts)

    def setup(self):
        self.http = self.fixture.prepare("primary")
        self.paths = self.fixture.layout("primary")
        self.project = self.paths["project"]
        self.scope = self.fixture.root / "primary/scope"

    def role_catalog(self, workspace):
        catalog = self.http.get("/api/v1/roles", workspace=str(workspace))
        public_shape(catalog)
        assert catalog["generation"] == self.http.generation
        assert catalog["workspace"] == (None if workspace == "global" else str(Path(workspace).resolve()))
        assert "PRIVATE_OFFICE_PROMPT_NEVER_PUBLIC" not in json.dumps(catalog)
        return catalog

    def preview(self, directory):
        entry = self.http.get("/api/v1/directory-entry", directory=str(directory))
        public_shape(entry)
        assert entry["generation"] == self.http.generation
        assert entry["directory"] == str(Path(directory).resolve())
        assert "PRIVATE_OFFICE_PROMPT_NEVER_PUBLIC" not in json.dumps(entry)
        return entry

    def launch_role(self, role, workspace, directory, title):
        payload = {"request_id": request_id(), "generation": self.http.generation,
                   "launch": {"role": role, "workspace": str(workspace), "directory": str(directory),
                              "prompt": "office fixture initial task", "title": title}}
        self.http.post("/api/v1/agents", payload)
        operation = self.http.operation(payload["request_id"])
        result = operation["result"]["Launched"]
        assert result["failed"] == 0 and len(result["agents"]) == 1, operation
        return self.live(result["agents"][0]), payload, operation

    def detail(self, reference):
        return self.http.get(f"/api/v1/agents/{reference['name']}", **exact_query(reference))

    def stopped(self, reference):
        return wait_for("exact stopped office member", lambda: next((view for view in self.http.agents()
                        if view["agent"]["name"] == reference["name"] and view["agent"].get("stopped_event")), None))

    def started(self):
        return [row for row in self.fixture.provider_records() if row["kind"] == "started"]

    def public_snapshot(self):
        views = self.http.agents()
        # Native hooks may finish a turn while GETs are observed. Lifecycle
        # status/detail are asynchronous activity, not preview side effects;
        # exact membership, refs, capabilities, messages and starts stay fixed.
        stable = [{key: value for key, value in view.items() if key not in {"status", "detail"}} for view in views]
        return {"agents": stable,
                "messages": [{"agent": view["agent"], "page": self.messages(view["agent"])} for view in views],
                "starts": self.started()}

    def pure_preview(self, directory, expected):
        before = self.public_snapshot()
        previews = [self.preview(directory) for _ in range(3)]
        assert all(entry["endpoint"]["status"] == expected for entry in previews), previews
        assert previews[0] == previews[1] == previews[2], previews
        after = self.public_snapshot()
        assert before == after, {"before": before, "after": after}
        return previews[0]

    def discovery(self):
        catalog = self.http.get("/api/v1/workspaces")
        paths = {row["path"] for row in catalog["workspaces"]}
        expected = {str(self.paths[key]) for key in ["project", "child", "empty"]}
        assert paths == expected, (paths, expected, catalog)
        assert all(row["sources"] for row in catalog["workspaces"])
        coverage = catalog["coverage"]
        assert not coverage["agent_limit_reached"] and coverage["scan_limit_reached"] and not coverage["errors"], catalog
        # The deliberate one/two/three/four fixture exceeds the depth cap;
        # honest discovery must mark incomplete instead of claiming full coverage.
        assert 1 <= coverage["agent_pages"] <= 10 and 1 <= coverage["directories_examined"] <= 512
        self.record("O01", catalog=catalog)

    def catalogs(self):
        child = self.role_catalog(self.paths["child"])
        roles = {role["name"]: role for role in child["roles"]}
        assert roles["reviewer"]["path"] == str(self.paths["child"] / ".agentx/roles/reviewer.toml"), child
        assert roles["reviewer"]["description"] == "child reviewer"
        assert roles["controller"]["path"] == str(self.project / ".agentx/roles/controller.toml")
        assert roles["global-only"]["path"] == str(self.scope / "roles/global-only.toml")
        global_roles = self.role_catalog("global")
        assert {role["name"] for role in global_roles["roles"]} == {"reviewer", "global-only"}
        assert {role["name"] for role in self.role_catalog(self.paths["explicit"])["roles"]} == {"reviewer", "global-only"}
        invalid = self.paths["invalid"] / ".agentx/roles"
        errors = [self.reject("GET", "/api/v1/directory-entry", (400, 422), query={"generation": self.http.generation, "directory": str(self.paths["cwd"])})]
        for content in ['broken = [', 'profile = "missing-profile"\nprompt = "test"\n']:
            (invalid / "bad.toml").write_text(content)
            if content.startswith("broken"):
                errors.append(self.reject("GET", "/api/v1/roles", (400, 422), query={"generation": self.http.generation, "workspace": str(self.paths["invalid"])}))
            else:
                catalog = self.role_catalog(self.paths["invalid"])
                assert next(role for role in catalog["roles"] if role["name"] == "bad")["profile"] == "missing-profile"
            errors.append(self.reject("GET", "/api/v1/directory-entry", (400, 422), query={"generation": self.http.generation, "directory": str(self.paths["invalid"])}))
        (invalid / "bad.toml").unlink()
        for name in ["one", "two"]:
            self.fixture.role(invalid, name, name, default=True)
        errors.append(self.reject("GET", "/api/v1/roles", (400, 422), query={"generation": self.http.generation, "workspace": str(self.paths["invalid"])}))
        for path in invalid.glob("*.toml"):
            path.unlink()
        for index in range(101):
            self.fixture.role(invalid, f"post-{index:03d}", "bounded role catalog")
        errors.append(self.reject("GET", "/api/v1/roles", (400, 422), query={"generation": self.http.generation, "workspace": str(self.paths["invalid"])}))
        for path in invalid.glob("*.toml"):
            path.unlink()
        self.record("O02", child=child, global_roles=global_roles, whole_query_errors=errors)

    def membership(self):
        view, payload, operation = self.launch_role("global:reviewer", self.paths["explicit"], self.paths["cwd"], "independent-membership")
        adopted = view["collaboration"]
        assert adopted["status"] == "adopted" and adopted["workspace"] == str(self.paths["explicit"]), view
        assert adopted["role"]["path"] == str(self.scope / "roles/reviewer.toml"), view
        assert adopted["role"]["namespace"] == "global:" + str(self.scope / "roles") and adopted["role"]["profile"] == "management-fixture", view
        assert view["directory"] == str(self.paths["cwd"]), view
        provider = wait_for("actual unrelated Provider cwd", lambda: next((row for row in self.started() if row["session"] == view["session_id"]), None))
        assert provider["cwd"] == str(self.paths["cwd"]), provider
        known = self.http.get("/api/v1/workspaces")
        assert str(self.paths["explicit"]) in {row["path"] for row in known["workspaces"]}
        assert str(self.paths["cwd"]) not in {row["path"] for row in known["workspaces"]}
        self.adopted_view, self.adopted_payload, self.adopted_receipt = view, payload, operation
        self.record("O03", agent=view, operation=operation, provider_cwd=provider["cwd"], catalog=known)

    def provenance(self):
        view = self.adopted_view
        original = view["collaboration"]
        path = self.scope / "roles/reviewer.toml"
        self.fixture.role(path.parent, "reviewer", "edited description")
        assert self.detail(view["agent"])["collaboration"] == original
        assert next(role for role in self.role_catalog("global")["roles"] if role["name"] == "reviewer")["description"] == "edited description"
        fresh, _, fresh_receipt = self.launch_role("global:reviewer", self.paths["explicit"], self.paths["cwd"], "fresh-after-role-edit")
        assert fresh["collaboration"]["role"]["description"] == "edited description"
        self.submit_target("stop", fresh["agent"])
        self.stopped(fresh["agent"])
        path.unlink()
        assert self.detail(view["agent"])["collaboration"] == original
        assert self.http.post("/api/v1/agents", self.adopted_payload) == self.adopted_receipt
        self.submit_target("stop", view["agent"])
        history = self.stopped(view["agent"])
        assert history["collaboration"] == original
        resumed, _ = self.submit_target("resume", history["agent"])
        successor = self.live(resumed["result"]["Launched"]["agents"][0])
        assert successor["collaboration"] == original, (original, successor)
        assert successor["session_id"] == history["session_id"]
        assert successor["agent"] != view["agent"]
        assert self.detail(history["agent"])["collaboration"] == original
        self.reject("GET", f"/api/v1/agents/{view['agent']['name']}", (409,), query={"generation": self.http.generation, **exact_query(view["agent"])})
        self.submit_target("stop", successor["agent"])
        known = self.http.get("/api/v1/workspaces")
        assert str(self.paths["explicit"]) in {row["path"] for row in known["workspaces"]}
        self.fixture.role(path.parent, "reviewer", "global reviewer")
        self.record("O04", original=original, fresh_after_edit=fresh, fresh_receipt=fresh_receipt, stopped=history, resumed=resumed, successor=successor, catalog=known)

    def directory_payload(self, directory, text):
        return {"request_id": request_id(), "generation": self.http.generation, "directory": str(directory),
                "message": {"text": text, "intent": "request", "reply_to": None, "thread": "office-acceptance"}}

    def directory_send(self, payload, disconnect=False):
        path = "/api/v1/directory-messages"
        if disconnect:
            self.http.call("POST", path, payload, disconnect=True)
        else:
            self.http.post(path, payload)
        receipt = self.http.operation(payload["request_id"])
        sent = receipt["result"]["DirectorySent"]
        assert sent["sent"]["recipients"] == [sent["recipient"]["name"]], sent
        message_id = sent["sent"]["message_id"]
        page = wait_for("actual directory recipient reply", lambda: self._reply_page(sent["recipient"], message_id))
        assert len([row for row in page["messages"] if row["id"] == message_id and row["text"] == payload["message"]["text"]]) == 1
        assert len([row for row in page["messages"] if row["reply_to"] == message_id]) == 1
        return receipt, sent, page

    def endpoints(self):
        nested = self.pure_preview(self.paths["nested"], "absent")
        assert nested["requested_workspace"] == str(self.paths["child"])
        assert nested["resolved_workspace"] == str(self.project)
        assert nested["role"]["path"] == str(self.project / ".agentx/roles/controller.toml")
        directory = self.paths["outside"]
        self.pure_preview(directory, "absent")
        first, _, _ = self.launch_role("dispatcher", directory, self.paths["cwd"], "endpoint-first")
        unique = self.pure_preview(directory, "adoptable")
        assert unique["endpoint"]["participant"] == first["agent"] and unique["endpoint"]["candidates"] == 1
        second, _, _ = self.launch_role("dispatcher", directory, self.paths["cwd"], "endpoint-second")
        ambiguous = self.pure_preview(directory, "ambiguous")
        assert ambiguous["endpoint"]["candidates"] == 2
        bad = self.directory_payload(directory, "ambiguity must not deliver")
        self.http.post("/api/v1/directory-messages", bad)
        failed = self.http.operation(bad["request_id"], success=False)
        assert failed["status"] == "Failed", failed
        assert not any(row["text"] == bad["message"]["text"] for member in [first, second] for row in self.messages(member["agent"])["messages"])
        self.submit_target("stop", second["agent"])
        self.stopped(second["agent"])
        self.pure_preview(directory, "adoptable")
        self.endpoint_first = first
        self.record("O05", inherited=nested, unique=unique, ambiguous=ambiguous, rejected_send=failed)

    def replay(self):
        directory = self.paths["outside"]
        before_starts = len(self.started())
        payload = self.directory_payload(directory, "directory original response intentionally lost")
        receipt, sent, page = self.directory_send(payload, disconnect=True)
        assert sent["recipient"] == self.endpoint_first["agent"] and sent["created"] is False, sent
        bound = self.pure_preview(directory, "bound")
        assert bound["endpoint"]["participant"] == sent["recipient"]
        path = directory / ".agentx/roles/dispatcher.toml"
        self.fixture.role(path.parent, "dispatcher", "definition drift", default=True)
        same_file = self.pure_preview(directory, "bound")
        assert same_file["role"]["description"] == "definition drift"
        assert self.detail(sent["recipient"])["collaboration"] == self.endpoint_first["collaboration"]
        self.fixture.role(path.parent, "dispatcher", "same identity, no longer default", default=False)
        alternate = self.fixture.role(path.parent, "alternate", "different default identity", default=True)
        drift = self.pure_preview(directory, "role_changed")
        assert drift["role"]["path"] == str(alternate)
        path.unlink()
        alternate.unlink()
        replay = self.http.post("/api/v1/directory-messages", payload)
        assert replay == receipt
        changed = copy.deepcopy(payload)
        changed["message"]["text"] = "same request ID different body"
        self.reject("POST", "/api/v1/directory-messages", (409,), payload=changed)
        assert len(self.started()) == before_starts
        assert self.messages(sent["recipient"])["messages"] == page["messages"]
        self.record("O06", receipt=receipt, replay=replay, bound=bound, same_file_edit=same_file, drift=drift, messages=page)

    def creation(self):
        payload = self.directory_payload(self.paths["nested"], "create inherited parent entry")
        starts_before = len(self.started())
        with ThreadPoolExecutor(max_workers=2) as executor:
            duplicate_admissions = list(executor.map(lambda _: self.http.post("/api/v1/directory-messages", payload), range(2)))
        created, sent, page = self.directory_send(payload)
        assert len(self.started()) == starts_before + 1, "Concurrent duplicate created more than one Provider"
        assert sent["created"] is True and sent["entry"]["resolved_workspace"] == str(self.project), sent
        parent = self.detail(sent["recipient"])
        assert parent["collaboration"]["workspace"] == str(self.project)
        # Native directory send launches at the declaring default workspace.
        assert parent["directory"] == str(self.project), parent
        again, reused, _ = self.directory_send(self.directory_payload(self.paths["nested"], "reuse inherited parent entry"))
        assert reused["created"] is False and reused["recipient"] == sent["recipient"]
        self.fixture.role(self.scope / "roles", "global-default", "global default", default=True)
        global_preview = self.pure_preview(self.paths["cwd"], "absent")
        assert global_preview["requested_workspace"] is None and global_preview["resolved_workspace"] is None
        global_receipt, global_sent, _ = self.directory_send(self.directory_payload(self.paths["cwd"], "create global entry"))
        global_agent = self.detail(global_sent["recipient"])
        assert global_agent["collaboration"]["status"] == "adopted" and global_agent["collaboration"]["workspace"] is None
        assert global_agent["directory"] == str(self.paths["cwd"]), global_agent
        assert global_sent["created"] is True
        self.record("O07", parent=parent, created=created, duplicate_admissions=duplicate_admissions, reused=again, global_preview=global_preview,
                    global_receipt=global_receipt, global_agent=global_agent)

    def admission(self):
        before = self.public_snapshot()
        valid = {"request_id": request_id(), "generation": self.http.generation,
                 "launch": {"role": "global:reviewer", "workspace": str(self.paths["explicit"]), "directory": str(self.paths["cwd"])}}
        for overrides in [{"tool": "qoder"}, {"profile": "management-fixture"}, {"workspace": "relative"}, {"role": ""}]:
            body = copy.deepcopy(valid)
            body["request_id"] = request_id()
            body["launch"].update(overrides)
            self.reject("POST", "/api/v1/agents", payload=body)
        for directory in ["relative", "", "bad\0directory"]:
            self.reject("POST", "/api/v1/directory-messages", payload=self.directory_payload(directory, "reject invalid directory"))
        stale = self.directory_payload(self.paths["nested"], "stale generation")
        stale["generation"] = request_id()
        self.reject("POST", "/api/v1/directory-messages", (409,), payload=stale)
        for path, query in [("/api/v1/roles", {"workspace": "relative"}), ("/api/v1/directory-entry", {"directory": "relative"}),
                            ("/api/v1/workspaces", {"directory": str(self.project)})]:
            self.reject("GET", path, query={"generation": self.http.generation, **query})
        assert before == self.public_snapshot()
        self.record("O08", rejected_without_public_effect=True)

    def coverage(self):
        root = self.project / "bounded-discovery"
        for index in range(105):
            (root / f"office-{index:03d}/.agentx/roles").mkdir(parents=True)
        capped = self.http.get("/api/v1/workspaces")
        assert len(capped["workspaces"]) <= 100 and capped["coverage"]["scan_limit_reached"], capped
        # Remove markers and exceed the independent directory-examination budget.
        for path in root.glob("office-*/.agentx/roles"):
            path.rmdir()
        for index in range(520):
            (root / f"plain-{index:03d}").mkdir()
        scan = self.http.get("/api/v1/workspaces")
        assert scan["coverage"]["directories_examined"] <= 512 and scan["coverage"]["scan_limit_reached"], scan
        self.record("O09", workspace_cap=capped, scan_cap=scan)

    def browser(self):
        self.http = self.fixture.prepare("browser")
        self.paths = self.fixture.layout("browser")
        config = {"url": self.http.url, "generation": self.http.generation,
                  "scope": str(self.fixture.root / "browser/scope"),
                  "paths": {key: str(value) for key, value in self.paths.items()},
                  "evidence": str(self.fixture.evidence / "office-browser")}
        path = self.fixture.evidence / "office-browser-input.json"
        path.write_text(json.dumps(config, indent=2) + "\n")
        completed = subprocess.run(["node", str(Path(__file__).with_name("browser.mjs")), str(path)],
                                   capture_output=True, text=True, env=os.environ.copy(), timeout=240)
        (self.fixture.evidence / "office-browser-output.json").write_text(json.dumps({"code": completed.returncode,
            "stdout": completed.stdout, "stderr": completed.stderr}, indent=2) + "\n")
        assert completed.returncode == 0, completed.stderr
        self.record("OB01-OB03", report="office-browser/report.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--evidence", required=True)
    parser.add_argument("--browser", action="store_true")
    parser.add_argument("--browser-only", action="store_true")
    parser.add_argument("--actions-only", action="store_true", help="New endpoint/action/coverage fixture; preserve earlier read/provenance passes on unchanged candidate")
    args = parser.parse_args()
    fixture = OfficeFixture(args.candidate, args.evidence)
    cases = OfficeCases(fixture)
    failure = None
    planned = ["O01", "O02", "O03", "O04", "O05", "O06", "O07", "O08", "O09"]
    try:
        if not args.browser_only:
            cases.setup()
            selected = [cases.endpoints, cases.replay, cases.creation, cases.admission, cases.coverage] if args.actions_only else [
                cases.discovery, cases.catalogs, cases.membership, cases.provenance, cases.endpoints,
                cases.replay, cases.creation, cases.admission, cases.coverage]
            for case in selected:
                case()
        if args.browser or args.browser_only:
            cases.browser()
    except Exception:
        failure = traceback.format_exc()
        print(failure, flush=True)
    finally:
        try:
            fixture.close()
        except Exception:
            failure = (failure or "") + "\nCleanup: " + traceback.format_exc()
        digest = hashlib.sha256(fixture.candidate.read_bytes()).hexdigest()
        if digest != fixture.manifest["sha256"]:
            failure = (failure or "") + "\nCandidate changed during execution"
        report = {"status": "fail" if failure else "pass", "failure": failure, "cases": cases.steps,
                  "unexecuted": [name for name in planned if name not in {step["case"] for step in cases.steps}],
                  "browser_requested": args.browser or args.browser_only, "browser_only": args.browser_only, "actions_only": args.actions_only,
                  "candidate_sha256": digest,
                  "limits": "Public HTTP/browser only. Native storage corruption/no-write witnesses separately owned; no paid Provider."}
        (fixture.evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    raise SystemExit(1 if failure else 0)


if __name__ == "__main__":
    main()
