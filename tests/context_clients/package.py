#!/usr/bin/env python3
"""Bind exact ZIP content to extracted artifact and optional served download."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
from urllib.request import urlopen
from zipfile import ZipFile


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("zip")
    parser.add_argument("unpacked")
    parser.add_argument("--download")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    archive, unpacked = Path(args.zip).resolve(strict=True), Path(args.unpacked).resolve(strict=True)
    rows = []
    with ZipFile(archive) as package:
        names = package.namelist()
        assert "manifest.json" in names and len(names) == len(set(names))
        assert 0 < len(names) < 100
        for item in package.infolist():
            path = PurePosixPath(item.filename)
            assert not path.is_absolute() and ".." not in path.parts
            assert not any(part.startswith(".") or part in {"node_modules", "target", "profiles", "user-data", "__pycache__"} for part in path.parts)
            assert path.suffix.lower() in {".json", ".js", ".css", ".html", ".md", ".png", ".svg"}
            assert item.file_size < 2 * 1024 * 1024
            data = package.read(item)
            assert data == (unpacked / item.filename).read_bytes(), item.filename
            rows.append({"name": item.filename, "size": len(data), "sha256": sha(data)})
        manifest = json.loads(package.read("manifest.json"))
        assert manifest["manifest_version"] == 3
    actual = {str(path.relative_to(unpacked)) for path in unpacked.rglob("*") if path.is_file()}
    assert actual == set(names), "unpacked artifact contains extra/missing files"
    result = {"zip": str(archive), "sha256": sha(archive.read_bytes()), "bytes": archive.stat().st_size,
              "unpacked": str(unpacked), "manifest": manifest, "files": rows,
              "secret_check": "Finite source asset inventory and forbidden user/cache/config paths; not a general secret scanner"}
    if args.download:
        with urlopen(args.download, timeout=20) as response:
            data = response.read(5 * 1024 * 1024)
            result["download"] = {"url": args.download, "status": response.status, "headers": dict(response.headers), "sha256": sha(data), "bytes": len(data)}
            assert response.status == 200 and data == archive.read_bytes()
    Path(args.output).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: result[k] for k in ["sha256", "bytes"]}))


if __name__ == "__main__":
    main()
