#!/usr/bin/env python3
"""Verify and adopt the native macOS package on scopes served by the PATH binary."""

import argparse
from contextlib import closing
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import sqlite3
import stat
import subprocess
import tempfile
import time
import urllib.request


HOME = Path.home()
INSTALLED = HOME / ".local/bin/agentx"
INDEXES = {"agentx_stopped_name_v1", "agentx_sub_owner_v1"}
ENV = {k: v for k, v in os.environb.items() if not k.startswith((b"AGENTX_", b"CODEX_"))}
PORTABLE_ENV = {b"PATH": b"/usr/bin:/bin:/usr/sbin:/sbin"}


def run(binary, *args, env=ENV, timeout=35):
    return subprocess.run([binary, *args], cwd=HOME, env=env, capture_output=True, text=True, timeout=timeout)


def read_status(binary, scope, env=ENV, starting=False):
    result = run(binary, "daemon", "status", "--scope", scope, env=env, timeout=8)
    parsed = {"state": re.search(r"^Daemon: (\S+)\s*$", result.stdout, re.M),
              "client": re.search(r"^Client: version=(\S+) cohort=(\S+)$", result.stdout, re.M),
              "daemon": re.search(r"^Daemon: version=(\S+) cohort=(\S+) instance=(\S+) generation=(\S+)$", result.stdout, re.M),
              "peer": re.search(r"^Peer: pid=(\d+) incarnation=(\S+)$", result.stdout, re.M),
              "endpoint": re.search(r"^Management: (\S+)$", result.stdout, re.M)}
    assert (result.returncode == 0 or (starting and result.returncode == 1)) and parsed["state"] and parsed["client"], (scope, result.stdout, result.stderr)
    return {**{k: v.groups() if v else None for k, v in parsed.items()}, "stdout": result.stdout}


def digest(path):
    hash_sum = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hash_sum.update(chunk)
    return hash_sum.hexdigest()


def database(path):
    return sqlite3.connect(path.as_uri() + "?mode=ro", uri=True, timeout=6)


def storage(scope):
    with closing(database(scope / "agentx.db")) as db:
        return {"schema": db.execute("PRAGMA user_version").fetchone()[0],
                "indexes": sorted(row[0] for row in db.execute("SELECT name FROM sqlite_schema WHERE type='index'") if row[0] in INDEXES),
                "fork_provenance": bool(db.execute("SELECT 1 FROM sqlite_schema WHERE type='table' AND name='agent_creation_forks'").fetchone())}


def processes():
    output = subprocess.check_output(["/bin/ps", "-axww", "-o", "uid=,pid=,command="], text=True)
    return {int(pid): command for line in output.splitlines() for uid, pid, command in [line.split(None, 2)] if int(uid) == os.getuid()}


def scopes():
    prefix = f"{INSTALLED} daemon serve --scope "
    return {Path(cmd[len(prefix):]).resolve(): pid for pid, cmd in processes().items() if cmd.startswith(prefix)}


def hosts(scope):
    with closing(database(scope / "agentx.db")) as db:
        pids = {pid for (pid,) in db.execute("SELECT DISTINCT pid FROM instances WHERE pid>0 AND COALESCE(origin_device_id,'')='' ")}
    # Preserve user-owned PTY supervisors as well as the recorded Provider PIDs.
    pids.update(pid for pid, cmd in processes().items() if re.match(r"\S*/agentx\S* pty(?: |$)", cmd))
    return process_identities(pids)


def process_identities(pids):
    if not pids:
        return {}
    output = run("/bin/ps", "-p", ",".join(map(str, sorted(pids))), "-o", "pid=,stat=,lstart=").stdout
    rows = [line.split(None, 2) for line in output.splitlines()]
    return {int(pid): started for pid, state, started in rows if not state.startswith("Z")}


def peer_environment(pid, scope):
    # Darwin KERN_PROCARGS2 separates argv and environment with NULs. Unlike
    # parsing ps output, this preserves whitespace and avoids logging values.
    lib = ctypes.CDLL(None, use_errno=True)
    mib = (ctypes.c_int * 3)(1, 49, pid)
    length = ctypes.c_size_t(0)
    if lib.sysctl(mib, 3, None, ctypes.byref(length), None, 0) != 0:
        raise OSError(ctypes.get_errno(), "cannot read daemon environment size")
    raw = ctypes.create_string_buffer(length.value)
    if lib.sysctl(mib, 3, raw, ctypes.byref(length), None, 0) != 0:
        raise OSError(ctypes.get_errno(), "cannot read daemon environment")
    data = raw.raw[:length.value]
    argc = int.from_bytes(data[:4], "little", signed=True)
    assert argc >= 3, "unexpected daemon argc"
    argv_start = data.index(b"\0", 4)
    fields = data[argv_start:].lstrip(b"\0").split(b"\0")
    assert fields[1:4] == [b"daemon", b"serve", b"--scope"], "PID no longer belongs to daemon serve"
    env = dict(item.split(b"=", 1) for item in fields[argc:] if b"=" in item)
    assert env.get(b"HOME") == os.fsencode(HOME) and env.get(b"PATH"), "daemon environment is incomplete"
    return env | {b"AGENTX_DIR": os.fsencode(scope)}


def checked(*args, **kwargs):
    result = run(*args, **kwargs)
    assert result.returncode == 0, (args, result.stderr)
    return result.stdout.strip()


def stop_fixture(child):
    if child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=8)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=5)


def verify(candidate):
    assert platform.machine() == "arm64" and platform.system() == "Darwin"
    assert checked("/usr/bin/lipo", candidate, "-archs") == "arm64"
    checked("/usr/bin/codesign", "--verify", candidate)
    version = checked(candidate, "--version")
    result = {"sha256": digest(candidate), "version": version, "dependencies": checked("/usr/bin/otool", "-L", candidate), "rehearsal": []}
    with tempfile.TemporaryDirectory(prefix="axv-", dir="/tmp") as directory:
        root = Path(directory).resolve()
        scope = root / "s"
        scope.mkdir(mode=0o700)
        env = PORTABLE_ENV | {b"HOME": os.fsencode(root), b"AGENTX_DIR": os.fsencode(scope)}
        generation = None
        for stage, binary, schema in (("old", INSTALLED, 28), ("candidate", candidate, 29), ("rollback", INSTALLED, 29)):
            with (root / "smoke.log").open("w+") as log:
                child = subprocess.Popen([binary, "daemon", "serve", "--scope", scope], env=env, cwd=root,
                                         stdin=subprocess.DEVNULL, stdout=log, stderr=log)
                try:
                    deadline = time.monotonic() + 15
                    while True:
                        probe = read_status(binary, scope, env, starting=True)
                        if probe["state"] == ("running",):
                            break
                        if child.poll() is not None or time.monotonic() >= deadline:
                            log.seek(0)
                            raise RuntimeError((stage, probe["stdout"], log.read()))
                        time.sleep(0.05)
                    generation = generation or probe["daemon"][3]
                    assert generation == probe["daemon"][3], stage
                    assert int(probe["peer"][0]) == child.pid, stage
                    expected = {"schema": schema, "indexes": sorted(INDEXES) if schema == 29 else [], "fork_provenance": schema == 29}
                    assert storage(scope) == expected, stage
                    checked(binary, "list", "--all", "--names", env=env)
                    result["rehearsal"].append({"stage": stage, "cohort": probe["daemon"][1], "generation_preserved": True, **expected})
                finally:
                    stop_fixture(child)
        assert not (scope / "archive").exists()
    return result


def health(endpoint):
    if not endpoint:
        return None
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    values = {}
    for path in ("/", "/api/v1/management", "/space/", "/inspector/"):
        with opener.open(endpoint[0].rstrip("/") + path, timeout=10) as result:
            assert result.status == 200
            values[path] = result.status
    return values


def backup_db(scope, path):
    with closing(database(scope / "agentx.db")) as source, closing(sqlite3.connect(path)) as dest:
        os.chmod(path, 0o600)
        start = time.monotonic()

        def progress(_code, _remaining, _total):
            if time.monotonic() - start > 30:
                raise TimeoutError(f"SQLite backup deadline: {scope}")

        source.backup(dest, pages=1024, progress=progress)
        assert dest.execute("PRAGMA quick_check(1)").fetchone()[0] == "ok", path
        version = dest.execute("PRAGMA user_version").fetchone()[0]
        generation = dest.execute("SELECT value FROM kv WHERE key='service:database_generation'").fetchone()[0]
        return {"path": str(path), "schema": version, "generation": generation, "quick_check": "ok"}


def write(path, value):
    stage = path.with_suffix(".tmp")
    stage.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    stage.replace(path)


def adopt(candidate, evidence, verification):
    assert INSTALLED.is_file() and not INSTALLED.is_symlink()
    old_digest = digest(INSTALLED)
    assert old_digest == "c46a8cd303ac428960bc8a3d44ceb7272389ebeccabf736ae9a1d181d6d35e29", "remote baseline bytes changed"
    candidate_digest = verification["sha256"]
    assert digest(candidate) == candidate_digest and candidate_digest != old_digest
    old_cohort = verification["rehearsal"][0]["cohort"]
    new_cohort = verification["rehearsal"][1]["cohort"]
    targets = scopes()
    assert targets, "no corresponding live daemon found"
    rollback = HOME / ".local/state/agentx/launch-hot-path-20261122"
    rollback.mkdir(parents=True, mode=0o700)
    backup = INSTALLED.with_name("agentx.before-launch-hot-path-20261122")
    assert not backup.exists(), backup
    evidence_path = evidence / "rollout.json"
    report = {"installed": str(INSTALLED), "sha256": candidate_digest, "backup": str(backup),
              "backup_sha256": old_digest, "scopes": []}
    retained_env = {}
    for number, (scope, pid) in enumerate(targets.items()):
        before = read_status(candidate, scope)
        assert before["peer"][0] == str(pid) and before["daemon"][1] == old_cohort, scope
        before_db = storage(scope)
        saved = backup_db(scope, rollback / f"scope-{number}-before.db")
        assert saved["schema"] == before_db["schema"] == 28 and saved["generation"] == before["daemon"][3]
        retained_env[scope] = peer_environment(pid, scope)
        config = scope / "config.toml"
        report["scopes"].append({"scope": str(scope), "before": before, "before_db": before_db, "data_backup": saved,
                                 "health_before": health(before["endpoint"]), "config_sha256": digest(config) if config.is_file() else None})
    assert scopes() == targets and digest(INSTALLED) == old_digest, "rollout targets changed"
    os.link(INSTALLED, backup)
    write(evidence_path, report)
    with tempfile.NamedTemporaryFile(dir=INSTALLED.parent, prefix=".agentx-stage-", delete=False) as handle:
        stage = Path(handle.name)
    try:
        shutil.copyfile(candidate, stage)
        stage.chmod(stat.S_IMODE(INSTALLED.stat().st_mode))
        checked("/usr/bin/codesign", "--verify", stage)
        assert digest(stage) == candidate_digest and digest(backup) == old_digest
        stage.replace(INSTALLED)
    finally:
        stage.unlink(missing_ok=True)
    assert checked(INSTALLED, "--version") == verification["version"] and digest(INSTALLED) == candidate_digest
    for entry in report["scopes"]:
        scope = Path(entry["scope"])
        before = entry["before"]
        assert read_status(candidate, scope) == before, "selected daemon changed before restart"
        original_hosts = hosts(scope)
        start = time.monotonic()
        try:
            restarted = run(INSTALLED, "daemon", "restart", "--scope", scope, env=retained_env[scope], timeout=75)
            entry["restart"] = {"exit": restarted.returncode, "stdout": restarted.stdout, "stderr": restarted.stderr}
        except subprocess.TimeoutExpired:
            entry["restart"] = {"exit": None, "timeout": True}
        entry["restart"]["seconds"] = round(time.monotonic() - start, 3)
        write(evidence_path, report)
        after = entry["after"] = read_status(INSTALLED, scope)
        write(evidence_path, report)
        assert after["daemon"][1] == new_cohort and after["daemon"][2] != before["daemon"][2]
        assert after["daemon"][3] == before["daemon"][3] and after["peer"] != before["peer"]
        assert after["endpoint"] == before["endpoint"] and scopes()[scope] == int(after["peer"][0])
        entry["after_db"] = storage(scope)
        assert entry["after_db"] == {"schema": 29, "indexes": sorted(INDEXES), "fork_provenance": True}
        config = scope / "config.toml"
        assert (digest(config) if config.is_file() else None) == entry["config_sha256"]
        found = process_identities(original_hosts)
        entry["hosts_before"] = original_hosts
        entry["missing_hosts"] = {pid: start for pid, start in original_hosts.items() if found.get(pid) != start}
        entry["health_after"] = health(after["endpoint"])
        names = checked(INSTALLED, "list", "--all", "--names", env=ENV | {b"AGENTX_DIR": os.fsencode(scope)})
        entry["names_count"] = len(names.splitlines())
        write(evidence_path, report)
        assert not entry["missing_hosts"], entry["missing_hosts"]
        print(f"{scope}: PID {before['peer'][0]} → {after['peer'][0]}; {len(original_hosts)} process identities retained; both indexes present", flush=True)
    print(f"Installed {INSTALLED}\nSHA-256 {candidate_digest}\nBackup {backup}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--adopt", action="store_true")
    args = parser.parse_args()
    candidate = args.candidate.resolve()
    evidence = candidate.parent
    os.umask(0o077)
    if args.adopt:
        verification = json.loads((evidence / "verify.json").read_text())
        adopt(candidate, evidence, verification)
    else:
        verification = verify(candidate)
        write(evidence / "verify.json", verification)
        print(json.dumps(verification, indent=2))


if __name__ == "__main__":
    main()
