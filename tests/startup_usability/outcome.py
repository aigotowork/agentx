#!/usr/bin/env python3
"""Run SU1–7 against fixed images and newly allocated private Linux scopes.

Exact attempts and owned process identities are durable even on failure. This
does not build, install, promote, run a Provider or touch a user data scope.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import fcntl
import hashlib
import http.client
import json
import os
from pathlib import Path
import re
import select
import shutil
import signal
import socket
import struct
import subprocess
import tempfile
import sys
import time
from urllib.parse import urlsplit

from fixture import Fixture, process_record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--old", required=True, type=Path)
    parser.add_argument("--label", required=True)
    parser.add_argument("--only-web", action="store_true", help="Reuse prior SU1–5 receipts; execute independent SU6–7 only")
    parser.add_argument("--reuse-http-from", type=Path, help="Reuse successful same-image SU6 HTTP checks; rebind only the new browser host")
    args = parser.parse_args()
    evidence = args.evidence / args.label
    evidence.mkdir()
    root = Path(tempfile.mkdtemp(prefix="agentx-startup-fixture-"))
    fixture = Fixture(root, evidence)
    candidate, old = args.candidate.resolve(strict=True), args.old.resolve(strict=True)
    resources = {"root": str(root), "artifacts": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in [candidate, old]},
                 "processes": [], "hosts": [], "ports": [], "cases": [], "cleanup": []}
    records = {}
    hosts = []
    def save():
        (evidence / "resources.json").write_text(json.dumps(resources, indent=2) + "\n")
    def run(label, artifact, scope, arguments, expected=0, timeout=45):
        result = fixture.run(label, artifact, scope, arguments, timeout)
        assert result.returncode == expected, (label, result.returncode, result.stderr)
        return result
    def peer(scope, artifact):
        record = fixture.peer(scope, artifact)
        records[str(scope)] = record
        resources["processes"].append(record); save()
        return record
    def same(record):
        current = process_record(record["pid"])
        assert all(current[k] == record[k] for k in current), (record, current)
    def case(name, facts):
        resources["cases"].append({"case": name, **facts}); save()
    http_attempts = []
    def request(origin, path, body=None):
        endpoint = urlsplit(origin)
        connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=10)
        try:
            headers = {"Origin": origin, "Content-Type": "application/json"} if body is not None else {}
            connection.request("POST" if body is not None else "GET", path, body=json.dumps(body) if body is not None else None, headers=headers)
            response = connection.getresponse(); data = response.read()
            http_attempts.append({"origin": origin, "path": path, "body": body, "status": response.status, "response": data.decode(errors="replace")})
            (evidence / "http-attempts.json").write_text(json.dumps(http_attempts, indent=2) + "\n")
            return response.status, data
        finally:
            connection.close()
    def serve(label, scope, options):
        output, error = evidence / f"{label}.stdout", evidence / f"{label}.stderr"
        command = [str(candidate), "workbench", "--port", "0", *options]
        with output.open("x") as out, error.open("x") as err:
            process = subprocess.Popen(command, cwd=root / "project", env={**fixture.env, "AGENTX_DIR": str(scope)},
                                       stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
        record = {**process_record(process.pid), "command": command, "scope_env": str(scope)}
        hosts.append((process, record)); resources["hosts"].append(record); save()
        deadline = time.monotonic() + 30
        while True:
            text = output.read_text().strip()
            if text.startswith("http://"):
                origin = text.splitlines()[0].rstrip("/"); resources["ports"].append(urlsplit(origin).port); save()
                return origin
            assert process.poll() is None, (label, process.returncode, error.read_text())
            if time.monotonic() > deadline:
                raise TimeoutError((label, error.read_text()))
            time.sleep(.05)
    save()
    def lifecycle_cases():
        missing = root / "missing/scope"
        status = run("SU1-missing-env", candidate, missing, ["daemon", "status"])
        assert "Daemon: stopped" in status.stdout and str(missing) in status.stdout
        assert not missing.parent.exists()
        explicit = root / "explicit-missing"
        status = run("SU1-explicit-wins", candidate, missing, ["daemon", "status", "--scope", str(explicit)])
        assert str(explicit) in status.stdout and not explicit.exists() and not missing.exists()
        case("SU1", {"missing_scope_created": False, "identity": "no participant or inherited identity"})

        cold = root / "cold"
        assert json.loads(run("SU2-cold-list", candidate, cold, ["list", "--json"]).stdout) == []
        cold_peer = peer(cold, candidate)
        status = run("SU2-status", candidate, cold, ["daemon", "status"])
        assert "Daemon: running" in status.stdout and f"pid={cold_peer['pid']}" in status.stdout
        case("SU2", {"peer": cold_peer, "ordinary_acquisition": True})

        legacy = root / "legacy"
        run("SU3-old-start", old, legacy, ["start", "--name", "sukeep"])
        before = json.loads(run("SU3-old-list", old, legacy, ["list", "--json"]).stdout)
        retained = next(row for row in before if row["name"] == "sukeep")
        old_peer = peer(legacy, old)
        lock = legacy / ".daemon/owner.lock"; lock_before = (lock.stat().st_dev, lock.stat().st_ino)
        refusal = run("SU3-ordinary-mismatch", candidate, legacy, ["list", "--json"], expected=1)
        assert "incompatible" in refusal.stderr and "daemon restart --scope" in refusal.stderr and str(legacy) in refusal.stderr
        diagnosis = run("SU3-mismatch-status", candidate, legacy, ["daemon", "status"])
        assert "running-incompatible" in diagnosis.stdout and "daemon version/cohort unavailable" in diagnosis.stdout
        same(old_peer)
        case("SU3", {"old_peer": old_peer, "retained_record": retained, "diagnostic": diagnosis.stdout})

        recovery = run("SU4-selected-restart", candidate, cold, ["daemon", "restart", "--scope", str(legacy)])
        new_peer = peer(legacy, candidate)
        assert old_peer["pid"] != new_peer["pid"]
        assert not Path(f"/proc/{old_peer['pid']}/exe").exists()
        assert lock_before == (lock.stat().st_dev, lock.stat().st_ino)
        same(cold_peer)
        after = json.loads(run("SU4-retained-list", candidate, legacy, ["list", "--json"]).stdout)
        after_record = next(row for row in after if row["name"] == "sukeep")
        for key in ["name", "agent_id", "created_at", "tool", "directory"]:
            assert retained[key] == after_record[key], (key, retained, after_record)
        case("SU4", {"new_peer": new_peer, "retained_record": after_record, "lock_inode_preserved": lock_before,
                     "unselected_scope_same_peer": cold_peer, "restart_output": recovery.stdout})

        # A second actual serve cannot acquire the live singleton.
        denied = fixture.run("SU5-second-serve", candidate, legacy, ["daemon", "serve", "--scope", str(legacy)])
        assert denied.returncode != 0 and "another daemon" in denied.stderr
        same(new_peer)
        alias = root / "legacy-alias"; alias.symlink_to(legacy, target_is_directory=True)
        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(pool.map(lambda index: run(f"SU5-serialized-restart-{index}", candidate, legacy if index == 0 else alias,
                                                      ["daemon", "restart"]), range(2)))
        final_peer = peer(legacy, candidate)
        assert lock_before == (lock.stat().st_dev, lock.stat().st_ino)
        held = root / "held"; (held / ".daemon").mkdir(parents=True, mode=0o700); held.chmod(0o700)
        held_lock = held / ".daemon/owner.lock"
        with held_lock.open("w") as owner:
            fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held_inode = held_lock.stat().st_ino
            status = run("SU5-held-status", candidate, held, ["daemon", "status"], expected=1)
            assert "unavailable" in status.stdout
            refusal = run("SU5-held-refusal", candidate, held, ["daemon", "restart"], expected=1)
            assert "refused" in refusal.stderr and "no replacement" in refusal.stderr
            assert held_lock.stat().st_ino == held_inode and not (held / "agentx.db").exists()
            assert not (held / ".daemon/stderr.log").exists()
        impostor_scope = root / "impostor"
        marker = evidence / "impostor-signalled.json"
        command = [sys.executable, str(Path(__file__).with_name("impostor.py")), str(impostor_scope), str(marker)]
        with (evidence / "impostor.stdout").open("x") as out, (evidence / "impostor.stderr").open("x") as err:
            impostor = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
        impostor_record = process_record(impostor.pid)
        resources["hosts"].append({**impostor_record, "command": command}); save()
        try:
            deadline = time.monotonic() + 5
            while not (evidence / "impostor.stdout").read_text().startswith("ready"):
                assert impostor.poll() is None
                if time.monotonic() > deadline:
                    raise TimeoutError("impostor fixture readiness")
                time.sleep(.02)
            refusal = run("SU5-impostor-refusal", candidate, impostor_scope, ["daemon", "restart"], expected=1)
            assert "peer argv is not exact" in refusal.stderr and "no signal/replacement" in refusal.stderr
            assert impostor.poll() is None and not marker.exists()
            assert not (impostor_scope / "agentx.db").exists()
            assert not (impostor_scope / ".daemon/stderr.log").exists()
        finally:
            if impostor.poll() is None:
                resources["cleanup"].append(Fixture.terminate(impostor_record)); impostor.wait(timeout=5); save()
        # Actual owner with failed Hello cannot be signalled/replaced on guess.
        pin = os.pidfd_open(final_peer["pid"])
        try:
            same(final_peer); signal.pidfd_send_signal(pin, signal.SIGSTOP)
            refusal = run("SU5-handshake-refusal", candidate, legacy, ["daemon", "restart"], expected=1, timeout=8)
            assert "handshake failed" in refusal.stderr and "no replacement" in refusal.stderr
            same(final_peer)
        finally:
            signal.pidfd_send_signal(pin, signal.SIGCONT); os.close(pin)
        case("SU5", {"singleton_peer": final_peer, "concurrent_restarts": [r.stdout for r in results],
                     "held_lock_refused": True, "impostor_not_signalled": True, "failed_hello_peer_unchanged": True})

    try:
        if not args.only_web:
            lifecycle_cases()
        webscope = root / "web-default"
        origin = serve("SU6-workbench-defaults", webscope, [])
        peer(webscope, candidate)
        code, body = request(origin, "/api/v1/connection", {})
        descriptor = json.loads(body)
        assert code == 200 and descriptor["scope"] == str(webscope) and descriptor["project"] == str(root / "project")
        assert descriptor["available"] and descriptor["collaboration"] is False
        if args.reuse_http_from:
            prior_resources = json.loads((args.reuse_http_from / "resources.json").read_text())
            assert prior_resources["artifacts"][str(candidate)] == resources["artifacts"][str(candidate)]
            prior_http = json.loads((args.reuse_http_from / "http-attempts.json").read_text())
            assets = [row["path"] for row in prior_http if row["path"].endswith((".js", ".css")) and row["status"] == 200]
            assert len(assets) >= 2 and any(row["path"] == "/api/v1/apply" and row["status"] == 403 for row in prior_http)
            resources["reused_web_http"] = str(args.reuse_http_from); save()
        else:
            code, html = request(origin, "/")
            assert code == 200
            assets = re.findall(r'(?:src|href)="([^"#]+\.(?:js|css))"', html.decode())
            assert len(assets) >= 2 and all(request(origin, asset)[0] == 200 for asset in assets)
            assert request(origin, "/api/v1/apply", {})[0] == 403
        browser_input = {"origin": origin, "project_label": "project", "descriptor": descriptor}
        (evidence / "browser-input.json").write_text(json.dumps(browser_input, indent=2) + "\n")
        node = json.loads((args.evidence / "tooling/tooling.json").read_text())["node"]
        browser = subprocess.run([node, str(Path(__file__).with_name("browser.mjs")), str(evidence / "browser-input.json"), str(evidence / "browser")],
                                 env={**fixture.env, "PATH": str(Path(node).parent) + ":/usr/bin:/bin"}, capture_output=True, text=True, timeout=45)
        (evidence / "browser-command.json").write_text(json.dumps({"exit_code": browser.returncode, "stdout": browser.stdout, "stderr": browser.stderr}, indent=2) + "\n")
        assert browser.returncode == 0, browser.stderr
        case("SU6", {"origin": origin, "descriptor": descriptor, "embedded_assets": assets, "browser": "passed"})

        explicit_scope = root / "web-explicit"; explicit_project = root / "other-project"; explicit_project.mkdir()
        unselected = root / "never-selected"
        explicit_origin = serve("SU7-workbench-explicit", unselected,
                                ["--scope", str(explicit_scope), "--project", str(explicit_project), "--collaboration"])
        peer(explicit_scope, candidate)
        code, body = request(explicit_origin, "/api/v1/connection", {})
        descriptor = json.loads(body)
        assert code == 200 and descriptor["scope"] == str(explicit_scope) and descriptor["project"] == str(explicit_project)
        assert descriptor["collaboration"] and descriptor["available"] and not unselected.exists()
        refused = request(explicit_origin, "/api/v1/read", {"scope": str(unselected), "generation": descriptor["generation"], "read": "Candidates"})
        assert refused[0] == 400 and not unselected.exists()
        case("SU7", {"origin": explicit_origin, "descriptor": descriptor, "request_cannot_select_scope": True})
        resources["result"] = "passed"
    except BaseException as error:
        resources["result"] = "failed"; resources["failure"] = repr(error); save()
        raise
    finally:
        for process, record in reversed(hosts):
            if process.poll() is None:
                resources["cleanup"].append(Fixture.terminate(record)); process.wait(timeout=5); save()
        for record in reversed(list(records.values())):
            resources["cleanup"].append(Fixture.terminate(record)); save()
        closed = []
        for port in resources["ports"]:
            with socket.socket() as check:
                check.settimeout(.2); assert check.connect_ex(("127.0.0.1", port)) != 0
            closed.append(port)
        resources["closed_ports"] = closed
        resources["root_removed"] = False; save()
        shutil.rmtree(root); resources["root_removed"] = not root.exists(); save()
    print(json.dumps({"result": resources["result"], "cases": len(resources["cases"]), "evidence": str(evidence), "root_removed": resources["root_removed"]}))


if __name__ == "__main__":
    main()
