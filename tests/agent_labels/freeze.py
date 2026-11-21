#!/usr/bin/python3
"""Retain labels native/Web/fixture inputs and inspect their later drift."""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import stat

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    assert Path(args.name).name == args.name
    root, evidence = args.root.resolve(), args.evidence.resolve()
    receipt_path = evidence / (args.name + "-source-receipt.json")
    if args.check:
        receipt = json.loads(receipt_path.read_text())
        drift = [name for name, expected in receipt["files"].items() if not (root / name).is_file() or digest(root / name) != expected]
        print(json.dumps({"receipt": str(receipt_path), "changed_inputs": drift}, indent=2))
        return int(bool(drift))
    evidence.mkdir(parents=True, exist_ok=True)
    destination = evidence / (args.name + "-source")
    destination.mkdir(exist_ok=False)
    module_spec = importlib.util.spec_from_file_location("base_freeze", root / "scripts/freeze-acceptance-source.py")
    base = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(base)
    paths = {root / name for name in base.FILES if (root / name).is_file()}
    excluded = {"node_modules", "__pycache__", "dist", ".vite", ".cache"}
    for directory in [*base.DIRECTORIES, "web"]:
        paths.update(path for path in (root / directory).rglob("*") if path.is_file() and not excluded.intersection(path.relative_to(root).parts))
    # Preserve the exact criterion/case map basis alongside executable inputs.
    for name in ["AGENTS.md", ".docs/agent-classification.md", ".controller/agent-labels/AGENTS.md", ".controller/agent-labels/implementation.delivery.md"]:
        paths.add(root / name)
    files = {}
    for source in sorted(paths):
        assert not source.is_symlink(), source
        relative = source.relative_to(root)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(source.read_bytes())
        shutil.copystat(source, target)
        target.chmod(stat.S_IMODE(source.stat().st_mode) & ~0o222)
        files[str(relative)] = digest(target)
    changed = [name for name, expected in files.items() if digest(root / name) != expected]
    receipt = {"root": str(root), "snapshot": str(destination), "files": files, "changed_during_capture": changed,
               "limits": "Native, Web, locks, scripts, fixtures and criterion sources; dependency directories excluded and tool versions recorded by execution. Cargo-generated embedded asset bytes require the separate actual asset receipt."}
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"snapshot": str(destination), "receipt": str(receipt_path), "files": len(files), "changed_during_capture": changed}))
    return int(bool(changed))


if __name__ == "__main__":
    raise SystemExit(main())
