"""Check the proposed label indexes against a disposable synthetic SQLite fixture.

This measures label-layer queries, not AgentX end-to-end listing or production SLA.
The temporary scope table represents an already-resolved native membership set;
it does not implement native exact-incarnation, workspace or status resolution.
"""

import argparse
import hashlib
import json
import platform
import re
import sqlite3
import statistics
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DDL = Path(__file__).with_name("storage.sql")
NATIVE = ROOT / "src/db/launch_relations.rs"


def owner(number):
    return f"{number:08x}-0000-4000-8000-{number:012x}"


def labels(number):
    result = {
        "project": f"project-{number % 1000:04d}",
        "feature": f"feature-{number // 1000 % 5}",
        "shared": "yes",
        "zone": f"zone-{number % 10}",
        **{f"extra-{i}": f"value-{number % (i + 2)}" for i in range(5)},
    }
    if number % 4:
        result["review/state"] = "approved" if number % 3 == 0 else "pending"
    return result


def measure(db, sql, parameters):
    plan = [row[3] for row in db.execute("EXPLAIN QUERY PLAN " + sql, parameters)]
    result = db.execute(sql, parameters).fetchall()
    samples = []
    for _ in range(5):
        started = time.perf_counter()
        assert db.execute(sql, parameters).fetchall() == result
        samples.append((time.perf_counter() - started) * 1000)
    progress_calls = 0

    def progress():
        nonlocal progress_calls
        progress_calls += 1
        return 0

    db.set_progress_handler(progress, 1000)
    try:
        db.execute(sql, parameters).fetchall()
    finally:
        db.set_progress_handler(None, 0)
    return {
        "sql": sql,
        "parameters": parameters,
        "plan": plan,
        "result_rows": len(result),
        "scalar_result": result[0][0] if len(result) == 1 and len(result[0]) == 1 else None,
        "warm_samples": len(samples),
        "warm_median_ms": round(statistics.median(samples), 3),
        "warm_max_ms": round(max(samples), 3),
        "vm_steps_rounded_down_1000": progress_calls * 1000,
    }


def run(db, population):
    db.executescript("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
    native_sql = re.search(r'const SCHEMA: &str = "(.*?)";', NATIVE.read_text(), re.S).group(1)
    db.executescript(native_sql)
    db.executescript(DDL.read_text())
    db.executescript("""
        CREATE TABLE fixture_scope (
            owner_creation TEXT PRIMARY KEY, workspace TEXT NOT NULL, name TEXT NOT NULL
        ) WITHOUT ROWID;
        CREATE INDEX fixture_scope_by_workspace_name
            ON fixture_scope(workspace, name, owner_creation);
    """)
    started = time.perf_counter()
    for start in range(0, population, 1000):
        chunk = range(start, min(population, start + 1000))
        with db:
            db.executemany("INSERT INTO agent_creations(id,name,incarnation,origin) VALUES (?,?,?,'external')",
                           ((owner(n), f"a{n:07d}", str(n + 1)) for n in chunk))
            db.executemany("INSERT INTO agent_creation_instances(name,incarnation,creation) VALUES (?,?,?)",
                           ((f"a{n:07d}", str(n + 1), owner(n)) for n in chunk))
            db.executemany("INSERT INTO agent_label_sets(owner_creation) VALUES (?)", ((owner(n),) for n in chunk))
            db.executemany("INSERT INTO agent_labels VALUES (?,?,?)",
                           ((owner(n), k, v) for n in chunk for k, v in labels(n).items()))
            db.executemany("INSERT INTO fixture_scope VALUES (?,?,?)",
                           ((owner(n), f"w{n % 10:02d}", f"a{n:07d}") for n in chunk))
    db.execute("ANALYZE")
    db.commit()
    seed_seconds = time.perf_counter() - started

    queries = {
        "one_owner": ("SELECT label_key,label_value FROM agent_labels WHERE owner_creation=? ORDER BY label_key", [owner(7)]),
        "positive_equal": ("SELECT owner_creation FROM agent_labels WHERE label_key=? AND label_value=? ORDER BY owner_creation", ["project", "project-0007"]),
        "positive_exists": ("SELECT owner_creation FROM agent_labels WHERE label_key=? LIMIT 101", ["review/state"]),
        "reverse_alias": ("SELECT name,incarnation FROM agent_creation_instances WHERE creation=?", [owner(7)]),
        "scoped_sparse_and": ("""SELECT s.name,s.owner_creation FROM agent_labels AS seed
            CROSS JOIN fixture_scope AS s ON s.owner_creation=seed.owner_creation
            WHERE seed.label_key=? AND seed.label_value=? AND s.workspace=? AND s.name>?
            AND EXISTS(SELECT 1 FROM agent_labels AS other
                WHERE other.owner_creation=seed.owner_creation AND other.label_key=? AND other.label_value=?)
            ORDER BY s.name LIMIT 101""", ["project", "project-0007", "w07", "", "feature", "feature-2"]),
        "scoped_dense_page": ("""SELECT s.name,s.owner_creation FROM fixture_scope AS s
            WHERE s.workspace=? AND s.name>? AND EXISTS(SELECT 1 FROM agent_labels AS l
                WHERE l.owner_creation=s.owner_creation AND l.label_key=? AND l.label_value=?)
            ORDER BY s.name LIMIT 101""", ["w00", "", "shared", "yes"]),
        "scoped_negative_page": ("""SELECT s.name,s.owner_creation FROM fixture_scope AS s
            WHERE s.workspace=? AND s.name>? AND NOT EXISTS(SELECT 1 FROM agent_labels AS l
                WHERE l.owner_creation=s.owner_creation AND l.label_key=? AND l.label_value=?)
            ORDER BY s.name LIMIT 101""", ["w00", "", "review/state", "approved"]),
        "scoped_negative_count": ("""SELECT COUNT(*) FROM fixture_scope AS s WHERE s.workspace=?
            AND NOT EXISTS(SELECT 1 FROM agent_labels AS l
                WHERE l.owner_creation=s.owner_creation AND l.label_key=? AND l.label_value=?)""",
            ["w00", "review/state", "approved"]),
        "scoped_absence_page": ("""SELECT s.name,s.owner_creation FROM fixture_scope AS s
            WHERE s.workspace=? AND s.name>? AND NOT EXISTS(SELECT 1 FROM agent_labels AS l
                WHERE l.owner_creation=s.owner_creation AND l.label_key=?)
            ORDER BY s.name LIMIT 101""", ["w00", "", "review/state"]),
        "scoped_deep_page": ("""SELECT s.name,s.owner_creation FROM fixture_scope AS s
            WHERE s.workspace=? AND s.name>? AND EXISTS(SELECT 1 FROM agent_labels AS l
                WHERE l.owner_creation=s.owner_creation AND l.label_key=? AND l.label_value=?)
            ORDER BY s.name LIMIT 101""", ["w00", f"a{population - 2000:07d}", "shared", "yes"]),
    }
    page_owners = [owner(n) for n in range(0, 1000, 10)]
    placeholders = ",".join("?" for _ in page_owners)
    queries["hydrate_page"] = (
        f"SELECT owner_creation,label_key,label_value FROM agent_labels WHERE owner_creation IN ({placeholders}) ORDER BY owner_creation,label_key", page_owners)
    evidence = {name: measure(db, *query) for name, query in queries.items()}
    for name in ["positive_equal", "positive_exists", "scoped_sparse_and"]:
        assert any("agent_labels_by_key_value_owner" in p and "SEARCH" in p for p in evidence[name]["plan"]), name
    for name in ["one_owner", "hydrate_page", "scoped_dense_page", "scoped_negative_page", "scoped_absence_page"]:
        assert any("PRIMARY KEY" in p and "SEARCH" in p for p in evidence[name]["plan"]), name
    assert any("agent_creation_instances_by_creation" in p and "SEARCH" in p for p in evidence["reverse_alias"]["plan"])
    assert all(not any("SCAN agent_labels" in p or "SCAN seed" in p or "SCAN l" == p for p in q["plan"]) for q in evidence.values())

    expected = []
    scope = [n for n in range(population) if n % 10 == 0]
    for n in scope:
        if labels(n).get("review/state") != "approved":
            expected.append((f"a{n:07d}", owner(n)))
    assert db.execute(*queries["scoped_negative_page"]).fetchall() == expected[:101]
    assert evidence["scoped_negative_count"]["scalar_result"] == len(expected)
    assert evidence["positive_equal"]["result_rows"] == len(range(7, population, 1000))
    assert evidence["scoped_sparse_and"]["result_rows"] == sum(n // 1000 % 5 == 2 for n in range(7, population, 1000))
    assert evidence["hydrate_page"]["result_rows"] == sum(len(labels(n)) for n in range(0, 1000, 10))

    try:
        with db:
            db.execute("UPDATE agent_label_sets SET revision=1 WHERE owner_creation=?", [owner(7)])
            db.execute("UPDATE agent_labels SET label_value='' WHERE owner_creation=? AND label_key='project'", [owner(7)])
    except sqlite3.IntegrityError:
        pass
    else:
        raise AssertionError("empty value accepted")
    assert db.execute("SELECT revision FROM agent_label_sets WHERE owner_creation=?", [owner(7)]).fetchone()[0] == 0
    assert db.execute("SELECT label_value FROM agent_labels WHERE owner_creation=? AND label_key='project'", [owner(7)]).fetchone()[0] == "project-0007"
    try:
        with db:
            db.execute("INSERT INTO agent_labels VALUES (?,?,?)", [owner(7), "project", "duplicate"])
    except sqlite3.IntegrityError:
        pass
    else:
        raise AssertionError("duplicate key accepted")

    write_samples = []
    for n in range(20):
        started = time.perf_counter()
        with db:
            changed = db.execute("UPDATE agent_label_sets SET revision=revision+1 WHERE owner_creation=? AND revision=?", [owner(7), n]).rowcount
            assert changed == 1
            db.execute("UPDATE agent_labels SET label_value=? WHERE owner_creation=? AND label_key='project'", [f"edit-{n}", owner(7)])
        write_samples.append((time.perf_counter() - started) * 1000)
    assert db.execute("UPDATE agent_label_sets SET revision=revision+1 WHERE owner_creation=? AND revision=0", [owner(7)]).rowcount == 0
    db.rollback()
    assert db.execute("PRAGMA foreign_key_check").fetchall() == []
    return {
        "evidence_boundary": "Disposable synthetic label-layer fixture; native identity/scope resolution and IPC/HTTP/hydration outside labels excluded; warm samples are not SLA",
        "python": platform.python_version(), "sqlite": sqlite3.sqlite_version,
        "ddl_sha256": hashlib.sha256(DDL.read_bytes()).hexdigest(),
        "native_creation_schema_sha256": hashlib.sha256(native_sql.encode()).hexdigest(),
        "fixture": {"agents": population, "workspaces": 10, "labels": db.execute("SELECT COUNT(*) FROM agent_labels").fetchone()[0],
                    "scope_agents": len(scope), "seed_seconds": round(seed_seconds, 3), "journal_mode": "wal", "synchronous": "normal", "statistics": "ANALYZE after fixture seed"},
        "database_allocated_mib": round(db.execute("PRAGMA page_count").fetchone()[0] * db.execute("PRAGMA page_size").fetchone()[0] / 1048576, 2),
        "queries": evidence,
        "write_20_transactions_ms": {"median": round(statistics.median(write_samples), 3), "max": round(max(write_samples), 3)},
        "correctness": ["missing-key inequality includes absent values", "multi-label intersection", "page hydration cardinality", "unique key", "empty-value rejection with revision rollback", "stale revision CAS refused", "foreign keys clean"],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agents", type=int, default=100000)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    if arguments.agents < 5000:
        parser.error("the fixed cardinality cases require at least 5000 agents")
    with tempfile.TemporaryDirectory(prefix="agentx-labels-design-") as folder:
        connection = sqlite3.connect(Path(folder) / "fixture.sqlite")
        try:
            report = run(connection, arguments.agents)
        finally:
            connection.close()
    arguments.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"fixture": report["fixture"], "query_median_ms": {k: v["warm_median_ms"] for k, v in report["queries"].items()}, "output": str(arguments.output)}, ensure_ascii=False))
