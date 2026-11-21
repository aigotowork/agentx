#!/usr/bin/env python3
"""Probe one disposable native Codex runtime using only a loopback SSE fixture.

Usage: python3 probe_native_protocol.py --codex /absolute/path/to/native/codex
       python3 probe_native_protocol.py --codex /absolute/path/to/native/codex --transport unix

The caller must select the native executable, not a wrapper requiring inherited
environment. No credentials/config/environment are inherited. All generated
state, traces and hook files stay in a fresh /tmp directory. No daemon is used.
The fixture chooses fixed output and one fixed client-executed dynamic tool;
it never asks Codex to execute shell commands. This is feasibility evidence,
not an agentx bridge implementation or a production-model integration test.
"""

import argparse
import hashlib
import http.server
import json
import os
import pathlib
import shlex
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time


class Trace:
    def __init__(self, root):
        self.root = root
        self.events = []
        self.cv = threading.Condition()

    def add(self, source, value):
        with self.cv:
            event = {"seq": len(self.events), "timeNs": time.time_ns(), "source": source, "value": value}
            self.events.append(event)
            with (self.root / "trace.jsonl").open("a") as f:
                f.write(json.dumps(event) + "\n")
            self.cv.notify_all()
        return event

    def wait(self, predicate, after=0, timeout=15):
        deadline = time.monotonic() + timeout
        with self.cv:
            while True:
                for event in self.events[after:]:
                    if predicate(event):
                        return event
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("Probe event timeout; inspect trace.jsonl and stderr files")
                self.cv.wait(remaining)


class Fixture(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def do_GET(self):
        data = json.dumps({"object": "list", "data": [{"id": "fixture-model", "object": "model"}]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        raw = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        body = json.loads(raw)
        event = self.server.trace.add("model", {"path": self.path, "body": body})
        # Looking at the last user message avoids retriggering markers in history.
        users = [x for x in body.get("input", []) if x.get("role") == "user"]
        prompt = json.dumps(users[-1] if users else {})
        tool_outputs = [x for x in body.get("input", []) if x.get("type") == "function_call_output"]
        use_tool = "TOOL_CASE" in prompt and not tool_outputs
        hold = "HOLD_CASE" in prompt
        response_id = "resp_fixture_" + str(event["seq"])
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Connection", "close")
        self.end_headers()

        def emit(kind, **fields):
            data = {"type": kind, **fields}
            self.wfile.write(("event: " + kind + "\ndata: " + json.dumps(data) + "\n\n").encode())
            self.wfile.flush()

        try:
            emit("response.created", response={"id": response_id, "object": "response", "status": "in_progress", "output": []})
            if hold:
                self.server.gate.wait(12)
            if use_tool:
                item = {"id": "fc_fixture", "type": "function_call", "call_id": "call_fixture", "name": "fixture_echo", "arguments": "{}"}
            else:
                item = {"id": "msg_fixture", "type": "message", "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": "fixture-ok", "annotations": []}]}
            emit("response.output_item.added", output_index=0, item={**item, "status": "in_progress"})
            emit("response.output_item.done", output_index=0, item=item)
            emit("response.completed", response={"id": response_id, "object": "response", "status": "completed", "output": [item], "usage": {"input_tokens": 1, "output_tokens": 1, "total_tokens": 2}})
        except (BrokenPipeError, ConnectionResetError):
            pass


class Client:
    def __init__(self, command, env, cwd, trace, name):
        self.trace, self.name = trace, name
        self.stderr = (trace.root / (name + ".stderr")).open("w")
        self.process = subprocess.Popen(command, env=env, cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr, text=True, start_new_session=True)
        threading.Thread(target=self.read, daemon=True).start()

    def read(self):
        for line in self.process.stdout:
            try:
                self.trace.add(self.name, json.loads(line))
            except json.JSONDecodeError:
                self.trace.add(self.name, {"invalidLine": line})
        self.trace.add(self.name, {"eof": True})

    def send(self, value):
        self.trace.add(self.name + ":send", value)
        self.process.stdin.write(json.dumps(value) + "\n")
        self.process.stdin.flush()

    def request(self, request_id, method, params):
        after = len(self.trace.events)
        self.send({"id": request_id, "method": method, "params": params})
        return self.trace.wait(lambda e: e["source"] == self.name and "method" not in e["value"] and type(e["value"].get("id")) is type(request_id) and e["value"].get("id") == request_id, after)

    def initialize(self, experimental=True):
        result = self.request("initialize", "initialize", {"clientInfo": {"name": "isolated_feasibility_probe", "version": "1"}, "capabilities": {"experimentalApi": experimental}})
        if "error" in result["value"]:
            raise RuntimeError(result)
        self.send({"method": "initialized", "params": {}})
        return result["value"]["result"]

    def stop(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGTERM)
                self.process.wait(timeout=3)
        self.stderr.close()


class UnixWebSocketClient(Client):
    """Small probe-only RFC6455 client for the native Unix WebSocket listener."""

    def __init__(self, path, trace, name):
        self.trace, self.name, self.buffer = trace, name, b""
        self.socket = socket.socket(socket.AF_UNIX)
        self.socket.settimeout(10)
        self.socket.connect(str(path))
        self.socket.sendall(b"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: MDEyMzQ1Njc4OWFiY2RlZg==\r\nSec-WebSocket-Version: 13\r\n\r\n")
        header = b""
        while b"\r\n\r\n" not in header:
            header += self.socket.recv(4096)
        header, self.buffer = header.split(b"\r\n\r\n", 1)
        if not header.startswith(b"HTTP/1.1 101"):
            raise RuntimeError(header)
        trace.add(name + ":handshake", {"status": header.split(b"\r\n")[0].decode()})
        self.socket.settimeout(None)
        threading.Thread(target=self.read, daemon=True).start()

    def take(self, count):
        while len(self.buffer) < count:
            data = self.socket.recv(max(4096, count - len(self.buffer)))
            if not data:
                raise EOFError()
            self.buffer += data
        data, self.buffer = self.buffer[:count], self.buffer[count:]
        return data

    def read(self):
        try:
            while True:
                first, second = self.take(2)
                size = second & 127
                if size in (126, 127):
                    size = int.from_bytes(self.take(2 if size == 126 else 8), "big")
                payload = self.take(size)
                opcode = first & 15
                if opcode == 8:
                    break
                if opcode == 9:
                    self.frame(payload, opcode=10)
                elif opcode == 1:
                    self.trace.add(self.name, json.loads(payload))
                elif opcode != 10:
                    raise RuntimeError("Probe does not implement fragmented/binary WebSocket frames")
        except (EOFError, OSError):
            pass
        except Exception as exc:
            self.trace.add(self.name, {"readError": repr(exc)})
        self.trace.add(self.name, {"eof": True})

    def frame(self, payload, opcode=1):
        length = len(payload)
        size = bytes([0x80 | length]) if length < 126 else bytes([0x80 | (126 if length < 65536 else 127)]) + length.to_bytes(2 if length < 65536 else 8, "big")
        mask = b"abcd"
        self.socket.sendall(bytes([0x80 | opcode]) + size + mask + bytes(v ^ mask[i % 4] for i, v in enumerate(payload)))

    def send(self, value):
        self.trace.add(self.name + ":send", value)
        self.frame(json.dumps(value).encode())

    def stop(self):
        self.socket.shutdown(socket.SHUT_RDWR)
        self.socket.close()


def install_config(root, port):
    home, project = root / "home", root / "project"
    home.mkdir()
    project.mkdir()
    (home / "config.toml").write_text(f'''model = "fixture-model"
model_provider = "fixture"
approval_policy = "never"
sandbox_mode = "read-only"
[features]
hooks = true
memories = false
[model_providers.fixture]
name = "Loopback fixture only"
base_url = "http://127.0.0.1:{port}/v1"
wire_api = "responses"
requires_openai_auth = false
request_max_retries = 0
stream_max_retries = 0
[analytics]
enabled = false
''')
    hook = root / "hook.py"
    hook.write_text('''import json,pathlib,sys,time
p=json.load(sys.stdin)
with pathlib.Path(sys.argv[1]).open('a') as f:
 f.write(json.dumps({'timeNs':time.time_ns(),'payload':p})+'\\n')
event=p.get('hook_event_name')
if event == 'UserPromptSubmit' and 'EMPTY_QUEUE_CASE' in p.get('prompt',''):
 print('fixture queue is empty',file=sys.stderr)
 sys.exit(2)
if event in ['SessionStart','UserPromptSubmit','PostToolUse']:
 print(json.dumps({'hookSpecificOutput':{'hookEventName':event,'additionalContext':'fixture-hook-context:'+event}}))
''')
    command = shlex.join([sys.executable, str(hook), str(root / "hooks.jsonl")])
    overrides = ["-c", "features.hooks=true"]
    for event in ["SessionStart", "UserPromptSubmit", "PostToolUse", "Stop", "Interrupt"]:
        declaration = "hooks." + event + "=[{hooks=[{type='command',command=" + json.dumps(command) + "}]}]"
        overrides.extend(["-c", declaration])
    env = {"PATH": "/usr/bin:/bin", "CODEX_HOME": str(home), "TMPDIR": str(root), "LANG": "C.UTF-8", "RUST_LOG": "error"}
    return project, overrides, env


def start_thread(client, project, request_id, stable=False):
    params = {"cwd": str(project), "model": "fixture-model", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "read-only"}
    if not stable:
        params.update({"allowProviderModelFallback": False, "dynamicTools": [{"type": "function", "name": "fixture_echo", "description": "Fixed local probe response; no side effects", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False}}]})
    result = client.request(request_id, "thread/start", params)
    return result["value"]["result"]["thread"]["id"]


def turn(client, request_id, thread_id, prompt, client_id=None):
    params = {"threadId": thread_id, "input": [{"type": "text", "text": prompt}]}
    if client_id is not None:
        params["clientUserMessageId"] = client_id
    return client.request(request_id, "turn/start", params)


def completed(trace, thread_id, after):
    return trace.wait(lambda e: e["value"].get("method") == "turn/completed" and e["value"].get("params", {}).get("threadId") == thread_id, after)


def focused_hooks(outer, trace, thread_id, fixture, summary):
    """Check hook rejection before model work, both idle and after admission."""
    mark = len(trace.events)
    summary["directStart"] = turn(outer, "direct-request", thread_id, "DIRECT_CASE", client_id="client-direct-1")["value"]
    summary["directCompleted"] = completed(trace, thread_id, mark)["value"]
    summary["directItems"] = [e["value"].get("params", {}).get("item", {}) for e in trace.events[mark:] if e["value"].get("method") == "item/completed"]
    mark = len(trace.events)
    summary["emptyStart"] = turn(outer, "empty-idle", thread_id, "<agentx>EMPTY_QUEUE_CASE</agentx>")["value"]
    summary["emptyCompleted"] = completed(trace, thread_id, mark)["value"]
    summary["emptyModelRequests"] = sum(e["source"] == "model" for e in trace.events[mark:])
    mark = len(trace.events)
    summary["holdStart"] = turn(outer, "hold", thread_id, "HOLD_CASE")["value"]
    trace.wait(lambda e: e["source"] == "model", mark)
    summary["activeEmptyStart"] = turn(outer, "empty-active", thread_id, "<agentx>EMPTY_QUEUE_CASE</agentx>")["value"]
    fixture.gate.set()
    summary["activeCompleted"] = completed(trace, thread_id, mark)["value"]
    summary["activeModelRequests"] = sum(e["source"] == "model" for e in trace.events[mark:])


def focused_valid_steer(outer, trace, thread_id, fixture, summary):
    """Check one valid active-turn steer and its client message correlation."""
    mark = len(trace.events)
    start = turn(outer, "steer-base", thread_id, "HOLD_CASE")
    active_turn_id = start["value"].get("result", {}).get("turn", {}).get("id")
    trace.wait(lambda e: e["source"] == "model" and e["seq"] >= mark)
    steer_mark = len(trace.events)
    summary["baseStart"] = start["value"]
    summary["activeTurnId"] = active_turn_id
    summary["steer"] = outer.request("steer-valid", "turn/steer", {"threadId": thread_id, "expectedTurnId": active_turn_id, "clientUserMessageId": "client-steer-1", "input": [{"type": "text", "text": "STEER_CASE"}]})["value"]
    fixture.gate.set()
    summary["completed"] = completed(trace, thread_id, mark)["value"]
    summary["steerItems"] = [e["value"].get("params", {}).get("item", {}) for e in trace.events[steer_mark:] if e["value"].get("method") == "item/completed"]
    summary["steerUserMessages"] = [item for item in summary["steerItems"] if item.get("type") == "userMessage"]
    summary["completedTurnId"] = summary["completed"].get("params", {}).get("turn", {}).get("id")
    summary["retainedActiveTurn"] = summary["completedTurnId"] == active_turn_id


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--codex", required=True, type=pathlib.Path)
    parser.add_argument("--transport", choices=["stdio", "unix", "unix-proxy"], default="stdio")
    parser.add_argument("--stable-api", action="store_true", help="Use experimentalApi=false and omit the experimental dynamic-tool case")
    parser.add_argument("--focused-hooks", action="store_true", help="Only check idle and busy empty-queue hook rejection")
    parser.add_argument("--valid-steer", action="store_true", help="Only check one valid active-turn turn/steer correlation")
    args = parser.parse_args()
    native = args.codex.resolve(strict=True)
    root = pathlib.Path(tempfile.mkdtemp(prefix="gina-native-protocol-"))
    print("artifacts=" + str(root), flush=True)
    trace = Trace(root)
    fixture = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Fixture)
    fixture.daemon_threads = True
    fixture.trace, fixture.gate = trace, threading.Event()
    threading.Thread(target=fixture.serve_forever, daemon=True).start()
    project, flags, env = install_config(root, fixture.server_port)
    clients, server = [], None
    summary = {"root": str(root), "native": str(native), "sha256": hashlib.sha256(native.read_bytes()).hexdigest(), "transport": args.transport, "fixturePort": fixture.server_port}
    try:
        # Discover hook hashes using the same session flag declarations, then
        # restart with trusted hashes. Trust affects only this disposable home.
        pre = Client([str(native), *flags, "app-server"], env, project, trace, "preflight")
        clients.append(pre)
        summary["initialize"] = pre.initialize()
        hooks = pre.request("hooks", "hooks/list", {"cwds": [str(project)]})["value"]["result"]["data"][0]["hooks"]
        state = {h["key"]: {"trusted_hash": h["currentHash"]} for h in hooks if h.get("command")}
        summary["hookCount"] = len(state)
        pre.stop()
        toml_state = "{" + ",".join(json.dumps(k) + "={trusted_hash=" + json.dumps(v["trusted_hash"]) + "}" for k, v in state.items()) + "}"
        flags.extend(["-c", "hooks.state=" + toml_state])
        if args.transport.startswith("unix"):
            sock = root / "native.sock"
            server = Client([str(native), *flags, "app-server", "--listen", "unix://" + str(sock)], env, project, trace, "native-server")
            clients.append(server)
            # Socket creation is a subprocess readiness condition, not an
            # agent wait; an Event provides a short bounded readiness backoff.
            deadline = time.monotonic() + 10
            while not sock.exists():
                if server.process.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError("Unix listener did not start")
                threading.Event().wait(0.02)
            command = [str(native), "app-server", "proxy", "--sock", str(sock)]
        else:
            command = [str(native), *flags, "app-server"]
        outer = UnixWebSocketClient(sock, trace, "outer") if args.transport == "unix" else Client(command, env, project, trace, "outer")
        clients.append(outer)
        outer.initialize(experimental=not args.stable_api)
        thread_id = start_thread(outer, project, 1, stable=args.stable_api)
        summary["thread"] = thread_id
        if args.valid_steer:
            focused_valid_steer(outer, trace, thread_id, fixture, summary)
        elif args.focused_hooks:
            focused_hooks(outer, trace, thread_id, fixture, summary)
        else:
            mark = len(trace.events)
            summary["firstStart"] = turn(outer, 2, thread_id, "FIRST_CASE")["value"]
            summary["firstCompleted"] = completed(trace, thread_id, mark)["value"]
            mark = len(trace.events)
            summary["wakeStart"] = turn(outer, "agentx:wake:1", thread_id, "<agentx>fixture idle wake</agentx>")["value"]
            summary["wakeCompleted"] = completed(trace, thread_id, mark)["value"]
            if not args.stable_api:
                mark = len(trace.events)
                summary["toolStart"] = turn(outer, 3, thread_id, "TOOL_CASE")["value"]
                request = trace.wait(lambda e: e["source"] == "outer" and e["value"].get("method") == "item/tool/call", mark)
                summary["serverRequest"] = request["value"]
                summary["oppositeDirectionSameId"] = outer.request(request["value"]["id"], "thread/read", {"threadId": thread_id})["value"]
                outer.send({"id": request["value"]["id"], "result": {"contentItems": [{"type": "inputText", "text": "fixed-fixture-tool-output"}], "success": True}})
                summary["toolCompleted"] = completed(trace, thread_id, mark)["value"]
            mark = len(trace.events)
            summary["holdStart"] = turn(outer, 4, thread_id, "HOLD_CASE")["value"]
            trace.wait(lambda e: e["source"] == "model", mark)
            summary["busyRead"] = outer.request("busy-read", "thread/read", {"threadId": thread_id, "includeTurns": True})["value"]
            summary["badSteer"] = outer.request("bad-steer", "turn/steer", {"threadId": thread_id, "expectedTurnId": "not-the-active-turn", "input": [{"type": "text", "text": "bad steer"}]})["value"]
            summary["busyStart"] = turn(outer, 5, thread_id, "BUSY_INPUT_CASE")["value"]
            fixture.gate.set()
            summary["busyCompleted"] = completed(trace, thread_id, mark)["value"]
            summary["int64Id"] = outer.request(9223372036854775807, "thread/read", {"threadId": thread_id})["value"]["id"]
            summary["stringId"] = outer.request("9223372036854775807", "thread/read", {"threadId": thread_id})["value"]["id"]
            second_thread = start_thread(outer, project, "second-thread", stable=args.stable_api)
            summary["secondThread"] = second_thread
            mark = len(trace.events)
            summary["secondThreadStart"] = turn(outer, "second-turn", second_thread, "SECOND_THREAD_CASE")["value"]
            summary["secondThreadCompleted"] = completed(trace, second_thread, mark)["value"]
            if args.transport == "unix":
                control = UnixWebSocketClient(sock, trace, "control")
                clients.append(control)
                control.initialize()
                summary["controlRead"] = control.request("control-read", "thread/read", {"threadId": thread_id, "includeTurns": True})["value"]
                mark = len(trace.events)
                summary["controlStart"] = turn(control, "control-wake", thread_id, "<agentx>second connection wake</agentx>")["value"]
                if "error" not in summary["controlStart"]:
                    summary["controlCompleted"] = completed(trace, thread_id, mark)["value"]
                summary["controlObservers"] = sorted({e["source"] for e in trace.events[mark:] if e["value"].get("method") == "turn/completed"})
        summary["outcome"] = "completed"
    except Exception as exc:
        summary["outcome"] = "incomplete"
        summary["error"] = repr(exc)
    finally:
        fixture.gate.set()
        for client in reversed(clients):
            client.stop()
        fixture.shutdown()
        summary["modelRequests"] = sum(e["source"] == "model" for e in trace.events)
        summary["hooks"] = [json.loads(s) for s in (root / "hooks.jsonl").read_text().splitlines()] if (root / "hooks.jsonl").exists() else []
        (root / "summary.json").write_text(json.dumps(summary, indent=2))
    print(json.dumps({k: summary[k] for k in ["outcome", "modelRequests", "error"] if k in summary}), flush=True)
    return 0 if summary["outcome"] == "completed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
