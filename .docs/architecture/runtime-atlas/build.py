#!/usr/bin/env python3
"""Apply AgentX source provenance checks, then use the shared offline flow viewer."""
from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TOOL = ROOT / "tools/flow-atlas"


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


contract = load("flow_atlas_contract", TOOL / "model.py")
builder = load("flow_atlas_builder", TOOL / "build.py")


def validate(model):
    """Keep runtime source authority local; generic tools do not read repository data."""
    normalized = contract.validate(model)
    def source_path(value):
        path = (ROOT / value).resolve()
        if not path.is_relative_to(ROOT) or not path.is_file():
            raise ValueError(f"Invalid repository source {value}")
        if not value.startswith(("src/", "web/", ".docs/", ".controller/")):
            raise ValueError(f"Source outside declared documentation/code roots: {value}")
    for source in normalized["sources"]:
        source_path(source["path"])
    for node in normalized["nodes"]:
        for module in node["modules"]:
            source_path(module["path"])
    return normalized


def build():
    model = validate(json.loads((HERE / "model.json").read_text()))
    html = builder.render(model)
    output = HERE / "index.html"
    output.write_text(html)
    shared = [TOOL / name for name in builder.TOOL_INPUTS]
    inputs = [HERE / "model.json", HERE / "build.py"] + shared
    receipt = {
        "output": "index.html",
        "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
        "bytes": output.stat().st_size,
        "inputs": {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in inputs},
        "provenance": {source["path"]: hashlib.sha256((ROOT / source["path"]).read_bytes()).hexdigest() for source in model["sources"]},
        "limits": "Source-grounded explanatory scenarios; no live processes, runtime APIs or Provider execution.",
    }
    (HERE / "build-receipt.json").write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"output": str(output), "sha256": receipt["sha256"], "bytes": receipt["bytes"]}))


if __name__ == "__main__":
    build()
