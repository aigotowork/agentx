#!/usr/bin/env python3
"""Probe native managed Codex only inside disposable Linux namespaces.

Requires bubblewrap and the complete native platform package. Never pass a
user home as --out: the output directory must not exist. Native evidence only;
this does not build or exercise the production AgentX connector.
"""

import argparse
import concurrent.futures
import hashlib
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading


def launch(package, output):
    package = package.resolve(strict=True)
    for relative in ["bin/codex", "codex-package.json"]:
        if not (package / relative).is_file():
            raise ValueError("--package must contain the complete native platform package")
    output = output.absolute()
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    tools = Path(__file__).resolve().parent
    command = [
        shutil.which("bwrap") or "bwrap", "--ro-bind", "/", "/", "--unshare-all", "--die-with-parent",
        "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/home", "--tmpfs", "/run", "--tmpfs", "/tmp",
        "--bind", str(output), "/mnt", "--ro-bind", str(package), "/opt", "--ro-bind", str(tools), "/tmp/tools",
        "/usr/bin/python3", "/tmp/tools/probe_managed_service.py", "--inside",
    ]
    with (package / "bin/codex").open("rb") as native:
        sha256 = hashlib.file_digest(native, "sha256").hexdigest()
    (output / "launch.json").write_text(json.dumps({"argv": command, "nativeSha256": sha256}, indent=2))
    return subprocess.run(command, check=False, timeout=120).returncode


def inside():
    # The outer entry has hidden host home/runtime paths and isolated process
    # and network namespaces before any native daemon command can run.
    os.environ.clear()
    os.environ.update(
        HOME="/mnt/os-home", CODEX_HOME="/mnt/codex-home", XDG_CONFIG_HOME="/mnt/config",
        XDG_RUNTIME_DIR="/mnt/runtime", XDG_CACHE_HOME="/mnt/cache", PATH="/usr/bin:/bin",
        DBUS_SESSION_BUS_ADDRESS="unix:path=/mnt/absent-bus", RUST_LOG="error",
        AGENTX_CODEX_SHARED="1", AGENTX_PROCESS_ID="codex:isolated-probe-generation", AGENTX_DIR="/mnt/agentx",
    )
    from probe_native_protocol import Client, Fixture, Trace, UnixWebSocketClient, completed, turn

    root = Path("/mnt")
    for name in ["os-home", "codex-home", "config", "runtime", "cache", "project", "agentx"]:
        (root / name).mkdir()
    os.chdir(root / "project")
    trace = Trace(root)
    fixture = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Fixture)
    fixture.trace, fixture.gate = trace, threading.Event()
    threading.Thread(target=fixture.serve_forever, daemon=True).start()
    (root / "codex-home/config.toml").write_text(f'''model = "fixture-model"
model_provider = "fixture"
approval_policy = "never"
sandbox_mode = "read-only"
[features]
memories = false
[model_providers.fixture]
name = "loopback fixed fixture"
base_url = "http://127.0.0.1:{fixture.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
request_max_retries = 0
stream_max_retries = 0
[analytics]
enabled = false
''')
    hook = root / "thread-hook.py"
    hook.write_text("import json,os,pathlib,sys\np=json.load(sys.stdin)\np['serviceEnv']={k:os.environ.get(k) for k in ['AGENTX_CODEX_SHARED','AGENTX_PROCESS_ID','AGENTX_INSTANCE_NAME','CODEX_HOME','AGENTX_DIR']}\nwith pathlib.Path('/mnt/hooks.jsonl').open('a') as f:f.write(json.dumps(p)+'\\n')\n")
    declaration = [{"hooks": [{"type": "command", "command": "/usr/bin/python3 /mnt/thread-hook.py"}]}]
    declaration_toml = "[{hooks=[{type='command',command='/usr/bin/python3 /mnt/thread-hook.py'}]}]"
    summary, clients = {}, []

    def manage(name, args):
        process = subprocess.run(["/opt/bin/codex", "app-server", "daemon", *args], capture_output=True, text=True, timeout=30)
        report = {"status": process.returncode, "stdout": process.stdout, "stderr": process.stderr}
        (root / (name + ".json")).write_text(json.dumps(report, indent=2))
        return report

    def read_pid():
        return json.loads((root / "codex-home/app-server-daemon/daemon.pid").read_text())

    def fixed_turn(client, label, thread_id):
        mark = len(trace.events)
        accepted = turn(client, label, thread_id, label)["value"]
        finished = completed(trace, thread_id, mark)["value"]
        if "error" in accepted or finished["params"]["turn"]["status"] != "completed":
            raise RuntimeError("fixture turn did not complete: " + label)
        return {"accepted": accepted, "finished": finished}

    try:
        preflight = Client(["/opt/bin/codex", "-c", "features.hooks=true", "-c", "hooks.UserPromptSubmit=" + declaration_toml, "app-server"], dict(os.environ), root / "project", trace, "preflight")
        clients.append(preflight)
        preflight.initialize()
        listed = preflight.request("hooks", "hooks/list", {"cwds": ["/mnt/project"]})["value"]
        summary["preflightHooks"] = listed
        state = {entry["key"]: {"trusted_hash": entry["currentHash"]} for entry in listed["result"]["data"][0]["hooks"] if entry.get("command")}
        preflight.stop()
        clients.remove(preflight)
        with concurrent.futures.ThreadPoolExecutor(2) as pool:
            summary["starts"] = list(pool.map(lambda number: manage("start-" + str(number), ["start", "-c", 'model="ignored-service-override"']), range(2)))
        reports = [json.loads(result["stdout"]) for result in summary["starts"]]
        assert {report["status"] for report in reports} == {"started", "alreadyRunning"}
        summary["pidBefore"] = read_pid()
        socket = Path("/mnt/codex-home/app-server-control/app-server-control.sock")
        a, b = (UnixWebSocketClient(socket, trace, name) for name in ["client-A", "client-B"])
        clients.extend([a, b])
        summary["initialize"] = [a.initialize(), b.initialize()]
        summary["configRead"] = a.request("config", "config/read", {"includeLayers": True})["value"]
        assert summary["configRead"]["result"]["config"]["model"] == "fixture-model"
        config = {"features": {"hooks": True}, "hooks": {"UserPromptSubmit": declaration, "state": state}}
        params = {"cwd": "/mnt/project", "model": "fixture-model", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "read-only", "config": config}
        summary["start"] = a.request("root-A", "thread/start", params)["value"]
        thread_id = summary["start"]["result"]["thread"]["id"]
        summary["startTurn"] = fixed_turn(a, "START_HOOK_CASE", thread_id)
        summary["resume"] = b.request("resume", "thread/resume", {"threadId": thread_id, "excludeTurns": True, "cwd": "/mnt/project", "config": config})["value"]
        assert "result" in summary["resume"]
        summary["resumeTurn"] = fixed_turn(b, "RESUME_HOOK_CASE", thread_id)
        summary["fork"] = b.request("fork", "thread/fork", {"threadId": thread_id, "excludeTurns": True, "cwd": "/mnt/project", "config": config})["value"]
        fork_id = summary["fork"]["result"]["thread"]["id"]
        summary["forkTurn"] = fixed_turn(b, "FORK_HOOK_CASE", fork_id)
        own_thread = b.request("root-B", "thread/start", params)["value"]["result"]["thread"]["id"]
        mark = len(trace.events)
        summary["heldStart"] = turn(b, "held", own_thread, "HOLD_CASE")["value"]
        trace.wait(lambda event: event["source"] == "model", mark)
        a.stop()
        clients.remove(a)
        fixture.gate.set()
        summary["completedAfterOtherDisconnect"] = completed(trace, own_thread, mark)["value"]
        assert summary["completedAfterOtherDisconnect"]["params"]["turn"]["status"] == "completed"
        summary["pidAfter"] = read_pid()
        assert summary["pidBefore"] == summary["pidAfter"]
        summary["laterTurn"] = fixed_turn(b, "LATER_CASE", own_thread)
        summary["hooks"] = [json.loads(line) for line in (root / "hooks.jsonl").read_text().splitlines()]
        assert len(summary["hooks"]) == 5
        assert all(event["serviceEnv"]["AGENTX_PROCESS_ID"] == "codex:isolated-probe-generation" and event["serviceEnv"]["AGENTX_INSTANCE_NAME"] is None for event in summary["hooks"])
        summary["outcome"] = "completed"
    except Exception as error:
        summary.update(outcome="incomplete", error=repr(error))
    finally:
        fixture.gate.set()
        for client in clients:
            try:
                client.stop()
            except Exception as error:
                summary.setdefault("cleanupErrors", []).append(repr(error))
        fixture.shutdown()
        try:
            summary["stop"] = manage("stop", ["stop"])
        except Exception as error:
            summary.setdefault("cleanupErrors", []).append(repr(error))
        summary["modelRequests"] = sum(event["source"] == "model" for event in trace.events)
        (root / "summary.json").write_text(json.dumps(summary, indent=2))
        print(json.dumps({key: summary[key] for key in ["outcome", "error", "modelRequests", "cleanupErrors"] if key in summary}), flush=True)
    return 0 if summary.get("outcome") == "completed" and not summary.get("cleanupErrors") else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.inside:
        sys.exit(inside())
    if not args.package or not args.out:
        parser.error("--package and --out are required")
    sys.exit(launch(args.package, args.out))
