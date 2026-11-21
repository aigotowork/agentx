#!/usr/bin/env python3
"""Check per-thread tool/hook identity env in one disposable native Codex server.

Reuse the native protocol probe's transport, trace and trusted-hook preparation.
Only a loopback fixed SSE model fixture is used. Its sole tool action runs an
absolute Python script in the temporary project that reads selected fake identity
variables and prints them. No user config, credentials or daemon is used.
"""

import argparse
import hashlib
import http.server
import json
import pathlib
import shlex
import sys
import tempfile
import threading

from probe_native_protocol import Client, Fixture, Trace, completed, install_config, turn


KEYS = ["SESSION_PROBE_ID", "AGENTX_PROCESS_ID", "AGENTX_INSTANCE_NAME", "CODEX_THREAD_ID", "CODEX_HOME"]


class EnvironmentFixture(Fixture):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
        event = self.server.trace.add("model", {"path": self.path, "body": body})
        users = [item for item in body.get("input", []) if item.get("role") == "user"]
        prompt = json.dumps(users[-1] if users else {})
        marker = next(case for case in self.server.cases if case in prompt)
        call_id = "call_environment_" + marker
        outputs = [item for item in body.get("input", []) if item.get("type") == "function_call_output" and item.get("call_id") == call_id]
        if outputs:
            item = {"id": "msg_" + marker, "type": "message", "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": "fixture-complete", "annotations": []}]}
        else:
            names = [tool.get("name") for tool in body.get("tools", [])]
            if "exec_command" not in names:
                self.server.trace.add("fixture-error", {"message": "native exec_command unavailable", "tools": names})
                self.send_error(500)
                return
            command = shlex.join([sys.executable, str(self.server.reader_script), marker])
            arguments = {"cmd": command, "shell": "/bin/sh", "login": False, "max_output_tokens": 1000, "yield_time_ms": 1000}
            item = {"id": "fc_" + marker, "type": "function_call", "name": "exec_command", "call_id": call_id, "arguments": json.dumps(arguments)}
        response_id = "resp_environment_" + str(event["seq"])
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Connection", "close")
        self.end_headers()
        frames = [
            ("response.created", {"response": {"id": response_id, "object": "response", "status": "in_progress", "output": []}}),
            ("response.output_item.added", {"output_index": 0, "item": {**item, "status": "in_progress"}}),
            ("response.output_item.done", {"output_index": 0, "item": item}),
            ("response.completed", {"response": {"id": response_id, "object": "response", "status": "completed", "output": [item], "usage": {"input_tokens": 1, "output_tokens": 1, "total_tokens": 2}}}),
        ]
        try:
            for kind, fields in frames:
                data = {"type": kind, **fields}
                self.wfile.write(("event: " + kind + "\ndata: " + json.dumps(data) + "\n\n").encode())
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--codex", required=True, type=pathlib.Path)
    args = parser.parse_args()
    native = args.codex.resolve(strict=True)
    root = pathlib.Path(tempfile.mkdtemp(prefix="zomi-session-environment-"))
    print("artifacts=" + str(root), flush=True)
    trace = Trace(root)
    fixture = http.server.ThreadingHTTPServer(("127.0.0.1", 0), EnvironmentFixture)
    fixture.daemon_threads = True
    fixture.trace = trace
    fixture.cases = ["ENV_CASE_A1", "ENV_CASE_B1", "ENV_CASE_A2"]
    threading.Thread(target=fixture.serve_forever, daemon=True).start()
    project, flags, env = install_config(root, fixture.server_port)
    env.update({"SESSION_PROBE_ID": "server", "AGENTX_PROCESS_ID": "server-execution", "AGENTX_INSTANCE_NAME": "server-participant"})
    reader = project / "read_selected_environment.py"
    reader.write_text("import json,os\nkeys=" + repr(KEYS) + "\nprint(json.dumps({key:os.environ.get(key) for key in keys}))\n")
    fixture.reader_script = reader
    hook = root / "hook.py"
    hook.write_text("import json,os,pathlib,sys\npayload=json.load(sys.stdin)\nkeys=" + repr(KEYS) + "\nrecord={'payload':payload,'environment':{key:os.environ.get(key) for key in keys}}\nwith pathlib.Path(sys.argv[1]).open('a') as stream: stream.write(json.dumps(record)+'\\n')\n")
    summary = {"root": str(root), "native": str(native), "sha256": hashlib.sha256(native.read_bytes()).hexdigest(), "cases": []}
    client = None
    preflight = None
    try:
        preflight = Client([str(native), *flags, "app-server"], env, project, trace, "preflight")
        preflight.initialize()
        listed = preflight.request("hooks", "hooks/list", {"cwds": [str(project)]})["value"]
        hooks = listed["result"]["data"][0]["hooks"]
        state = "{" + ",".join(json.dumps(item["key"]) + "={trusted_hash=" + json.dumps(item["currentHash"]) + "}" for item in hooks if item.get("command")) + "}"
        preflight.stop()
        preflight = None
        flags.extend(["-c", "hooks.state=" + state])
        client = Client([str(native), *flags, "app-server"], env, project, trace, "shared-native")
        summary["initialize"] = client.initialize(experimental=False)
        threads = {}
        for label in ["A", "B"]:
            identity = {"SESSION_PROBE_ID": "session-" + label, "AGENTX_PROCESS_ID": "execution-" + label, "AGENTX_INSTANCE_NAME": "participant-" + label}
            config = {"shell_environment_policy": {"inherit": "all", "set": identity}}
            response = client.request("thread-" + label, "thread/start", {"cwd": str(project), "model": "fixture-model", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "read-only", "config": config})["value"]
            summary["thread-" + label] = response
            if "error" in response:
                raise RuntimeError("thread-level config rejected: " + json.dumps(response["error"]))
            threads[label] = response["result"]["thread"]["id"]
        for marker in fixture.cases:
            label = marker.split("_")[-1][0]
            mark = len(trace.events)
            start = turn(client, marker, threads[label], marker)["value"]
            completion = completed(trace, threads[label], mark)["value"]
            command = trace.wait(lambda event: event["source"] == client.name and event["value"].get("method") == "item/completed" and event["value"].get("params", {}).get("threadId") == threads[label] and event["value"].get("params", {}).get("item", {}).get("id") == "call_environment_" + marker, after=mark)["value"]["params"]["item"]
            result = {"case": marker, "threadId": threads[label], "start": start, "completed": completion, "command": command, "toolEnvironment": json.loads(command["aggregatedOutput"]) if command["exitCode"] == 0 else None}
            summary["cases"].append(result)
            if result["toolEnvironment"] is None:
                raise RuntimeError("native tool did not produce selected environment output: " + marker)
            expected = {"SESSION_PROBE_ID": "session-" + label, "AGENTX_PROCESS_ID": "execution-" + label, "AGENTX_INSTANCE_NAME": "participant-" + label, "CODEX_THREAD_ID": threads[label], "CODEX_HOME": str(root / "home")}
            assert result["toolEnvironment"] == expected, result
        summary["outcome"] = "completed"
    except Exception as error:
        summary["outcome"] = "incomplete"
        summary["error"] = repr(error)
    finally:
        for connection in [client, preflight]:
            if connection:
                connection.stop()
        fixture.shutdown()
        hook_log = root / "hooks.jsonl"
        summary["hooks"] = [json.loads(row) for row in hook_log.read_text().splitlines()] if hook_log.exists() else []
        summary["modelRequests"] = sum(event["source"] == "model" for event in trace.events)
        (root / "summary.json").write_text(json.dumps(summary, indent=2))
    print(json.dumps({key: summary[key] for key in ["outcome", "modelRequests", "error"] if key in summary}), flush=True)
    return 0 if summary["outcome"] == "completed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
