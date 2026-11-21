#!/usr/bin/env python3
"""Embed an already exported InspectorArtifact without importing project code."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
INPUTS = ('index.template.html', 'inspector.css', 'inspector.js', '../spatial/viewport.css', '../spatial/viewport.js')


def render(model: dict) -> str:
    """Render exported design data; structural design validity remains compiler-owned."""
    if not isinstance(model, dict) or model.get('format') != 'flow-atlas/inspector-1':
        raise ValueError('Expected flow-atlas/inspector-1 artifact')
    for name in ('domains','modules','objects','schemas','operations','processes','diagnostics','coverage'):
        if not isinstance(model.get(name),list):
            raise ValueError(f'{name}: expected exported array')
    encoded = json.dumps(model, ensure_ascii=False, allow_nan=False, separators=(',',':'))
    for literal, escape in (('<','\\u003c'),('>','\\u003e'),('&','\\u0026'),('\u2028','\\u2028'),('\u2029','\\u2029')):
        encoded = encoded.replace(literal,escape)
    values = {
        '__INSPECTOR_MODEL__': encoded,
        '__INSPECTOR_SCRIPT__': re.sub(r'</script', r'<\\/script', (HERE/'../spatial/viewport.js').read_text() + '\n;\n' + (HERE/'inspector.js').read_text(),flags=re.I),
        '__INSPECTOR_STYLES__': re.sub(r'</style', r'<\\/style', (HERE/'inspector.css').read_text() + '\n' + (HERE/'../spatial/viewport.css').read_text(),flags=re.I),
    }
    template = (HERE/'index.template.html').read_text()
    if any(template.count(token)!=1 for token in values):
        raise ValueError('Expected exactly one of each Inspector slot')
    return re.sub(r'__INSPECTOR_(?:MODEL|SCRIPT|STYLES)__',lambda match:values[match.group()],template)


def main() -> int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('model',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    try:
        model_path=args.model.resolve(); output=args.output.resolve()
        receipt_path=output.with_suffix('.receipt.json')
        protected={model_path,Path(__file__).resolve(),*((HERE/name).resolve() for name in INPUTS)}
        if output in protected or receipt_path in protected or output==receipt_path:
            raise ValueError('Output/receipt must not overwrite input or tool sources')
        raw=model_path.read_bytes();html=render(json.loads(raw))
        artifact=html.encode(); receipt={'format':'flow-atlas/inspector-1','model':str(model_path),'modelSha256':hashlib.sha256(raw).hexdigest(),'sha256':hashlib.sha256(artifact).hexdigest(),'bytes':len(artifact),'inputs':{name:hashlib.sha256((HERE/name).read_bytes()).hexdigest() for name in (*INPUTS,'build.py')}}
        output.parent.mkdir(parents=True,exist_ok=True)
        output.write_bytes(artifact);receipt_path.write_text(json.dumps(receipt,indent=2)+'\n')
        print(json.dumps({'output':str(output),**receipt}));return 0
    except (ValueError,OSError,TypeError) as error:
        print(f'flow-atlas-inspector: {error}',file=sys.stderr);return 1


if __name__=='__main__':
    raise SystemExit(main())
