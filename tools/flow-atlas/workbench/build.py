#!/usr/bin/env python3
"""Embed a portable domain artifact as one offline workbench; never import user code."""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile

HERE = Path(__file__).resolve().parent
STYLES = ('workbench.css', '../spatial/viewport.css', 'linked.css')
SCRIPTS = ('../spatial/viewport.js', '../viewer/layout.js', 'dom.js', 'views.js', 'collaboration.js', 'details.js', 'player.js', 'layout.js', 'linked.js', 'app.js')
INPUTS = ('index.template.html', *STYLES, *SCRIPTS, 'build.py')


def _digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _render(model: dict) -> tuple[str, dict[str, str]]:
    if not isinstance(model, dict) or model.get('format') != 'flow-atlas/workbench-1':
        raise ValueError('Expected flow-atlas/workbench-1 artifact')
    definition = model.get('definition')
    if not isinstance(definition, dict):
        raise ValueError('definition: expected object')
    for name in ('sources', 'contexts', 'modules', 'values', 'objects', 'rules', 'operations', 'relationships', 'processes', 'schemas'):
        if not isinstance(definition.get(name), list):
            raise ValueError(f'definition.{name}: expected array')
    for name in ('scenarios', 'diagnostics', 'coverage'):
        if not isinstance(model.get(name), list):
            raise ValueError(f'{name}: expected array')
    if any(not isinstance(s, dict) or not isinstance(s.get('checkpoints'), list) or not s['checkpoints'] for s in model['scenarios']):
        raise ValueError('scenarios: expected authoritative nonempty checkpoints')
    inputs = {name: (HERE / name).read_bytes() for name in INPUTS}
    template = inputs['index.template.html'].decode('utf-8')
    encoded = json.dumps(model, ensure_ascii=False, allow_nan=False, separators=(',', ':'))
    for literal, escape in (('<', '\\u003c'), ('>', '\\u003e'), ('&', '\\u0026'), ('\u2028', '\\u2028'), ('\u2029', '\\u2029')):
        encoded = encoded.replace(literal, escape)
    scripts = '\n;\n'.join(inputs[name].decode('utf-8') for name in SCRIPTS)
    styles = '\n'.join(inputs[name].decode('utf-8') for name in STYLES)
    values = {
        '__WORKBENCH_MODEL__': encoded,
        '__WORKBENCH_SCRIPT__': re.sub(r'</script', lambda match: '<\\/' + match.group()[2:], scripts, flags=re.I),
        '__WORKBENCH_STYLES__': re.sub(r'</style', lambda match: '<\\/' + match.group()[2:], styles, flags=re.I),
    }
    if any(template.count(token) != 1 for token in values):
        raise ValueError('Expected exactly one occurrence of each workbench template slot')
    return re.sub(r'__WORKBENCH_(?:MODEL|SCRIPT|STYLES)__', lambda match: values[match.group()], template), {name: _digest(data) for name, data in inputs.items()}


def render(model: dict) -> str:
    """Embed JSON as inert data; semantic validity and checkpoints remain Core-owned."""
    return _render(model)[0]


def _write(path: Path, data: bytes) -> None:
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=f'.{path.name}.', delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(data)
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('model', type=Path, help='Already exported workbench JSON')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        model_path = args.model.resolve()
        output = args.output.resolve()
        receipt_path = output.with_suffix('.receipt.json')
        protected = {model_path, *((HERE / name).resolve() for name in INPUTS)}
        # Protect all maintained Flow Atlas sources, not just embedded leaves.
        source_root = HERE.parent.resolve()
        maintained = set(source_root.rglob('*.py')) | set(source_root.rglob('*.js')) | set(source_root.rglob('*.ts')) | set(source_root.rglob('*.css')) | set(source_root.rglob('*.md')) | set(source_root.rglob('*.template.html'))
        protected.update(path.resolve() for path in maintained)
        if output == receipt_path or output in protected or receipt_path in protected:
            raise ValueError('Output/receipt must not overwrite input or tool sources')
        raw = model_path.read_bytes()
        html, inputs = _render(json.loads(raw.decode('utf-8')))
        artifact = html.encode('utf-8')
        receipt = {'format': 'flow-atlas/workbench-1', 'output': str(output), 'sha256': _digest(artifact), 'bytes': len(artifact), 'model': str(model_path), 'modelSha256': _digest(raw), 'inputs': inputs, 'limits': 'Source-first exported design and authoritative authored/observed checkpoints; illustrative playback; no native execution.'}
        receipt_bytes = (json.dumps(receipt, ensure_ascii=False, indent=2) + '\n').encode('utf-8')
        output.parent.mkdir(parents=True, exist_ok=True)
        _write(output, artifact)
        _write(receipt_path, receipt_bytes)
        print(json.dumps({'output': str(output), 'receipt': str(receipt_path), 'sha256': receipt['sha256'], 'bytes': receipt['bytes']}))
        return 0
    except (ValueError, TypeError, OSError, UnicodeError, OverflowError, RecursionError) as error:
        print(f'flow-atlas-workbench: {error}', file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130


if __name__ == '__main__':
    sys.exit(main())
