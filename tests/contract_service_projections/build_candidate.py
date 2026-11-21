#!/usr/bin/env python3
"""Coordinate one native build into an explicit immutable candidate, never a live install."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=["debug", "release"], required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    candidate = args.candidate.absolute()
    if not candidate.parent.is_dir() or candidate.exists():
        raise ValueError("candidate needs an existing owned parent and a new explicit filename")
    args.evidence.mkdir(parents=True, exist_ok=False)
    paths = sorted(path for path in (repo / "src").rglob("*") if path.is_file())
    paths += [repo / name for name in ["Cargo.toml", "Cargo.lock", "build.rs", "justfile", "package.json", "package-lock.json"]]
    protected = {str(path): digest(path) for path in [repo / ".bin/agentx", Path.home() / ".local/bin/agentx"] if path.is_file()}
    command = ["cargo", "build", "--locked", "--bin", "agentx"] if args.profile == "debug" else ["just", "package", str(candidate)]
    manifest = {"cwd": str(repo), "candidate": str(candidate), "profile": args.profile, "command": command,
                "source_sha256": {str(path.relative_to(repo)): digest(path) for path in paths},
                "protected_sha256": protected, "status": "building"}
    record = args.evidence / "build-manifest.json"
    record.write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"command": command, "candidate": str(candidate)}), flush=True)
    with (args.evidence / "build.log").open("x") as log:
        result = subprocess.run(command, cwd=repo, stdout=log, stderr=subprocess.STDOUT)
    if result.returncode == 0 and args.profile == "debug":
        with (repo / "target/debug/agentx").open("rb") as source, candidate.open("xb") as destination:
            shutil.copyfileobj(source, destination)
        candidate.chmod(0o755)
    manifest.update(build_exit_code=result.returncode, status="built" if result.returncode == 0 else "failed",
                    protected_unchanged=all(Path(path).is_file() and digest(Path(path)) == value for path, value in protected.items()),
                    changed_sources_during_build=[path for path, value in manifest["source_sha256"].items() if digest(repo / path) != value])
    if candidate.is_file():
        manifest["candidate_sha256"] = digest(candidate)
    record.write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({key: manifest[key] for key in ["status", "candidate", "protected_unchanged", "changed_sources_during_build"]}), flush=True)
    if not manifest["protected_unchanged"]:
        raise RuntimeError("protected binary digest changed during coordinated build")
    raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
