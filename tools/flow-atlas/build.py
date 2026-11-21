#!/usr/bin/env python3
"""Build one offline atlas from a model and the single shared viewer source."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import tempfile
from typing import Any

HERE = Path(__file__).resolve().parent
VIEWER_INPUTS = ("viewer/index.template.html", "viewer/atlas.css", "viewer/layouts/workspace.css", "spatial/viewport.css", "spatial/viewport.js", "core.js", "viewer/ports.js", "viewer/layout.js", "viewer/canvas.js", "viewer/views.js", "viewer/layout-ui.js", "viewer/atlas.js")
TOOL_INPUTS = (*VIEWER_INPUTS, "model.py", "build.py")

# Load the sibling contract by path: importlib consumers need no sys.path edits
# and cannot accidentally select an unrelated caller module named `model`.
_spec = importlib.util.spec_from_file_location("flow_atlas_model", HERE / "model.py")
_model = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_model)
validate = _model.validate


def _digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _render(model: dict[str, Any]) -> tuple[str, dict[str, str]]:
    normalized = validate(model)
    inputs = {name: (HERE / name).read_bytes() for name in TOOL_INPUTS}
    template = inputs["viewer/index.template.html"].decode("utf-8")
    css = "\n".join(inputs[name].decode("utf-8") for name in ("viewer/atlas.css", "viewer/layouts/workspace.css", "spatial/viewport.css"))
    scripts = "\n;\n".join(inputs[name].decode("utf-8") for name in ("spatial/viewport.js", "core.js", "viewer/ports.js", "viewer/layout.js", "viewer/canvas.js", "viewer/views.js", "viewer/layout-ui.js", "viewer/atlas.js"))
    encoded_model = json.dumps(normalized, ensure_ascii=False, allow_nan=False, separators=(",", ":"))
    for literal, escape in (("<", "\\u003c"), (">", "\\u003e"), ("&", "\\u0026"), ("\u2028", "\\u2028"), ("\u2029", "\\u2029")):
        encoded_model = encoded_model.replace(literal, escape)
    values = {
        "__ATLAS_STYLES__": re.sub(r"</style", lambda match: "<\\/" + match.group()[2:], css, flags=re.IGNORECASE),
        "__ATLAS_MODEL__": encoded_model,
        "__ATLAS_SCRIPT__": re.sub(r"</script", lambda match: "<\\/" + match.group()[2:], scripts, flags=re.IGNORECASE),
    }
    for token in values:
        if template.count(token) != 1:
            raise ValueError(f"viewer/index.template.html: expected exactly one {token} token")
    # One substitution pass prevents model text that resembles a slot from being
    # interpreted as template syntax after it has already been safely encoded.
    html = re.sub(r"__ATLAS_(?:STYLES|MODEL|SCRIPT)__", lambda match: values[match.group()], template)
    return html, {name: _digest(data) for name, data in inputs.items()}


def render(model: dict[str, Any]) -> str:
    """Validate/normalize model and return standalone HTML without writing files.

    Embed core, ports, layout, canvas, views, layout-ui and atlas scripts in that order, using the
    viewer's three existing __ATLAS_*__ slots. Model JSON escapes HTML syntax.
    Resource paths resolve relative to this file, never the caller's cwd.
    """
    return _render(model)[0]


def _write(path: Path, data: bytes) -> None:
    """Replace one artifact atomically, preserving the previous file on failure."""
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=f".{path.name}.", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(data)
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main(argv: list[str] | None = None) -> int:
    """CLI: build MODEL --output OUTPUT; stdout is one JSON artifact receipt."""
    parser = argparse.ArgumentParser(description="Build a finite declarative flow model into one offline HTML file.")
    parser.add_argument("model", metavar="MODEL", type=Path, help="UTF-8 schemaVersion 1 JSON model")
    parser.add_argument("--output", required=True, metavar="OUTPUT", type=Path, help="HTML destination; sibling .receipt.json records exact inputs")
    args = parser.parse_args(argv)
    try:
        model_path = args.model.resolve()
        output = args.output.resolve()
        receipt_path = output.with_suffix(".receipt.json")
        protected = {model_path, *((HERE / name).resolve() for name in TOOL_INPUTS)}
        if output in protected or receipt_path in protected:
            raise ValueError("output/receipt must not overwrite the model or tool source")
        raw_model = model_path.read_bytes()
        html, input_digests = _render(json.loads(raw_model.decode("utf-8")))
        artifact = html.encode("utf-8")
        receipt = {
            "output": str(output),
            "sha256": _digest(artifact),
            "bytes": len(artifact),
            "model": str(model_path),
            "modelSha256": _digest(raw_model),
            "inputs": input_digests,
            "limits": "Finite authored explanatory trace; illustrative time; no live data or execution.",
        }
        receipt_bytes = (json.dumps(receipt, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
        # Parse, validate, load resources, check slots and prepare both artifacts
        # before creating directories or replacing any previous output.
        output.parent.mkdir(parents=True, exist_ok=True)
        _write(output, artifact)
        _write(receipt_path, receipt_bytes)
        print(json.dumps({"output": str(output), "receipt": str(receipt_path), "sha256": receipt["sha256"], "bytes": receipt["bytes"]}, ensure_ascii=False))
        return 0
    except (ValueError, OSError, UnicodeError, OverflowError, RecursionError) as error:
        print(f"flow-atlas: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130


if __name__ == "__main__":
    sys.exit(main())
