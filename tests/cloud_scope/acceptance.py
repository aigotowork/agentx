#!/usr/bin/env python3
"""Exercise the native deployment and selected public consumers in a fresh Namespace."""

import argparse
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing, nullcontext
from pathlib import Path
import re
import sqlite3
import subprocess
import time
import uuid

from fixture import CLOUD_PROTOCOL, COMPACT_ID, LEGACY_TOKEN, Deployment, UNEXPOSED, compact_id, exact, require
import management
import native
import space


AUTH_DENIED = (401, 403)
CREATED = (200, 201)


def request_id():
    return str(uuid.uuid4())


def provision(fixture, name):
    payload = {"request_id": request_id(), "token": fixture.token()}
    receipt = fixture.expect("PUT", fixture.admin + f"/namespaces/{name}", fixture.admin_token, payload, statuses=CREATED)
    require(receipt["namespace_id"] == name and receipt["credential_version"] == 1 and bool(receipt["generation"]),
            "Provisioning must establish an exact scope generation and credential version 1")
    require(receipt["namespace_url"] == fixture.public_url + f"/namespaces/{name}", "Receipt must publish the configured HTTPS URL")
    return payload, receipt


def deployment(fixture):
    admin, admin_token = fixture.admin, fixture.admin_token
    scope = "/namespaces/alpha/api/v1"
    fixture.expect("GET", admin + "/overview", statuses=AUTH_DENIED)
    empty = fixture.expect("GET", admin + "/overview", admin_token)
    require(empty["namespaces"] == empty["hosts"] == empty["agents"] == 0, "Fresh deployment must start empty")
    require(empty["deployment_id"] and empty["protocol_version"] == CLOUD_PROTOCOL, "Overview must identify the v2 deployment")

    env = fixture.environment("retired-credential")
    minted = subprocess.check_output([fixture.candidate, "cloud-token"], env=env, text=True, timeout=5).strip()
    fixture.write("compact-token.json", {"length": len(minted), "ascii_alphanumeric": bool(re.fullmatch(COMPACT_ID, minted))})
    require(bool(re.fullmatch(COMPACT_ID, minted)), "Native CLI must mint a 12 character ASCII bearer")
    fixture.tokens.append(LEGACY_TOKEN)

    def rejected_start(label, root, token, diagnostic):
        result = fixture.command(label, [fixture.candidate, "daemon", "cloud", "--root", root,
            "--listen", "127.0.0.1:0", "--public-url", fixture.public_url],
            {**env, "AGENTX_ADMIN_TOKEN": token}, check=False, timeout=5)
        require(result.returncode != 0 and diagnostic in result.stderr.lower(), f"Deployment must reject {label} at startup")

    rejected_start("retired-admin-token", fixture.runtime / "retired-admin", LEGACY_TOKEN, "token")
    legacy_root = fixture.runtime / "retired-catalogue"
    legacy_root.mkdir(mode=0o700)
    with closing(sqlite3.connect(legacy_root / "deployment.sqlite3")) as database:
        database.executescript("PRAGMA user_version=1; CREATE TABLE deployment(singleton INTEGER PRIMARY KEY, id TEXT NOT NULL); INSERT INTO deployment VALUES(1,'legacy-fixture');")
        rejected_start("retired-catalogue", legacy_root, admin_token, "database format")
        require(database.execute("PRAGMA user_version").fetchone() == (1,)
                and database.execute("SELECT id FROM deployment").fetchone() == ("legacy-fixture",),
                "Refusing a retired catalogue must not migrate or replace it")
    fixture.expect("PUT", admin + "/namespaces/retired-token", admin_token,
                   {"request_id": request_id(), "token": LEGACY_TOKEN}, statuses=(400,))

    create, first = provision(fixture, "alpha")
    neighbor, _ = provision(fixture, "bravo")
    for token in (None, neighbor["token"], admin_token):
        fixture.expect("GET", scope + "/descriptor", token, statuses=AUTH_DENIED)
    fixture.expect("GET", admin + "/overview", create["token"], statuses=AUTH_DENIED)
    scope_before = fixture.expect("GET", scope + "/descriptor", create["token"])
    require(scope_before == {"namespace_id": "alpha", "generation": first["generation"], "protocol_version": CLOUD_PROTOCOL},
            "Descriptor must identify the created scope and protocol")

    page = fixture.expect("GET", admin + "/namespaces", admin_token, query={"limit": 1})
    require([item["namespace_id"] for item in page["items"]] == ["alpha"] and page["next"], "First page must expose alpha and a cursor")
    page = fixture.expect("GET", admin + "/namespaces", admin_token, query={"limit": 1, "after": page["next"]})
    require([item["namespace_id"] for item in page["items"]] == ["bravo"] and page["next"] is None, "Second page must terminate at bravo")

    rotates = admin + "/namespaces/alpha/token-rotations"
    rotation = {"request_id": request_id(), "token": fixture.token(), "expected_version": 1}
    fixture.expect("POST", rotates, admin_token, {**rotation, "token": LEGACY_TOKEN}, statuses=(400,))
    changed = fixture.expect("POST", rotates, admin_token, rotation, statuses=CREATED)
    require(changed == {**first, "credential_version": 2}, "Rotation must only advance the credential version")
    fixture.expect("POST", rotates, admin_token, {**rotation, "request_id": request_id(), "token": fixture.token()}, statuses=(409,))
    fixture.expect("POST", rotates, admin_token, {**rotation, "token": fixture.token()}, statuses=(409,))
    fixture.expect("GET", scope + "/descriptor", create["token"], statuses=AUTH_DENIED)
    require(fixture.expect("GET", scope + "/descriptor", rotation["token"]) == scope_before,
            "New credential must authenticate the same scope generation")

    fixture.stop(fixture.process)
    fixture.start()
    require(fixture.expect("POST", rotates, admin_token, rotation, statuses=CREATED) == changed,
            "Exact rotation replay after restart must return the same receipt")
    require(fixture.expect("PUT", admin + "/namespaces/alpha", admin_token, create, statuses=CREATED) == first,
            "Original create replay must keep its original receipt after rotation and restart")
    fixture.expect("GET", scope + "/descriptor", create["token"], statuses=AUTH_DENIED)
    require(fixture.expect("GET", scope + "/descriptor", rotation["token"]) == scope_before,
            "New credential and scope generation must survive restart")
    fixture.expect("GET", "/namespaces/bravo/api/v1/descriptor", neighbor["token"])

    # Break only the neighbor's private inventory after a clean stop, leaving its catalog entry intact.
    fixture.stop(fixture.process)
    (fixture.runtime / "deployment/namespaces/bravo").rename(fixture.runtime / "detached-bravo")
    fixture.start()
    neighbor_state = fixture.expect("GET", admin + "/namespaces/bravo", admin_token)
    require(neighbor_state["namespace_id"] == "bravo" and neighbor_state["available"] is False,
            "Missing scope storage must leave a visible unavailable catalog row")
    overview = fixture.expect("GET", admin + "/overview", admin_token)
    require(overview["namespaces"] == 2 and overview["unavailable"] == 1, "Deployment overview must count the damaged scope")
    require(overview["deployment_id"] == empty["deployment_id"] and overview["protocol_version"] == empty["protocol_version"],
            "Deployment and protocol identity must survive listener restarts")
    fixture.expect("GET", admin + "/namespaces/bravo/hosts", admin_token, statuses=(503,))
    require(fixture.expect("GET", admin + "/namespaces/alpha", admin_token)["available"] is True,
            "Damaged neighbor must not disturb the healthy scope")
    return {"scope_receipts": [first, changed], "deployment_starts": fixture.boots,
            "retired_bearer_rejected": True, "retired_catalogue_rejected": True,
            "offline_scope_inventory": neighbor_state}


def communication(fixture):
    created, scope = provision(fixture, "messages")
    other_credential, other_namespace = provision(fixture, "neighbor")
    base = "/namespaces/messages/api/v1"
    token = created["token"]
    generation = scope["generation"]

    def call(method, route, payload=None, **kwargs):
        return fixture.expect(method, base + route, token, payload, **kwargs)

    def station(host, name):
        installation = {"generation": generation, "installation_id": request_id()}
        identity = {"name": name, "host": host, "incarnation": compact_id(), "workspace": UNEXPOSED}
        host_row = call("PUT", f"/hosts/{host}", installation, statuses=CREATED)
        require(host_row["name"] == host and host_row["online"] is True, "Registered host must appear online")
        registration = {**installation, "host": host, "incarnation": identity["incarnation"], "workspace": UNEXPOSED}
        require(call("PUT", f"/agents/{name}", registration,
                     statuses=CREATED) == identity, "Agent registration returned a different identity")
        return installation, identity

    west, sender = station("west", "sender")
    east, receiver = station("east", "receiver")
    call("PUT", "/hosts/east", west, statuses=(409,))
    call("PUT", "/hosts/east", {**east, "generation": request_id()}, statuses=(409,))
    call("PUT", "/agents/receiver", {**west, "host": "west", "incarnation": compact_id(), "workspace": UNEXPOSED},
         statuses=(409,))
    for old in (request_id(), "18446744073709551615"):
        call("PUT", "/agents/retiredformat", {**east, "host": "east", "incarnation": old, "workspace": UNEXPOSED}, statuses=(400,))
        call("GET", "/messages", query={**exact(receiver), "incarnation": old, "generation": generation}, statuses=(400,))

    admin_agents = fixture.expect("GET", fixture.admin + "/namespaces/messages/agents", fixture.admin_token)
    require(sorted(admin_agents["items"], key=lambda agent: agent["name"]) == [receiver, sender],
            "Deployment admin must see both exact communication identities")
    admin_hosts = fixture.expect("GET", fixture.admin + "/namespaces/messages/hosts", fixture.admin_token)
    require(sorted(host["name"] for host in admin_hosts["items"]) == ["east", "west"], "Admin host inventory must agree with enrollment")
    fixture.expect("GET", "/namespaces/neighbor/api/v1/agents", token, statuses=AUTH_DENIED)
    other = fixture.expect("GET", "/namespaces/neighbor/api/v1/agents", other_credential["token"])
    require(other == {"generation": other_namespace["generation"], "items": [], "next": None}, "Independent scope must not inherit agents")

    outgoing = {**west, "request_id": request_id(), "from": exact(sender), "to": "@east/receiver",
                "body": "cross-host request 雪", "intent": "request", "thread": "cloud-acceptance"}
    call("POST", "/messages", {**outgoing, "request_id": request_id(), "to": "@west/receiver"}, statuses=(404, 409))
    message = call("POST", "/messages", outgoing, statuses=(200, 201, 202))
    require(exact(message["from"]) == exact(sender) and exact(message["to"]) == exact(receiver)
            and message["body"] == outgoing["body"]
            and isinstance(message["id"], str) and message["id"].isascii() and message["id"].isdecimal(),
            "Message receipt must preserve exact endpoints, text and decimal wire ID")
    require(call("POST", "/messages", outgoing, statuses=(200, 201, 202)) == message,
            "Exact retry must return the original message")
    call("POST", "/messages", {**outgoing, "body": "different input"}, statuses=(409,))

    operation = f"/operations/{outgoing['request_id']}"
    state = call("GET", operation)
    require(state["status"] == "accepted" and state["message"] == message, "Lookup must recover admission and exact message")
    claim = {**east, "agent": receiver, "claim_id": request_id()}
    call("POST", "/inbox/claim", {**claim, "installation_id": west["installation_id"]}, statuses=(401, 403, 409))
    batch = call("POST", "/inbox/claim", {**claim, "limit": 1})
    require(batch["claim_id"] == claim["claim_id"] and batch["items"] == [message], "Claim must acquire exactly one message")
    require(call("GET", operation)["status"] == "claimed", "Operation must reflect the claim")
    fixture.stop(fixture.process)
    fixture.start()
    require(call("POST", "/inbox/recover", claim) == batch, "Recovery after restart must preserve the outstanding claim and exact message")
    confirmation = {**claim, "message_ids": [message["id"]]}
    receipts = [call("POST", "/inbox/confirm", confirmation) for _ in range(2)]
    require(receipts == [{"confirmed": [message["id"]]}] * 2, "Confirm must be idempotent when its first response was unavailable")
    require(call("GET", operation)["status"] == "confirmed", "Operation must reflect confirmation")
    require(not call("POST", "/inbox/claim", {**claim, "claim_id": request_id()})["items"], "Confirmed message must not be redelivered")

    new_token = fixture.token()
    rotate = {"request_id": request_id(), "token": new_token, "expected_version": 1}
    old_wait = {**claim, "claim_id": request_id(), "wait_ms": 3000}
    with ThreadPoolExecutor(max_workers=1) as pool:
        held = pool.submit(fixture.request, "POST", base + "/inbox/claim", token, old_wait)
        time.sleep(0.25)  # Let the outstanding empty claim enter its bounded wait.
        require(not held.done(), "Empty claim must wait before rotation")
        fixture.expect("POST", fixture.admin + "/namespaces/messages/token-rotations",
                       fixture.admin_token, rotate, statuses=CREATED)
        status, body = held.result(timeout=5)
        require(status in AUTH_DENIED and isinstance(body, dict) and body.get("code"),
                "A claim already waiting under the retired credential must be rejected")
    token = new_token
    require(call("GET", operation)["status"] == "confirmed", "Rotation must preserve admitted message and claim data")

    reply_input = {**east, "request_id": request_id(), "from": receiver, "to": "@sender", "intent": "ack",
                   "body": "cross-host reply", "reply_to": message["id"], "thread": outgoing["thread"]}
    reply = call("POST", "/messages", reply_input, statuses=(200, 201, 202))
    require(exact(reply["to"]) == exact(sender) and exact(reply["from"]) == exact(receiver) and reply["reply_to"] == message["id"]
            and reply["thread"] == outgoing["thread"], "Reply must carry the original exact relation")
    received = call("POST", "/inbox/claim", {**west, "agent": sender, "claim_id": request_id()})
    require(received["items"] == [reply], "Sender must receive the related reply")

    retirement = {**east, "host": receiver["host"], "incarnation": receiver["incarnation"], "workspace": UNEXPOSED}
    require(exact(call("POST", "/agents/receiver/retire", retirement)) == exact(receiver), "Retirement must target the exact Agent")
    call("PUT", "/agents/receiver", retirement, statuses=(409,))
    replacement = call("PUT", "/agents/receiver", {**retirement, "incarnation": compact_id()}, statuses=CREATED)
    require(replacement["name"] == receiver["name"] and replacement["incarnation"] != receiver["incarnation"],
            "Retired short name may be used by a new incarnation only")
    require(call("POST", "/messages", outgoing, statuses=(200, 201, 202)) == message,
            "Replay after a namesake replaces its target must keep the old exact endpoint")
    require(not call("POST", "/inbox/claim", {**east, "agent": replacement, "claim_id": request_id()})["items"],
            "New incarnation must not inherit the old Agent's inbox")

    wait = {**east, "agent": replacement, "claim_id": request_id(), "wait_ms": 3500}
    with ThreadPoolExecutor(max_workers=1) as pool:
        sleeping = pool.submit(call, "POST", "/inbox/claim", wait)
        time.sleep(0.25)
        require(not sleeping.done(), "Receiver must still wait before the next message is admitted")
        wake_message = call("POST", "/messages", {**outgoing, "request_id": request_id(), "body": "wake outstanding poll"},
                            statuses=(200, 201, 202))
        observed = sleeping.result(timeout=2)
        require(observed["items"] == [wake_message] and exact(wake_message["to"]) == exact(replacement),
                "An already waiting receiver must claim a newly accepted send to its exact incarnation")
    return {"request": message, "reply": reply, "operation": operation, "wake_message": wake_message,
            "retired_incarnations_rejected": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--suite", choices=("deployment", "communication", "space", "native", "cli", "tui", "browser"), required=True)
    parser.add_argument("--consumer", type=Path, help="external project with an installed @agentx/client package (CLI suite)")
    parser.add_argument("--chromium", type=Path, help="real Chromium executable (browser suite)")
    parser.add_argument("--native-https", action="store_true", help="use the private HTTPS terminator and CA for the native suite")
    args = parser.parse_args()
    if args.suite == "cli" and args.consumer is None:
        parser.error("--consumer is required for CLI acceptance")
    if args.suite == "browser" and args.chromium is None:
        parser.error("--chromium is required for browser acceptance")
    if args.native_https and args.suite != "native":
        parser.error("--native-https requires the native suite")

    fixture = Deployment(args.candidate, args.evidence)
    try:
        drop = ("PUT", fixture.admin + "/namespaces/cli-managed") if args.suite == "cli" else (None, None)
        needs_tls = args.suite in ("cli", "browser") or args.native_https
        transport = management.tls_front(fixture, *drop) if needs_tls else nullcontext((None, None, None))
        with transport as (certificate, pin, attempts):
            fixture.start()
            if args.suite == "native":
                created, _ = provision(fixture, "native-agents")
                evidence = native.run(fixture, created["token"], certificate)
            elif args.suite == "space":
                evidence = space.run(fixture, provision)
            elif args.suite == "cli":
                evidence = management.cli(fixture, args.consumer.resolve(), certificate, attempts)
            elif args.suite == "browser":
                evidence = management.browser(fixture, args.chromium.resolve(strict=True), pin)
            else:
                case = {"deployment": deployment, "communication": communication, "tui": management.tui}[args.suite]
                evidence = case(fixture)
        fixture.result["cases"][args.suite] = {"status": "passed", **evidence}
    except Exception as error:
        fixture.result["cases"][args.suite] = {"status": "failed", "error": str(error)}
        raise
    finally:
        fixture.close()
    print(f"{args.suite}: passed; evidence: {fixture.directory / 'result.json'}")


if __name__ == "__main__":
    main()
