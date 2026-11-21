/** Independent task-centered walkthrough of the separate experience proposal; does not invoke domain code. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const require=createRequire(import.meta.url);
const {chromium}=require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const [artifact,evidence,technical,unknownEnvelope]=process.argv.slice(2);
assert(artifact&&evidence,'Usage: node tests/flow_atlas_experience_browser.mjs prototype.html evidence-directory [technical.html] [unknown-envelope.html]');
await mkdir(evidence,{recursive:true});
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
const results=[];
const included=process.env.FLOW_ATLAS_EXPERIENCE_CASES?.split(',');
const read=page=>page.evaluate(()=>FlowAtlasWorkbench.state());
const ref=(host,kind,id)=>host.locator(`[data-ref-kind="${kind}"][data-ref-id="${id}"]`);
const closeDetails=async page=>{if(await page.getByTestId('subject-details').isVisible())await page.getByTestId('details-close').click();};
const view=(page,name)=>page.getByTestId('experience-view-'+name).click();
async function seek(page,index){await page.getByTestId('experience-checkpoint-'+index).click();assert.equal((await read(page)).presentation.index,index);}
function near(actual,expected,tolerance=0.3){assert(Math.abs(actual-expected)<tolerance,`${actual} differs from ${expected}`);}

async function capture(page,name){
  const observation=await page.evaluate(()=>{
    const bounds=node=>{if(!node)return null;const b=node.getBoundingClientRect();return {x:b.x,y:b.y,width:b.width,height:b.height};};
    return {state:FlowAtlasWorkbench.state(),viewport:{width:innerWidth,height:innerHeight},body:{width:document.documentElement.scrollWidth,height:document.documentElement.scrollHeight},regions:Object.fromEntries(['navigation','canvas','details','player'].map(id=>[id,bounds(document.getElementById(id))])),text:document.body.innerText,visibleControls:[...document.querySelectorAll('button,input,select')].filter(n=>n.getClientRects().length&&!n.closest('[hidden]')).map(n=>({id:n.id,name:n.getAttribute('aria-label')||n.textContent,testid:n.dataset.testid,disabled:n.disabled})),canvasText:[...document.querySelectorAll('#canvas svg text')].map(n=>({text:n.textContent,fontSize:getComputedStyle(n).fontSize}))};
  });
  await writeFile(join(evidence,name+'.json'),JSON.stringify(observation,null,2)+'\n');
  await page.screenshot({path:join(evidence,name+'.png')});
  return observation;
}

async function run(name,fn,{mobile=false,source=artifact}={}){
  if(included&&!included.includes(name))return;
  const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1440,height:900},isMobile:mobile,hasTouch:mobile});
  const page=await context.newPage(),errors=[],requests=[];
  page.setDefaultTimeout(7000);page.on('pageerror',e=>errors.push(e.message));
  page.on('request',r=>requests.push(r.url()));
  try{
    await page.goto(/^https?:/.test(source)?source:pathToFileURL(source).href);
    await page.getByTestId('viewport-fit').waitFor();
    await page.getByTestId('experience-view-objects').waitFor();
    await fn(page);
    assert.deepEqual(errors,[]);
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),mobile?390:1440,'No document overflow');
    assert(requests.every(url=>url.startsWith('file:')||url.startsWith('data:')||url===source),'No external resources or API');
    results.push({name,status:'passed',errors,requests});
  }catch(error){
    await capture(page,name+'-failure').catch(()=>{});
    results.push({name,status:'failed',message:error.message,stack:error.stack,errors,requests});
  }finally{await context.close();await writeFile(join(evidence,'result.json'),JSON.stringify({browser:browser.version(),results},null,2)+'\n');}
}

try{
  await run('orientation-method-return',async page=>{
    for(const name of ['objects','process','dataflow','state','sequence'])assert(await page.getByTestId('experience-view-'+name).isVisible());
    assert(await page.getByTestId('experience-nav-model').isVisible());
    assert(await page.getByTestId('experience-nav-cases').isVisible());
    assert.equal((await read(page)).scenarioId,'');
    assert.equal(await page.getByTestId('scenario-player').isVisible(),false);
    await capture(page,'01-overview');
    await page.getByTestId('experience-nav-model').click();
    await ref(page.getByTestId('resource-navigation'),'object','object.order').click();
    const canvas=page.getByTestId('workbench-canvas'),port=canvas.locator('[data-port-id="operation.confirm"][data-ref-kind="operation"]');
    assert(await port.count()>0,'Named method remains an actual canvas Port');
    await port.first().click();
    const details=page.getByTestId('subject-details');
    assert.match(await details.innerText(),/native\.order\.confirm/);
    assert.match(await details.innerText(),/Schema/);
    await capture(page,'02-method');
    assert(await page.getByTestId('experience-back').isEnabled());
    await page.getByTestId('experience-back').click();
    assert.equal((await read(page)).selection.id,'object.order');
    await closeDetails(page);
    assert.equal((await read(page)).selection.id,'object.order');
    await page.getByTestId('resource-search').fill('confirm');
    const match=ref(page.getByTestId('resource-navigation'),'operation','operation.confirm');
    await match.focus();await page.keyboard.press('Enter');
    assert.equal((await read(page)).selection.id,'operation.confirm');
    await page.keyboard.press('Escape');assert.equal(await details.isVisible(),false);
  });

  await run('parameter-views-context',async page=>{
    await ref(page.getByTestId('resource-navigation'),'object','object.order').click();
    const details=page.getByTestId('subject-details');
    await ref(details,'operation','operation.confirm').click();
    await ref(details,'step','confirm').click();
    assert.equal((await read(page)).view,'process');
    await ref(details,'binding','binding.confirm').click();
    assert.equal((await read(page)).view,'dataflow');
    assert.match(await details.innerText(),/reserve.*reserved/s);
    const binding=page.locator('path[data-binding-id="binding.confirm"]');
    assert.equal(await binding.getAttribute('data-source-step-id'),'reserve');
    assert.equal(await binding.getAttribute('data-source-outcome-id'),'reserved');
    await capture(page,'03-dataflow');
    for(const name of ['sequence','state','objects','process','dataflow']){await view(page,name);assert.equal((await read(page)).view,name);assert.equal((await read(page)).selection.id,'binding.confirm');}
    await page.getByTestId('experience-back').click();
    assert.equal((await read(page)).selection.id,'confirm');
    await closeDetails(page);
  });

  await run('scenario-clock-evidence',async page=>{
    await page.getByTestId('experience-nav-cases').click();
    assert(await ref(page.getByTestId('resource-navigation'),'scenario','scenario.lost-response').isVisible());
    await ref(page.getByTestId('resource-navigation'),'scenario','scenario.lost-response').click();
    await closeDetails(page);await view(page,'objects');
    assert.equal(await page.getByTestId('player-speed').inputValue(),'0.5');
    await page.getByTestId('player-speed').selectOption('0.25');
    await page.getByTestId('player-play').click();
    await page.waitForFunction(()=>FlowAtlasWorkbench.state().presentation.progress>0.12);
    assert.equal((await read(page)).presentation.index,0,'Motion precedes authoritative application');
    await page.getByTestId('player-play').click();
    const paused=await read(page);await view(page,'sequence');
    const retained=await read(page);assert.equal(retained.presentation.playing,false);near(retained.presentation.elapsed,paused.presentation.elapsed,0.001);
    await capture(page,'04-paused');
    await page.getByTestId('player-play').click();
    await page.waitForFunction(old=>FlowAtlasWorkbench.state().presentation.elapsed>old,paused.presentation.elapsed);
    await page.getByTestId('player-play').click();
    const last=await page.evaluate(()=>JSON.parse(document.getElementById('workbench-model').textContent).scenarios.find(s=>s.id==='scenario.lost-response').checkpoints.length-1);
    await seek(page,last);await view(page,'objects');
    const final=await read(page);
    const expected=await page.evaluate(()=>JSON.parse(document.getElementById('workbench-model').textContent).scenarios.find(s=>s.id==='scenario.lost-response').checkpoints.at(-1));
    assert.deepEqual(final.checkpoint,expected,'Proposal uses the exact authoritative checkpoint');
    assert(final.checkpoint.calls.some(c=>c.phase==='unknown'&&c.completion==='local-outcome'&&c.effectOutcomeId==='reserved'));
    assert.match(await page.getByTestId('experience-caller').innerText(),/unknown|未知/i);
    assert.match(await page.getByTestId('experience-effect').innerText(),/reserved|提交|预留/i);
    await capture(page,'05-committed-unknown');
    await page.getByTestId('player-expectations').click();await capture(page,'06-expectations');
    await closeDetails(page);await page.getByTestId('compare-toggle').click();assert.match(await page.getByTestId('subject-details').innerText(),/匹配凭据|丢失|库存不足/);
    await closeDetails(page);await page.getByTestId('diagnostics-toggle').click();
    assert.match(await page.getByTestId('subject-details').innerText(),/declared|source-bound|checked|声明|绑定|检查范围/);
    await capture(page,'07-checks');
  });

  await run('layout-port-camera',async page=>{
    const host=page.getByTestId('workbench-canvas');await page.getByTestId('viewport-actual').click();
    const camera=await read(page),box=await host.boundingBox();
    await page.mouse.move(box.x+8,box.y+8);await page.mouse.down();await page.mouse.move(box.x+68,box.y+28,{steps:5});await page.mouse.up();
    near((await read(page)).camera.x,camera.camera.x-60/camera.camera.zoom);
    await page.getByTestId('viewport-fit').click();
    const before=await read(page),handle=page.locator('[data-layout-handle="object.stock"]'),rect=await handle.boundingBox();
    await page.mouse.move(rect.x+rect.width/2,rect.y+rect.height/2);await page.mouse.down();await page.mouse.move(rect.x+rect.width/2+32,rect.y+rect.height/2+16,{steps:6});await page.mouse.up();
    const moved=await read(page);near(moved.layout.positions['object.stock'].x-before.layout.positions['object.stock'].x,32/before.camera.zoom);near(moved.layout.positions['object.stock'].y-before.layout.positions['object.stock'].y,16/before.camera.zoom);
    const endpoints=await page.evaluate(()=>[...document.querySelectorAll('path[data-target-port]')].map(p=>{const a=p.getPointAtLength(p.getTotalLength()),pin=[...document.querySelectorAll('circle[data-port-id]')].find(n=>n.dataset.portId===p.dataset.targetPort&&n.dataset.nodeId===p.dataset.targetNode);return pin?Math.hypot(a.x-Number(pin.getAttribute('cx')),a.y-Number(pin.getAttribute('cy'))):null;}));
    assert(endpoints.length&&endpoints.every(d=>d!==null&&d<0.01),'Calls end at real Port centers');
    await page.getByTestId('layout-toggle').click();await page.getByTestId('layout-undo').click();assert.deepEqual((await read(page)).layout.positions,before.layout.positions);
    await page.keyboard.press('Escape');assert.deepEqual((await read(page)).checkpoint,before.checkpoint);
    const dimensions=await host.evaluate(node=>{const a=node.getBoundingClientRect(),b=node.querySelector('svg').getBoundingClientRect();return {host:{width:a.width,height:a.height},svg:{width:b.width,height:b.height}};});
    near(dimensions.host.width,dimensions.svg.width);near(dimensions.host.height,dimensions.svg.height);
    await capture(page,'08-layout');
  });

  await run('mobile-task',async page=>{
    await capture(page,'09-mobile-initial');
    await page.getByTestId('navigation-toggle').tap();await page.getByTestId('resource-search').fill('confirm');
    await ref(page.getByTestId('resource-navigation'),'operation','operation.confirm').tap();
    assert(await page.getByTestId('subject-details').isVisible());
    assert.equal(await page.getByTestId('resource-navigation').isVisible(),false);
    await capture(page,'10-mobile-method');
    await page.keyboard.press('Escape');assert.equal(await page.getByTestId('subject-details').isVisible(),false);
    await page.getByTestId('scenario-select').selectOption('scenario.lost-response');await view(page,'sequence');
    await page.getByTestId('player-next').tap();assert.equal((await read(page)).presentation.index,1);
    await page.getByTestId('player-expectations').tap();await capture(page,'11-mobile-evidence');
    await page.getByTestId('details-close').tap();
    const host=await page.getByTestId('workbench-canvas').boundingBox(),prior=await read(page),cdp=await page.context().newCDPSession(page);
    const touch=(x,y)=>({x,y,id:1,radiusX:2,radiusY:2});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch(host.x+8,host.y+8)]});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[touch(host.x+58,host.y+28)]});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
    assert((await read(page)).camera.x<prior.camera.x-30/prior.camera.zoom);
  },{mobile:true});

  if(technical)await run('technical-reuse',async page=>{
    await page.getByTestId('scenario-select').selectOption('tech.scenario');await seek(page,2);
    const snapshot=await read(page);assert.equal(snapshot.checkpoint.states['tech.player-1'].phase,'paused');assert.equal(snapshot.checkpoint.states['tech.player-2'].phase,'playing');
    for(const name of ['objects','process','dataflow','state','sequence'])await view(page,name);
    await capture(page,'12-technical');
  },{source:technical});
  if(unknownEnvelope)await run('unknown-envelope',async page=>{
    await page.getByTestId('scenario-select').selectOption('scenario.lost-response');
    const last=await page.evaluate(()=>JSON.parse(document.getElementById('workbench-model').textContent).scenarios.find(s=>s.id==='scenario.lost-response').checkpoints.length-1);
    await seek(page,last);
    const snapshot=await read(page),call=snapshot.checkpoint.calls.find(c=>c.phase==='unknown');
    assert.equal(call.completion,'reply');assert.deepEqual(call.payload,{status:'unknown'});
    const text=await page.getByTestId('experience-caller').innerText();
    assert.match(text,/未知/);assert.match(text,/回复|响应/);assert.doesNotMatch(text,/无返回|未收到/);
    assert.match(await page.getByTestId('experience-effect').innerText(),/reserved|提交|预留/i);
    await capture(page,'13-received-unknown');
  },{source:unknownEnvelope});
}finally{await browser.close();await writeFile(join(evidence,'closed.json'),JSON.stringify({closed:true})+'\n');}
assert.equal(results.filter(r=>r.status==='failed').length,0,JSON.stringify(results.filter(r=>r.status==='failed'),null,2));
