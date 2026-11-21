#!/usr/bin/env python3
"""Retain precise lock changes; never authorize unrelated dependency upgrades."""

import argparse
import hashlib
import json
from pathlib import Path


def compare(before, after):
    old = json.loads(before.read_text())["packages"]
    new = json.loads(after.read_text())["packages"]
    identity = ("version", "resolved", "integrity")
    changed = []
    for name in sorted(old.keys() & new.keys() - {""}):
        fields = {key: {"old": old[name].get(key), "new": new[name].get(key)}
                  for key in identity if old[name].get(key) != new[name].get(key)}
        if fields:
            changed.append({"package": name, "changes": fields})
    return {"before_sha256": hashlib.sha256(before.read_bytes()).hexdigest(),
            "after_sha256": hashlib.sha256(after.read_bytes()).hexdigest(),
            "before_packages": len(old), "after_packages": len(new),
            "changed_existing": changed, "removed": sorted(old.keys() - new.keys()),
            "added": {name: {key: new[name].get(key) for key in identity}
                      for name in sorted(new.keys() - old.keys())},
            "unchanged_existing_identities": len(old.keys() & new.keys()) - 1 - len(changed),
            "limits": "Identity comparison only; acceptance owner must disposition each changed/removed existing entry from the actual dependency closure before publishing."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    report = compare(args.before, args.after)
    with args.receipt.open("x") as receipt:
        receipt.write(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: value for key, value in report.items() if key != "added"}, indent=2))
