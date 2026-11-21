"""Linux process checks run on the machine that owns the private fixture root."""

import hashlib
import json
import os
from pathlib import Path
import select
import signal
import socket
import struct
import sys


def process(pid):
    proc = Path(f"/proc/{pid}")
    return {"pid": pid, "exe": str((proc / "exe").resolve(strict=True)),
            "argv": [os.fsdecode(arg) for arg in (proc / "cmdline").read_bytes().rstrip(b"\0").split(b"\0")],
            "start_ticks": (proc / "stat").read_text().rsplit(")", 1)[1].split()[19]}


def peer(root, candidate):
    scope = root / "scope"
    digest = hashlib.sha256(os.fsencode(scope)).hexdigest()
    with socket.socket(socket.AF_UNIX) as client:
        client.connect(f"/tmp/agentx-{os.geteuid()}/{digest}.sock")
        pid, uid, _gid = struct.unpack("3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        record = process(pid)
    if uid != os.geteuid() or record["exe"] != candidate or record["argv"][1:5] != [
        "daemon", "serve", "--scope", str(scope)
    ]:
        raise AssertionError("Selected scope has no verified fixture daemon peer")
    return record


def terminate(record, allow_kill=False):
    try:
        descriptor = os.pidfd_open(record["pid"])
    except ProcessLookupError:
        return
    try:
        if process(record["pid"]) != record:
            raise AssertionError("Process identity no longer matches the fixture")
        signal.pidfd_send_signal(descriptor, signal.SIGTERM)
        poller = select.poll()
        poller.register(descriptor, select.POLLIN)
        if not poller.poll(8000):
            if not allow_kill:
                raise TimeoutError("Verified fixture daemon did not stop gracefully")
            signal.pidfd_send_signal(descriptor, signal.SIGKILL)
            if not poller.poll(3000):
                raise TimeoutError("Verified fixture process did not exit")
    except (FileNotFoundError, ProcessLookupError):
        pass
    finally:
        os.close(descriptor)


def fixture_processes(root, candidate):
    """Only select known fixture executables with its otherwise unique private HOME."""
    private_home = os.fsencode(f"HOME={root / 'home'}")
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            if entry.stat().st_uid != os.getuid() or private_home not in (entry / "environ").read_bytes().split(b"\0"):
                continue
            record = process(int(entry.name))
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            continue
        if record["exe"] == candidate or Path(record["exe"]).name in ("codex", "codex-code-mode-host", "caddy", "tmux"):
            yield record


def public_record(record):
    # Executable, peer PID and incarnation suffice. Child argv can carry private context.
    return {key: value for key, value in record.items() if key != "argv"}


def main(action, root, candidate):
    root = Path(root).resolve(strict=True)
    if root.parent != Path("/tmp") or not root.name.startswith("axd-"):
        raise ValueError("Expected the independently allocated private fixture root")
    if action in ("peer", "stop"):
        record = peer(root, candidate)
        if action == "stop":
            terminate(record)
        return public_record(record)
    if action == "workers":
        return [public_record(record) for record in fixture_processes(root, candidate)
                if record["exe"] == candidate and record["argv"][1:2] == ["__cloud-host"]]
    if action == "cleanup":
        stopped = []
        # Stop the native owner first so that it cannot recreate any host during cleanup.
        for record in sorted(fixture_processes(root, candidate),
                             key=lambda item: item["argv"][1:3] != ["daemon", "serve"]):
            terminate(record, allow_kill=True)
            stopped.append(public_record(record))
        if list(fixture_processes(root, candidate)):
            raise RuntimeError("Owned fixture processes remain after cleanup")
        return stopped
    raise ValueError("Unknown fixture station action")


if __name__ == "__main__":
    print(json.dumps(main(*sys.argv[1:])))
