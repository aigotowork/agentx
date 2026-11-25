"""Upgrade an original release daemon using the new archive on the same OS."""

import argparse
import contextlib
import json
import os
from pathlib import Path
import re
import signal
import sqlite3
import subprocess
import tempfile
import time

verify = __import__("verify-native-artifact")


def test_upgrade(new, old, root, version):
    home = root / "home"
    home.mkdir()
    children = []
    patterns = {key: re.compile(rf"\b{key}=([^\s]+)") for key in ("pid", "instance", "generation")}

    def env(scope):
        inherited = {key: os.environ[key] for key in ("PATH", "SystemRoot", "WINDIR", "COMSPEC", "PATHEXT") if key in os.environ}
        return {**inherited, "AGENTX_DIR": str(scope), "HOME": str(home), "USERPROFILE": str(home),
                "APPDATA": str(home), "LOCALAPPDATA": str(home), "CI": "1", "NO_COLOR": "1", "AGENTX_UPDATE_VERIFY": "1"}

    def run(binary, scope, *args, extra=None, check=True):
        result = subprocess.run([str(binary), *args], env={**env(scope), **(extra or {})}, cwd=root,
                                stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=50)
        if check and result.returncode:
            raise ValueError(f"{args[0]} exited {result.returncode}: {result.stderr.strip()}")
        return result.stdout

    def status(binary, scope):
        output = run(binary, scope, "daemon", "status", check=False)
        if "Daemon: running\n" not in output:
            return None
        return {key: pattern.search(output).group(1) for key, pattern in patterns.items()}

    def start(scope):
        scope.mkdir(mode=0o700)
        process = subprocess.Popen([str(old), "daemon", "serve", "--scope", str(scope)], env=env(scope), cwd=root,
                                   stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        children.append(process)
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline and process.poll() is None:
            before = status(old, scope)
            if before is not None and int(before["pid"]) == process.pid:
                return process, before
            time.sleep(0.05)
        raise ValueError("published daemon never became ready")

    target, neighbor = root / "target", root / "neighbor"
    try:
        old_target, before = start(target)
        old_neighbor, neighbor_before = start(neighbor)
        run(old, target, "start", "--name", "receiver", extra={"AGENTX_PROCESS_ID": "archive-fixture"})
        sent = json.loads(run(old, target, "send", "--from", "upgrade-fixture", "--json", "@receiver", "--", "Retained original receipt"))
        with contextlib.closing(sqlite3.connect((target / "agentx.db").as_uri() + "?mode=ro", uri=True)) as db:
            request_id, = db.execute("SELECT request_id FROM message_acceptances WHERE event_id=?", (sent["event_id"],)).fetchone()
        restarted = run(new, target, "daemon", "restart")
        verify.require("Native IPC upgraded." in restarted, "legacy restart path was not exercised")
        verify.require(old_target.wait(timeout=8) == 0, "legacy daemon did not exit gracefully")
        after = status(new, target)
        verify.require(after and before["instance"] != after["instance"] and before["generation"] == after["generation"],
                       "restart did not retain the original generation")
        receipt = json.loads(run(new, target, "send", "--receipt", request_id, "--json"))
        verify.require(receipt["event_id"] == sent["event_id"] and receipt["delivered_to"] == sent["delivered_to"]
                       and receipt["request_id"] == request_id, "original receipt was lost or changed")
        verify.require(old_neighbor.poll() is None and status(old, neighbor) == neighbor_before, "neighbor was changed")
        print(f"Packaged {version} adopted 0.7.33, kept its original receipt and left the neighbor unchanged")
    finally:
        try:
            replacement = status(new, target) if target.exists() else None
            # The direct-status peer is selected by this test's private, newly
            # allocated scope. Never signal a PID from the old child slot.
            if replacement and all(int(replacement["pid"]) != owned.pid for owned in children):
                with contextlib.suppress(ProcessLookupError):
                    os.kill(int(replacement["pid"]), signal.SIGTERM)
                deadline = time.monotonic() + 12
                while status(new, target) and time.monotonic() < deadline:
                    time.sleep(0.1)
        finally:
            for owned in children:
                if owned.poll() is None:
                    owned.terminate()
                owned.wait(timeout=12)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate_dir", type=Path)
    parser.add_argument("legacy_dir", type=Path)
    parser.add_argument("target")
    parser.add_argument("version")
    args = parser.parse_args()
    windows = args.target.endswith("-pc-windows-msvc")
    archive = f"agentx-{args.target}.{'zip' if windows else 'tar.gz'}"
    name = "agentx.exe" if windows else "agentx"
    with tempfile.TemporaryDirectory(prefix="agentx-upgrade-") as temp:
        root = Path(temp).resolve()
        binaries = []
        for directory, label, version in [(args.candidate_dir, "candidate", args.version), (args.legacy_dir, "legacy", "0.7.33")]:
            file = directory / archive
            verify.check_checksum(file)
            executable = root / label / name
            executable.parent.mkdir()
            verify.extract_cli(file, executable)
            verify.check_architecture(executable, args.target)
            output = subprocess.check_output([str(executable), "--version"], cwd=root, text=True, timeout=20).strip()
            verify.require(output == f"agentx {version}", f"wrong {label} version: {output}")
            binaries.append(executable)
        test_upgrade(*binaries, root, args.version)


if __name__ == "__main__":
    main()
