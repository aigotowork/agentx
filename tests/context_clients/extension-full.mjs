/** Corrected packaged full-Space consumer in the actual MV3 sidePanel host. */
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {createHash} from 'node:crypto';
import {writeFile} from 'node:fs/promises';
import {dirname,join} from 'node:path';
import {execFileSync} from 'node:child_process';
import {config,save,until,api,messages} from './common.mjs';
import {openExtensionHost} from './extension_host.mjs';
import {extensionCheckpoint} from './extension_checkpoint.mjs';
const server=createServer((_request,response)=>{response.setHeader('Content-Type','text/html;charset=utf-8');response.end('<!doctype html><title>真实网页上下文夹具</title><body style="background:#eaf5ff;font:24px sans-serif"><h1>订单 AX-EXTENSION</h1><p id="selection">SELECTED-CONTEXT-SENTINEL 需要核对售后政策。</p><p>PAGE-CONTEXT-SENTINEL 商品分包未齐。</p></body>');});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const sourceOrigin=`http://127.0.0.1:${server.address().port}`;
let host,panel,frame,release,stage='host';const traffic=[],errors=[];
const composer='.hall-composer';const draftSelector='textarea[aria-label="Agent 消息草稿"]';
const text=()=>frame.$eval(draftSelector,element=>element.value);
async function click(selector){await frame.waitForSelector(selector,{visible:true});await frame.click(selector);}
async function mode(kind){await click(`.presentation-switch button:nth-child(${kind==='scene'?1:2})`);}
async function choose(ref){
  await frame.waitForSelector('.hall-directory-person',{visible:true});
  const handles=await frame.$$('.hall-directory-person');let found;
  for(const handle of handles)if((await handle.evaluate(element=>element.textContent)).includes(ref.name)){found=handle;break;}
  assert(found,'native roster exact member '+ref.name);await found.click();await frame.waitForSelector(draftSelector,{visible:true});
}
async function setDraft(value){await frame.locator(draftSelector).fill(value);}
async function capture(id,meaning){
  const state=frame?{url:frame.url(),draft:await text().catch(()=>null),body:await frame.$eval('body',element=>element.innerText)}:null;
  if(frame){const element=await frame.$('body');await element.screenshot({path:`${config.evidence}/${id}-child.png`});}
  await extensionCheckpoint(panel,config.evidence,id,meaning,state);
}
try{
  host=await openExtensionHost(config,sourceOrigin);panel=host.panel;panel.setDefaultTimeout(20000);
  const tabsBefore=await host.worker.evaluate(()=>chrome.tabs.query({active:true,currentWindow:true}));await save('tabs-before-connect',tabsBefore);
  panel.on('pageerror',error=>errors.push(error.message));
  panel.on('request',request=>{if(request.url().includes('/api/'))traffic.push({method:request.method(),url:request.url()});});
  assert.equal((await panel.$$('#target,#files,#attach')).length,0);
  stage='service-connect-no-host-grant';
  await panel.$eval('#settings',element=>element.open=true);await panel.locator('#service').fill(config.url);await panel.click('#connect');
  await until('actual full iframe without host grant',async()=>await panel.$eval('#conversation',element=>element.src.endsWith('/space/')&&!element.hidden),45000);
  await save('extension-permissions',{actual:await host.worker.evaluate(()=>chrome.permissions.getAll()),selectedService:config.url,source:sourceOrigin,note:'1.2.0 intentionally has no optional host permission; iframe uses frame-src and page capture uses activeTab/scripting.'});
  await save('tabs-after-connect',await host.worker.evaluate(()=>chrome.tabs.query({active:true,currentWindow:true})));
  frame=await until('actual full native frame',async()=>panel.frames().find(frame=>frame.url()===config.url+'/space/'));
  await frame.waitForSelector('.presentation-switch',{visible:true});await until('bridge ready',async()=>!(await panel.$eval('#selection',element=>element.disabled)));
  stage='no-selection';await panel.click('#selection');await until('no-selection visible rejection',async()=>await panel.$eval('.feedback-block',element=>element.dataset.error==='true'));
  assert.match(await panel.$eval('#status',element=>element.innerText),/Space|选择|会话/);
  assert.equal((await messages()).length,0);
  await mode('chat');await choose(config.a);await capture('CS03-real-full-panel','Genuine packaged sidePanel with SAME complete native Space and native member selection; parent duplicates removed.');
  stage='selection-page';
  await host.source.bringToFront();await save('tabs-before-capture',await host.worker.evaluate(()=>chrome.tabs.query({active:true,currentWindow:true})));
  await host.source.evaluate(()=>{const range=document.createRange();range.selectNodeContents(document.querySelector('#selection'));const selection=getSelection();selection.removeAllRanges();selection.addRange(range);});
  await panel.click('#selection');await until('real selected text in child',async()=>(await text()).includes('SELECTED-CONTEXT-SENTINEL'));
  assert(!(await text()).includes('PAGE-CONTEXT-SENTINEL'));
  await until('page available',async()=>!(await panel.$eval('#page',element=>element.disabled)));await panel.click('#page');
  await until('real full page in child',async()=>(await text()).includes('PAGE-CONTEXT-SENTINEL'));
  stage='real-screenshot';await until('screenshot available',async()=>!(await panel.$eval('#screenshot',element=>element.disabled)));await panel.click('#screenshot');
  await until('actual screenshot File in iframe',async()=>{const error=await panel.$eval('.feedback-block',element=>element.dataset.error==='true');if(error)throw Error(await panel.$eval('#status',element=>element.innerText));return(await frame.$$(`${composer} button[aria-label^="移除 page-"]`)).length===1;});
  const localFile=join(dirname(config.profile),'native-extension-file.txt');await writeFile(localFile,'full Space extension native file sentinel');
  const input=await frame.$('input[aria-label="选择附件"]');await input.uploadFile(localFile);
  await until('native local file attached',async()=>(await frame.$$(`${composer} button[aria-label="移除 native-extension-file.txt"]`)).length===1);
  assert.equal((await messages()).length,0);await capture('CS03-real-capture-draft','Real scripting selection/page and captureVisibleTab screenshot reconstruct into native child draft; child owns local attachment input; no automatic send.');
  stage='native-send';
  const send='button[aria-label="发送 AgentX 消息"]';await until('deliberate native send ready',async()=>!(await frame.$eval(send,element=>element.disabled)));
  const pending=panel.waitForRequest(request=>request.method()==='POST'&&new URL(request.url()).pathname.endsWith('/messages'));
  await click(send);const request=await pending,body=JSON.parse(request.postData());assert.deepEqual(body.agent,config.a);assert.equal(body.message.attachments.length,2);
  const operation=await until('native original operation receipt',async()=>{const result=await api('/api/v1/operations/'+body.request_id);assert(!['Failed','Unknown'].includes(result.status),JSON.stringify(result));return result.status==='Succeeded'?result:null;});
  const event=await until('native durable message',async()=>(await messages()).find(message=>message.id===operation.result.Sent.message_id));assert.equal(event.attachments.length,2);
  const image=event.attachments.find(item=>item.media_type==='image/png');assert(image);const response=await fetch(new URL(image.url,config.url));assert.equal(response.status,200);const bytes=Buffer.from(await response.arrayBuffer());assert.equal(bytes.subarray(1,4).toString(),'PNG');
  const width=bytes.readUInt32BE(16),height=bytes.readUInt32BE(20);assert(width>200&&height>200);
  await save('CS03-native-deliberate',{body,operation,event,screenshot:{width,height,size:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex')}});
  stage='restricted';const restricted=await host.context.newPage();await restricted.goto('chrome://version/');await restricted.bringToFront();await until('capture control ready',async()=>!(await panel.$eval('#selection',element=>element.disabled)));await panel.click('#selection');
  await until('restricted-page visible failure',async()=>await panel.$eval('.feedback-block',element=>element.dataset.error==='true'));assert.match(await panel.$eval('#status',element=>element.innerText),/页面|采集|授权|权限/);await capture('CS03-restricted','Actual browser restricted page produces recoverable capture failure with no automatic message.');
  stage='service-change';await restricted.close();await host.source.bringToFront();
  const back=await frame.$('button[aria-label="返回总览"]');if(back)await back.click();await choose(config.a);await until('prior native operation reconciled before new capture',async()=>{const status=await frame.$eval('.composer-toolbar',element=>element.innerText);return !/原操作未确定|提交中/.test(status);});
  let held=false;const gate=new Promise(resolve=>release=resolve);
  await panel.setRequestInterception(true);
  panel.on('request',async request=>{if(request.isInterceptResolutionHandled())return;if(request.method()==='POST'&&request.url().includes('/api/v1/attachments?')){held=true;await gate;}if(!request.isInterceptResolutionHandled())await request.continue().catch(()=>{});});
  await panel.click('#screenshot');await until('actual old-service screenshot upload held',async()=>held);
  await panel.$eval('#settings',element=>element.open=true);await panel.locator('#service').fill('http://127.0.0.1:9');await panel.click('#connect');
  await until('failed-capture copy retained after service change',async()=>await panel.$eval('#retained',element=>!element.hidden));release();release=null;
  assert.equal((await messages(config.b)).length,0);
  await save('CS03-service-change',{status:await panel.$eval('#status',element=>element.innerText),retained:await panel.$eval('#retained',element=>({hidden:element.hidden,text:element.innerText,image:!!element.querySelector('img')?.getAttribute('src')})),iframe:await panel.$eval('#conversation',element=>element.src),traffic});
  await save('extension-full-result',{status:'pass',traffic,errors,source:sourceOrigin,limits:['Linux Chrome-for-Testing configured browser action, not Edge/store or physical toolbar click','DOM Range selection and native CDP file input, not physical OS drag/clipboard','Real screenshot bytes/size/native effect observed independently']});
}catch(error){await save('extension-full-failure',{stage,error:error.stack,traffic,errors,panel:await panel?.$eval('body',element=>element.innerText).catch(()=>null)});throw error;}
finally{release?.();await host?.close();await new Promise(resolve=>server.close(resolve));}
