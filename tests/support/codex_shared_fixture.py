"""Disposable real-process fixture for shared-runtime acceptance.

The integration runner explicitly supplies both executables. This module reuses
the feasibility probe's JSON client and loopback model, strips inherited state,
and owns only paths/processes created in its temporary tree. A real native
service must not be started until its owner has proved the service mechanism
honors these isolated paths; env isolation alone is not such proof.
"""

import importlib.util
import http.server
import json
import os
import pathlib
import shutil
import signal
import subprocess
import tempfile
import threading
import time


REPO = pathlib.Path(__file__).resolve().parents[2]
PROBE_PATH = REPO / ".docs/codex-activation/.tools/probe_native_protocol.py"
SPEC = importlib.util.spec_from_file_location("codex_native_probe", PROBE_PATH)
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


class TerminalBridgeProcess:
    """Own the frontend socket and the AgentX process for one connection."""
    def __init__(self, command, env, cwd, trace, name):
        self.directory = tempfile.TemporaryDirectory(prefix="tui-", dir=trace.root)
        self.socket = pathlib.Path(self.directory.name) / "bridge.sock"
        stderr = trace.root / (name + ".stderr")
        with stderr.open("w") as errors:
            self.process = subprocess.Popen(command, env={**env, "AGENTX_CODEX_TUI_SOCKET": str(self.socket)}, cwd=cwd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=errors, start_new_session=True)
        try:
            deadline = time.monotonic() + 35
            while not self.socket.is_socket():
                if self.process.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError("AgentX TUI socket did not start: " + stderr.read_text())
                threading.Event().wait(0.03)
        except Exception:
            self.stop()
            raise

    def stop(self):
        try:
            try:
                self.process.wait(timeout=4)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                self.process.wait(timeout=5)
        finally:
            self.directory.cleanup()


class TerminalSocketClient(PROBE.UnixWebSocketClient):
    """Enter the real bridge through the same connection as the native TUI."""

    def __init__(self, command, env, cwd, trace, name):
        self.bridge = TerminalBridgeProcess(command, env, cwd, trace, name)
        self.process = self.bridge.process
        try:
            super().__init__(self.bridge.socket, trace, name)
            self.connected = True
        except Exception:
            self.bridge.stop()
            raise

    def stop(self):
        try:
            if self.connected:
                self.connected = False
                try:
                    super().stop()
                except OSError:
                    self.socket.close()
        finally:
            self.bridge.stop()


class CollaborationModel(PROBE.Fixture):
    """Fixed parent/child tool script; no arbitrary generated commands."""

    def do_POST(self):
        if not any(getattr(self.server, name, False) for name in ["collaboration", "controls", "environment", "heterogeneous", "direct_terminal"]):
            return super().do_POST()
        body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
        event = self.server.trace.add("model", {"path": self.path, "body": body})
        users = [item for item in body.get("input", []) if item.get("role") == "user"]
        latest = json.dumps(users[-1] if users else {})
        outputs = {item.get("call_id") for item in body.get("input", []) if item.get("type") == "function_call_output"}
        action, text = None, "fixture-ok"
        if "Generate a concise, single-line task title" in json.dumps(body):
            text = '{"title":"Shared runtime acceptance"}'
        elif "DIRECT_PARENT_INITIAL" in latest:
            action = None if "direct-launch-child" in outputs else "direct-launch-child"
            text = "DIRECT_PARENT_READY"
        elif "DIRECT_CHILD_INITIAL" in latest:
            action = None if "direct-ready" in outputs else "direct-ready"
            text = "DIRECT_CHILD_READY"
        elif "DIRECT_CHILD_SEND" in latest:
            action = None if "direct-child-send" in outputs else "direct-child-send"
            text = "DIRECT_CHILD_SENT"
        elif "DIRECT_BUSY_PARENT" in latest:
            (self.server.fixture_root / "direct-busy").touch()
            self.server.gate.wait(12)
            text = "DIRECT_BUSY_DONE"
        elif "DIRECT_MAIL_TO_PARENT" in latest:
            action = None if "direct-reply" in outputs else "direct-reply"
            text = "DIRECT_PARENT_REPLIED"
        elif "DIRECT_MAIL_TO_CHILD" in latest:
            text = "DIRECT_CHILD_RECEIVED"
        elif "DIRECT_OUTAGE_PROMPT" in latest:
            text = "DIRECT_OUTAGE_COMPLETED"
        elif "DIRECT_RECOVERY_FAIL" in latest:
            self.send_error(403, "direct recovery fixture request forbidden")
            return
        elif "DIRECT_VIEW_OTHER" in latest:
            text = "DIRECT_OTHER_VISIBLE"
        elif getattr(self.server, "direct_terminal", False) and "Continue the current task." in latest:
            text = "DIRECT_RECOVERY_COMPLETED"
        elif getattr(self.server, "direct_terminal", False) and "instances ready:" in latest:
            text = "DIRECT_LAUNCH_NOTICE_RECEIVED"
        elif "ENVIRONMENT_PROBE_" in latest:
            label = next(label for label in ["A1", "B", "A2"] if "ENVIRONMENT_PROBE_" + label in latest)
            action = None if "environment-" + label in outputs else "environment-" + label
            text = "ENVIRONMENT_DONE_" + label
        elif "HETERO_CLAUDE_REPLY" in latest:
            text = "HETERO_CODEX_ROUND_TRIP_DONE"
        elif "HETERO_CODEX_INITIAL" in latest:
            action = None if "hetero-send" in outputs else "hetero-send"
            text = "HETERO_CODEX_MESSAGE_SENT"
        elif "SHARED_BUSINESS_REPLY" in latest:
            text = "CHILD_COMPLETED_BUSINESS_REPLY"
        elif "SHARED_CHILD_REPLY" in latest:
            action = None if "parent-reply" in outputs else "parent-reply"
            text = "PARENT_COMPLETED_ROUND_TRIP"
        elif "SHARED_CHILD_INITIAL" in latest:
            action = None if "child-send" in outputs else "child-send"
            text = "CHILD_SENT_REPLY"
        elif "SHARED_PARENT_INITIAL" in latest:
            action = None if "spawn" in outputs else "spawn"
            text = "PARENT_IDLE_WAITING_FOR_CHILD"
        elif "PASEO_HELD_MAIL" in latest:
            text = "PASEO_MAIL_PROCESSED"
        elif "PASEO_APPROVAL_CASE" in latest:
            action = None if "approval-tool" in outputs else "approval-tool"
            text = "PASEO_APPROVAL_DONE"
        elif "HOLD_CASE" in latest:
            text = "PASEO_HELD_TURN_DONE"
            (self.server.fixture_root / "model-held").touch()
            self.server.gate.wait(12)
        else:
            self.server.trace.add("fixture:unexpected", body)
            self.send_error(500, "unexpected collaboration model request")
            return
        response_id = "shared_" + str(event["seq"])
        if action:
            # The path and action come exclusively from this fixture. The real
            # native tool must supply its own correctly isolated AgentX identity.
            import shlex
            helper = REPO / "tests/support/codex_shared_child.py"
            helper_action = "approval-write" if action == "approval-tool" else action
            if action.startswith("environment-"):
                helper_action = "environment"
            command = shlex.join(["/usr/bin/python3", str(helper), helper_action, str(self.server.fixture_root)])
            if action.startswith("environment-"):
                command += " " + shlex.quote(action.removeprefix("environment-"))
            arguments = {"cmd": command}
            if action == "approval-tool":
                arguments.update(sandbox_permissions="require_escalated", justification="Write one fixture-owned approval receipt inside the disposable namespace")
            item = {"id": "tool_" + action, "type": "function_call", "call_id": action, "name": "exec_command", "arguments": json.dumps(arguments)}
        else:
            item = {"id": "message_" + str(event["seq"]), "type": "message", "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": text, "annotations": []}]}
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Connection", "close")
        self.end_headers()
        events = [
            ("response.created", {"response": {"id": response_id, "status": "in_progress", "output": []}}),
            ("response.output_item.added", {"output_index": 0, "item": {**item, "status": "in_progress"}}),
            ("response.output_item.done", {"output_index": 0, "item": item}),
            ("response.completed", {"response": {"id": response_id, "status": "completed", "output": [item], "usage": {"input_tokens": 1, "output_tokens": 1, "total_tokens": 2}}}),
        ]
        for kind, value in events:
            self.wfile.write(("event: " + kind + "\ndata: " + json.dumps({"type": kind, **value}) + "\n\n").encode())
            self.wfile.flush()


class RuntimeFixture:
    """Own one disposable shared scope; callers own scenario assertions."""

    def __init__(self, agentx, codex, evidence_root):
        assert os.environ.get("AGENTX_ACCEPTANCE_NAMESPACE") == "isolated-v1", "use the namespace acceptance runner"
        self.agentx = pathlib.Path(agentx).resolve(strict=True)
        self.codex = pathlib.Path(codex).resolve(strict=True)
        self.evidence_root = pathlib.Path(evidence_root).resolve()
        self.evidence_root.mkdir(parents=True, exist_ok=True)
        state_root = self.evidence_root / "state"
        state_root.mkdir(exist_ok=True)
        self.root = pathlib.Path(tempfile.mkdtemp(prefix="csr-", dir=state_root))
        self.clients = []
        self.owned_processes = []
        self.provider_names = []
        self.trace = PROBE.Trace(self.root)
        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), CollaborationModel)
        self.server.daemon_threads = True
        self.server.trace = self.trace
        self.server.gate = threading.Event()
        self.server.fixture_root = self.root
        self.project, self.flags, _ = PROBE.install_config(self.root, self.server.server_port)
        self.native_home = self.root / "home"
        self.user_home = self.root / "user-home"
        self.bin_dir = self.root / "bin"
        for name in ["user-home", "bin", "tmp", "agentx", "xdg/config", "xdg/data", "xdg/cache", "xdg/state", "xdg/runtime"]:
            (self.root / name).mkdir(parents=True, exist_ok=True)
        os.chmod(self.root / "xdg/runtime", 0o700)
        (self.bin_dir / "agentx").symlink_to(self.agentx)
        (self.bin_dir / "codex").symlink_to(self.codex)
        self.env = {
            "PATH": str(self.bin_dir) + ":/usr/bin:/bin",
            "HOME": str(self.user_home),
            "CODEX_HOME": str(self.native_home),
            "AGENTX_DIR": str(self.root / "agentx"),
            # The shared stdio entry accepts global roots. An unmarked cwd
            # otherwise adopts the personal workspace on current AgentX.
            "AGENTX_WORKSPACE": "global",
            "AGENTX_TEST_ROOT": str(self.root),
            "AGENTX_TEST_DB_NOSYNC": "1",
            "AGENTX_ACCEPTANCE_NAMESPACE": "isolated-v1",
            "TMPDIR": str(self.root / "tmp"),
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
            "TERM": "xterm-256color",
            "NO_COLOR": "1",
            "XDG_CONFIG_HOME": str(self.root / "xdg/config"),
            "XDG_DATA_HOME": str(self.root / "xdg/data"),
            "XDG_CACHE_HOME": str(self.root / "xdg/cache"),
            "XDG_STATE_HOME": str(self.root / "xdg/state"),
            "XDG_RUNTIME_DIR": str(self.root / "xdg/runtime"),
            "DBUS_SESSION_BUS_ADDRESS": "unix:path=" + str(self.root / "absent-bus"),
            "RUST_LOG": "error",
        }
        # This environment is built from fixture constants after env_clear;
        # retaining it never captures ambient credentials or user settings.
        (self.root / "fixture-environment.json").write_text(json.dumps(self.env, indent=2) + "\n")
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def open_client(self, name, native_args=None, *, terminal_socket=False):
        """Enter real AgentX; no direct native substitute for composition."""
        args = ["app-server"] if native_args is None else native_args
        command = [str(self.agentx), "launch", "--tool", "codex", "--stdio", "--go", "--", *args]
        client_type = TerminalSocketClient if terminal_socket else PROBE.Client
        client = client_type(command, self.env, self.project, self.trace, name)
        self.clients.append(client)
        return client

    def run_agentx(self, args, stdin=None):
        return subprocess.run([str(self.agentx), *args], cwd=self.project, env=self.env, input=stdin, text=True, capture_output=True, timeout=15)

    def start_thread(self, client, request_id):
        return PROBE.start_thread(client, self.project, request_id, stable=True)

    def turn(self, client, request_id, thread_id, text, client_id=None):
        return PROBE.turn(client, request_id, thread_id, text, client_id)

    def completed(self, thread_id, after):
        return PROBE.completed(self.trace, thread_id, after)

    def assert_no_protocol_pollution(self):
        invalid = [event for event in self.trace.events if "invalidLine" in event.get("value", {})]
        assert not invalid, invalid

    def retain(self, summary):
        (self.root / "result.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
        target = self.evidence_root / self.root.name
        target.mkdir()
        for path in self.root.iterdir():
            if path.is_file() and (path.suffix in {".json", ".jsonl", ".log"} or path.name.endswith((".stderr", ".stdout"))):
                shutil.copy2(path, target / path.name)
            elif path.name == "child-trace":
                shutil.copytree(path, target / path.name, ignore=shutil.ignore_patterns("tui-*"))
        return target

    def close(self):
        """Stop fixture connections, then explicitly registered owned services.

        Service ownership is added only after the native-service handoff proves
        its exact generation/cleanup contract. Never discover/kill by name or
        call global daemon management from this helper.
        """
        self.server.gate.set()
        errors = []
        for name in self.provider_names:
            try:
                stopped = self.run_agentx(["kill", name])
                if stopped.returncode:
                    errors.append("fixture provider cleanup: " + stopped.stderr)
            except (OSError, subprocess.TimeoutExpired) as error:
                errors.append(repr(error))
        for record_path in [self.root / "child-proxy.json", self.root / "child-driver.json"]:
            if not record_path.exists():
                continue
            record = json.loads(record_path.read_text())
            stat = pathlib.Path(f"/proc/{record['pid']}/stat")
            try:
                # Compare start ticks before signalling an explicitly recorded
                # fixture process. Never discover/kill a service by name.
                if stat.read_text().rsplit(")", 1)[1].split()[19] == record["startTicks"]:
                    os.killpg(record["pid"], signal.SIGTERM)
            except FileNotFoundError:
                pass
            except OSError as error:
                errors.append(repr(error))
        for client in reversed(self.clients):
            try:
                client.stop()
            except (OSError, subprocess.TimeoutExpired) as error:
                errors.append(repr(error))
            finally:
                if client.process.poll() is None:
                    try:
                        os.killpg(client.process.pid, signal.SIGKILL)
                        client.process.wait(timeout=3)
                    except (OSError, subprocess.TimeoutExpired) as error:
                        errors.append(repr(error))
        # This management command is permitted only inside the disposable PID,
        # network and filesystem namespaces enforced by the acceptance runner.
        # The isolated native-service probe established this exact home scope.
        try:
            stopped = subprocess.run([str(self.codex), "app-server", "daemon", "stop"], cwd=self.project, env=self.env, capture_output=True, text=True, timeout=10)
            self.trace.add("cleanup:daemon", {"status": stopped.returncode, "stdout": stopped.stdout, "stderr": stopped.stderr})
            if stopped.returncode:
                errors.append("isolated daemon stop failed: " + stopped.stderr)
        except (OSError, subprocess.TimeoutExpired) as error:
            errors.append(repr(error))
        for process in reversed(self.owned_processes):
            try:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait(timeout=3)
            except (OSError, subprocess.TimeoutExpired) as error:
                errors.append(repr(error))
        self.server.shutdown()
        self.server.server_close()
        if errors:
            raise RuntimeError("fixture cleanup failed: " + "; ".join(errors))
        shutil.rmtree(self.root)
