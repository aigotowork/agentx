#!/usr/bin/env python3
"""Bind the released artifact to fixed inputs and audit only owned resources.

Run after all product attempts finish. Retained evidence is read-only; only the
three named disposable preparation/check directories may be removed here.
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--name", default="final-audit")
    args = parser.parse_args()
    evidence, source, artifact = (p.resolve(strict=True) for p in [args.evidence, args.source, args.artifact])
    repo = args.repository.resolve(strict=True)
    if not source.is_relative_to(evidence) or not artifact.is_relative_to(source):
        parser.error("fixed source and artifact must belong to the evidence root")
    if Path(args.name).name != args.name:
        parser.error("receipt name must be a simple path component")
    output = evidence / (args.name + ".json")
    if output.exists():
        parser.error("final receipt exists; preserve it")
    inputs = json.loads((evidence / (source.name + "-receipt.json")).read_text())["files"]
    fixed_changes, shared_changes = [], []
    for name, expected in inputs.items():
        fixed, shared = source / name, repo / name
        if not fixed.is_file() or digest(fixed) != expected:
            fixed_changes.append(name)
        if not shared.is_file() or digest(shared) != expected:
            shared_changes.append(name)
    records, roots, endpoints, ports = {}, [], set(), []

    def collect(value, origin):
        if isinstance(value, dict):
            if "pid" in value and "start_ticks" in value:
                key = (value["pid"], str(value["start_ticks"]))
                records.setdefault(key, set()).add(origin)
            if "endpoint" in value:
                endpoints.add(value["endpoint"])
            for child in value.values():
                collect(child, origin)
        elif isinstance(value, list):
            for child in value:
                collect(child, origin)

    for directory in sorted(evidence.glob("product-r*")):
        for path in directory.rglob("*.json"):
            collect(json.loads(path.read_text()), str(path.relative_to(evidence)))
        resources = json.loads((directory / "resources.json").read_text())
        roots.append({"root": resources["root"], "exists": Path(resources["root"]).exists()})
        ports.extend(resources.get("ports", []))
    executing = []
    for (pid, start), origins in records.items():
        try:
            tail = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
            if tail[19] == start and tail[0] != "Z":
                executing.append({"pid": pid, "start_ticks": start, "sources": sorted(origins)})
        except FileNotFoundError:
            pass

    # Match only the two acceptance-owned product image inodes before reading
    # any process argv. Protected/user process command lines are not inspected.
    images = {}
    for path in [evidence / "r1-fixed-source/.bin/agentx", artifact]:
        stat = path.stat()
        images[(stat.st_dev, stat.st_ino)] = str(path)
    owned_images_running = []
    for proc in Path("/proc").iterdir():
        if not proc.name.isdigit():
            continue
        try:
            stat = (proc / "exe").stat()
            image = images.get((stat.st_dev, stat.st_ino))
            if image:
                tail = (proc / "stat").read_text().rsplit(")", 1)[1].split()
                owned_images_running.append({"pid": int(proc.name), "start_ticks": tail[19], "image": image,
                                             "argv": (proc / "cmdline").read_bytes().decode().split("\0")})
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            pass

    baseline = json.loads((evidence / "protected-identities.json").read_text())
    protected = []
    for previous in [baseline["workbench_observed"], baseline["project_daemon_observed_from_prior_record"],
                     *baseline["protected_demo_observed_from_prior_record"]]:
        proc = Path(f"/proc/{previous['pid']}")
        try:
            tail = (proc / "stat").read_text().rsplit(")", 1)[1].split()
            stat = (proc / "exe").stat()
            observed = {"pid": previous["pid"], "start_ticks": tail[19],
                        "image_device": stat.st_dev, "image_inode": stat.st_ino}
            observed["unchanged"] = all(previous[key] == value for key, value in observed.items())
        except FileNotFoundError:
            observed = {"pid": previous["pid"], "unchanged": False, "absent": True}
        protected.append(observed)

    import socket
    open_ports = []
    for port in ports:
        with socket.socket() as probe:
            if probe.connect_ex(("127.0.0.1", port)) == 0:
                open_ports.append(port)
    stale_endpoints = [name for name in sorted(endpoints) if Path(name).exists()]
    bin_sha = digest(repo / ".bin/agentx")
    preserved = bin_sha == digest(evidence / "baseline/agentx")
    shared_product_changes = [name for name in shared_changes if not name.startswith("tests/") and not name.endswith(".md")]
    clean = not any([fixed_changes, shared_product_changes, executing, owned_images_running, open_ports, stale_endpoints])
    clean = clean and all(not row["exists"] for row in roots) and all(row["unchanged"] for row in protected) and preserved
    removed = []
    if clean:
        for name in ["prepared-fixture", "check-scope", "check-tmp"]:
            path = evidence / name
            if path.exists():
                shutil.rmtree(path)
            removed.append({"path": str(path), "removed": not path.exists()})
    report = {"passed": clean, "source": str(source), "artifact": str(artifact), "sha256": digest(artifact),
              "bytes": artifact.stat().st_size, "fixed_input_count": len(inputs), "fixed_input_changes": fixed_changes,
              "shared_input_changes": shared_changes, "shared_product_changes": shared_product_changes,
              "recorded_incarnations": len(records), "same_incarnation_executing": executing,
              "owned_product_images_executing": owned_images_running, "fixture_roots": roots,
              "ports": ports, "open_ports": open_ports, "stale_endpoints": stale_endpoints,
              "protected_processes": protected, "repository_bin_sha256": bin_sha,
              "repository_bin_still_baseline": preserved, "removed_preparation_directories": removed,
              "limits": "Linux local finite acceptance; named protected /proc metadata only, no user service operation or installed program change."}
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ["passed", "sha256", "recorded_incarnations", "shared_input_changes"]}))
    raise SystemExit(0 if clean else 1)


if __name__ == "__main__":
    main()
