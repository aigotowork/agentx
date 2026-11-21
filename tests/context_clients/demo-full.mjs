/** Corrected real floating wrapper: full native Space alone owns navigation/files. */
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile} from 'node:fs/promises';
import {chromium,config,browserExecutable,save,until,messages,draft,capture,deliberate} from './common.mjs';
const browser=await chromium.launch({executablePath:browserExecutable,headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server']});
const context=await browser.newContext({viewport:{width:1200,height:900},locale:'zh-CN'});const page=await context.newPage();page.setDefaultTimeout(20000);
const traffic=[],errors=[];let stage='launch',frame;
page.on('request',request=>{if(request.url().includes('/api/'))traffic.push({method:request.method(),url:request.url(),parent:request.frame()===page.mainFrame()});});
page.on('pageerror',error=>errors.push(error.message));await context.tracing.start({screenshots:true,snapshots:true,sources:true});
async function choose(ref){await frame.locator('.hall-directory-person').filter({hasText:ref.name}).click();await draft(frame).waitFor();}
async function back(){await frame.getByRole('button',{name:'返回总览',exact:true}).click();}
try{
  const source={};for(const file of ['server.mjs','public/index.html','public/app.js','public/bridge.js','public/scenario.js','public/style.css','public/layouts/workspace.css'])source[file]=createHash('sha256').update(await readFile('web/context-demo/'+file)).digest('hex');await save('demo-source',source);
  await page.goto(config.demo);assert.equal(await page.locator('#assistant').isVisible(),false);
  assert.equal(await page.locator('#target,#attachments,#add-files').count(),0);
  await page.locator('#launcher').click();frame=await until('same full Space',async()=>page.frames().find(frame=>frame.url()===config.url+'/space/'));
  await frame.getByRole('button',{name:'聊天',exact:true}).waitFor();await until('bridge ready',async()=>/Space.*就绪|已连接|可|完整 Space/.test(await page.locator('#bridge-status').innerText()));
  stage='unselected-capture';await page.locator('#ask-summary').click();
  await page.locator('#capture-summary').waitFor({state:'visible'});await until('bind action available',async()=>!(await page.locator('#rebind-context').isDisabled()));
  await page.locator('#rebind-context').click();await until('visible unavailable selection',async()=>/选择|打开|保留/.test(await page.locator('#bridge-status').innerText()));
  assert.equal(await page.locator('#add-context').isDisabled(),true);assert.equal(traffic.filter(row=>row.method==='POST').length,0);
  stage='native-selection';await frame.getByRole('button',{name:'聊天',exact:true}).click();await choose(config.a);
  await draft(frame).fill('CS02 retained full-Space A draft');const documentURL=frame.url();
  await page.locator('#close-assistant').click();assert.equal(await page.locator('#launcher').evaluate(element=>element===document.activeElement),true);
  await page.keyboard.press('Enter');await page.locator('#assistant').waitFor({state:'visible'});assert.equal(frame.url(),documentURL);assert.equal(await draft(frame).inputValue(),'CS02 retained full-Space A draft');
  await page.locator('#close-assistant').focus();await page.keyboard.press('Escape');assert.equal(await page.locator('#assistant').isVisible(),false);await page.keyboard.press('Enter');
  await page.locator('#rebind-context').click();await until('bound explicit A capture',async()=>!(await page.locator('#add-context').isDisabled()));await page.locator('#add-context').click();
  await until('real contextual order draft',async()=>(await draft(frame).inputValue()).includes('AX-20261115-0182'));
  await frame.getByLabel('选择附件',{exact:true}).setInputFiles([{name:'同一Space截图.png',mimeType:'image/png',buffer:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=','base64')},{name:'原生附件.txt',mimeType:'text/plain',buffer:Buffer.from('full Space native attachment sentinel')}]);
  await until('two native pending attachments',async()=>await frame.locator('.hall-composer').getByRole('button',{name:/^移除 /}).count()===2);
  assert.equal((await messages()).length,0);await capture(page,'CS02-demo-pending','Corrected wrapper hosts full native Space, binds exact current member and uses native attachment intake; draft only.',frame);
  stage='binding-change';await page.locator('#ask-reply').click();await until('capture bound A',async()=>!(await page.locator('#add-context').isDisabled()));
  await back();await choose(config.b);await draft(frame).fill('B-only draft');await page.locator('#add-context').click();
  await until('old binding rejected',async()=>/未确认添加|选择已改变/.test(await page.locator('#bridge-status').innerText()));
  assert.equal(await draft(frame).inputValue(),'B-only draft');assert.equal(await page.locator('#capture-summary').isVisible(),true);
  await page.locator('#rebind-context').click();await until('explicit B rebind',async()=>!(await page.locator('#add-context').isDisabled()));await page.locator('#add-context').click();
  await until('explicit B addition',async()=>(await draft(frame).inputValue()).length>'B-only draft'.length);assert.equal((await messages(config.b)).length,0);
  await back();await choose(config.a);assert((await draft(frame).inputValue()).includes('CS02 retained full-Space A draft'));
  stage='deliberate';await deliberate(page,frame,'CS02-demo-deliberate',2);
  stage='narrow';await page.setViewportSize({width:390,height:800});const bounds=await page.locator('#assistant').boundingBox();assert(bounds.x>=0&&bounds.x+bounds.width<=391,JSON.stringify(bounds));
  await frame.getByRole('button',{name:'场景',exact:true}).click();await frame.getByRole('button',{name:'聊天',exact:true}).click();await draft(frame).waitFor();
  await capture(page,'CS02-demo-narrow','390px floating host preserves canonical full-Space scene/chat/member/conversation controls.',frame);
  assert.equal(traffic.filter(row=>row.parent&&row.url.includes('/api/discovery')).length,0);
  await save('demo-full-result',{status:'pass',source,traffic,errors,limits:['Parent Escape applies when focus is in parent; child cannot bubble across origins','Physical clipboard/drop not exercised','Service origin is runner configuration, never an arbitrary browser proxy']});
}catch(error){await save('demo-full-failure',{stage,error:error.stack,traffic,errors,body:await page.locator('body').innerText()});throw error;}
finally{await context.tracing.stop({path:`${config.evidence}/trace.zip`});await browser.close();}
