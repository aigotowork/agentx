"""Read a few exact live history pages without logging message bodies or writing data."""

import argparse
import json
import sqlite3
import struct
import time
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import quote, urlencode
from urllib.request import ProxyHandler, build_opener


def select_samples(scope, incident):
    """Use indexed metadata seeks to choose at most two conversations per scope."""
    with sqlite3.connect((scope / "agentx.db").as_uri() + "?mode=ro", uri=True, timeout=0.25) as db:
        deadline = time.monotonic() + 0.5
        db.set_progress_handler(lambda: int(time.monotonic() > deadline), 1000)
        covered, ceiling = db.execute("SELECT through_id,ceiling_id FROM conversation_route_coverage").fetchone()
        assert covered >= ceiling, "Conversation index backfill is incomplete"
        stop = """SELECT instance,id,CAST(json_extract(data,'$.snapshot.created_at') AS REAL)
                  FROM events WHERE type='life' AND json_extract(data,'$.action')='stopped'
                  AND json_type(data,'$.snapshot')='object'
                  AND COALESCE(json_extract(data,'$.snapshot.origin_device_id'),'')=''"""
        samples = {}
        if incident:
            current = db.execute("SELECT created_at FROM instances WHERE name=? AND COALESCE(origin_device_id,'')=''", (incident,)).fetchone()
            stopped = db.execute(stop + " AND instance=? ORDER BY id DESC LIMIT 1", (incident,)).fetchone()
            if current:
                bits, = struct.unpack(">Q", struct.pack(">d", current[0]))
                samples["previously failing name"] = (incident, {"incarnation": str(bits)})
            elif stopped:
                samples["previously failing name"] = (incident, {"stopped_event": str(stopped[1])})
            else:
                raise RuntimeError("No current or retained stopped reference for the incident name")
        for name, end, start in db.execute(stop + " ORDER BY id DESC LIMIT 24"):
            if start is None:
                continue
            # Limit 21 serves as a cheap cutoff; the public endpoint still owns
            # the exact incarnation bounds and determines the actual page.
            rows = db.execute(
                """SELECT 1 FROM conversation_routes AS r CROSS JOIN events AS e ON e.id=r.event_id
                   WHERE r.participant=? AND r.event_id<=? AND unixepoch(e.timestamp)>=?
                   ORDER BY r.event_id DESC LIMIT 21""",
                (name, end, int(start)),
            ).fetchall()
            if len(rows) == 21:
                samples["retained long conversation"] = (name, {"stopped_event": str(end)})
                break
        return {"through": covered, "ceiling": ceiling}, samples


def read_sample(get, generation, name, reference, compare_legacy):
    """Verify order, independent live watermark, and non-overlapping pages."""
    base = "/agents/" + quote(name, safe="")
    params = {"generation": generation, **reference, "limit": 20}
    result = {"kind": next(iter(reference))}

    def page(label, path, extra=None):
        status, data, elapsed = get(path, {**params, **(extra or {})})
        summary = {"status": status, "elapsedMs": elapsed}
        result[label] = summary
        if status == 200:
            ids = [int(message["id"]) for message in data["messages"]]
            assert ids == sorted(set(ids)) and len(ids) <= 20 and data["generation"] == generation
            summary["records"] = len(ids)
            return data, ids
        return None, []

    latest, newer_ids = page("latest", base + "/message-history")
    if latest is not None:
        assert latest["live_after"] is not None
        if latest["next_before"]:
            older, older_ids = page("older", base + "/message-history", {"before": latest["next_before"]})
            if older is not None:
                assert older["live_after"] is None and older_ids and max(older_ids) < min(newer_ids)
        _, ids = page("catchup", base + "/messages", {"after": latest["live_after"]})
        assert all(event_id > int(latest["live_after"]) for event_id in ids)
    if compare_legacy:
        page("legacyFirstPage", base + "/messages")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scope", type=Path)
    parser.add_argument("port", type=int)
    parser.add_argument("--incident")
    args = parser.parse_args()
    scope = args.scope.resolve(strict=True)
    coverage, samples = select_samples(scope, args.incident)
    opener = build_opener(ProxyHandler({}))

    def get(path, query=None):
        start = time.monotonic()
        url = f"http://127.0.0.1:{args.port}/api/v1{path}" + ("?" + urlencode(query) if query else "")
        try:
            with opener.open(url, timeout=4) as response:
                return response.status, json.load(response), round((time.monotonic() - start) * 1000)
        except HTTPError as error:
            with error:
                return error.code, None, round((time.monotonic() - start) * 1000)

    _, descriptor, _ = get("/management")
    assert descriptor["scope"] == str(scope) and descriptor["supports_message_history"] is True
    result = {"port": args.port, "coverage": coverage, "samples": {}}
    for label, (name, reference) in samples.items():
        result["samples"][label] = read_sample(get, descriptor["generation"], name, reference, label.startswith("previously"))
    print(json.dumps(result, indent=2))
    status_ok = all(page["status"] == 200 for sample in result["samples"].values() for page in sample.values() if isinstance(page, dict))
    return 0 if status_ok and "older" in result["samples"].get("retained long conversation", {}) else 1


if __name__ == "__main__":
    raise SystemExit(main())
