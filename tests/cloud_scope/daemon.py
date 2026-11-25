#!/usr/bin/env python3
"""Accept cloud-connected native daemons through actual public and Provider interfaces."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import http.client
import json
from pathlib import Path
import re
import shlex
import stat
import tempfile
import time
from urllib.parse import urlparse

from acceptance import provision, request_id
from daemon_rig import SCOPE, model_forward, topology
from fixture import COMPACT_ID, WORKSPACE_ID, Deployment, compact_id, exact, require, wait_for
from management import certificate_pin
from native import Model


DENIED = (401, 403)
TERMINAL = {"succeeded", "failed", "uncertain"}
NETWORK_PROOF = """import errno, os, socket, sys
bad = {"AGENTX_NAMESPACE_TOKEN", "AGENTX_ADMIN_TOKEN", "AGENTX_CLOUD_CONTEXT"}.intersection(os.environ)
if bad:
    raise SystemExit("unexpected cloud credential variables: " + ", ".join(sorted(bad)))
try:
    socket.create_connection((sys.argv[1], int(sys.argv[2])), timeout=3).close()
except OSError as e:
    if e.errno not in (errno.EPERM, errno.EACCES, errno.ENETUNREACH, errno.ECONNREFUSED):
        raise
    print("cloud-acceptance-egress-denied=" + errno.errorcode[e.errno])
else:
    raise SystemExit("direct cloud connection unexpectedly succeeded")
print("cloud-acceptance-credential-env-absent")
"""


class SandboxedModel(Model):
    def __init__(self, fixture):
        super().__init__(fixture)
        origin = urlparse(fixture.public_url)
        self.network_probe = shlex.join(["python3", "-c", NETWORK_PROOF, origin.hostname, str(origin.port)])

    def tool_command(self, args):
        send = shlex.join(["agentx", *args])
        # Use the same user operation, from the same exact Agent, twice.
        return f"{self.network_probe} && {send} && {send}"

    def respond(self, request):
        body = json.dumps(request, ensure_ascii=False)
        require(all(secret not in body for secret in self.fixture.tokens),
                "A cloud credential reached the Provider context")
        return super().respond(request)

    def unexpected_turn(self, say, latest):
        return say("other-fixture-turn", "No scripted action for this input", None)


class PublicControl:
    def __init__(self, fixture, token, generation):
        self.fixture, self.token, self.generation = fixture, token, generation
        self.scope = f"/namespaces/{SCOPE}/api/v1"
        self.admin = f"{fixture.admin}/namespaces/{SCOPE}"
        self.operations = self.admin + "/operations"

    def capabilities(self, host, statuses=(200,)):
        return self.fixture.expect("GET", f"{self.admin}/hosts/{host}/capabilities",
                                   self.fixture.admin_token, statuses=statuses)

    def agents(self, **query):
        page = self.fixture.expect("GET", self.admin + "/agents", self.fixture.admin_token, query=query)
        return page["items"]

    def payload(self, host, command):
        return {"generation": self.generation, "request_id": request_id(), "host": host["host"],
                "installation_id": host["installation_id"], "command": command}

    def submit(self, payload, statuses=(200, 201, 202)):
        return self.fixture.expect("POST", self.operations, self.fixture.admin_token, payload, statuses=statuses)

    def reject(self, payload, code, status=409):
        result = self.submit(payload, statuses=(status,))
        require(result["code"] == code, f"Expected {code}, got {result['code']}")

    def receipt(self, payload):
        receipt = self.fixture.expect("GET", self.operations + "/" + payload["request_id"], self.fixture.admin_token)
        require(all(receipt[key] == payload[key] for key in ("generation", "host", "installation_id", "request_id")),
                "Control receipt must retain the exact requested identity")
        require(all(receipt["command"].get(key) == value for key, value in payload["command"].items()),
                "Control receipt changed the specified typed command")
        return receipt

    def succeeded(self, payload):
        def terminal():
            value = self.receipt(payload)
            return value if value["status"] in TERMINAL else False

        receipt = wait_for(f"verified remote {payload['command']['kind']} {payload['request_id']}", terminal, 75, interval=0.5)
        require(receipt["status"] == "succeeded" and receipt["claim_id"] and receipt["outcome"]["status"] == "succeeded",
                f"Remote control failed: {receipt['request_id']} {receipt['outcome']}")
        agent = receipt["outcome"]["agent"]
        require(agent["host"] == payload["host"] and agent["name"] == payload["command"]["name"]
                and re.fullmatch(COMPACT_ID, agent["incarnation"]), "Successful control receipt must identify the exact affected Agent")
        return receipt

    def confirmed(self, message_request):
        def confirmed_message():
            value = self.fixture.expect("GET", f"{self.scope}/operations/{message_request}", self.token)
            return value["message"] if value["status"] == "confirmed" else False

        return wait_for("confirmed Provider delivery", confirmed_message, 40, interval=0.5)

    def await_host(self, name, validate_default=True):
        def online():
            status, row = self.fixture.request("GET", f"{self.admin}/hosts/{name}/capabilities", self.fixture.admin_token)
            if status == 404:
                return False
            require(status == 200, f"{name} capability read failed: {status}")
            return row if row["online"] else False

        host = wait_for(f"{name} live daemon control lease", online, 40, interval=0.5)
        require(host["generation"] == self.generation and host["host"] == name and host["installation_id"],
                "Control lease must identify its exact scope generation and installation")
        require(isinstance(host["observed_at"], int) and abs(time.time() - host["observed_at"]) < 30,
                "Capabilities must identify when the live catalogue was observed")
        if validate_default:
            workspace, = host["workspaces"]
            require(host["project_roots"] == ["work"] and workspace["name"] == "workspace"
                    and re.fullmatch(WORKSPACE_ID, workspace["workspace_id"]) and workspace["default_available"]
                    and workspace["default_reason"] is None, "Capabilities must separate a project from its root alias")
            roles = {entry["name"]: entry for entry in workspace["roles"]}
            require(roles["acceptance"]["available"] and roles["acceptance"]["reason"] is None
                    and not roles["unavailable"]["available"] and roles["unavailable"]["reason"],
                    "Catalogue must distinguish an eligible local role from an unresolvable role")
        return host


class PhysicalConsumers:
    """Use the real embedded browser and an independently installed package CLI."""

    def __init__(self, fixture, ca, chromium, consumer):
        self.fixture, self.ca = fixture, ca
        self.chromium = chromium.resolve(strict=True)
        package = consumer.resolve(strict=True) / "node_modules/@agentx/client"
        self.cli = consumer.resolve(strict=True) / "node_modules/.bin/agentx-http"
        require(self.cli.resolve(strict=True).is_relative_to(package), "Select an externally installed @agentx/client")
        self.package = json.loads((package / "package.json").read_text())
        require(self.package["name"] == "@agentx/client", "Unexpected physical acceptance CLI package")

    def launch(self, payload):
        fixture = self.fixture
        command = {key: value for key, value in payload["command"].items() if key != "kind"}
        env = {**fixture.environment("physical-browser"), "CLOUD_BROWSER": str(self.chromium),
               "CLOUD_BROWSER_URL": fixture.public_url, "CLOUD_CERT_PIN": certificate_pin(fixture.runtime / "tls.crt"),
               "CLOUD_ADMIN_TOKEN": fixture.admin_token, "CLOUD_EVIDENCE": str(fixture.directory),
               "CLOUD_CONTROL_LAUNCH": json.dumps({"namespace": SCOPE, "host": payload["host"], **command})}
        helper = Path(__file__).with_name("browser.mjs").resolve()
        with tempfile.TemporaryDirectory(prefix="agentx-cloud-chrome-", dir="/tmp") as temporary:
            result = fixture.command("browser-launch", ["node", helper],
                                     {**env, "TMPDIR": temporary, "CLOUD_CHROME_PROFILE": temporary + "/profile"},
                                     cwd=helper.parents[2], timeout=100)
        require(all(secret not in result.stdout + result.stderr for secret in fixture.tokens),
                "Browser helper disclosed a fixture credential")
        observed = json.loads(result.stdout)
        receipt = observed["receipt"]
        require(observed["secureContext"] and receipt["host"] == payload["host"]
                and receipt["generation"] == payload["generation"] and receipt["installation_id"] == payload["installation_id"]
                and receipt["command"] == payload["command"] and receipt["request_id"],
                "Browser must submit the exact typed launch over the real secure deployment")
        return observed

    def stop(self, payload):
        fixture = self.fixture
        env = {**fixture.environment("physical-cli"), "AGENTX_ADMIN_URL": fixture.public_url,
               "AGENTX_ADMIN_TOKEN": fixture.admin_token, "NODE_EXTRA_CA_CERTS": str(self.ca)}
        pending = fixture.runtime / "physical-cli" / "stop.json"
        args = ["admin", "operations", "stop", SCOPE, "--generation", payload["generation"],
                "--host", payload["host"], "--installation-id", payload["installation_id"],
                "--name", payload["command"]["name"], "--incarnation", payload["command"]["incarnation"],
                "--request-id", payload["request_id"], "--pending", pending]
        command = fixture.command("cli-stop", [self.cli, *args], env, timeout=35)
        receipts = json.loads(command.stdout)
        require(receipts["request_id"] == payload["request_id"] and receipts["command"] == payload["command"],
                "Independent CLI must submit an exact stop for the Web-launched Agent")
        require(stat.S_IMODE(pending.stat().st_mode) == 0o600
                and all(secret not in pending.read_text() + command.stdout + command.stderr for secret in fixture.tokens),
                "Exact CLI stop must retain only private, bearer-free retry metadata")
        return receipts


def cold_start(station):
    cold = station.command("cold-status", "daemon", "status")
    require("Daemon: stopped" in cold.stdout, "Enrollment and passive diagnostics must not start the daemon")
    station.command("ordinary-list", "list", "--json")
    peer = station.process("peer")
    running = station.command("running-status", "daemon", "status")
    require(f"Peer: pid={peer['pid']} " in running.stdout and "Daemon: running" in running.stdout,
            "Ordinary list must auto-start a daemon matching the verified socket peer")
    station.fixture.write(f"{station.label}-cold-peer.json", peer)
    return peer


def rejected_controls(control, host, launch):
    fixture = control.fixture
    caps = f"{control.admin}/hosts/{host['host']}/capabilities"
    for bearer in (None, control.token):
        fixture.expect("GET", caps, bearer, statuses=DENIED)
        fixture.expect("POST", control.operations, bearer, launch, statuses=DENIED)
    poll = f"{control.scope}/hosts/{host['host']}/control/poll"
    poll_payload = {"generation": control.generation, "installation_id": host["installation_id"], "lease_id": request_id()}
    fixture.expect("POST", poll, fixture.admin_token, poll_payload, statuses=DENIED)
    bad_lease = fixture.expect("POST", poll, control.token, poll_payload, statuses=(409,))
    require(bad_lease["code"] == "stale_lease", "A scope bearer alone must not acquire a foreign lease")
    for field, code in (("generation", "stale_generation"), ("installation_id", "host_conflict")):
        control.reject({**launch, "request_id": request_id(), field: request_id()}, code)
    for field, value, code in (("workspace_id", "WS000000000F", "capability_unavailable"),
                               ("role", "unavailable", "capability_unavailable"),
                               ("role", "unknown", "capability_unavailable"), ("name", "bad/name", "invalid_input"),
                               ("initial_prompt", "x" * (16 * 1024 + 1), "invalid_input"),
                               ("argv", ["not-supported"], "invalid_input")):
        control.reject({**launch, "request_id": request_id(), "command": {**launch["command"], field: value}},
                       code, status=400 if code == "invalid_input" else 409)
    require(control.agents() == [], "Rejected operations must not launch any Agent")


def lease_fencing(control):
    """Drive boundary states via the public protocol on two synthetic installations."""
    fixture = control.fixture
    legacy_identity = {"generation": control.generation, "installation_id": request_id()}
    fixture.expect("PUT", f"{control.scope}/hosts/legacy", control.token, legacy_identity)
    legacy = control.capabilities("legacy")
    require(not legacy["online"] and legacy["observed_at"] is None and not legacy["workspaces"],
            "A legacy communication heartbeat must not enable daemon administration")
    workspace_id = "WS000000000A"
    control.reject(control.payload(legacy, {"kind": "launch", "workspace_id": workspace_id}), "host_offline")

    host_route = f"{control.scope}/hosts/fence/control"
    identity = {"generation": control.generation, "installation_id": request_id(), "lease_id": request_id()}
    catalogue = {"project_roots": ["work"],
                 "workspaces": [{"workspace_id": workspace_id, "name": "workspace", "default_available": True,
                                 "default_reason": None, "roles": []}]}
    host = fixture.expect("PUT", f"{host_route}/lease", control.token, {**identity, **catalogue})
    command = {"kind": "launch", "workspace_id": workspace_id}
    claimed, queued = (control.payload(host, command) for _ in range(2))
    control.submit(claimed)
    claim = fixture.expect("POST", f"{host_route}/poll", control.token, identity)["operation"]
    require(claim["request_id"] == claimed["request_id"] and claim["status"] == "running" and claim["claim_id"],
            "A valid lease must atomically establish a specific claim")
    require(fixture.expect("POST", f"{host_route}/poll", control.token, identity)["operation"] == claim,
            "Same-lease poll retry must return the existing claim")
    control.submit(queued)
    successor = {**identity, "lease_id": request_id()}
    fixture.expect("PUT", f"{host_route}/lease", control.token,
                   {**successor, **catalogue, "previous_lease_id": identity["lease_id"]})
    uncertain, unclaimed = control.receipt(claimed), control.receipt(queued)
    require(uncertain["status"] == "uncertain" and uncertain["claim_id"] == claim["claim_id"]
            and uncertain["outcome"] == {"status": "uncertain", "code": "host_lease_lost"}
            and unclaimed["status"] == "failed" and unclaimed["claim_id"] is None
            and unclaimed["outcome"] == {"status": "failed", "code": "host_lease_lost"},
            "Lease replacement must distinguish unknown execution after claim from failure before claim")
    require(fixture.expect("POST", f"{host_route}/poll", control.token, successor) == {"operation": None},
            "Replacement lease must never execute either operation from its predecessor")
    report_route = f"{host_route}/operations/{claimed['request_id']}/report"
    outcome = {"status": "failed", "code": "fixture_failed_before_launch"}
    report = {"claim_id": claim["claim_id"], "outcome": outcome}
    old = fixture.expect("POST", report_route, control.token, {**identity, **report}, statuses=(409,))
    require(old["code"] == "stale_lease", "Prior lease cannot reconcile execution")
    fixture.expect("POST", report_route, control.token, {**successor, **report, "claim_id": request_id()}, statuses=(409,))
    resolved = fixture.expect("POST", report_route, control.token, {**successor, **report})
    require(resolved["outcome"] == outcome and resolved == control.receipt(claimed)
            and fixture.expect("POST", report_route, control.token, {**successor, **report}) == resolved,
            "Only the live replacement and original claim may reconcile and idempotently replay a known outcome")
    return {"legacy_only": legacy, "before_claim": unclaimed, "after_claim": uncertain, "reconciled": resolved}


def stop(control, host, agent, submit=None):
    payload = control.payload(host, {"kind": "stop", "name": agent["name"], "incarnation": agent["incarnation"]})
    (submit or control.submit)(payload)
    receipt = control.succeeded(payload)
    require(exact(receipt["outcome"]["agent"]) == exact(agent), "Stop operation did not refer to the exact original incarnation")
    wait_for("stopped Agent removed from active cloud inventory",
             lambda: all(exact(row) != exact(agent) for row in control.agents()))
    return receipt


def verify_worker_stopped(station, worker):
    wait_for(f"{station.label} exact local supervisor stopped", lambda: not station.process("workers"), 10)
    station.fixture.write(f"{station.label}-supervisor-lifecycle.json", {"verified_worker": worker, "remaining": []})


def prove_round_trip(control, model, east, west, receiver, west_host):
    model.receiver = f"@{receiver['host']}/{receiver['name']}"
    west.command("native-launch-sender", "launch", "--tool", "codex", "--headless", "--name", "sender",
                 "--dir", west.workspace, "--go", "--json", "--agentx-prompt", model.launch_prompt)
    wait_for("both Codex tools complete under the network policy",
             lambda: model.seen("request-complete") and model.seen("reply-complete"), 65)
    delivered = wait_for("sender Codex received the correlated reply", lambda: model.seen("reply-observed-by-sender"), 45)
    for event in ("request-complete", "reply-complete"):
        evidence = json.dumps(model.seen(event))
        require("cloud-acceptance-egress-denied=" in evidence and "cloud-acceptance-credential-env-absent" in evidence,
                f"{event} must prove direct network denial, absent credentials and successful ordinary native send")
    request, reply = control.confirmed(model.request_id), control.confirmed(model.reply_id)
    require(request["from"] == reply["to"] and request["to"] == reply["from"]
            and reply["reply_to"] == request["id"] and request["thread"] == reply["thread"] == "native-proof",
            "Both confirmed receipts must retain the exact request/reply relation")
    agents = control.agents()
    sender, = [row for row in agents if row["host"] == west_host["host"] and row["name"] == "sender"]
    require(len(agents) == 2 and receiver in agents and exact(request["from"]) == exact(sender)
            and exact(request["to"]) == exact(receiver),
            "Cloud inventory, confirmed receipts and Provider observations must use the same two Agent identities")
    native_list = json.loads(east.command("native-list", "list", "--json").stdout)
    require(all(any(all(row.get(key) == value for key, value in exact(agent).items()) for row in native_list)
                for agent in agents), "Ordinary native list did not return the same exact cloud identities")
    inspected = json.loads(west.command("native-inspect-send", "send", "--iam", sender["name"],
                                        "--operation", model.request_id).stdout)
    require(inspected["status"] == "confirmed" and inspected["message"] == request,
            "Native operation inspection must observe the same confirmed receipt")
    denied_id = request_id()
    denied = west.command("native-foreign-sender-denied", "send", "--iam", receiver["name"],
                          "@west/sender", "--request-id", denied_id, "--", "must not be sent", check=False)
    require(denied.returncode != 0, "A tool or operator must not select an Agent from a different installation")
    control.fixture.expect("GET", f"{control.scope}/operations/{denied_id}", control.token, statuses=(404,))
    require(len(delivered) == 1, "Duplicate send yielded more than one related delivery to sender")
    return sender, [request, reply]


def reconnect(control, east, west, east_host, east_peer, prior):
    require(east.process("stop") == east_peer, "Offline simulation must terminate only the originally verified peer")
    offline = wait_for("east control lease expires while west remains live",
                       lambda: (row if not (row := control.capabilities("east"))["online"] else False), 45, interval=1)
    require(offline["installation_id"] == east_host["installation_id"] and offline["workspaces"] == east_host["workspaces"],
            "Offline row must retain its observed catalogue and selected installation")
    require(control.capabilities("west")["online"], "East disconnect must not disturb the west lease")
    new_launch = control.payload(east_host, {"kind": "launch", "workspace_id": east_host["workspaces"][0]["workspace_id"],
                                             "name": "restart"})
    control.reject(new_launch, "host_offline")
    replay = control.submit(prior)
    require(replay == control.receipt(prior) and replay["status"] == "succeeded",
            "Exact replay must still return its original terminal outcome while the host is offline")
    east.command("reacquire-by-list", "list", "--json")
    second_peer = east.process("peer")
    require(second_peer != east_peer, "Reconnect must use a new native process incarnation")
    after = control.await_host("east")
    require(after["installation_id"] == east_host["installation_id"] and after["observed_at"] > offline["observed_at"],
            "Reconnect must retain installation identity and refresh the catalogue")
    require(west.process("peer")["pid"] != second_peer["pid"] or east.ssh,
            "Native stations must have independent local peer processes")
    new_launch["request_id"] = request_id()
    control.submit(new_launch)
    launched = control.succeeded(new_launch)
    stopped = stop(control, after, launched["outcome"]["agent"])
    return {"offline": offline, "after": after, "peer": second_peer, "default_launch": launched, "stop": stopped}


def exercise(fixture, physical, physical_codex=None, consumer=None, chromium=None, control_only=False, east_jump=None):
    with topology(fixture, physical, east_jump) as (east, west, ca, loss, polls), SandboxedModel(fixture) as model:
        created, scope = provision(fixture, SCOPE)
        control = PublicControl(fixture, created["token"], scope["generation"])
        consum = PhysicalConsumers(fixture, ca, chromium, consumer) if physical else None
        browser_receipt = None
        port = model_forward(fixture, east, model.server.server_port) if physical else None
        require(control.agents() == [], "Provisioned cloud scope must start with no Agents")
        def prepare(station, model_port):
            station.prepare_worker(scope["namespace_url"], ca, control.token, station.label, model, model_port,
                                   codex_bin=physical_codex)
            return cold_start(station)

        with ThreadPoolExecutor(max_workers=2) as pool:
            workers = [pool.submit(prepare, station, model_port) for station, model_port in ((east, port), (west, None))]
            east_peer, west_peer = (worker.result() for worker in workers)
        east_host, west_host = control.await_host("east"), control.await_host("west")
        require(east_host["installation_id"] != west_host["installation_id"] and control.agents() == [],
                "Both exact native installations must publish live capability before any Agent begins")
        if polls is not None:
            sample = wait_for("one real empty poll waits at least one second",
                              lambda: [row for row in polls if row["path"].endswith("/east/control/poll")
                                       and row["empty"] and 1 <= row["seconds"] <= 12], 20)
            fixture.write("idle-control-polls.json", sample)

        payload = control.payload(east_host, {"kind": "launch", "workspace_id": east_host["workspaces"][0]["workspace_id"],
                                              "role": "acceptance",
                                              "name": "receiver", "title": "Daemon acceptance",
                                              "initial_prompt": model.ready_prompt})
        rejected_controls(control, east_host, payload)
        if loss is not None:
            try:
                control.submit(payload)
            except http.client.RemoteDisconnected:
                pass
            else:
                raise AssertionError("Expected exactly one committed admin launch response to be discarded")
            require(any(attempt["dropped"] for attempt in loss), "Transport fault did not discard a committed response")
            replay = control.submit(payload)
            require(replay["request_id"] == payload["request_id"], "Retry recovered a different launch")
        else:
            browser_receipt = consum.launch(payload)
            payload["request_id"] = browser_receipt["receipt"]["request_id"]
        launched = control.succeeded(payload)
        require(control.submit(payload) == launched, "Completed launch must replay with the same exact receipt")
        control.reject({**payload, "command": {**payload["command"], "title": "Changed input"}}, "conflict")
        receiver = launched["outcome"]["agent"]
        wait_for("real remotely launched Codex receives explicit initial task",
                 lambda: model.seen("receiver-ready"), 40)
        require(control.agents() == [receiver], "Exactly one remote Agent may result from the launch and retries")
        east_worker, = east.process("workers")

        messages, sender, west_worker = None, None, None
        if not control_only:
            sender, messages = prove_round_trip(control, model, east, west, receiver, west_host)
            west_worker, = west.process("workers")
        diagnostics = east.command("online-diagnostics", "daemon", "status").stdout
        require("Cloud daemon: online" in diagnostics and scope["namespace_url"] in diagnostics,
                "Local diagnostics must identify the live cloud connection without its credential")
        stale = control.payload(east_host, {"kind": "stop", "name": receiver["name"], "incarnation": compact_id()})
        control.reject(stale, "stale_agent")
        require(receiver in control.agents(), "Stale stop had a side effect on the current exact target")
        east_stop = stop(control, east_host, receiver, consum.stop if consum else None)
        verify_worker_stopped(east, east_worker)
        if not control_only:
            require(len(model.seen("reply") or []) == 1 and len(model.seen("reply-observed-by-sender") or []) == 1,
                    "Replaying native sends delivered the same request or reply more than once")
        returned = reconnect(control, east, west, east_host, east_peer, payload)
        require(west.process("peer") == west_peer, "East restart must preserve the exact west daemon")
        stops = [east_stop]
        if sender:
            require(west.process("workers") == [west_worker] and control.agents() == [sender],
                    "East restart must preserve the exact west Agent")
            stops.append(stop(control, west_host, sender))
            verify_worker_stopped(west, west_worker)
        require(control.agents() == [], "Successful exact stops must leave no active Agents")
        fencing = lease_fencing(control) if not physical else None
        return {"topology": "physical HTTPS on three machines" if physical else "private HTTPS on one Linux machine",
                "generation": scope["generation"], "initial_hosts": [east_host, west_host], "remote_launch": launched,
                "confirmed_messages": messages, "exact_stops": stops, "reconnect": returned,
                "message_coverage": "excluded by --control-only" if control_only else "two confirmed directions",
                "wire_fencing": fencing,
                "physical_consumers": {"browser": browser_receipt, "cli_package_version": consum.package["version"]}
                    if consum else None,
                "provider_evidence": "model-observations.json", "model": "scripted Responses service",
                "committed_launch_response_discarded": loss is not None}


def main():
    options = argparse.ArgumentParser(description=__doc__)
    options.add_argument("--candidate", required=True, type=Path)
    options.add_argument("--evidence", required=True, type=Path)
    options.add_argument("--physical", nargs=3, metavar=("CLOUD_SSH", "CLOUD_PRIVATE_IP", "EAST_SSH"),
                         help="Use two explicitly authorized SSH hosts and this machine")
    options.add_argument("--physical-codex-bin", type=Path,
                         help="Stage codex and codex-code-mode-host from this local directory on the SSH worker")
    options.add_argument("--east-jump", help="Explicit SSH jump for the fixture's east worker")
    options.add_argument("--consumer", type=Path, help="External install root for the physical admin CLI")
    options.add_argument("--chromium", type=Path, help="Chromium or Chrome executable for the physical browser")
    options.add_argument("--control-only", action="store_true",
                         help="Verify the physical control plane separately; does not establish tool messaging")
    args = options.parse_args()
    if args.physical and not (args.consumer and args.chromium):
        options.error("--physical requires both --consumer and --chromium")
    if not args.physical and (args.physical_codex_bin or args.consumer or args.chromium or args.control_only or args.east_jump):
        options.error("Physical options require --physical")
    fixture = Deployment(args.candidate, args.evidence)
    case = "cloud-daemon-control" if args.control_only else "cloud-daemon"
    try:
        fixture.result["cases"][case] = {"status": "passed", **exercise(
            fixture, args.physical, args.physical_codex_bin, args.consumer, args.chromium, args.control_only, args.east_jump)}
    except Exception as error:
        fixture.result["cases"][case] = {"status": "failed", "error": str(error)}
        raise
    finally:
        fixture.close()
    print(f"{case}: passed; evidence: {fixture.directory / 'result.json'}")


if __name__ == "__main__":
    main()
