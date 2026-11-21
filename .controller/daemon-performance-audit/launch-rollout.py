#!/usr/bin/env python3
"""Install the verified fresh-launch artifact and switch only project/home on Linux."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import subprocess
import tempfile
import time
import urllib.request


PROJECT = Path(__file__).resolve().parents[2]
HOME = Path.home()
ART = PROJECT / ".artifacts/launch-performance-audit"
CANDIDATE = ART / "agentx-candidate"
EXPECTED = "56d8caa4a9b88f933f4434f97b9462197fcafa5b36bcc919c8b750b9088f2694"
COHORT = "native-build-v1:5abd237a-a857-456a-8457-b2dfec6a08cc"
TARGETS = {
    "project": (PROJECT / ".agentx", PROJECT / ".bin/agentx", 38023),
    "home": (HOME / ".agentx", HOME / ".local/bin/agentx", 38000),
}
INDEXES = {"agentx_stopped_name_v1", "agentx_sub_owner_v1"}
ROUTES = ("/", "/api/v1/management", "/space/", "/inspector/")
ENV = {k: v for k, v in os.environb.items() if not k.startswith((b"AGENTX_", b"CODEX_"))}


def digest(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def run(binary, *args, env=ENV, timeout=35):
    return subprocess.run([binary, *args], cwd=HOME, env=env, capture_output=True, text=True, timeout=timeout)


def status(binary, scope):
    result = run(binary, "daemon", "status", "--scope", scope, timeout=8)
    assert result.returncode == 0 and "Daemon: running\n" in result.stdout, (scope, result.stdout, result.stderr)
    identity = re.search(r"Daemon: version=(\S+) cohort=(\S+) instance=(\S+) generation=(\S+)", result.stdout)
    peer = re.search(r"Peer: pid=(\d+) incarnation=(\S+)", result.stdout)
    endpoint = re.search(r"^Management: (\S+)", result.stdout, re.M)
    return dict(zip(("version", "cohort", "instance", "generation"), identity.groups())) | {
        "pid": int(peer[1]), "incarnation": peer[2], "endpoint": endpoint[1] if endpoint else None,
    }


def read_db(scope):
    return sqlite3.connect((scope / "agentx.db").as_uri() + "?mode=ro", uri=True, timeout=5)


def storage(scope):
    with read_db(scope) as db:
        return {"version": db.execute("PRAGMA user_version").fetchone()[0],
                "indexes": sorted(index for (index,) in db.execute("SELECT name FROM sqlite_schema WHERE type='index'") if index in INDEXES)}


def snapshot(scope):
    with read_db(scope) as db:
        participant_pids = {pid for (pid,) in db.execute("SELECT DISTINCT pid FROM instances WHERE pid>0 AND COALESCE(origin_device_id,'')=''")}
    hosts = {}
    marker = b"AGENTX_DIR=" + os.fsencode(scope)
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        pid = int(entry.name)
        try:
            if pid not in participant_pids:
                if (entry / "cmdline").read_bytes().split(b"\0", 2)[1:2] != [b"pty"]:
                    continue
                if marker not in (entry / "environ").read_bytes().split(b"\0"):
                    continue
            stat = (entry / "stat").read_text().rsplit(")", 1)[1].split()
            if stat[0] != "Z":
                hosts[pid] = stat[19]
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            continue
    return hosts


def http(endpoint):
    direct = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    values = {}
    for route in ROUTES:
        with direct.open(endpoint.rstrip("/") + route, timeout=15) as response:
            values[route] = response.status
            if route.startswith("/api/"):
                assert isinstance(json.load(response), dict), route
    return values


def receipt(name, value):
    path = ART / f"rollout-{name}.json"
    stage = path.with_suffix(".tmp")
    stage.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    stage.replace(path)


def replace_binary(source, dest, expected):
    with tempfile.NamedTemporaryFile(prefix=".agentx-stage-", dir=dest.parent, delete=False) as handle:
        stage = Path(handle.name)
    try:
        shutil.copyfile(source, stage)
        stage.chmod(0o755)
        assert digest(stage) == expected, stage
        os.replace(stage, dest)
    finally:
        stage.unlink(missing_ok=True)


def restart(binary, scope, env):
    start = time.monotonic()
    try:
        result = run(binary, "daemon", "restart", "--scope", scope, env=env, timeout=80)
        return {"exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
                "seconds": round(time.monotonic() - start, 3)}
    except subprocess.TimeoutExpired:
        return {"exit": None, "seconds": round(time.monotonic() - start, 3)}


def adopt(name):
    scope, binary, port = TARGETS[name]
    assert digest(CANDIDATE) == EXPECTED
    assert binary.is_file() and not binary.is_symlink()
    before = status(CANDIDATE, scope)
    previous = digest(binary)
    assert previous != EXPECTED and digest(f"/proc/{before['pid']}/exe") == previous, "installed and loaded versions differ"
    assert before["endpoint"] == f"http://127.0.0.1:{port}/", before
    before_db = storage(scope)
    config = scope / "config.toml"
    config_sha = digest(config) if config.is_file() else None
    db_backup = ART / "rollback-data" / f"{name}-before.db"
    with sqlite3.connect(db_backup.resolve().as_uri() + "?mode=ro", uri=True) as db:
        assert db.execute("PRAGMA user_version").fetchone()[0] == before_db["version"]
        assert before["generation"] in db.execute("SELECT value FROM kv WHERE key='service:database_generation'").fetchone()[0]
    backup = binary.with_name("agentx.before-launch-hot-path-20261122")
    assert not backup.exists(), backup
    before_http = http(before["endpoint"])
    env = dict(row.split(b"=", 1) for row in Path(f"/proc/{before['pid']}/environ").read_bytes().split(b"\0") if b"=" in row)
    env[b"AGENTX_DIR"] = os.fsencode(scope)
    live_before = snapshot(scope)
    assert status(CANDIDATE, scope) == before and digest(binary) == previous, "live owner changed during preparation"
    os.link(binary, backup)
    assert digest(backup) == previous
    report = {"scope": str(scope), "before": before, "before_db": before_db, "before_http": before_http,
              "config_sha256": config_sha, "binary": str(binary), "backup": str(backup), "backup_sha256": previous,
              "data_snapshot": str(db_backup), "before_hosts": live_before, "installed_sha256": EXPECTED}
    receipt(name, report)
    replace_binary(CANDIDATE, binary, EXPECTED)
    report["restart"] = restart(binary, scope, env)
    receipt(name, report)
    # An uncertain CLI result must be reconciled with the exact live owner;
    # never retry a restart just because the reply was lost.
    after = status(CANDIDATE, scope)
    report.update(after=after, after_db=storage(scope))
    receipt(name, report)
    assert after["cohort"] == COHORT and digest(f"/proc/{after['pid']}/exe") == EXPECTED, after
    assert after["instance"] != before["instance"] and after["pid"] != before["pid"], after
    assert after["generation"] == before["generation"] and after["endpoint"] == before["endpoint"], after
    assert report["after_db"] == {"version": 29, "indexes": sorted(INDEXES)}, report["after_db"]
    assert (digest(config) if config.is_file() else None) == config_sha
    live_after = snapshot(scope)
    names = run(binary, "list", "--all", "--names", env=ENV | {b"AGENTX_DIR": os.fsencode(scope)})
    after_http = http(after["endpoint"])
    missing = {pid: start for pid, start in live_before.items() if live_after.get(pid) != start}
    report.update(after_hosts=live_after, missing_hosts=missing, names_exit=names.returncode,
                  names_count=len(names.stdout.splitlines()), http=after_http)
    receipt(name, report)
    assert not missing and names.returncode == 0 and all(code == 200 for code in after_http.values()), (missing, names.stderr, after_http)
    print(f"{name}: {before['pid']} → {after['pid']}; kept {len(live_before)} process identities; HTTP OK; both indexes present", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=TARGETS)
    adopt(parser.parse_args().target)


if __name__ == "__main__":
    main()
