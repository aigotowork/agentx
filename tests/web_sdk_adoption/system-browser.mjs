import assert from 'node:assert/strict';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
const c=JSON.parse(await readFile(process.argv[2],'utf8'));
await mkdir(c.evidence,{recursive:false});
const {chromium}=createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE??'/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser=await chromium.launch({executablePath:process.env.AGENTX_BROWSER_EXECUTABLE??'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server']});
const ctx=await browser.newContext(); const page=await ctx.newPage();const traffic=[],errors=[],ui=[];let result,failure;
const cd=await ctx.newCDPSession(page);await cd.send('Network.enable');
cd.on('Network.requestWillBeSent',e=>{if(e.request.url.includes('/api/'))traffic.push({kind:'request',method:e.request.method,url:e.request.url,headers:e.request.headers,body:e.request.postData});});
cd.on('Network.responseReceived',e=>{if(e.response.url.includes('/api/'))traffic.push({kind:'response',url:e.response.url,status:e.response.status,headers:e.response.headers});});
page.on('pageerror',e=>errors.push(e.message));
try{
 await page.goto(c.page);await page.waitForFunction(()=>!!globalThis.acceptanceSDK);
 result=await page.evaluate(async c=>{
  const s=globalThis.acceptanceSDK, checks=[];
  const check=(name,ok,value)=>{if(!ok)throw new Error(name+': '+s.stringifyJson(value));checks.push({name,value});};
  const m=s.createManagementClient({baseUrl:c.urls.management});const w=s.createWorkbenchClient({baseUrl:c.urls.workbench});const o=s.createObservationClient({baseUrl:c.urls.observation});
  const desc=await m.describe();check('management exact descriptor',desc.generation===c.generation,desc);
  const row=await m.getAgent({generation:c.generation,agent:c.agent});check('lossless exact Agent',row.agent.incarnation===c.agent.incarnation,row.agent);
  const bytes=new Uint8Array([0,1,127,128,255,0,42]);const file=await m.attachments.upload({generation:c.generation,name:'browser-nul.bin',media_type:'application/octet-stream',bytes:new Blob([bytes])});
  const download=new Uint8Array(await(await m.attachments.download({generation:c.generation,id:file.id})).arrayBuffer());check('cross-origin binary roundtrip',bytes.every((v,i)=>download[i]===v)&&bytes.length===download.length,{file,bytes:[...download]});
  const body={generation:c.generation,request_id:crypto.randomUUID(),agent:c.agent,message:{text:'browser SDK cross-origin actual message',intent:'request',thread:null,reply_to:null,attachments:[file.id]}};
  const loss=s.createManagementClient({baseUrl:c.urls.management,fetch:async(input,init)=>{const res=await fetch(input,init);if(res.status===202&&String(input instanceof Request?input.url:input).includes('/messages')){await res.arrayBuffer();throw new TypeError('deliberately discard actual admitted response');}return res;}});
  let unknown;try{await loss.send(body);}catch(e){unknown=e;}
  check('lost admitted response retains original intent',unknown?.input?.request_id===body.request_id,{name:unknown?.name,input:unknown?.input});
  const receipt=await m.waitForOperation(body,{timeoutMs:15000,pollIntervalMs:100});check('original receipt lookup resolves Sent',receipt.status==='Succeeded'&&!!receipt.result?.Sent,receipt);
  const replay=await m.send(body);check('same intent replay same message',replay.result?.Sent?.message_id===receipt.result.Sent.message_id,replay);
  let conflict;try{await m.send({...body,message:{...body.message,text:'changed intent'}});}catch(e){conflict={name:e.name,status:e.status,code:e.code};}check('changed intent refuses',!!conflict,conflict);
  const settings=await m.inspection.get(c.generation);check('Inspector exact generation',settings.generation===c.generation,settings.settings);
  const wd=await w.describe();check('independent Workbench descriptor',wd.generation===c.generation,wd);
  const call={generation:c.generation,command_id:crypto.randomUUID(),work:null,expected_revision:null,command:{CreateWork:{brief:{title:'Browser SDK bounded work',goal:'Verify actual browser command codec',source:null,criteria:[{id:'proof',text:'real native receipt',required:true}]},implementer:s.participantFromAgent(c.agent),checker:s.participantFromAgent(c.agent)}}};
  const created=await w.apply(call);check('native Work creation or exact domain refusal',!!created.Applied||!!created.Rejected,created);
  if(created.Applied){const again=await w.apply(call);check('Work original replay',again.Applied?.replayed===true,again);const detail=await w.works.get(c.generation,created.Applied.reference);check('Work bigint native revision',typeof detail.Read?.Detail?.work?.revision==='bigint',detail);}
  const scope=await o.scope();check('passive exact scope',scope.canonical_scope===c.scope,scope);
  const snap=await o.runtime.snapshot();check('native passive snapshot',!!snap.Inspection,snap);
  const connections=await o.runtime.connections({before:null,limit:8n});check('Observation marker integer request',!!connections.Inspection,connections);
  o.close();return s.stringifyJson({checks});
 },c);
 const parsed=JSON.parse(result);assert(parsed.checks.length>=12);
 for(const [name,url] of Object.entries(c.urls)){
  const p=await ctx.newPage();const api=[];p.on('response',r=>{if(r.url().includes('/api/'))api.push({url:r.url(),status:r.status()});});
  await p.goto(url,{waitUntil:'domcontentloaded'});await p.waitForTimeout(1800);
  const text=await p.locator('body').innerText();await p.screenshot({path:`${c.evidence}/${name}.png`,fullPage:true});
  assert(api.some(x=>x.status===200),`${name} consumer has no successful API response`);ui.push({name,url,text:text.slice(0,14000),api});await p.close();
 }
 assert.deepEqual(errors,[]);
}catch(e){failure={message:e.message,stack:e.stack};}
finally{await writeFile(`${c.evidence}/result.json`,JSON.stringify({status:failure?'failed':'passed',browser:browser.version(),result:result?JSON.parse(result):null,ui,traffic,errors,failure},null,2)+'\n');await browser.close();}
if(failure)throw new Error(failure.message);
