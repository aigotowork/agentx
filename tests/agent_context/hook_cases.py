#!/usr/bin/env python3
"""Candidate hook integration in actual launcher-created disposable identities."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from startup_usability.fixture import Fixture

assert '--handoff' in sys.argv, 'Explicit candidate handoff required'
artifact = Path('/tmp/agentx-context-candidate')
root = Path(tempfile.mkdtemp(prefix='agentx-startup-fixture-context-hooks-'))
evidence = root / 'evidence'; evidence.mkdir()
fixture = Fixture(root,evidence)
binary = root/'bin'; binary.mkdir()
for tool in ['codex','claude']:
    shutil.copy2(Path(__file__).with_name('hook_recipient.py'),binary/tool)
    (binary/tool).chmod(0o755)
os.symlink(artifact,binary/'agentx')
fixture.env.update({'PATH':f'{binary}:/usr/bin:/bin','SHELL':'/bin/sh','CONTEXT_CANDIDATE':str(artifact),
                    'CONTEXT_PREFLIGHT_STUB':str(Path(__file__).resolve().parents[1]/'environment_loading/codex_stub.py'),
                    'AGENTX_CLAUDE_DELIVERY':'inject'})
results,cleanup=[],[]
for tool in ['codex','claude']:
    scope=root/tool/'scope'; scope.mkdir(parents=True)
    capture=root/tool/'capture'; capture.mkdir()
    home=root/tool/'home'; home.mkdir()
    (scope/'config.toml').write_text('[context]\nsources = [{id="guide", file="guide.txt"}]\n')
    (root/'project/guide.txt').write_text('STARTUP_HOOK')
    fixture.env.update({'CODEX_HOME':str(home),'CLAUDE_CONFIG_DIR':str(home),'CONTEXT_HOOK_CAPTURE':str(capture)})
    result=fixture.run(tool,artifact,scope,['--env-file=','launch','--tool',tool,'--go',*(['--stdio'] if tool=='codex' else ['--run-here']),'--',*(['exec'] if tool=='codex' else [])])
    try:
        receipt=json.loads((capture/'result.json').read_text())
    except Exception as error:
        receipt={'error':repr(error),'stdout':result.stdout,'stderr':result.stderr}
    results.append({'tool':tool,'exit_code':result.returncode,'receipt':receipt})
    peer=fixture.peer(scope,artifact)
    cleanup.append(Fixture.terminate(peer))
summary={'root':str(root),'candidate_sha256':hashlib.sha256(artifact.read_bytes()).hexdigest(),'results':results,'cleanup':cleanup,
         'limits':'Real AgentX hooks in launcher-created identities; provider recipient is a fixture. No native compaction or model consumption proven.'}
(evidence/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
raise SystemExit(any(row['exit_code'] or 'error' in row['receipt'] for row in results))
