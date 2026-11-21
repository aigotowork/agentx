#!/usr/bin/env python3
"""Compare launch selectors and their exact result sets on disposable SQLite."""

import json
import sqlite3
import time


def measure(db, label, sql, args=()):
    plan = [r[3] for r in db.execute("EXPLAIN QUERY PLAN " + sql, args)]
    ticks = 0

    def count():
        nonlocal ticks
        ticks += 1
        return 0

    db.set_progress_handler(count, 100)
    start = time.perf_counter()
    rows = sorted(db.execute(sql, args).fetchall())
    elapsed = time.perf_counter() - start
    db.set_progress_handler(None, 0)
    print(json.dumps({"query": label, "ms": round(elapsed * 1000, 2), "vm_steps_floor_100": ticks * 100,
                      "rows": len(rows), "plan": plan}), flush=True)
    return rows


def main():
    db = sqlite3.connect(":memory:")
    db.executescript("""
        CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT);
        CREATE TABLE events (id INTEGER PRIMARY KEY, type TEXT, instance TEXT, data TEXT);
        CREATE INDEX idx_type ON events(type);
        CREATE INDEX idx_type_instance ON events(type, instance);
        WITH RECURSIVE ids(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM ids WHERE i<150000)
        INSERT INTO kv SELECT 'history:' || i, '{}' FROM ids;
        WITH RECURSIVE ids(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM ids WHERE i<100000)
        INSERT INTO events(type,instance,data) SELECT 'life', 'nava', '{"action":"created"}' FROM ids;
    """)
    db.executemany("INSERT INTO kv VALUES (?, ?)", [(key, json.dumps(value)) for key, value in [
        ("events_sub:regular", {"caller": "luna"}),
        ("EVENTS-SUB:legacy", {"caller": "luna"}),
        ("events_sub:thread", {"caller": "luna", "delivery_only": 1, "auto_thread_member": 1}),
        ("events_sub:pinned", {"caller": "luna", "delivery_only": 1}),
        ("events_sub:other", {"caller": "nava"}),
    ]])
    db.executemany("INSERT INTO events(type,instance,data) VALUES('life',?,?)", [
        ("luna", '{"action":"stopped"}'), ("luna", '{"action":"stopped"}'),
        ("nava", '{"action":"stopped","placeholder":1}'), ("luna", '{"action":"ready","batch_id":"batch"}'),
    ])
    # SELECT the exact DELETE victims; do not time physical writes or commits.
    subscriptions = "SELECT key FROM kv WHERE key LIKE 'events_sub:%' AND json_extract(value,'$.caller')=?"
    regular = subscriptions + " AND COALESCE(json_extract(value,'$.delivery_only'),0)!=1"
    memberships = subscriptions + " AND json_extract(value,'$.auto_thread_member')=1 AND COALESCE(json_extract(value,'$.delivery_only'),0)=1"
    stopped = "type='life' AND json_extract(data,'$.action')='stopped' AND COALESCE(json_extract(data,'$.placeholder'),0)!=1"
    ready = "type='life' AND json_extract(data,'$.action')='ready'"
    queries = {
        "cleanup_subscriptions": (regular, ("luna",)), "cleanup_memberships": (memberships, ("luna",)),
        "reserved_names": (f"SELECT DISTINCT instance FROM events WHERE {stopped}", ()),
        "batch_ready": (f"SELECT instance FROM events WHERE {ready} AND json_extract(data,'$.batch_id')=?", ("batch",)),
    }
    baseline = {label: measure(db, label + "/base", sql, args) for label, (sql, args) in queries.items()}
    # Isolated candidate indexes: keep the original LIKE pattern and predicates.
    db.executescript(f"""
        CREATE INDEX audit_sub_caller ON kv(json_extract(value,'$.caller')) WHERE key LIKE 'events_sub:%';
        CREATE INDEX audit_stopped_names ON events(instance) WHERE {stopped};
        CREATE INDEX audit_ready_batch ON events(json_extract(data,'$.batch_id')) WHERE {ready};
    """)
    for label, (sql, args) in queries.items():
        if label == "reserved_names":
            sql = sql.replace("FROM events WHERE", "FROM events INDEXED BY audit_stopped_names WHERE")
        assert measure(db, label + "/indexed", sql, args) == baseline[label], label


if __name__ == "__main__":
    main()
