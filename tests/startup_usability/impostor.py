#!/usr/bin/env python3
"""Disposable same-user endpoint that cannot supply a verified daemon instance."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import struct
import sys


def main():
    scope = Path(sys.argv[1]).absolute()
    marker = Path(sys.argv[2])
    if scope.parent.parent != Path("/tmp") or not scope.parent.name.startswith("agentx-startup-fixture-"):
        raise ValueError("requires an allocated disposable scope")
    state = scope / ".daemon"; state.mkdir(parents=True, mode=0o700); scope.chmod(0o700)
    endpoint = Path(f"/tmp/agentx-{os.geteuid()}/{hashlib.sha256(os.fsencode(scope)).hexdigest()}.sock")
    def signalled(signum, _frame):
        marker.write_text(json.dumps({"signal": signum, "pid": os.getpid()}) + "\n")
        raise SystemExit(0)
    signal.signal(signal.SIGTERM, signalled)
    with (state / "owner.lock").open("w") as lock, socket.socket(socket.AF_UNIX) as server:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        server.bind(str(endpoint)); endpoint.chmod(0o600); server.listen(2)
        print("ready", flush=True)
        try:
            while True:
                connection, _ = server.accept()
                with connection:
                    connection.settimeout(5)
                    prefix = connection.recv(4)
                    if len(prefix) != 4:
                        continue
                    length = struct.unpack("!I", prefix)[0]
                    if length > 65536:
                        raise ValueError("unexpected request size")
                    body = b""
                    while len(body) < length:
                        part = connection.recv(length - len(body))
                        if not part:
                            raise EOFError("truncated fixture status Call")
                        body += part
                    assert json.loads(body)["Call"]["request"] == "DaemonStatus"
                    response = json.dumps({"Failure": {"kind": "incompatible", "detail": "owned impostor fixture"}}).encode()
                    connection.sendall(struct.pack("!I", len(response)) + response)
        finally:
            endpoint.unlink()


if __name__ == "__main__":
    main()
