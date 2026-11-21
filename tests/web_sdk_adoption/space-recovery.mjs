import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
const c=JSON.parse(await readFile(process.argv[2],'utf8'));await mkdir(c.evidence,{recursive:false});
const {chromium}=createRequire(import.meta.url)('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');const b=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--no-proxy-server','--disable-dev-shm-usage']});const page=await b.newPage();let dropped,failure;const posts=[];
page.on('request',r=>{if(r.method()==='POST'&&new URL(r.url()).pathname.endsWith('/messages'))posts.push(r.postDataJSON());});
async function select(){await page.getByRole('button',{name:'聊天',exact:true}).click();await page.getByRole('combobox',{name:'工作区范围',exact:true}).selectOption('');await page.getByRole('navigation',{name:'办公室内容'}).getByRole('button',{name:/^成员/}).click();await page.getByRole('region',{name:'Agent 名册'}).getByRole('button',{name:new RegExp('^@'+c.agent.name+' ·')}).click();}
try{
 await page.goto(c.url+'/space/');await select();
 await page.route('**/api/v1/operations/**',r=>r.abort('failed'));
 await page.route('**/api/v1/agents/*/messages',async r=>{if(r.request().method()!=='POST')return r.continue();const body=r.request().postDataJSON();assert.equal(body.agent.incarnation,c.agent.incarnation);assert(await page.evaluate(id=>(localStorage.getItem('agentx.space.operations.v1')??'').includes(id),body.request_id));const upstream=await r.fetch();assert.equal(upstream.status(),202);dropped={body,accepted:await upstream.json()};await r.abort('failed');});
 await page.getByRole('textbox',{name:'Agent 消息草稿',exact:true}).fill('bounded real Space recovery');await page.getByRole('button',{name:'发送 AgentX 消息',exact:true}).click();await page.waitForFunction(()=>!!localStorage.getItem('agentx.space.operations.v1'));while(!dropped)await new Promise(r=>setTimeout(r,30));
 await page.unroute('**/api/v1/agents/*/messages');await page.reload();await select();const composer=page.getByRole('textbox',{name:'Agent 消息草稿',exact:true});await composer.fill('replacement must stay intact');assert(await page.getByRole('button',{name:'发送 AgentX 消息',exact:true}).isDisabled());assert(await page.evaluate(id=>(localStorage.getItem('agentx.space.operations.v1')??'').includes(id),dropped.body.request_id));
 await page.screenshot({path:c.evidence+'/pending.png'});
 await page.unroute('**/api/v1/operations/**');await page.locator('.live-receipts > summary').click();await page.locator('.live-receipts section').filter({hasText:dropped.body.request_id}).getByRole('button',{name:'查询原回执',exact:true}).click();
 await page.waitForFunction(id=>JSON.parse(localStorage.getItem('agentx.space.operations.v1')).find(r=>r.id===id)?.operation?.status==='Succeeded',dropped.body.request_id);
 assert.equal(posts.length,1);assert.equal(await composer.inputValue(),'replacement must stay intact');assert(!(await page.getByRole('button',{name:'发送 AgentX 消息',exact:true}).isDisabled()));
}catch(e){failure={message:e.message,stack:e.stack};}finally{await writeFile(c.evidence+'/result.json',JSON.stringify({status:failure?'failed':'passed',dropped,posts,failure,browser:b.version()},null,2)+'\n');await b.close();}if(failure)throw new Error(failure.message);
