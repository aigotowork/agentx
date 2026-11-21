"""Disposable startup acceptance primitives; importing creates no resources.

The owning Task gates execution on coherent fixed source and owns exact
artifacts/scopes. No Provider launch, default data access or process search.
"""

import hashlib
import json
import os
from pathlib import Path
import select
import signal
import socket
import struct
import subprocess
import time


def process_record(pid):
    proc = Path(f"/proc/{pid}")
    return {"pid": pid, "start_ticks": (proc / "stat").read_text().rsplit(")", 1)[1].split()[19],
            "executable": str((proc / "exe").resolve(strict=True)),
            "argv": (proc / "cmdline").read_bytes().rstrip(b"\0").decode().split("\0")}


class Fixture:
    """Commands target only explicit owned data and preserve each attempt once."""

    def __init__(self, root, evidence):
        self.root = Path(root).resolve(strict=True)
        self.evidence = Path(evidence).resolve(strict=True)
        if self.root.parent != Path("/tmp") or not self.root.name.startswith("agentx-startup-fixture-"):
            raise ValueError("requires the Task's allocated disposable root")
        for name in ["project", "home", "tmp"]:
            (self.root / name).mkdir(exist_ok=True)
        self.env = {"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8", "NO_COLOR": "1", "CI": "1",
                    "HOME": str(self.root / "home"), "TMPDIR": str(self.root / "tmp"),
                    "XDG_CONFIG_HOME": str(self.root / "home/config"),
                    "XDG_CACHE_HOME": str(self.root / "home/cache"),
                    "XDG_DATA_HOME": str(self.root / "home/share"),
                    "AGENTX_UPDATE_VERIFY": "1"}

    def run(self, label, artifact, scope, arguments, timeout=45, cwd=None):
        """Record exact command/output before the caller judges its outcome."""
        scope = Path(scope).absolute()
        if not scope.is_relative_to(self.root):
            raise ValueError("scope must be inside the owned fixture root")
        receipt = self.evidence / f"{label}.json"
        if receipt.exists():
            raise FileExistsError(receipt)
        command = [str(Path(artifact).resolve(strict=True)), *arguments]
        started = time.monotonic()
        try:
            result = subprocess.run(command, cwd=cwd or self.root / "project",
                                    env={**self.env, "AGENTX_DIR": str(scope)}, stdin=subprocess.DEVNULL,
                                    capture_output=True, text=True, timeout=timeout, start_new_session=True)
        except subprocess.TimeoutExpired as error:
            decode = lambda value: value.decode(errors="replace") if isinstance(value, bytes) else value
            receipt.write_text(json.dumps({"command": command, "scope_env": str(scope), "exit_code": "timeout",
                                           "stdout": decode(error.stdout), "stderr": decode(error.stderr),
                                           "elapsed_seconds": time.monotonic() - started}, indent=2) + "\n")
            raise
        record = {"command": command, "scope_env": str(scope), "cwd": str(cwd or self.root / "project"),
                  "exit_code": result.returncode, "elapsed_seconds": time.monotonic() - started,
                  "stdout": result.stdout, "stderr": result.stderr}
        receipt.write_text(json.dumps(record, indent=2) + "\n")
        return result

    def peer(self, scope, artifact):
        """Bind only this scope's kernel peer to its fixed executable/argv."""
        scope = Path(scope).resolve(strict=True)
        if not scope.is_relative_to(self.root):
            raise ValueError("peer scope is outside this fixture")
        endpoint = f"/tmp/agentx-{os.geteuid()}/{hashlib.sha256(os.fsencode(scope)).hexdigest()}.sock"
        with socket.socket(socket.AF_UNIX) as client:
            client.connect(endpoint)
            pid, uid, _gid = struct.unpack("3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        record = process_record(pid)
        expected = str(Path(artifact).resolve(strict=True))
        if uid != os.geteuid() or record["executable"] != expected or record["argv"][1:5] != ["daemon", "serve", "--scope", str(scope)]:
            raise RuntimeError({"unowned_endpoint": record, "scope": str(scope)})
        return {**record, "scope": str(scope), "endpoint": endpoint}

    @staticmethod
    def terminate(record, timeout=40):
        """Gracefully stop only a recorded incarnation using a pinned pidfd."""
        try:
            descriptor = os.pidfd_open(record["pid"])
        except ProcessLookupError:
            return {**record, "already_exited": True}
        try:
            current = process_record(record["pid"])
            if any(current[key] != record[key] for key in current):
                raise RuntimeError("recorded fixture process identity changed")
            signal.pidfd_send_signal(descriptor, signal.SIGTERM)
            poll = select.poll(); poll.register(descriptor, select.POLLIN)
            if not poll.poll(int(timeout * 1000)):
                raise TimeoutError("exact owned fixture did not exit after graceful stop")
            return {**record, "stopped": True}
        finally:
            os.close(descriptor)
