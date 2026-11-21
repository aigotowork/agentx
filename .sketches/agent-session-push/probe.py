#!/usr/bin/env python3
"""Reproduce isolated session-control and stdio MCP probes with a fixed local model."""
import argparse
import http.server
import importlib.util
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("native_probe", HERE.parent.parent / ".docs/codex-activation/.tools/probe_native_protocol.py")
NATIVE = importlib.util.module_from_spec(SPEC)
sys.dont_write_bytecode = True
SPEC.loader.exec_module(NATIVE)


def serve_mcp(root):
    """A per-process Unix inbox drives real protocol notifications over stdout."""
    pid = os.getpid()
    sock = root / f"m-{pid}.sock"
    out = threading.Lock()

    def record(info):
        with (root / "mcp.jsonl").open("a") as file:
            file.write(json.dumps({"pid": pid, **info}) + "\n")

    def emit(msg):
        with out:
            print(json.dumps({"jsonrpc": "2.0", **msg}), flush=True)

    def inbox(server):
        while True:
            conn, _ = server.accept()
            with conn:
                message = json.loads(conn.recv(1024))
                emit(message)
                record({"sent": message["method"]})
                conn.sendall(b"written")

    with socket.socket(socket.AF_UNIX) as server:
        server.bind(str(sock))
        server.listen()
        threading.Thread(target=inbox, args=(server,), daemon=True).start()
        for line in sys.stdin:
            msg = json.loads(line)
            method = msg.get("method")
            record({"received": method})
            if "id" not in msg:
                continue
            if method == "initialize":
                result = {"protocolVersion": "2025-11-25", "serverInfo": {"name": "probe", "version": "1"},
                          "capabilities": {"tools": {"listChanged": True}, "resources": {"subscribe": True}}}
            elif method == "tools/list":
                result = {"tools": [{"name": "probe_echo", "description": "Isolated fixture", "inputSchema": {"type": "object"}}]}
            elif method == "resources/list":
                result = {"resources": [{"uri": "agentx://inbox", "name": "Fixture Inbox"}]}
            elif method in ("ping", "resources/subscribe"):
                result = {}
            else:
                emit({"id": msg["id"], "error": {"code": -32601, "message": "Unknown method"}})
                continue
            emit({"id": msg["id"], "result": result})


class Model(NATIVE.Fixture):
    """Support both Codex Responses and OpenAI-compatible chat completions."""
    def do_POST(self):
        if not self.path.endswith("/chat/completions"):
            return super().do_POST()
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.server.trace.add("model", {"path": self.path, "body": body})
        prompt = json.dumps([x for x in body["messages"] if x["role"] == "user"][-1])
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Connection", "close")
        self.end_headers()
        if "HOLD_CASE" in prompt:
            self.server.gate.wait(12)
        for delta, reason in [({"role": "assistant", "content": "fixture-ok"}, None), ({}, "stop")]:
            data = {"id": "fixture", "object": "chat.completion.chunk", "created": 1, "model": "fixture-model",
                    "choices": [{"index": 0, "delta": delta, "finish_reason": reason}]}
            self.wfile.write(("data: " + json.dumps(data) + "\n\n").encode())
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()


def model_request(trace, token, after):
    return trace.wait(lambda e: e["source"] == "model" and token in json.dumps(e["value"]["body"]), after)


def no_turn(trace, after, timeout=0.8):
    try:
        trace.wait(lambda e: e["source"] == "model" or
                   e["value"].get("method") == "turn/started" and e["source"] == "native", after, timeout)
    except TimeoutError:
        return
    raise AssertionError("Unexpected turn without a prompt")


def checked(event):
    value = event["value"]
    if "error" in value:
        raise RuntimeError(value["error"])
    return value["result"]


def native_mcp(codex, root, project, env, trace):
    config = Path(env["CODEX_HOME"]) / "config.toml"
    with config.open("a") as file:
        args = [str(Path(__file__).resolve()), "--mcp-child", str(root)]
        file.write(f'\n[mcp_servers.probe]\ncommand = {json.dumps(sys.executable)}\nargs = {json.dumps(args)}\n')
    try:
        native = NATIVE.Client([codex, "app-server"], env, project, trace, "native")
        try:
            native.initialize()
            rejected = native.request("event-unsupported", "mcpServer/event/stream/start", {
                "threadId": "00000000-0000-0000-0000-000000000001", "server": "probe",
                "subscriptionId": "probe-1", "name": "message.received", "arguments": {},
            })["value"]["error"]["message"]
            assert "only supported for hosted apps" in rejected, rejected
            baseline = []
            for i in (1, 2):
                target = NATIVE.start_thread(native, project, f"thread-{i}", stable=True)
                after = len(trace.events)
                checked(NATIVE.turn(native, i, target, f"MCP_BASELINE_{i}"))
                req = model_request(trace, f"MCP_BASELINE_{i}", after)
                assert "probe_echo" in json.dumps(req["value"]["body"].get("tools", []))
                NATIVE.completed(trace, target, after)
                baseline.append((target, sorted(p.name for p in root.glob("m-*.sock"))))
            assert baseline[0][0] != baseline[1][0]
            after = len(trace.events)
            assert len(baseline[0][1]) == 1 and baseline[0][1] == baseline[1][1], baseline
            notifications = [
                {"method": "notifications/tools/list_changed", "params": {}},
                {"method": "notifications/agentx/message", "params": {"content": "IDLE_MCP_NOTIFICATION"}},
            ]
            for message in notifications:
                with socket.socket(socket.AF_UNIX) as sender:
                    sender.connect(str(root / baseline[0][1][0]))
                    sender.sendall(json.dumps(message).encode())
                    assert sender.recv(64) == b"written"
                no_turn(trace, after)
            checked(NATIVE.turn(native, "explicit", baseline[0][0], "EXPLICIT_NATIVE_WAKE"))
            requested = model_request(trace, "EXPLICIT_NATIVE_WAKE", after)
            assert "IDLE_MCP_NOTIFICATION" not in json.dumps(requested["value"])
            NATIVE.completed(trace, baseline[0][0], after)
            print(json.dumps({"case": "native_mcp", "threads": baseline, "customEventRejection": rejected,
                              "noticesWritten": [n["method"] for n in notifications],
                              "noticeStartedTurn": False, "noticeInNextPrompt": False,
                              "explicitSameThreadWake": True}), flush=True)
        finally:
            native.stop()
    finally:
        config.write_text(config.read_text().split("\n[mcp_servers.probe]", 1)[0])


class ACP(NATIVE.Client):
    def send(self, value):
        super().send({"jsonrpc": "2.0", **value})

    def initialize(self, version=1):
        info = {"name": "isolated-session-probe", "version": "1"}
        fields = {"capabilities": {}, "info": info} if version == 2 else {"clientCapabilities": {}, "clientInfo": info}
        result = checked(self.request("initialize", "initialize", {"protocolVersion": version, **fields}))
        assert result["protocolVersion"] == version, result
        return result

    def new(self, request_id, project):
        return checked(self.request(request_id, "session/new", {"cwd": str(project), "mcpServers": []}))["sessionId"]

    def prompt(self, request_id, session, text):
        self.send({"id": request_id, "method": "session/prompt", "params": {
            "sessionId": session, "prompt": [{"type": "text", "text": text}]}})

    def finish(self, request_id, session, after):
        self.trace.wait(lambda e: e["source"] == self.name and e["value"].get("method") == "session/update"
            and e["value"]["params"]["sessionId"] == session
            and e["value"]["params"]["update"].get("sessionUpdate") == "agent_message_chunk"
            and "fixture-ok" in json.dumps(e["value"]), after)
        outcome = self.trace.wait(lambda e: e["source"] == self.name and e["value"].get("id") == request_id
                                      and "method" not in e["value"], after)
        assert checked(outcome)["stopReason"] == "end_turn", outcome


def acp_case(name, command, project, env, trace, server):
    def connect(suffix):
        return ACP(command, env, project, trace, name + suffix)
    def send(client, request_id, session, token):
        after = len(trace.events)
        client.prompt(request_id, session, token)
        req = model_request(trace, token, after)
        client.finish(request_id, session, after)
        return req["value"]["body"]
    client = connect("")
    try:
        caps = client.initialize().get("agentCapabilities", {})
        assert caps.get("loadSession") is True, caps
        a = client.new("new-a", project)
        b = client.new("new-b", project)
        assert a != b
        send(client, 1, a, "ORIGINAL_A_MARKER")
        body = send(client, 2, b, "ISOLATED_B_MARKER")
        assert "ORIGINAL_A_MARKER" not in json.dumps(body), body
        body = send(client, 3, a, "IDLE_WAKE_A_MARKER")
        assert "ORIGINAL_A_MARKER" in json.dumps(body) and "ISOLATED_B_MARKER" not in json.dumps(body), body
        server.gate.clear()
        after = len(trace.events)
        client.prompt(4, a, "HOLD_CASE")
        model_request(trace, "HOLD_CASE", after)
        queued = "BUSY_QUEUED_MARKER"  # Pending in the client; never sent during active work.
        no_turn(trace, len(trace.events), 0.4)
        server.gate.set()
        client.finish(4, a, after)
        send(client, 5, a, queued)
    finally:
        client.stop()
    client = connect("-reopened")
    try:
        client.initialize()
        checked(client.request("load", "session/load", {"sessionId": a, "cwd": str(project), "mcpServers": []}))
        body = send(client, 6, a, "AFTER_RECONNECT_MARKER")
        assert "BUSY_QUEUED_MARKER" in json.dumps(body) and "ISOLATED_B_MARKER" not in json.dumps(body)
        send(client, 7, a, "REPEATED_EVENT_ID_42")
        body = send(client, 8, a, "REPEATED_EVENT_ID_42")
        latest_messages = body.get("input", body.get("messages", []))
        assert sum("REPEATED_EVENT_ID_42" in json.dumps(item)
                   for item in latest_messages if item.get("role") == "user") == 2
    finally:
        client.stop()
    print(json.dumps({"case": name, "protocolVersion": 1, "sameSessionWake": True, "twoSessionsIsolated": True,
                      "clientQueueAfterTurn": True, "cleanRestartAndLoad": True, "explicitDuplicateCreatesTwoTurns": True}),
          flush=True)


def acp_v2_case(command, project, env, trace, server):
    def connect(name):
        client = ACP(command, env, project, trace, name)
        try:
            client.initialize(2)
            return client
        except Exception:
            client.stop()
            raise

    client = connect("codex_acp_v2")
    try:
        session = client.new("v2-new", project)
        params = {"sessionId": session, "idempotencyKey": "fixture-event-42",
                  "prompt": [{"type": "text", "text": "HOLD_CASE_V2"}]}
        server.gate.clear()
        first = len(trace.events)
        accepted = checked(client.request("v2-first", "session/prompt", params))
        message_id = accepted["messageId"]
        model_request(trace, "HOLD_CASE_V2", first)
        # The model is still blocked. v1 would be waiting for the turn response here.
        same = checked(client.request("v2-duplicate", "session/prompt", params))
        assert same["messageId"] == message_id, same
        server.gate.set()
        trace.wait(lambda e: e["source"] == client.name and e["value"].get("method") == "session/update"
                   and e["value"].get("params", {}).get("update", {}).get("sessionUpdate") == "state_update"
                   and '"idle"' in json.dumps(e["value"]["params"]["update"]), first)
    finally:
        server.gate.set()
        client.stop()
    client = connect("codex_acp_v2-reopened")
    try:
        checked(client.request("v2-resume", "session/resume", {"sessionId": session, "cwd": str(project), "mcpServers": []}))
        after = len(trace.events)
        restored = checked(client.request("v2-retry", "session/prompt", params))
        assert restored["messageId"] == message_id, (restored, accepted)
        no_turn(trace, after)
    finally:
        client.stop()
    print(json.dumps({"case": "codex_acp_v2", "protocolVersion": 2, "receiptBeforeModelFinishes": True,
                      "sameKeySameMessage": True, "sameKeyAfterCleanRestart": True}), flush=True)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--codex", required=True)
    p.add_argument("--codex-acp")
    p.add_argument("--opencode")
    p.add_argument("--skip-native", action="store_true")
    p.add_argument("--codex-acp-v2", action="store_true")
    p.add_argument("--only-v2", action="store_true", help="Run just the Codex ACP v2 Preview case")
    args = p.parse_args()
    if (args.codex_acp_v2 or args.only_v2) and not args.codex_acp:
        p.error("The ACP v2 probe requires --codex-acp")
    root = Path(tempfile.mkdtemp(prefix="agent-push-"))
    print("artifacts=" + str(root), flush=True)
    trace = NATIVE.Trace(root)
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), Model) as server:
        server.trace, server.gate = trace, threading.Event()
        server.gate.set()
        threading.Thread(target=server.serve_forever, daemon=True).start()
        project, _, env = NATIVE.install_config(root, server.server_port)
        env.update({"HOME": str(root), "PATH": os.environ["PATH"], "CODEX_PATH": args.codex,
                    "XDG_CONFIG_HOME": str(root / "xdg-config"), "XDG_DATA_HOME": str(root / "xdg-data"),
                    "XDG_CACHE_HOME": str(root / "xdg-cache"), "XDG_STATE_HOME": str(root / "xdg-state"),
                    "OPENCODE_DISABLE_AUTOUPDATE": "true", "OPENCODE_DISABLE_MODELS_FETCH": "true",
                    "OPENCODE_DISABLE_LSP_DOWNLOAD": "true", "OPENCODE_DISABLE_DEFAULT_PLUGINS": "true",
                    "OPENCODE_MODELS_URL": f"http://127.0.0.1:{server.server_port}"})
        failures = []
        cases = []
        if not args.skip_native and not args.only_v2:
            cases.append(("native", lambda: native_mcp(args.codex, root, project, env, trace)))
        if args.codex_acp:
            if not args.only_v2:
                cases.append(("codex_acp", lambda: acp_case("codex_acp", [args.codex_acp], project, env, trace, server)))
            if args.codex_acp_v2 or args.only_v2:
                cases.append(("codex_acp_v2", lambda: acp_v2_case([args.codex_acp], project, env, trace, server)))
        if args.opencode and not args.only_v2:
            config = {"model": "fixture/fixture-model", "autoupdate": False, "enabled_providers": ["fixture"],
                "permission": "deny", "provider": {"fixture": {"npm": "@ai-sdk/openai-compatible", "name": "Loopback only",
                "options": {"baseURL": f"http://127.0.0.1:{server.server_port}/v1", "apiKey": "local-fixture"},
                "models": {"fixture-model": {"name": "Fixture Model", "limit": {"context": 65536, "output": 4096}}}}}}
            env["OPENCODE_CONFIG_CONTENT"] = json.dumps(config)
            cases.append(("opencode_acp", lambda: acp_case("opencode_acp", [args.opencode, "acp"], project, env, trace, server)))
        for name, run in cases:
            try:
                run()
            except Exception as exc:
                import traceback
                traceback.print_exc()
                failures.append((name, str(exc)))
        server.shutdown()
        if failures:
            sys.exit(str(failures))


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--mcp-child":
        serve_mcp(Path(sys.argv[2]))
    else:
        main()
