#!/usr/bin/env python3
"""Preserve one successful source-built artifact without replacing old evidence."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--build-receipt", type=Path, required=True)
    parser.add_argument("--profile", required=True)
    args = parser.parse_args()
    if Path(args.name).name != args.name:
        parser.error("name must be a single path component")
    build = json.loads(args.build_receipt.read_text())
    if build["exit_code"] != 0:
        parser.error("artifact requires a successful build receipt")
    artifact = args.evidence / args.name
    receipt_path = args.evidence / (args.name + "-receipt.json")
    if artifact.exists() or receipt_path.exists():
        parser.error("evidence exists; preserve it and choose a new name")
    with args.artifact.open("rb") as source, artifact.open("xb") as target:
        shutil.copyfileobj(source, target)
    artifact.chmod(0o555)
    with artifact.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    receipt = {"artifact": str(artifact), "sha256": digest,
               "bytes": artifact.stat().st_size, "source": build["cwd"],
               "profile": args.profile, "build_receipt": str(args.build_receipt)}
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
