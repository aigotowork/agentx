#!/usr/bin/env python3
"""Retain independently hashed outcome driver inputs before an actual attempt."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    source = Path(__file__).resolve().parent
    args.destination.mkdir()
    digests = {}
    for path in sorted(source.rglob("*")):
        if not path.is_file() or "__pycache__" in path.parts:
            continue
        relative = path.relative_to(source)
        data = path.read_bytes()
        target = args.destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        shutil.copymode(path, target)
        digest = hashlib.sha256(data).hexdigest()
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"input changed during preservation: {relative}")
        digests[str(relative)] = digest
    args.destination.with_suffix(".json").write_text(json.dumps({"source": str(source), "retained": str(args.destination), "sha256": digests}, indent=2) + "\n")
    print(json.dumps({"retained": str(args.destination), "files": len(digests)}))


if __name__ == "__main__":
    main()
