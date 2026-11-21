#!/usr/bin/env python3
"""Freeze the released native and both browser inputs with the existing runner."""

from pathlib import Path
import runpy


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[2]
    runner = runpy.run_path(str(root / "scripts/freeze-acceptance-source.py"))
    runner["DIRECTORIES"].append("web")
    # The existing freezer already captures root manifests/locks/build metadata
    # and this tests directory. No generated dist, node_modules or live state.
    raise SystemExit(runner["main"]())
