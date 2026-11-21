#!/usr/bin/env python3
"""Finite native/HTTP/browser acceptance. Requires an explicit frozen artifact."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import traceback
import uuid

from relation_fixture import RelationFixture, wait_for
from provider import process_identity


def exact(reference):
    return {key: value for key, value in reference.items() if key != "name" and value is not None}


class Cases:
    def __init__(self, fixture, external_http=False):
        self.fixture, self.steps, self.refs = fixture, [], {}
        self.external_http = external_http
        self.http = fixture.prepare("primary")
        self.paths = fixture.layout("primary")

    def record(self, case, **facts):
        self.steps.append({"case": case, "status": "pass", **facts})
        (self.fixture.evidence / "cases.json").write_text(json.dumps(self.steps, indent=2) + "\n")
        print(case + " passed", flush=True)

    def relations(self, reference, **query):
        return self.http.get(f"/api/v1/agents/{reference['name']}/launch-relations", **exact(reference), **query)

    def current(self, name):
        return wait_for("actual generated-hook participant " + name, lambda: next((row for row in self.http.agents()
            if row["agent"]["name"] == name and row["agent"]["incarnation"] and row["session_id"] and row["can_send"]), None))

    def native(self, label, actor=None, command="launch", target=None, office="project", force_cli=False):
        before = {row["agent"]["incarnation"] for row in self.http.agents() if row["agent"]["incarnation"]}
        argv = [command] + ([target] if target else [])
        if command == "launch":
            argv += ["--role", "reviewer", "--workspace", str(self.paths[office])]
        argv += ["--dir", str(self.paths[office]), "--headless", "--terminal", "pty", "--go", "--json", "--title", label]
        if actor:
            receipt = self.fixture.command(actor["name"], argv)
        elif self.external_http and command == "launch" and not force_cli:
            payload = {"generation": self.http.generation, "request_id": str(uuid.uuid4()),
                       "launch": {"role": "reviewer", "workspace": str(self.paths[office]), "directory": str(self.paths[office]),
                                  "prompt": "local generated-hook acceptance", "tool": None, "profile": None, "title": label}}
            self.http.post("/api/v1/agents", payload)
            self.capture_http_workers(payload["request_id"])
            receipt = self.http.operation(payload["request_id"])
            assert receipt["result"]["Launched"]["failed"] == 0, receipt
        else:
            result = self.fixture.cli("primary", label, argv)
            receipt = {"stdout": result.stdout, "stderr": result.stderr}
        def created():
            rows = [row for row in self.http.agents() if row["agent"]["incarnation"] and row["agent"]["incarnation"] not in before and row["title"] == label]
            assert len(rows) <= 1, rows
            return rows[0] if rows and rows[0]["session_id"] and rows[0]["can_send"] else None
        view = wait_for("new native exact participant " + label, created, timeout=40)
        self.refs[label] = view["agent"]
        return view, receipt

    def capture_http_workers(self, request_id):
        """One passive snapshot during existing admission; no extra launch/poll."""
        observed = []
        for entry in Path("/proc").iterdir():
            if not entry.name.isdigit():
                continue
            try:
                environment = dict(field.split(b"=", 1) for field in (entry / "environ").read_bytes().split(b"\0") if b"=" in field)
                if environment.get(b"AGENTX_MANAGEMENT_FIXTURE") != os.fsencode(self.fixture.root):
                    continue
                identity = process_identity(int(entry.name))
                if identity["executable"] == str(self.fixture.candidate):
                    identity["exe_sha256"] = hashlib.sha256((entry / "exe").read_bytes()).hexdigest()
                    observed.append(identity)
            except (OSError, ValueError):
                continue
        (self.fixture.evidence / f"http-worker-{request_id}.json").write_text(json.dumps({"request_id": request_id,
            "observed": observed, "limit": "One passive process snapshot after actual HTTP admission; short-lived worker may be absent."}, indent=2) + "\n")

    @staticmethod
    def edge(item, reference):
        assert item["name"] == reference["name"] and item["incarnation"] == reference["incarnation"], (item, reference)
        assert item["availability"] == "available" and item["agent"]["agent"] == reference, (item, reference)

    def tree(self):
        a, ar = self.native("A")
        if self.external_http:
            external, cli_receipt = self.native("external-cli", force_cli=True)
            external_page = self.relations(external["agent"])
            assert external_page["origin"] == "external" and external_page["upstream"] is None, external_page
            self.record("R01-external-cli", view=external, native=cli_receipt, page=external_page)
        b, br = self.native("B", a["agent"])
        c, cr = self.native("C", a["agent"])
        d, dr = self.native("D", b["agent"], office="child")
        pages = {label: self.relations(self.refs[label]) for label in "ABCD"}
        assert pages["A"]["origin"] == "external" and pages["A"]["upstream"] is None, pages
        assert {item["incarnation"] for item in pages["A"]["downstream"]} == {b["agent"]["incarnation"], c["agent"]["incarnation"]}, pages
        for child, parent in [("B", "A"), ("C", "A"), ("D", "B")]:
            assert pages[child]["origin"] == "agent", pages[child]
            self.edge(pages[child]["upstream"], self.refs[parent])
        assert len(pages["B"]["downstream"]) == 1 and not pages["D"]["downstream"]
        self.edge(pages["B"]["downstream"][0], d["agent"])
        assert d["collaboration"]["workspace"] == str(self.paths["child"])
        self.record("R01-R02", refs=self.refs.copy(), pages=pages, native=[ar, br, cr, dr])

    def paging(self):
        first = self.relations(self.refs["A"], limit=1)
        assert len(first["downstream"]) == 1 and first["next"], first
        second = self.relations(self.refs["A"], limit=1, after=first["next"])
        assert len(second["downstream"]) == 1 and second["next"] is None, second
        assert {row["incarnation"] for row in first["downstream"] + second["downstream"]} == {self.refs[key]["incarnation"] for key in "BC"}
        errors = []
        for values in [{"limit": 0}, {"limit": 101}, {"after": "not-a-creation-cursor"}]:
            status, body = self.http.call("GET", f"/api/v1/agents/{self.refs['A']['name']}/launch-relations",
                query={"generation": self.http.generation, **exact(self.refs["A"]), **values})
            assert status in [400, 409], (status, body)
            errors.append({"input": values, "status": status, "body": body})
        status, body = self.http.call("GET", f"/api/v1/agents/{self.refs['D']['name']}/launch-relations",
            query={"generation": self.http.generation, **exact(self.refs["D"]), "after": first["next"]})
        assert status in [400, 409], (status, body)
        self.record("R03", first=first, second=second, invalid=errors, foreign_cursor={"status": status, "body": body})

    def browser(self, phase):
        config = {"url": self.http.url, "generation": self.http.generation, "scope": str(self.fixture.root / "primary/scope"),
                  "refs": self.refs, "phase": phase, "offices": {key: str(value) for key, value in self.paths.items()},
                  "evidence": str(self.fixture.evidence / ("browser-" + phase))}
        if phase == "reset":
            config.update(candidate=str(self.fixture.candidate), environment=self.fixture.environment("primary"),
                          cwd=str(self.fixture.root / "primary/project"))
        path = self.fixture.evidence / ("browser-" + phase + "-input.json")
        path.write_text(json.dumps(config, indent=2) + "\n")
        result = subprocess.run(["node", str(Path(__file__).with_name("browser.mjs")), str(path)], capture_output=True, text=True, timeout=180)
        (self.fixture.evidence / ("browser-" + phase + "-output.json")).write_text(json.dumps({"code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}, indent=2) + "\n")
        assert result.returncode == 0, (result.stdout, result.stderr)
        self.record("R04-R11-" + phase, report=config["evidence"] + "/report.json")

    def stop(self, label):
        reference = self.refs[label]
        payload = {"generation": self.http.generation, "request_id": str(uuid.uuid4()), "agent": reference}
        self.http.post(f"/api/v1/agents/{reference['name']}/stop", payload)
        receipt = self.http.operation(payload["request_id"])
        view = wait_for("exact stopped history", lambda: next((row for row in self.http.agents()
            if row["agent"]["name"] == reference["name"] and row["agent"]["stopped_event"]), None))
        return view, receipt

    def lifecycle(self):
        old_b = self.refs["B"].copy()
        stopped, receipt = self.stop("B")
        self.refs["B-stopped"] = stopped["agent"]
        a_page, d_page = self.relations(self.refs["A"]), self.relations(self.refs["D"])
        b_item = next(item for item in a_page["downstream"] if item["incarnation"] == old_b["incarnation"])
        assert b_item["agent"]["agent"] == stopped["agent"] and b_item["agent"]["status"] == "stopped", b_item
        assert d_page["upstream"] == b_item, (d_page, b_item)
        self.record("R05-stopped", stop=receipt, original=old_b, stopped=stopped, parent=a_page, child=d_page)
        self.browser("stopped")
        resumed, resume_receipt = self.native("B-resumed", command="resume", target=old_b["name"])
        resumed_page = self.relations(resumed["agent"])
        self.edge(resumed_page["upstream"], self.refs["A"])
        self.edge(resumed_page["downstream"][0], self.refs["D"])
        assert resumed["agent"]["incarnation"] != old_b["incarnation"]
        after = self.relations(self.refs["A"])
        old_target = next(item for item in after["downstream"] if item["incarnation"] == old_b["incarnation"])
        assert old_target["agent"]["agent"] == stopped["agent"], old_target
        forked, fork_receipt = self.native("fork-D-by-A", actor=self.refs["A"], command="fork", target=self.refs["D"]["name"])
        fork_page = self.relations(forked["agent"])
        self.edge(fork_page["upstream"], self.refs["A"])
        self.record("R06", resumed=resumed, resume=resume_receipt, resume_page=resumed_page, original_target=old_target,
                    fork=fork_receipt, forked=forked, fork_page=fork_page)

    def stopped_statuses(self):
        original_a, original_d = self.refs["A"].copy(), self.refs["D"].copy()
        stopped_a, a_receipt = self.stop("A")
        stopped_d, d_receipt = self.stop("D")
        resumed = self.relations(self.refs["B-resumed"])
        assert resumed["upstream"]["incarnation"] == original_a["incarnation"]
        assert resumed["upstream"]["agent"]["agent"] == stopped_a["agent"]
        assert resumed["upstream"]["agent"]["status"] == "stopped"
        d_item = next(item for item in resumed["downstream"] if item["incarnation"] == original_d["incarnation"])
        assert d_item["agent"]["agent"] == stopped_d["agent"] and d_item["agent"]["status"] == "stopped"
        self.refs["A"], self.refs["D"] = stopped_a["agent"], stopped_d["agent"]
        self.record("R07-stopped-status", upstream=a_receipt, downstream=d_receipt, page=resumed)

    def restart_reset(self, browser_reset=False):
        before = {key: self.relations(self.refs[key]) for key in ["A", "D", "B-resumed"]}
        generation = self.http.generation
        self.fixture.stop_daemon("primary")
        self.http = self.fixture.start("primary")
        assert self.http.generation == generation
        after = {key: self.relations(self.refs[key]) for key in before}
        for key in before:
            assert [(item["name"], item["incarnation"]) for item in after[key]["downstream"]] == [(item["name"], item["incarnation"]) for item in before[key]["downstream"]]
            assert after[key]["origin"] == before[key]["origin"]
            assert after[key]["upstream"] == before[key]["upstream"]
        self.record("R09", before=before, after=after)
        if browser_reset:
            self.browser("reset")
        else:
            self.fixture.cli("primary", "reset", ["reset"])
        descriptor = wait_for("changed generation", lambda: self.changed_generation(generation))
        assert descriptor["generation"] != generation
        status, body = self.http.call("GET", f"/api/v1/agents/{self.refs['A']['name']}/launch-relations", query={"generation": generation, **exact(self.refs["A"])})
        assert status == 409, (status, body)
        self.http.generation = descriptor["generation"]
        assert self.http.agents() == []
        self.record("R10-api", old_generation=generation, descriptor=descriptor, stale={"status": status, "body": body})

    def changed_generation(self, generation):
        value = self.http.get("/api/v1/management", generation=None)
        return value if value["generation"] != generation else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--api-only", action="store_true")
    parser.add_argument("--external-http", action="store_true", help="partial evidence: external A uses public management, CLI admission remains unverified")
    args = parser.parse_args()
    assert hashlib.sha256(args.candidate.read_bytes()).hexdigest() == args.sha256, "candidate hash mismatch"
    fixture = RelationFixture(args.candidate, args.evidence)
    failure = None
    try:
        cases = Cases(fixture, external_http=args.external_http)
        cases.tree()
        cases.paging()
        if not args.api_only:
            cases.browser("tree")
        # lifecycle() retains a browser checkpoint for stopped exact history.
        if args.api_only:
            cases.browser = lambda _phase: None
        cases.lifecycle()
        cases.stopped_statuses()
        cases.restart_reset(browser_reset=not args.api_only)
    except Exception:
        failure = traceback.format_exc()
        print(failure, flush=True)
    finally:
        try:
            fixture.close()
        except Exception:
            failure = (failure or "") + traceback.format_exc()
        (fixture.evidence / "result.json").write_text(json.dumps({"status": "fail" if failure else "pass", "failure": failure,
            "external_admission": "public-management-and-native-cli" if args.external_http else "native-cli",
            "limits": ["local Qoder generated-hook substitute, no paid Provider compatibility", "R05 unrelated successor, R07 retention removal and R08 legacy/failure require native supplement evidence"]}, indent=2) + "\n")
    if failure:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
