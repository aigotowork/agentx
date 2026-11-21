#!/usr/bin/env python3
"""Preserve pre-workbench artifacts/receipts without executing any product.

Owned by the independent acceptance Task. Reads only named prior evidence and
demo metadata; never copies the live database or sends a service request.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def preserve(root, destination):
    records = []

    def copy_file(source, relative):
        source = source.resolve(strict=True)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        before = digest(source)
        with source.open("rb") as reader, target.open("xb") as writer:
            shutil.copyfileobj(reader, writer)
        target.chmod(0o555 if source.stat().st_mode & 0o111 else 0o444)
        actual = digest(target)
        after = digest(source)
        records.append({"source": str(source), "copy": str(target),
                        "bytes": target.stat().st_size, "sha256": actual,
                        "stable": before == actual == after})
        if not records[-1]["stable"]:
            raise RuntimeError(f"source changed while preserving {source}")

    def copy_tree(source, relative, excluded=()):
        if not source.exists():
            raise FileNotFoundError(source)
        for path in sorted(source.rglob("*")):
            if path.relative_to(source).parts[0] in excluded:
                continue
            if path.is_symlink():
                raise RuntimeError(f"unexpected preserved-input symlink: {path}")
            if path.is_file():
                copy_file(path, relative / path.relative_to(source))

    copy_file(root / ".bin/agentx", Path("baseline/agentx"))
    for path in sorted((root / ".bin").iterdir()):
        if path.is_dir() and "web" in path.name:
            copy_tree(path, Path("baseline/bin-assets") / path.name)
    copy_tree(root / "web/observability", Path("baseline/observability-source"))
    for name in ["package.json", "package-lock.json", "Cargo.toml", "Cargo.lock", "build.rs", "justfile"]:
        copy_file(root / name, Path("baseline/inputs") / name)

    prior = Path("/tmp/agentx-lan-9jf25e31")
    # This evidence root contains only the prior acceptance's disposable data.
    copy_tree(prior, Path("prior/lan"))
    native = Path("/tmp/agentx-ddd-integration-wlwzi0g_")
    for path in sorted(native.iterdir()):
        if path.is_file() and path.suffix in {".json", ".log", ".txt", ".diff", ".stderr"}:
            copy_file(path, Path("prior/native") / path.name)
    copy_file(native / "candidate-s10-package-agentx", Path("prior/native/accepted-agentx"))
    copy_tree(native / "candidate-s10-lint-source", Path("prior/native/accepted-source"),
              excluded=("target", "node_modules", ".bin"))

    demo = Path("/home/long/.local/state/agentx-observability-demo")
    for name in ["README.md", "runtime.json", "web.pid"]:
        copy_file(demo / name, Path("baseline/demo-metadata") / name)
    copy_file(demo / "bin/agentx", Path("baseline/demo-agentx"))
    services = []
    for pid in [1497466, 1497500]:
        proc = Path(f"/proc/{pid}")
        record = {"pid": pid, "present": proc.exists()}
        if proc.exists():
            record.update({"executable": str((proc / "exe").resolve()),
                           "start_ticks": (proc / "stat").read_text().rsplit(")", 1)[1].split()[19],
                           "argv": (proc / "cmdline").read_bytes().decode().split("\0")[:-1]})
        services.append(record)
    sockets = subprocess.run(["ss", "-ltnp"], text=True, capture_output=True, check=True).stdout
    (destination / "listener-baseline.txt").write_text(sockets)
    report = {"created_utc": datetime.now(timezone.utc).isoformat(), "checkout": str(root),
              "preservation": records, "protected_processes": services,
              "limits": "Read-only artifact/source/receipt preservation. No product execution, live database read/copy, request, stop, restart, installation or support claim."}
    (destination / "preservation.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"evidence": str(destination), "preserved_files": len(records),
                      "bytes": sum(item["bytes"] for item in records), "protected_processes": services}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    destination = Path(tempfile.mkdtemp(prefix="agentx-workbench-acceptance-"))
    preserve(args.root.resolve(), destination)


if __name__ == "__main__":
    main()
