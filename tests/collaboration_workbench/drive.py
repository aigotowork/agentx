#!/usr/bin/env python3
"""Explicit steps for the one authorized actual-product outcome fixture."""
import argparse
import json
from pathlib import Path
import subprocess
import time

from http_client import WorkbenchHttp
from session import ActualSession


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--label", required=True)
    operations = parser.add_subparsers(dest="operation", required=True)
    start = operations.add_parser("start"); start.add_argument("artifact", type=Path)
    launch = operations.add_parser("launch"); launch.add_argument("role", choices=["implementer", "checker"])
    serve = operations.add_parser("serve"); serve.add_argument("kind", choices=["readonly", "diagnostic"])
    browser = operations.add_parser("browser"); browser.add_argument("input", type=Path)
    read = operations.add_parser("read"); read.add_argument("work")
    wait = operations.add_parser("wait"); wait.add_argument("work"); wait.add_argument("field", choices=["questions", "results", "checks"]); wait.add_argument("count", type=int); wait.add_argument("--seconds", type=int, default=180)
    args = parser.parse_args()
    evidence = args.evidence.resolve()
    resources_path = evidence / "runtime-resources.json"
    resources = json.loads(resources_path.read_text()); plan = json.loads((evidence / "runtime-plan.json").read_text())
    session = ActualSession(resources["owned_root"], evidence, resources["operator_name"])
    def save():
        resources_path.write_text(json.dumps(resources, indent=2) + "\n")
    if args.operation == "start":
        (evidence / "runtime-resources-pre-start.json").write_text(resources_path.read_text())
        session.stage(args.artifact)
        resources["status"] = "staged"; save()
        process = session.serve(args.label, plan["ports"]["collaborative"])
        resources["processes"].append(process); resources["ports"].append(process["port"]); save()
        resources["processes"].append(session.daemon_record())
        resources["status"] = "collaborative-host-started"; save(); print(json.dumps(resources))
    elif args.operation == "launch":
        result = session.launch(args.label, args.role)
        resources["provider_participants"].extend({"role": args.role, "name": name, "launch": result} for name in result["names"])
        save(); print(json.dumps(result))
        if result["launched"] != 1 or result["failed"]:
            raise RuntimeError("actual role launch incomplete; retain result and clean only recorded fixture resources")
    elif args.operation == "serve":
        process = session.serve(args.label, plan["ports"][args.kind], collaboration=False, diagnostics=args.kind == "diagnostic")
        resources["processes"].append(process); resources["ports"].append(process["port"]); save(); print(json.dumps(process))
    elif args.operation == "browser":
        info = json.loads((evidence / "tooling/tooling.json").read_text())
        command = [info["node"], str(Path(__file__).with_name("browser_flow.mjs")), str(args.input.resolve()), str(evidence / args.label)]
        raise SystemExit(subprocess.call(["python3", str(evidence / "run-acceptance-check.py"), "--evidence", str(evidence), "--name", args.label, "--", *command]))
    else:
        http = WorkbenchHttp(plan["browser_origin"], evidence / f"{args.label}-http")
        descriptor = http.post("connection", "connection", {})
        reference = {key: descriptor[key] for key in ["scope", "project", "generation"]}; reference["id"] = args.work
        deadline = time.monotonic() + (args.seconds if args.operation == "wait" else 0)
        index = 0
        while True:
            response = http.post(f"detail-{index:03}", "read", {"generation": descriptor["generation"], "read": {"Detail": reference}})
            detail = response["Read"]["Detail"]
            if args.operation == "read" or len(detail["work"][args.field]) >= args.count:
                (evidence / f"{args.label}-detail.json").write_text(json.dumps(detail, ensure_ascii=False, indent=2) + "\n")
                print(json.dumps(detail, ensure_ascii=False)); break
            if time.monotonic() >= deadline:
                raise TimeoutError(f"actual work {args.work}: {args.field} did not reach {args.count}")
            index += 1; time.sleep(2)


if __name__ == "__main__":
    main()
