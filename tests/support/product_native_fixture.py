#!/usr/bin/env python3
"""Start native Codex with only a fixed loopback Responses model in a fresh namespace."""

import argparse
import http.server
import json
import os
import pathlib
import signal
import subprocess
import sys
import threading


class Responses(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_GET(self):
        data = b'{"object":"list","data":[{"id":"fixture-model","object":"model"}]}'
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        assert self.path == "/v1/responses"
        output = [x for x in body["input"] if x.get("type") == "function_call_output" and x.get("call_id") == "fixture_proposal"]
        if output:
            assert len(output) == 1 and output[0].get("output"), output
            item = {"id": "fixture_reply", "type": "message", "role": "assistant", "status": "completed",
                    "content": [{"type": "output_text", "text": "已提出将原声增益调整到 80% 的提案。请先预览，再决定是否接受。", "annotations": []}]}
        else:
            names = [tool.get("name") for tool in body.get("tools", [])]
            assert "propose_working_parameter" in names and "read_pinned_inputs" in names, names
            item = {"id": "fixture_call", "type": "function_call", "call_id": "fixture_proposal",
                    "name": "propose_working_parameter", "arguments": json.dumps(self.server.arguments, ensure_ascii=False)}
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Connection", "close")
        self.end_headers()
        response_id = "resp_reply" if output else "resp_tool"

        def event(kind, **fields):
            data = json.dumps({"type": kind, **fields}, ensure_ascii=False)
            self.wfile.write(f"event: {kind}\ndata: {data}\n\n".encode())
            self.wfile.flush()

        event("response.created", response={"id": response_id, "object": "response", "status": "in_progress", "output": []})
        event("response.output_item.added", output_index=0, item={**item, "status": "in_progress"})
        event("response.output_item.done", output_index=0, item=item)
        event("response.completed", response={"id": response_id, "object": "response", "status": "completed", "output": [item],
                                               "usage": {"input_tokens": 1, "output_tokens": 1, "total_tokens": 2}})


def run_inside(home, workspace):
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Responses)
    server.daemon_threads = True
    pin = json.loads(pathlib.Path("/fixture-pin.json").read_text())
    server.arguments = {"nodeId": pin["nodeId"], "inputId": pin["inputId"], "displayedValue": 80,
                        "summary": "将原声增益从 90% 调到 80%，保持其他元素不变。"}
    threading.Thread(target=server.serve_forever, daemon=True).start()
    (home / "config.toml").write_text(f'''model = "fixture-model"
model_provider = "fixture"
approval_policy = "never"
sandbox_mode = "read-only"
[features]
hooks = false
memories = false
shell_tool = false
[model_providers.fixture]
name = "Loopback fixture only"
base_url = "http://127.0.0.1:{server.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
request_max_retries = 0
stream_max_retries = 0
[analytics]
enabled = false
''')
    native = subprocess.Popen(["/opt/codex/bin/codex", "app-server"], cwd=workspace, env={
        "PATH": "/usr/bin:/bin", "HOME": str(home), "CODEX_HOME": str(home),
        "LANG": "C.UTF-8", "RUST_LOG": "error", "TMPDIR": "/tmp",
    })

    def end(*_):
        if native.poll() is None:
            native.terminate()

    signal.signal(signal.SIGTERM, end)
    signal.signal(signal.SIGINT, end)
    try:
        return native.wait()
    finally:
        server.shutdown()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native-package", type=pathlib.Path)
    parser.add_argument("--home", type=pathlib.Path, required=True)
    parser.add_argument("--workspace", type=pathlib.Path, required=True)
    parser.add_argument("--pin", type=pathlib.Path)
    parser.add_argument("--inside", action="store_true")
    args = parser.parse_args()
    if args.inside:
        return run_inside(args.home, args.workspace)
    home = args.home.resolve(strict=True)
    workspace = args.workspace.resolve(strict=True)
    package = args.native_package.resolve(strict=True)
    pin = args.pin.resolve(strict=True)
    assert home != workspace and not home.is_relative_to(workspace) and not workspace.is_relative_to(home)
    assert (package / "bin/codex").is_file() and not (home / "config.toml").exists(), "fresh dedicated native home required"
    command = ["/usr/bin/bwrap", "--unshare-all", "--die-with-parent", "--new-session", "--proc", "/proc", "--dev", "/dev",
               "--ro-bind", "/usr", "/usr", "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib64", "/lib64", "--symlink", "usr/bin", "/bin",
               "--tmpfs", "/tmp", "--ro-bind", str(package), "/opt/codex", "--bind", str(home), str(home),
               "--ro-bind", str(workspace), str(workspace), "--ro-bind", str(pin), "/fixture-pin.json",
               "--ro-bind", __file__, "/fixture.py", "--clearenv", "--chdir", str(workspace),
               "/usr/bin/python3", "-I", "/fixture.py", "--inside", "--home", str(home), "--workspace", str(workspace)]
    os.execv(command[0], command)


if __name__ == "__main__":
    sys.exit(main())
