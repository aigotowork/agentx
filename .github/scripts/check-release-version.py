"""Check that the native release and its versioned projections agree."""

import json
from pathlib import Path
import re
import sys
import tomllib


def main():
    root = Path(__file__).resolve().parents[2]
    cargo = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["package"]
    version = cargo["version"]
    if not re.fullmatch(r"(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*)){2}", version):
        sys.exit(f"Expected a stable Cargo version X.Y.Z, got {version!r}")

    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    cli = next((entry for entry in lock["package"] if entry["name"] == cargo["name"] and "source" not in entry), {})
    changelog = next((line for line in (root / "CHANGELOG.md").read_text(encoding="utf-8").splitlines() if line.startswith("## ")), "")
    heading = re.fullmatch(r"## \[(.+)\]", changelog)
    sdk = root / "packages/agentx-client"
    spec = json.loads((sdk / "cloud-admin.openapi.json").read_text(encoding="utf-8"))
    wire = (sdk / "src/generated/cloud-admin/wire-schema.gen.ts").read_text(encoding="utf-8")
    wire = wire.split("export const wireDocument = ", 1)[1].strip().removesuffix(" as const;")

    versions = {
        "Cargo.lock root package": cli.get("version"),
        "first CHANGELOG.md version": heading[1] if heading else None,
        "Cloud Admin OpenAPI": spec["info"]["version"],
        "Cloud Admin generated wire document": json.loads(wire)["info"]["version"],
    }
    if len(sys.argv) > 1:
        versions["release tag"] = sys.argv[1].removeprefix("v") if sys.argv[1].startswith("v") else None
    mismatches = [f"{name}: expected {version}, found {value!r}" for name, value in versions.items() if value != version]
    if mismatches:
        sys.exit("\n".join(mismatches))
    print(f"AgentX {version}: release versions agree")


if __name__ == "__main__":
    main()
