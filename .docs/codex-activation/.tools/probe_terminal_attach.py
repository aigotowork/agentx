#!/usr/bin/env python3
"""Compare direct native attach of blank threads and the historical bridge.

Run only inside the disposable namespaces constructed by main().
"""

import argparse
import http.server
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import threading
import time

from probe_native_protocol import Client, Trace, UnixWebSocketClient, install_config
from probe_session_environment import EnvironmentFixture, KEYS


# Historical transport comparison only. Ordinary terminals connect directly.
BRIDGE = r'''
const fs = require('node:fs');
const http = require('node:http');
const { WebSocket, WebSocketServer } = require('/tmp/ws');
const config = JSON.parse(fs.readFileSync('/mnt/bridge-config.json'));
const server = http.createServer();
const wss = new WebSocketServer({ server });
wss.on('connection', (tui) => {
  const native = new WebSocket('ws+unix:///mnt/home/app-server-control/app-server-control.sock:/');
  const pending = [];
  const creates = new Set();
  tui.on('message', (frame) => {
    let data = frame.toString();
    const rpc = JSON.parse(data);
    if (rpc.method === 'thread/start') {
      creates.add(rpc.id);
      const current = rpc.params.config || {};
      rpc.params.config = { ...current, ...config, features: { ...current.features, ...config.features } };
      data = JSON.stringify(rpc);
    }
    if (native.readyState === WebSocket.OPEN) native.send(data);
    else pending.push(data);
  });
  native.on('open', () => pending.splice(0).forEach((frame) => native.send(frame)));
  native.on('message', (frame) => {
    if (tui.readyState !== WebSocket.OPEN) return;
    const rpc = JSON.parse(frame);
    if (creates.delete(rpc.id) && rpc.result?.thread) fs.writeFileSync('/mnt/bridge-thread-id', rpc.result.thread.id);
    tui.send(frame.toString());
  });
  tui.on('close', () => native.close());
  native.on('close', () => tui.close());
  native.on('error', () => tui.close());
});
server.listen('/mnt/bridge.sock', () => fs.chmodSync('/mnt/bridge.sock', 0o600));
'''


def run(*args, check=True, timeout=24):
    result = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"{Path(args[0]).name} exited {result.returncode}: {result.stderr.strip()}")
    return result.stdout


def rpc(client, label, method, params):
    response = client.request(label, method, params)["value"]
    if "error" in response:
        raise RuntimeError(f"{method}: {response['error']}")
    return response["result"]


def wait_screen(tmux, pane, marker, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        screen = run(*tmux, "capture-pane", "-p", "-J", "-t", pane)
        if marker in "".join(line.rstrip() for line in screen.splitlines()):
            return
        time.sleep(0.1)
    raise RuntimeError(f"{pane} did not show {marker!r}")


def inside(tmux_program):
    root = Path("/mnt")
    native = "/opt/bin/codex"
    tmux_config = root / "tmux.conf"
    tmux_config.write_text("set -g remain-on-exit on\n")
    tmux = [tmux_program, "-S", "/mnt/tmux.sock", "-f", str(tmux_config)]
    trace = Trace(root)
    fixture = http.server.ThreadingHTTPServer(("127.0.0.1", 0), EnvironmentFixture)
    fixture.trace = trace
    fixture.cases = ["TUI_FIRST", "TUI_SECOND", "TUI_SLASH_NEW", "DIRECT_MAIL_TARGET_A", "TUI_BRIDGE_NEW"]
    threading.Thread(target=fixture.serve_forever, daemon=True).start()
    project, _, base = install_config(root, fixture.server_port)
    config_path = root / "home/config.toml"
    # The outer namespaces already isolate the fixture. Avoid a nested bwrap
    # for the fixed tool command, since it is unavailable on some Linux hosts.
    config_path.write_text(config_path.read_text().replace('sandbox_mode = "read-only"', 'sandbox_mode = "danger-full-access"') + f'\n[projects."{project}"]\ntrust_level = "trusted"\n')
    for directory in ["os-home", "config", "runtime", "cache"]:
        (root / directory).mkdir(mode=0o700)
    os.environ.clear()
    os.environ.update(base, HOME="/mnt/os-home", XDG_CONFIG_HOME="/mnt/config", XDG_RUNTIME_DIR="/mnt/runtime", XDG_CACHE_HOME="/mnt/cache", DBUS_SESSION_BUS_ADDRESS="unix:path=/mnt/no-bus", TERM="xterm-256color")
    os.chdir(project)
    reader = project / "read-env.py"
    reader.write_text(f"import json,os\nprint(json.dumps({{key:os.environ.get(key) for key in {KEYS!r}}}))\n")
    fixture.reader_script = reader
    hook = root / "record-hook.py"
    hook.write_text("import json,os,sys\np=json.load(sys.stdin)\nwith open('/mnt/events.jsonl','a') as f:f.write(json.dumps({'payload':p,'identity':os.getenv('FIXTURE_HOOK_ID')})+'\\n')\n")
    command = shlex.join(["env", "FIXTURE_HOOK_ID=child", sys.executable, str(hook)])
    declaration = [{"hooks": [{"type": "command", "command": command}]}]
    override = "hooks.UserPromptSubmit=[{hooks=[{type='command',command=" + json.dumps(command) + "}]}]"
    creator = observer = preflight = bridge = None
    started = False
    try:
        preflight = Client([native, "-c", "features.hooks=true", "-c", override, "app-server"], dict(os.environ), project, trace, "trust")
        preflight.initialize()
        trust = rpc(preflight, "trust", "hooks/list", {"cwds": [str(project)]})
        hashes = {row["key"]: {"trusted_hash": row["currentHash"]} for row in trust["data"][0]["hooks"] if row.get("command") == command}
        assert len(hashes) == 1, "the native hook trust receipt must identify the fixture hook"
        preflight.stop()
        preflight = None
        start = json.loads(run(native, "app-server", "daemon", "start", timeout=40))
        started = start.get("status") == "started"
        assert started, "the probe requires its own daemon"
        socket = root / "home/app-server-control/app-server-control.sock"
        observer = UnixWebSocketClient(socket, trace, "observer")
        creator = UnixWebSocketClient(socket, trace, "creator")
        for client in [observer, creator]:
            client.initialize()
        pid_path = root / "home/app-server-daemon/daemon.pid"
        generation = pid_path.read_bytes()
        identity = {"SESSION_PROBE_ID": "prepared", "AGENTX_PROCESS_ID": "fixture-execution", "AGENTX_INSTANCE_NAME": "fixture-child"}
        config = {"features": {"hooks": True}, "hooks": {"UserPromptSubmit": declaration, "state": hashes}, "shell_environment_policy": {"set": identity}}
        thread = rpc(creator, "create", "thread/start", {"cwd": str(project), "config": config, "threadSource": "user", "historyMode": "paginated"})["thread"]["id"]
        run(*tmux, "new-session", "-d", "-x", "105", "-y", "28", "-s", "blank", "--", native, "--no-alt-screen", "--remote", "unix://", "resume", thread)
        wait_screen(tmux, "blank", "no rollout found for thread id")
        assert thread in rpc(observer, "blank", "thread/loaded/list", {})["data"]
        print(json.dumps({"blankThreadLoadedButNativeResumeRejected": True}), flush=True)
        # A name persists the blank thread. Keep its configured live instance
        # subscribed until the TUI attaches; disk alone may reload defaults.
        rpc(creator, "name", "thread/name/set", {"threadId": thread, "name": "Terminal attach probe"})
        run(*tmux, "new-session", "-d", "-x", "105", "-y", "28", "-s", "child", "--", native, "--no-alt-screen", "--remote", "unix://", "resume", thread)
        wait_screen(tmux, "child", "Terminal attach probe")
        creator.stop()
        creator = None
        # An overview client can see the blank thread on the same native server.
        run(*tmux, "new-session", "-d", "-x", "105", "-y", "28", "-s", "direct-center", "--", native, "agents", "--remote", "unix://")
        wait_screen(tmux, "direct-center", "Terminal attach probe")
        assert not [row for row in trace.events if row["source"] == "model"]
        print(json.dumps({"directNamedBlankThreadAttached": True, "nativeCommandCenterShowsBlankThread": True, "setupConnectionClosed": True, "requestsBeforeFirstPrompt": 0}), flush=True)

        def turn_observation(marker, after):
            call_id = "call_environment_" + marker
            def output(row):
                items = row["value"].get("body", {}).get("input", []) if row["source"] == "model" else []
                return next((item for item in items if item.get("type") == "function_call_output" and item.get("call_id") == call_id), None)
            model_event = trace.wait(lambda row: output(row) is not None, after=after, timeout=14)
            result = output(model_event)["output"]
            hook_path = root / "events.jsonl"
            hooks = [json.loads(row) for row in hook_path.read_text().splitlines()] if hook_path.exists() else []
            if not isinstance(result, str):
                result = json.dumps(result)
            tool_env = next(json.loads(line) for line in result.splitlines() if line.startswith('{"SESSION_PROBE_ID"'))
            return tool_env, [item for item in hooks if marker in item["payload"].get("prompt", "")]

        def verify(marker, after, thread, ownership):
            tool_env, hooks = turn_observation(marker, after)
            expected = {**ownership, "CODEX_HOME": str(root / "home"), "CODEX_THREAD_ID": thread}
            hook_ok = any(item["identity"] == "child" and item["payload"].get("session_id") == thread for item in hooks)
            tool_ok = tool_env == expected
            loaded = thread in rpc(observer, marker, "thread/loaded/list", {})["data"]
            idle = False
            for index in range(18):
                read = rpc(observer, f"{marker}-read-{index}", "thread/read", {"threadId": thread})
                if read["thread"]["status"]["type"] == "idle":
                    idle = True
                    break
                time.sleep(0.1)
            print(json.dumps({"turn": marker, "modelReceivedToolResult": True, "loadedOnSharedDaemon": loaded, "hookIdentity": hook_ok, "toolIdentity": tool_ok, "idleAfterTurn": idle, "sameDaemon": generation == pid_path.read_bytes()}), flush=True)
            return hook_ok and tool_ok and idle and loaded and generation == pid_path.read_bytes()

        def submit(pane, marker):
            marks = len(trace.events)
            run(*tmux, "send-keys", "-t", pane, "-l", marker)
            time.sleep(0.25)  # Allow native paste-burst detection to end.
            run(*tmux, "send-keys", "-t", pane, "Enter")
            return marks

        first = verify("TUI_FIRST", submit("child", "TUI_FIRST"), thread, identity)
        run(*tmux, "send-keys", "-t", "direct-center", "Enter")
        wait_screen(tmux, "direct-center", "fixture-complete")
        second = verify("TUI_SECOND", submit("direct-center", "TUI_SECOND"), thread, identity)

        submit("direct-center", "/new")
        wait_screen(tmux, "direct-center", "Ask Codex to do anything")
        new_env, new_hooks = turn_observation("TUI_SLASH_NEW", submit("direct-center", "TUI_SLASH_NEW"))
        assert new_env["CODEX_THREAD_ID"] and new_env["CODEX_THREAD_ID"] != thread
        print(json.dumps({"slashNewCreatedDifferentThread": True, "slashNewInheritedToolIdentity": new_env["AGENTX_INSTANCE_NAME"] is not None, "slashNewInheritedHook": bool(new_hooks)}), flush=True)

        # The observer is an independent client: it never resumes a TUI or
        # forwards a TUI request. The switched pane is now displaying B.
        new_thread = new_env["CODEX_THREAD_ID"]
        wait_screen(tmux, "direct-center", "fixture-complete")
        def history(label, target):
            return rpc(observer, label, "thread/items/list", {"threadId": target, "limit": 30, "sortDirection": "desc"})["data"]

        before = history("before-mail-B", new_thread)
        mail = "DIRECT_MAIL_TARGET_A"
        correlation = "fixture-independent-mail-A"
        marks = len(trace.events)
        rpc(observer, "mail-A", "turn/start", {"threadId": thread, "input": [{"type": "text", "text": mail}], "clientUserMessageId": correlation})
        assert verify(mail, marks, thread, identity)
        recorded = [entry["item"] for entry in history("after-mail-A", thread) if entry["item"].get("clientId") == correlation]
        assert len(recorded) == 1 and recorded[0]["type"] == "userMessage" and recorded[0]["content"] == [{"type": "text", "text": mail, "text_elements": []}], recorded
        assert history("after-mail-B", new_thread) == before
        print(json.dumps({"independentNativeMailRecordedOnA": True, "visibleThreadBUnchanged": True, "noTuiProxy": True}), flush=True)

        fresh_identity = {**identity, "SESSION_PROBE_ID": "bridged", "AGENTX_INSTANCE_NAME": "fixture-bridged-child"}
        (root / "bridge.cjs").write_text(BRIDGE)
        (root / "bridge-config.json").write_text(json.dumps({**config, "shell_environment_policy": {"set": fresh_identity}}))
        bridge = subprocess.Popen(["/tmp/node", "/mnt/bridge.cjs"], start_new_session=True)
        socket = root / "bridge.sock"
        for _ in range(30):
            if socket.is_socket():
                break
            time.sleep(0.1)
        assert socket.is_socket(), "test bridge did not listen"
        marks = len(trace.events)
        run(*tmux, "new-session", "-d", "-x", "105", "-y", "28", "-s", "bridge", "--", native, "--no-alt-screen", "--remote", f"unix://{socket}", "TUI_BRIDGE_NEW")
        id_path = root / "bridge-thread-id"
        for _ in range(120):
            if id_path.is_file() and id_path.stat().st_size > 0:
                break
            time.sleep(0.1)
        bridged_thread = id_path.read_text()
        assert bridged_thread != thread
        fresh = verify("TUI_BRIDGE_NEW", marks, bridged_thread, fresh_identity)
        run(*tmux, "new-session", "-d", "-x", "105", "-y", "28", "-s", "center", "--", native, "agents", "--remote", "unix://")
        wait_screen(tmux, "center", "TUI_BRIDGE_NEW")
        print(json.dumps({"freshThreadInNativeCommandCenter": True}), flush=True)
        return 0 if first and second and fresh else 1
    except Exception:
        for pane in ["blank", "child", "direct-center", "bridge", "center"]:
            screen = run(*tmux, "capture-pane", "-p", "-t", pane, check=False)
            if screen:
                print(screen.rstrip(), file=sys.stderr)
        raise
    finally:
        run(*tmux, "kill-server", check=False)
        if bridge:
            bridge.terminate()
            bridge.wait(timeout=5)
        for client in [creator, observer, preflight]:
            if client:
                client.stop()
        if started:
            run(native, "app-server", "daemon", "stop")
        fixture.shutdown()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, help="Complete installed native Codex platform package")
    parser.add_argument("--scratch-parent", type=Path, default=Path(".agentx/.tmp"))
    parser.add_argument("--inside", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.inside:
        return inside(args.inside)
    if not args.package or not (args.package / "codex-package.json").is_file():
        parser.error("--package must contain codex-package.json and bin/codex")
    tmux = Path(shutil.which("tmux") or "/usr/bin/tmux").resolve(strict=True)
    node = Path(shutil.which("node") or "/usr/bin/node").resolve(strict=True)
    ws = Path(run(str(node), "-p", "require.resolve('ws/package.json')").strip()).parent
    tools = Path(__file__).resolve().parent
    with tempfile.TemporaryDirectory(prefix="codex-terminal-attach-", dir=args.scratch_parent.resolve()) as scratch:
        command = ["bwrap", "--ro-bind", "/", "/", "--unshare-all", "--die-with-parent", "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/home", "--tmpfs", "/run", "--tmpfs", "/tmp", "--bind", scratch, "/mnt", "--ro-bind", str(args.package.resolve()), "/opt", "--ro-bind", str(tools), "/tmp/tools"]
        if tmux.is_relative_to("/home/linuxbrew/.linuxbrew"):
            command += ["--ro-bind", "/home/linuxbrew/.linuxbrew", "/home/linuxbrew/.linuxbrew"]
        command += ["--ro-bind", str(node), "/tmp/node", "--ro-bind", str(ws), "/tmp/ws"]
        return subprocess.run([*command, "/usr/bin/python3", "/tmp/tools/probe_terminal_attach.py", "--inside", str(tmux)], timeout=95).returncode


if __name__ == "__main__":
    raise SystemExit(main())
