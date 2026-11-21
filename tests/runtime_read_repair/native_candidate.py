#!/usr/bin/env python3
"""Run the GET-only browser driver against an offline-sanitized retained database copy."""
import argparse
import hashlib
import json
from pathlib import Path
import select
import sqlite3
import subprocess
import tempfile
import time
import urllib.request


def await_ready(process, scope):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("Owned candidate daemon exited before readiness")
        try:
            endpoint = json.loads((scope / ".daemon/management.json").read_text())
            with urllib.request.urlopen(endpoint["url"] + "api/v1/management", timeout=3) as response:
                descriptor = json.load(response)
            if descriptor["scope"] == str(scope):
                return endpoint, descriptor
        except (OSError, ValueError):
            select.select([], [], [], 0.1)
    raise TimeoutError("Owned candidate readiness exceeded30s")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("backup", type=Path)
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    candidate = args.candidate.resolve(strict=True)
    backup = args.backup.resolve(strict=True)
    evidence = args.evidence.resolve()
    evidence.mkdir(mode=0o700, parents=True, exist_ok=False)
    root = Path(tempfile.mkdtemp(prefix="agentx-runtime-read-independent-"))
    scope, project = root / "scope", root / "project"
    scope.mkdir(mode=0o700)
    project.mkdir()
    database = scope / "agentx.db"
    source_stat = backup.stat()
    with sqlite3.connect(backup.as_uri() + "?mode=ro", uri=True) as source, sqlite3.connect(database) as destination:
        source.backup(destination)
    database.chmod(0o600)
    receipt = {"candidate": str(candidate), "backup": str(backup), "backup_size": source_stat.st_size,
               "backup_mtime_ns": source_stat.st_mtime_ns, "root": str(root),
               "boundary": "Offline copy only; no source DB writes, Provider launch, user process contact or service replacement."}
    with candidate.open("rb") as stream:
        receipt["candidate_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
    with sqlite3.connect(database) as connection:
        connection.execute("PRAGMA foreign_keys=ON")
        receipt["copied_counts"] = {name: connection.execute("SELECT COUNT(*) FROM " + name).fetchone()[0] for name in ["events", "instances"]}
        receipt["copied_counts"]["stopped"] = connection.execute("SELECT COUNT(*) FROM events WHERE type='life' AND json_extract(data,'$.action')='stopped'").fetchone()[0]
        present = {row[0] for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table'")}
        removed = {}
        for table in ["claude_actor_capabilities", "process_bindings", "session_bindings", "notify_endpoints", "participant_attachments", "inbox_claims", "participant_sessions"]:
            if table in present:
                removed[table] = connection.execute("DELETE FROM " + table).rowcount
        detached = connection.execute("UPDATE instances SET pid=NULL,status='inactive',status_context='fixture:detached',session_id=NULL,parent_session_id=NULL,tcp_mode=0,background=0,running_tasks=''").rowcount
        removed["inspection_kv"] = connection.execute("DELETE FROM kv WHERE key LIKE 'inspection:%'").rowcount
        receipt["sanitation"] = {"deleted_runtime_rows": removed, "detached_current_instances": detached,
                                 "inspection": "All Inspector settings/jobs/attempts removed in copy; fresh default collection/recovery disabled.",
                                 "events": "Unchanged; complete retained event/history workload preserved."}
    config = '[management]\nenabled = true\nhost = "127.0.0.1"\nport = 0\nproject = ' + json.dumps(str(project)) + '\n'
    (scope / "config.toml").write_text(config)
    # The read-only candidate has no ambient identity, credential or Provider environment.
    environment = {"AGENTX_DIR": str(scope), "NO_COLOR": "1", "CI": "1", "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8"}
    for field in ["CONFIG", "CACHE", "DATA", "STATE"]:
        environment[f"XDG_{field}_HOME"] = str(root / field.lower())
    process = None
    try:
        with (evidence / "daemon.stdout").open("w") as stdout, (evidence / "daemon.stderr").open("w") as stderr:
            process = subprocess.Popen([str(candidate), "daemon", "serve", "--scope", str(scope)], cwd=project,
                                       env=environment, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
        receipt["owned_pid"] = process.pid
        endpoint, descriptor = await_ready(process, scope)
        receipt["endpoint"], receipt["descriptor"] = endpoint, descriptor
        browser_config = {"disposable": True, "url": endpoint["url"].rstrip("/"), "generation": descriptor["generation"],
                          "scope": str(scope), "evidence": str(evidence / "browser")}
        (evidence / "browser-config.json").write_text(json.dumps(browser_config, indent=2) + "\n")
        result = subprocess.run(["node", str(Path(__file__).with_name("candidate.browser.mjs")), str(evidence / "browser-config.json")], timeout=90)
        receipt["browser_returncode"] = result.returncode
        receipt["status"] = "passed" if result.returncode == 0 else "failed"
        if result.returncode:
            raise RuntimeError("Independent candidate browser driver failed; retained browser/result.json owns details")
    except BaseException as error:
        receipt["status"], receipt["error"] = "failed", str(error)
        raise
    finally:
        if process is not None:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
            receipt["cleanup"] = {"owned_pid": process.pid, "exit_code": process.returncode, "stopped": True}
        receipt["backup_metadata_unchanged"] = backup.stat().st_size == source_stat.st_size and backup.stat().st_mtime_ns == source_stat.st_mtime_ns
        (evidence / "result.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    main()
