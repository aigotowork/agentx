/** Packaged MV3 acceptance in the genuine browser side panel and active-tab host. */
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {chromium,config,browserExecutable,save,until,messages,draft,capture,deliberate} from './common.mjs';
const server=createServer((_req,res)=>{res.setHeader('Content-Type','text/html;charset=utf-8');res.end('<!doctype html><html lang="zh-CN"><title>真实浏览器采集夹具</title><body style="background:#e8f4ff;font:24px sans-serif"><h1>订单 AX-ACCEPTANCE</h1><p id="selected">SELECTED-CONTEXT-SENTINEL 订单商品受损，需要核对售后政策。</p><p>PAGE-CONTEXT-SENTINEL 已签收一件，另一件等待补发。</p></body></html>');});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const origin=`http://127.0.0.1:${server.address().port}`;
const browser=await chromium.launchPersistentContext(config.profile,{executablePath:browserExecutable,headless:false,ignoreDefaultArgs:['--disable-extensions'],args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server',`--disable-extensions-except=${config.extension}`,`--load-extension=${config.extension}`],viewport:{width:1200,height:860},locale:'zh-CN'});
const traffic=[],errors=[],dialogs=[];let stage='load',panel,tab;
browser.on('page',page=>{page.on('pageerror',e=>errors.push(e.message));});
browser.on('request',request=>{if(request.url().includes('/api/'))traffic.push({method:request.method(),url:request.url()});});
await browser.tracing.start({screenshots:true,snapshots:true,sources:true});
function keys(...names){execFileSync('python3',['tests/context_clients/x11_keys.py',...names],{env:process.env});}
try{
  const manifest=JSON.parse(await readFile(config.extension+'/manifest.json','utf8'));
  const worker=browser.serviceWorkers()[0]??await browser.waitForEvent('serviceworker');
  const extensionId=new URL(worker.url()).host;
  tab=await browser.newPage();await tab.goto(origin);await tab.bringToFront();
  const baseline=(await messages()).length;
  keys('Alt_L','Shift_L','a');
  panel=await until('actual action-opened side panel',async()=>browser.pages().find(page=>page.url()===`chrome-extension://${extensionId}/panel.html`));
  panel.setDefaultTimeout(18000);
  await save('extension-host',{version:browser.browser()?.version(),extensionId,worker:worker.url(),panel:panel.url(),manifest,method:'Actual X11 Alt+Shift+A invokes _execute_action in fresh owned Xvfb browser',pages:browser.pages().map(page=>page.url())});
  stage='connect';
  await panel.locator('#settings').evaluate(element=>element.open=true);
  await panel.locator('#service').fill(config.url);
  await panel.locator('#connect').click();
  // Optional-host permission prompt belongs to browser chrome; Enter is an actual X11 key.
  keys('Return');
  await until('real native discovery options',async()=>{
    const text=await panel.locator('#status').innerText();dialogs.push(text);
    return await panel.locator('#target option').count()>2;
  });
  const value=await panel.locator('#target option').evaluateAll((options,ref)=>options.find(option=>option.textContent.includes(ref.name))?.value,config.a);
  assert(value,'exact native target');await panel.locator('#target').selectOption(value);
  const frame=await until('actual embedded AgentX',async()=>panel.frames().find(frame=>frame.url().startsWith(config.url+'/space/')));
  await draft(frame).waitFor();await until('capture ready',async()=>!(await panel.locator('#selection').isDisabled()));
  assert.equal((await messages()).length,baseline);
  await capture(panel,'CC05-real-panel','Actual action-opened MV3 sidePanel, native discovery and accepted AgentX iframe.',frame);
  stage='selection';
  await tab.evaluate(()=>{const range=document.createRange();range.selectNodeContents(document.querySelector('#selected'));const selection=getSelection();selection.removeAllRanges();selection.addRange(range);});
  await panel.locator('#selection').click();
  await until('actual scripting selection in draft',async()=>(await draft(frame).inputValue()).includes('SELECTED-CONTEXT-SENTINEL'));
  assert(!(await draft(frame).inputValue()).includes('PAGE-CONTEXT-SENTINEL'));
  await until('page capture available',async()=>!(await panel.locator('#page').isDisabled()));await panel.locator('#page').click();
  await until('actual scripting page in draft',async()=>(await draft(frame).inputValue()).includes('PAGE-CONTEXT-SENTINEL'));
  stage='screenshot';
  await until('screenshot available',async()=>!(await panel.locator('#screenshot').isDisabled()));await panel.locator('#screenshot').click();
  const composer=frame.locator('.hall-composer');
  await until('actual reconstructed screenshot attachment',async()=>{
    const status=await panel.locator('#status').innerText();
    if(await panel.locator('.feedback-block').getAttribute('data-error')==='true')throw Error(status);
    return await composer.getByRole('button',{name:/^移除 page-.*\.png$/}).count()===1;
  });
  stage='file';
  const chooser=panel.waitForEvent('filechooser');await panel.locator('#attach').click();await(await chooser).setFiles({name:'browser-context.txt',mimeType:'text/plain',buffer:Buffer.from('extension real local file sentinel')});
  await until('local file draft',async()=>await composer.getByRole('button',{name:'移除 browser-context.txt',exact:true}).count()===1);
  assert.equal((await messages()).length,baseline);
  await capture(panel,'CC06-pending','Actual Chrome scripting/visible-tab screenshot and selected local file reconstructed into iframe draft; no automatic message.',frame);
  const event=await deliberate(panel,frame,'CC06-deliberate',2);
  const image=event.attachments.find(item=>item.media_type==='image/png');assert(image);
  const response=await fetch(new URL(image.url,config.url));assert.equal(response.status,200);const bytes=Buffer.from(await response.arrayBuffer());
  assert.equal(bytes.subarray(1,4).toString(),'PNG');const width=bytes.readUInt32BE(16),height=bytes.readUInt32BE(20);assert(width>200&&height>200);
  await save('CC06-screenshot',{attachment:image,width,height,size:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex'),source:origin});
  stage='restricted';await tab.goto('chrome://version/');
  await until('capture ready after send',async()=>!(await panel.locator('#selection').isDisabled()));await panel.locator('#selection').click();
  await until('restricted-page visible error',async()=>await panel.locator('.feedback-block').getAttribute('data-error')==='true');
  assert.match(await panel.locator('#status').innerText(),/页面|采集|授权|权限/);
  await capture(panel,'CC07-restricted','Actual restricted browser page produces recoverable visible capture error.',frame);
  assert.equal((await messages(config.b)).length,0);
  await save('extension-result',{status:'pass',extensionId,manifest,traffic,errors,dialogs,limits:['Chrome-for-Testing Linux; Edge not executed','Selection created through DOM Range, not physical drag','Screenshot uses real captureVisibleTab; no OS clipboard claim']});
}catch(error){const session=await browser.newCDPSession(tab??browser.pages()[0]);const targets=await session.send('Target.getTargets');await save('extension-failure',{stage,error:error.stack,traffic,errors,dialogs,targets,pages:browser.pages().map(page=>page.url()),panelText:await panel?.locator('body').innerText().catch(()=>null)});throw error;}
finally{await browser.tracing.stop({path:`${config.evidence}/trace.zip`});await browser.close();await new Promise(resolve=>server.close(resolve));}
