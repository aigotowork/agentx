#!/usr/bin/python3
"""Public-only finite acceptance of one immutable composed candidate; never builds."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import traceback
import uuid

from live_fixture import LiveFixture, wait_for


def identity():
    return str(uuid.uuid4())


def exact(reference):
    return {key: value for key, value in reference.items() if key != "name" and value is not None}


class Cases:
    def __init__(self, fixture, setup_only=False):
        self.fixture, self.steps = fixture, []
        self.setup_only = setup_only
        self.http = fixture.prepare("primary")
        self.paths = fixture.layout("primary")

    def record(self, case, **facts):
        self.steps.append({"case": case, "status": "setup" if self.setup_only else "pass", **facts})
        (self.fixture.evidence / "cases.json").write_text(json.dumps(self.steps, indent=2) + "\n")
        print(case + (" fixture prepared" if self.setup_only else " passed"), flush=True)

    def reject(self, method, path, codes=(400, 409), **arguments):
        status, body = self.http.call(method, path, **arguments)
        assert status in codes and isinstance(body.get("error", {}).get("code"), str), (status, body)
        return body

    def snapshot(self):
        return self.http.get("/api/v1/inspection")

    def inspect_post(self, route, variant, **fields):
        status, result = self.http.call("POST", "/api/v1/inspection/" + route,
                                        {variant: {"generation": self.http.generation, **fields}})
        assert status == 200, (route, status, result)
        return result

    def configure(self, snapshot, enabled):
        return self.inspect_post("configure", "Configure", expected_revision=snapshot["settings"]["revision"],
                                 enabled=enabled, recovery={"enabled": False, "error_duration_seconds": 600})

    def startup(self):
        embedded = {}
        assert not self.http.agents()
        for prefix in ["space", "inspector"]:
            status, html = self.http.call("GET", f"/{prefix}/")
            assert status == 200 and isinstance(html, str) and "<html" in html, (prefix, status, html)
            assets = re.findall(r'(?:src|href)=["\'](/[^"\']+\.(?:js|css))["\']', html)
            assert assets and all(path.startswith(f"/{prefix}/") for path in assets), assets
            hashes = {}
            for asset in assets:
                status, content = self.http.call("GET", asset)
                assert status == 200 and isinstance(content, str) and content, (asset, status)
                hashes[asset] = hashlib.sha256(content.encode()).hexdigest()
            embedded[prefix] = hashes
        snapshot = self.snapshot()
        assert snapshot["generation"] == self.http.generation
        assert snapshot["settings"]["enabled"] is False and snapshot["settings"]["collected_from"] is None
        assert snapshot["observations"] == [] and snapshot["jobs"] == [] and snapshot["attempts"] == []
        assert snapshot["settings"]["recovery"] == {"enabled": False, "error_duration_seconds": 600}
        self.record("L01-api", embedded=embedded, initial=snapshot)

    def launch(self):
        payload = {"generation": self.http.generation, "request_id": identity(),
                   "launch": {"role": "reviewer", "workspace": str(self.paths["child"]),
                              "directory": str(self.paths["cwd"]), "prompt": "live acceptance initial fixture",
                              "tool": None, "profile": None, "title": "live-api-collection"}}
        self.http.post("/api/v1/agents", payload)
        launched = self.http.operation(payload["request_id"])
        result = launched["result"]["Launched"]
        assert result["failed"] == 0 and len(result["agents"]) == 1, launched
        name = result["agents"][0]
        self.view = wait_for("real hook session", lambda: next((view for view in self.http.agents()
            if view["agent"]["name"] == name and view["session_id"] and view["can_send"]), None))
        self.reference = self.view["agent"]
        assert self.view["collaboration"]["workspace"] == str(self.paths["child"])
        assert self.view["directory"] == str(self.paths["cwd"])
        self.launch_receipt = launched
        self.record("L03-api", launch=launched, view=self.view,
                    workspaces=self.http.get("/api/v1/workspaces"),
                    child_roles=self.http.get("/api/v1/roles", workspace=str(self.paths["child"])))

    def observed(self, predicate, description):
        def read():
            snapshot = self.snapshot()
            return snapshot if predicate(snapshot) else None
        return wait_for(description, read, timeout=40)

    def collection(self):
        before = self.snapshot()
        assert before["observations"] == [], "pre-enable backfill"
        start = int(time.time())
        enabled = self.configure(before, True)
        assert start <= enabled["settings"]["collected_from"] <= int(time.time())
        assert enabled["settings"]["revision"] > before["settings"]["revision"]
        sampled = self.observed(lambda value: any(row["agent"] == self.reference for row in value["observations"]), "collector native sample")
        rows = [row for row in sampled["observations"] if row["agent"] == self.reference]
        assert all(row["at"] >= enabled["settings"]["collected_from"] for row in sampled["observations"])
        assert all(row.get("sample") is None or row["sample"].get("usage") is None for row in rows), "missing tokens became known zero"
        assert sampled["coverage"]["basis"] and sampled["coverage"]["observed_at"] is not None
        conflict = self.reject("POST", "/api/v1/inspection/configure", payload={"Configure": {
            "generation": self.http.generation, "expected_revision": before["settings"]["revision"],
            "enabled": False, "recovery": before["settings"]["recovery"]}})
        assert self.snapshot()["settings"] == sampled["settings"], "CAS conflict changed settings"
        disabled = self.configure(sampled, False)
        assert not disabled["settings"]["enabled"] and not disabled["settings"]["recovery"]["enabled"]
        frozen_ids = [row["id"] for row in disabled["observations"]]
        generation = self.http.generation
        self.fixture.stop_daemon("primary")
        self.http = self.fixture.start("primary")
        retained = self.snapshot()
        assert self.http.generation == generation and retained["settings"] == disabled["settings"]
        assert [row["id"] for row in retained["observations"]] == frozen_ids
        assert retained["coverage"]["gaps"], "disabled collection gap missing"
        # The host's first tick runs even when disabled. Its native observation time
        # cannot advance; retained data is not a newly manufactured sample.
        assert retained["coverage"]["observed_at"] == disabled["coverage"]["observed_at"]
        reenabling = self.configure(retained, True)
        assert reenabling["settings"]["revision"] > disabled["settings"]["revision"]
        assert reenabling["settings"]["collected_from"] == enabled["settings"]["collected_from"]
        self.record("I01-I03", enabled=enabled, sampled=sampled, cas_conflict=conflict,
                    disabled=disabled, restarted=retained, reenabled=reenabling)

    def usage(self):
        content = self.append_usage()
        def known(value):
            return any(row["agent"] == self.reference and row.get("sample") and row["sample"].get("usage")
                       for row in value["observations"])
        first = self.observed(known, "native parser usage checkpoint")
        measured = [row for row in first["observations"] if row["agent"] == self.reference and row.get("sample") and row["sample"].get("usage")][-1]
        expected = measured["sample"]["usage"]
        assert {key: expected[key] for key in ["input", "output", "cache_read", "cache_write", "reasoning"]} == {
            "input": "101", "output": "17", "cache_read": "23", "cache_write": "5", "reasoning": "7"}, expected
        assert expected["unit"] == "tokens" and "parser_normalized" in expected["basis"]
        source = measured["sample"]["source_key"]
        second = self.observed(lambda value: any(row["id"] != measured["id"] and row["at"] > measured["at"]
            and row["agent"] == self.reference and row.get("sample") and row["sample"].get("usage") for row in value["observations"]), "second cumulative sample")
        current = [row for row in second["observations"] if row["agent"] == self.reference and row.get("sample") and row["sample"].get("usage")][-1]
        assert current["sample"]["source_key"] == source and current["sample"]["usage"] == expected, "cumulative samples summed twice"
        self.measured_source = source
        self.record("I02-api", fixture_content=content, first=measured, second=current)

    def append_usage(self):
        # Fixture-owned Provider content only; daemon storage remains inaccessible.
        started = next(row for row in self.fixture.provider_records()
                       if row["kind"] == "started" and row["name"] == self.reference["name"])
        transcript = Path(started["transcript"]).resolve(strict=True)
        assert transcript.is_relative_to(self.fixture.root) and started["scope"] == str(self.fixture.root / "primary/scope")
        content = {"type": "assistant", "uuid": identity(), "timestamp": "2026-11-15T00:00:00Z",
                   "sessionId": started["session"], "message": {"id": "acceptance-usage-source", "role": "assistant",
                   "model": "fixture-model-no-provider", "content": [{"type": "text", "text": "fixture measured checkpoint"}],
                   "usage": {"input_tokens": 101, "output_tokens": 17, "cache_read_input_tokens": 23,
                             "cache_creation_input_tokens": 5, "output_tokens_details": {"thinking_tokens": 7}}}}
        with transcript.open("a") as stream:
            stream.write(json.dumps(content) + "\n")
        return content

    def analysis(self):
        snapshot = self.snapshot()
        inputs = [row["id"] for row in snapshot["observations"]][-3:]
        assert inputs
        request = identity()
        job = self.inspect_post("analyze", "Analyze", request_id=request, observation_ids=inputs)
        assert job["observation_ids"] == inputs and job["status"] in ["queued", "succeeded"]
        pinned = {row["id"]: row for row in snapshot["observations"] if row["id"] in inputs}
        replay = self.inspect_post("analyze", "Analyze", request_id=request, observation_ids=inputs)
        assert replay["id"] == job["id"] and replay["input_revision"] == job["input_revision"]
        conflict = self.reject("POST", "/api/v1/inspection/analyze", payload={"Analyze": {
            "generation": self.http.generation, "request_id": request, "observation_ids": inputs[:1] if len(inputs) > 1 else inputs + ["other-input"]}})
        generation = self.http.generation
        self.fixture.stop_daemon("primary")
        self.http = self.fixture.start("primary")
        assert self.http.generation == generation
        completed = self.observed(lambda value: any(row["id"] == job["id"] and row["status"] == "succeeded" for row in value["jobs"]), "durable deterministic report")
        result = next(row for row in completed["jobs"] if row["id"] == job["id"])
        report = result["report"]
        assert report["algorithm"] == "native-evidence-v1" and report["observation_ids"] == inputs
        assert report["observation_count"] == len(inputs) and report["caveats"]
        assert report["distinct_agents"] == len({json.dumps(row["agent"], sort_keys=True) for row in pinned.values()})
        assert report["unknown_usage_count"] == sum(not (row.get("sample") and row["sample"].get("usage")) for row in pinned.values())
        assert report["usage_sources"] == len({row["sample"]["source_key"] for row in pinned.values() if row.get("sample") and row["sample"].get("usage")})
        retained = self.inspect_post("analyze", "Analyze", request_id=request, observation_ids=inputs)
        assert retained == result
        self.record("I04-api", queued=job, replay=replay, conflict=conflict, after_restart=result,
                    queued_restart_window_observed=job["status"] == "queued")

    def resume_usage(self):
        original = self.reference
        stop = {"generation": self.http.generation, "request_id": identity(), "agent": original}
        self.http.post(f"/api/v1/agents/{original['name']}/stop", stop)
        self.http.operation(stop["request_id"])
        history = wait_for("usage stopped history", lambda: next((row for row in self.http.agents()
            if row["agent"]["name"] == original["name"] and row["agent"]["stopped_event"]), None))
        request = {"generation": self.http.generation, "request_id": identity(), "agent": history["agent"]}
        self.http.post(f"/api/v1/agents/{original['name']}/resume", request)
        resumed = self.http.operation(request["request_id"])
        name = resumed["result"]["Launched"]["agents"][0]
        self.view = wait_for("usage successor ready", lambda: next((row for row in self.http.agents()
            if row["agent"]["name"] == name and row["agent"]["incarnation"] and row["can_send"] and row["session_id"]), None))
        self.reference = self.view["agent"]
        assert self.reference != original and self.view["session_id"] == history["session_id"]
        supplied = self.observed(lambda value: any(row["agent"] == self.reference and row.get("sample")
            and row["sample"].get("usage") for row in value["observations"]), "resumed source checkpoint")
        checkpoint = [row for row in supplied["observations"] if row["agent"] == self.reference and row.get("sample") and row["sample"].get("usage")][-1]
        assert checkpoint["sample"]["source_key"] == self.measured_source, "resume invented another source identity"
        assert checkpoint["sample"]["usage"]["input"] == "101" and checkpoint["sample"]["usage"]["output"] == "17", "resumed cumulative checkpoint doubled"
        self.record("I02-L07-resumed-source", original=original, history=history, resumed=resumed, checkpoint=checkpoint)

    def boundary(self):
        snapshot = self.snapshot()
        # Public routes must not become privileged collector/run controls.
        for route, variant, fields in [
            ("collect", "Collect", {"expected_revision": snapshot["settings"]["revision"], "samples": [], "discovery_complete": True}),
            ("run-analysis", "RunAnalysis", {"id": identity()}),
            ("configure", "Collect", {"expected_revision": snapshot["settings"]["revision"], "samples": [], "discovery_complete": True})]:
            self.reject("POST", "/api/v1/inspection/" + route, codes=(400, 404),
                        payload={variant: {"generation": self.http.generation, **fields}})
        other = self.fixture.prepare("other")
        assert other.generation != self.http.generation and not other.agents()
        status, result = other.call("GET", "/api/v1/operations/" + self.launch_receipt["request_id"], query={"generation": other.generation})
        assert status == 404, result
        self.reject("GET", "/api/v1/inspection", codes=(409,), query={"generation": other.generation})
        self.reject("POST", "/api/v1/inspection/configure", codes=(409,), payload={"Configure": {
            "generation": other.generation, "expected_revision": snapshot["settings"]["revision"],
            "enabled": False, "recovery": snapshot["settings"]["recovery"]}})
        assert self.snapshot()["settings"] == snapshot["settings"]
        self.fixture.stop_daemon("other")
        self.record("L08-api-boundary", other_generation=other.generation)

    def browser(self, start=None):
        config = {"url": self.http.url, "generation": self.http.generation,
                  "project": str(self.paths["project"]), "scope": str(self.fixture.root / "primary/scope"),
                  "paths": {key: str(path) for key, path in self.paths.items()},
                  "reference": self.reference, "recovery": self.recovery_fixture,
                  "candidate": str(self.fixture.candidate), "environment": self.fixture.environment("primary"),
                  "evidence": str(self.fixture.evidence / "browser")}
        if start:
            config["start"] = start
        config_path = self.fixture.evidence / "browser-config.json"
        config_path.write_text(json.dumps(config, indent=2) + "\n")
        with (self.fixture.evidence / "browser.stdout").open("x") as stdout, (self.fixture.evidence / "browser.stderr").open("x") as stderr:
            result = subprocess.run(["node", str(Path(__file__).with_name("browser.mjs")), str(config_path)],
                                    env=os.environ.copy(), stdout=stdout, stderr=stderr, timeout=420)
        assert result.returncode == 0, "browser failed; see retained browser/report.json and stderr"
        self.record("L-browser", report="browser/report.json")

    def recovery_setup(self):
        payload = {"generation": self.http.generation, "request_id": identity(), "launch": {
            "tool": "codex", "directory": str(self.paths["project"]), "prompt": "local fixture only", "title": "live-hook-error"}}
        self.http.post("/api/v1/agents", payload)
        receipt = self.http.operation(payload["request_id"])
        result = receipt["result"]["Launched"]
        assert result["failed"] == 0 and len(result["agents"]) == 1, receipt
        name = result["agents"][0]
        def ready():
            return next((row for row in self.fixture.provider_records() if row["kind"] == "codex-started" and row["name"] == name), None)
        provider = wait_for("local Codex generated hooks", ready)
        control = Path(provider["control"])
        assert control.is_relative_to(self.fixture.root / "providers")
        command = {"phase": "failed", "turn": "live-acceptance-error-turn"}
        control.write_text(json.dumps(command) + "\n")
        error = self.observed(lambda value: any(row["agent"]["name"] == name and row.get("model_request")
            and row["model_request"].get("error") and row["status"] == "error" for row in value["observations"]), "native failed Hook observation")
        current = [row for row in error["observations"] if row["agent"]["name"] == name and row.get("model_request") and row["model_request"].get("error")][-1]
        self.recovery_fixture = {"name": name, "agent": current["agent"], "session": provider["session"],
                                 "control": str(control), "turn": command["turn"]}
        assert not any(attempt["episode"]["agent"] == current["agent"] for attempt in error["attempts"]), "recovery default was enabled"
        self.record("I05-hook-setup", receipt=receipt, failed_observation=current)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--api-only", action="store_true", help="Explicit partial execution; never full acceptance")
    parser.add_argument("--browser-only", action="store_true", help="Fresh browser setup; link an unchanged candidate's retained passed API run")
    parser.add_argument("--browser-start", choices=["inspector-message-transcript", "inspector-lost-admitted-response", "stop-history-resume"], help="With --browser-only, continue after retained passed checkpoints")
    arguments = parser.parse_args()
    assert not (arguments.api_only and arguments.browser_only)
    assert not arguments.browser_start or arguments.browser_only
    assert arguments.candidate.is_absolute() and not arguments.candidate.is_symlink(), "frozen absolute candidate file required"
    candidate = arguments.candidate.resolve(strict=True)
    assert re.fullmatch(r"[0-9a-f]{64}", arguments.sha256), "explicit SHA-256 required"
    assert hashlib.sha256(candidate.read_bytes()).hexdigest() == arguments.sha256, "candidate handoff hash mismatch"
    fixture = LiveFixture(candidate, arguments.evidence)
    failure = None
    stage = "startup"
    try:
        cases = Cases(fixture, setup_only=arguments.browser_only)
        if arguments.browser_only:
            stage = "browser-fixture-setup"
            cases.launch()
            cases.append_usage()
            cases.configure(cases.snapshot(), True)
            cases.recovery_setup()
        else:
            for stage in ["startup", "launch", "collection", "usage", "analysis", "resume_usage", "boundary", "recovery_setup"]:
                getattr(cases, stage)()
        if not arguments.api_only:
            stage = "browser"
            cases.browser(start=arguments.browser_start)
    except Exception as error:
        failure = {"stage": stage, "message": str(error), "traceback": traceback.format_exc()}
        print(failure["traceback"], file=sys.stderr)
    finally:
        try:
            fixture.close()
        except Exception as error:
            failure = {"prior": failure, "stage": "cleanup", "message": str(error), "traceback": traceback.format_exc()}
        report = {"status": "fail" if failure else "partial" if arguments.api_only else "pass",
                  "execution": "browser-only; API setup is prerequisite, link prior API evidence" if arguments.browser_only else "full",
                  "browser_start": arguments.browser_start,
                  "failure": failure, "candidate": str(candidate), "sha256": arguments.sha256,
                  "cases": getattr(locals().get("cases"), "steps", []),
                  "limits": ["Local fixture Providers, no paid model interoperability", "Native deterministic storage/episode cases require integration-owned execution", "Codex trust discovery uses a finite local hooks/list responder, no real app-server or model"]}
        (fixture.evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    return 1 if failure else 0


if __name__ == "__main__":
    sys.exit(main())
