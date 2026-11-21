#!/usr/bin/env python3
"""Existing deficient exporter: preserve this baseline, then repair it."""

import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    rows = json.loads(args.input.read_text(encoding="utf-8"))
    fields = ["id", "customer", "note", "total"]
    lines = [",".join(fields)]
    lines.extend(",".join(str(row[field]) for field in fields) for row in rows)
    args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
