#!/usr/bin/env python3
"""Build a separate offline experience proposal from inert workbench JSON and maintained UI sources."""
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
TOOL = HERE.parent.parent
STYLES = ('../../workbench/workbench.css', '../../spatial/viewport.css', 'styles/tokens.css', 'layouts/workspace.css', 'styles/components.css', '../../workbench/linked.css')
SCRIPTS = ('../../spatial/viewport.js', '../../viewer/layout.js', '../../workbench/dom.js', '../../workbench/views.js', '../../workbench/collaboration.js', '../../workbench/details.js', '../../workbench/player.js', '../../workbench/layout.js', 'projection.js', '../../workbench/linked.js', '../../workbench/app.js', 'enhancement.js')
INPUTS = ('index.template.html', *STYLES, *SCRIPTS, 'build.py')


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def render(model: dict) -> tuple[str, dict[str, str]]:
    """Validate the portable envelope; never import or run user code or reduce checkpoints."""
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
        raise ValueError('scenarios: expected nonempty authoritative checkpoints')
    sources = {name: (HERE / name).read_bytes() for name in INPUTS}
    encoded = json.dumps(model, ensure_ascii=False, allow_nan=False, separators=(',', ':'))
    for literal, escape in (('<', '\\u003c'), ('>', '\\u003e'), ('&', '\\u0026'), ('\u2028', '\\u2028'), ('\u2029', '\\u2029')):
        encoded = encoded.replace(literal, escape)
    scripts = '\n;\n'.join(sources[name].decode('utf-8') for name in SCRIPTS)
    styles = '\n'.join(sources[name].decode('utf-8') for name in STYLES)
    values = {
        '__EXPERIENCE_MODEL__': encoded,
        '__EXPERIENCE_SCRIPT__': re.sub(r'</script', lambda m: '<\\/' + m.group()[2:], scripts, flags=re.I),
        '__EXPERIENCE_STYLES__': re.sub(r'</style', lambda m: '<\\/' + m.group()[2:], styles, flags=re.I),
    }
    template = sources['index.template.html'].decode('utf-8')
    if any(template.count(token) != 1 for token in values):
        raise ValueError('Expected exactly one occurrence of each experience slot')
    html = re.sub(r'__EXPERIENCE_(?:MODEL|SCRIPT|STYLES)__', lambda m: values[m.group()], template)
    return html, {name: digest(data) for name, data in sources.items()}


def write(path: Path, data: bytes) -> None:
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=f'.{path.name}.', delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(data)
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('model', type=Path, help='Already exported portable workbench JSON')
    parser.add_argument('--output', type=Path, required=True, help='New, separate .html artifact')
    args = parser.parse_args()
    try:
        model_path, output = args.model.resolve(), args.output.resolve()
        receipt_path = output.with_suffix('.receipt.json')
        if output.suffix != '.html' or output == receipt_path:
            raise ValueError('Output must have .html suffix')
        protected = {model_path, *((HERE / name).resolve() for name in INPUTS)}
        # Proposals must never overwrite accepted examples or any maintained tool source.
        maintained_suffixes = {'.py', '.js', '.ts', '.css', '.md', '.json'}
        protected.update(path.resolve() for path in TOOL.rglob('*') if path.is_file() and (path.suffix in maintained_suffixes or path.name.endswith('.template.html') or path.parent == TOOL / 'examples'))
        if output in protected or receipt_path in protected:
            raise ValueError('Output/receipt must not overwrite input, accepted examples or maintained tool sources')
        raw = model_path.read_bytes()
        html, inputs = render(json.loads(raw.decode('utf-8')))
        data = html.encode('utf-8')
        receipt = {'format': 'flow-atlas/experience-proposal-1', 'output': str(output), 'sha256': digest(data), 'bytes': len(data), 'model': str(model_path), 'modelSha256': digest(raw), 'inputs': inputs, 'limits': 'Experience proposal; original artifact/checkpoints; illustrative animation; no native/source execution; independent browser evidence is separate.'}
        output.parent.mkdir(parents=True, exist_ok=True)
        write(output, data)
        write(receipt_path, (json.dumps(receipt, ensure_ascii=False, indent=2) + '\n').encode('utf-8'))
        print(json.dumps({'output': str(output), 'receipt': str(receipt_path), 'sha256': receipt['sha256'], 'bytes': len(data)}))
        return 0
    except (ValueError, TypeError, OSError, UnicodeError, OverflowError, RecursionError) as error:
        print(f'flow-atlas-experience: {error}', file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130


if __name__ == '__main__':
    sys.exit(main())
