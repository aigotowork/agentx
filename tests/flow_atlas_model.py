#!/usr/bin/env python3
"""Consumer contract cases for the standalone authored-flow tool; no product I/O."""
import copy
import importlib.util
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / 'tools/flow-atlas'
spec = importlib.util.spec_from_file_location('flow_model_cases', TOOL / 'model.py')
contract = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contract)
example = json.loads((TOOL / 'examples/request.json').read_text())
normalized = contract.validate(example)
assert normalized is not example
assert example == json.loads((TOOL / 'examples/request.json').read_text()), 'Validation cannot edit the author model'
assert normalized['defaults']['modeId'] != 'appserver'
assert all(item['id'] not in {'connection', 'host', 'provider', 'turn', 'inbox'} for item in normalized['factDefinitions'])

mutations = {
    'duplicate node': lambda m: m['nodes'].append(copy.deepcopy(m['nodes'][0])),
    'dangling endpoint': lambda m: m['edges'][0].update(to='missing-node'),
    'unknown fact patch': lambda m: m['scenarios'][0]['steps'][0]['patch'].setdefault('facts', {}).update(undeclared='value'),
    'incomplete initial': lambda m: m['scenarios'][0]['initial']['nodes'].pop(next(iter(m['scenarios'][0]['initial']['nodes']))),
    'prototype key': lambda m: m['nodes'][0].update(id='__proto__'),
    'unsafe identifier': lambda m: m['nodes'][0].update(id='bad\"selector'),
}
for name, mutate in mutations.items():
    candidate = copy.deepcopy(example)
    mutate(candidate)
    try:
        contract.validate(candidate)
    except ValueError as error:
        assert str(error), f'{name}: contextual error required'
    else:
        raise AssertionError(f'Invalid input accepted: {name}')

transport = next((si, ti, bi) for si, s in enumerate(example['scenarios']) for ti, t in enumerate(s['steps']) for bi, b in enumerate(t['beats']) if b['kind'] == 'transfer')
for name, field, value in [('direction', 'direction', 'sideways'), ('undeclared edge', 'edgeId', 'missing-edge')]:
    candidate = copy.deepcopy(example)
    si, ti, bi = transport
    candidate['scenarios'][si]['steps'][ti]['beats'][bi][field] = value
    try:
        contract.validate(candidate)
    except ValueError:
        pass
    else:
        raise AssertionError(f'Invalid transport accepted: {name}')

# Rendering must keep author text as data even when it resembles HTML or a slot.
spec = importlib.util.spec_from_file_location('flow_builder_cases', TOOL / 'build.py')
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)
text_model = copy.deepcopy(example)
text_model['title'] = '</script><script>window.injected=true</script> __ATLAS_SCRIPT__'
html = builder.render(text_model)
embedded = re.search(r'<script id="atlas-model" type="application/json">(.*?)</script>', html, re.S).group(1)
assert json.loads(embedded)['title'] == text_model['title']
assert '<script>' not in embedded and html.count('<script') == 2

with tempfile.TemporaryDirectory(prefix='flow-atlas-contract-') as directory:
    work = Path(directory)
    output = work / 'example.html'
    subprocess.run([sys.executable, str(TOOL / 'build.py'), str(TOOL / 'examples/request.json'), '--output', str(output)], cwd=work, check=True, capture_output=True, text=True)
    html = output.read_text()
    assert 'FlowAtlas' in html and '__ATLAS_' not in html
    invalid = copy.deepcopy(example)
    invalid['defaults']['modeId'] = 'missing-mode'
    bad = work / 'bad.json'
    bad.write_text(json.dumps(invalid))
    before = output.read_bytes()
    result = subprocess.run([sys.executable, str(TOOL / 'build.py'), str(bad), '--output', str(output)], cwd=work, capture_output=True, text=True)
    assert result.returncode != 0
    assert output.read_bytes() == before, 'Rejected input cannot overwrite a working artifact'

print('PASS: unrelated model, normalization isolation, rejected references/beats/IDs, unrelated-cwd offline build, failure preserves output')
