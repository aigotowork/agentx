/** Focused previously unexecuted stale/current-selection cases; no happy-path replay. */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {chromium,config,browserExecutable,api,save,until,messages,draft,capture} from './common.mjs';
const browser=await chromium.launch({executablePath:browserExecutable,headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server']});
const context=await browser.newContext({viewport:{width:1100,height:850},locale:'zh-CN'});const page=await context.newPage();page.setDefaultTimeout(20000);
const traffic=[];page.on('request',r=>{if(r.method()==='POST')traffic.push({url:r.url(),body:r.postData()});});
try{
  await page.goto(config.demo);await page.locator('#launcher').click();
  await until('current exact targets',async()=>await page.locator('#target option').count()>2);
  async function select(ref){const value=await page.locator('#target option').evaluateAll((options,ref)=>options.find(option=>option.textContent.includes(ref.name))?.value,ref);assert(value);await page.locator('#target').selectOption(value);const frame=await until('exact child frame',async()=>page.frames().find(f=>f.url().includes('/space/')&&new URL(f.url()).searchParams.get('agent')===ref.name));await draft(frame).waitFor();await until('parent ready',async()=>/可添加上下文/.test(await page.locator('#bridge-status').innerText()));return frame;}
  await select(config.a);
  const body={request_id:randomUUID(),generation:config.generation,agent:config.a};
  const response=await fetch(`${config.url}/api/v1/agents/${config.a.name}/stop`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});assert.equal(response.status,202);
  const stopped=await until('native stop original receipt',async()=>{const op=await api('/api/v1/operations/'+body.request_id);assert(!['Failed','Unknown'].includes(op.status));return op.status==='Succeeded'?op:null;});
  await page.locator('#ask-summary').click();await until('stale exact-target rejection',async()=>/添加未完成|不可用|无法|不能|失败/.test(await page.locator('#bridge-status').innerText()));
  assert.equal(traffic.filter(row=>row.url.endsWith('/messages')).length,0);
  await page.locator('#refresh-targets').click();await until('stale selection removed',async()=>await page.locator('#target').inputValue()==='');
  assert.equal(await page.locator('#agent-frame').isVisible(),false);
  await capture(page,'CC03-stale','Stopped exact recipient rejects context; discovery clears selection instead of choosing current B.');
  const frame=await select(config.b);await page.locator('#ask-summary').click();await until('explicit B draft',async()=>(await draft(frame).inputValue()).length>30);
  assert.equal((await messages(config.b)).length,0);await capture(page,'CC03-explicit-B','Current B receives pending context only after explicit selection, without automatic send.',frame);
  await save('demo-fences-result',{status:'pass',stopped,traffic,limits:['Service is fixed per runner; changing AGENTX_ORIGIN requires independent runner and cannot silently retarget existing page']});
}catch(error){await save('demo-fences-failure',{error:error.stack,traffic,body:await page.locator('body').innerText()});throw error;}
finally{await browser.close();}
