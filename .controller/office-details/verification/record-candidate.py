"""Record one finished fixed office candidate; does not build, accept or deploy it."""
from pathlib import Path
import argparse
import hashlib
import json

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('revision')
args = parser.parse_args()
if not args.revision.startswith('r') or not args.revision[1:].isdigit():
    parser.error('revision must be r followed by digits')
root = Path(__file__).resolve().parents[3]
v = Path(__file__).resolve().parent
revision = args.revision
old = json.loads((v / 'source-inputs-r3.json').read_text())
files = [{'path': item['path'], 'sha256': hashlib.sha256((root / item['path']).read_bytes()).hexdigest()} for item in old['files']]
manifest = {'description': f'Source identity captured after {revision} packaging; preserved existing dirty inputs', 'manifest_sha256': hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest(), 'files': files}
(v / f'source-inputs-{revision}.json').write_text(json.dumps(manifest, indent=2) + '\n')
p = root / '.bin' / f'agentx-office-details-{revision}'
receipt = json.loads((v / 'package-r3-receipt.json').read_text())
receipt.update(path=str(p), sha256=hashlib.sha256(p.read_bytes()).hexdigest(), bytes=p.stat().st_size, command=['just', 'package', f'.bin/{p.name}'], log=f'package-{revision}.log', source_manifest=f'source-inputs-{revision}.json')
(v / f'package-{revision}-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
plan = json.loads((v / 'rollout-plan.json').read_text())
plan['candidate'].update(path=str(p), sha256=receipt['sha256'])
plan['method']['command'][0] = str(p)
(v / 'rollout-plan.json').write_text(json.dumps(plan, indent=2) + '\n')
print(json.dumps(receipt))
