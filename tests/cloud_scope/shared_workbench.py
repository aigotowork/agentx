"""Drive the embedded cloud Space through a real daemon, role launch and Codex reply."""
import argparse
import json
import os
from pathlib import Path
import subprocess

from acceptance import provision
from daemon import PublicControl, TERMINAL, cold_start
from daemon_rig import SCOPE, Station, use_https
from fixture import Deployment, require, wait_for
from management import tls_front
from native import Model


class BrowserModel(Model):
    """The assistant returns ordinary final text without calling any AgentX tool."""
    operator_reply = "收到，已经核对完这条消息。"

    progress = "正在逐项核对这条消息。"

    def response_chunks(self, event, item):
        if event != "browser-final":
            yield from super().response_chunks(event, item)
            return
        events = next(super().response_chunks(event, item)).split(b"\n\n")
        progress = {**item, "id": "msg_browser_progress", "phase": "commentary",
                    "content": [{"type": "output_text", "text": self.progress}]}
        intermediate = json.dumps({"type": "response.output_item.done", "item": progress}).encode()
        yield events[0] + b"\n\nevent: response.output_item.done\ndata: " + intermediate + b"\n\n"
        # The browser releases the final only after it has actually rendered the earlier commentary.
        wait_for("commentary rendered before final", lambda: (self.fixture.runtime / "progress-seen").exists(), 40)
        yield b"\n\n".join(events[1:])

    def respond(self, request):
        users = [item for item in request["input"] if item.get("role") == "user"]
        latest = users[-1].get("content", []) if users else []
        if any(self.operator_message in item.get("text", "") for item in latest):
            message = {"type": "message", "role": "assistant", "phase": "final_answer", "id": "msg_browser_reply",
                       "content": [{"type": "output_text", "text": self.operator_reply}]}
            return "browser-final", message, None
        return super().respond(request)


def exercise(fixture):
    station = Station(fixture, "east")
    try:
        station.initialize()
        with tls_front(fixture) as (ca, pin, _), BrowserModel(fixture) as model:
            fixture.start()
            use_https(fixture, ca)
            created, scope = provision(fixture, SCOPE)
            station.prepare_worker(scope["scope_url"], ca, created["token"], "east", model)
            cold_start(station)
            control = PublicControl(fixture, created["token"], scope["generation"])
            host = control.await_host("east")
            require(host.get("directory_launch") is True, "The native lease must advertise launch directories")
            (station.workspace / "runs").mkdir()
            (station.workspace / "outside").symlink_to(station.root / "home", target_is_directory=True)
            control.reject(control.payload(host, {"kind": "launch", "workspace": "work", "working_directory": "../home"}),
                           "invalid_input", status=400)
            escape = control.payload(host, {"kind": "launch", "workspace": "work", "name": "escape",
                                           "working_directory": "outside/no-create", "create_directory": True})
            control.submit(escape)

            def escape_terminal():
                receipt = control.receipt(escape)
                return receipt if receipt["status"] in TERMINAL else False

            rejection = wait_for("native symlink refusal", escape_terminal, 35, interval=0.2)
            require(rejection["outcome"] == {"status": "failed", "code": "directory_unsafe"}
                    and not (station.root / "home/no-create").exists() and not control.agents(),
                    "A symlink launch must not create an outside directory or register an Agent")
            env = {**os.environ, "SPACE_ENTRY": scope["scope_url"] + "/space/?access=admin",
                   "SPACE_TOKEN": fixture.admin_token, "SPACE_PIN": pin, "SPACE_MESSAGE": model.operator_message,
                   "SPACE_PROGRESS": model.progress, "SPACE_PROGRESS_SEEN": str(fixture.runtime / "progress-seen"),
                   "SPACE_REPLY": model.operator_reply, "SPACE_EVIDENCE": str(fixture.directory),
                   "SPACE_EXPECTED_CWD": str(station.workspace / "runs/first")}
            subprocess.run(["node", Path(__file__).with_suffix(".mjs")], env=env, check=True, timeout=180)
            require(model.seen("browser-final"), "Actual Provider must have received the browser message")
            agent, = control.agents()
            require(agent["host"] == "east" and agent["name"] == "receiver", "Only the selected fixture instance may launch")
            supervisor = station.root / "scope/cloud-client/supervisors" / (agent["incarnation"] + ".json")
            witness = json.loads(supervisor.read_text())
            cwd = (Path("/proc") / str(witness["pid"]) / "cwd").resolve(strict=True)
            require(witness["ready"] and cwd == station.workspace / "runs/first" and cwd.is_dir(),
                    "The actual supervisor must start in the directory created from Space")
            fixture.write("launch-directory.json", {
                "host": agent["host"], "workspace": "work", "working_directory": "runs/first",
                "supervisor_cwd_verified": True, "symlink_refused": rejection["outcome"]["code"],
            })
            command = control.payload(host, {"kind": "stop", "name": agent["name"], "incarnation": agent["incarnation"]})
            control.submit(command)
            control.succeeded(command)
            return agent
    finally:
        station.close()
        station.close_ssh()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--evidence", required=True, type=Path)
    args = parser.parse_args()
    fixture = Deployment(args.candidate, args.evidence)
    try:
        fixture.result["cases"]["shared-space"] = {"status": "passed", "agent": exercise(fixture)}
    except Exception as error:
        fixture.result["cases"]["shared-space"] = {"status": "failed", "error": str(error)}
        raise
    finally:
        fixture.close()
