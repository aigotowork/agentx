#!/usr/bin/env python3
"""One private fixture, three native listeners, actual cross-origin browser SDK."""
import argparse, hashlib, json, subprocess, sys, threading, uuid
from pathlib import Path
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/live_ui_integration'))
from live_fixture import LiveFixture, wait_for
from provider import process_identity

def main():
 p=argparse.ArgumentParser(description=__doc__)
 for key in ['candidate','evidence']: p.add_argument('--'+key,required=True,type=Path)
 p.add_argument('--sha256',required=True)
 p.add_argument('--resume',type=Path)
 a=p.parse_args(); candidate=a.candidate.resolve(strict=True)
 assert hashlib.sha256(candidate.read_bytes()).hexdigest()==a.sha256
 a.evidence.mkdir(parents=True,exist_ok=False)
 f=LiveFixture.reopen_owned(candidate,a.evidence/'fixture',a.resume) if a.resume else LiveFixture(candidate,a.evidence/'fixture'); server=None
 try:
  h=f.start('primary') if a.resume else f.prepare('primary'); env=f.environment('primary'); scope=f.root/'primary/scope'; project=f.root/'primary/project'
  urls={'management':h.url}
  for name,args in [('observation',['observe-web']),('workbench',['workbench','--project',str(project),'--collaboration'])]:
   out=a.evidence/(name+'.stdout'); err=a.evidence/(name+'.stderr')
   with out.open('w') as o,err.open('w') as e:
    proc=subprocess.Popen([str(candidate),*args,'--scope',str(scope),'--host','127.0.0.1','--port','0'],env=dict(env),cwd=project,stdout=o,stderr=e,start_new_session=True)
   rec=process_identity(proc.pid); f.processes.append(rec); f.manifest['processes'].append(rec); f.persist()
   def ready():
    assert proc.poll() is None,err.read_text()
    return next((l.strip() for l in out.read_text().splitlines() if l.startswith('http://')),None)
   urls[name]=wait_for(name+' ready',ready)
  payload={'generation':h.generation,'request_id':str(uuid.uuid4()),'launch':{'tool':'qoder','profile':None,'directory':None,'prompt':'browser SDK system fixture','title':'browser-sdk-system'}}
  h.post('/api/v1/agents',payload); launched=h.operation(payload['request_id']); name=launched['result']['Launched']['agents'][0]
  ref=wait_for('fixture ready',lambda:next((r['agent'] for r in h.agents() if r['agent']['name']==name and r['can_send'] and r['session_id']),None))
  static=a.evidence/'static'; static.mkdir()
  entry=static/'sdk-entry.mjs'; entry.write_text("import * as sdk from "+json.dumps(str(ROOT/'packages/agentx-client/dist/index.js'))+"; globalThis.acceptanceSDK=sdk;\n")
  subprocess.run([str(ROOT/'node_modules/.bin/esbuild'),str(entry),'--bundle','--platform=browser','--format=iife','--outfile='+str(static/'sdk.js')],check=True)
  (static/'index.html').write_text('<!doctype html><meta charset="utf-8"><script src="/sdk.js"></script><main>Independent SDK acceptance</main>')
  class Handler(SimpleHTTPRequestHandler):
   def __init__(self,*args,**kwargs): super().__init__(*args,directory=str(static),**kwargs)
   def log_message(self,*args): pass
  server=ThreadingHTTPServer(('127.0.0.1',0),Handler); threading.Thread(target=server.serve_forever,daemon=True).start()
  config={'urls':urls,'generation':h.generation,'agent':ref,'scope':str(scope),'project':str(project),'page':f'http://127.0.0.1:{server.server_port}','evidence':str(a.evidence/'browser')}
  cp=a.evidence/'config.json'; cp.write_text(json.dumps(config,indent=2)+'\n')
  subprocess.run(['node',str(Path(__file__).with_name('system-browser.mjs')),str(cp)],cwd=ROOT,check=True,timeout=180)
  (a.evidence/'native-final.json').write_text(json.dumps({'agents':h.agents(),'messages':h.get(f'/api/v1/agents/{name}/messages',incarnation=ref['incarnation'])},indent=2)+'\n')
 finally:
  if server: server.shutdown();server.server_close()
  f.close()
if __name__=='__main__': main()
