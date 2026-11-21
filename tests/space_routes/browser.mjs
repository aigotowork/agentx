/** Finite independent consumer scenarios: actual complete App, UI and browser History. */
import {chromium} from '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {installFixture,exactURL,ref,row,scope,generation} from './fixture.mjs';
const [base,out]=process.argv.slice(2);assert(base&&out);const host=new URL(base);assert.equal(host.hostname,'127.0.0.1');assert(!['38023','18959','18967','18971'].includes(host.port));
await mkdir(out,{recursive:false});
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server']});
const results=[],errors=[];let current;
const save=(name,data)=>writeFile(`${out}/${name}.json`,JSON.stringify(data,null,2));
const button=(p,name)=>p.getByRole('button',{name,exact:true});
const draft=p=>p.getByRole('textbox',{name:'Agent 消息草稿',exact:true});
const office=(p,value)=>p.getByLabel('工作区范围').selectOption(value);
const tab=(p,label)=>p.getByRole('navigation',{name:'办公室内容',exact:true}).getByRole('button',{name:new RegExp(`^${label}( |$)`)}).click();
const person=(p,name)=>p.locator('.hall-directory-person').filter({hasText:'@'+name}).click();
const params=p=>new URL(p.url()).searchParams;
async function until(label,fn){const end=Date.now()+7000;while(Date.now()<end){if(await fn())return;await new Promise(r=>setTimeout(r,60));}throw Error('Timed out: '+label);}
async function ready(p){await p.getByRole('navigation',{name:'办公室内容',exact:true}).waitFor();}
async function fresh(fixture){const c=await browser.newContext({viewport:{width:1440,height:1000},locale:'zh-CN'});await c.tracing.start({screenshots:true,snapshots:true,sources:true});const f=fixture===undefined?null:await installFixture(c,fixture);const p=await c.newPage();p.setDefaultTimeout(6500);p.on('pageerror',e=>errors.push(e.message));return {c,p,f};}
async function capture(p,id,extra={}){await save(id,{url:p.url(),semantic:await p.locator('body').ariaSnapshot(),text:await p.locator('body').innerText(),...extra});await p.screenshot({path:`${out}/${id}.png`});}
async function check(id,fn){if(process.env.SPACE_ROUTES_CASES&&!new RegExp(process.env.SPACE_ROUTES_CASES).test(id))return;current=await fresh(fn.fixture);try{await fn(current);results.push({id,status:'passed'});}catch(e){results.push({id,status:'failed',error:e.stack});await capture(current.p,id+'-failure',{traffic:current.f?.traffic}).catch(()=>{});}finally{if(current.f){await save(id+'-traffic',current.f);if(current.f.unexpected.length)results.push({id:id+'-unexpected',status:'failed',unexpected:current.f.unexpected});}await current.c.tracing.stop({path:`${out}/${id}-trace.zip`});await current.c.close();console.log(JSON.stringify(results.at(-1)));await save('results',{base,results,errors});}}
function simulated(fn,options={}){fn.fixture=options;return fn;}
async function roundtrip(c,p,assertion){const url=p.url();const second=await c.newPage();second.setDefaultTimeout(6500);try{await second.goto(url);await ready(second);await assertion(second);await second.reload();await ready(second);await assertion(second);}finally{await second.close();}assert.equal(p.url(),url);}
try {
await check('R1-office-view-resource',async({p,c})=>{
 await p.goto(base+'/space/?supply=demo&host_hint=kept#anchor');await ready(p);
 for(const value of ['/projects/atlas/web','$global','','/projects/empty']){await office(p,value);assert.equal(params(p).get('office')??'',value);await roundtrip(c,p,async q=>assert.equal(await q.getByLabel('工作区范围').inputValue(),value));}
 await office(p,'/projects/atlas/web');await button(p,'聊天').click();await tab(p,'历史');assert.equal(params(p).get('view'),'chat');assert.equal(params(p).get('tab'),'history');await roundtrip(c,p,async q=>assert.equal(await q.locator('.hall-directory-person').count(),1));
 await tab(p,'技能');await roundtrip(c,p,async q=>assert(await q.getByText('技能列表未提供',{exact:true}).isVisible()));
 assert.equal(params(p).get('host_hint'),'kept');assert.equal(new URL(p.url()).hash,'#anchor');await capture(p,'R1-skills');
});
await check('R2-exact-history-channel-facts',async({p,c})=>{
 await p.goto(base+'/space/?supply=demo&office=%2Fprojects%2Fatlas%2Fweb&view=chat');await ready(p);await person(p,'mora');await draft(p).fill('current exact draft');const currentURL=p.url();assert.equal(params(p).get('record'),'current');const currentInc=params(p).get('incarnation');
 await tab(p,'历史');await person(p,'mora');await draft(p).fill('stopped exact draft');assert.equal(params(p).get('record'),'stopped');assert.notEqual(params(p).get('incarnation'),currentInc);const stoppedURL=p.url();
 for(const [label,value] of [['模型记录','model'],['本页笔记','notes'],['消息',null]]){await button(p,label).click();assert.equal(params(p).get('channel'),value);await roundtrip(c,p,async q=>{assert((await q.locator('.hall-conversation-heading').innerText()).includes('停止历史'));assert.equal(params(q).get('channel'),value);});}
 await p.getByLabel('会话详情',{exact:true}).click();assert.equal(params(p).get('detail'),'agent');await roundtrip(c,p,async q=>assert(await q.locator('.reader-facts').isVisible()));
 await button(p,'返回总览').click();assert(!params(p).has('agent'));await p.goBack();await draft(p).waitFor();assert.equal(await draft(p).inputValue(),'stopped exact draft');await p.goForward();assert(!params(p).has('agent'));
 await tab(p,'成员');await person(p,'mora');assert.equal(await draft(p).inputValue(),'current exact draft');assert.equal(params(p).get('incarnation'),currentInc);await capture(p,'R2-current',{currentURL,stoppedURL});
});
await check('R3-post-filter-history',async({p,c})=>{
 await p.goto(base+'/space/?supply=demo&layout=sidebar&host_hint=keep#frag');await ready(p);await button(p,'聊天').click();await tab(p,'岗位');const posts=p.getByRole('region',{name:'可用岗位'}).getByRole('button');assert(await posts.count()>1);await posts.nth(1).click();const post=params(p).get('post');assert(post);await roundtrip(c,p,async q=>{assert.equal(params(q).get('post'),post);assert.equal(await q.locator('.post-detail').count(),1);});
 const length=await p.evaluate(()=>history.length);const search=p.locator('input[type="search"]');await search.fill('review');assert.equal(await p.evaluate(()=>history.length),length);assert.equal(params(p).get('q'),'review');await search.fill('');assert.equal(await p.evaluate(()=>history.length),length);
 await tab(p,'技能');assert.equal(await p.evaluate(()=>history.length),length+1);await p.goBack();assert.equal(params(p).get('post'),post);await p.goForward();assert.equal(params(p).get('tab'),'skills');assert.equal(params(p).get('layout'),'sidebar');assert.equal(params(p).get('host_hint'),'keep');assert.equal(new URL(p.url()).hash,'#frag');await capture(p,'R3-history');
});
await check('R4-scene-panels-mailbox',simulated(async({p,c,f})=>{
 await p.goto(base+'/space/?panel=directory');await ready(p);await p.getByRole('region',{name:'场景名册'}).waitFor();await roundtrip(c,p,async q=>assert(await q.getByRole('region',{name:'场景名册'}).isVisible()));await button(p,'关闭名册').click();assert(!params(p).has('panel'));await p.goBack();await p.getByRole('region',{name:'场景名册'}).waitFor();
 await p.goto(base+'/space/?panel=mailbox');await p.getByText('Latest simulated letter',{exact:true}).waitFor();await button(p,'全部历史').click();assert.equal(params(p).get('mail'),'all');await button(p,'更早信件').click();await p.getByText('Older simulated letter',{exact:true}).waitFor();assert.equal(params(p).get('before'),'cursor-old');await roundtrip(c,p,async q=>assert(await q.getByText('Older simulated letter',{exact:true}).isVisible()));await capture(p,'R4-older');await button(p,'回到最新').click();assert(!params(p).has('before'));assert(f.traffic.every(r=>r.method==='GET'));
}));
await check('R5-delayed-and-paged-exact',simulated(async({p,c,f})=>{
 let release;f.state.descriptorGate=new Promise(r=>release=r);const url=exactURL(base);await p.goto(url);await ready(p);assert.equal(p.url(),url);await p.getByRole('alert').waitFor();assert.equal(await draft(p).count(),0);const length=await p.evaluate(()=>history.length);release();f.state.descriptorGate=null;await draft(p).waitFor();assert.equal(p.url(),url);assert.equal(await p.evaluate(()=>history.length),length);await draft(p).fill('survives refresh');await until('second poll',()=>f.traffic.filter(r=>new URL(r.url).pathname==='/api/v1/management').length>1);assert.equal(await draft(p).inputValue(),'survives refresh');assert.equal(p.url(),url);assert.equal(await p.evaluate(()=>history.length),length);await capture(p,'R5-delayed');
 await p.close();f.state.paged=true;const q=await c.newPage();await q.goto(url);await draft(q).waitFor();assert(f.traffic.some(r=>new URL(r.url).pathname==='/api/v1/agents/mora'));assert.equal(q.url(),url);await capture(q,'R5-paged');await q.close();assert(f.traffic.every(r=>r.method==='GET'));
}));
await check('R6-invalid-stale-superseded',simulated(async({p,f})=>{
 for(const extra of [{generation:'699'},{scope:'foreign'},{incarnation:'999'},{incarnation:null}]){await p.goto(exactURL(base,ref(),extra));await ready(p);await p.getByRole('alert').waitFor();assert.equal(await draft(p).count(),0);assert((await p.getByRole('alert').innerText()).length>5);}
 for(const query of ['?view=wrong','?view=chat&view=scene','?tab=posts&post=unknown','?office=%2Fnot-loaded']){await p.goto(base+'/space/'+query);await ready(p);await p.getByRole('alert').waitFor();assert.equal(await p.locator('.post-detail').count(),0);}
 f.state.rows=[row(ref('mora','999')),row(ref('sira','102'))];await p.goto(exactURL(base));await ready(p);await p.getByRole('alert').waitFor();assert.equal(await draft(p).count(),0);await office(p,'/empty');assert(!params(p).has('agent'));f.state.rows.push(row());await until('refresh after supersede',()=>f.traffic.filter(r=>new URL(r.url).pathname==='/api/v1/management').length>8);assert.equal(await draft(p).count(),0);assert.equal(params(p).get('office'),'/empty');await capture(p,'R6-superseded');assert(f.traffic.every(r=>r.method==='GET'));
}));
await check('R7-R8-iframe-drafts-context-responsive',simulated(async({p,c,f})=>{
 f.state.allowUpload=true;
 await c.route('**/routes-verification-parent',r=>r.fulfill({contentType:'text/html',body:'<!doctype html><html><body style="margin:0"><iframe id="space" style="display:block;border:0;width:100vw;height:100vh"></iframe><script>window.results=[];window.addEventListener("message",e=>{if(e.source===document.querySelector("iframe").contentWindow)results.push(e.data)});</script></body></html>'}));
 await p.goto(base+'/routes-verification-parent');await p.locator('iframe').evaluate((el,url)=>el.src=url,exactURL(base,ref(),{layout:'sidebar',host_hint:'embedded'}));
 const frame=await untilFrame();
 async function untilFrame(){let frame;await until('iframe complete App',()=>{frame=p.frames().find(f=>f.url().includes('/space/'));return !!frame;});return frame;}
 await draft(frame).waitFor();
 let seq=0;async function bridge(type,fields={}){const requestId='routes-'+(++seq);await p.evaluate(({requestId,type,fields})=>document.querySelector('iframe').contentWindow.postMessage({protocol:'agentx.context.v1',requestId,type,...fields},'*'),{requestId,type,fields});let value;await until(type+' reply',async()=>{value=await p.evaluate(id=>window.results.find(r=>r.requestId===id),requestId);return !!value;});return value;}
 const a=await bridge('getTarget');assert.equal(a.status,'selected');assert.deepEqual(a.target.agent,ref());
 await draft(frame).fill('A exact draft');await frame.getByLabel('选择附件',{exact:true}).setInputFiles({name:'kept.txt',mimeType:'text/plain',buffer:Buffer.from('kept')});await frame.locator('.composer-attachments summary').click();await button(frame,'移除 kept.txt').waitFor();
 await button(frame,'返回总览').click();await person(frame,'sira');await draft(frame).fill('B exact draft');await button(frame,'返回总览').click();await person(frame,'mora');assert.equal(await draft(frame).inputValue(),'A exact draft');assert.equal(await frame.locator('.composer-attachments').count(),1);
 const stale=await bridge('addContext',{target:a.target,selectionRevision:a.selectionRevision,context:{text:'STALE-CONTEXT-MUST-NOT-ENTER'}});assert.equal(stale.status,'error');assert.equal(await draft(frame).inputValue(),'A exact draft');const selected=await bridge('getTarget');assert.notEqual(selected.selectionRevision,a.selectionRevision);
 const added=await bridge('addContext',{target:selected.target,selectionRevision:selected.selectionRevision,context:{text:'fresh parent context'}});assert.equal(added.status,'added');assert((await draft(frame).inputValue()).includes('fresh parent context'));
 const before=frame.url();await frame.evaluate(()=>history.back());await until('history close',()=>!new URL(frame.url()).searchParams.has('agent'));await frame.evaluate(()=>history.forward());await draft(frame).waitFor();assert.equal(frame.url(),before);assert((await draft(frame).inputValue()).includes('A exact draft'));assert.equal(await frame.locator('.composer-attachments').count(),1);
 for(const viewport of [{width:390,height:800},{width:390,height:480}]){
  await p.setViewportSize(viewport);await draft(frame).waitFor();const geometry=await frame.evaluate(()=>({width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth}));assert(geometry.scrollWidth<=geometry.width+1,JSON.stringify(geometry));
  for(const item of [draft(frame),button(frame,'发送 AgentX 消息'),button(frame,'返回总览')]){const box=await item.boundingBox();assert(box&&box.x>=-1&&box.y>=-1&&box.x+box.width<=viewport.width+1&&box.y+box.height<=viewport.height+1,JSON.stringify({viewport,box}));await item.click({trial:true});}
  await capture(p,'R8-iframe-'+viewport.height,{frameURL:frame.url(),geometry,frameSemantic:await frame.locator('body').ariaSnapshot()});
 }
 assert.equal(new URL(frame.url()).searchParams.get('layout'),'sidebar');assert(f.traffic.filter(r=>r.method!=='GET').every(r=>new URL(r.url).pathname==='/api/v1/attachments'));assert.equal(f.traffic.filter(r=>r.method==='POST').length,1);await save('R7-bridge',{initial:a,current:selected,stale,added,traffic:f.traffic});
}));
await check('R9-native-stopped-status-and-scene-ui',simulated(async({p,c,f})=>{
 await p.goto(exactURL(base,ref('mora',null,'99')));await ready(p);await p.getByRole('heading',{name:/@mora/}).waitFor();assert((await p.locator('.hall-conversation-heading').innerText()).includes('停止历史'));assert.equal(params(p).get('stopped_event'),'99');assert.equal(params(p).get('incarnation'),null);
 await roundtrip(c,p,async q=>{assert((await q.locator('.hall-conversation-heading').innerText()).includes('停止历史'));assert.equal(params(q).get('stopped_event'),'99');});
 await button(p,'模型记录').click();assert.equal(params(p).get('channel'),'model');await roundtrip(c,p,async q=>{assert(await q.getByText('请主动读取 Provider 记录',{exact:true}).isVisible());});
 await button(p,'返回总览').click();await tab(p,'成员');const length=await p.evaluate(()=>history.length);await p.getByLabel('状态筛选',{exact:true}).selectOption('idle');assert.equal(params(p).get('state'),'idle');assert.equal(await p.evaluate(()=>history.length),length);await roundtrip(c,p,async q=>assert.equal(await q.getByLabel('状态筛选',{exact:true}).inputValue(),'idle'));await p.getByLabel('状态筛选',{exact:true}).selectOption('');
 await button(p,'场景').click();await button(p,'名册').click();assert.equal(params(p).get('panel'),'directory');await button(p,'关闭名册').click();await button(p,'@bigboss 人类通信端').click();assert.equal(params(p).get('panel'),'mailbox');await p.getByText('Latest simulated letter',{exact:true}).waitFor();await button(p,'关闭人类消息').click();assert(!params(p).has('panel'));
 await p.getByRole('button',{name:'@mora · 等待消息 · AgentX 原生 API',exact:true}).click();await draft(p).waitFor();await draft(p).fill('Short scene draft');await p.setViewportSize({width:390,height:480});
 const box=await draft(p).boundingBox();assert(box&&box.x>=0&&box.y>=0&&box.x+box.width<=391&&box.y+box.height<=481,JSON.stringify(box));await button(p,'发送 AgentX 消息').click({trial:true});await capture(p,'R9-short-scene');assert(f.traffic.every(r=>r.method==='GET'));
}));
} finally {await browser.close();await save('results',{base,results,errors});}
if(results.some(r=>r.status!=='passed')||errors.length)process.exitCode=1;
