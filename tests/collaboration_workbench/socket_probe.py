#!/usr/bin/env python3
"""Read only one selected fixture UDS identity; no native frame or effect."""
import hashlib
import json
import os
from pathlib import Path
import socket
import struct
import sys

scope = Path(sys.argv[1]).resolve(strict=True)
endpoint = Path(f"/tmp/agentx-{os.geteuid()}/{hashlib.sha256(str(scope).encode()).hexdigest()}.sock")
record = {"scope": str(scope), "endpoint": str(endpoint), "euid": os.geteuid(), "pid": os.getpid(), "self_pid_namespace": os.readlink('/proc/self/ns/pid')}
try:
    stat = endpoint.stat(); record.update(inode=stat.st_ino, device=stat.st_dev, mode=oct(stat.st_mode), owner=stat.st_uid)
    with socket.socket(socket.AF_UNIX) as client:
        client.settimeout(3)
        client.connect(str(endpoint)); record['connected'] = True
        record['peer_pid'], record['peer_uid'], record['peer_gid'] = struct.unpack('3i', client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
except OSError as error:
    record.update(errno=error.errno, error=str(error))
print(json.dumps(record))
