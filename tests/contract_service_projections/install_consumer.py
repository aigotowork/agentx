#!/usr/bin/env python3
"""Install an immutable tarball into an owned external consumer; never install globally."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tarball", type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--project", type=Path, help="reuse the existing owned external consumer for a complete batch")
    args = parser.parse_args()
    tarball = args.tarball.resolve(strict=True)
    digest = hashlib.file_digest(tarball.open("rb"), "sha256").hexdigest()
    if digest != args.sha256:
        raise ValueError("immutable tarball digest differs from supplied handoff")
    args.evidence.mkdir(parents=True, exist_ok=False)
    if args.project:
        project = args.project.resolve(strict=True)
        package = json.loads((project / "package.json").read_text())
        if package.get("name") != "agentx-external-acceptance" or package.get("private") is not True:
            raise ValueError("reuse requires the already owned external acceptance project")
        root = project.parent
        lock = project / "package-lock.json"
        if lock.is_file():
            shutil.copyfile(lock, args.evidence / "previous-package-lock.json")
    else:
        root = Path(tempfile.mkdtemp(prefix="agentx-contract-package-"))
        project = root / "consumer"
        project.mkdir()
        (root / "home").mkdir()
        (root / "empty-npmrc").touch()
        (project / "package.json").write_text(json.dumps({"name": "agentx-external-acceptance", "private": True, "type": "module"}))
    shutil.copyfile(Path(__file__).with_name("sdk_driver.mjs"), project / "sdk-driver.mjs")
    environment = {**os.environ, "HOME": str(root / "home"), "npm_config_cache": str(root / "npm-cache"),
                   "NPM_CONFIG_USERCONFIG": str(root / "empty-npmrc")}
    manifest = {"root": str(root), "consumer_project": str(project), "tarball": str(tarball),
                "tarball_sha256": digest, "command": ["npm", "install", "--ignore-scripts", "--no-audit", "--no-fund", str(tarball)],
                "status": "installing", "reused_project": bool(args.project)}
    manifest_path = args.evidence / "install-manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    with (args.evidence / "install.log").open("x") as log:
        result = subprocess.run(manifest["command"], cwd=project, env=environment, stdout=log, stderr=subprocess.STDOUT)
    manifest.update(install_exit_code=result.returncode, status="installed" if result.returncode == 0 else "failed")
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest))
    raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
