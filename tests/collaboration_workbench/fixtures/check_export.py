#!/usr/bin/env python3
"""Independent C1–C3 probe for one immutable exporter revision.

The checker runs this from its own checkout of the reported commit. This
fixture deliberately retains failing baseline evidence instead of changing
criteria to match actual output. AgentX stores the attributed report only.
"""

import argparse
import csv
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def run(source, evidence):
    evidence.mkdir(parents=True, exist_ok=False)
    rows = json.loads((source / "orders.json").read_text(encoding="utf-8"))
    fields = ["id", "customer", "note", "total"]
    expected = [fields] + [[str(row[field]) for field in fields] for row in rows]
    invocations = []
    checks = []
    with tempfile.TemporaryDirectory(prefix="workbench-csv-check-") as directory:
        output = Path(directory) / "orders.csv"

        def invoke(label, extra=()):
            argv = [sys.executable, str(source / "export_orders.py"),
                    "--input", str(source / "orders.json"), "--output", str(output), *extra]
            result = subprocess.run(argv, cwd=directory, capture_output=True, text=True, timeout=15)
            invocations.append({"case": label, "argv": argv, "exit": result.returncode,
                                "stdout": result.stdout, "stderr": result.stderr})
            return result

        fresh = invoke("fresh-export")
        text = output.read_text(encoding="utf-8") if output.exists() else ""
        parsed = list(csv.reader(io.StringIO(text)))
        checks.append({"criterion": "C1", "passed": fresh.returncode == 0 and parsed == expected,
                       "expected": expected, "observed": parsed})
        (evidence / "fresh-output.csv").write_text(text, encoding="utf-8")
        sentinel = b"existing destination must survive\n"
        output.write_bytes(sentinel)
        refused = invoke("existing-refused")
        retained = output.read_bytes() == sentinel
        forced = invoke("explicit-force", ["--force"])
        forced_rows = list(csv.reader(io.StringIO(output.read_text(encoding="utf-8"))))
        checks.append({"criterion": "C2", "passed": refused.returncode != 0 and retained and forced.returncode == 0 and forced_rows == expected,
                       "refused_exit": refused.returncode, "unchanged": retained,
                       "forced_exit": forced.returncode, "forced_rows": forced_rows})
        checks.append({"criterion": "C3", "passed": fresh.returncode == 0 and not fresh.stderr,
                       "fresh_exit": fresh.returncode, "fresh_stderr": fresh.stderr})
    commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=source, capture_output=True, text=True, check=True).stdout.strip()
    report = {"source_commit": commit, "source_sha256": hashlib.sha256((source / "export_orders.py").read_bytes()).hexdigest(),
              "input_sha256": hashlib.sha256((source / "orders.json").read_bytes()).hexdigest(),
              "checks": checks, "invocations": invocations,
              "verdict": "passed" if all(check["passed"] for check in checks) else "failed"}
    (evidence / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["verdict"] == "passed" else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    raise SystemExit(run(args.source.resolve(), args.evidence.resolve()))
