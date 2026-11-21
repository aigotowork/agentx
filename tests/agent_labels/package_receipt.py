#!/usr/bin/python3
"""Retain the exact labels candidate and Cargo-generated browser resources."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--name', required=True)
    parser.add_argument('--frontend', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    start = json.loads((args.evidence / f'package-{args.name}-start.json').read_text())['started_unix']
    outputs = [p for p in (root / 'target/release/build').glob('agentx-*/output') if p.stat().st_mtime >= start]
    assert len(outputs) == 1, outputs
    output = outputs[0]
    cohort = re.search(r'cargo::rustc-env=AGENTX_NATIVE_BUILD_ID=(\S+)', output.read_text()).group(1)
    target = args.evidence / f'{args.name}-embedded-assets'
    target.mkdir(exist_ok=False)
    files = {}
    for app in ['management', 'workbench', 'observability', 'agent-space', 'inspector']:
        source = output.parent / 'out' / f'{app}-web'
        shutil.copytree(source, target / f'{app}-web')
        for path in (target / f'{app}-web').rglob('*'):
            if path.is_file():
                files[str(path.relative_to(target))] = {'bytes': path.stat().st_size, 'sha256': sha(path)}
    frontend = json.loads((args.frontend / 'manifest.json').read_text())
    compared = {}
    for entry in frontend['files']:
        name = entry['file']
        expected, actual = sha(args.frontend / name), sha(target / 'management-web' / name)
        assert expected == actual, (name, expected, actual)
        compared[name] = actual
    candidate = args.candidate.resolve(strict=True)
    baseline = json.loads((args.evidence / 'build-environment.json').read_text())['protected_binaries']
    protected = {name: {'before': expected, 'after': sha(Path(name))} for name, expected in baseline.items()}
    assert all(v['before'] == v['after'] for v in protected.values()), protected
    source_receipt = json.loads((args.evidence / f'{args.name}-source-receipt.json').read_text())
    changed = [name for name, expected in source_receipt['files'].items() if not (root / name).is_file() or sha(root / name) != expected]
    # Documentation, scenario or fixture corrections do not alter this native
    # product build, but all drift remains visible rather than silently ignored.
    product_drift = [name for name in changed if (name.startswith('src/') or name.startswith('web/')) and '/tests/' not in name and not name.endswith('.md')]
    assert not product_drift, product_drift
    receipt = {'candidate': str(candidate), 'bytes': candidate.stat().st_size, 'sha256': sha(candidate),
               'version': subprocess.check_output([str(candidate), '--version'], text=True).strip(),
               'cohort': cohort, 'cargo_output': str(output), 'assets': files,
               'frontend_build': str(args.frontend), 'management_asset_match': compared,
               'source_receipt': f'{args.name}-source-receipt.json', 'changed_inputs_after_freeze': changed,
               'changed_product_inputs': product_drift, 'protected_binaries': protected,
               'limits': 'Exact local release package/embedded build resources; no live adoption or installed binary replacement.'}
    (args.evidence / f'package-{args.name}-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps({key: receipt[key] for key in ['candidate', 'bytes', 'sha256', 'version', 'cohort', 'changed_product_inputs', 'changed_inputs_after_freeze']}, indent=2))


if __name__ == '__main__':
    main()
