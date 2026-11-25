"""Drive the embedded cloud Space through a real daemon, role launch and Codex reply."""
import argparse
import base64
import hashlib
import os
import re
import subprocess
from concurrent.futures import ThreadPoolExecutor
from contextlib import ExitStack
import json
from pathlib import Path
import tempfile
import time
import tomllib

from acceptance import provision
from daemon import PublicControl, TERMINAL, cold_start, stop
from daemon_rig import SCOPE, Station, use_https
from fixture import CLOUD_PROTOCOL, COMPACT_ID, LEGACY_TOKEN, WORKSPACE_ID, Deployment, exact, project, require, wait_for
from management import tls_front
from native import Model
from space import resource


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


def catalogue(control, base, token, host=None):
    """Read every page with a one-row limit to exercise the public opaque cursor."""
    items, cursors = [], set()
    query = {"limit": 1, **({"host": host} if host else {})}
    while True:
        page = control.fixture.expect("GET", base + "/workspaces", token, query=query)
        require(len(page["items"]) <= 1, "Workspace page ignored its limit")
        items.extend(page["items"])
        after = page["next"]
        if after is None:
            return items
        require(page["items"] and after not in cursors, "Workspace cursor did not advance")
        cursors.add(after)
        query["after"] = after


def roster(control, cursor=None):
    query = {"generation": control.generation, **({"cursor": cursor} if cursor else {})}
    return control.fixture.expect("GET", control.scope + "/changes", control.token, query=query)


def revision_cursor(snapshot):
    return ".".join(str(snapshot[key]) for key in ("epoch", "roster", "messages", "transcript"))


def assert_members(control, host, workspace_id, names):
    query = {"host": host, "workspace_id": workspace_id, "facts": "true"}
    active = control.agents(**query)
    read_only = control.fixture.expect("GET", control.scope + "/agents", control.token, query=query)
    membership = lambda rows: [{**exact(row), "workspace": row["workspace"]} for row in rows]
    require(read_only["next"] is None and membership(read_only["items"]) == membership(active),
            "Admin and Namespace disagree on exact project members")
    require({row["name"] for row in active} == set(names)
            and all(row["host"] == host and row["workspace"] == project(workspace_id) for row in active),
            f"Unexpected membership for {host}: {active}")
    return {row["name"]: row for row in active}


def browser(fixture, chromium, namespace, pin, token, phase, inputs):
    env = {**fixture.environment("browser-" + phase), "SPACE_ENTRY": namespace["namespace_url"] + "/space/",
           "SPACE_TOKEN": token, "SPACE_PIN": pin, "SPACE_BROWSER": str(chromium),
           "SPACE_EVIDENCE": str(fixture.directory), "SPACE_PHASE": phase, "SPACE_INPUTS": json.dumps(inputs)}
    helper = Path(__file__).with_suffix(".mjs").resolve()
    # Chromium may open a Unix socket under its profile. Keep the private path short.
    with tempfile.TemporaryDirectory(prefix="axns-chrome-", dir="/tmp") as profile:
        result = fixture.command("space-" + phase, ["node", helper],
                                 {**env, "TMPDIR": profile, "SPACE_CHROME_PROFILE": profile + "/profile"}, timeout=180)
    return json.loads(result.stdout)


def retired_interfaces(fixture, station, namespace, namespace_token):
    name = namespace["namespace_id"]
    paths = [f"/scopes/{name}/space/", f"/scopes/{name}/api/v1/descriptor",
             "/api/v1/admin/scopes", f"/api/v1/admin/scopes/{name}/workspaces"]
    statuses = {path: resource(fixture, "GET", path, fixture.admin_token if path.startswith(fixture.admin) else namespace_token)[0]
                for path in paths}
    require(set(statuses.values()) == {404}, f"Old public routes must not alias Namespace routes: {statuses}")
    before = (station.root / "scope/cloud.toml").read_bytes()
    common = ["daemon", "connect", "--configure-only", "--scope", station.root / "scope",
              "--host", "east", "--token-file", station.root / "scope-token"]
    for flag, value, other in (("--url", namespace["namespace_url"], ()),
                               ("--workspace", f"work={station.workspace}", ("--namespace-url", namespace["namespace_url"]))):
        result = station.command("retired-" + flag[2:], *common, *other, flag, value, check=False)
        require(result.returncode and flag in result.stderr, f"Retired enrollment option accepted: {flag}")
    fixture.tokens.append(LEGACY_TOKEN)
    token_file = station.root / "retired-scope-token"
    station.write(token_file, LEGACY_TOKEN + "\n")
    invalid = station.command("retired-enrollment-token", *common[:-1], token_file,
                              "--namespace-url", namespace["namespace_url"], check=False)
    require(invalid.returncode and "invalid namespace token" in invalid.stderr,
            "Native enrollment must reject a retired bearer at credential validation")
    require((station.root / "scope/cloud.toml").read_bytes() == before, "Rejected enrollment changed private configuration")
    return statuses


def check_diagnostic_protocol(fixture, chromium, ca, pin, namespace, control, overrides):
    descriptor = control.scope + "/descriptor"
    overview = fixture.admin + "/overview"
    require(fixture.expect("GET", descriptor, control.token)["protocol_version"] ==
            fixture.expect("GET", overview, fixture.admin_token)["protocol_version"] == CLOUD_PROTOCOL,
            "Both deployed descriptors must announce v2 on /api/v1")
    env = {**fixture.environment("direct-protocol"), "AGENTX_NAMESPACE_URL": namespace["namespace_url"],
           "AGENTX_NAMESPACE_TOKEN": control.token, "SSL_CERT_FILE": str(ca)}
    argv = [fixture.candidate, "list", "--json"]
    fixture.command("native-v2-protocol", argv, env)
    overrides.update({descriptor: 3, overview: 3})
    try:
        fixture.command("native-diagnostic-version", argv, env)
        base = control.scope.rstrip("/") + "/"
        return {access: browser(fixture, chromium, namespace, pin, token, "diagnostic-" + access,
                                {"apiPath": path, "resourceBase": resource, "generation": control.generation, "descriptor": descriptor, "overview": overview})
                for access, path, resource, token in (("namespace", descriptor, base, control.token),
                    ("admin", control.admin, control.admin + "/", fixture.admin_token))}
    finally:
        overrides.clear()


def exercise(fixture, chromium):
    with ExitStack() as resources:
        overrides = {}
        ca, pin, _ = resources.enter_context(tls_front(fixture, protocol_overrides=overrides))
        model = resources.enter_context(BrowserModel(fixture))

        def local_station(label):
            station = Station(fixture, label)
            resources.callback(station.close)
            station.initialize()
            return station

        east, west = (local_station(label) for label in ("east", "west"))
        fixture.start()
        use_https(fixture, ca)
        created, namespace = provision(fixture, SCOPE)
        control = PublicControl(fixture, created["token"], namespace["generation"])
        protocol_diagnostics = check_diagnostic_protocol(fixture, chromium, ca, pin, namespace, control, overrides)
        east_project, west_project = east.workspace / "shared", west.workspace / "shared"
        local_cwd, web_cwd = east_project / "nested/native", east_project / "nested/web"
        local_cwd.mkdir(parents=True)
        (east.workspace / "staging/deep-project").mkdir(parents=True)
        (east_project / "escape").symlink_to(east.root / "home", target_is_directory=True)
        # Neither the plain opened root nor a symlink to a recognized project should appear as a project.
        (west.workspace / "alias-of-east").symlink_to(east_project, target_is_directory=True)

        def prepare(station, path, role):
            station.prepare_worker(namespace["namespace_url"], ca, control.token, station.label, model, project=path, role=role)
            config = tomllib.loads((station.root / "scope/cloud.toml").read_text())
            require(config["namespace_url"] == namespace["namespace_url"] and config["host"] == station.label
                    and config["project_roots"] == {"work": str(station.workspace)}
                    and "workspaces" not in config and "url" not in config, "Private enrollment does not match the new contract")
            return cold_start(station)

        with ThreadPoolExecutor(max_workers=2) as pool:
            workers = [pool.submit(prepare, station, path, role) for station, path, role in
                       ((east, east_project, "east-special"), (west, west_project, "west-special"))]
            east_peer, west_peer = (worker.result() for worker in workers)
        east_host, west_host = (control.await_host(host, validate_default=False) for host in ("east", "west"))
        require(east_host["installation_id"] != west_host["installation_id"] and east_peer["pid"] != west_peer["pid"],
                "Candidates must run as two separate native installations")
        require(all(row["project_roots"] == ["work"] and row["workspace_management"] and row["directory_launch"]
                    for row in (east_host, west_host)), "Native leases must expose root eligibility and directory launch")
        initial = catalogue(control, control.scope, control.token)
        require(initial == catalogue(control, control.admin, fixture.admin_token), "Admin and Namespace catalogues must agree")
        distinct = {row["host"]: row for row in initial}
        require(len(initial) == 2 and set(distinct) == {"east", "west"}, "Only the two recognized direct projects may be exposed")
        east_row, west_row = distinct["east"], distinct["west"]
        east_id, west_id = east_row["workspace_id"], west_row["workspace_id"]
        require(east_id != west_id and all(row["name"] == "shared" and row["online"]
                    and re.fullmatch(WORKSPACE_ID, row["workspace_id"])
                    and set(row) == {"host", "online", "workspace_id", "name", "default_available", "default_reason", "roles"}
                    for row in initial), "Same-name projects must retain distinct opaque identities and no paths")
        for row, role in ((east_row, "east-special"), (west_row, "west-special")):
            require(catalogue(control, control.scope, control.token, row["host"]) == [row],
                    "A host filter must isolate the corresponding project")
            available = {entry["name"] for entry in row["roles"] if entry["available"]}
            require(available == {role}, f"Project roles leaked across installations: {row['host']} {available}")
            assert_members(control, row["host"], row["workspace_id"], [])

        control.reject(control.payload(east_host, {"kind": "launch", "workspace_id": west_id}), "capability_unavailable")
        control.reject(control.payload(east_host, {"kind": "launch", "workspace_id": east_id, "role": "west-special"}),
                       "capability_unavailable")
        control.reject(control.payload(east_host, {"kind": "launch", "workspace_id": east_id, "working_directory": "../home"}),
                       "invalid_input", status=400)
        escape = control.payload(east_host, {"kind": "launch", "workspace_id": east_id, "name": "escape",
                                           "working_directory": "escape/no-create", "create_directory": True})
        control.submit(escape)
        rejection = wait_for("native symlink refusal", lambda: (r if (r := control.receipt(escape))["status"] in TERMINAL else False), 35)
        require(rejection["outcome"] == {"status": "failed", "code": "directory_unsafe"}
                and not (east.root / "home/no-create").exists() and not control.agents(),
                "A symlink launch must not write outside the project or register an Agent")

        native_launch = east.command("native-project-launch", "launch", "--name", "localmember", "--role", "east-special",
                                     "--headless", "--go", "--json", "--agentx-prompt", model.ready_prompt, cwd=local_cwd)
        native = json.loads(native_launch.stdout)
        require(bool(re.fullmatch(COMPACT_ID, native["agent"]["incarnation"])), "New cloud incarnation must be compact")
        native_cwd = (Path("/proc") / str(native["pid"]) / "cwd").resolve(strict=True)
        require(wait_for("native Codex initial turn", lambda: model.seen("receiver-ready"), 35),
                "The local project launch must start a real Provider")
        local = wait_for("native exact membership published", lambda: next(
            (row for row in control.agents(host="east", workspace_id=east_id, facts="true") if row["name"] == "localmember"), None), 25)
        require(native_cwd == local_cwd and exact(local) == exact(native["agent"])
                and local["facts"] and local["facts"]["cwd"] == str(local_cwd) and local["facts"]["role"] == "east-special",
                "Native launch facts must retain the nested cwd and selected role")

        legacy_east = "ws_" + base64.urlsafe_b64encode(
            hashlib.sha256(b"agentx:cloud-project-workspace:v1\0" + os.fsencode(east_project.resolve())).digest()
        ).decode().rstrip("=")
        for base, token in ((control.scope, control.token), (control.admin, fixture.admin_token)):
            fixture.expect("GET", base + "/agents", token, query={"host": "east", "workspace_id": legacy_east}, statuses=(400,))
        control.reject(control.payload(east_host, {"kind": "launch", "workspace_id": legacy_east}), "invalid_input", status=400)

        # Explicit global and an unopened project's membership must be visible in the overview.
        unopened = east.root / "unopened-project"
        (unopened / ".agentx").mkdir(parents=True)
        for name, cwd, selectors in (("globalmember", local_cwd, ("--workspace", "global")),
                                     ("hiddenmember", unopened, ("--workspace", unopened))):
            east.command("native-" + name, "launch", "--name", name, "--profile", "acceptance-codex",
                         *selectors, "--headless", "--go", "--json", "--agentx-prompt", model.ready_prompt, cwd=cwd)
        nonproject = wait_for("global and unexposed Agents published", lambda: (
            rows if len(rows := [row for row in control.agents(host="east") if row["name"] != "localmember"]) == 2 else None), 25)
        require({row["name"]: row["workspace"] for row in nonproject} ==
                {"globalmember": {"kind": "global"}, "hiddenmember": {"kind": "unexposed"}},
                "Native launch must distinguish explicit Global and an unopened project")

        before_add = roster(control)
        inputs = {"generation": control.generation, "east": east_id, "west": west_id, "local": exact(local),
                  "retiredWorkspace": legacy_east,
                  "cwd": str(web_cwd), "message": model.operator_message, "reply": model.operator_reply,
                  "progress": model.progress, "progressSeen": str(fixture.runtime / "progress-seen")}
        admin = browser(fixture, chromium, namespace, pin, fixture.admin_token, "admin", inputs)
        require(model.seen("browser-final"), "A real Codex Provider must observe the browser message")
        members = assert_members(control, "east", east_id, ["localmember", "receiver"])
        assert_members(control, "west", west_id, [])
        assert_members(control, "west", east_id, [])
        agent = members["receiver"]
        require(exact(agent) == admin["exact"] and agent["facts"]["cwd"] == str(web_cwd)
                and agent["facts"]["role"] == "east-special", "Browser exact route must match the native registration and facts")
        launch_receipt = control.receipt({"request_id": admin["launchRequest"], "generation": control.generation,
            "host": "east", "installation_id": east_host["installation_id"],
            "command": {"kind": "launch", "workspace_id": east_id, "role": "east-special", "name": "receiver",
                        "working_directory": "nested/web", "create_directory": True}})
        require(launch_receipt["status"] == "succeeded" and exact(launch_receipt["outcome"]["agent"]) == exact(agent)
                and launch_receipt["outcome"]["agent"]["workspace"] == project(east_id),
                "The managed launch receipt must report the actual exact registration and workspace")
        supervisor = east.root / "scope/cloud-client/supervisors" / (agent["incarnation"] + ".json")
        witness = json.loads(supervisor.read_text())
        observed_cwd = (Path("/proc") / str(witness["pid"]) / "cwd").resolve(strict=True)
        require(witness["ready"] and observed_cwd == web_cwd,
                "The verified launch supervisor must run in the cwd actually created from Space")

        added = {}
        for name, directory, create in (("Deep project", "staging/deep-project", False), ("New project", "new-project", True)):
            receipt = control.receipt({"request_id": admin["addRequests"][name], "generation": control.generation,
                "host": "east", "installation_id": east_host["installation_id"],
                "command": {"kind": "add_workspace", "parent": "work", "name": name, "directory": directory,
                            "create_directory": create}})
            require(receipt["status"] == "succeeded" and receipt["outcome"]["status"] == "workspace_added"
                    and (east.workspace / directory).is_dir(), "Browser registration must have a durable actual directory effect")
            added[name] = receipt
        before = catalogue(control, control.scope, control.token)
        require({row["name"]: row["workspace_id"] for row in before if row["host"] == "east"} ==
                {"shared": east_id, **{name: rec["outcome"]["workspace_id"] for name, rec in added.items()}},
                "Public catalogue must match both explicit receipts and the automatically discovered project")
        require(roster(control)["roster"] > before_add["roster"], "Project and member changes must advance roster")
        future_stop = control.payload(east_host, {"kind": "stop", "name": agent["name"], "incarnation": agent["incarnation"]})
        fixture.expect("POST", control.operations, control.token, future_stop, statuses=(401, 403))
        fixture.expect("GET", control.operations + "/" + future_stop["request_id"], fixture.admin_token, statuses=(404,))
        require(exact(assert_members(control, "east", east_id, ["localmember", "receiver"])["receiver"]) == exact(agent),
                "A Namespace bearer creating a valid Admin operation must have no side effect")
        stop(control, east_host, agent)
        historical = fixture.expect("GET", control.scope + "/transcript", control.token,
                                    query={**exact(agent), "generation": control.generation})
        require(historical["agent"] == {**exact(agent), "workspace": project(east_id)},
                "The exact retired Agent must retain its original project membership")
        for row in (local, *nonproject):
            stop(control, east_host, row)
        require(not control.agents(), "Exact Admin stops must retire every fixture Agent before offline simulation")

        lease = control.capabilities("east")
        stable = roster(control)
        wait_for("an identical renewal advances the observation timestamp",
                 lambda: control.capabilities("east")["observed_at"] > lease["observed_at"], 20, interval=0.8)
        require(roster(control)["roster"] == stable["roster"], "Identical native renewals must not invalidate the roster")
        stable = roster(control)
        observed_at = control.capabilities("east")["observed_at"]
        require(east.process("stop") == east_peer, "Offline simulation must stop only the verified east daemon")
        wait_for("east lease approaches expiration", lambda: time.time() > observed_at + 21, 28, interval=0.3)
        offline_revision = roster(control, revision_cursor(stable))
        require(offline_revision["roster"] > stable["roster"], "Lease expiry must wake a waiting roster reader")
        offline = catalogue(control, control.scope, control.token)
        require(offline == [{**row, "online": row["host"] == "west"} for row in before]
                and offline == catalogue(control, control.admin, fixture.admin_token)
                and west.process("peer") == west_peer, "Offline catalogue must retain all east IDs while west stays available")
        read_only = browser(fixture, chromium, namespace, pin, control.token, "namespace",
                            {**inputs, "readUrl": admin["restoredOnReload"], "added": {name: rec["outcome"]["workspace_id"] for name, rec in added.items()}})

        east.command("restart-native-daemon", "list", "--json")
        restored_host = control.await_host("east", validate_default=False)
        restored_peer = east.process("peer")
        restored = catalogue(control, control.scope, control.token)
        require(restored_peer != east_peer and restored_host["installation_id"] == east_host["installation_id"]
                and restored == before, "Restart must preserve installation identity, explicit registrations and discovered IDs")
        retired = retired_interfaces(fixture, east, namespace, control.token)
        return {"namespace_id": namespace["namespace_id"], "generation": control.generation,
                "installations": {"east": east_host["installation_id"], "west": west_host["installation_id"]},
                "peers": {"east": east_peer, "west": west_peer, "east_restarted": restored_peer},
                "initial_catalogue": initial, "offline_catalogue": offline, "restored_catalogue": restored,
                "exact_project_members": members, "nonproject_members": nonproject, "browser": {"admin": admin, "namespace": read_only},
                "managed_launch": launch_receipt, "explicit_registrations": added,
                "native_supervisor_cwd": str(native_cwd), "managed_supervisor_cwd": str(observed_cwd),
                "roster": {"before_expiry": stable, "after_expiry": offline_revision},
                "retired_http_statuses": retired, "protocol_diagnostics": protocol_diagnostics,
                "retired_workspace_http_rejected": True, "read_only_admin_command_denied": True}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--chromium", default="/usr/bin/google-chrome", type=Path)
    args = parser.parse_args()
    fixture = Deployment(args.candidate, args.evidence)
    try:
        fixture.result["cases"]["namespace-space"] = {"status": "passed",
            **exercise(fixture, args.chromium.resolve(strict=True))}
    except Exception as error:
        fixture.result["cases"]["namespace-space"] = {"status": "failed", "error": fixture.redact(str(error))}
        raise
    finally:
        fixture.close()
    print(f"namespace-space: passed; evidence: {fixture.directory / 'result.json'}")
