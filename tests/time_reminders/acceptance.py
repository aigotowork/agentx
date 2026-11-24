#!/usr/bin/env python3
"""Finite Linux acceptance of an explicit AgentX candidate in a fresh Scope."""

import argparse
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing, contextmanager
from datetime import datetime
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import sqlite3
import struct
import subprocess
import sys
from tempfile import TemporaryDirectory
from threading import Event
import time

CALLER, TARGET, OTHER, STOPPED = "luna", "peso", "rune", "nita"
TIMING_MS = 2_000


def now_ms():
    return time.time_ns() // 1_000_000


def participant(name, created):
    return {"canonical_name": name, "incarnation_bits": struct.unpack(">Q", struct.pack(">d", created))[0]}


def wait_for(label, probe, seconds):
    deadline = time.monotonic() + seconds
    while True:
        result = probe()
        if result is not None:
            return result
        if time.monotonic() >= deadline:
            raise AssertionError(f"timed out waiting for {label}")
        time.sleep(0.04)


def json_lines(output):
    return [json.loads(line) for line in output.splitlines() if line.strip()]


class Fixture:
    """Own the foreground daemon, its private Scope, and all CLI invocations."""

    def __init__(self, binary, root):
        self.binary, self.root = binary, root
        self.scope = root / "scope"
        self.scope.mkdir(mode=0o700)
        (self.scope / ".daemon").mkdir(mode=0o700)
        for name in ("home", "work", "tmp", "runtime"):
            (root / name).mkdir(mode=0o700)
        self.env = {
            "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8", "NO_COLOR": "1", "CI": "1",
            "HOME": str(root / "home"), "TMPDIR": str(root / "tmp"),
            "XDG_CONFIG_HOME": str(root / "home/config"),
            "XDG_CACHE_HOME": str(root / "home/cache"),
            "XDG_DATA_HOME": str(root / "home/share"),
            "XDG_RUNTIME_DIR": str(root / "runtime"),
            "AGENTX_DIR": str(self.scope), "AGENTX_UPDATE_VERIFY": "1",
        }
        self.db = self.scope / "agentx.db"
        self.daemon = None
        self.log = root / "daemon.log"

    def process(self, *args, actor=None, isolated_scope=None):
        env = self.env.copy()
        if isolated_scope is not None:
            env["AGENTX_DIR"] = str(isolated_scope)
        identity = ("--iam", actor) if actor else ()
        argv = [str(self.binary), *identity, *args]
        return argv, dict(cwd=self.root / "work", env=env, stdin=subprocess.DEVNULL)

    def run(self, *args, actor=None, rejected=False, isolated_scope=None):
        argv, options = self.process(*args, actor=actor, isolated_scope=isolated_scope)
        result = subprocess.run(argv, **options, capture_output=True, text=True, timeout=8)
        detail = f"{argv[1:]} exited {result.returncode}\nstdout: {result.stdout}\nstderr: {result.stderr}"
        if rejected:
            assert result.returncode > 0, detail
            assert (result.stderr + result.stdout).strip(), detail
            assert not re.search(r"daemon (?:unavailable|request deadline expired|is incompatible)", result.stderr, re.I), detail
            return result.stderr + result.stdout
        assert result.returncode == 0, detail
        return result.stdout

    def start(self):
        assert self.daemon is None
        argv, options = self.process("daemon", "serve", "--scope", str(self.scope))
        with self.log.open("ab") as log:
            self.daemon = subprocess.Popen(argv, **options, stdout=log, stderr=log)

        def ready():
            assert self.daemon.poll() is None, f"owned daemon exited {self.daemon.returncode}"
            status_argv, status_options = self.process("daemon", "status", "--scope", str(self.scope))
            result = subprocess.run(status_argv, **status_options, capture_output=True, text=True, timeout=3)
            if result.returncode == 0 and re.search(r"^Daemon: running$", result.stdout, re.M):
                assert re.search(rf"^Peer: pid={self.daemon.pid}\b", result.stdout, re.M), result.stdout
                return now_ms()
            return None

        return wait_for("owned daemon readiness", ready, 8)

    def stop(self):
        if self.daemon is None:
            return
        child, self.daemon = self.daemon, None
        if child.poll() is not None:
            return
        child.terminate()
        try:
            child.wait(timeout=6)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=3)
            raise AssertionError("owned daemon required SIGKILL during cleanup")
        assert child.returncode == 0, f"owned daemon stopped with exit {child.returncode}"

    def seed(self, names, *, replace=False, delete=()):
        """Write only identity fixtures, with the daemon completely stopped."""
        assert self.daemon is None
        created = time.time()
        with closing(sqlite3.connect(self.db.as_uri() + "?mode=rw", uri=True)) as db, db:
            for name in delete:
                assert db.execute("DELETE FROM instances WHERE name = ?", (name,)).rowcount == 1, name
            for name in names:
                if replace:
                    assert db.execute("DELETE FROM instances WHERE name = ?", (name,)).rowcount == 1, name
                db.execute(
                    """INSERT INTO instances
                       (name, created_at, status, status_time, last_seen, last_event_id, directory, tool)
                       VALUES (?, ?, 'active', ?, ?, 0, ?, 'codex')""",
                    (name, created, int(created), int(created), str(self.root / "work")),
                )
        return {name: participant(name, created) for name in names}

    @contextmanager
    def read_db(self):
        with closing(sqlite3.connect(self.db.as_uri() + "?mode=ro", uri=True, timeout=0.25)) as db:
            db.row_factory = sqlite3.Row
            yield db

    def persisted(self, reminder_id):
        with self.read_db() as db:
            row = db.execute("SELECT * FROM reminders WHERE id = ?", (reminder_id,)).fetchone()
            assert row is not None, reminder_id
            return dict(row)

    def settled(self, view, state, *, seconds=7, on_time=False):
        def observe():
            row = self.persisted(view["id"])
            return row if row["state"] != "pending" else None

        row = wait_for(f"{view['id']} to become {state}", observe, seconds)
        assert row["state"] == state, row
        if on_time:
            lag = row["completed_at_ms"] - view["due_at_ms"]
            assert 0 <= lag <= TIMING_MS, f"{view['id']} completed {lag} ms relative to due"
            print(f"  {view['message']}: enqueued {lag} ms after due", flush=True)
        return row

    def show(self, reminder_id):
        result = json.loads(self.run("events", "sub", "show", reminder_id))
        assert result["id"] == reminder_id and result["kind"] == "time", result
        return result

    def create(self, delay, message, target, *, actor=CALLER, explicit=False):
        options = ("--for", target["canonical_name"]) if explicit else ()
        before = now_ms()
        output = self.run("events", "sub", "--in", delay, "--message", message, *options, actor=actor)
        after = now_ms()
        match = re.search(r"(?m)^Reminder (rem-[\w-]+) created\b", output)
        assert match, output
        reminder_id = match[1]
        for line in (f"target: {target['canonical_name']}", f"stop: agentx events unsub {reminder_id}",
                     f"show: agentx events sub show {reminder_id}"):
            assert line in output, output
        due = re.search(r"(?m)^\s*due:\s*(\S+)", output)
        assert due and due[1].endswith("Z"), output
        view = self.show(reminder_id)
        seconds = int(delay[:-1]) * {"s": 1, "m": 60, "h": 3600, "d": 86400}[delay[-1]]
        assert before + seconds * 1000 - 1 <= view["due_at_ms"] <= after + seconds * 1000 + 1, view
        rendered = int(datetime.fromisoformat(due[1][:-1] + "+00:00").timestamp() * 1000)
        assert abs(rendered - view["due_at_ms"]) < 1000, (output, view)
        assert view["state"] == "pending" and view["message"] == message and view["target"] == target, view
        assert view["delivery_event_id"] is None and view["failure_reason"] is None, view
        return view

    def inbox(self, actor):
        return json_lines(self.run("listen", "--timeout", "1", "--json", actor=actor))

    def message_events(self):
        return json_lines(self.run("events", "--type", "message", "--full", "--last", "100"))


def help_pages(f):
    pristine = f.root / "help-only-scope"
    for flag, terms in [("--help", ["--in", "--message", "--for"]),
                        ("--help-all", ["--in", "--message", "show", "--history"])]:
        output = f.run("events", "sub", flag, isolated_scope=pristine)
        missing = [term for term in terms if term not in output]
        assert not missing, f"candidate events sub {flag} is missing {missing}:\n{output}"
    assert not pristine.exists(), "help initialized a Scope"


def grammar_and_cancel(f, actors):
    target = actors[TARGET]
    views = [f.create(delay, "grammar case", target, explicit=True) for delay in ("10s", "10m", "2h", "1d", "365d", "1d")]
    assert len({v["id"] for v in views}) == len(views), views
    listing = f.run("events", "sub", "list")
    for view in views:
        line = next((line for line in listing.splitlines() if view["id"] in line), "")
        assert all(term in line for term in (TARGET, "at-time", "pending", "grammar case")), listing

    for i, view in enumerate(views):
        reminder_id = view["id"]
        if i == 0:
            f.run("events", "unsub", reminder_id, actor=OTHER, rejected=True)
            f.run("events", "unsub", reminder_id, rejected=True)
            assert f.show(reminder_id)["state"] == "pending"
        f.run("events", "unsub", reminder_id, actor=TARGET if i == 0 else CALLER)
        canceled = f.show(reminder_id)
        assert canceled["state"] == "canceled" and canceled["delivery_event_id"] is None, canceled
    active, history = (f.run("events", "sub", "list", *args) for args in ((), ("--history",)))
    for view in views:
        assert view["id"] not in active and view["id"] in history, (active, history)

    command = ("events", "sub")
    reject = lambda *args: f.run(*command, *args, actor=CALLER, rejected=True)
    for bad in ("0s", "-2s", "10x", "366d", "18446744073709551616d", "2026-12-01T00:00:00Z"):
        reject(f"--in={bad}", "--message", "invalid timer")
    reject("--in", "2s")
    reject("--message", "missing delay")
    reject("--in", "2s", "--message", "")
    for conflict in (("--type", "message"), ("type = 'message'",), ("--once",), ("--on-hit", "unused"),
                     ("--as", CALLER), ("-b",), ("--device", "ZZZZ")):
        reject("--in", "2s", "--message", "conflicting timer", *conflict)
    for forbidden_target in ("bigboss", "no-such-participant"):
        reject("--in", "2s", "--message", "invalid target", "--for", forbidden_target)
    for selector in ((), ("--for", TARGET)):
        f.run(*command, "--in", "2s", "--message", "no caller", *selector, rejected=True)
    with f.read_db() as db:
        assert db.execute("SELECT count(*) FROM reminders").fetchone()[0] == len(views)

    f.run("events", "sub", "show", "rem-missing", rejected=True)


def legacy_subscriptions(f):
    def create(*args):
        output = f.run("events", "sub", "--for", OTHER, *args, actor=CALLER)
        match = re.search(r"\bsub-[\w-]+", output)
        assert match, output
        return match[0]

    once = create("--type", "message", "--once")
    sql = create("type = 'life'")
    listing = f.run("events", "sub", "list")
    for sub_id, mode in ((once, "once"), (sql, "continuous")):
        assert any(all(term in line for term in (sub_id, OTHER, mode)) for line in listing.splitlines()), listing
    return once, sql


def under_native_load(f, view):
    """Keep four CLI workers querying while timing persists independently."""
    stop = Event()

    def worker():
        replies = []
        while not stop.is_set():
            f.run("events", "--last", "1", "--full")
            replies.append(now_ms())
        return replies

    with ThreadPoolExecutor(max_workers=4) as pool:
        jobs = [pool.submit(worker) for _ in range(4)]
        try:
            row = f.settled(view, "enqueued", on_time=True)
            time.sleep(0.15)  # Measure successful requests on both sides of completion.
        finally:
            stop.set()
        replies = sorted(stamp for job in jobs for stamp in job.result())

    due, completed = view["due_at_ms"], row["completed_at_ms"]
    near_due = [t for t in replies if due - 1_000 <= t <= completed + 1_000]
    assert len(near_due) >= 4 and near_due[0] < due <= completed <= near_due[-1], near_due
    gaps = [second - first for first, second in zip(near_due, near_due[1:])]
    assert max(gaps) <= 500, f"native load lapsed near due/completion; maximum reply gap {max(gaps)} ms"
    print(f"  native load: {len(near_due)} replies near the due window; maximum gap {max(gaps)} ms", flush=True)


def expect_message(f, view, events):
    reminder_id, body = view["id"], view["message"]
    shown = f.show(reminder_id)
    assert shown["state"] == "enqueued" and shown["failure_reason"] is None, shown
    hits = [event for event in events if event["data"].get("reminder_id") == reminder_id or event["data"].get("text") == body]
    assert len(hits) == 1 and hits[0]["id"] == shown["delivery_event_id"], hits
    data = hits[0]["data"]
    assert data["reminder_id"] == reminder_id and data["text"] == body, data
    assert data["sender_kind"] == "system" and data["delivery_fence"] == view["target"], data
    assert data["mentions"] == [view["target"]["canonical_name"]], data
    return hits[0]["id"]


def exercise(f):
    help_pages(f)
    f.start()
    f.stop()
    actors = f.seed((CALLER, TARGET, OTHER, STOPPED))
    f.start()
    once, sql = legacy_subscriptions(f)

    print("CLI grammar, exact target, cancel and history", flush=True)
    grammar_and_cancel(f, actors)
    canceled = f.create("4s", "canceled before due", actors[TARGET], explicit=True)
    f.run("events", "unsub", canceled["id"], actor=CALLER)

    print("Quiet CLI window and sustained native reads", flush=True)
    idle = f.create("2s", "idle reminder — 回查", actors[CALLER])
    f.settled(idle, "enqueued", on_time=True)
    idle_event = expect_message(f, idle, f.message_events())
    assert [msg["event_id"] for msg in f.inbox(CALLER)] == [idle_event]
    assert not f.inbox(OTHER), "idle reminder reached a bystander or fired legacy subscription"
    f.run("events", "unsub", idle["id"], actor=CALLER, rejected=True)
    old = f.create("2s", "queued for the original target", actors[TARGET], explicit=True)
    under_native_load(f, old)
    old_event = expect_message(f, old, f.message_events())
    assert once in f.run("events", "sub", "list"), "reminder fired the message predicate"
    assert now_ms() >= canceled["due_at_ms"]
    assert f.show(canceled["id"])["state"] == "canceled"

    print("Legacy predicate still fires for ordinary direct traffic", flush=True)
    ordinary = "ordinary direct message across replacement"
    f.run("send", f"@{TARGET}", "--intent", "inform", "--", ordinary, actor=CALLER)
    notices = f.inbox(OTHER)
    assert len(notices) == 1 and notices[0]["from"] == "[agentx-events]" and f"[sub:{once}]" in notices[0]["text"], notices
    assert once not in f.run("events", "sub", "list"), "one-shot predicate was retained"

    print("Overdue restart, replaced and stopped targets, duplicate protection", flush=True)
    recovery = f.create("5s", "overdue recovery with an unchanged target", actors[CALLER])
    stale = f.create("5s", "pending for the replaced target", actors[TARGET], explicit=True)
    stopped = f.create("5s", "pending for the stopped target", actors[STOPPED], explicit=True)
    f.stop()
    due_views = (recovery, stale, stopped)
    assert all(f.persisted(v["id"])["state"] == "pending" for v in due_views), "daemon stopped after an appointment"
    new_target = f.seed((TARGET,), replace=True, delete=(STOPPED,))[TARGET]
    wait_for("reminders to become overdue while stopped", lambda: True if now_ms() > max(v["due_at_ms"] for v in due_views) else None, 7)
    assert all(f.persisted(v["id"])["state"] == "pending" for v in due_views)
    ready_at = f.start()
    for view, state in zip(due_views, ("enqueued", "failed", "failed")):
        row = f.settled(view, state, seconds=3)
        assert view["due_at_ms"] <= row["completed_at_ms"] <= ready_at + TIMING_MS, row
    for view, reason in ((stale, r"replac|incarnation"), (stopped, r"stop|missing")):
        shown = f.show(view["id"])
        assert shown["target"] == view["target"] and shown["state"] == "failed", shown
        assert re.search(reason, shown["failure_reason"] or "", re.I) and shown["delivery_event_id"] is None, shown

    recovered_event = expect_message(f, recovery, f.message_events())
    f.stop()
    f.start()
    time.sleep(TIMING_MS / 1000)
    events = f.message_events()
    for view, event_id in ((idle, idle_event), (old, old_event), (recovery, recovered_event)):
        assert expect_message(f, view, events) == event_id, "restart changed the delivery event"
    for view in (canceled, stale, stopped):
        assert all(e["data"].get("reminder_id") != view["id"] and e["data"].get("text") != view["message"] for e in events), view

    history = f.run("events", "sub", "list", "--history")
    active = f.run("events", "sub", "list")
    for view in (idle, old, canceled, recovery, stale, stopped):
        assert view["id"] in history and view["id"] not in active, (history, active)
    assert sql in active, "continuous legacy subscription lost on restart"
    f.run("events", "unsub", sql, actor=CALLER)
    assert sql not in f.run("events", "sub", "list")
    assert f.show(old["id"])["target"] != new_target
    replacement_inbox = f.inbox(TARGET)
    assert [msg["text"] for msg in replacement_inbox] == [ordinary], replacement_inbox
    assert [msg["event_id"] for msg in f.inbox(CALLER)] == [recovered_event]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path, help="path to the built AgentX native executable")
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("this fixture requires Linux")
    binary = args.candidate.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error(f"not an executable file: {binary}")

    # Fixture cleanup also runs when interrupted by the operator or a CI timeout.
    def interrupted(_signum, _frame):
        raise KeyboardInterrupt("acceptance interrupted")

    signal.signal(signal.SIGTERM, interrupted)
    with TemporaryDirectory(prefix="agentx-reminders-", dir="/tmp") as directory:
        fixture = Fixture(binary, Path(directory))
        # Prevent ordinary CLI acquisition from spawning a detached replacement
        # if the owned daemon unexpectedly exits during a case.
        with (fixture.scope / ".daemon/startup.lock").open("xb") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            try:
                print(f"Candidate: {binary} ({fixture.run('--version').strip()})", flush=True)
                exercise(fixture)
            except BaseException:
                if fixture.log.exists():
                    print(f"Daemon log tail:\n{fixture.log.read_text(errors='replace')[-5000:]}", file=sys.stderr)
                raise
            finally:
                fixture.stop()
    print("PASS: isolated one-shot reminder CLI acceptance")


if __name__ == "__main__":
    main()
