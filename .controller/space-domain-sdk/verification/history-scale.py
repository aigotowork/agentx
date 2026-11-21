#!/usr/bin/env python3
"""Exercise sparse exact Management message history on a disposable AgentX daemon."""

import argparse
from contextlib import closing
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import struct
import sys
import time
import traceback
import uuid

ROOT = Path(__file__).resolve().parents[3]
ARTIFACTS = ROOT / ".artifacts/space-domain-sdk"
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tests/live_ui_integration"))
from live_fixture import LiveFixture, wait_for

SCOPE = "primary"
LEGACY = ROOT / ".bin/agentx-space-domain-sdk-r1"
LEGACY_SHA = "fc0d6a97d389ce72d8a66b7cd83c8ab58ed54db025e1f505e16c8e3a50e40511"
JOURNAL_INSERT = "INSERT INTO events(id,timestamp,type,instance,data) VALUES (?,?,'message',?,?)"
COVERAGE = "SELECT through_id,ceiling_id FROM conversation_route_coverage WHERE singleton=1"
ATTACHMENT = "scale attachment; exact public read\n"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def stage_pause(fixture):
    # Gate only the private Provider script and acknowledge from inside its
    # event loop, after any prior generated-hook subprocess finished.
    provider = fixture.root / "bin/qoder"
    source = provider.read_text()
    marker = "        while running and time.monotonic() < deadline:\n"
    assert source.count(marker) == 1, "upstream Qoder fixture loop changed"
    send = 'os.environ["AGENTX_MANAGEMENT_CANDIDATE"], "send"'
    assert source.count(send) == 1, "upstream Qoder fixture send changed"
    provider.write_text(source.replace(send, '(root / "bin/agentx").as_posix(), "send"').replace(marker, marker + """\
            if (root / "hold-provider").exists():
                (root / "provider-paused").touch()
                select.select([], [], [], 0.05)
                continue
"""))


def reserve_ids(db, count):
    """Reserve journal IDs so the real HTTP stop lands above the seed."""
    assert db.resolve(strict=True) == db
    with closing(sqlite3.connect(db.as_uri() + "?mode=rw", uri=True)) as conn, conn:
        before, = conn.execute("SELECT seq FROM sqlite_sequence WHERE name='events'").fetchone()
        assert conn.execute("UPDATE sqlite_sequence SET seq=? WHERE name='events'", (before + count,)).rowcount == 1
        return before + 1


def seed(db, reference, noise, first_id, stopped):
    """Populate the reserved history window after the fixture daemon and Agent stop."""
    name = reference["name"]
    created = struct.unpack("!d", int(reference["incarnation"]).to_bytes(8, "big"))[0]
    past = datetime.fromtimestamp(created - 1, timezone.utc).isoformat()
    now = datetime.fromtimestamp(created + 0.25, timezone.utc).isoformat()
    next_id = first_id
    expected = []
    with closing(sqlite3.connect(db.as_uri() + "?mode=rw", uri=True)) as conn, conn:
        conn.execute("PRAGMA synchronous=OFF")  # Disposable journal, discarded on interrupted seeding.
        assert not conn.execute("SELECT 1 FROM sqlite_schema WHERE name='conversation_route_coverage'").fetchone(), "bootstrap must predate the projection"
        stop_id = int(stopped["stopped_event"])
        stop_timestamp, = conn.execute("SELECT timestamp FROM events WHERE id=? AND instance=? AND type='life' AND json_extract(data,'$.action')='stopped'", (stop_id, name)).fetchone()
        assert datetime.fromisoformat(stop_timestamp) >= datetime.fromisoformat(now)

        def add(sender, recipients, text, *, at=now, display=None, mentions=(), visible=True, reply=None):
            nonlocal next_id
            data = {"from": display or sender, "text": text, "delivered_to": recipients,
                    "scope": "mentions", "mentions": list(mentions), "sender_kind": "agent",
                    "intent": "inform", "thread": "history-scale"}
            if reply:
                data["reply_to"] = int(reply)
            event_id = str(next_id)
            conn.execute(JOURNAL_INSERT, (next_id, at, sender, json.dumps(data)))
            next_id += 1
            if visible:
                expected.append({"id": event_id, "at": at, "from": data["from"], "text": text,
                                 "recipients": recipients, "intent": "inform", "reply_to": reply,
                                 "thread": "history-scale", "attachments": []})

        payload = json.dumps({"from": "background", "text": "unrelated scale message", "sender_kind": "agent",
                              "scope": "mentions", "mentions": ["noise-a"], "intent": "inform",
                              "delivered_to": [f"noise-{letter}" for letter in "abcdefgh"]})
        background = (now, "background", payload)

        def add_noise(count):
            nonlocal next_id
            conn.executemany(JOURNAL_INSERT, ((event_id, *background) for event_id in range(next_id, next_id + count)))
            next_id += count

        add(name, ["spectator"], "exclude before exact incarnation", at=past, visible=False)
        add("other", ["spectator"], "exclude display sender and mentions", display=name, mentions=[name], visible=False)
        add("other", [name, "spectator", name], "incoming actual delivered recipient")
        add(name, ["spectator"], "outgoing routing instance, different display sender", display="alias", reply=expected[0]["id"])
        add("other", [name], "third adjacent match for cold continuation")
        for index in range(5):
            add_noise(noise // 6)
            add(name if index % 2 else "other", [name] if index % 2 == 0 else [], f"sparse match {index + 4}")
        add_noise(noise - 5 * (noise // 6))
        for index in range(36):
            add(name, ["spectator"], f"byte budget {index:02d} " + "x" * 60_000)
        assert next_id == first_id + noise + 46 <= stop_id
        ceiling = conn.execute("SELECT MAX(id) FROM events").fetchone()[0]
    return expected, ceiling


def coverage(db):
    # This is only a timing marker. Never use storage contents as the HTTP oracle.
    with closing(sqlite3.connect(db.as_uri() + "?mode=ro", uri=True, timeout=1)) as conn:
        if conn.execute("SELECT 1 FROM sqlite_schema WHERE name='conversation_route_coverage'").fetchone():
            return conn.execute(COVERAGE).fetchone()


def target(http, agent):
    return {"request_id": str(uuid.uuid4()), "generation": http.generation, "agent": agent}


def path_for(agent):
    return f"/api/v1/agents/{agent['name']}/messages"


def exact(agent):
    return {key: value for key, value in agent.items() if key != "name" and value is not None}


def find_agent(http, name, *, stopped=False):
    return wait_for(f"{'stopped' if stopped else 'ready'} exact Agent {name}", lambda: next((
        row for row in http.agents()
        if row["agent"]["name"] == name and (row["agent"].get("stopped_event") if stopped else
                                             row["agent"].get("incarnation") and row["session_id"] and row["can_send"])
    ), None), timeout=25)


class Check:
    def __init__(self, fixture, args, report):
        self.fixture, self.args, self.report = fixture, args, report
        self.db = fixture.root / SCOPE / "scope/agentx.db"
        self.http = None

    def read(self, agent, label, limit, after=None):
        query = {"generation": self.http.generation, **exact(agent), "limit": limit, "after": after}
        # Keep exact request and response in the fixture's HTTP trace. Keep this
        # report concise even when a page contains nearly 2 MiB of text.
        receipt = {"case": label, "limit": limit, "after": after,
                   "trace": f"{self.http.prefix}-http-{self.http.sequence + 1:04d}.json"}
        self.report["reads"].append(receipt)
        started = time.monotonic()
        try:
            code, body = self.http.call("GET", path_for(agent), query=query)
        finally:
            receipt["ms"] = round((time.monotonic() - started) * 1000, 2)
        receipt["http"] = code
        if code == 200:
            assert body["generation"] == self.http.generation and type(body["has_more"]) is bool
            assert isinstance(body["next_after"], str) and all(isinstance(row["id"], str) for row in body["messages"])
            receipt.update(count=len(body["messages"]), has_more=body["has_more"], next_after=body["next_after"])
            if label.startswith("steady-"):
                assert receipt["ms"] < self.args.steady_ms, receipt
        else:
            receipt["error"] = body
        return code, body

    def page(self, agent, label, limit, after=None):
        code, response = self.read(agent, label, limit, after)
        assert code == 200, self.report["reads"][-1]
        return response

    def steady_get(self, label, path, **query):
        receipt = {"case": label, "trace": f"{self.http.prefix}-http-{self.http.sequence + 1:04d}.json"}
        self.report["reads"].append(receipt)
        started = time.monotonic()
        try:
            code, response = self.http.call("GET", path, query={"generation": self.http.generation, **query})
        finally:
            receipt["ms"] = round((time.monotonic() - started) * 1000, 2)
        receipt["http"] = code
        if code != 200:
            receipt["error"] = response
        assert code == 200 and receipt["ms"] < self.args.steady_ms, receipt
        assert response.get("generation", self.http.generation) == self.http.generation
        return response

    @staticmethod
    def assert_list(page, expected, *, more, after=None):
        actual = page["messages"]
        assert actual == expected, {"returned_ids": [row["id"] for row in actual],
                                    "expected_ids": [row["id"] for row in expected]}
        assert page["has_more"] is more
        assert page["next_after"] == (expected[-1]["id"] if expected else after), page["next_after"]

    def collect(self, agent, label, limit, expected):
        after, offset = None, 0
        for _ in range(len(expected) + 1):
            page = self.page(agent, label, limit, after)
            count = len(page["messages"])
            end = offset + count
            assert 0 < count <= limit and end <= len(expected)
            self.assert_list(page, expected[offset:end], more=end < len(expected))
            if not page["has_more"]:
                return page["next_after"]
            offset, after = end, page["next_after"]
        raise AssertionError("page walk did not terminate")

    def append(self, agent, after, *, attachment=None):
        text = "history scale live request"
        payload = {**target(self.http, agent), "message": {"text": text, "thread": "history-scale",
                   "intent": "request", "attachments": [attachment["id"]] if attachment else []}}
        self.http.post(path_for(agent), payload)
        operation = self.http.operation(payload["request_id"])
        event_id = operation["result"]["Sent"]["message_id"]
        found = []
        cursor = after

        def incoming():
            nonlocal cursor
            page = self.page(agent, "incremental", 20, cursor)
            assert not page["has_more"]
            fresh = page["messages"]
            cursor = page["next_after"]
            found.extend(fresh)
            assert len(found) <= 2, [row["id"] for row in found]
            return len(found) == 2

        wait_for("actual Qoder reply from the prior opaque cursor", incoming, timeout=25)
        sent, reply = found
        assert sent["id"] == event_id and sent["from"] == "bigboss" and sent["recipients"] == [agent["name"]], sent
        if attachment:
            original, separator, context = sent["text"].partition("\n\n")
            assert original == text and separator and attachment["name"] in context, sent
        else:
            assert sent["text"] == text, sent
        assert sent["attachments"] == ([attachment] if attachment else []), sent
        assert reply["id"] != event_id and reply["from"] == agent["name"] and reply["reply_to"] == event_id
        # The human mailbox address is not an Agent delivery receipt. The
        # reply is visible here because the exact Agent is its routing sender.
        assert reply["recipients"] == [] and reply["text"] == f"fixture reply #{event_id}: {text}", reply
        tail = self.page(agent, "incremental-no-replay", 20, cursor)
        self.assert_list(tail, [], more=False, after=cursor)
        self.report.setdefault("operations", []).append(operation)
        return found

    def action(self, agent, action):
        request = target(self.http, agent)
        self.http.post(f"/api/v1/agents/{agent['name']}/{action}", request)
        return self.http.operation(request["request_id"])

    def maintenance(self, stopped, successor, previous):
        # Remove two previously visible rows using different journal mutations.
        # Use a stopped daemon so the test cannot race its writer. Test the
        # trigger effects through exact stopped history after restart.
        self.action(successor, "stop")
        self.fixture.stop_daemon(SCOPE)
        assert self.fixture.servers[SCOPE].poll() is not None
        with closing(sqlite3.connect(self.db.as_uri() + "?mode=rw", uri=True)) as conn, conn:
            assert conn.execute("UPDATE events SET data=json_set(data,'$.delivered_to',json(?)) WHERE id=?",
                                ('["spectator"]', previous[0]["id"])).rowcount == 1
            assert conn.execute("DELETE FROM events WHERE id=?", (previous[1]["id"],)).rowcount == 1
        generation = self.http.generation
        self.http = self.fixture.start(SCOPE)
        assert self.http.generation == generation
        self.collect(stopped, "after-offline-update-delete", 100, previous[2:])

        self.fixture.cli(SCOPE, "history-reset", ["reset", "--go"])
        new = wait_for("reset generation published", lambda: (
            value if (value := self.http.get("/api/v1/management"))["generation"] != generation else None
        ), timeout=10)
        assert self.read(stopped, "previous-generation-after-reset", 20)[0] == 409
        self.http.generation = new["generation"]
        assert self.http.agents() == []
        assert self.read(stopped, "old-stopped-id-after-reset", 20)[0] == 409
        self.report["maintenance"] = {"updated_id": previous[0]["id"], "deleted_id": previous[1]["id"],
                                      "before_reset": generation, "after_reset": self.http.generation}

    def run(self):
        f, args = self.fixture, self.args
        stage_pause(f)
        candidate = f.candidate
        try:
            f.candidate = args.bootstrap
            old_http = f.prepare(SCOPE)
            project = f.root / SCOPE / "project"
            (project / ".agentx/roles").mkdir(parents=True)
            launch = {"request_id": str(uuid.uuid4()), "generation": old_http.generation,
                      "launch": {"tool": "qoder", "workspace": str(project), "prompt": "isolated sparse history", "title": "history-scale"}}
            old_http.post("/api/v1/agents", launch)
            name, = old_http.operation(launch["request_id"])["result"]["Launched"]["agents"]
            initial = find_agent(old_http, name)
            (f.root / "hold-provider").touch()
            wait_for("private Provider acknowledged pause", (f.root / "provider-paused").is_file, timeout=10)
            f.stop_daemon(SCOPE)
            assert f.servers[SCOPE].poll() is not None
            first_id = reserve_ids(self.db, args.noise + 46)
            self.http = f.start(SCOPE)
            assert self.http.generation == old_http.generation
            (f.root / "hold-provider").unlink()
            self.action(initial["agent"], "stop")
            stopped = find_agent(self.http, name, stopped=True)["agent"]
            f.stop_daemon(SCOPE)
            assert f.servers[SCOPE].poll() is not None
            seed_started = time.monotonic()
            expected, ceiling = seed(self.db, initial["agent"], args.noise, first_id, stopped)
            seed_ms = round((time.monotonic() - seed_started) * 1000, 2)
        finally:
            f.candidate = candidate

        self.report.update(seed={"unrelated": args.noise, "small_matches": 8, "byte_test_matches": 36,
                                 "journal_ceiling": ceiling, "bootstrap": str(args.bootstrap),
                                 "bootstrap_sha256": args.bootstrap_sha256,
                                 "offline_seed_ms": seed_ms})
        base = time.monotonic()
        self.http = f.start(SCOPE)
        assert self.http.generation == old_http.generation
        self.report.update(initial_agent=initial["agent"], stopped_agent=stopped)
        before = coverage(self.db)
        cold = self.report["cold_coverage"] = {"before_open": before}
        code, page = self.read(stopped, "cold-open", 20)
        cold["after_open"] = coverage(self.db)
        if code == 200:
            self.assert_list(page, expected[:20], more=True)
        else:
            assert code == 503 and "interrupt" in json.dumps(page).lower(), self.report["reads"][-1]
        prefix = self.page(stopped, "cold-prefix", 2)
        end = cold["after_prefix"] = coverage(self.db)
        self.assert_list(prefix, expected[:2], more=True)
        if args.sha256 == args.bootstrap_sha256:
            assert before is None and end is None
            if code != 200:
                self.report["result"] = "legacy-open-interrupted"
                return
        else:
            assert before and before[1] >= ceiling, "candidate did not start backfill for seeded rows"
            if before[0] >= before[1] or end[0] >= end[1]:
                self.report["cold_observation_limit"] = "coverage may already have completed during cold reads"

            def complete():
                point = coverage(self.db)
                return point if point and point[0] >= point[1] >= ceiling else None

            self.report["steady_coverage"] = {"watermark": wait_for("conversation routes coverage", complete, args.coverage_timeout),
                                              "elapsed_ms": round((time.monotonic() - base) * 1000, 2)}

        self.assert_list(self.page(stopped, "steady-open", 20), expected[:20], more=True)
        small = self.page(stopped, "steady-count-first", 3)
        self.assert_list(small, expected[:3], more=True)
        self.assert_list(self.page(stopped, "steady-count-next", 3, small["next_after"]), expected[3:6], more=True)
        byte = self.page(stopped, "steady-byte-first", 100)
        n = len(byte["messages"])
        assert 8 < n < len(expected) < 100, n
        self.assert_list(byte, expected[:n], more=True)
        self.assert_list(self.page(stopped, "steady-byte-next", 100, byte["next_after"]), expected[n:], more=False)
        tail = expected[-1]["id"]
        self.assert_list(self.page(stopped, "steady-empty-tail", 20, tail), [], more=False, after=tail)
        assert self.read(initial["agent"], "stale-original-current", 20)[0] == 409

        successor_name, = self.action(stopped, "resume")["result"]["Launched"]["agents"]

        def current():
            roster = self.steady_get("steady-current-roster", "/api/v1/agents", current_only="true")
            return next((row for row in roster["agents"] if row["agent"]["name"] == successor_name
                         and row["agent"]["incarnation"] and row["session_id"] and row["can_send"]), None)

        view = wait_for("new exact session in warm roster", current, timeout=25)
        successor = view["agent"]
        self.report["resumed_agent"] = successor
        assert successor_name == name and successor["incarnation"] != initial["agent"]["incarnation"] and view["session_id"] == initial["session_id"]
        details = self.steady_get("steady-current-detail", f"/api/v1/agents/{name}", **exact(successor))
        pending = sum(name in row["recipients"] for row in expected)
        assert pending == details["pending_messages"] == view["pending_messages"], (pending, details, view)
        assert details["agent"] == successor and details["can_send"]
        workspaces = self.steady_get("steady-workspaces", "/api/v1/workspaces")
        assert details["collaboration"]["workspace"] == str(project)
        assert str(project) in [workspace["path"] for workspace in workspaces["workspaces"]], workspaces
        self.report["current_reads"] = {"checked_unread": pending, "workspace": str(project)}
        empty = self.page(successor, "steady-resumed-empty", 20)
        self.assert_list(empty, [], more=False, after=empty["next_after"])

        status, attachment = self.http.call("POST", "/api/v1/attachments", raw=ATTACHMENT,
                                           query={"generation": self.http.generation, "name": "scale.txt", "media_type": "text/plain"},
                                           headers={"Content-Type": "application/octet-stream"})
        assert status == 201, (status, attachment)
        current_messages = self.append(successor, empty["next_after"], attachment=attachment)
        self.assert_list(self.page(successor, "steady-current-open-after-append", 20), current_messages, more=False)
        assert attachment["url"].startswith("/api/v1/attachments/")
        assert self.http.call("GET", attachment["url"]) == (200, ATTACHMENT)

        self.collect(stopped, "stopped-history-after-resume", 100, expected)
        if args.maintenance:
            self.maintenance(stopped, successor, expected)
        self.report["result"] = "pass" if code == 200 else "steady-pass-cold-open-interrupted"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("sha256", help="expected candidate SHA-256")
    parser.add_argument("evidence", type=Path, help="new directory under .artifacts/space-domain-sdk")
    parser.add_argument("--bootstrap", type=Path, default=LEGACY, help="pre-projection binary used only to stage the scope")
    parser.add_argument("--bootstrap-sha256", default=LEGACY_SHA)
    parser.add_argument("--noise", type=int, default=750_000)
    parser.add_argument("--coverage-timeout", type=float, default=150, help="seconds to wait for candidate backfill")
    parser.add_argument("--steady-ms", type=float, default=1500, help="per steady-state HTTP read bound in ms")
    parser.add_argument("--allow-cold-unavailable", action="store_true", help="exit zero on a bounded cold 503 only if all later checks pass")
    parser.add_argument("--maintenance", action="store_true", help="also verify private journal update/delete and public generation reset")
    args = parser.parse_args()
    if not __debug__:
        parser.error("the fixture requires Python assertions; do not use -O")
    if args.noise < 1 or args.coverage_timeout <= 0 or args.steady_ms <= 0:
        parser.error("positive scale and time limits required")
    args.evidence = args.evidence.resolve()
    if args.evidence == ARTIFACTS or not args.evidence.is_relative_to(ARTIFACTS):
        parser.error("evidence must be a new directory under .artifacts/space-domain-sdk")
    args.candidate, args.bootstrap = (path.resolve(strict=True) for path in (args.candidate, args.bootstrap))
    if digest(args.candidate) != args.sha256 or digest(args.bootstrap) != args.bootstrap_sha256:
        parser.error("binary SHA-256 mismatch")
    fixture = LiveFixture(args.candidate, args.evidence)
    report = {"candidate": str(args.candidate), "sha256": args.sha256, "configured_noise": args.noise,
              "reads": [], "result": "failed"}
    try:
        Check(fixture, args, report).run()
    except KeyboardInterrupt:
        report["result"] = "interrupted"
        raise
    except Exception:
        report["error"] = traceback.format_exc()
        raise
    finally:
        try:
            fixture.close()
            assert digest(args.bootstrap) == args.bootstrap_sha256, "bootstrap changed during the run"
            logs = fixture.root / SCOPE / "scope/.tmp/logs"
            if logs.is_dir():
                shutil.copytree(logs, args.evidence / "private-logs")
            shutil.rmtree(fixture.root)
            report["fixture_root_removed"] = True
        except Exception:
            report["cleanup_error"] = traceback.format_exc()
            raise
        finally:
            (args.evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"{report['result']}: {args.evidence / 'report.json'}", flush=True)
    return {"pass": 0, "legacy-open-interrupted": 2,
            "steady-pass-cold-open-interrupted": 0 if args.allow_cold_unavailable else 2}[report["result"]]


if __name__ == "__main__":
    sys.exit(main())
