"""Fixed real-native parent/child actions used by the acceptance model fixture."""

import importlib.util
import json
import os
import pathlib
import sqlite3
import subprocess
import sys
import threading
import time


ROOT = pathlib.Path(sys.argv[2])
ACTION = sys.argv[1]
AGENTX = ROOT / "bin/agentx"
assert os.environ.get("AGENTX_ACCEPTANCE_NAMESPACE") == "isolated-v1", "use the disposable namespace runner"


def identity():
    name = os.environ.get("AGENTX_INSTANCE_NAME", "")
    assert name, "native shell tool did not receive its per-thread identity"
    return name


def process_record(pid):
    ticks = pathlib.Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[19]
    return {"pid": pid, "startTicks": ticks}


def write_json(name, value):
    (ROOT / name).write_text(json.dumps(value) + "\n")


def wait_file(name, timeout=20):
    deadline = time.monotonic() + timeout
    while not (ROOT / name).exists():
        assert time.monotonic() < deadline, "timed out waiting for fixture file " + name
        threading.Event().wait(0.02)


def send(to, text):
    sender = identity()
    result = subprocess.run([str(AGENTX), "send", "@" + to, "--intent", "inform", "--iam", sender, "--", text], capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, {"stdout": result.stdout, "stderr": result.stderr}
    return {"from": sender, "to": to, "text": text, "stdout": result.stdout}


def spawn():
    parent = identity()
    write_json("parent-identity.json", {"name": parent, "thread": os.environ.get("CODEX_THREAD_ID")})
    write_json("native-shell-environment.json", dict(os.environ))
    with (ROOT / "child-driver.stdout").open("w") as out, (ROOT / "child-driver.stderr").open("w") as err:
        process = subprocess.Popen([sys.executable, __file__, "run", str(ROOT)], stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
    write_json("child-driver.json", process_record(process.pid))
    print("fixture native child driver started")


def run_child():
    # The parent tool's environment intentionally flows through the public
    # launcher, which must scrub parent identity and attach a distinct root.
    probe_path = pathlib.Path(__file__).resolve().parents[2] / ".docs/codex-activation/.tools/probe_native_protocol.py"
    spec = importlib.util.spec_from_file_location("native_probe", probe_path)
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    child_trace_root = ROOT / "child-trace"
    child_trace_root.mkdir()
    trace = probe.Trace(child_trace_root)
    client_type = probe.Client
    if (ROOT / "terminal-socket-case").exists():
        from codex_shared_fixture import TerminalSocketClient

        client_type = TerminalSocketClient
    client = client_type([str(AGENTX), "launch", "--tool", "codex", "--stdio", "--go", "--", "app-server"], dict(os.environ), ROOT / "project", trace, "child")
    write_json("child-proxy.json", process_record(client.process.pid))
    try:
        client.initialize(experimental=False)
        result = client.request("child-root", "thread/start", {"cwd": str(ROOT / "project"), "model": "fixture-model", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "danger-full-access"})
        thread = result["value"]["result"]["thread"]["id"]
        start = len(trace.events)
        response = probe.turn(client, "child-first", thread, "SHARED_CHILD_INITIAL")
        assert "error" not in response["value"], response
        recorded = trace.wait(lambda event: event["source"] == "child" and event["value"].get("method") == "item/completed" and event["value"].get("params", {}).get("item", {}).get("type") == "userMessage" and "SHARED_BUSINESS_REPLY" in json.dumps(event["value"]), start, timeout=25)
        probe.completed(trace, thread, recorded["seq"])
        write_json("child-done.json", {"thread": thread, "businessInput": recorded["value"]})
    except Exception as error:
        write_json("child-error.json", {"error": repr(error)})
        raise
    finally:
        client.stop()


def child_send():
    child = identity()
    write_json("child-identity.json", {"name": child, "thread": os.environ.get("CODEX_THREAD_ID")})
    wait_file("parent-idle")
    parent = json.loads((ROOT / "parent-identity.json").read_text())["name"]
    assert parent != child, "child inherited the parent AgentX identity"
    if (ROOT / "terminal-socket-case").exists():
        wrong = subprocess.run([str(AGENTX), "send", "@" + child, "--intent", "inform", "--iam", parent, "--", "WRONG_IDENTITY_MUST_NOT_BE_SENT"], capture_output=True, text=True, timeout=10)
        assert wrong.returncode != 0, "a child thread was able to send with the parent's identity"
        write_json("crossed-identity.json", {"exitCode": wrong.returncode})
    write_json("child-sent.json", send(parent, "SHARED_CHILD_REPLY"))


def parent_reply():
    child = json.loads((ROOT / "child-identity.json").read_text())["name"]
    write_json("parent-replied.json", send(child, "SHARED_BUSINESS_REPLY"))


def approval_write():
    (ROOT / "approval-effect.txt").write_text("approved exactly once\n")


def lookup():
    with sqlite3.connect(ROOT / "agentx/agentx.db") as db:
        rows = db.execute("SELECT participant_name FROM participant_sessions WHERE provider='codex' AND scope=? AND session_id=?", (str(ROOT / "home"), sys.argv[3])).fetchall()
    assert len(rows) == 1, rows
    print(rows[0][0])


def environment():
    label = sys.argv[3]
    write_json("shell-environment-" + label + ".json", dict(os.environ))
    print(json.dumps({"name": identity(), "thread": os.environ.get("CODEX_THREAD_ID"), "marker": os.environ.get("ACCEPTANCE_ROOT")}))


def direct_ready(label):
    value = {"name": identity(), "thread": os.environ["CODEX_THREAD_ID"]}
    write_json("direct-" + label + ".json", value)
    return value


def direct_launch_child():
    direct_ready("parent")
    command = [str(AGENTX), "launch", "--tool", "codex", "--title", "Direct child", "--headless", "--terminal", "tmux", "--go", "--json", "--", "DIRECT_CHILD_INITIAL"]
    result = subprocess.run(command, capture_output=True, text=True, timeout=20)
    write_json("direct-launch.json", {"code": result.returncode, "stdout": result.stdout, "stderr": result.stderr, "fixtureToolPath": os.environ.get("PATH")})
    assert result.returncode == 0, result.stderr or result.stdout


def direct_mail(label, target, text):
    direct_ready(label)
    other = json.loads((ROOT / ("direct-" + target + ".json")).read_text())["name"]
    write_json(label + ".json", send(other, text))


def hetero_send():
    claude = json.loads((ROOT / "hetero-claude.json").read_text())["name"]
    write_json("hetero-codex.json", {"name": identity(), "thread": os.environ.get("CODEX_THREAD_ID")})
    write_json("hetero-codex-sent.json", send(claude, "HETERO_CODEX_MESSAGE"))


def hetero_native():
    # Native Claude is deliberately a fixture process; launcher identity and
    # actual AgentX hook/message transports are the behavior under test here.
    name = identity()
    payload = {"session_id": "isolated-heterogeneous-claude", "cwd": str(ROOT / "project"), "source": "startup", "transcript_path": "", "tool_name": "Bash", "tool_input": {"command": "fixture"}}
    result = subprocess.run([str(AGENTX), "sessionstart"], input=json.dumps(payload), capture_output=True, text=True, timeout=10)
    assert result.returncode == 0 and name in result.stdout, {"stdout": result.stdout, "stderr": result.stderr}
    write_json("hetero-claude.json", {"name": name, "process": os.environ["AGENTX_PROCESS_ID"], "bootstrap": result.stdout})
    wait_file("hetero-read", timeout=25)
    result = subprocess.run([str(AGENTX), "post"], input=json.dumps(payload), capture_output=True, text=True, timeout=10)
    assert result.returncode == 0 and "HETERO_CODEX_MESSAGE" in result.stdout, {"stdout": result.stdout, "stderr": result.stderr}
    write_json("hetero-hook-receipt.json", {"name": name, "output": result.stdout})
    parent = json.loads((ROOT / "hetero-codex.json").read_text())["name"]
    write_json("hetero-claude-sent.json", send(parent, "HETERO_CLAUDE_REPLY"))
    threading.Event().wait(35)


{"spawn": spawn, "run": run_child, "child-send": child_send, "parent-reply": parent_reply, "approval-write": approval_write, "lookup": lookup, "environment": environment, "hetero-send": hetero_send, "hetero-native": hetero_native,
 "direct-launch-child": direct_launch_child, "direct-ready": lambda: direct_ready("child"), "direct-child-send": lambda: direct_mail("child-sender", "parent", "DIRECT_MAIL_TO_PARENT"),
 "direct-reply": lambda: direct_mail("parent-reply", "child", "DIRECT_MAIL_TO_CHILD")}[ACTION]()
