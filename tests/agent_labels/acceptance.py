#!/usr/bin/python3
"""Finite independent public CLI/HTTP labels acceptance; explicit candidate only."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import hashlib
import http.client
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import traceback
from urllib.parse import urlencode
import uuid

MANAGEMENT_FIXTURES = Path(__file__).resolve().parents[1] / "agent_management"
sys.path.insert(0, str(MANAGEMENT_FIXTURES))
from fixture import Fixture, Http, wait_for


def identity():
    return str(uuid.uuid4())


def exact(reference):
    return {key: value for key, value in reference.items() if key != "name" and value is not None}


class ObservedHttp(Http):
    """Retain caller latency separately from native storage/query-plan evidence."""

    def call(self, method, path, *args, **options):
        started = time.monotonic()
        try:
            return super().call(method, path, *args, **options)
        finally:
            self.timings.append({"method": method, "path": path, "query": options.get("query"), "milliseconds": (time.monotonic() - started) * 1000})


class LabelsFixture(Fixture):
    """Reuse finite Provider/cleanup, retaining ordinary SQLite durability settings."""

    def __init__(self, candidate, evidence):
        super().__init__(candidate, evidence)
        self.timings = []
        # The original management fixture never exercised fork and reuses a
        # --resume session verbatim. Adapt only this run's private copy to the
        # Provider's distinct --fork-session result; preserve upstream runners.
        provider = self.root / "bin/qoder"
        content = provider.read_text()
        original = 'session = arguments[arguments.index("--resume") + 1] if resumed else str(uuid.uuid4())'
        assert content.count(original) == 1, "management Provider fixture changed; reassess fork binding"
        provider.write_text(content.replace(original, 'session = arguments[arguments.index("--resume") + 1] if resumed and "--fork-session" not in arguments else str(uuid.uuid4())'))
        self.manifest["provider_adaptation"] = {"original_sha256": hashlib.sha256((MANAGEMENT_FIXTURES / "provider.py").read_bytes()).hexdigest(),
                                                "private_sha256": hashlib.sha256(provider.read_bytes()).hexdigest(),
                                                "behavior": "--fork-session reports a new local fixture session; native production implementation unchanged"}
        self.persist()

    def connect(self, name, project=None):
        original = super().connect(name, project)
        observed = ObservedHttp(original.url, self.evidence, original.prefix)
        observed.generation, observed.sequence, observed.timings = original.generation, original.sequence, self.timings
        self.clients[name] = observed
        return observed

    def environment(self, name):
        result = super().environment(name)
        result.pop("AGENTX_TEST_DB_NOSYNC", None)
        return result


class Cases:
    """Compose public observations; never inspect managed storage."""

    def __init__(self, fixture):
        self.fixture, self.http, self.reference = fixture, None, None
        self.results, self.sequence = [], 0
        self.stage = "setup"

    def record(self, case, **facts):
        self.results.append({"case": case, "status": "pass", **facts})
        (self.fixture.evidence / "cases.json").write_text(json.dumps(self.results, indent=2, ensure_ascii=False) + "\n")
        print(case + " passed", flush=True)

    def cli(self, args, valid=True):
        self.sequence += 1
        result = subprocess.run([str(self.fixture.candidate), *args], env=self.fixture.environment("primary"),
                                cwd=self.fixture.root / "primary/project", capture_output=True, text=True, timeout=60)
        record = {"argv": args, "code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
        (self.fixture.evidence / f"cli-{self.sequence:03d}.json").write_text(json.dumps(record, indent=2) + "\n")
        assert (result.returncode == 0) == valid, record
        return json.loads(result.stdout) if valid and "--json" in args else record

    def page(self, labels=(), **fields):
        query = {"generation": self.http.generation, "current_only": "true", "label": list(labels), **fields}
        status, body = self.http.call("GET", "/api/v1/agents?" + urlencode({k: v for k, v in query.items() if v is not None}, doseq=True))
        assert status == 200, (status, body)
        assert body["generation"] == self.http.generation and len(body["agents"]) <= int(fields.get("limit", 100)), body
        assert "total" not in body, "ordinary filtered polling must not force total counting"
        return body

    def snapshot(self, reference=None):
        reference = reference or self.reference
        value = self.http.get(f"/api/v1/agents/{reference['name']}/labels", **exact(reference))
        assert value["agent"] == reference and value["generation"] == self.http.generation, value
        assert isinstance(value["revision"], str) and isinstance(value["labels"], dict), value
        return value

    def payload(self, reference=None, set=None, remove=None, revision=None):
        return {"request_id": identity(), "generation": self.http.generation,
                "agent": reference or self.reference, "set": set or {}, "remove": remove or [], "if_revision": revision}

    def patch(self, payload):
        self.http.post(f"/api/v1/agents/{payload['agent']['name']}/labels", payload)
        operation = self.http.operation(payload["request_id"])
        value = operation["result"]["LabelsPatched"]
        assert isinstance(value["changed"], bool) and value["snapshot"]["agent"] == payload["agent"], value
        return value, operation

    def refuse(self, method, path, expected=(400, 409), **options):
        status, value = self.http.call(method, path, **options)
        assert status in expected and isinstance(value.get("error"), dict), (status, value)
        assert isinstance(value["error"].get("code"), str) and value["error"].get("message"), value
        return {"status": status, "response": value}

    def live(self, name):
        return wait_for("fixture Provider ready " + name,
                        lambda: next((view for view in self.page()["agents"] if view["agent"]["name"] == name
                                      and view["agent"].get("incarnation") and view["session_id"] and view["can_send"]), None), timeout=45)

    def launch(self, title, labels):
        payload = {"request_id": identity(), "generation": self.http.generation,
                   "launch": {"tool": "qoder", "profile": None, "directory": None,
                              "prompt": "labels acceptance fixture", "title": title, "labels": labels}}
        self.http.post("/api/v1/agents", payload)
        result = self.http.operation(payload["request_id"])["result"]["Launched"]
        assert result["failed"] == 0 and len(result["agents"]) == 1, result
        view = self.live(result["agents"][0])
        assert view["labels"] == labels and isinstance(view["labels_revision"], str), view
        return view

    def target_action(self, action, reference):
        payload = {"request_id": identity(), "generation": self.http.generation, "agent": reference}
        self.http.post(f"/api/v1/agents/{reference['name']}/{action}", payload)
        return self.http.operation(payload["request_id"])

    def creation(self):
        self.http = self.fixture.prepare("primary")
        descriptor = self.http.get("/api/v1/management")
        assert descriptor["supports_labels"] is True and self.page()["agents"] == [], descriptor
        if self.fixture.assets:
            manifest = json.loads((self.fixture.assets / "manifest.json").read_text())
            assets = []
            for entry in manifest["files"]:
                connection = http.client.HTTPConnection(self.http.origin.hostname, self.http.origin.port, timeout=15)
                try:
                    connection.request("GET", entry["url"])
                    response = connection.getresponse()
                    data = response.read()
                    assert response.status == 200 and data == (self.fixture.assets / entry["file"]).read_bytes(), entry
                    assets.append({**entry, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
                finally:
                    connection.close()
            (self.fixture.evidence / "served-assets.json").write_text(json.dumps(assets, indent=2) + "\n")
        self.initial = {"project": "atlas", "feature": "search", "unknown/key": "<img src=x onerror=alert(1)>", "note": "a=b,界"}
        self.cli(["launch", "--tool", "qoder", "--no-profile", "--headless", "--go", "--json", "--title", "labels-cli-initial",
                  *[item for key, value in self.initial.items() for item in ["--label", key + "=" + value]]])
        view = wait_for("CLI launch visible", lambda: next((v for v in self.page()["agents"] if v["title"] == "labels-cli-initial"), None))
        self.reference = self.live(view["agent"]["name"])["agent"]
        snapshot = self.snapshot()
        native = self.cli(["labels", "get", self.reference["name"], "--json"])
        assert snapshot["labels"] == native["labels"] == self.initial and snapshot["revision"] == native["revision"], (snapshot, native)
        self.other = self.launch("labels-http-other", {"project": "comet", "feature": "search"})["agent"]
        self.empty = self.launch("labels-http-empty", {})["agent"]
        self.record("L01", descriptor=descriptor, initial_snapshot=snapshot, cli_snapshot=native, other=self.other, empty=self.empty)

    def patching(self):
        before = self.snapshot()
        changed, operation = self.patch(self.payload(set={"plugin/state": "pending"}))
        assert changed["changed"] and changed["snapshot"]["revision"] != before["revision"], changed
        assert changed["snapshot"]["labels"] == {**self.initial, "plugin/state": "pending"}, changed
        noop, _ = self.patch(self.payload(set={"plugin/state": "pending"}, remove=["absent"]))
        assert not noop["changed"] and noop["snapshot"] == changed["snapshot"], noop
        cli = self.cli(["labels", "set", self.reference["name"], "cli/key=值", "--json"])
        assert self.snapshot()["labels"]["plugin/state"] == "pending" and self.snapshot()["labels"]["cli/key"] == "值"
        self.cli(["labels", "remove", self.reference["name"], "cli/key", "--json"])
        assert self.snapshot() == noop["snapshot"] or (self.snapshot()["labels"] == noop["snapshot"]["labels"] and self.snapshot()["revision"] != noop["snapshot"]["revision"])
        self.record("L02", changed=operation, noop=noop, cli_patch=cli, final=self.snapshot())

    def selection(self):
        names = {self.reference["name"], self.other["name"], self.empty["name"]}
        cases = [(["project=atlas"], {self.reference["name"]}), (["project!=atlas"], names - {self.reference["name"]}),
                 (["project"], {self.reference["name"], self.other["name"]}), (["!project"], {self.empty["name"]}),
                 (["project", "project!=atlas"], {self.other["name"]}), (["feature=search", "project=atlas"], {self.reference["name"]}),
                 (["project=atlas", "project=comet"], set()), (["note=a=b,界"], {self.reference["name"]}),
                 (["note=a=b"], set()), (["project=Atlas"], set())]
        observations = []
        for selectors, expected in cases:
            page = self.page(selectors)
            found = {item["agent"]["name"] for item in page["agents"]}
            assert found == expected and page["next"] is None, (selectors, expected, page)
            args = ["list", "--json", *[arg for selector in selectors for arg in ["--label", selector]]]
            listed = self.cli(args)
            assert {item["name"] for item in listed} == expected, (selectors, listed)
            observations.append({"selectors": selectors, "names": sorted(found)})
        cursor, seen, pages = None, [], []
        for _ in range(10):
            page = self.page(["feature=search"], limit=1, after=cursor)
            pages.append(page)
            seen.extend(item["agent"]["name"] for item in page["agents"])
            if page["next"] is None:
                break
            if cursor is None:
                self.selector_cursor = page["next"]
            assert page["next"] != cursor
            cursor = page["next"]
        else:
            raise AssertionError("bounded keyset pagination did not finish")
        assert len(seen) == 2 and set(seen) == {self.reference["name"], self.other["name"]}, seen
        self.refuse("GET", "/api/v1/agents?" + urlencode({"generation": self.http.generation, "current_only": "true", "label": "!project", "after": self.selector_cursor}))
        self.refuse("GET", "/api/v1/agents", query={"generation": self.http.generation, "current_only": "true", "label": "project", "after": "1:00000000000000000001"})
        self.record("L03", observations=observations, pages=pages, no_exact_total=True)

    def refusals(self):
        path = f"/api/v1/agents/{self.reference['name']}/labels"
        before, observations = self.snapshot(), []
        invalid = [{"Bad": "x"}, {"agentx/owned": "x"}, {"a/b/c": "x"}, {"empty": ""}, {"space": " trim"},
                   {"control": "a\u0000b"}, {"long": "界" * 86}, {"number": 1}, {"array": ["x"]}, {"null": None},
                   {f"k{index}": "v" for index in range(65)}]
        for values in invalid:
            observations.append(self.refuse("POST", path, payload=self.payload(set={"partial": "must-not-stick", **values})))
            assert self.snapshot() == before
        for body in [self.payload(set={"same": "x"}, remove=["same"]), self.payload(remove=["duplicate", "duplicate"])]:
            observations.append(self.refuse("POST", path, payload=body))
        valid = json.dumps(self.payload(set={"duplicate": "one"}))
        duplicate = valid.replace('"duplicate": "one"', '"duplicate":"one","duplicate":"two"')
        observations.append(self.refuse("POST", path, raw=duplicate))
        wrong_generation = self.payload(set={"partial": "must-not-stick"})
        wrong_generation["generation"] = str(uuid.uuid4())
        observations.append(self.refuse("POST", path, payload=wrong_generation))
        stale = self.payload(set={"partial": "must-not-stick"})
        stale["agent"] = {**self.reference, "incarnation": str(int(self.reference["incarnation"]) + 1)}
        observations.append(self.refuse("POST", path, payload=stale))
        for selector in ["project=", "!", "Bad=x"]:
            self.refuse("GET", "/api/v1/agents", query={"generation": self.http.generation, "current_only": "true", "label": selector})
        query = urlencode({"generation": self.http.generation, "current_only": "true", "label": [f"k{i}=x" for i in range(17)]}, doseq=True)
        self.refuse("GET", "/api/v1/agents?" + query)
        for arguments in [["labels", "set", self.reference["name"], "repeat=one", "repeat=two"],
                          ["launch", "--tool", "qoder", "--no-profile", "--headless", "--go", "--label", "repeat=one", "--label", "repeat=two"]]:
            self.cli(arguments, valid=False)
        assert self.snapshot() == before
        # The exact byte ceiling gets its own empty Agent, so unrelated labels
        # cannot influence the boundary calculation.
        boundary = {f"k{i:02d}": "x" * 253 for i in range(64)}
        assert sum(len(k.encode()) + len(v.encode()) for k, v in boundary.items()) == 16384
        accepted, _ = self.patch(self.payload(self.empty, set=boundary))
        assert len(accepted["snapshot"]["labels"]) == 64
        self.refuse("POST", f"/api/v1/agents/{self.empty['name']}/labels", payload=self.payload(self.empty, set={"k00": "x" * 254}))
        assert self.snapshot(self.empty) == accepted["snapshot"]
        self.patch(self.payload(self.empty, remove=list(boundary)))
        self.record("L04", refusals=observations, boundary_bytes=16384, unchanged=before)

    def recovery(self):
        first, second = self.payload(set={"plugin/a": "one"}), self.payload(set={"plugin/b": "two"})
        # Independent connections avoid any client state race; each request
        # uses the same exact target but has its own native operation identity.
        def write(payload):
            return self.patch(payload)
        with ThreadPoolExecutor(max_workers=2) as executor:
            values = list(executor.map(write, [first, second]))
        snapshot = self.snapshot()
        assert snapshot["labels"]["plugin/a"] == "one" and snapshot["labels"]["plugin/b"] == "two", snapshot
        old_revision = snapshot["revision"]
        self.patch(self.payload(set={"plugin/state": "external"}))
        conflict = self.refuse("POST", f"/api/v1/agents/{self.reference['name']}/labels",
                              payload=self.payload(set={"plugin/state": "stale-overwrite"}, revision=old_revision), expected=(409,))
        assert self.snapshot()["labels"]["plugin/state"] == "external"
        payload = self.payload(set={"plugin/state": "approved"})
        outcome, original = self.patch(payload)
        path = f"/api/v1/agents/{self.reference['name']}/labels"
        assert self.http.post(path, payload) == original
        changed = copy.deepcopy(payload)
        changed["set"]["plugin/state"] = "different"
        id_conflict = self.refuse("POST", path, payload=changed, expected=(409,))
        lost = self.payload(set={"plugin/lost": "committed"})
        self.http.call("POST", path, payload=lost, disconnect=True)
        recovered = self.http.operation(lost["request_id"])
        assert self.http.post(path, lost) == recovered and self.snapshot()["labels"]["plugin/lost"] == "committed"
        self.record("L05", disjoint_operations=[value[1] for value in values], conflict=conflict,
                    original=original, changed_id_body=id_conflict, lost_request=lost, recovered=recovered)

    def lifecycle(self):
        before, old = self.snapshot(), self.reference
        forked = self.target_action("fork", old)["result"]["Launched"]
        assert forked["failed"] == 0 and len(forked["agents"]) == 1, forked
        child = self.live(forked["agents"][0])
        assert child["agent"] != old and child["labels"] == {}, child
        self.target_action("stop", old)
        stopped = wait_for("native stopped history", lambda: next((view for view in self.http.agents()
                           if view["agent"]["name"] == old["name"] and view["agent"].get("stopped_event")), None))
        # Old history may be unknown; if recorded it must be the real snapshot.
        assert stopped["labels"] in (None, before["labels"]), stopped
        resumed = self.target_action("resume", stopped["agent"])["result"]["Launched"]
        assert resumed["failed"] == 0 and len(resumed["agents"]) == 1, resumed
        successor = self.live(resumed["agents"][0])
        assert successor["agent"] != old
        self.reference = successor["agent"]
        after = self.snapshot()
        assert after["labels"] == before["labels"] and after["revision"] == before["revision"], (before, after)
        stale = self.refuse("POST", f"/api/v1/agents/{old['name']}/labels", payload=self.payload(old, set={"stale": "must-not-stick"}), expected=(409,))
        assert self.snapshot() == after
        # A second formal resume establishes multiple native aliases without
        # resetting the mutable owner state.
        self.target_action("stop", self.reference)
        stopped2 = wait_for("second stopped history", lambda: next((view for view in self.http.agents()
                           if view["agent"]["name"] == self.reference["name"] and view["agent"].get("stopped_event")
                           and view["agent"] != stopped["agent"]), None))
        resumed2 = self.target_action("resume", stopped2["agent"])["result"]["Launched"]
        self.reference = self.live(resumed2["agents"][0])["agent"]
        final = self.snapshot()
        assert final["labels"] == before["labels"] and final["revision"] == before["revision"]
        self.record("L06", fork=child["agent"], stopped=stopped, successor=successor["agent"], stale=stale,
                    second_resume=self.reference, snapshot=final,
                    limits="Arbitrary fresh same-name creation is a provider-native test seam, not exposed by public launch; consume its independent concentrated native result.")

    def browser(self):
        config = {"url": self.http.url, "generation": self.http.generation,
                  "project": str(self.fixture.root / "primary/project"), "evidence": str(self.fixture.evidence / "browser"),
                  "agent": self.reference, "other": self.other, "empty": self.empty}
        path = self.fixture.evidence / "browser-config.json"
        path.write_text(json.dumps(config, indent=2) + "\n")
        result = subprocess.run(["node", str(Path(__file__).with_name("browser.mjs")), str(path)],
                                capture_output=True, text=True, timeout=300)
        (self.fixture.evidence / "browser-driver.json").write_text(json.dumps({"code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}, indent=2) + "\n")
        assert result.returncode == 0, result.stderr
        self.record("browser", report=str(self.fixture.evidence / "browser/report.json"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--browser", action="store_true")
    parser.add_argument("--browser-only", action="store_true")
    parser.add_argument("--assets", type=Path, help="Exact retained Cargo management-web output to compare with actual HTTP bytes")
    args = parser.parse_args()
    fixture = LabelsFixture(args.candidate, args.evidence)
    fixture.assets = args.assets
    source_root = Path(__file__).resolve().parents[2]
    harnesses = [Path(__file__).resolve(), Path(__file__).with_name("browser.mjs").resolve(),
                 MANAGEMENT_FIXTURES / "fixture.py", MANAGEMENT_FIXTURES / "provider.py",
                 source_root / "tests/collaboration_workbench/browser_checkpoint.mjs"]
    harness_digests = {}
    for source in harnesses:
        relative = source.relative_to(source_root)
        target = fixture.evidence / "harness-inputs" / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(source.read_bytes())
        harness_digests[str(relative)] = hashlib.sha256(target.read_bytes()).hexdigest()
    fixture.manifest["harness_inputs"] = harness_digests
    fixture.persist()
    cases, failure = Cases(fixture), None
    started = time.time()
    try:
        methods = ["creation"] if args.browser_only else ["creation", "patching", "selection", "refusals", "recovery", "lifecycle"]
        if args.browser or args.browser_only:
            methods.append("browser")
        for name in methods:
            cases.stage = name
            getattr(cases, name)()
    except Exception as error:
        failure = {"stage": cases.stage, "message": str(error), "traceback": traceback.format_exc()}
        print(json.dumps(failure), flush=True)
    finally:
        try:
            fixture.close()
        except Exception as error:
            failure = {"prior_failure": failure, "cleanup_error": str(error), "traceback": traceback.format_exc()}
        report = {"status": "fail" if failure else "pass", "started": started, "finished": time.time(), "failure": failure,
                  "cases": cases.results, "candidate": fixture.manifest,
                  "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  "limits": "Finite Linux local public CLI/HTTP and optional Chromium replay. Fixture Provider, no paid model/user participants/live adoption. Native migration/query-plan/same-name cases require separately linked native results."}
        (fixture.evidence / "report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
        (fixture.evidence / "http-timings.json").write_text(json.dumps(fixture.timings, indent=2) + "\n")
    return int(failure is not None)


if __name__ == "__main__":
    raise SystemExit(main())
