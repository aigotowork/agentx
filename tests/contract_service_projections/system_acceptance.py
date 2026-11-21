#!/usr/bin/env python3
"""One retained scope: installed public package, real listeners and finite actors."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import traceback
import uuid

from package_acceptance import PublicCases
from native_http import Fixture, run as native_run, wait_for
from provider import process_identity


def encode(value):
    if isinstance(value, bool) or value is None or isinstance(value, str):
        return value
    if isinstance(value, int):
        return {"$integer": str(value)}
    if isinstance(value, list):
        return [encode(item) for item in value]
    if isinstance(value, dict):
        return {key: encode(item) for key, item in value.items()}
    raise TypeError(type(value))


class SystemCases(PublicCases):
    def __init__(self, fixture, project):
        super().__init__(fixture, project)
        self.urls = {}

    def domain(self, domain, action, *arguments):
        envelopes = []
        for component, value in arguments:
            envelopes.append({"component": component, "wireText": json.dumps(encode(value))} if component else value)
        result = self.sdk(action, url=self.urls[domain], domain=domain, arguments=envelopes)
        assert result["ok"], result
        return result["value"]

    def management(self, action, *arguments):
        result = self.sdk(action, arguments=list(arguments))
        assert result["ok"], result
        return result["value"]

    def listener(self, command, domain):
        f = self.fixture
        stdout = f.evidence / (domain + "-listener.stdout")
        stderr = f.evidence / (domain + "-listener.stderr")
        with stdout.open("x") as out, stderr.open("x") as err:
            process = subprocess.Popen([str(f.candidate), *command], cwd=f.root / "primary/project", env=self.env,
                                       stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
        record = process_identity(process.pid)
        f.processes.append(record)
        f.manifest["processes"].append(record)
        f.persist()

        def ready():
            assert process.poll() is None, stderr.read_text()
            return next((line.strip() for line in stdout.read_text().splitlines() if line.startswith("http://")), None)

        self.urls[domain] = wait_for(domain + " listener", ready)
        return self.urls[domain]

    def passive(self):
        f = self.fixture
        scope = f.root / "primary/scope"
        databases = {str(path): path.stat().st_mtime_ns for path in scope.glob("*.db")}
        self.listener(["observe-web", "--scope", str(scope), "--host", "127.0.0.1", "--port", "0"], "observation")
        result = self.domain("observation", "runtime.snapshot")
        assert "LocalFailure" in result, result
        assert not (scope / ".daemon/management.json").exists()
        assert {str(path): path.stat().st_mtime_ns for path in scope.glob("*.db")} == databases
        self.record("CS-09-passive-missing-daemon", response=result)

    def management_flow(self, ref):
        f = self.fixture
        http = f.clients["primary"]
        g = http.generation
        exact = {"generation": g, "agent": ref}
        before = self.management("listAgents", {"generation": g})
        directory = self.sdk("directory.inspect", arguments=[{"generation": g, "directory": str(f.root / "primary/project")}])
        # This private project deliberately has no default role. Preserve its
        # native refusal while still checking that preview creates no Agent.
        assert directory["ok"] or directory["error"]["code"] == "InvalidInput", directory
        assert self.management("listAgents", {"generation": g}) == before
        catalog = self.management("workspaces.list", g)
        self.record("CS-07-directory-preview-no-launch", directory=directory, catalog=catalog)
        labels = self.management("agents.labels.get", exact)
        patch = {**exact, "request_id": str(uuid.uuid4()), "set": {"acceptance": "system"}, "remove": [], "if_revision": labels["revision"]}
        self.management("agents.labels.patch", patch)
        operation = self.management("waitForOperation", patch, {"timeoutMs": 15000})
        assert operation["status"] == "Succeeded", operation
        changed = self.management("agents.labels.get", exact)
        assert changed["labels"]["acceptance"] == "system" and changed["revision"] != labels["revision"]
        stale = self.sdk("agents.labels.patch", {**patch, "request_id": str(uuid.uuid4()), "set": {"acceptance": "wrong"}})
        assert not stale["ok"], stale
        self.record("CS-07-label-revision", before=labels, after=changed, refusal=stale)
        binary = f.root / "attachment.bin"
        binary.write_bytes(bytes(range(256)) + b"\x00acceptance\xff")
        uploaded = self.sdk("attachments.upload", {"generation": g, "name": "acceptance.bin", "media_type": "application/octet-stream"}, binaryUploadFile=str(binary))
        assert uploaded["ok"], uploaded
        attachment = uploaded["value"]
        downloaded = self.management("attachments.download", {"generation": g, "id": attachment["id"]})
        assert bytes(downloaded["bytes"]) == binary.read_bytes()
        message = self.message(ref, "binary attachment through public SDK")
        message["message"]["attachments"] = [attachment["id"]]
        self.management("send", message)
        sent = self.management("waitForSent", message, {"timeoutMs": 15000, "pollIntervalMs": 30})
        self.effect(ref, sent["message_id"])
        mailbox = self.management("humanMailbox.list", {"generation": g, "pending": True})
        assert mailbox["messages"], mailbox
        ids = [mailbox["messages"][0]["id"]]
        handled = self.management("humanMailbox.handle", {"generation": g, "ids": ids})
        self.record("CS-06-07-binary-mailbox", attachment=attachment, bytes_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), handled=handled)
        inspection = self.management("inspection.get", g)
        config = {"generation": g, "expected_revision": inspection["settings"]["revision"], "enabled": True,
                  "recovery": {"enabled": False, "error_duration_seconds": 600}}
        enabled = self.management("inspection.configure", config)
        stale = self.sdk("inspection.configure", config)
        assert not stale["ok"], stale

        def observed():
            snapshot = self.management("inspection.get", g)
            return snapshot if snapshot["observations"] else None

        snapshot = wait_for("actual Inspector collection", observed, timeout=35)
        analysis = {"generation": g, "request_id": "system-analysis", "observation_ids": [snapshot["observations"][0]["id"]]}
        job = self.management("inspection.analyze", analysis)
        assert self.management("inspection.analyze", analysis) == job
        canceled = self.management("inspection.cancelAnalysis", {"generation": g, "id": job["id"]})
        disabled = self.management("inspection.configure", {**config, "expected_revision": enabled["settings"]["revision"], "enabled": False})
        assert disabled["settings"]["enabled"] is False
        self.record("CS-07-inspector-real-collection-analysis", snapshot=snapshot, job=job, cancellation=canceled, disabled=disabled, stale=stale)

    def work_flow(self, implementer):
        f = self.fixture
        g = f.clients["primary"].generation
        scope, project = f.root / "primary/scope", f.root / "primary/project"
        (project / "capability.txt").write_bytes(b"independent fixture capability\n")
        launch = {"generation": g, "request_id": str(uuid.uuid4()), "launch": {"tool": "qoder", "profile": None, "directory": None,
                  "prompt": "independent finite checker", "title": "contract-checker"}}
        self.management("agents.launch", launch)
        receipt = self.management("waitForOperation", launch, {"timeoutMs": 15000})
        name = receipt["result"]["Launched"]["agents"][0]
        checker = wait_for("finite checker", lambda: next((view["agent"] for view in self.management("listAgents", {"generation": g})["agents"]
                            if view["agent"]["name"] == name and view["can_send"] and view["session_id"]), None))
        self.listener(["workbench", "--scope", str(scope), "--project", str(project), "--collaboration", "--host", "127.0.0.1", "--port", "0"], "workbench")
        descriptor = self.domain("workbench", "describe")
        assert descriptor["generation"] == g
        participant = lambda ref: {"canonical_name": ref["name"], "incarnation_bits": int(ref["incarnation"])}
        brief = {"title": "Finite real capability", "goal": "Read and independently verify exact fixture bytes", "source": None,
                 "criteria": [{"id": "bytes", "text": "Exact bytes and source SHA256", "required": True}]}
        call = {"generation": g, "command_id": str(uuid.uuid4()), "work": None, "expected_revision": None,
                "command": {"CreateWork": {"brief": brief, "implementer": participant(implementer), "checker": participant(checker)}}}
        created = self.domain("workbench", "apply", ("ApplyCall", call))
        reference = created["Applied"]["reference"]

        def detail():
            response = self.domain("workbench", "read", ("ReadCall", {"generation": g, "read": {"Detail": reference}}))
            return response["Read"]["Detail"]

        def apply(command, revision=None):
            call = {"generation": g, "command_id": str(uuid.uuid4()), "work": reference,
                    "expected_revision": detail()["work"]["revision"] if revision is None else revision, "command": command}
            return call, self.domain("workbench", "apply", ("ApplyCall", call))

        _, dispatched = apply({"DispatchAssignment": {"role": "Implementer"}})
        assert "Applied" in dispatched, dispatched
        _, conflict = apply({"UpdateBrief": brief}, revision=1)
        assert "RevisionConflict" in conflict["Rejected"], conflict

        def actor(ref, action):
            directory = f.root / "actor-jobs" / ref["name"]
            directory.mkdir(parents=True, exist_ok=True)
            command_id = str(uuid.uuid4())
            path = directory / (command_id + ".request.json")
            path.write_text(json.dumps({"work": reference["id"], "action": action, "command_id": command_id}))
            output = path.with_name(command_id + ".response.json")
            result = wait_for("native attributed " + action, lambda: json.loads(output.read_text()) if output.exists() else None)
            assert "failure" not in result and result["applied"]["exit"] == 0, result
            assert "Applied" in json.loads(result["applied"]["stdout"]), result
            return command_id, result

        result_id, result = actor(implementer, "result")
        _, requested = apply({"RequestCheck": {"result_id": result_id, "reason": "Independent exact-byte check"}})
        assert "Applied" in requested, requested
        check_id, check = actor(checker, "check")
        eligible = detail()
        assert eligible["eligibility"]["selection"]["check_id"] == check_id, eligible
        accepted_call, accepted = apply({"AcceptResult": eligible["eligibility"]["selection"]})
        assert accepted["Applied"]["effect"] == "Accepted", accepted
        replay = self.domain("workbench", "apply", ("ApplyCall", accepted_call))
        assert replay["Applied"]["replayed"] is True, replay
        _, closed = apply({"SendNote": {"roles": ["Implementer"], "text": "must refuse closed work"}})
        assert closed["Rejected"] == "Closed", closed
        self.record("CS-08-actual-native-attributed-acceptance", result=result, check=check, accepted=accepted, replay=replay, closed=closed,
                    boundary="public installed package→HTTP/native; authentic finite actor subprocess reports concrete bytes, no model judgment")

    def observation_flow(self):
        initial = self.domain("observation", "runtime.snapshot")
        daemon = initial["Inspection"]["ready"]["daemon"]
        head = self.domain("observation", "events.describe")
        config = self.domain("observation", "configuration.head", (None, {"expected": None}))
        cli = self.execute("observation-cli", [self.node, self.bin, "observe", "snapshot", "--observation-url", self.urls["observation"]])
        assert cli.returncode == 0 and "Inspection" in json.loads(cli.stdout), cli
        event_head = head["Inspection"]["response"]["Events"]["Ok"]["Head"]
        config_head = config["Inspection"]["response"]["Configuration"]["Ok"]["Head"]
        schema = json.loads((Path(__file__).resolve().parents[2] / ".docs/api-panorama/openapi/observation.json").read_text())
        filter_value = {key: None for key in schema["components"]["schemas"]["ObservationFilter"]["properties"]}
        event_input = {"window": {"after": {"stream": event_head["stream"], "sequence": 0}, "through": event_head["newest"], "filter": filter_value}, "limit": 8}
        gap = self.domain("observation", "events.read", ("ObservationRead", event_input))
        if event_head["oldest"] > 1:
            assert "Gap" in gap["Inspection"]["response"]["Events"]["Ok"]["Read"], gap
            self.record("CS-09-actual-retention-gap", response=gap)
        for action in ["runtime.connections", "runtime.peers", "runtime.hosts", "runtime.clientReports.list"]:
            response = self.domain("observation", action, ("RuntimeList", {"before": None, "limit": 8}))
            assert "Inspection" in response, response
        self.fixture.stop_daemon("primary")
        missing = self.domain("observation", "runtime.snapshot")
        assert "LocalFailure" in missing
        self.fixture.start("primary")
        changed = self.domain("observation", "runtime.snapshot", (None, {"expectedDaemon": daemon}))
        assert "DaemonChanged" in changed["Inspection"]["response"], changed
        epoch = self.domain("observation", "events.read", ("ObservationRead", event_input))
        assert "EpochChanged" in epoch["Inspection"]["response"]["Events"]["Ok"]["Read"], epoch
        current = self.domain("observation", "configuration.head", (None, {"expected": None}))["Inspection"]["response"]["Configuration"]["Ok"]["Head"]
        comparison = self.domain("observation", "configuration.compare", ("CompareConfig", {
            "left": {"epoch": config_head["epoch"], "sequence": max(1, config_head["last_issued"])},
            "right": {"epoch": current["epoch"], "sequence": max(1, current["last_issued"])}, "fields": "All"}))
        assert "Incomparable" in json.dumps(comparison) or "EpochChanged" in json.dumps(comparison), comparison
        self.record("CS-09-owned-restart-preserves-era", initial=initial, events=head, configuration=config, stopped=missing, changed=changed,
                    old_event_window=epoch, comparison=comparison, actual_gap_probe=gap)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ["candidate", "spec", "project", "evidence"]:
        parser.add_argument("--" + field, type=Path, required=True)
    parser.add_argument("--resume", type=Path, help="previous cleaned receipt; retain its exact existing scope")
    parser.add_argument("--from-stage", choices=["management", "work", "pilot", "observation"], default="management")
    args = parser.parse_args()
    f = Fixture.reopen_owned(args.candidate, args.evidence, args.resume) if args.resume else Fixture(args.candidate, args.evidence, root=args.project.parent / "agentx-management-system")
    result = {"status": "failed", "boundary": "actual installed public package and native listeners; finite local Provider only"}
    try:
        shutil.copyfile(Path(__file__).with_name("actor.py"), f.root / "bin/contract_actor.py")
        if not args.resume:
            f.configure("primary")
        cases = SystemCases(f, args.project)
        cases.passive()
        f.start("primary")
        if args.resume:
            http = f.clients["primary"]
            launch = {"request_id": str(uuid.uuid4()), "generation": http.generation,
                      "launch": {"tool": "qoder", "profile": None, "directory": None, "prompt": "resumed finite integration actor", "title": "contract-system"}}
            http.post("/api/v1/agents", launch)
            name = http.operation(launch["request_id"])["result"]["Launched"]["agents"][0]
            ref = wait_for("resumed actor", lambda: next((view["agent"] for view in http.agents() if view["agent"]["name"] == name and view["can_send"] and view["session_id"]), None))
        else:
            ref, launch = native_run(f, args.spec)
        if args.from_stage == "management":
            cases.management_flow(ref)
        if args.from_stage in ["management", "work"]:
            cases.work_flow(ref)
        if args.from_stage in ["management", "work", "pilot"]:
            view, original = cases.actual(ref, launch)
            cases.finite(view, original)
        cases.observation_flow()
        result["status"] = "passed"
    except Exception:
        result["failure"] = traceback.format_exc()
        raise
    finally:
        try:
            f.close()
        except Exception:
            result["cleanup_failure"] = traceback.format_exc()
            raise
        finally:
            (f.evidence / "system-result.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
