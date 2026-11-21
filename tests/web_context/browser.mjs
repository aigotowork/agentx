/** Finite consumer journeys against real candidate HTTP and cross-origin iframe. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {randomUUID} from 'node:crypto';
import {checkpoint} from '../collaboration_workbench/browser_checkpoint.mjs';
const config=JSON.parse(await readFile(process.argv[2],'utf8'));
await mkdir(config.evidence,{recursive:false});
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.AGENTX_PLAYWRIGHT_PACKAGE??'/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
for(const url of [config.url,config.parent]) {const u=new URL(url);assert.equal(u.hostname,'127.0.0.1');assert(![18959,18967,18971,38023].includes(Number(u.port)));}
assert.notEqual(new URL(config.parent).origin,new URL(config.url).origin);
const browser=await chromium.launch({executablePath:process.env.AGENTX_BROWSER_EXECUTABLE??'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server']});
const context=await browser.newContext({viewport:{width:1100,height:900},locale:'zh-CN'});
await context.tracing.start({screenshots:true,snapshots:true,sources:true});
const parent=await context.newPage(),traffic=[],errors=[],steps=[];
parent.setDefaultTimeout(20000);
parent.on('pageerror',e=>errors.push(e.message));
parent.on('request',r=>{if(r.url().includes('/api/'))traffic.push({method:r.method(),url:r.url(),post:r.method()==='POST'&&r.url().includes('/messages')?r.postData():undefined});});
let frame,stage='open',failure;
const target=agent=>({scope:config.scope,generation:config.generation,agent});
const protocol='agentx.context.v1';
const draft=()=>frame.getByRole('textbox',{name:'Agent 消息草稿',exact:true});
const send=()=>frame.getByRole('button',{name:'发送 AgentX 消息',exact:true});
const composer=()=>frame.locator('.hall-composer');
async function until(description,observe,timeout=35000){const end=Date.now()+timeout;while(Date.now()<end){const value=await observe();if(value)return value;await new Promise(resolve=>setTimeout(resolve,100));}throw new Error(description+' timed out');}
async function api(path,query={}){const url=new URL(path,config.url);for(const [k,v]of Object.entries({generation:config.generation,...query}))if(v!=null)url.searchParams.set(k,v);const response=await context.request.get(url.href);assert.equal(response.status(),200,await response.text());return response.json();}
async function messages(agent=config.a){return(await api(`/api/v1/agents/${agent.name}/messages`,agent)).messages;}
async function receipt(id){return until('terminal receipt '+id,async()=>{const op=await api('/api/v1/operations/'+id);assert(!['Failed','Unknown'].includes(op.status),JSON.stringify(op));return op.status==='Succeeded'?op:null;});}
async function capture(id,meaning,state={}){
  const child={url:frame.url(),draft:await draft().inputValue().catch(()=>null),aria:await frame.locator('body').ariaSnapshot(),geometry:await frame.evaluate(()=>({width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth,body:document.body.innerText}))};
  await writeFile(`${config.evidence}/${id}-child.json`,JSON.stringify(child,null,2));
  await frame.locator('body').screenshot({path:`${config.evidence}/${id}-child.png`});
  const entry=await checkpoint(parent,config.evidence,id,meaning,{...state,child});steps.push(entry);
  await writeFile(`${config.evidence}/steps.json`,JSON.stringify(steps,null,2));console.log(id+' passed');
}
async function open(agent=config.a){
  await parent.locator('#service').fill(config.url);await parent.locator('#target').fill(JSON.stringify(target(agent)));
  await parent.getByRole('button',{name:'Open iframe',exact:true}).click();
  await until('actual child frame',async()=>{frame=parent.frames().find(f=>f.url().startsWith(config.url+'/space/'));return frame;});
  await draft().waitFor();await until('ready parent',async()=>!(await parent.locator('#add').isDisabled()));
}
async function post(input,{id=randomUUID(),agent=config.a,files=[],times=1,type='addContext'}={}){
  const before=await parent.evaluate(()=>window.results.length);
  await parent.evaluate(({input,id,agent,files,times,type,config,protocol})=>{
    const data={protocol,type,requestId:id,target:{scope:config.scope,generation:config.generation,agent},context:{...input}};
    if(files.length)data.context.files=files.map(f=>f.blob?new Blob([Uint8Array.from(atob(f.data),c=>c.charCodeAt(0))],{type:f.type}):new File([Uint8Array.from(atob(f.data),c=>c.charCodeAt(0))],f.name,{type:f.type}));
    for(let i=0;i<times;i++)document.querySelector('iframe').contentWindow.postMessage(data,config.url);
  },{input,id,agent,files,times,type,config,protocol});
  return until('correlated parent result '+id,async()=>{const rows=await parent.evaluate(({id,before})=>window.results.slice(before).filter(r=>r.type==='result'&&r.requestId===id),{id,before});return rows.length>=times?rows:undefined;});
}
async function clear(){await draft().fill('');while(await composer().getByRole('button',{name:/^移除 /}).count())await composer().getByRole('button',{name:/^移除 /}).first().click();}
async function deliberate(label,expectedAttachments){
  await until('send ready',async()=>!(await send().isDisabled()));
  const request=parent.waitForRequest(r=>r.method()==='POST'&&new URL(r.url()).pathname.endsWith('/messages'));
  await send().click();const req=await request;const body=req.postDataJSON();
  assert.deepEqual(body.agent,config.a);assert.equal(body.generation,config.generation);
  assert.equal(body.message.attachments?.length??0,expectedAttachments);
  const op=await receipt(body.request_id);
  const event=await until('actual native sent message',async()=>(await messages()).find(v=>v.id===op.result.Sent.message_id));
  assert.equal(event.attachments?.length??0,expectedAttachments);
  await until('UI receipt reconciled',async()=>!(await send().isDisabled()));
  await capture(label,'Deliberate native send with exact recipient and durable attachment references.',{body,op,event});return{body,op,event};
}
try{
  await parent.goto(config.parent,{waitUntil:'domcontentloaded'});
  await parent.evaluate(()=>{window.results=[];window.addEventListener('message',event=>{if(event.source===document.querySelector('iframe').contentWindow)window.results.push(event.data);});});
  const baseline=(await messages()).length;
  await open();
  const png={name:'截屏.png',type:'image/png',data:config.png};
  if(config.start==='recovery'){
    const blobs=await post({text:'raw Blob prerequisite'},{files:[{blob:true,type:'application/octet-stream',data:Buffer.from('blob bytes').toString('base64')}]});
    assert.equal(blobs[0].status,'added');await composer().getByRole('button',{name:'移除 attachment',exact:true}).waitFor();await clear();
  }
  if(config.start==='narrow-recovery')await parent.addStyleTag({content:'main{grid-template-columns:1fr 360px!important}iframe{width:360px!important;height:740px!important}'});
  if(!config.start){
  assert.equal(traffic.filter(r=>r.method==='POST').length,0);
  await capture('WC01-WC02-focused','Cross-origin parent hosts actual sidebar; exact query navigates without mutations.',{target:target(config.a)});
  stage='parent-context';
  const id=randomUUID();
  let result=await post({title:'Parent page',text:'context duplicate sentinel',url:'https://example.test/source'},{id,times:2});
  assert(result.every(r=>r.status==='added'),JSON.stringify(result));
  await post({text:'changed duplicate body must not append'},{id});
  assert.equal((await draft().inputValue()).split('context duplicate sentinel').length-1,1);
  assert(!(await draft().inputValue()).includes('changed duplicate'));
  assert.equal((await messages()).length,baseline);
  result=await post({url:'javascript:alert(1)'});assert.equal(result[0].status,'error');
  result=await post({text:'must not launch'},{type:'launch'});assert.equal(result[0].status,'error');
  result=await post({text:'foreign generation'},{agent:{...config.a,incarnation:'1'}});assert.equal(result[0].status,'error');
  const beforeWrong=await draft().inputValue();
  await frame.evaluate(({protocol,target})=>{const sibling=document.createElement('iframe');document.body.append(sibling);sibling.contentWindow.parent.postMessage({protocol,type:'addContext',requestId:'sibling-wrong-source',target,context:{text:'WRONG SOURCE'}},'*');}, {protocol,target:target(config.a)});
  await post({text:'validation recovered'});
  assert((await draft().inputValue()).startsWith(beforeWrong));
  assert(!(await draft().inputValue()).includes('WRONG SOURCE'));
  await capture('WC03-WC04-parent','Ready/correlated context deduplicates pending/completed requests; invalid target/URL and non-parent source do not mutate native messages.');
  const file={name:'context.bin',type:'application/octet-stream',data:Buffer.from('parent binary\0bytes').toString('base64')};
  result=await post({text:'binary parent context'},{files:[png,file]});assert.equal(result[0].status,'added',JSON.stringify(result));
  assert.equal(await composer().getByRole('button',{name:/^移除 /}).count(),2);
  await composer().getByRole('button',{name:'移除 context.bin',exact:true}).click();
  await capture('WC05-review','Structured-cloned File bytes enter removable pending composer; native mailbox unchanged.');
  await deliberate('WC06-parent-send',1);await clear();
  stage='manual-intake';
  await frame.getByLabel('选择附件',{exact:true}).setInputFiles({name:'picker.bin',mimeType:'application/octet-stream',buffer:Buffer.from('picker bytes')});
  await until('picker ready',async()=>await composer().getByRole('button',{name:'移除 picker.bin',exact:true}).count());
  await deliberate('WC07-WC09-attachment-only',1);await clear();
  await draft().fill('ordinary browser text only');await deliberate('WC07-text-only',0);await clear();
  for(const kind of ['drop','paste']){
    await composer().evaluate((element,{kind,png})=>{
      const data=new DataTransfer();data.items.add(new File([Uint8Array.from(atob(png),c=>c.charCodeAt(0))],kind+'.png',{type:'image/png'}));
      if(kind==='drop')element.dispatchEvent(new DragEvent('drop',{dataTransfer:data,bubbles:true,cancelable:true}));
      else element.querySelector('textarea').dispatchEvent(new ClipboardEvent('paste',{clipboardData:data,bubbles:true,cancelable:true}));
    },{kind,png:config.png});
    await until(kind+' ready',async()=>await composer().getByRole('button',{name:`移除 ${kind}.png`,exact:true}).count());
  }
  await deliberate('WC10-drop-paste',2);await clear();
  stage='upload-failure';
  await draft().fill('retain previous draft');
  let uploadCount=0;
  await parent.route('**/api/v1/attachments?**',async route=>{if(route.request().method()!=='POST'){await route.continue();return;}uploadCount++;if(uploadCount===2)await route.abort('failed');else await route.continue();});
  result=await post({text:'must not commit partial batch'},{files:[png,file]});
  assert.equal(result[0].status,'error');assert.equal(await draft().inputValue(),'retain previous draft');assert.equal(await composer().getByRole('button',{name:/^移除 /}).count(),0);
  await parent.unroute('**/api/v1/attachments?**');
  await parent.route('**/api/v1/attachments?**',route=>route.request().method()==='POST'?route.abort('failed'):route.continue());
  await frame.getByLabel('选择附件',{exact:true}).setInputFiles({name:'failed.bin',mimeType:'application/octet-stream',buffer:Buffer.from('failure')});
  await composer().getByRole('alert').waitFor();
  await capture('WC11-upload-error','Partial parent batch leaves prior draft intact; manual upload failure is actionable with no dangling reference.');
  await parent.unroute('**/api/v1/attachments?**');await clear();
  stage='late-identity';
  let release,held;const gate=new Promise(resolve=>release=resolve);
  await parent.route('**/api/v1/attachments?**',async route=>{if(route.request().method()!=='POST'){await route.continue();return;}held=true;await gate;await route.continue();});
  const pending=post({text:'late A context'},{files:[png]});await until('held upload',async()=>held);
  // A second valid addContext moves visible selection to B while A's upload remains in flight.
  await post({text:'B-only draft'},{agent:config.b});assert.equal(await draft().inputValue(),'B-only draft');
  release();result=await pending;assert.equal(result[0].status,'added',JSON.stringify(result));
  await parent.unroute('**/api/v1/attachments?**');
  assert((await draft().inputValue()).includes('late A context'));assert(!(await draft().inputValue()).includes('B-only draft'));
  await post({text:'B still isolated'},{agent:config.b});assert((await draft().inputValue()).includes('B-only draft'));assert(!(await draft().inputValue()).includes('late A'));
  await post({text:'return A'},{agent:config.a});await capture('WC12-late-identity','Late A upload remains in exact A draft while B draft stays isolated.');await clear();
  }
  if(config.start!=='remaining'){
  stage='lost-admitted-response';
  await post({text:'browser admitted reply lost'},{files:[png]});
  let lost,attempts=0;
  await parent.route('**/api/v1/operations/**',route=>route.abort('failed'));
  await parent.route('**/api/v1/agents/*/messages',async route=>{
    if(route.request().method()!=='POST'){await route.continue();return;}attempts++;
    const body=route.request().postDataJSON();
    const stored=await frame.evaluate(()=>Object.values(localStorage));assert(stored.some(v=>v.includes(body.request_id)),'journal not persisted before POST');
    const response=await route.fetch();assert.equal(response.status(),202);lost=body;await route.abort('failed');
  });
  await send().click();await until('real upstream admission',async()=>lost);const resolved=await receipt(lost.request_id);
  await parent.unroute('**/api/v1/agents/*/messages');
  await frame.goto(frame.url(),{waitUntil:'domcontentloaded'});await draft().waitFor();await draft().fill('replacement blocked');assert(await send().isDisabled());
  await capture('WC13-unknown','Lost admitted reply remains uncertain across iframe reload without automatic resend.',{lost,resolved});
  assert.equal(attempts,1);assert.equal((await messages()).filter(v=>v.id===resolved.result.Sent.message_id).length,1);
  await parent.unroute('**/api/v1/operations/**');
  const details=frame.locator('.live-receipts');await details.locator('summary').click();
  const popup=await details.locator(':scope > div').evaluate(el=>{const rect=el.getBoundingClientRect();return{x:rect.x,right:rect.right,width:rect.width,viewport:innerWidth};});
  assert(popup.x>=0&&popup.right<=popup.viewport,JSON.stringify(popup));
  await capture('WC13-popup-open','Original receipt popup is contained inside the actual child viewport before a normal pointer lookup.',{popup,resolved});
  await details.locator('section').filter({hasText:lost.request_id}).getByRole('button',{name:'查询原回执',exact:true}).click();
  await until('original receipt reconciles',async()=>!(await send().isDisabled()));await clear();
  await details.locator('summary').click();
  await capture('WC13-reconciled','Original-ID lookup resolves actual native admission through reachable recovery control.',{resolved,attempts});
  }
  if(!['recovery','narrow-recovery'].includes(config.start)){
  stage='durable-history';
  await frame.getByRole('link',{name:'打开附件 截屏.png',exact:true}).first().waitFor();
  const cards=await frame.locator('.context-attachments a').count();assert(cards>=2);
  const image=frame.locator('.context-attachments img').first();await until('actual image loaded',()=>image.evaluate(el=>el.complete&&el.naturalWidth>0));
  const fileLink=frame.getByRole('link',{name:'打开附件 picker.bin',exact:true}).first();
  const response=await context.request.get(new URL(await fileLink.getAttribute('href'),config.url).href);assert.equal((await response.body()).toString(),'picker bytes');
  await capture('WC08-history','Fresh child page restores structured image/file cards and actual browser image/download bytes.');
  }
  if(config.start!=='narrow-recovery'){
  stage='sidebar';
  await parent.addStyleTag({content:'main{grid-template-columns:1fr 360px!important}iframe{width:360px!important;height:740px!important}'});
  await post({text:'narrow sidebar review'},{files:[{...png,name:'a-long-context-screen-name-截图.png'}]});
  await frame.getByRole('button',{name:'添加附件',exact:true}).focus();assert.equal(await frame.evaluate(()=>document.activeElement?.getAttribute('aria-label')),'添加附件');
  const dimensions=await frame.evaluate(()=>({width:innerWidth,scroll:document.documentElement.scrollWidth}));assert(dimensions.scroll<=dimensions.width,JSON.stringify(dimensions));
  await capture('WC15-sidebar','360px iframe retains exact target, context image/remove and reachable keyboard attach/send actions.',{dimensions});
  await draft().press('Control+Enter');await until('keyboard send actual effect',async()=>(await messages()).some(m=>m.text.includes('narrow sidebar review')));
  stage='successor-isolation';
  async function mutate(action,agent){const response=await context.request.post(`${config.url}/api/v1/agents/${agent.name}/${action}`,{data:{request_id:randomUUID(),generation:config.generation,agent}});assert.equal(response.status(),202,await response.text());return receipt((await response.json()).request_id);}
  await mutate('stop',config.a);
  const stopped=await until('stopped history',async()=>(await api('/api/v1/agents')).agents.find(v=>v.agent.name===config.a.name&&v.agent.stopped_event));
  const resumed=await mutate('resume',stopped.agent);
  const successor=await until('sendable successor',async()=>(await api('/api/v1/agents')).agents.find(v=>v.agent.name===resumed.result.Launched.agents[0]&&v.can_send&&v.agent.incarnation));
  assert.notDeepEqual(successor.agent,config.a);
  const rejected=await post({text:'stale A must not target successor'});assert.equal(rejected[0].status,'error');
  const unknown=await post({text:'unsupported cannot launch'},{type:'launch'});assert.equal(unknown[0].status,'error');
  const staleBody={request_id:randomUUID(),generation:config.generation,agent:config.a,message:{text:'stale direct send',attachments:[],intent:'request',reply_to:null,thread:null}};
  const stale=await context.request.post(`${config.url}/api/v1/agents/${config.a.name}/messages`,{data:staleBody});assert.equal(stale.status(),409);
  await capture('WC12-successor','Stopped exact instance cannot deliver parent context or native send to resumed namesake.',{old:config.a,history:stopped.agent,successor:successor.agent,parentResult:rejected});
  stage='invalid-query';
  const invalid=await context.newPage();const invalidPosts=[];invalid.on('request',r=>{if(r.method()==='POST')invalidPosts.push(r.url());});
  const url=new URL(frame.url());url.searchParams.set('incarnation',config.a.incarnation);await invalid.goto(url.href,{waitUntil:'domcontentloaded'});
  await until('explicit stale query',async()=>/精确目标|失效/.test(await invalid.locator('body').innerText()));assert.equal(invalidPosts.length,0);await invalid.close();
  const history=await context.newPage();url.searchParams.delete('incarnation');url.searchParams.set('stopped_event',stopped.agent.stopped_event);
  await history.goto(url.href,{waitUntil:'domcontentloaded'});await history.getByRole('textbox',{name:'Agent 消息草稿',exact:true}).waitFor();
  assert(await history.getByRole('button',{name:'发送 AgentX 消息',exact:true}).isDisabled());
  await history.close();
  const office=await context.newPage();url.searchParams.delete('stopped_event');url.searchParams.set('incarnation',successor.agent.incarnation);url.searchParams.set('office','/wrong-owned-office');
  await office.goto(url.href,{waitUntil:'domcontentloaded'});await until('office mismatch explicit',async()=>(await office.locator('body').innerText()).includes('office 与精确 Agent 归属不匹配'));await office.close();
  }
  assert.equal(errors.length,0,JSON.stringify(errors));
}catch(error){failure=error.stack;console.error(stage+'\n'+failure);if(frame)await capture('failure-'+stage,'Failed consumer checkpoint; preserve original evidence.',{failure}).catch(()=>{});}
finally{
  await writeFile(`${config.evidence}/traffic.json`,JSON.stringify(traffic,null,2));
  await writeFile(`${config.evidence}/parent-results.json`,JSON.stringify(await parent.evaluate(()=>window.results??[]).catch(()=>[]),null,2));
  await writeFile(`${config.evidence}/result.json`,JSON.stringify({status:failure?'fail':'pass',stage,failure,errors,limits:'Synthetic browser DataTransfer/ClipboardEvent; no OS clipboard or real model visual comprehension.'},null,2));
  await context.tracing.stop({path:`${config.evidence}/trace.zip`});await browser.close();
}
if(failure)process.exitCode=1;
