#!/usr/bin/env python3
"""Run selected real AgentX/native acceptance cases inside disposable namespaces.

Usage: python3 tests/codex_shared_runtime.py --agentx /absolute/local/agentx
       --native-package /absolute/vendor/x86_64-unknown-linux-musl
       --evidence /absolute/disposable/evidence --cases envelopes roots shared-eof

No Cargo/build, installs, paid inference or user service operations occur. The
outer process starts bubblewrap; all native service management happens inside
the proven isolated topology. Explicit paseo-* cases use the installed provider
API; generic JSON client cases alone do not verify Paseo.
See .docs/codex-activation/acceptance.md for case meaning and remaining gaps.
"""

import argparse
import concurrent.futures
import hashlib
import json
import os
import pathlib
import shutil
import sqlite3
import subprocess
import sys
import threading
import time
import traceback

from support.codex_shared_fixture import PROBE, REPO, RuntimeFixture, TerminalBridgeProcess


def response_value(response):
    value = response["value"]
    assert "error" not in value, value
    return value["result"]


def root_participant(fixture, thread):
    result = fixture.run_agentx(["list", "--json"])
    assert result.returncode == 0, result.stderr
    rows = json.loads(result.stdout)
    with sqlite3.connect(fixture.root / "agentx/agentx.db") as db:
        mapping = db.execute("SELECT participant_name FROM participant_sessions WHERE provider='codex' AND scope=? AND session_id=?", (str(fixture.native_home), thread)).fetchall()
    assert len(mapping) == 1, {"thread": thread, "mapping": mapping}
    matches = [row for row in rows if row.get("name") == mapping[0][0]]
    assert len(matches) == 1, {"thread": thread, "instances": rows}
    return matches[0]


def backend_record(fixture):
    path = fixture.native_home / "app-server-daemon/daemon.pid"
    assert path.is_file(), "production entry did not establish the selected native daemon"
    value = json.loads(path.read_text())
    assert value.get("pid"), value
    return value


def envelopes(fixture):
    """A6.1/A6.3: opaque requests, typed IDs and long frames over real bridge."""
    client = fixture.open_client("envelopes")
    client.initialize(experimental=False)
    vectors = json.loads((REPO / "tests/support/codex_shared_vectors.json").read_text())
    requests = vectors["clientRequests"]
    after = len(fixture.trace.events)
    for request in requests:
        client.send(request)
    seen = []
    for request in requests:
        result = fixture.trace.wait(lambda event: event["source"] == client.name and "method" not in event["value"] and type(event["value"].get("id")) is type(request["id"]) and event["value"].get("id") == request["id"], after)
        seen.append(result["value"])
    # The native error path must also survive a frame above a PTY line limit.
    text = "长帧\\\"😀" * 40000
    response = client.request("large-request", "fixture/futureMethod", {"literal": text})
    assert "error" in response["value"], response
    fixture.assert_no_protocol_pollution()
    return {"cases": ["A6.1 external ID preservation", "A6.3 large request"], "responses": seen, "largePayloadBytes": len(text.encode())}


def roots(fixture):
    """A4.1/A5.1: two qualified roots and read-only non-registration."""
    client = fixture.open_client("roots")
    client.initialize(experimental=False)
    root_a = fixture.start_thread(client, "root-a")
    root_b = fixture.start_thread(client, "root-b")
    a = root_participant(fixture, root_a)
    b = root_participant(fixture, root_b)
    assert a["name"] != b["name"], (a, b)
    response_value(client.request("read-a", "thread/read", {"threadId": root_a, "includeTurns": False}))
    assert root_participant(fixture, root_a)["name"] == a["name"]
    assert root_participant(fixture, root_b)["name"] == b["name"]
    return {"cases": ["A4.1 distinct roots", "A5.1 thread/read"], "rootA": root_a, "rootB": root_b, "participantA": a["name"], "participantB": b["name"]}


def shared_eof(fixture):
    """A9.1/A9.2: simultaneous entries, one backend and surviving connection."""
    a = fixture.open_client("shared-a")
    b = fixture.open_client("shared-b")
    with concurrent.futures.ThreadPoolExecutor(2) as pool:
        initializers = [pool.submit(client.initialize, False) for client in [a, b]]
        for job in initializers:
            job.result(timeout=25)
    generation = backend_record(fixture)
    root_a = fixture.start_thread(a, "start-a")
    root_b = fixture.start_thread(b, "start-b")
    # Exercise a real model turn before disconnect; a read alone is weaker.
    mark = len(fixture.trace.events)
    response_value(fixture.turn(b, "before-eof", root_b, "BEFORE_EOF_CASE"))
    fixture.completed(root_b, mark)
    a.stop()
    assert a.process.poll() is not None
    assert b.process.poll() is None
    assert backend_record(fixture) == generation
    mark = len(fixture.trace.events)
    response_value(fixture.turn(b, "after-eof", root_b, "AFTER_EOF_CASE"))
    fixture.completed(root_b, mark)
    assert backend_record(fixture) == generation
    fixture.assert_no_protocol_pollution()
    return {"cases": ["A9.1 simultaneous entries", "A9.2 stdin EOF"], "generation": generation, "rootA": root_a, "survivingRoot": root_b}


def thread_environment(fixture):
    """A4.1: actual native tools run A/B/A with per-root identities/config."""
    fixture.server.environment = True
    client = fixture.open_client("environment")
    client.initialize(experimental=False)
    roots = {}
    for label in ["A", "B"]:
        params = {"cwd": str(fixture.project), "model": "fixture-model", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "danger-full-access", "config": {"shell_environment_policy": {"set": {"ACCEPTANCE_ROOT": label}}}}
        roots[label] = response_value(client.request("environment-root-" + label, "thread/start", params))["thread"]["id"]
    observations = {}
    for label in ["A1", "B", "A2"]:
        root = roots[label[0]]
        mark = len(fixture.trace.events)
        response_value(fixture.turn(client, "environment-turn-" + label, root, "ENVIRONMENT_PROBE_" + label))
        fixture.completed(root, mark)
        env = json.loads((fixture.root / ("shell-environment-" + label + ".json")).read_text())
        row = root_participant(fixture, root)
        assert env["AGENTX_INSTANCE_NAME"] == row["name"], env
        assert env["CODEX_THREAD_ID"] == root, env
        assert env["ACCEPTANCE_ROOT"] == label[0], env
        with sqlite3.connect(fixture.root / "agentx/agentx.db") as db:
            announced = db.execute("SELECT name_announced FROM instances WHERE name=?", (row["name"],)).fetchone()[0]
        assert announced, "real thread hook never flushed its bootstrap identity"
        observations[label] = {"name": row["name"], "thread": root, "marker": env["ACCEPTANCE_ROOT"], "bootstrapFlushed": bool(announced)}
    assert observations["A1"] == observations["A2"]
    assert observations["A1"]["name"] != observations["B"]["name"]
    fixture.assert_no_protocol_pollution()
    return {"cases": ["A4.1 actual A/B/A shell and hook isolation"], "observations": observations}


def root_lifecycle(fixture):
    """A4.2: disconnected root resumes canonically; a native fork is distinct."""
    fixture.server.environment = True
    first = fixture.open_client("lifecycle-first")
    first.initialize(experimental=False)
    root = response_value(first.request("create", "thread/start", {"cwd": str(fixture.project), "model": "fixture-model", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "danger-full-access"}))["thread"]["id"]
    name = root_participant(fixture, root)["name"]
    with sqlite3.connect(fixture.root / "agentx/agentx.db") as db:
        old_binding = db.execute("SELECT binding_id FROM participant_attachments WHERE participant_name=? AND active=1", (name,)).fetchone()[0]
    mark = len(fixture.trace.events)
    response_value(fixture.turn(first, "before-resume", root, "ENVIRONMENT_PROBE_A1"))
    fixture.completed(root, mark)
    first.stop()
    second = fixture.open_client("lifecycle-second")
    second.initialize(experimental=False)
    resumed = response_value(second.request("resume", "thread/resume", {"threadId": root, "cwd": str(fixture.project), "sandbox": "danger-full-access", "approvalPolicy": "never", "config": {"shell_environment_policy": {"set": {"ACCEPTANCE_ROOT": "resumed"}}}}))["thread"]["id"]
    assert resumed == root
    assert root_participant(fixture, resumed)["name"] == name
    with sqlite3.connect(fixture.root / "agentx/agentx.db") as db:
        new_binding = db.execute("SELECT binding_id FROM participant_attachments WHERE participant_name=? AND active=1", (name,)).fetchone()[0]
    assert old_binding != new_binding
    mark = len(fixture.trace.events)
    response_value(fixture.turn(second, "after-resume", root, "ENVIRONMENT_PROBE_A2"))
    fixture.completed(root, mark)
    fork = response_value(second.request("fork", "thread/fork", {"threadId": root, "cwd": str(fixture.project), "sandbox": "danger-full-access", "approvalPolicy": "never", "config": {"shell_environment_policy": {"set": {"ACCEPTANCE_ROOT": "forked"}}}}))["thread"]["id"]
    fork_name = root_participant(fixture, fork)["name"]
    assert fork != root and fork_name != name
    mark = len(fixture.trace.events)
    response_value(fixture.turn(second, "after-fork", fork, "ENVIRONMENT_PROBE_B"))
    fixture.completed(fork, mark)
    resumed_env = json.loads((fixture.root / "shell-environment-A2.json").read_text())
    fork_env = json.loads((fixture.root / "shell-environment-B.json").read_text())
    assert resumed_env["AGENTX_INSTANCE_NAME"] == name
    assert resumed_env["ACCEPTANCE_ROOT"] == "resumed"
    assert fork_env["AGENTX_INSTANCE_NAME"] == fork_name
    assert fork_env["ACCEPTANCE_ROOT"] == "forked"
    fixture.assert_no_protocol_pollution()
    return {"cases": ["A4.2 canonical resume and distinct fork"], "root": root, "participant": name, "oldBinding": old_binding, "newBinding": new_binding, "fork": fork, "forkParticipant": fork_name}


def send_mail(fixture, participant, text):
    # A named outside sender tests the real AgentX routing boundary. It does
    # not claim a real parent-spawned model child or heterogeneous provider.
    sender = fixture.run_agentx(["start", "--as", "lobo", "--iam", "lobo"])
    assert sender.returncode == 0, sender.stderr
    result = fixture.run_agentx(["send", "--from", "fixture-child", "--iam", "lobo", "--intent", "inform", "@" + participant, "--", text])
    assert result.returncode == 0, {"stdout": result.stdout, "stderr": result.stderr}


def round_trip_receipts(fixture, parent, child):
    """Independent real SQLite evidence for both business-message receipts."""
    with sqlite3.connect(fixture.root / "agentx/agentx.db") as db:
        messages = [(row[0], json.loads(row[1])) for row in db.execute("SELECT id,data FROM events WHERE type='message' ORDER BY id")]
        cursors = dict(db.execute("SELECT name,last_event_id FROM instances WHERE name IN (?,?)", (parent, child)))
        claims = db.execute("SELECT resolution FROM inbox_claims WHERE lineage IN (SELECT id FROM participant_sessions WHERE participant_name IN (?,?))", (parent, child)).fetchall()
    selected = []
    for sender, receiver, text in [(child, parent, "SHARED_CHILD_REPLY"), (parent, child, "SHARED_BUSINESS_REPLY")]:
        matches = [(event_id, message) for event_id, message in messages if message.get("text") == text]
        assert len(matches) == 1, messages
        event_id, message = matches[0]
        assert message["from"] == sender and receiver in message["mentions"], message
        assert cursors[receiver] >= event_id, {"cursor": cursors, "message": event_id}
        selected.append(message)
    assert len(claims) >= 2 and all(row[0] == "confirmed" for row in claims), claims
    return {"messages": selected, "deliveredCursors": cursors, "claimResolutions": [row[0] for row in claims]}


def mailbox_turn(fixture, busy):
    """A2.1/A10.1: real pending mail while outer input remains untouched."""
    client = fixture.open_client("busy-mail" if busy else "idle-mail")
    client.initialize(experimental=False)
    root = fixture.start_thread(client, "mail-root")
    participant = root_participant(fixture, root)["name"]
    start = len(fixture.trace.events)
    initial = response_value(fixture.turn(client, "initial-turn", root, "HOLD_CASE" if busy else "INITIAL_IDLE_CASE"))
    if busy:
        fixture.trace.wait(lambda event: event["source"] == "model", start)
    else:
        fixture.completed(root, start)
    mark = len(fixture.trace.events)
    token = "SHARED_ACCEPTANCE_BUSY_MAIL" if busy else "SHARED_ACCEPTANCE_IDLE_MAIL"
    send_mail(fixture, participant, token)
    # No client request is sent after this point; only AgentX may activate it.
    recorded = fixture.trace.wait(lambda event: event["source"] == client.name and event["value"].get("method") == "item/completed" and event["value"].get("params", {}).get("threadId") == root and event["value"].get("params", {}).get("item", {}).get("type") == "userMessage" and token in json.dumps(event["value"], ensure_ascii=False), mark)
    fixture.server.gate.set()
    complete = fixture.completed(root, mark)
    if busy:
        assert complete["value"]["params"]["turn"]["id"] == initial["turn"]["id"], complete
    assert not [event for event in fixture.trace.events[mark:] if event["source"] == client.name and "id" in event["value"] and "method" not in event["value"]], "internal response leaked to outer client"
    fixture.assert_no_protocol_pollution()
    return {"cases": ["A2.1" if busy else "A10.1 generic native client"], "root": root, "participant": participant, "record": recorded["value"], "completion": complete["value"], "limit": "Outside fixture sender; does not establish A1 parent-spawned child or unmodified Paseo."}


def parent_child(fixture, terminal_socket=False):
    """A1 real native parent spawns a real native AgentX child and replies."""
    fixture.server.collaboration = True
    if terminal_socket:
        (fixture.root / "terminal-socket-case").touch()
    client = fixture.open_client("parent", terminal_socket=terminal_socket)
    client.initialize(experimental=False)
    result = client.request("parent-root", "thread/start", {"cwd": str(fixture.project), "model": "fixture-model", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "danger-full-access"})
    root = response_value(result)["thread"]["id"]
    parent = root_participant(fixture, root)["name"]
    generation = backend_record(fixture)
    start = len(fixture.trace.events)
    response_value(fixture.turn(client, "parent-first", root, "SHARED_PARENT_INITIAL"))
    initial_complete = fixture.completed(root, start)
    mark = len(fixture.trace.events)
    # The child waits for this effect after the native parent is idle. The
    # harness sends no outer request after the original parent turn.
    (fixture.root / "parent-idle").touch()
    recorded = fixture.trace.wait(lambda event: event["source"] == "parent" and event["value"].get("method") == "item/completed" and event["value"].get("params", {}).get("threadId") == root and event["value"].get("params", {}).get("item", {}).get("type") == "userMessage" and "SHARED_CHILD_REPLY" in json.dumps(event["value"]), mark, timeout=25)
    final_complete = fixture.completed(root, recorded["seq"])
    deadline = time.monotonic() + 25
    while not (fixture.root / "child-done.json").exists():
        assert not (fixture.root / "child-error.json").exists(), (fixture.root / "child-error.json").read_text()
        assert time.monotonic() < deadline, "real child did not receive the parent's business reply"
        threading.Event().wait(0.02)
    parent_identity = json.loads((fixture.root / "parent-identity.json").read_text())
    child_identity = json.loads((fixture.root / "child-identity.json").read_text())
    child_done = json.loads((fixture.root / "child-done.json").read_text())
    assert parent_identity["name"] == parent
    assert parent_identity["thread"] == root
    assert child_identity["name"] != parent
    assert child_identity["thread"] == child_done["thread"]
    assert child_done["thread"] != root
    assert json.loads((fixture.root / "child-sent.json").read_text())["to"] == parent
    assert json.loads((fixture.root / "parent-replied.json").read_text())["to"] == child_identity["name"]
    receipts = round_trip_receipts(fixture, parent, child_identity["name"])
    if terminal_socket:
        assert backend_record(fixture) == generation
        assert (fixture.root / "crossed-identity.json").exists()
        with sqlite3.connect(fixture.root / "agentx/agentx.db") as db:
            owners = db.execute("SELECT participant_name, execution, owner FROM participant_attachments ORDER BY participant_name").fetchall()
            wrong = db.execute("SELECT COUNT(*) FROM events WHERE type='message' AND json_extract(data, '$.text')='WRONG_IDENTITY_MUST_NOT_BE_SENT'").fetchone()[0]
        assert {record[0] for record in owners} == {parent, child_identity["name"]} and len(owners) == 2, owners
        assert owners[0][1] == owners[1][1] and owners[0][2] != owners[1][2], owners
        assert wrong == 0
        receipts.update(bridge="unix-websocket", sharedExecution=True, separateOwners=True, crossedIdentityRejected=True)
    assert not [event for event in fixture.trace.events if event["source"] == "fixture:unexpected"]
    fixture.assert_no_protocol_pollution()
    return {"cases": ["A1.1 real AgentX/native parent-child round trip", "A4.1 parent/child tool environment"], "parent": parent_identity, "child": child_identity, "initialCompletion": initial_complete["value"], "inputRecord": recorded["value"], "finalCompletion": final_complete["value"], **receipts, "limit": "Fixed loopback model and generic native protocol client; actual Paseo provider API is a separate case."}


def terminal_tui(fixture):
    """Start a real Codex TUI in detached tmux and observe its exact live thread."""
    fixture.server.environment = True
    with (fixture.native_home / "config.toml").open("a") as config:
        config.write(f'\n[projects."{fixture.project}"]\ntrust_level = "trusted"\n')
    bridge = TerminalBridgeProcess([str(fixture.agentx), "launch", "--tool", "codex", "--stdio", "--go", "--", "app-server"], fixture.env, fixture.project, fixture.trace, "interactive-bridge")
    config = fixture.root / "tmux.conf"
    config.write_text("set -g remain-on-exit on\n")
    tmux = [os.environ["AGENTX_ACCEPTANCE_TMUX"], "-S", str(fixture.root / "tmux.sock"), "-f", str(config)]

    def command(args, check=True):
        return subprocess.run(args, env=fixture.env, cwd=fixture.project, capture_output=True, text=True, timeout=10, check=check).stdout

    def new_pane(name, *argv):
        command([*tmux, "new-session", "-d", "-x", "115", "-y", "30", "-s", name, "--", str(fixture.codex), *argv])

    def wait_until(probe, label):
        deadline = time.monotonic() + 18
        while True:
            if value := probe():
                return value
            assert time.monotonic() < deadline, label
            threading.Event().wait(0.1)

    def environment(label):
        path = fixture.root / f"shell-environment-{label}.json"
        if path.exists() and path.stat().st_size:
            return json.loads(path.read_text())

    observer = None
    try:
        new_pane("child", "--remote", "unix://" + str(bridge.socket), "-C", str(fixture.project), "-s", "danger-full-access", "-a", "never", "ENVIRONMENT_PROBE_A1")
        info = wait_until(lambda: environment("A1"), "real Codex TUI did not run the identity tool")
        thread = info["CODEX_THREAD_ID"]
        participant = root_participant(fixture, thread)["name"]
        assert participant == info["AGENTX_INSTANCE_NAME"]
        assert "AGENTX_CODEX_TUI_SOCKET" not in info
        observer = PROBE.UnixWebSocketClient(fixture.native_home / "app-server-control/app-server-control.sock", fixture.trace, "tui-observer")
        observer.initialize()
        assert thread in response_value(observer.request("loaded", "thread/loaded/list", {}))["data"]
        generation = backend_record(fixture)
        new_pane("center", "agents", "--remote", "unix://")
        wait_until(lambda: "ENVIRONMENT_PROBE_A1" in command([*tmux, "capture-pane", "-p", "-t", "center"]), "thread was missing from the native Command Center")
        command([*tmux, "send-keys", "-t", "center", "Enter"])
        wait_until(lambda: "Agent Command Center" not in command([*tmux, "capture-pane", "-p", "-t", "center"]) and "ENVIRONMENT_DONE_A1" in command([*tmux, "capture-pane", "-p", "-t", "center"]), "Command Center did not switch to the live conversation")
        command([*tmux, "send-keys", "-t", "center", "-l", "ENVIRONMENT_PROBE_A2"])
        threading.Event().wait(0.25)
        command([*tmux, "send-keys", "-t", "center", "Enter"])
        after = wait_until(lambda: environment("A2"), "switched TUI did not run the identity tool")
        assert after["CODEX_THREAD_ID"] == thread and after["AGENTX_INSTANCE_NAME"] == participant
        assert backend_record(fixture) == generation
        return {"nativeTui": "tmux", "participant": participant, "thread": thread, "sameDaemon": True, "inCommandCenter": True, "identityAfterNativeSwitch": True}
    except Exception:
        for pane in ["child", "center"]:
            (fixture.root / (pane + ".stdout")).write_text(command([*tmux, "capture-pane", "-p", "-t", pane], check=False))
        raise
    finally:
        if observer:
            observer.stop()
        command([*tmux, "kill-server"], check=False)
        bridge.stop()


def terminal_daemon_outage(fixture):
    """The legacy PTY fallback survives a sustained AgentX outage."""
    fixture.server.environment = True
    tmux_bin = os.environ["AGENTX_ACCEPTANCE_TMUX"]
    (fixture.bin_dir / "tmux").symlink_to(tmux_bin)
    socket_path = fixture.root / "tmux.sock"
    fixture.env["AGENTX_TMUX_SOCKET"] = str(socket_path)
    tmux = [tmux_bin, "-S", str(socket_path)]
    scope = fixture.root / "agentx"

    def pane(*args, check=True):
        return subprocess.run([*tmux, *args], env=fixture.env, capture_output=True, text=True, timeout=5, check=check).stdout

    def status():
        result = fixture.run_agentx(["daemon", "status"])
        # A passive status read can report unavailable while serve initializes.
        assert "Daemon:" in result.stdout, (result.stdout, result.stderr)
        return result.stdout

    def wait_for(predicate, label):
        deadline = time.monotonic() + 15
        while not predicate():
            assert time.monotonic() < deadline, label
            threading.Event().wait(0.1)

    def turn_environment(label):
        wait_for(lambda: "ENVIRONMENT_DONE_" + label in pane("capture-pane", "-p"), "Codex did not complete turn " + label)
        return json.loads((fixture.root / ("shell-environment-" + label + ".json")).read_text())

    with (fixture.root / "agentx-daemon.stderr").open("w") as log:
        daemon = subprocess.Popen([str(fixture.agentx), "daemon", "serve", "--scope", str(scope)], env=fixture.env, cwd=fixture.project, stdout=subprocess.DEVNULL, stderr=log, start_new_session=True)
    fixture.owned_processes.append(daemon)
    try:
        wait_for(lambda: daemon.poll() is None and f"Peer: pid={daemon.pid} " in status(), "fixture daemon did not become ready")
        launch = fixture.run_agentx(["launch", "--tool", "codex", "--headless", "--terminal", "tmux", "--go", "--json", "--", "--no-daemon", "-s", "danger-full-access", "-a", "never", "ENVIRONMENT_PROBE_A1"])
        assert launch.returncode == 0, (launch.stdout, launch.stderr)
        before = turn_environment("A1")
        thread, name = before["CODEX_THREAD_ID"], before["AGENTX_INSTANCE_NAME"]
        with sqlite3.connect(scope / "agentx.db") as db:
            rows = db.execute("SELECT pid FROM instances WHERE name=? AND session_id=?", (name, thread)).fetchall()
        assert len(rows) == 1 and rows[0][0], rows
        pid = rows[0][0]
        stat = pathlib.Path(f"/proc/{pid}/stat")
        start_ticks = stat.read_text().rsplit(")", 1)[1].split()[19]
        assert pathlib.Path(f"/proc/{pid}/exe").resolve() == fixture.codex

        # Occupy only the fixture's auto-start lock path. Hooks must see an
        # unavailable service for the entire second turn, not restart it.
        lock = scope / ".daemon/startup.lock"
        lock.unlink(missing_ok=True)
        lock.mkdir()
        daemon.kill()
        daemon.wait(timeout=5)
        assert "Daemon: stopped" in status()
        pane("send-keys", "-l", "ENVIRONMENT_PROBE_A2")
        threading.Event().wait(0.25)  # Allow native paste-burst detection to end.
        pane("send-keys", "Enter")
        after = turn_environment("A2")
        assert (after["CODEX_THREAD_ID"], after["AGENTX_INSTANCE_NAME"], after["AGENTX_PROCESS_ID"]) == (thread, name, before["AGENTX_PROCESS_ID"])
        assert stat.read_text().rsplit(")", 1)[1].split()[19] == start_ticks
        assert "Daemon: stopped" in status()
        return {"nativeTui": "agentx launch --headless --terminal tmux -- --no-daemon", "thread": thread, "codexPid": pid, "sameCodexProcess": True, "sameToolIdentity": True, "agentxDaemonStayedDown": True}
    finally:
        (fixture.root / "outage-pane.stdout").write_text(pane("capture-pane", "-p", check=False))
        pane("kill-server", check=False)


def terminal_direct(fixture):
    """Foreground parent and delegated tmux child share a native server without a relay."""
    fixture.server.direct_terminal = True
    tmux_bin = os.environ["AGENTX_ACCEPTANCE_TMUX"]
    (fixture.bin_dir / "tmux").symlink_to(tmux_bin)
    foreground_socket = fixture.root / "foreground.sock"
    child_socket = fixture.root / "tmux.sock"
    fixture.env.update(AGENTX_TMUX_SOCKET=str(child_socket), SHELL="/bin/bash")
    home_config = fixture.native_home / "config.toml"
    home_config.write_text(home_config.read_text().replace('sandbox_mode = "read-only"', 'sandbox_mode = "danger-full-access"') + f'\n[projects."{fixture.project}"]\ntrust_level = "trusted"\n')
    (fixture.root / "codex-config-before.log").write_text(home_config.read_text())
    scope = fixture.root / "agentx"

    def tmux(socket, *args, check=True):
        return subprocess.run([tmux_bin, "-S", str(socket), *args], env=fixture.env, cwd=fixture.project, capture_output=True, text=True, timeout=8, check=check).stdout

    def wait_for(check, label, duration=40):
        deadline = time.monotonic() + duration
        while not (value := check()):
            assert time.monotonic() < deadline, label
            threading.Event().wait(0.1)
        return value

    def screen(socket, marker, target=None):
        pane = ["-t", target] if target else []
        return wait_for(lambda: marker in tmux(socket, "capture-pane", "-p", *pane, check=False), marker)

    def submit(socket, target, text):
        pane = ["-t", target] if target else []
        tmux(socket, "send-keys", *pane, "-l", text)
        threading.Event().wait(0.25)
        tmux(socket, "send-keys", *pane, "Enter")

    with (fixture.root / "agentx-daemon.stderr").open("w") as log:
        daemon = subprocess.Popen([str(fixture.agentx), "daemon", "serve", "--scope", str(scope)], env=fixture.env, cwd=fixture.project, stdout=subprocess.DEVNULL, stderr=log, start_new_session=True)
    fixture.owned_processes.append(daemon)
    observer = None
    try:
        wait_for(lambda: f"Peer: pid={daemon.pid} " in fixture.run_agentx(["daemon", "status"]).stdout, "fixture daemon not ready")
        tmux(foreground_socket, "new-session", "-d", "-x", "115", "-y", "32", "-s", "parent", "--", str(fixture.agentx), "launch", "--tool", "codex", "--run-here", "--title", "Direct parent", "--go", "--", "DIRECT_PARENT_INITIAL")
        screen(foreground_socket, "DIRECT_PARENT_READY", "parent")
        launch_receipt = fixture.root / "direct-launch.json"
        wait_for(launch_receipt.is_file, "delegated launch did not finish readiness reporting")
        launched = json.loads(launch_receipt.read_text())
        assert launched["code"] == 0, launched
        screen(child_socket, "DIRECT_CHILD_READY")
        parent, child = [json.loads((fixture.root / f"direct-{label}.json").read_text()) for label in ["parent", "child"]]
        assert parent["name"] != child["name"] and parent["thread"] != child["thread"]
        generation = backend_record(fixture)
        observer = PROBE.UnixWebSocketClient(fixture.native_home / "app-server-control/app-server-control.sock", fixture.trace, "direct-observer")
        observer.initialize()
        loaded = response_value(observer.request("loaded", "thread/loaded/list", {}))["data"]
        assert {parent["thread"], child["thread"]}.issubset(loaded), loaded
        tmux(foreground_socket, "new-session", "-d", "-x", "115", "-y", "32", "-s", "center", "--", str(fixture.codex), "agents", "--remote", "unix://")
        screen(foreground_socket, "Direct child", "center")
        screen(foreground_socket, "Direct parent", "center")
        tmux(foreground_socket, "new-session", "-d", "-x", "115", "-y", "32", "-s", "attach", "--", "/usr/bin/env", "-u", "TMUX", "-u", "TMUX_PANE", str(fixture.agentx), "attach", child["name"])
        screen(foreground_socket, "DIRECT_CHILD_READY", "attach")
        tmux(foreground_socket, "kill-session", "-t", "attach")

        submit(foreground_socket, "parent", "DIRECT_BUSY_PARENT")
        wait_for((fixture.root / "direct-busy").is_file, "parent model did not become busy")
        submit(child_socket, None, "DIRECT_CHILD_SEND")
        screen(child_socket, "DIRECT_CHILD_SENT")
        read = response_value(observer.request("busy-parent", "thread/read", {"threadId": parent["thread"]}))
        assert read["thread"]["status"]["type"] == "active"
        assert not (fixture.root / "direct-parent-reply.json").exists()
        with sqlite3.connect(scope / "agentx.db") as db:
            assert db.execute("SELECT COUNT(*) FROM inbox_claims WHERE instr(messages, 'DIRECT_MAIL_TO_PARENT') > 0").fetchone() == (0,)
        fixture.server.gate.set()
        screen(foreground_socket, "DIRECT_PARENT_REPLIED", "parent")
        screen(child_socket, "DIRECT_CHILD_RECEIVED")

        def receipts():
            with sqlite3.connect(scope / "agentx.db") as db:
                claims = db.execute("SELECT resolution FROM inbox_claims WHERE instr(messages, 'DIRECT_MAIL_TO_PARENT') > 0 OR instr(messages, 'DIRECT_MAIL_TO_CHILD') > 0").fetchall()
                if claims != [("confirmed",), ("confirmed",)]:
                    return None
                events = [(event, json.loads(data)) for event, data in db.execute("SELECT id,data FROM events WHERE type='message'")]
                cursors = dict(db.execute("SELECT name,last_event_id FROM instances"))
                assert db.execute("SELECT COUNT(*) FROM notify_endpoints WHERE kind='inject' AND instance IN (?,?)", (parent["name"], child["name"])).fetchone() == (0,)
                for sender, receiver, marker in [(parent, child, "DIRECT_MAIL_TO_CHILD"), (child, parent, "DIRECT_MAIL_TO_PARENT")]:
                    found = [(event, msg) for event, msg in events if msg.get("text") == marker]
                    assert len(found) == 1 and found[0][1]["from"] == sender["name"] and receiver["name"] in found[0][1]["mentions"], found
                    assert cursors[receiver["name"]] >= found[0][0]
                return len(claims)

        confirmed = wait_for(receipts, "mail claims did not confirm")
        assert backend_record(fixture) == generation

        def status(name):
            with sqlite3.connect(scope / "agentx.db") as db:
                return db.execute("SELECT status, model_request FROM instances WHERE name=?", (name,)).fetchone()

        def history(thread, label):
            return response_value(observer.request(label, "thread/items/list", {"threadId": thread, "limit": 30, "sortDirection": "desc"}))["data"]

        failed_mail = fixture.run_agentx(["send", "--iam", parent["name"], "--intent", "inform", "@" + child["name"], "--", "DIRECT_RECOVERY_FAIL"])
        assert failed_mail.returncode == 0, failed_mail.stderr
        wait_for(lambda: status(child["name"])[0] == "error", "real Codex 403 did not reach AgentX")
        assert "403" in json.loads(status(child["name"])[1])["error"]["reason"]
        def failed_input_receipt():
            with sqlite3.connect(scope / "agentx.db") as db:
                return db.execute("SELECT resolution FROM inbox_claims WHERE instr(messages, 'DIRECT_RECOVERY_FAIL') > 0").fetchall() == [("confirmed",)]

        wait_for(failed_input_receipt, "recorded mail to the failed turn was not confirmed")
        loaded_before = set(response_value(observer.request("before-new", "thread/loaded/list", {}))["data"])
        submit(child_socket, None, "/new")
        screen(child_socket, "Ask Codex to do anything")
        submit(child_socket, None, "DIRECT_VIEW_OTHER")
        screen(child_socket, "DIRECT_OTHER_VISIBLE")
        created = set(response_value(observer.request("after-new", "thread/loaded/list", {}))["data"]) - loaded_before
        def visible_session(thread):
            packet = observer.request("new-view-" + thread, "thread/read", {"threadId": thread})["value"]
            return "DIRECT_VIEW_OTHER" in packet.get("result", {}).get("thread", {}).get("preview", "")

        [other] = [thread for thread in created if visible_session(thread)]
        other_before = history(other, "other-before-recover")
        before = history(parent["thread"], "parent-before-recover")

        def request_recovery():
            call = fixture.run_agentx(["recover", "@" + child["name"], "--json"])
            rows = json.loads(call.stdout)
            (fixture.root / "direct-recovery.json").write_text(json.dumps(rows))
            return rows[0] if rows[0]["status"] in {"queued", "already_requested", "in_progress", "submitted"} else None

        queued = wait_for(request_recovery, "native model failure was never eligible")
        correlation = "agentx:recover:" + queued["request_id"]
        failed_thread = response_value(observer.request("failed-thread", "thread/read", {"threadId": child["thread"]}))["thread"]
        (fixture.root / "direct-failed-thread.json").write_text(json.dumps(failed_thread))
        repeated = json.loads(fixture.run_agentx(["recover", "@" + child["name"], "--json"]).stdout)[0]
        assert repeated.get("request_id", queued["request_id"]) == queued["request_id"], repeated

        def recovery_items():
            return [entry["item"] for entry in history(child["thread"], "child-recovery") if (entry["item"].get("clientId") or "").startswith("agentx:recover:")]

        def recovery_confirmed():
            with sqlite3.connect(scope / "agentx.db") as db:
                stored = db.execute("SELECT value FROM kv WHERE key=?", ("model_recovery:" + child["name"],)).fetchone()
            return stored and json.loads(stored[0])["attempt"]["phase"] == "Submitted"

        wait_for(recovery_items, "original native child thread did not record continuation")
        wait_for(lambda: status(child["name"])[0] != "error" and json.loads(status(child["name"])[1])["error"] is None, "successful model request did not clear the error")
        wait_for(recovery_confirmed, "native recovery did not confirm its recorded input")
        assert [item["clientId"] for item in recovery_items()] == [correlation]
        assert history(parent["thread"], "parent-after-recover") == before
        assert history(other, "other-after-recover") == other_before
        # Reopen the exact native conversation for the outage check. This
        # separate TUI is also an ordinary direct native client.
        tmux(foreground_socket, "new-session", "-d", "-x", "115", "-y", "32", "-s", "child-view", "--", str(fixture.codex), "--remote", "unix://", "resume", child["thread"])
        screen(foreground_socket, "DIRECT_RECOVERY_COMPLETED", "child-view")

        lock = scope / ".daemon/startup.lock"
        lock.unlink(missing_ok=True)
        lock.mkdir()
        daemon.kill()
        daemon.wait(timeout=5)
        assert "Daemon: stopped" in fixture.run_agentx(["daemon", "status"]).stdout
        submit(foreground_socket, "parent", "DIRECT_OUTAGE_PROMPT")
        submit(foreground_socket, "child-view", "DIRECT_OUTAGE_PROMPT")
        screen(foreground_socket, "DIRECT_OUTAGE_COMPLETED", "parent")
        screen(foreground_socket, "DIRECT_OUTAGE_COMPLETED", "child-view")
        for identity in [parent, child]:
            read = response_value(observer.request("read-" + identity["name"], "thread/items/list", {"threadId": identity["thread"], "limit": 8, "sortDirection": "desc"}))
            assert any("DIRECT_OUTAGE_PROMPT" in json.dumps(entry["item"]) for entry in read["data"])
        assert "Daemon: stopped" in fixture.run_agentx(["daemon", "status"]).stdout and backend_record(fixture) == generation
        return {"parent": parent, "child": child, "sharedByDefault": True, "nativeCommandCenter": True, "deferredWhileBusy": True, "exactMessageAttribution": True, "confirmedClaims": confirmed, "tmuxReattached": True, "nativeRecoveryAfter403WhileSwitched": True, "sameNativeGeneration": True, "agentxDaemonStayedDown": True}
    finally:
        fixture.server.gate.set()
        if (scope / "agentx.db").is_file():
            with sqlite3.connect(scope / "agentx.db") as db:
                episodes = db.execute("SELECT key,value FROM kv WHERE key LIKE 'model_recovery:%'").fetchall()
                (fixture.root / "recovery-episodes.json").write_text(json.dumps(episodes))
        (fixture.root / "codex-config-after.log").write_text(home_config.read_text())
        internal_log = scope / ".tmp/logs/agentx.log"
        if internal_log.is_file():
            shutil.copy2(internal_log, fixture.root / "agentx.log")
        if observer:
            observer.stop()
        pid_record = fixture.native_home / "app-server-daemon/daemon.pid"
        if pid_record.is_file():
            environment = pathlib.Path(f"/proc/{json.loads(pid_record.read_text())['pid']}/environ")
            if environment.is_file():
                path = next((entry for entry in environment.read_bytes().split(b"\0") if entry.startswith(b"PATH=")), b"")
                (fixture.root / "native-path.log").write_bytes(path + b"\n")
        for socket, target in [(foreground_socket, "parent"), (foreground_socket, "attach"), (foreground_socket, "child-view"), (child_socket, None)]:
            pane = ["-t", target] if target else []
            (fixture.root / ((target or socket.stem) + ".stdout")).write_text(tmux(socket, "capture-pane", "-p", *pane, check=False))
        for socket in [foreground_socket, child_socket]:
            tmux(socket, "kill-server", check=False)


def heterogeneous(fixture):
    """A11.2: real native Codex exchanges mail with a launched Claude fixture."""
    import shlex
    fixture.server.heterogeneous = True
    helper = REPO / "tests/support/codex_shared_child.py"
    script = "#!/bin/sh\nif [ \"${1-}\" = --version ]; then printf '2.1.200 (Claude Code)\\n'; exit 0; fi\nexec " + shlex.join(["/usr/bin/python3", str(helper), "hetero-native", str(fixture.root)]) + "\n"
    fake = fixture.bin_dir / "claude"
    fake.write_text(script)
    fake.chmod(0o755)
    launched = fixture.run_agentx(["launch", "--tool", "claude", "--count", "1", "--headless", "--json", "--go", "--", "-p", "fixed heterogeneous fixture"])
    assert launched.returncode == 0, launched.stderr
    report = json.loads(launched.stdout)
    assert report["launched"] == 1, report
    claude_name = report["names"][0]
    fixture.provider_names.append(claude_name)
    deadline = time.monotonic() + 15
    while not (fixture.root / "hetero-claude.json").exists():
        assert time.monotonic() < deadline, "Claude fixture hook startup did not complete"
        threading.Event().wait(0.02)
    client = fixture.open_client("heterogeneous-codex")
    client.initialize(experimental=False)
    root = response_value(client.request("heterogeneous-root", "thread/start", {"cwd": str(fixture.project), "model": "fixture-model", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "danger-full-access"}))["thread"]["id"]
    codex_name = root_participant(fixture, root)["name"]
    mark = len(fixture.trace.events)
    response_value(fixture.turn(client, "heterogeneous-turn", root, "HETERO_CODEX_INITIAL"))
    fixture.completed(root, mark)
    mark = len(fixture.trace.events)
    (fixture.root / "hetero-read").touch()
    record = fixture.trace.wait(lambda event: event["source"] == client.name and event["value"].get("method") == "item/completed" and event["value"].get("params", {}).get("item", {}).get("type") == "userMessage" and "HETERO_CLAUDE_REPLY" in json.dumps(event["value"]), mark, timeout=25)
    fixture.completed(root, record["seq"])
    with sqlite3.connect(fixture.root / "agentx/agentx.db") as db:
        providers = dict(db.execute("SELECT name,tool FROM instances WHERE name IN (?,?)", (claude_name, codex_name)))
        events = [(row[0], json.loads(row[1])) for row in db.execute("SELECT id,data FROM events WHERE type='message' ORDER BY id")]
        cursors = dict(db.execute("SELECT name,last_event_id FROM instances WHERE name IN (?,?)", (claude_name, codex_name)))
    assert providers == {claude_name: "claude", codex_name: "codex"}, providers
    expected = [(codex_name, claude_name, "HETERO_CODEX_MESSAGE"), (claude_name, codex_name, "HETERO_CLAUDE_REPLY")]
    for sender, receiver, text in expected:
        messages = [(event_id, event) for event_id, event in events if event.get("text") == text]
        assert len(messages) == 1, events
        event_id, event = messages[0]
        assert event["from"] == sender and receiver in event["mentions"], event
        assert cursors[receiver] >= event_id, (cursors, event_id)
    return {"cases": ["A11.2 bidirectional heterogeneous message delivery"], "providers": providers, "cursors": cursors, "messages": events, "nativeRecord": record["value"], "limit": "Actual native Codex and actual AgentX Claude print/hook paths; Claude vendor runtime is a fixture process."}


def paseo(fixture, scenario):
    """Run the installed unmodified provider's actual concrete client API."""
    assert fixture.paseo_provider and fixture.node, "select --paseo-cli and --node for actual-client acceptance"
    fixture.server.collaboration = scenario == "parent-child"
    fixture.server.controls = scenario == "controls"
    with (fixture.root / "paseo.stdout").open("w") as out, (fixture.root / "paseo.stderr").open("w") as err:
        process = subprocess.Popen([str(fixture.node), str(REPO / "tests/support/paseo_shared_runtime.mjs"), str(fixture.paseo_provider), str(fixture.root), scenario], env=fixture.env, cwd=fixture.project, stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
        fixture.owned_processes.append(process)
        process.wait(timeout=75)
    report_path = fixture.root / "paseo-result.json"
    assert report_path.exists(), (fixture.root / "paseo.stderr").read_text()
    result = json.loads(report_path.read_text())
    assert process.returncode == 0 and result["status"] == "passed", result
    if scenario == "parent-child":
        parent = json.loads((fixture.root / "parent-identity.json").read_text())
        child = json.loads((fixture.root / "child-identity.json").read_text())
        assert parent["name"] != child["name"]
        assert parent["thread"] == result["nativeThread"]
        assert json.loads((fixture.root / "child-sent.json").read_text())["to"] == parent["name"]
        assert json.loads((fixture.root / "parent-replied.json").read_text())["to"] == child["name"]
        result["businessReceipts"] = round_trip_receipts(fixture, parent["name"], child["name"])
    return {"cases": ["A1.1", "A10.1"] if scenario == "parent-child" else ["A2.2", "A3 user cancel", "A6 native approval", "A10.1 controls"], "actualClient": result, "limit": "Installed unmodified Paseo provider API; desktop/mobile GUI rendering not exercised."}


CASES = {"envelopes": envelopes, "roots": roots, "thread-environment": thread_environment, "root-lifecycle": root_lifecycle, "shared-eof": shared_eof, "idle-mail": lambda fixture: mailbox_turn(fixture, False), "busy-mail": lambda fixture: mailbox_turn(fixture, True), "parent-child": parent_child, "terminal-parent-child": lambda fixture: parent_child(fixture, True), "heterogeneous": heterogeneous, "paseo-parent-child": lambda fixture: paseo(fixture, "parent-child"), "paseo-controls": lambda fixture: paseo(fixture, "controls")}
CASES["terminal-tui"] = terminal_tui
CASES["terminal-daemon-outage"] = terminal_daemon_outage
CASES["terminal-direct"] = terminal_direct


def in_namespace(args):
    # Only the outer launcher sets this after unsharing PID/network/mounts.
    assert os.environ.get("AGENTX_ACCEPTANCE_NAMESPACE") == "isolated-v1"
    summary = {"agentx": str(args.agentx), "nativePackage": str(args.native_package), "cases": {}, "productLimit": "Paseo provider API requires explicitly selected actual-client cases; GUI rendering remains unverified."}
    failed = False
    for name in args.cases:
        fixture = RuntimeFixture(args.agentx, args.native_package / "bin/codex", args.evidence)
        fixture.paseo_provider = args.paseo_cli / "node_modules/@getpaseo/server/dist/server/server/agent/providers/codex-app-server-agent.js" if args.paseo_cli else None
        fixture.node = args.node
        result = {"status": "failed"}
        try:
            result.update(CASES[name](fixture))
            result["status"] = "passed"
        except Exception:
            failed = True
            result["failure"] = traceback.format_exc()
        finally:
            # Keep primary failure evidence even if cleanup itself fails.
            target = fixture.retain(result)
            try:
                fixture.close()
                result["cleanup"] = "connections closed; isolated native daemon stopped; disposable state removed"
            except Exception:
                failed = True
                result["status"] = "failed"
                result["cleanupFailure"] = traceback.format_exc()
            result["evidence"] = str(target)
            (target / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
            summary["cases"][name] = result
    (args.evidence / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({name: result["status"] for name, result in summary["cases"].items()}))
    return int(failed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agentx", required=True, type=pathlib.Path)
    parser.add_argument("--native-package", required=True, type=pathlib.Path)
    parser.add_argument("--evidence", required=True, type=pathlib.Path)
    parser.add_argument("--paseo-cli", type=pathlib.Path)
    parser.add_argument("--node", type=pathlib.Path)
    parser.add_argument("--tmpfs-state", action="store_true", help="Keep disposable service state in memory; retain receipts on disk")
    parser.add_argument("--cases", nargs="+", choices=list(CASES), default=[name for name in CASES if not name.startswith(("paseo-", "terminal-"))])
    parser.add_argument("--in-namespace", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.in_namespace:
        return in_namespace(args)
    agentx = args.agentx.resolve(strict=True)
    package = args.native_package.resolve(strict=True)
    assert (package / "bin/codex").is_file(), "pass the full native platform package directory"
    assert (package / "bin/codex-package.json").is_file() or (package / "codex-package.json").is_file(), "native daemon needs its complete package resources"
    evidence = args.evidence.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    assert shutil.which("bwrap"), "bubblewrap isolation is required for native daemon acceptance"
    command = ["bwrap", "--ro-bind", "/", "/", "--unshare-all", "--die-with-parent", "--new-session", "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/home", "--tmpfs", "/run", "--tmpfs", "/tmp", "--ro-bind", str(REPO), "/tmp/work", "--ro-bind", str(agentx), "/tmp/test-agentx", "--ro-bind", str(package), "/opt", "--bind", str(evidence), "/mnt", "--clearenv", "--setenv", "PATH", "/usr/bin:/bin", "--setenv", "AGENTX_ACCEPTANCE_NAMESPACE", "isolated-v1", "--chdir", "/tmp/work", "/usr/bin/python3", "/tmp/work/tests/codex_shared_runtime.py", "--agentx", "/tmp/test-agentx", "--native-package", "/opt", "--evidence", "/mnt", "--in-namespace", "--cases", *args.cases]
    if args.tmpfs_state or any(name.startswith("terminal-") for name in args.cases):
        # Native daemon management installs a package into its home. Keep that
        # disposable copy off the worktree disk; retained receipts stay in /mnt.
        index = command.index("--clearenv")
        command[index:index] = ["--tmpfs", "/mnt/state"]
    if any(name.startswith("paseo-") for name in args.cases):
        assert args.paseo_cli and args.node, "actual Paseo cases require --paseo-cli and --node"
        # Insert mounts before the executable, leaving the installed provider
        # and Node untouched. /home remains hidden from fixture processes.
        index = command.index("--clearenv")
        command[index:index] = ["--ro-bind", str(args.paseo_cli.resolve(strict=True)), "/tmp/paseo-cli", "--ro-bind", str(args.node.resolve(strict=True)), "/tmp/fixture-node"]
        command.extend(["--paseo-cli", "/tmp/paseo-cli", "--node", "/tmp/fixture-node"])
    if {"terminal-tui", "terminal-daemon-outage", "terminal-direct"}.intersection(args.cases):
        tmux = pathlib.Path(shutil.which("tmux") or "/usr/bin/tmux").resolve(strict=True)
        if tmux.is_relative_to("/home/linuxbrew/.linuxbrew"):
            index = command.index("--clearenv")
            command[index:index] = ["--ro-bind", "/home/linuxbrew/.linuxbrew", "/home/linuxbrew/.linuxbrew"]
        index = command.index("--chdir")
        command[index:index] = ["--setenv", "AGENTX_ACCEPTANCE_TMUX", str(tmux)]
    basis = {"command": command, "agentxSha256": hashlib.sha256(agentx.read_bytes()).hexdigest(), "nativeSha256": hashlib.sha256((package / "bin/codex").read_bytes()).hexdigest()}
    (evidence / "basis.json").write_text(json.dumps(basis, indent=2) + "\n")
    return subprocess.run(command, check=False).returncode


if __name__ == "__main__":
    sys.exit(main())
