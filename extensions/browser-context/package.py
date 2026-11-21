#!/usr/bin/env python3
"""Build deterministic MV3 payloads; never replace different bytes at a fixed version."""
import hashlib
import io
import json
from pathlib import Path
import zipfile

SOURCE = Path(__file__).resolve().parent
ROOT = SOURCE.parents[1]
FILES = ['AGENTS.md', 'README.md', 'design.md', 'manifest.json', 'background.js', 'panel.html', 'panel.css', 'panel.js', 'bridge.js', 'capture.js', 'service.js']

def build():
    manifest = json.loads((SOURCE / 'manifest.json').read_text())
    name = 'agentx-browser-context-' + manifest['version']
    output = ROOT / '.artifacts/context-clients'
    unpacked, archive = output / name, output / (name + '.zip')
    payload = {filename:(SOURCE / filename).read_bytes() for filename in sorted(FILES)}
    memory = io.BytesIO()
    with zipfile.ZipFile(memory, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as package:
        for filename, data in payload.items():
            info = zipfile.ZipInfo(filename, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            package.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    encoded = memory.getvalue()
    if archive.exists() and archive.read_bytes() != encoded:
        raise SystemExit('Fixed version already has different ZIP bytes; increment manifest version before packaging.')
    if unpacked.exists():
        actual = {str(path.relative_to(unpacked)):path.read_bytes() for path in unpacked.rglob('*') if path.is_file()}
        if actual != payload:
            raise SystemExit('Existing unpacked artifact differs; preserve evidence and choose a new version/output.')
    output.mkdir(parents=True, exist_ok=True)
    unpacked.mkdir(exist_ok=True)
    for filename, data in payload.items():
        if not (unpacked / filename).exists():
            (unpacked / filename).write_bytes(data)
    if not archive.exists():
        archive.write_bytes(encoded)
    receipt = {'version':manifest['version'], 'zip':str(archive.relative_to(ROOT)), 'unpacked':str(unpacked.relative_to(ROOT)), 'sha256':hashlib.sha256(encoded).hexdigest(), 'bytes':len(encoded), 'files':{filename:hashlib.sha256(data).hexdigest() for filename,data in payload.items()}}
    (output / (name + '.receipt.json')).write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(receipt,ensure_ascii=False))

if __name__ == '__main__':
    build()
