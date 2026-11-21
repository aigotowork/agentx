#!/usr/bin/env python3
"""Real ordinary Codex compact with a deterministic loopback model transport.

This proves native request construction and hook execution, not remote model
interpretation. No inherited credentials, user homes or managed native daemon.
"""
import argparse
import hashlib
import http.server
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import threading

repo = Path(__file__).resolve().parents[2]
sys.path.insert(0,str(repo/'tests'))
from startup_usability.fixture import Fixture
spec=importlib.util.spec_from_file_location('protocol_probe',repo/'.docs/codex-activation/.tools/probe_native_protocol.py')
probe=importlib.util.module_from_spec(spec); spec.loader.exec_module(probe)

class Model(http.server.BaseHTTPRequestHandler):
    protocol_version='HTTP/1.1'
    def log_message(self,*args): pass
    def do_GET(self):
        body=json.dumps({'object':'list','data':[{'id':'fixture-model','object':'model'}]}).encode()
        self.send_response(200); self.send_header('Content-Length',str(len(body))); self.end_headers(); self.wfile.write(body)
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        self.server.trace.add('model',{'path':self.path,'body':body})
        self.send_response(200); self.send_header('Content-Type','text/event-stream'); self.send_header('Connection','close'); self.end_headers()
        item={'id':'msg_fixture','type':'message','role':'assistant','status':'completed','content':[{'type':'output_text','text':'Fixture summary and response.','annotations':[]}]}
        response={'id':'resp_fixture','object':'response','status':'completed','output':[item],'usage':{'input_tokens':2000,'output_tokens':10,'total_tokens':2010}}
        for kind,extra in [('response.created',{'response':{**response,'status':'in_progress','output':[]}}),('response.output_item.added',{'output_index':0,'item':{**item,'status':'in_progress'}}),('response.output_item.done',{'output_index':0,'item':item}),('response.completed',{'response':response})]:
            self.wfile.write(('event: '+kind+'\ndata: '+json.dumps({'type':kind,**extra})+'\n\n').encode()); self.wfile.flush()

parser=argparse.ArgumentParser(); parser.add_argument('--mode',choices=['manual','auto'],required=True); parser.add_argument('--handoff',action='store_true',required=True)
args=parser.parse_args()
artifact=Path('/tmp/agentx-context-lifecycle-candidate')
native=Path('/home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex')
assert native.is_file(),native
root=Path(tempfile.mkdtemp(prefix='agentx-startup-fixture-native-context-'))
evidence=root/'evidence'; evidence.mkdir()
fixture=Fixture(root,evidence)
trace=probe.Trace(evidence)
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Model); server.daemon_threads=True; server.trace=trace
threading.Thread(target=server.serve_forever,daemon=True).start()
home=root/'native-home'; home.mkdir()
scope=root/'scope'; scope.mkdir()
binary=root/'bin'; binary.mkdir()
shutil.copy2(repo/'tests/agent_context/native_entry.py',binary/'codex'); (binary/'codex').chmod(0o755)
os.symlink(artifact,binary/'agentx')
(home/'config.toml').write_text(f'''model = "fixture-model"
model_provider = "fixture"
model_auto_compact_token_limit = {100 if args.mode == 'auto' else 1000000}
developer_instructions = "NATIVE_DEVELOPER_KEEP"
approval_policy = "never"
sandbox_mode = "read-only"
[features]
memories = false
[model_providers.fixture]
name = "Isolated loopback fixture"
base_url = "http://127.0.0.1:{server.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
request_max_retries = 0
stream_max_retries = 0
[analytics]
enabled = false
''')
(scope/'config.toml').write_text('[instructions]\nsources = [{id="fixed", file="fixed.txt"}]\n[context]\nsources = [{id="guide", file="guide.txt"}]\n')
fixed=root/'project/fixed.txt'; fixed.write_text('LIFECYCLE_FIXED_SENTINEL')
guide=root/'project/guide.txt'; guide.write_text('NATIVE_BEFORE')
env={**fixture.env,'PATH':f'{binary}:/usr/bin:/bin','AGENTX_DIR':str(scope),'CODEX_HOME':str(home),'CONTEXT_REAL_CODEX':str(native),'RUST_LOG':'error'}
command=[str(artifact),'--env-file=','launch','--tool','codex','--stdio','--go','--','exec']
summary={'mode':args.mode,'root':str(root),'candidate_sha256':hashlib.sha256(artifact.read_bytes()).hexdigest(),'native_sha256':hashlib.sha256(native.read_bytes()).hexdigest(),'command':command,'model':'deterministic local protocol fixture','passed':False}
client=None
try:
    client=probe.Client(command,env,root/'project',trace,'native')
    summary['initialize']=client.initialize()
    result=client.request('thread','thread/start',{'cwd':str(root/'project'),'model':'fixture-model','modelProvider':'fixture','approvalPolicy':'never','sandbox':'read-only'})
    assert 'error' not in result['value'],result
    tid=result['value']['result']['thread']['id']; summary['thread_id']=tid
    mark=len(trace.events); probe.turn(client,'first',tid,'FIRST_REAL_NATIVE_REQUEST'); probe.completed(trace,tid,mark)
    first_models=[event['value']['body'] for event in trace.events if event['source']=='model']
    assert first_models, 'no first model request'
    first=json.dumps(first_models[-1])
    assert 'NATIVE_BEFORE' in first and 'LIFECYCLE_FIXED_SENTINEL' in first, first
    first_developer=[item for item in first_models[-1].get('input',[]) if item.get('role')=='developer']
    fixed_messages=[item for item in first_developer if 'LIFECYCLE_FIXED_SENTINEL' in json.dumps(item)]
    assert fixed_messages and all('NATIVE_BEFORE' not in json.dumps(item) for item in fixed_messages), fixed_messages
    assert any('NATIVE_BEFORE' in json.dumps(item) and 'agentx_runtime_context' in json.dumps(item) for item in first_developer), first_developer
    fixed.write_text('LIFECYCLE_FIXED_CHANGED')
    fixed.unlink()
    guide.write_text('NATIVE_AFTER_COMPACT')
    mark=len(trace.events)
    if args.mode=='manual':
        compact=client.request('compact','thread/compact/start',{'threadId':tid}); summary['compact_response']=compact['value']
        assert 'error' not in compact['value'],compact
        trace.wait(lambda event:event['source']=='native' and event['value'].get('method')=='item/completed' and event['value'].get('params',{}).get('item',{}).get('type')=='contextCompaction',mark,timeout=25)
        mark=len(trace.events)
    started=probe.turn(client,'after',tid,'SECOND_REAL_NATIVE_REQUEST')
    assert 'error' not in started['value'],started
    turn_id=started['value']['result']['turn']['id']
    trace.wait(lambda event:event['source']=='native' and event['value'].get('method')=='turn/completed' and event['value'].get('params',{}).get('turn',{}).get('id')==turn_id,mark,timeout=25)
    models=[event for event in trace.events[mark:] if event['source']=='model']
    assert models, 'no model request after compact'
    last=json.dumps(models[-1]['value']['body'])
    summary['fresh_context_in_request']='NATIVE_AFTER_COMPACT' in last
    summary['native_developer_retained']='NATIVE_DEVELOPER_KEEP' in last
    final_body=models[-1]['value']['body']
    developer=json.dumps([item for item in final_body.get('input',[]) if item.get('role')=='developer'])
    summary['fixed_retained_after_removal']='LIFECYCLE_FIXED_SENTINEL' in developer and 'LIFECYCLE_FIXED_CHANGED' not in last
    final_developer=[item for item in final_body.get('input',[]) if item.get('role')=='developer']
    summary['dynamic_separate_from_fixed']=all('NATIVE_AFTER_COMPACT' not in json.dumps(item) and 'NATIVE_BEFORE' not in json.dumps(item) for item in final_developer if 'LIFECYCLE_FIXED_SENTINEL' in json.dumps(item))
    summary['dynamic_developer_message']=any('NATIVE_AFTER_COMPACT' in json.dumps(item) and 'agentx_runtime_context' in json.dumps(item) for item in final_developer)
    assert summary['fixed_retained_after_removal'] and summary['dynamic_separate_from_fixed'] and summary['dynamic_developer_message'], summary
    summary['compaction_items']=[event for event in trace.events if event['source']=='native' and event['value'].get('method')=='item/completed' and event['value'].get('params',{}).get('item',{}).get('type')=='contextCompaction']
    summary['user_items']=[event['value']['params']['item'] for event in trace.events if event['source']=='native' and event['value'].get('method')=='item/completed' and event['value'].get('params',{}).get('item',{}).get('type')=='userMessage']
    assert summary['fresh_context_in_request'] and summary['native_developer_retained'],summary
    assert summary['compaction_items'], 'no native compaction event'
    assert len(summary['user_items'])==2,summary['user_items']
    # Removing all dynamic sources must emit a clearing snapshot, never reread
    # the deleted frozen file or retain a current assignment from old context.
    (scope/'config.toml').write_text('[instructions]\nsources = [{id="fixed", file="fixed.txt"}]\n[context]\nsources = []\n')
    mark=len(trace.events)
    empty=client.request('compact-empty','thread/compact/start',{'threadId':tid})
    assert 'error' not in empty['value'], empty
    trace.wait(lambda event:event['source']=='native' and event['value'].get('method')=='item/completed' and event['value'].get('params',{}).get('item',{}).get('type')=='contextCompaction',mark,timeout=25)
    mark=len(trace.events)
    third=probe.turn(client,'empty',tid,'THIRD_EMPTY_SNAPSHOT')
    third_id=third['value']['result']['turn']['id']
    trace.wait(lambda event:event['source']=='native' and event['value'].get('method')=='turn/completed' and event['value'].get('params',{}).get('turn',{}).get('id')==third_id,mark,timeout=25)
    third_models=[event for event in trace.events[mark:] if event['source']=='model']
    third_body=third_models[-1]['value']['body']
    summary['empty_snapshot_clears']='No current runtime context.' in json.dumps(third_body)
    summary['fixed_survives_empty']='LIFECYCLE_FIXED_SENTINEL' in json.dumps(third_body)
    all_users=[event for event in trace.events if event['source']=='native' and event['value'].get('method')=='item/completed' and event['value'].get('params',{}).get('item',{}).get('type')=='userMessage']
    assert summary['empty_snapshot_clears'] and summary['fixed_survives_empty'] and len(all_users)==3, summary
    summary['passed']=True
except Exception as error:
    summary['error']=repr(error)
finally:
    if client: client.stop()
    server.shutdown(); server.server_close()
    try: summary['cleanup']=Fixture.terminate(fixture.peer(scope,artifact))
    except Exception as error: summary['cleanup_error']=repr(error)
    (evidence/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(summary,indent=2))
raise SystemExit(not summary['passed'])
