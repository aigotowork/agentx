"""Two actual Codex app servers with a finite loopback Responses model."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import select
import shlex
import shutil
import signal
import tempfile
import threading
import uuid

from fixture import require, wait_for


def sse(response_id, item):
    events = [
        {"type": "response.created", "response": {"id": response_id}},
        {"type": "response.output_item.done", "item": item},
        {"type": "response.completed", "response": {"id": response_id, "usage": {
            "input_tokens": 0, "input_tokens_details": None, "output_tokens": 0,
            "output_tokens_details": None, "total_tokens": 0}}},
    ]
    return "".join(f"event: {event['type']}\ndata: {json.dumps(event)}\n\n" for event in events).encode()


class Model:
    """Only recognized inputs emit shell calls; the deployment and app servers are real."""

    ready_prompt = "cloud-acceptance-receiver-ready"
    launch_prompt = "cloud-acceptance-sender-start"
    message = "cloud-acceptance-native-request-雪"
    reply = "cloud-acceptance-native-related-reply"
    sender = "@west/sender"
    receiver = "@east/receiver"

    def __init__(self, fixture):
        self.fixture = fixture
        self.request_id, self.reply_id = str(uuid.uuid4()), str(uuid.uuid4())
        self.observed = {}
        self.errors = []
        self.lock = threading.Lock()
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                try:
                    require(self.path == "/v1/responses", f"Unexpected fixture model path: {self.path}")
                    size = int(self.headers.get("Content-Length", 0))
                    require(0 < size <= 2_000_000, "Missing or oversized Responses input")
                    event, item, evidence = owner.respond(json.loads(self.rfile.read(size)))
                    with owner.lock:
                        owner.observed.setdefault(event, []).append(evidence)
                    output = sse("resp_" + event.replace("-", "_"), item)
                    self.send_response(200)
                    self.send_header("Content-Type", "text/event-stream")
                    self.send_header("Content-Length", str(len(output)))
                    self.end_headers()
                    self.wfile.write(output)
                except (AssertionError, KeyError, ValueError) as error:
                    with owner.lock:
                        owner.errors.append(str(error))
                    self.send_error(500)

            def log_message(self, *_args):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *_args):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()
        self.fixture.write("model-observations.json", {"events": self.observed, "errors": self.errors})

    def respond(self, request):
        inputs = request["input"]
        user = [item for item in inputs if item.get("role") == "user"]
        latest = "\n".join(part.get("text", "") for part in user[-1].get("content", [])) if user else ""
        outputs = [item for item in inputs if item.get("type") == "function_call_output"]
        bootstrap = json.dumps(request, ensure_ascii=False)

        def say(event, reply, evidence):
            return event, {"type": "message", "role": "assistant", "id": "msg_" + event,
                           "content": [{"type": "output_text", "text": reply}]}, evidence

        def action(event, args):
            call_id = "call_" + event
            output = next((item["output"] for item in reversed(outputs) if item.get("call_id") == call_id), None)
            if output is not None:
                serialized = json.dumps(output, ensure_ascii=False)
                if f"{event}-shell-ok" not in serialized:
                    with self.lock:
                        self.observed.setdefault(event + "-tool-failed", []).append(output)
                    raise AssertionError(f"Actual Codex tool did not complete {event}; see model observations")
                return say(event + "-complete", "Tool finished", output)
            command = self.tool_command(args) + f" && printf '%s' {event}-shell-ok"
            return event, {"type": "function_call", "call_id": call_id, "name": "exec_command",
                           "arguments": json.dumps({"cmd": command, "login": False})}, latest

        if "Generate a concise, single-line task title" in bootstrap:
            return say("title", '{"title":"Cloud acceptance"}', None)
        if self.reply in latest:
            require(self.receiver in latest and self.sender in latest and "reply-to #" in latest,
                    "Sender's Codex must observe the exact cross-host reply envelope")
            return say("reply-observed-by-sender", "Reply reached sender", latest)
        if self.message in latest:
            before = latest.partition(self.message)[0]
            reference = re.search(r"request #([0-9]+)\b", before)
            require(reference and self.sender in before and self.receiver in before,
                    "Receiver's Codex must observe the exact request and its numeric reply reference")
            return action("reply", ["send", self.sender, "--request-id", self.reply_id,
                                    "--intent", "ack", "--reply-to", reference[1], "--thread", "native-proof", "--", self.reply])
        if self.launch_prompt in latest:
            return action("request", ["send", self.receiver, "--request-id", self.request_id,
                                      "--intent", "request", "--thread", "native-proof", "--", self.message])
        if self.ready_prompt in latest:
            return say("receiver-ready", "Ready for the cross-host request", latest)
        return self.unexpected_turn(say, latest)

    def tool_command(self, args):
        return shlex.join([str(self.fixture.candidate), *args])

    def unexpected_turn(self, say, latest):
        raise AssertionError("Unexpected model turn: no known acceptance prompt in the latest user input")

    def seen(self, event):
        with self.lock:
            require(not self.errors, "Local model visitor failed; see model-observations.json")
            return self.observed.get(event)

    def configuration(self, port=None, restrict_network=False):
        mode = "workspace-write" if restrict_network else "danger-full-access"
        sandbox = "[sandbox_workspace_write]\nnetwork_access = false\n" if restrict_network else ""
        return f'''model = "gpt-5.5"
model_provider = "cloud_acceptance"
approval_policy = "never"
sandbox_mode = "{mode}"

{sandbox}

[model_providers.cloud_acceptance]
name = "Cloud acceptance loopback model"
base_url = "http://127.0.0.1:{port or self.server.server_port}/v1"
env_key = "CLOUD_ACCEPTANCE_MODEL_KEY"
wire_api = "responses"
requires_openai_auth = false
'''

    def configure(self, env):
        codex_home = Path(env["CODEX_HOME"])
        (codex_home / "config.toml").write_text(self.configuration())
        env["CLOUD_ACCEPTANCE_MODEL_KEY"] = "local-model-no-real-credential"


def stop_private_processes(fixture, roots):
    """Use pidfds to signal only the detached processes with a private fixture home."""
    targets = {os.fsencode(f"HOME={root}") for root in roots}
    stopped = []
    active = []
    for path in Path("/proc").iterdir():
        if not path.name.isdecimal():
            continue
        fd = None
        try:
            if path.stat().st_uid != os.getuid():
                continue
            cmdline = (path / "cmdline").read_bytes()
            if b"codex" not in cmdline and b"cloud-host" not in cmdline:
                continue
            fd = os.pidfd_open(int(path.name))
            if targets.isdisjoint((path / "environ").read_bytes().split(b"\0")):
                continue
            signal.pidfd_send_signal(fd, signal.SIGTERM)
            active.append((int(path.name), fd))
            fd = None
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            pass
        finally:
            if fd is not None:
                os.close(fd)
    for pid, fd in active:
        try:
            poller = select.poll()
            poller.register(fd, select.POLLIN)
            if not poller.poll(4000):
                signal.pidfd_send_signal(fd, signal.SIGKILL)
            require(poller.poll(2000), f"Private native process {pid} did not exit")
            stopped.append(pid)
        except ProcessLookupError:
            stopped.append(pid)
        finally:
            os.close(fd)
    fixture.result["detached_host_pids_exited"] = stopped


def run(fixture, credential, certificate=None):
    scope = "native-agents"
    admin_path = fixture.admin + f"/scopes/{scope}"
    origin = fixture.public_url if certificate else fixture.url
    hosts = {host: {**fixture.environment(host), "AGENTX_SCOPE_URL": origin + f"/scopes/{scope}",
                    "AGENTX_SCOPE_TOKEN": credential, "AGENTX_HOST_NAME": host} for host in ("east", "west")}
    codex = shutil.which("codex", path=hosts["east"]["PATH"])
    require(codex, "Native acceptance requires a real Codex CLI on PATH")
    version = fixture.command("codex-version", [codex, "--version"], hosts["east"]).stdout.strip()
    if certificate:
        untrusted = fixture.command("native-list-untrusted", [fixture.candidate, "list", "--json"], hosts["east"], check=False)
        require(untrusted.returncode != 0, "Native client accepted a private HTTPS certificate without its CA")
        for env in hosts.values():
            env["SSL_CERT_FILE"] = str(certificate)

    def child(host, label, *args):
        return fixture.command(label, [fixture.candidate, *args], hosts[host],
                               cwd=fixture.runtime / host / "workspace", timeout=65)

    short_sockets = tempfile.TemporaryDirectory(prefix="agentx-cloud-native-", dir="/tmp")
    for host, env in hosts.items():
        socket_root = Path(short_sockets.name) / host
        socket_root.mkdir(mode=0o700)
        env["TMPDIR"] = str(socket_root)
    try:
        with Model(fixture) as model:
            for env in hosts.values():
                model.configure(env)
                private_bin = Path(env["HOME"]).parent / "bin"
                private_bin.mkdir()
                (private_bin / "agentx").symlink_to(fixture.candidate)
                env["PATH"] = str(private_bin) + os.pathsep + env["PATH"]
            child("east", "launch-receiver", "launch", "--tool", "codex", "--name", "receiver",
                  "--headless", "--go", "--json", "--agentx-prompt", model.ready_prompt)
            wait_for("East Codex reaches its initial turn", lambda: model.seen("receiver-ready"), 30)
            child("west", "launch-sender", "launch", "--tool", "codex", "--name", "sender",
                  "--headless", "--go", "--json", "--agentx-prompt", model.launch_prompt)
            wait_for("West's real Codex observes East's related reply", lambda: model.seen("reply-observed-by-sender"), 45)
            wait_for("Both real Codex tool calls finish", lambda: model.seen("request-complete") and model.seen("reply-complete"))
            agents = fixture.expect("GET", admin_path + "/agents", fixture.admin_token)["items"]
            require({(agent["host"], agent["name"]) for agent in agents} == {("east", "receiver"), ("west", "sender")}
                    and all(set(agent) == {"host", "name", "incarnation"} for agent in agents),
                    "Admin inventory must agree with the two native actors and omit local process data")
            listing = json.loads(child("west", "native-list", "list", "--json").stdout)
            require(sorted(listing, key=lambda agent: agent["name"]) == sorted(agents, key=lambda agent: agent["name"]),
                    "Native list and admin HTTP must agree on exact Agents")

            def confirmed(request_id):
                route = f"/scopes/{scope}/api/v1/operations/{request_id}"
                receipt = fixture.expect("GET", route, credential)
                return receipt["message"] if receipt["status"] == "confirmed" else False

            request = wait_for("Receiver host confirms Provider acceptance", lambda: confirmed(model.request_id))
            reply = wait_for("Sender host confirms Provider acceptance", lambda: confirmed(model.reply_id))
            require(request["from"] == reply["to"] and request["to"] == reply["from"]
                    and reply["reply_to"] == request["id"] and request["thread"] == reply["thread"] == "native-proof",
                    "Scope receipts must preserve the exact native request, reply and thread relation")
            return {"codex_version": version, "model": "scripted localhost Responses stream", "agents": agents,
                    "transport": "verified private HTTPS" if certificate else "loopback HTTP",
                    "messages": [request, reply], "provider_evidence": "model-observations.json"}
    finally:
        try:
            stop_private_processes(fixture, [env["HOME"] for env in hosts.values()])
        finally:
            for host in hosts:
                for log in (fixture.runtime / host / "scope").rglob("*.log"):
                    (fixture.directory / f"native-{host}-{log.name}").write_text(fixture.redact(log.read_text(errors="replace")))
            short_sockets.cleanup()
