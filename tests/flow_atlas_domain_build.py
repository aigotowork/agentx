#!/usr/bin/env python3
"""Independent JSON-only embedding/receipt/preservation cases; no business source executes."""
from copy import deepcopy
import hashlib
from html.parser import HTMLParser
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
BUILDER = ROOT / 'tools/flow-atlas/workbench/build.py'


class Embeddings(HTMLParser):
    def __init__(self):
        super().__init__()
        self.scripts = []
        self.styles = 0

    def handle_starttag(self, tag, attrs):
        if tag == 'script':
            self.scripts.append(dict(attrs))
        if tag == 'style':
            self.styles += 1


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    model_path, evidence = map(Path, sys.argv[1:3])
    evidence.mkdir(parents=True, exist_ok=False)
    original = model_path.read_bytes()
    model = json.loads(original)
    spec = importlib.util.spec_from_file_location('domain_workbench_build', BUILDER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    untouched = deepcopy(model)
    rendered = module.render(model)
    assert model == untouched, 'Pure render cannot change exported checkpoints'
    cases = []

    def cli(source, target, expected, builder=BUILDER):
        result = subprocess.run([sys.executable, str(builder), str(source), '--output', str(target)], capture_output=True, text=True)
        cases.append({'input': str(source), 'output': str(target), 'code': result.returncode, 'stderr': result.stderr, 'stdout': result.stdout})
        assert result.returncode == expected, result.stderr
        return result

    output = evidence / 'candidate.html'
    cli(model_path, output, 0)
    assert output.read_text() == rendered
    receipt = json.loads(output.with_suffix('.receipt.json').read_text())
    assert receipt['sha256'] == digest(output)
    assert receipt['modelSha256'] == hashlib.sha256(original).hexdigest()
    assert receipt['bytes'] == output.stat().st_size
    for name, expected in receipt['inputs'].items():
        assert digest(BUILDER.parent / name) == expected, name
    clean_html, clean_receipt = output.read_bytes(), output.with_suffix('.receipt.json').read_bytes()

    invalid = evidence / 'invalid.json'
    for payload in ['{bad json', json.dumps({'format':'wrong'}), json.dumps({**model,'scenarios':[{'checkpoints':[]}]}), json.dumps({**model,'coverage':float('nan')})]:
        invalid.write_text(payload)
        cli(invalid, output, 1)
        assert output.read_bytes() == clean_html
        assert output.with_suffix('.receipt.json').read_bytes() == clean_receipt
    cli(model_path, model_path, 1)
    assert model_path.read_bytes() == original
    input_receipt = evidence / 'protected.receipt.json'
    input_receipt.write_bytes(original)
    cli(input_receipt, evidence / 'protected.html', 1)
    assert input_receipt.read_bytes() == original

    # Destructive protection probes run against disposable copies of the tool tree.
    copied = evidence / 'copied-tool'
    shutil.copytree(ROOT / 'tools/flow-atlas', copied, ignore=shutil.ignore_patterns('__pycache__', 'examples', 'node_modules'))
    copied_builder = copied / 'workbench/build.py'
    for protected in [copied_builder, copied / 'domain/contracts.ts', copied / 'workbench/index.template.html']:
        before = protected.read_bytes()
        cli(model_path, protected, 1, copied_builder)
        assert protected.read_bytes() == before

    hostile = deepcopy(model)
    hostile['label'] = '</script><script>globalThis.AUTHORED_EXECUTED=true</script> __WORKBENCH_SCRIPT__ <&>\u2028\u2029'
    hostile['definition']['objects'][0]['metadata'] = {
        'summary': '<img src=x onerror="globalThis.AUTHORED_EXECUTED=true"> __WORKBENCH_MODEL__',
        'documents': [
            {'id':'unsafe-js','label':'javascript URL','uri':'javascript:globalThis.AUTHORED_EXECUTED=true'},
            {'id':'unsafe-data','label':'data URL','uri':'data:text/html,<script>bad</script>'},
            {'id':'unsafe-file','label':'file URL','uri':'file:///etc/passwd'},
            {'id':'safe-http','label':'Safe docs','uri':'https://example.com/design'},
        ],
    }
    hostile_path = evidence / 'hostile.json'
    hostile_path.write_text(json.dumps(hostile, ensure_ascii=False))
    hostile_output = evidence / 'hostile.html'
    cli(hostile_path, hostile_output, 0)
    parser = Embeddings()
    parser.feed(hostile_output.read_text())
    assert len(parser.scripts) == 2 and parser.styles == 1
    assert [s.get('type') for s in parser.scripts] == ['application/json', None]
    assert '</script><script>globalThis.AUTHORED_EXECUTED=true' not in hostile_output.read_text()
    assert model_path.read_bytes() == original, 'Builder/receipt never modify their source'
    report = {'result':'passed','cases':cases,'candidate':str(output),'sha256':digest(output),'hostile':str(hostile_output),'sourcePreserved':True}
    (evidence / 'result.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
