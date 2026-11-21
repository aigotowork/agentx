#!/usr/bin/env python3
"""DD-12 fixed-binary CLI latency/contention; all state is disposable.

Run identical arguments for preserved baseline and candidate. This measures
end-to-end CLI work, including process startup; it is not an IPC microbenchmark.
Reports retain failed operations and raw latency samples. Linux only: scoped
detached daemon cleanup is verified through executable and exact scope argv.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time


def fixture_daemons(binary, scope):
    found = []
    for proc in Path("/proc").iterdir():
        if not proc.name.isdecimal():
            continue
        try:
            if (proc / "exe").resolve() != binary:
                continue
            argv = (proc / "cmdline").read_bytes().split(b"\0")
            expected = [b"daemon", b"serve", b"--scope", os.fsencode(scope)]
            if argv[1:5] == expected:
                found.append(int(proc.name))
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            continue
    return found


def stop_fixture_daemons(binary, scope):
    owned = fixture_daemons(binary, scope)
    for pid in owned:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    deadline = time.monotonic() + 3
    while fixture_daemons(binary, scope) and time.monotonic() < deadline:
        time.sleep(0.02)
    for pid in fixture_daemons(binary, scope):
        os.kill(pid, signal.SIGKILL)
    deadline = time.monotonic() + 3
    while fixture_daemons(binary, scope) and time.monotonic() < deadline:
        time.sleep(0.02)
    remaining = fixture_daemons(binary, scope)
    if remaining:
        raise RuntimeError(f"fixture daemon cleanup incomplete: {remaining}")
    return owned


def summarize(rows, elapsed):
    values = sorted(row["milliseconds"] for row in rows)
    quantile = lambda part: values[max(0, math.ceil(len(values) * part) - 1)]
    return {
        "operations": len(rows),
        "errors": sum(row["code"] != 0 for row in rows),
        "wall_seconds": elapsed,
        "operations_per_second": len(rows) / elapsed,
        "p50_ms": quantile(0.5),
        "p95_ms": quantile(0.95),
        "max_ms": max(values),
        "samples": rows,
    }


def measure(binary, count, workers):
    with tempfile.TemporaryDirectory(prefix="agentx-ddd-measure-") as root_name:
        root = Path(root_name)
        scope = root / "data"
        home = root / "home"
        workspace = root / "workspace"
        for directory in [scope, home, workspace, root / "tmp"]:
            directory.mkdir()
        environment = {
            "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8",
            "HOME": str(home), "AGENTX_DIR": str(scope), "CI": "1", "NO_COLOR": "1",
            "AGENTX_UPDATE_VERIFY": "1",
            "XDG_CONFIG_HOME": str(home / "config"), "XDG_CACHE_HOME": str(home / "cache"),
            "XDG_DATA_HOME": str(home / "share"), "CODEX_HOME": str(home / "codex"),
            "CLAUDE_CONFIG_DIR": str(home / "claude"), "TMPDIR": str(root / "tmp"),
        }

        def run(arguments):
            started = time.perf_counter()
            try:
                result = subprocess.run([str(binary), *arguments], env=environment, cwd=workspace,
                                        stdin=subprocess.DEVNULL, capture_output=True, timeout=15,
                                        start_new_session=True, text=True)
                return {"milliseconds": (time.perf_counter() - started) * 1000,
                        "code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
            except subprocess.TimeoutExpired:
                return {"milliseconds": (time.perf_counter() - started) * 1000,
                        "code": -1, "stdout": "", "stderr": "fixture deadline exceeded"}

        def batch(arguments, concurrency):
            started = time.perf_counter()
            with ThreadPoolExecutor(max_workers=concurrency) as pool:
                rows = list(pool.map(lambda _: run(arguments), range(count)))
            return summarize(rows, time.perf_counter() - started)

        result = {"scope": str(scope), "count": count, "workers": workers}
        try:
            result["cold_list"] = run(["list", "--json"])
            if result["cold_list"]["code"] != 0 or json.loads(result["cold_list"]["stdout"]) != []:
                raise RuntimeError(f"fresh-scope isolation failed: {result['cold_list']}")
            result["warm_list_sequential"] = batch(["list", "--json"], 1)
            result["warm_list_concurrent"] = batch(["list", "--json"], workers)
            result["warm_events_concurrent"] = batch(["events", "--last", "20"], workers)
            # Each start is a distinct short-lived manual participant. This
            # exercises concurrent durable writes without a real Provider.
            result["start_concurrent"] = batch(["start"], workers)
        finally:
            result["cleaned_daemon_pids"] = stop_fixture_daemons(binary, scope)
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--count", type=int, default=30)
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not Path("/proc/self/fd").is_dir() or args.count < 1 or args.workers < 1:
        parser.error("requires Linux /proc and positive count/workers")
    binary = args.binary.resolve(strict=True)
    receipt = {"label": args.label, "binary": str(binary),
               "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
               "clock": "perf_counter", "platform": os.uname().sysname,
               "limits": "Local end-to-end debug/release bytes as identified; no Provider/network workload. Cold sample includes fresh DB initialization. Warm samples share one scope. No speed claim from a single run."}
    try:
        receipt["measurements"] = measure(binary, args.count, args.workers)
    except Exception as error:
        receipt["failure"] = str(error)
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({key: value for key, value in receipt.items() if key != "measurements"}))
    return int("failure" in receipt or any(isinstance(row, dict) and row.get("errors", 0) for row in receipt.get("measurements", {}).values()))


if __name__ == "__main__":
    raise SystemExit(main())
