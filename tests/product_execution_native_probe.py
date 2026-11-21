#!/usr/bin/env python3
"""Check dedicated product execution evidence against native Codex and a fixed loopback model.

Pass the explicit platform package directory containing bin/codex. Requires Linux
and bubblewrap. The process and network namespaces, Codex home and project are
disposable. Stdout is the evidence summary. This does not exercise AgentX or MCP.
"""

import argparse
import hashlib
import http.server
import json
import os
import pathlib
import runpy
import subprocess
import sys
import tempfile
import threading

REPO = pathlib.Path(__file__).resolve().parents[1]


def probe(native, wire):
    helpers = runpy.run_path(str(REPO / ".docs/codex-activation/.tools/probe_native_protocol.py"))
    with tempfile.TemporaryDirectory(prefix="product-execution-") as directory:
        root = pathlib.Path(directory)
        trace = helpers["Trace"](root)
        fixture = http.server.ThreadingHTTPServer(("127.0.0.1", 0), helpers["Fixture"])
        fixture.daemon_threads = True
        fixture.trace, fixture.gate = trace, threading.Event()
        threading.Thread(target=fixture.serve_forever, daemon=True).start()
        project, _, env = helpers["install_config"](root, fixture.server_port)
        env["HOME"] = env["CODEX_HOME"]
        tool = {
            "type": "function", "name": "fixture_echo", "description": "Fixed loopback probe",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
        }
        client = None
        try:
            client = helpers["Client"]([str(native), "-c", "features.hooks=false", "app-server"], env, project, trace, "runner")
            initialize = client.initialize()
            assert initialize["codexHome"] == env["CODEX_HOME"], initialize
            def start_root(key, tools):
                started = client.request(key, "thread/start", {
                    "cwd": str(project), "model": "fixture-model", "modelProvider": "fixture",
                    "approvalPolicy": "never", "sandbox": "read-only",
                    "allowProviderModelFallback": False, "dynamicTools": tools,
                })["value"]["result"]
                thread = started["thread"]
                assert thread["cwd"] == str(project) and thread["status"]["type"] == "idle", thread
                return thread

            def installed(thread, tools):
                rollout = pathlib.Path(thread["path"]).resolve()
                assert rollout.is_relative_to(pathlib.Path(env["CODEX_HOME"]) / "sessions")
                if not rollout.is_file():
                    return False
                with rollout.open() as entries:
                    first = entries.readline()
                if not first:
                    return False
                meta = json.loads(first)
                return meta["type"] == "session_meta" and meta["payload"]["id"] == thread["id"] and meta["payload"].get("dynamic_tools") == tools

            def name_root(key, thread, tools):
                named = client.request(key, "thread/name/set", {"threadId": thread["id"], "name": "Product execution probe"})["value"]
                assert "result" in named, named
                assert installed(thread, tools), "naming the persisted thread must materialize the native installation"

            thread = start_root("root", [tool])
            thread_id = thread["id"]

            # Check whether the native installation is independently visible
            # before any turn. A returned path is not itself a persisted record.
            read = client.request("initial-read", "thread/read", {"threadId": thread_id, "includeTurns": False})["value"]
            before_turn = installed(thread, [tool])
            name_root("name", thread, [tool])
            catalog_receipt = None
            if wire is not None:
                data = wire.read_bytes()
                catalog = json.loads(data)["installation"]
                catalog_tools = [{
                    "type": "function", "name": entry["name"], "description": "Product execution tool",
                    "inputSchema": entry["inputSchema"],
                } for entry in catalog["declarations"]]
                assert [entry["name"] for entry in catalog_tools] == catalog["names"]
                catalog_thread = start_root("catalog-root", catalog_tools)
                name_root("catalog-name", catalog_thread, catalog_tools)
                catalog_receipt = {"wireSha256": hashlib.sha256(data).hexdigest(), "nativeNames": catalog["names"]}
            assert not any(event["source"] == "model" for event in trace.events)
            content = "TOOL_CASE: exact frozen product input"
            client_id = "agentx-inbox:fixture-batch"
            after = len(trace.events)
            turn = helpers["turn"](client, "input", thread_id, content, client_id=client_id)["value"]["result"]["turn"]["id"]

            def native_event(event, method):
                data = event["value"]
                return event["source"] == "runner" and data.get("method") == method and data.get("params", {}).get("threadId") == thread_id

            def completed_item(item_type):
                return trace.wait(lambda event: native_event(event, "item/completed")
                                  and event["value"]["params"]["turnId"] == turn
                                  and event["value"]["params"]["item"]["type"] == item_type, after)

            input_event = completed_item("userMessage")
            item = input_event["value"]["params"]["item"]
            assert item["id"] and item["clientId"] == client_id and len(item["content"]) == 1, item
            assert item["content"][0]["type"] == "text" and item["content"][0]["text"] == content, item
            request_event = trace.wait(lambda event: native_event(event, "item/tool/call"), after)
            request = request_event["value"]
            call = request["params"]
            assert input_event["seq"] < request_event["seq"], request_event
            assert type(request["id"]) in (int, str) and call["callId"], request
            assert call["turnId"] == turn and call["tool"] == tool["name"] and call["arguments"] == {}, call
            model = trace.wait(lambda event: event["source"] == "model", after)["value"]
            declarations = [value for value in model["body"]["tools"] if value.get("name") == tool["name"]]
            assert len(declarations) == 1 and declarations[0]["parameters"] == tool["inputSchema"], declarations

            response = [{"type": "inputText", "text": "fixed tool result"}]
            client.send({"id": request["id"], "result": {"contentItems": response, "success": True}})
            written = completed_item("dynamicToolCall")
            completion = written["value"]["params"]["item"]
            assert completion["id"] == call["callId"] and completion["contentItems"] == response and completion["success"], completion
            reply_event = completed_item("agentMessage")
            reply = reply_event["value"]["params"]["item"]
            terminal_event = trace.wait(lambda event: native_event(event, "turn/completed") and event["value"]["params"]["turn"]["id"] == turn, after)
            terminal = terminal_event["value"]["params"]["turn"]
            assert written["seq"] < reply_event["seq"] < terminal_event["seq"]
            assert terminal["status"] == "completed" and reply["id"] and reply["text"] == "fixture-ok", terminal
            assert installed(thread, [tool]), "native rollout must retain the exact installed tool after the turn"
            print(json.dumps({
                "codex": thread["cliVersion"], "sha256": hashlib.sha256(native.read_bytes()).hexdigest(),
                "initialThreadRead": "result" in read, "initialNativeToolRecord": before_turn,
                "preTurnRecordAfterName": True, "preTurnCatalogRecord": catalog_receipt,
                "thread": thread_id, "turn": turn, "inputItem": item["id"],
                "serverRequestId": request["id"], "nativeToolCallId": call["callId"], "replyItem": reply["id"],
                "terminal": terminal["status"], "modelRequests": sum(event["source"] == "model" for event in trace.events),
            }, sort_keys=True), flush=True)
        finally:
            if client is not None:
                client.stop()
            fixture.shutdown()
            fixture.server_close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native-package", required=True, type=pathlib.Path)
    parser.add_argument("--tool-wire", type=pathlib.Path, help="Optional Hypit runner wire export to verify before any turn")
    args = parser.parse_args()
    package = args.native_package.resolve(strict=True)
    native = package / "bin/codex"
    if package == pathlib.Path("/opt") and os.environ.get("AGENTX_PRODUCT_PROBE_ISOLATED") == "v1":
        probe(native, args.tool_wire)
        return 0
    assert native.is_file(), "Pass the native platform package directory"
    command = [
        "bwrap", "--ro-bind", "/", "/", "--unshare-all", "--die-with-parent", "--new-session",
        "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/home", "--tmpfs", "/run", "--tmpfs", "/tmp",
        "--ro-bind", str(REPO / "tests/product_execution_native_probe.py"), "/tmp/work/tests/product_execution_native_probe.py",
        "--ro-bind", str(REPO / ".docs/codex-activation/.tools/probe_native_protocol.py"), "/tmp/work/.docs/codex-activation/.tools/probe_native_protocol.py",
        "--ro-bind", str(package), "/opt",
    ]
    if args.tool_wire is not None:
        command += ["--ro-bind", str(args.tool_wire.resolve(strict=True)), "/tmp/runner-wire.json"]
    command += [
        "--clearenv", "--setenv", "PATH", "/usr/bin:/bin",
        "--setenv", "AGENTX_PRODUCT_PROBE_ISOLATED", "v1", "--chdir", "/tmp",
        "/usr/bin/python3", "-I", "/tmp/work/tests/product_execution_native_probe.py", "--native-package", "/opt",
    ]
    if args.tool_wire is not None:
        command += ["--tool-wire", "/tmp/runner-wire.json"]
    return subprocess.run(command, check=False, timeout=60).returncode


if __name__ == "__main__":
    sys.exit(main())
