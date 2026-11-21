#!/usr/bin/env python3
"""Capture selected build/test inputs without copying local user state.

Used by integration when concurrent implementation makes the shared checkout
an unstable compiler input. It never alters Git or the source checkout.
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import stat


DIRECTORIES = ["src", "tests", "scripts", "plugin", "skills"]
FILES = ["Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain", "rust-toolchain.toml", ".cargo/config", ".cargo/config.toml", "rustfmt.toml", "justfile", "package.json", "package-lock.json", "tsconfig.json"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--test-overlay-root", type=Path, help="Source of explicitly selected test-only corrections over a preserved input")
    parser.add_argument("--test-overlay", action="append", default=[], help="Relative test-only file to replace during capture, repeatable")
    parser.add_argument("--overlay-root", type=Path, help="Source of an explicitly authorized narrow production change over a preserved input")
    parser.add_argument("--overlay", action="append", default=[], help="Relative captured input to replace, repeatable; authorization stays in the Task")
    args = parser.parse_args()
    if Path(args.name).name != args.name:
        parser.error("name must be a single path component")
    if bool(args.test_overlay_root) != bool(args.test_overlay):
        parser.error("test overlays require both --test-overlay-root and explicit --test-overlay files")
    if bool(args.overlay_root) != bool(args.overlay):
        parser.error("source overlays require both --overlay-root and explicit --overlay files")
    if args.test_overlay and args.overlay:
        parser.error("select one explicit overlay mode per capture")
    for name in args.test_overlay:
        path = Path(name)
        if path.is_absolute() or ".." in path.parts or not (path.parts[0] == "tests" or path.name in {"acceptance_tests.rs", "test_support.rs"}):
            parser.error("overlays must be explicit test-only relative paths")
    overlays = set(args.test_overlay or args.overlay)
    overlay_root = args.test_overlay_root or args.overlay_root
    if any(Path(name).is_absolute() or ".." in Path(name).parts for name in overlays):
        parser.error("source overlays must be explicit relative paths")
    root = args.root.resolve()
    destination = args.evidence / (args.name + "-source")
    destination.mkdir(parents=True, exist_ok=False)
    paths = [root / name for name in FILES if (root / name).is_file()]
    for name in DIRECTORIES:
        paths.extend(path for path in (root / name).rglob("*") if path.is_file() and "__pycache__" not in path.parts and "node_modules" not in path.parts)
    # A narrow authorized change can add a new generator or fixture. Record
    # those explicit additions rather than silently omitting new build inputs.
    base_paths = {str(path.relative_to(root)) for path in paths}
    added_overlays = overlays - base_paths
    paths.extend(root / name for name in added_overlays)
    digests = {}
    inputs = {}
    for path in sorted(paths):
        relative = path.relative_to(root)
        if str(relative) in overlays:
            path = overlay_root.resolve() / relative
        if path.is_symlink():
            raise RuntimeError(f"unexpected symlink in build inputs: {path}")
        data = path.read_bytes()
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        shutil.copystat(path, target)
        target.chmod(stat.S_IMODE(path.stat().st_mode) & ~0o222)
        digests[str(relative)] = hashlib.sha256(data).hexdigest()
        inputs[str(relative)] = path
    if overlays - inputs.keys():
        raise RuntimeError(f"test overlay does not replace an existing captured input: {sorted(overlays - inputs.keys())}")
    changed = [name for name, digest in digests.items() if hashlib.sha256(inputs[name].read_bytes()).hexdigest() != digest]
    receipt = {"root": str(root), "snapshot": str(destination), "files": digests,
               "test_overlay_root": str(args.test_overlay_root.resolve()) if args.test_overlay_root else None, "test_overlays": sorted(args.test_overlay),
               "source_overlay_root": str(args.overlay_root.resolve()) if args.overlay_root else None, "source_overlays": sorted(args.overlay),
               "added_overlay_inputs": sorted(added_overlays),
               "changed_during_capture": changed, "limits": "Selected build/test inputs only; source paths, not Git commit identity. No live state copied. Preserve this snapshot; capture a new name after relevant fixes."}
    output = args.evidence / (args.name + "-source-receipt.json")
    output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"snapshot": str(destination), "receipt": str(output), "files": len(digests), "changed_during_capture": changed}))
    return int(bool(changed))


if __name__ == "__main__":
    raise SystemExit(main())
