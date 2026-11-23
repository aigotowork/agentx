"""Drive the embedded cloud Space through a real daemon, role launch and Codex reply."""
import argparse
import os
from pathlib import Path
import subprocess

from acceptance import provision
from daemon import PublicControl, cold_start
from daemon_rig import SCOPE, Station, use_https
from fixture import Deployment, require
from management import tls_front
from native import Model


class BrowserModel(Model):
    """The assistant returns ordinary final text without calling any AgentX tool."""
    operator_reply = "收到，已经核对完这条消息。"

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
            env = {**os.environ, "SPACE_ENTRY": scope["scope_url"] + "/space/?access=admin",
                   "SPACE_TOKEN": fixture.admin_token, "SPACE_PIN": pin, "SPACE_MESSAGE": model.operator_message,
                   "SPACE_REPLY": model.operator_reply}
            subprocess.run(["node", Path(__file__).with_suffix(".mjs")], env=env, check=True, timeout=180)
            require(model.seen("browser-final"), "Actual Provider must have received the browser message")
            agent, = control.agents()
            require(agent["host"] == "east" and agent["name"] == "receiver", "Only the selected fixture instance may launch")
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
