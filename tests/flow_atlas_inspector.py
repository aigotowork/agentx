#!/usr/bin/env python3
"""Inspector builds keep author data inert and preserve working output on rejected input."""
import copy
import importlib.util
import json
from pathlib import Path
import re
import sys
spec=importlib.util.spec_from_file_location('inspector',Path(__file__).resolve().parents[1]/'tools/flow-atlas/inspector/build.py')
builder=importlib.util.module_from_spec(spec);spec.loader.exec_module(builder)
model=json.loads(Path(sys.argv[1]).read_text());original=copy.deepcopy(model)
model['label']='</script><script>window.injected=true</script> __INSPECTOR_SCRIPT__'
html=builder.render(model)
embedded=re.search(r'<script id="design-model" type="application/json">(.*?)</script>',html,re.S).group(1)
assert json.loads(embedded)['label']==model['label']
assert '<script>' not in embedded and html.count('<script')==2
assert builder.render(original)==builder.render(original)
try: builder.render({'format':'wrong'})
except ValueError: pass
else: raise AssertionError('Invalid artifact accepted')
print('PASS: inert author text, single-pass slots, deterministic inspector and format boundary')
