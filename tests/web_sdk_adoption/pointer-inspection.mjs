import assert from 'node:assert/strict';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
const c=JSON.parse(await readFile(process.argv[2],'utf8'));await mkdir(c.evidence,{recursive:false});
const {chromium}=createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE??'/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const b=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server']});
const ctx=await b.newContext({viewport:{width:1440,height:900}}),page=await ctx.newPage(),checks=[],errors=[];let failure;
page.on('pageerror',e=>errors.push(e.message));
const sources=()=>page.evaluate(()=>window.dancingai.inspection.sources().filter(s=>s.id.startsWith('agent-space:')).map(s=>s.id));
try{
 await page.goto(c.url+'/space/');
 await page.waitForFunction(()=>window.dancingai?.inspection?.sources().some(s=>s.id.startsWith('agent-space:')));
 const initial=await sources();assert(initial.length);checks.push({name:'provider ready with floating tools suspended',sources:initial});
 const toggle=page.getByRole('button',{name:'检查页面',exact:true});
 await toggle.click();assert.equal(await toggle.getAttribute('aria-pressed'),'true');await page.screenshot({path:c.evidence+'/inspection-open.png'});
 await page.getByRole('button',{name:'收起页面检查',exact:true}).click();assert.equal(await toggle.getAttribute('aria-pressed'),'false');assert.deepEqual(await sources(),initial);checks.push({name:'public open/suspend retains provider lease'});
 await page.setViewportSize({width:390,height:844});await toggle.click();await page.getByRole('button',{name:'收起页面检查',exact:true}).click();assert.equal(await toggle.getAttribute('aria-pressed'),'false');checks.push({name:'narrow ordinary pointer reopen/dismiss'});await page.setViewportSize({width:1440,height:900});
 await page.getByRole('button',{name:'聊天',exact:true}).click();await page.waitForFunction(()=>!window.dancingai.inspection.sources().some(s=>s.id.startsWith('agent-space:')));
 await page.getByRole('button',{name:'场景',exact:true}).click();await page.waitForFunction(()=>window.dancingai.inspection.sources().some(s=>s.id.startsWith('agent-space:')));checks.push({name:'chat disposal and scene remount renew discovery',sources:await sources()});
 await page.getByRole('button',{name:'聊天',exact:true}).click();
 await page.getByRole('combobox',{name:'工作区范围',exact:true}).selectOption('');
 await page.getByRole('navigation',{name:'办公室内容'}).getByRole('button',{name:/^成员/}).click();
 await page.getByRole('region',{name:'Agent 名册'}).getByRole('button',{name:new RegExp('^@'+c.agent.name+' ·')}).click();
 const composer=page.getByRole('textbox',{name:'Agent 消息草稿',exact:true});
 for(const width of [1440,390]){
  await page.setViewportSize({width,height:900});await composer.fill('normal pointer acceptance width '+width);
  const response=page.waitForResponse(r=>r.request().method()==='POST'&&new URL(r.url()).pathname.endsWith('/messages'));
  await page.getByRole('button',{name:'发送 AgentX 消息',exact:true}).click();const res=await response;assert.equal(res.status(),202);const accepted=await res.json();
  await page.waitForFunction(id=>JSON.parse(localStorage.getItem('agentx.space.operations.v1')??'[]').find(r=>r.id===id)?.operation?.status==='Succeeded',accepted.request_id);
  await page.waitForFunction(()=>document.querySelector('textarea[aria-label="Agent 消息草稿"]').value==='');
  checks.push({name:'normal pointer send clears only confirmed draft',width,accepted});await page.screenshot({path:`${c.evidence}/pointer-${width}.png`});
 }
 await page.reload();await page.getByRole('button',{name:'检查页面',exact:true}).waitFor();assert.equal(await page.getByRole('button',{name:'检查页面',exact:true}).getAttribute('aria-pressed'),'false');checks.push({name:'reload begins with inspection suspended'});assert.deepEqual(errors,[]);
}catch(e){failure={message:e.message,stack:e.stack};}
finally{await writeFile(c.evidence+'/result.json',JSON.stringify({status:failure?'failed':'passed',checks,errors,failure,browser:b.version()},null,2)+'\n');await b.close();}
if(failure)throw new Error(failure.message);
