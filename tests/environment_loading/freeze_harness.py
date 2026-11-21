#!/usr/bin/env python3
"""Retain one exact consumer-only harness for a new acceptance attempt."""

import argparse
import ast
import hashlib
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--reason", required=True)
    args = parser.parse_args()
    if Path(args.name).name != args.name:
        parser.error("name must be a simple path component")
    source = Path(__file__).resolve().parents[2]
    destination = args.evidence / args.name
    destination.mkdir(exist_ok=False)
    records = {}
    paths = [*(source / "tests/environment_loading").glob("*.py"), source / "tests/startup_usability/fixture.py"]
    for path in paths:
        relative = path.relative_to(source)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        data = path.read_bytes()
        ast.parse(data, filename=str(relative))
        target.write_bytes(data)
        target.chmod(0o444)
        records[str(relative)] = hashlib.sha256(data).hexdigest()
    receipt = {"root": str(destination), "files": records, "syntax_passed": True, "reason": args.reason}
    output = args.evidence / (args.name + ".json")
    output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"harness": str(destination), "receipt": str(output), "files": len(records)}))


if __name__ == "__main__":
    main()
