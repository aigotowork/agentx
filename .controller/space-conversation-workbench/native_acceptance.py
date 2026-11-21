#!/usr/bin/env python3
"""Compose a bounded, private AgentX daemon, public SDK and frozen Space build."""
import argparse
from contextlib import closing
from datetime import datetime, timezone
import json
from pathlib import Path
import sqlite3
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tests/live_ui_integration"))
from acceptance import Cases
from live_fixture import LiveFixture


def seed(fixture, reference, first, last):
    """Seed a few historical messages in the disposable journal only."""
    db = (fixture.root / "primary/scope/agentx.db").resolve(strict=True)
    assert db.is_relative_to(fixture.root) and fixture.root.name.startswith("agentx-management-")
    now = datetime.now(timezone.utc).isoformat()
    with closing(sqlite3.connect(db.as_uri() + "?mode=rw", uri=True, timeout=3)) as conn, conn:
        return [
            str(conn.execute(
                "INSERT INTO events(timestamp,type,instance,data) VALUES (?,'message',?,?)",
                (now, reference["name"], json.dumps(dict(
                    text=f"原生样例 {i:02d}：核对会话边界与继续读取。\n\n```text\npage: {i:02d}\n```",
                    delivered_to=["space-reviewer"], **{"from": "@" + reference["name"]}
                ), ensure_ascii=False)),
            ).lastrowid)
            for i in range(first, last + 1)
        ]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("build", type=Path)
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    fixture = LiveFixture(args.candidate, args.evidence)
    try:
        # Stage a fake Provider's Fork semantics privately: distinct session and
        # generated hooks. Lifecycle, web receipt and relation reads remain native.
        provider = fixture.root / "bin/qoder"
        source = provider.read_text()
        marker = 'resumed = "--resume" in arguments'
        assert source.count(marker) == 1
        provider.write_text(source.replace(marker, marker + ' and "--fork-session" not in arguments'))

        cases = Cases(fixture, setup_only=True)
        cases.launch()
        http = cases.http
        descriptor = http.get("/api/v1/management")
        assert descriptor["supports_message_history"] is True
        ids = seed(fixture, cases.reference, 1, 47)
        config = fixture.evidence / "browser-config.json"
        config.write_text(json.dumps(dict(
            url=http.url, scope=descriptor["scope"], generation=http.generation,
            reference=cases.reference, ids=ids, workspace=str(cases.paths["child"]),
            build=str(args.build.resolve(strict=True)),
        )))
        subprocess.run(["node", str(Path(__file__).with_name("native.browser.mjs")), str(config)], cwd=ROOT, check=True, timeout=100)
    finally:
        fixture.close()


if __name__ == "__main__":
    main()
