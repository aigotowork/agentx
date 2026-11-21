/** Current complete-Space iframe, geometry/continuity and exact-selection fence cases. */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {chromium,config,browserExecutable,save,until,messages,draft,capture} from './common.mjs';
const browser=await chromium.launch({executablePath:browserExecutable,headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server']});
const context=await browser.newContext({viewport:{width:1200,height:900},locale:'zh-CN'});
const page=await context.newPage();page.setDefaultTimeout(20000);
const traffic=[],steps=[],errors=[];let stage='open',frame,release;
page.on('pageerror',error=>errors.push(error.message));
page.on('request',request=>{if(request.url().includes('/api/'))traffic.push({method:request.method(),url:request.url()});});
await context.tracing.start({screenshots:true,snapshots:true,sources:true});
async function request(type,body={}){
  const requestId=randomUUID();
  await page.evaluate(({type,body,requestId,url})=>document.querySelector('iframe').contentWindow.postMessage({protocol:'agentx.context.v1',type,requestId,...body},url),{type,body,requestId,url:config.url});
  return until('correlated '+type,()=>page.evaluate(id=>window.results.find(row=>row.requestId===id),requestId));
}
async function selected(ref=config.a){const target=await request('getTarget');assert.equal(target.status,'selected',JSON.stringify(target));assert.deepEqual(target.target,{scope:config.scope,generation:config.generation,agent:ref});return target;}
async function choose(ref){await frame.locator('.hall-directory-person').filter({hasText:ref.name}).click();await draft(frame).waitFor();return selected(ref);}
async function back(){await frame.getByRole('button',{name:'返回总览',exact:true}).click();await until('unavailable outside reader',async()=>(await request('getTarget')).status==='unavailable');}
async function checkpoint(id,meaning){await capture(page,id,meaning,frame);steps.push(id);await save('space-steps',steps);}
try{
  await page.goto(config.parent);
  await page.locator('#space').evaluate((element,url)=>element.src=url+'/space/',config.url);
  frame=await until('actual canonical frame',async()=>page.frames().find(frame=>frame.url()===config.url+'/space/'));
  await frame.getByRole('button',{name:'聊天',exact:true}).waitFor();
  assert.equal((await request('hello')).type,'ready');
  assert.equal((await request('getTarget')).status,'unavailable');
  await frame.getByRole('combobox',{name:'工作区范围',exact:true}).waitFor();
  await frame.getByRole('navigation',{name:'办公室内容',exact:true}).waitFor();
  await frame.getByRole('button',{name:'聊天',exact:true}).click();
  const initial=await choose(config.a);await draft(frame).fill('CS01 exact A retained draft');
  stage='narrow';
  for(const width of [450,390,360,320]){
    await page.locator('#space').evaluate((element,width)=>element.style.width=width+'px',width);
    await until('actual child width '+width,()=>frame.evaluate(width=>innerWidth===width,width));
    const geometry=await frame.evaluate(()=>({width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth,
      controls:[...document.querySelectorAll('.presentation-switch button,.overview-fields select,.hall-composer textarea,.hall-local-append,.reader-back')].map(element=>{const bounds=element.getBoundingClientRect();return{label:element.getAttribute('aria-label')??element.textContent,visible:bounds.width>0&&bounds.height>0,x:bounds.x,y:bounds.y,width:bounds.width,height:bounds.height};})}));
    assert(geometry.scrollWidth<=width+1,JSON.stringify(geometry));
    assert(geometry.controls.every(control=>!control.visible||control.x>=-1&&control.x+control.width<=width+1&&control.y>=-1&&control.y+control.height<=801),JSON.stringify(geometry));
    assert.equal(await draft(frame).inputValue(),'CS01 exact A retained draft');await save('CS01-geometry-'+width,geometry);
    await checkpoint('CS01-width-'+width,'Same complete Space, native office controls and selected exact reader/composer at '+width+'×800.');
  }
  stage='return-and-fences';
  await back();await choose(config.b);await draft(frame).fill('CS01 exact B retained draft');await back();await choose(config.a);
  assert.equal(await draft(frame).inputValue(),'CS01 exact A retained draft');
  const stale=await request('addContext',{target:initial.target,selectionRevision:initial.selectionRevision,context:{text:'STALE-MUST-NOT-ENTER'}});
  assert.equal(stale.status,'error');assert(!(await draft(frame).inputValue()).includes('STALE-MUST'));
  const beforePresentation=await selected();const expandedClose=frame.locator('.office-expanded-object button').filter({hasText:'×'});if(await expandedClose.count())await expandedClose.first().click();await frame.getByRole('button',{name:'场景',exact:true}).click();
  const camera=()=>frame.evaluate(()=>{const scene=document.querySelector('.office-scene'),anchor=document.querySelector('.office-human');return{zoom:getComputedStyle(scene).getPropertyValue('--office-head-rise'),x:anchor.style.getPropertyValue('--object-x'),y:anchor.style.getPropertyValue('--object-y')};});
  const cameraBefore=await camera();await frame.getByRole('button',{name:'聊天',exact:true}).click();await frame.getByRole('button',{name:'场景',exact:true}).click();
  await until('observable camera preserved',async()=>JSON.stringify(await camera())===JSON.stringify(cameraBefore));
  assert.equal((await selected()).selectionRevision,beforePresentation.selectionRevision);
  await checkpoint('CS01-scene-return','Canonical scene/list return preserves current revision, draft and observable fixed world-anchor camera coordinates.');
  await frame.getByRole('button',{name:'聊天',exact:true}).click();
  stage='held-upload-fence';
  const bound=await selected();let held=false;const gate=new Promise(resolve=>release=resolve);
  await page.route('**/api/v1/attachments?**',async route=>{if(route.request().method()!=='POST'){await route.continue();return;}held=true;await gate;await route.continue();});
  const requestId=randomUUID();
  await page.evaluate(({url,bound,requestId})=>{const file=new File(['actual held upload sentinel'],'race.txt',{type:'text/plain'});document.querySelector('iframe').contentWindow.postMessage({protocol:'agentx.context.v1',type:'addContext',requestId,target:bound.target,selectionRevision:bound.selectionRevision,context:{text:'LATE-A-MUST-NOT-COMMIT',files:[file]}},url);},{url:config.url,bound,requestId});
  await until('real upload held',async()=>held);await back();await choose(config.b);assert.equal(await draft(frame).inputValue(),'CS01 exact B retained draft');
  release();release=null;
  const result=await until('guarded original result',()=>page.evaluate(id=>window.results.find(row=>row.requestId===id),requestId));assert.equal(result.status,'error');
  assert.equal(await draft(frame).inputValue(),'CS01 exact B retained draft');await page.unroute('**/api/v1/attachments?**');
  await back();await choose(config.a);assert.equal(await draft(frame).inputValue(),'CS01 exact A retained draft');
  assert.equal((await messages()).length,0);assert.equal((await messages(config.b)).length,0);
  await checkpoint('CS02-race-fence','Real uploaded bytes finish after A→B but pre-commit guard rejects without touching either exact draft or sending.');
  await save('space-result',{status:'pass',steps,traffic,errors,stale,result,cameraBefore,limits:['Observed camera through fixed world-anchor DOM coordinates/zoom, not private React state','Native geometry/binary/receipt internals reuse accepted backend evidence','One decisive held-upload race; no exhaustive state Cartesian product']});
}catch(error){await save('space-failure',{stage,error:error.stack,traffic,errors,body:await frame?.locator('body').innerText().catch(()=>null)});throw error;}
finally{release?.();await context.tracing.stop({path:`${config.evidence}/trace.zip`});await browser.close();}
