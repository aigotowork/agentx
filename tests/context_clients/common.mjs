/** Consumer-owned browser bindings; real native effects remain publicly read. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile, writeFile} from 'node:fs/promises';
import {checkpoint} from '../collaboration_workbench/browser_checkpoint.mjs';
const require=createRequire(import.meta.url);
export const {chromium}=require(process.env.AGENTX_PLAYWRIGHT_PACKAGE??'/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
export const config=JSON.parse(await readFile(process.argv[2],'utf8'));
assert.equal(new URL(config.url).hostname,'127.0.0.1');
assert(![18959,18967,18971,38023,38024].includes(Number(new URL(config.url).port)));
export const browserExecutable=process.env.AGENTX_BROWSER_EXECUTABLE??'/home/long/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
export async function until(label,read,timeout=30000){const end=Date.now()+timeout;while(Date.now()<end){const value=await read();if(value)return value;await new Promise(resolve=>setTimeout(resolve,100));}throw Error(label+' timed out');}
export async function save(name,value){await writeFile(`${config.evidence}/${name}.json`,JSON.stringify(value,null,2)+'\n');}
export async function api(path,query={}){const url=new URL(path,config.url);for(const[k,v]of Object.entries({generation:config.generation,...query}))if(v!=null)url.searchParams.set(k,v);const response=await fetch(url);assert.equal(response.status,200,await response.clone().text());return response.json();}
export async function messages(agent=config.a){return(await api(`/api/v1/agents/${agent.name}/messages`,agent)).messages;}
export const draft=frame=>frame.getByRole('textbox',{name:'Agent 消息草稿',exact:true});
export async function capture(page,id,meaning,frame){
  const state=frame?{url:frame.url(),draft:await draft(frame).inputValue().catch(()=>null),semantic:await frame.locator('body').ariaSnapshot()}:null;
  if(frame)await frame.locator('body').screenshot({path:`${config.evidence}/${id}-child.png`});
  await checkpoint(page,config.evidence,id,meaning,state);
}
export async function deliberate(page,frame,id,attachmentCount){
  const send=frame.getByRole('button',{name:'发送 AgentX 消息',exact:true});
  await until('ready deliberate send',async()=>!(await send.isDisabled()));
  const pending=page.waitForRequest(r=>r.method()==='POST'&&new URL(r.url()).pathname.endsWith('/messages'));
  await send.click();const body=(await pending).postDataJSON();
  assert.deepEqual(body.agent,config.a);assert.equal(body.message.attachments.length,attachmentCount);
  const operation=await until('original operation receipt',async()=>{const op=await api('/api/v1/operations/'+body.request_id);assert(!['Failed','Unknown'].includes(op.status),JSON.stringify(op));return op.status==='Succeeded'?op:null;});
  const event=await until('native message',async()=>(await messages()).find(row=>row.id===operation.result.Sent.message_id));
  assert.equal(event.attachments.length,attachmentCount);
  await save(id,{body,operation,event});return event;
}
