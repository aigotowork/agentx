#!/usr/bin/env python3
"""Check candidate TUI and packaged client against an isolated native control router.

A scripted host publishes a lease and reports effects. Real Provider execution
and three-machine acceptance are covered by daemon.py.
"""

import argparse
import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from acceptance import provision
from fixture import COMPACT_ID, Deployment, project, require, wait_for
from management import tls_front


def exercise(fixture, cli, chromium, ca, dropped_replies):
    created, scope = provision(fixture, "lab")
    origin = "/namespaces/lab/api/v1"
    admin = fixture.admin + "/namespaces/lab"
    admin_token = fixture.admin_token
    scope_token = created["token"]
    target = {"generation": scope["generation"], "installation_id": "installation-test"}
    host = "host-a"
    base = f"{origin}/hosts/{host}/control"
    lease = {**target, "lease_id": "lease-test"}
    workspace_id = "WS000000000B"
    catalogue = {"workspace_id": workspace_id, "name": "lab-work", "default_available": True, "default_reason": None,
                 "roles": [{"name": "reviewer", "available": True, "reason": None},
                           {"name": "unavailable", "available": False, "reason": "tool_unavailable"}]}
    renewal = {**lease, "workspaces": [catalogue], "project_roots": ["work"]}
    fixture.expect("PUT", base + "/lease", scope_token, renewal)
    env = {**fixture.environment("control-cli"), "AGENTX_ADMIN_TOKEN": admin_token, "AGENTX_ADMIN_URL": fixture.url,
           "SSL_CERT_FILE": str(ca)}

    def cli_call(label, words):
        return json.loads(fixture.command(label, [cli, "admin", *words], env).stdout)

    def poll():
        return fixture.expect("POST", base + "/poll", scope_token, lease)["operation"]

    def complete(receipt, agent):
        assert receipt["status"] == "running"
        return fixture.expect("POST", base + f"/operations/{receipt['request_id']}/report", scope_token,
                              {**lease, "claim_id": receipt["claim_id"], "outcome": {"status": "succeeded", "agent": agent}})

    def register(agent):
        fixture.expect("PUT", origin + f"/agents/{agent['name']}", scope_token,
                       {**target, "host": agent["host"], "incarnation": agent["incarnation"],
                        "workspace": agent["workspace"]})

    caps = cli_call("packaged-capabilities", ["hosts", "capabilities", "lab", "--host", host])
    assert caps["online"] and caps["observed_at"] > 0 and caps["workspaces"] == [catalogue] and caps["project_roots"] == ["work"]

    # Reuse the same embedded browser helper as the actual physical lane.
    if chromium:
        with tempfile.TemporaryDirectory(prefix="cloud-control-chrome-") as profile:
            command = {"namespace": "lab", "host": host, "workspace_id": workspace_id, "role": "reviewer",
                       "name": "browser_agent", "title": "Browser control", "initial_prompt": "Read the acceptance notes."}
            browser_env = {**fixture.environment("control-browser"), "CLOUD_ADMIN_TOKEN": admin_token,
                           "CLOUD_BROWSER_URL": fixture.url, "CLOUD_BROWSER": chromium, "CLOUD_CERT_PIN": "",
                           "CLOUD_EVIDENCE": str(fixture.directory), "CLOUD_CONTROL_LAUNCH": json.dumps(command),
                           "CLOUD_CHROME_PROFILE": profile + "/profile", "TMPDIR": profile}
            output = fixture.command("embedded-web-launch", ["node", Path(__file__).with_name("browser.mjs").resolve()],
                                     browser_env, timeout=35)
            browser_receipt = json.loads(output.stdout)["receipt"]
            claim = poll()
            assert claim["request_id"] == browser_receipt["request_id"]
            browser_agent = {"host": host, "name": command["name"], "incarnation": "BrowserInc01",
                             "workspace": project(workspace_id)}
            register(browser_agent)
            complete(claim, browser_agent)
            operation = cli_call("packaged-stop", ["operations", "stop", "lab", "--generation", caps["generation"],
                                  "--host", host, "--installation-id", caps["installation_id"], "--name", browser_agent["name"],
                                  "--incarnation", browser_agent["incarnation"], "--pending", fixture.runtime / "stop.json"])
            claimed_stop = poll()
            assert operation["request_id"] == claimed_stop["request_id"]
            assert claimed_stop["command"] == {"kind": "stop", "name": browser_agent["name"], "incarnation": browser_agent["incarnation"]}
            assert complete(claimed_stop, browser_agent)["status"] == "succeeded"
            confirmed = cli_call("packaged-exact-stop-receipt", ["operations", "show", "lab", "--request-id", operation["request_id"]])
            assert confirmed["outcome"]["agent"] == browser_agent

    with tempfile.TemporaryDirectory(prefix="cloud-control-tui-") as root:
        socket = Path(root) / "tmux"
        cli_tmux = shutil.which("tmux")
        require(cli_tmux, "Control TUI probe requires tmux")

        def terminal(*args, check=True):
            return subprocess.run([cli_tmux, "-S", socket, "-f", "/dev/null", *args], env=env, capture_output=True, text=True,
                                  timeout=5, check=check).stdout

        def keys(*args):
            terminal("send-keys", "-t", "control", *args)

        def screen(label, *values):
            rendered = ""
            def visible():
                nonlocal rendered
                rendered = terminal("capture-pane", "-pt", "control")
                return "Working…" not in rendered and all(value in rendered for value in values)
            try:
                wait_for(label, visible, timeout=8)
                return rendered
            finally:
                (fixture.directory / (label + ".txt")).write_text(fixture.redact(rendered))

        try:
            fixture.expect("PUT", base + "/lease", scope_token, renewal)
            terminal("new-session", "-d", "-s", "control", "-x", "92", "-y", "24",
                     str(fixture.candidate), "tui", "--admin-url", fixture.public_url)
            screen("tui-control-overview", "AgentX / Deployment", "lab", "o Receipt")
            keys("Enter")
            screen("tui-control-hosts", "[Hosts]", host, "l Launch")
            keys("l")
            screen("tui-control-catalogue", "Launch Agent", "Management online", "Default Codex", "lab-work")
            keys("Tab", "Right", "Right")
            screen("tui-control-unavailable-role", "tool_unavailable")
            keys("Enter")
            screen("tui-control-no-unavailable-launch", "Select an available role.")
            assert poll() is None
            keys("Left", "Tab")
            keys("-l", "terminal_agent")
            keys("Tab")
            keys("-l", "Terminal control")
            keys("Tab")
            keys("-l", "Review the fixture.")
            keys("Enter")
            capture = screen("tui-control-launch-lost-reply", "Submission not confirmed", "Enter Check")
            request_id = re.search(rf"\b{COMPACT_ID}\b",
                                   capture.split("Submission not confirmed", 1)[1])[0]
            assert dropped_replies == [{"status": 200, "dropped": True}]
            keys("Enter")
            screen("tui-control-launch-queued", "Queued", request_id, "Enter Check receipt")
            assert len(dropped_replies) == 1, "Receipt lookup must not resend"
            queued = poll()
            assert queued["request_id"] == request_id
            assert queued["command"] == {"kind": "launch", "workspace_id": workspace_id, "role": "reviewer", "name": "terminal_agent",
                                          "title": "Terminal control", "initial_prompt": "Review the fixture."}
            agent = {"host": host, "name": "terminal_agent", "incarnation": "Terminal0001",
                     "workspace": project(workspace_id)}
            register(agent)
            complete(queued, agent)
            keys("Enter")
            screen("tui-control-launch-succeeded", "Succeeded", agent["incarnation"])
            keys("Escape")
            keys("Tab")
            screen("tui-control-agents", "[Agents]", "terminal_agent")
            terminal("resize-window", "-t", "control", "-x", "48", "-y", "17")
            keys("s")
            screen("tui-control-exact-stop", "Stop Agent", "terminal_agent", agent["incarnation"])
            keys("Enter")
            screen("tui-control-stop-queued", "Queued")
            stopped = poll()
            assert stopped["command"] == {"kind": "stop", "name": agent["name"], "incarnation": agent["incarnation"]}
            complete(stopped, agent)
            keys("Enter")
            screen("tui-control-stop-succeeded", "Succeeded", agent["incarnation"])
            keys("Escape")
            keys("o")
            screen("tui-control-find-dialog", "Operation receipt", "Request ID")
            keys("-l", request_id)
            keys("Enter")
            screen("tui-control-original-receipt", "Succeeded", "reviewer")
            return {"browser_launch_packaged_stop": bool(chromium), "native_tui_launch_stop_and_lookup": True,
                    "native_tui_lost_reply_lookup_without_resubmission": True, "scripted_host": True}
        finally:
            terminal("kill-server", check=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("cli", type=Path)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--chromium")
    args = parser.parse_args()
    fixture = Deployment(args.candidate, args.evidence)
    try:
        # CLI and Web use the direct candidate; only the TUI crosses the once-dropping TLS terminator.
        with tls_front(fixture, drop_method="POST", drop_path="/api/v1/admin/namespaces/lab/operations") as (ca, _, attempts):
            fixture.start()
            fixture.result["cases"]["control_consumers"] = exercise(fixture, args.cli.absolute(), args.chromium, ca, attempts)
    finally:
        fixture.close()


if __name__ == "__main__":
    main()
