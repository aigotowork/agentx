"""Verify the embedded Space entry and the authenticated cloud conversation."""

from contextlib import closing
from html.parser import HTMLParser
import sqlite3
import uuid

from fixture import require


class AssetReferences(HTMLParser):
    def __init__(self, html):
        super().__init__()
        self.urls = []
        self.feed(html)

    def handle_starttag(self, tag, attrs):
        fields = dict(attrs)
        if tag == "script" and "src" in fields:
            self.urls.append(fields["src"])
        if tag == "link" and fields.get("rel") in ("stylesheet", "modulepreload"):
            self.urls.append(fields["href"])


def resource(fixture, method, path):
    with closing(fixture.connect()) as connection:
        connection.request(method, path)
        response = connection.getresponse()
        return response.status, dict((k.lower(), v) for k, v in response.getheaders()), response.read()


def assets(fixture, scope_id, generation):
    entry = f"/scopes/{scope_id}/space/"
    exact = f"access=scope&view=list&host=west&name=reader&incarnation=first&generation={generation}"
    status, fields, _ = resource(fixture, "GET", entry[:-1] + "?" + exact + "&token=discard-me")
    require(status == 307 and fields["location"] == entry + "?" + exact, "Canonical redirect must keep only nonsecret navigation")
    status, fields, body = resource(fixture, "GET", entry)
    require(status == 200 and fields["content-type"].startswith("text/html"), "Scoped Space must return the embedded document")
    require(fields["cache-control"] == "no-store" and fields["referrer-policy"] == "no-referrer", "Space shell must not cache or refer")
    policy = fields["content-security-policy"]
    require("script-src 'self';" in policy and "connect-src 'self';" in policy and "frame-ancestors 'none'" in policy,
            "Scope shell must allow only the intended same-origin scripts and requests")
    require(b"/var/lib" not in body and all(token.encode() not in body for token in fixture.tokens),
            "Space shell must not bind a local path or publish deployment credentials")
    urls = AssetReferences(body.decode()).urls
    require(bool(urls) and all(url.startswith("/space/") for url in urls), "Every entry resource must resolve from a scoped Space URL")
    for url in urls:
        asset_status, asset_headers, asset_body = resource(fixture, "GET", url)
        expect_cache = "public, max-age=31536000, immutable" if url.startswith("/space/assets/") else "no-store"
        require(asset_status == 200 and asset_body and asset_headers["cache-control"] == expect_cache, f"Wrong asset/cache policy: {url}")
        require(asset_headers["x-content-type-options"] == "nosniff" and
                asset_headers["content-type"].startswith(("text/javascript", "text/css")), f"Wrong asset content type: {url}")
    status, head, empty = resource(fixture, "HEAD", entry)
    require(status == 200 and not empty and head["content-type"] == fields["content-type"], "HEAD must match the Space representation")
    require(resource(fixture, "GET", "/scopes/unprovisioned/space/")[2] == body, "Unauthenticated shells must not reveal scope existence")
    for path in ("/scopes/-invalid/space/", "/scopes/%61lpha/space/", entry + "unknown", "/space/",
                 "/space/assets/not-present.js", "/api/v1/management", "/api/v1/agents"):
        status, headers, _ = resource(fixture, "GET", path)
        require(status == 404 and headers["cache-control"] == "no-store", f"Route must not alias Space/local APIs: {path}")
    require(resource(fixture, "POST", entry)[0] == 405, "Space shell must be read only")
    for path in ("/deployment", "/deployment/"):
        require(resource(fixture, "GET", path)[0] == 200, f"Deployment console must remain reachable: {path}")
    return urls


def run(fixture, provision):
    created, receipt = provision(fixture, "alpha")
    other, _ = provision(fixture, "neighbor")
    token, generation = created["token"], receipt["generation"]
    urls = assets(fixture, "alpha", generation)
    base = "/scopes/alpha/api/v1"
    history = base + "/messages"
    admin_scope = fixture.admin + "/scopes/alpha"
    admin_messages = admin_scope + "/messages"
    operations = fixture.admin + "/scopes/alpha/operations"

    def call(method, path, body=None, **options):
        return fixture.expect(method, base + path, token, body, **options)

    def operator_call(method, payload=None, **options):
        return fixture.expect(method, admin_messages, fixture.admin_token, payload, **options)

    def install(host):
        station = {"generation": generation, "installation_id": str(uuid.uuid4())}
        call("PUT", f"/hosts/{host}", station)
        return station

    def register(station, host, name, incarnation):
        return call("PUT", f"/agents/{name}", {**station, "host": host, "incarnation": incarnation})

    def send(station, sender, receiver, body, **fields):
        request_id = str(uuid.uuid4())
        data = {**station, "request_id": request_id, "from": sender, "to": f"@{receiver['host']}/{receiver['name']}",
                "body": body, "intent": "inform", **fields}
        return call("POST", "/messages", data), request_id

    west, east = install("west"), install("east")
    reader = register(west, "west", "reader", "first")
    counterpart = register(east, "east", "counterpart", "source")
    unrelated = register(east, "east", "unrelated", "other")
    query = {**reader, "generation": generation}

    require(call("GET", "/messages", query=query) == {"items": [], "next": None}, "Known empty registration must return an empty page")
    for denied in (None, other["token"], fixture.admin_token):
        fixture.expect("GET", history, denied, query=query, statuses=(401,))
    fixture.expect("POST", operations, token, {}, statuses=(401,))
    fixture.expect("GET", operations + "/any", token, statuses=(401,))
    require(call("GET", "/messages", query={**query, "generation": "stale"}, statuses=(409,))["code"] == "stale_generation",
            "History requires the current generation")
    for override in ({"limit": 0}, {"limit": 101}, {"before": "00"}, {"before": "9223372036854775808"}, {"extra": "rejected"}):
        call("GET", "/messages", query={**query, **override}, statuses=(400,))
    for override in ({"incarnation": "other"}, {"host": "east"}):
        call("GET", "/messages", query={**query, **override}, statuses=(404,))

    from concurrent.futures import ThreadPoolExecutor
    change_query = {"generation": generation}
    revisions = call("GET", "/changes", query=change_query)
    cursor = ".".join(str(revisions[k]) for k in ("epoch", "roster", "messages", "transcript"))
    with ThreadPoolExecutor(max_workers=1) as pool:
        waiting = pool.submit(fixture.expect, "GET", admin_scope + "/changes", fixture.admin_token,
                              query={**change_query, "cursor": cursor})
        first, message_request = send(east, counterpart, reader, "inbound")
        changed = waiting.result(timeout=5)
    require(changed["messages"] > revisions["messages"] and changed["roster"] == revisions["roster"],
            "Both bearers observe the committed message without invalidating directory pages")
    send(east, counterpart, unrelated, "not part of this reader history")
    reply, _ = send(west, reader, counterpart, "reply", intent="ack", reply_to=first["id"], thread="history")
    self_send, _ = send(west, reader, reader, "self")
    newer = call("GET", "/messages", query={**query, "after": first["id"], "limit": 1})
    require(newer == {"items": [reply], "next": reply["id"]}, "Forward reads must exclude the watermark and keep ascending order")
    require(call("GET", "/messages", query={**query, "after": newer["next"]}) == {"items": [self_send], "next": None},
            "Forward cursor must neither lose nor duplicate rows")
    first_page = call("GET", "/messages", query={**query, "limit": 2})
    require(first_page == {"items": [self_send, reply], "next": reply["id"]}, "History must merge directions in descending ID order without duplicate self-sends")
    send(west, reader, counterpart, "new after first page")
    older = call("GET", "/messages", query={**query, "before": first_page["next"]})
    require(older == {"items": [first], "next": None}, "Cursor must exclude the last ID and ignore newer inserts")
    require(call("GET", f"/operations/{message_request}")["status"] == "accepted", "History must not claim the inbox")
    status, headers, _ = resource(fixture, "GET", history)
    require(status == 401 and headers["cache-control"] == "no-store", "Auth errors must not cache")

    human_input = {"generation": generation, "request_id": message_request, "to": reader, "body": "Operator asks 雪"}
    for denied in (None, token, other["token"]):
        fixture.expect("GET", admin_messages, denied, query=query, statuses=(401,))
        fixture.expect("POST", admin_messages, denied, human_input, statuses=(401,))
    call("POST", "/messages", human_input, statuses=(400,))
    require(operator_call("GET", query={**query, "generation": "stale"}, statuses=(409,))["code"] == "stale_generation",
            "Admin history must use the member generation fence")
    require(operator_call("POST", {**human_input, "generation": "stale"}, statuses=(409,))["code"] == "stale_generation",
            "Admin send must require the current generation")
    operator_call("POST", {**human_input, "from": counterpart}, statuses=(400,))
    operator_call("POST", {**human_input, "to": {**reader, "host": "east"}}, statuses=(409,))
    human_message = operator_call("POST", human_input)
    operator = human_message["from"]
    operator_address = f"@{operator['host']}/{operator['name']}"
    require(human_message["to"] == reader and human_message["body"] == human_input["body"]
            and human_message["intent"] == "request" and human_message["id"] != first["id"]
            and human_message["reply_to"] is None and human_message["thread"] is None,
            "Human admission must assign its own sender and a separate ID from a scope send with the same request ID")
    require(call("GET", f"/operations/{message_request}")["message"] == first,
            "Admin admission must not replace a scope member's replay receipt")
    call("PUT", f"/hosts/{operator['host']}", west, statuses=(400,))
    call("PUT", f"/agents/{operator['name']}", {**west, "host": "west", "incarnation": operator["incarnation"]}, statuses=(400,))
    call("POST", "/messages", {**west, "request_id": str(uuid.uuid4()), "from": operator, "to": "@reader",
                               "body": "spoofed operator", "intent": "request"}, statuses=(400,))
    require(operator not in call("GET", "/agents")["items"]
            and {row["name"] for row in call("GET", "/hosts")["items"]} == {"west", "east"},
            "The server-owned operator must not appear as a host or Agent")
    inventory = fixture.expect("GET", admin_scope, fixture.admin_token)
    require((inventory["hosts"], inventory["online_hosts"], inventory["agents"]) == (2, 2, 3),
            "Server-owned operator must not affect inventory counts")
    inbox = call("POST", "/inbox/claim", {**west, "agent": reader, "claim_id": str(uuid.uuid4())})
    require(human_message in inbox["items"], "Operator messages must arrive through the ordinary inbox")

    answer_input = {**west, "request_id": str(uuid.uuid4()), "from": reader, "to": operator_address,
                    "body": "Answer for the operator", "intent": "ack", "reply_to": human_message["id"]}
    call("POST", "/messages", {**answer_input, "reply_to": None}, statuses=(400,))
    call("POST", "/messages", {**answer_input, "from": counterpart, "installation_id": east["installation_id"]}, statuses=(404,))
    answer = call("POST", "/messages", answer_input)
    require(answer["to"] == operator and answer["from"] == reader and answer["reply_to"] == human_message["id"],
            "The regular send path must persist the related Agent reply")
    require(call("GET", f"/operations/{answer_input['request_id']}")["status"] == "confirmed",
            "The operator reply is delivered directly to history")
    sent_page = operator_call("GET", query={**query, "limit": 2})
    require(sent_page == {"items": [answer, human_message], "next": human_message["id"]}
            and sent_page == call("GET", "/messages", query={**query, "limit": 2}),
            "Both read paths must expose the same durable conversation and cursor")

    call("POST", "/agents/reader/retire", {**west, "host": "west", "incarnation": "first"})
    replacement = register(east, "east", "reader", "replacement")
    replacement_query = {**replacement, "generation": generation}
    require(call("GET", "/messages", query=replacement_query) == {"items": [], "next": None},
            "A namesake on another host must not inherit the old history")
    require(call("GET", "/messages", query={**query, "before": first_page["next"]}) == older,
            "An exact retired registration must still resolve its own rows")
    require(operator_call("POST", human_input) == human_message, "Exact replay after retirement must return the original message")
    require(operator_call("POST", {**human_input, "body": "Different"}, statuses=(409,))["code"] == "conflict",
            "Changed replay must report a conflict even after the target retires")
    require(operator_call("POST", {**human_input, "request_id": str(uuid.uuid4())}, statuses=(409,))["code"] == "stale_agent",
            "A new human message must never follow an exact retired target to its namesake")

    # Cross the byte cap using valid UTF-8 bodies. Avoid journaling large fixture payloads.
    large = []
    for _ in range(17):
        body = {**east, "request_id": str(uuid.uuid4()), "from": counterpart, "to": "@east/reader", "intent": "inform", "body": "é" * 32768}
        status, item = fixture.request("POST", history, token, body, record=False)
        require(status == 200, "A 64 KiB body must fit the existing send contract")
        large.append(item["id"])
    status, bounded = fixture.request("GET", history, token, query=replacement_query, record=False)
    require(status == 200 and [msg["id"] for msg in bounded["items"]] == large[1:][::-1] and bounded["next"] == large[1],
            "Page size must be bounded by UTF-8 body bytes, with a usable cursor")
    status, final = fixture.request("GET", history, token, query={**replacement_query, "before": bounded["next"]}, record=False)
    require(status == 200 and [msg["id"] for msg in final["items"]] == large[:1] and final["next"] is None,
            "Byte-limit cursor must skip no rows")

    # Emulate the index state of a scope provisioned by the pre-Space deployment.
    fixture.stop(fixture.process)
    with closing(sqlite3.connect(fixture.runtime / "deployment/scopes/alpha/communication.sqlite3")) as storage:
        storage.executescript("DROP INDEX messages_from_history; DROP INDEX messages_to_history;")
    fixture.start()
    require(call("GET", "/messages", query={**query, "before": first_page["next"]}) == older,
            "Reopening a pre-Space scope must establish the additive indexes without losing messages")
    require(operator_call("GET", query={**query, "limit": 2}) == sent_page and operator_call("POST", human_input) == human_message,
            "Restart must preserve both sides of the conversation and operator replay identity")
    new_token = fixture.token()
    fixture.expect("POST", fixture.admin + "/scopes/alpha/token-rotations", fixture.admin_token,
                   {"request_id": str(uuid.uuid4()), "expected_version": 1, "token": new_token})
    fixture.expect("GET", history, token, query=query, statuses=(401,))
    require(fixture.expect("GET", history, new_token, query={**query, "before": first_page["next"]}) == older,
            "Rotated bearer must see the same history for the same generation")
    return {"assets": urls, "exact_agent": reader, "message_ids": [first["id"], reply["id"], self_send["id"]],
            "operator_message_ids": [human_message["id"], answer["id"]],
            "rotation_and_additive_upgrade": True, "deployment_starts": fixture.boots}
