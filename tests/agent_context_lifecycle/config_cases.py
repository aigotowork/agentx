#!/usr/bin/env python3
"""Exercise public config-key persistence without launching a provider."""
import json
from pathlib import Path
import sys
import tempfile
import tomllib

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from startup_usability.fixture import Fixture

assert '--handoff' in sys.argv
root=Path(tempfile.mkdtemp(prefix='agentx-startup-fixture-context-config-'))
evidence=root/'evidence'; evidence.mkdir()
fixture=Fixture(root,evidence)
scope=root/'scope'; scope.mkdir()
artifact=Path('/tmp/agentx-context-lifecycle-candidate')
path=scope/'config.toml'
results=[]
def run(label,args):
    return fixture.run(label,artifact,scope,['--env-file=','config',*args])
try:
    sources=[{'id':'custom','text':'CONFIG_LITERAL'}]
    result=run('set',['instruction_sources',json.dumps(sources)])
    assert result.returncode==0,result.stderr
    assert tomllib.loads(path.read_text())['instructions']['sources']==sources
    results.append({'case':'config-key-array-persistence','passed':True})
    result=run('get',['instruction_sources','--json'])
    assert result.returncode==0 and 'CONFIG_LITERAL' in result.stdout,(result.stdout,result.stderr)
    results.append({'case':'config-key-get','passed':True})
    before=path.read_bytes()
    result=run('reject',['instruction_sources','[{"id":"duplicate","text":"a"},{"id":"duplicate","text":"b"}]'])
    assert result.returncode!=0 and path.read_bytes()==before,(result.stdout,result.stderr)
    results.append({'case':'config-reject-atomic','passed':True})
    result=run('disable',['instruction_sources','[]'])
    assert result.returncode==0 and tomllib.loads(path.read_text())['instructions']['sources']==[]
    results.append({'case':'config-key-empty-array','passed':True})
    result=run('unset',['instruction_sources','--unset'])
    assert result.returncode==0 and 'sources' not in tomllib.loads(path.read_text()).get('instructions',{})
    results.append({'case':'config-key-unset','passed':True})
except Exception as error:
    results.append({'passed':False,'error':repr(error)})
finally:
    cleanup=None
    try: cleanup=Fixture.terminate(fixture.peer(scope,artifact))
    except (FileNotFoundError,ConnectionRefusedError): pass
    report={'root':str(root),'results':results,'cleanup':cleanup}
    (evidence/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))
raise SystemExit(any(not row['passed'] for row in results))
