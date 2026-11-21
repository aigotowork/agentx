#!/usr/bin/env python3
"""Summarize retained Cargo/rustfmt logs without rerunning or altering checks."""

import argparse
import json
from pathlib import Path
import re


def summarize(log, source_root):
    text = log.read_text(errors="replace")
    diagnostics = []
    starts = list(re.finditer(r"^(warning|error)(?:\[[^\]]+\])?:[^\n]*", text, re.MULTILINE))
    for index, start in enumerate(starts):
        end = starts[index + 1].start() if index + 1 < len(starts) else len(text)
        block = text[start.start():end]
        site = re.search(r"-->\s+([^\n]+)", block)
        diagnostics.append({"level": start[1], "message": start[0], "site": site[1] if site else None})
    prefix = str(source_root).rstrip("/") + "/" if source_root else ""
    formatting = sorted({path.removeprefix(prefix) for path in re.findall(r"^Diff in (.*?):\d+:", text, re.MULTILINE)})
    return {"log": str(log.resolve()), "diagnostics": diagnostics, "formatting_files": formatting}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("summary exists; preserve it and choose a new name")
    result = summarize(args.log, args.source_root)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"output": str(args.output), "located_warnings": sum(row["level"] == "warning" and row["site"] is not None for row in result["diagnostics"]),
                      "located_errors": sum(row["level"] == "error" and row["site"] is not None for row in result["diagnostics"]), "formatting_files": len(result["formatting_files"])}))


if __name__ == "__main__":
    main()
