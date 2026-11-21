/** Independent linked-workspace investigation cases. Consumes portable fixtures; never executes native methods. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const require=createRequire(import.meta.url);
const {chromium}=require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const [artifact,evidence,technical,repeated,nativeArtifact]=process.argv.slice(2);
assert(artifact&&evidence,'Usage: node tests/flow_atlas_linked_browser.mjs linked.html evidence-dir [technical.html] [repeated-projection.html] [native.html]');
const included=process.env.FLOW_ATLAS_LINKED_CASES?.split(',');
await mkdir(evidence,{recursive:true});
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
const results=[];
const state=page=>page.evaluate(()=>FlowAtlasWorkbench.state());
const model=page=>page.evaluate(()=>JSON.parse(document.getElementById('workbench-model').textContent));
const pane=(page,name)=>page.getByTestId('linked-pane-'+name);
const near=(a,b,tolerance=0.01)=>assert(Math.abs(a-b)<tolerance,`${a} differs from ${b}`);
const close=async page=>{if(await page.getByTestId('subject-details').isVisible())await page.getByTestId('details-close').click();};
async function scenario(page,id){await page.getByTestId('scenario-select').selectOption(id);await close(page);}
async function seek(page,index){await page.getByTestId('player-position').evaluate((node,value)=>{node.value=String(value);node.dispatchEvent(new Event('input',{bubbles:true}));},index);assert.equal((await state(page)).presentation.index,index);}
async function last(page){const s=await state(page),m=await model(page);await seek(page,m.scenarios.find(item=>item.id===s.scenarioId).checkpoints.length-1);}
function occurrence(page,paneId,visitId){return pane(page,paneId).locator(`[data-visit-id="${visitId}"][role="button"]`).first();}
async function chooseOccurrence(page,paneId,visitId){const node=occurrence(page,paneId,visitId);await node.focus();await page.keyboard.press('Enter');}
async function capture(page,name){
  const observation=await page.evaluate(()=>{
    const bounds=node=>{if(!node)return null;const b=node.getBoundingClientRect();return {x:b.x,y:b.y,width:b.width,height:b.height};};
    return {state:FlowAtlasWorkbench.state(),viewport:{width:innerWidth,height:innerHeight},document:{width:document.documentElement.scrollWidth,height:document.documentElement.scrollHeight},regions:Object.fromEntries(['linked-pane-primary','linked-pane-secondary','subject-details','scenario-player'].map(id=>[id,bounds(document.querySelector(`[data-testid="${id}"]`))])),text:document.body.innerText,visibleControls:[...document.querySelectorAll('button,select,input')].filter(n=>n.getClientRects().length&&!n.closest('[hidden]')).map(n=>({testid:n.dataset.testid,id:n.id,name:n.getAttribute('aria-label')||n.textContent,disabled:n.disabled})),paneText:[...document.querySelectorAll('[data-pane] svg.domain-scene text')].map(n=>{const matrix=n.getScreenCTM();return {pane:n.closest('[data-pane]')?.dataset.pane,text:n.textContent,fontSize:getComputedStyle(n).fontSize,effectiveFont:parseFloat(getComputedStyle(n).fontSize)*(matrix?Math.hypot(matrix.a,matrix.b):1),bounds:bounds(n)};})};
  });
  await writeFile(join(evidence,name+'.json'),JSON.stringify(observation,null,2)+'\n');
  await page.screenshot({path:join(evidence,name+'.png')});return observation;
}
async function run(name,fn,{mobile=false,source=artifact}={}){
  if(included&&!included.includes(name))return;
  const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1440,height:900},hasTouch:mobile,isMobile:mobile});
  await context.addInitScript(()=>{
    // Observe the public clock factory and the actual parsed model references, without changing semantics.
    let owner;globalThis.__linkedPlayerCount=0;
    Object.defineProperty(globalThis,'WorkbenchPlayer',{configurable:true,get(){return owner;},set(value){owner=Object.freeze({...value,create(...args){globalThis.__linkedPlayerCount++;return value.create(...args);}});}});
    const original=JSON.parse;globalThis.__linkedModels=[];
    JSON.parse=function(...args){const value=original.apply(this,args);if(value?.format==='flow-atlas/workbench-1')globalThis.__linkedModels.push({value,before:JSON.stringify(value)});return value;};
  });
  const page=await context.newPage(),errors=[],requests=[];page.setDefaultTimeout(8000);
  page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>requests.push(r.url()));
  try{
    await page.goto(/^https?:/.test(source)?source:pathToFileURL(source).href);
    await page.getByTestId('linked-mode').waitFor();await pane(page,'primary').waitFor();
    await fn(page);
    assert.deepEqual(errors,[],'No page errors');
    assert.equal(await page.evaluate(()=>__linkedPlayerCount),1,'Exactly one presentation clock owner');
    assert(await page.evaluate(()=>__linkedModels.every(item=>JSON.stringify(item.value)===item.before)),'All actual parsed semantic artifacts remain immutable');
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),mobile?390:1440,'No document overflow');
    const duplicates=await page.evaluate(()=>{const seen=new Set(),duplicates=[];for(const node of document.querySelectorAll('[id]')){if(seen.has(node.id))duplicates.push(node.id);seen.add(node.id);}return duplicates;});
    assert.deepEqual(duplicates,[],'Pane-local SVG markers and DOM IDs stay unique');
    assert(requests.every(url=>url.startsWith('file:')||url.startsWith('data:')||url===source),'No external resources or API access');
    results.push({name,status:'passed',errors,requests});
  }catch(error){await capture(page,name+'-failure').catch(()=>{});results.push({name,status:'failed',message:error.message,stack:error.stack,errors,requests});}
  finally{await context.close();await writeFile(join(evidence,'result.json'),JSON.stringify({browser:browser.version(),results},null,2)+'\n');}
}
try{
  await run('definition-branches',async page=>{
    const s=await state(page),m=await model(page),process=m.definition.processes.find(p=>p.id===s.processId);
    assert.equal(s.scenarioId,'');assert.equal(s.linked.mode,'paired');assert.equal(s.linked.panes.primary.view,'process');assert.equal(s.linked.panes.secondary.view,'sequence');
    assert.equal(await page.getByTestId('scenario-player').isVisible(),false);
    assert.equal(await pane(page,'secondary').locator('[data-message-kind="request"]').count(),0,'Design sequence cannot imply declaration-order execution');
    for(const flow of process.flows)assert(await pane(page,'primary').locator(`[data-ref-kind="flow"][data-ref-id="${flow.id}"]`).count()>0,`Declared branch ${flow.id} remains inspectable`);
    await capture(page,'01-definition');
  });
  await run('scenario-path-truth',async page=>{
    const m=await model(page);
    for(const id of ['scenario.success','scenario.refusal','scenario.lost-response']){
      await scenario(page,id);const sc=m.scenarios.find(s=>s.id===id);
      assert.equal((await state(page)).presentation.index,0);
      const expected=sc.visits.map(v=>v.callId).sort();
      const initialRequests=await pane(page,'secondary').locator('[data-message-kind="request"]').evaluateAll(nodes=>nodes.map(n=>n.dataset.callId).sort());
      assert.deepEqual(initialRequests,expected,'Scenario sequence contains exactly authored visits');
      assert.equal(await pane(page,'primary').locator('[data-visit-id][data-applied="true"]').count(),0,'Selected scenario path is not an applied prefix at initial checkpoint');
      if(id==='scenario.success'){
        assert(await pane(page,'primary').locator('[data-flow-id="flow.reserved"][data-scenario-path="true"]').count()>0);
        assert.equal(await pane(page,'primary').locator('[data-flow-id="flow.refused"][data-scenario-path="true"]').count(),0,'Exclusive refusal branch stays declared but unvisited');
      }
      await last(page);const current=await state(page);assert.deepEqual(current.checkpoint,sc.checkpoints.at(-1));
      for(const visit of sc.visits){
        assert(await pane(page,'primary').locator(`[data-visit-id="${visit.id}"][data-scenario-path="true"]`).count()>0);
        const replies=pane(page,'secondary').locator(`[data-message-kind="reply"][data-call-id="${visit.callId}"]`);
        assert.equal(await replies.count(),visit.completion==='reply'?1:0,`Reply truth for ${visit.id}`);
      }
      for(const name of ['primary','secondary'])assert.equal(current.linked.panes[name].checkpointIndex,current.presentation.index);
      if(id==='scenario.lost-response'){
        const call=current.checkpoint.calls.find(c=>c.stepId==='reserve');assert.equal(call.completion,'local-outcome');assert.equal(call.phase,'unknown');assert.equal(call.effectOutcomeId,'reserved');
        assert.equal(await pane(page,'secondary').locator(`[data-message-kind="local-outcome"][data-call-id="${call.callId}"]`).count(),1);
        assert.equal(await pane(page,'secondary').locator('[data-step-id="confirm"][data-message-kind]').count(),0);
      }
      await capture(page,'02-'+id.split('.').at(-1));
    }
  });
  await run('bidirectional-focus',async page=>{
    await scenario(page,'scenario.success');await seek(page,5);
    const initial=await state(page);
    await chooseOccurrence(page,'primary','success:visit:1');
    let s=await state(page);assert.equal(s.linked.focus.stepId,'reserve');assert.equal(s.linked.focus.callId,'success:call:1');assert.equal(s.linked.focus.visitId,'success:visit:1');
    assert.equal(s.linked.panes.primary.view,'process');assert.equal(s.linked.panes.secondary.view,'sequence');assert.deepEqual(s.checkpoint,initial.checkpoint);
    assert(await pane(page,'secondary').locator('[data-call-id="success:call:1"][data-selected="true"]').count()>0);
    assert(await page.getByTestId('subject-details').isVisible());await close(page);
    await chooseOccurrence(page,'secondary','success:visit:0');s=await state(page);
    assert.equal(s.linked.focus.stepId,'begin');assert.equal(s.linked.focus.visitId,'success:visit:0');assert.deepEqual(s.checkpoint,initial.checkpoint);
    assert(await pane(page,'primary').locator('[data-visit-id="success:visit:0"][data-selected="true"]').count()>0);
    await capture(page,'03-linked-focus');await close(page);
    await page.getByTestId('linked-secondary-view').selectOption('state');s=await state(page);
    assert.equal(s.linked.panes.secondary.view,'state');assert.equal(s.linked.focus.callId,'success:call:0');assert.deepEqual(s.checkpoint,initial.checkpoint);
    await capture(page,'04-state-companion');
  });
  await run('contract-drill-and-layout',async page=>{
    await scenario(page,'scenario.success');await seek(page,5);
    await chooseOccurrence(page,'primary','success:visit:1');const before=await state(page),details=page.getByTestId('subject-details');
    // The investigation opens real native contracts/provenance, without replacing the paired subject.
    const operation=details.locator('[data-ref-kind="operation"][data-ref-id="operation.reserve"]').first();
    await operation.click();assert.match(await details.innerText(),/Schema|schema/);
    assert.equal((await state(page)).linked.panes.primary.view,'process');assert.equal((await state(page)).linked.panes.secondary.view,'sequence');
    await close(page);await page.getByTestId('experience-view-objects').click();
    assert.equal((await state(page)).linked.panes.primary.view,'objects');
    const beforeLayout=await state(page),handle=pane(page,'primary').locator('[data-layout-handle="stock-book"]');
    await handle.focus();await page.keyboard.press('ArrowRight');const moved=await state(page);
    assert(moved.layout.positions['stock-book'].x>beforeLayout.layout.positions['stock-book'].x,'Explicit object view keeps editable presentation layout');
    assert.deepEqual(moved.checkpoint,before.checkpoint);assert.deepEqual(moved.linked.panes.secondary.camera,beforeLayout.linked.panes.secondary.camera);
    await page.getByTestId('layout-toggle').click();await page.getByTestId('layout-undo').click();
    assert.deepEqual((await state(page)).layout.positions,beforeLayout.layout.positions);
    await page.keyboard.press('Escape');await capture(page,'04b-object-contract-layout');
  });
  await run('one-clock-applied-boundary',async page=>{
    await scenario(page,'scenario.success');await page.getByTestId('player-speed').selectOption('0.25');
    const m=await model(page),sc=m.scenarios.find(s=>s.id==='scenario.success');
    await page.getByTestId('player-play').click();await page.waitForFunction(()=>FlowAtlasWorkbench.state().presentation.progress>0.08);
    await page.getByTestId('player-play').click();const paused=await state(page);
    assert.equal(paused.presentation.index,0);assert.deepEqual(paused.checkpoint,sc.checkpoints[0]);
    for(const name of ['primary','secondary'])assert.equal(paused.linked.panes[name].checkpointIndex,0);
    await chooseOccurrence(page,'secondary','success:visit:1');await close(page);
    let retained=await state(page);near(retained.presentation.elapsed,paused.presentation.elapsed,0.0001);assert.equal(retained.presentation.playing,false);
    await page.getByTestId('linked-mode').selectOption('single');await page.getByTestId('linked-mode').selectOption('paired');
    retained=await state(page);near(retained.presentation.elapsed,paused.presentation.elapsed,0.0001);assert.deepEqual(retained.checkpoint,paused.checkpoint);
    for(const name of ['primary','secondary']){
      const cue=pane(page,name).getByTestId('linked-payload-cue');assert(await cue.isVisible(),'Flow data remains accessible in each pane');
      assert.equal(await cue.getAttribute('data-call-id'),'success:call:0','Payload belongs to the in-flight call, independent of selected future visit');
      await cue.locator('summary').click();assert.match(await cue.getByTestId('flow-payload').innerText(),/intent-42/);await cue.locator('summary').click();
    }
    await page.getByTestId('player-play').click();await page.waitForFunction(old=>FlowAtlasWorkbench.state().presentation.elapsed>old,paused.presentation.elapsed);
    await page.getByTestId('player-play').click();await capture(page,'05-paused-transition');
    await scenario(page,'scenario.refusal');retained=await state(page);assert.equal(retained.presentation.index,0);assert.equal(retained.presentation.elapsed,0);assert.equal(retained.presentation.playing,false);assert.equal(retained.linked.focus?.scenarioId??'scenario.refusal','scenario.refusal');
  });
  await run('independent-cameras',async page=>{
    await scenario(page,'scenario.success');const before=await state(page);
    const svg=pane(page,'secondary').locator('svg.domain-scene');const box=await svg.boundingBox();
    await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.keyboard.down('Space');await page.mouse.down();await page.mouse.move(box.x+box.width/2+52,box.y+box.height/2+21,{steps:6});await page.mouse.up();await page.keyboard.up('Space');
    const moved=await state(page);assert.notDeepEqual(moved.linked.panes.secondary.camera,before.linked.panes.secondary.camera);assert.deepEqual(moved.linked.panes.primary.camera,before.linked.panes.primary.camera);assert.deepEqual(moved.checkpoint,before.checkpoint);
    await page.getByTestId('linked-mode').selectOption('single');await page.getByTestId('linked-mode').selectOption('paired');
    const restored=await state(page);near(restored.linked.panes.secondary.camera.zoom,moved.linked.panes.secondary.camera.zoom);assert.deepEqual(restored.linked.focus,moved.linked.focus);
    assert(await page.evaluate(()=>{const a=FlowAtlasWorkbench.state();a.linked.panes.primary.camera.x=999999;a.linked.mode='changed';return FlowAtlasWorkbench.state().linked.mode!=='changed'&&FlowAtlasWorkbench.state().linked.panes.primary.camera.x!==999999;}),'Public snapshots remain detached');
    await capture(page,'06-independent-cameras');
  });
  await run('mobile-linked-investigation',async page=>{
    await scenario(page,'scenario.lost-response');await last(page);
    const primary=await state(page);assert(await page.getByTestId('linked-show-secondary').isVisible());
    await page.getByTestId('linked-show-secondary').tap();assert(await pane(page,'secondary').isVisible());assert.equal((await state(page)).linked.activePane,'secondary');
    await chooseOccurrence(page,'secondary','lost-response:visit:1');assert(await page.getByTestId('subject-details').isVisible());await page.keyboard.press('Escape');
    const selected=await state(page);assert.equal(selected.linked.focus.stepId,'reserve');assert.deepEqual(selected.checkpoint,primary.checkpoint);
    await capture(page,'07-mobile-sequence');await page.getByTestId('linked-show-primary').tap();assert(await pane(page,'primary').isVisible());assert.equal((await state(page)).linked.focus.callId,selected.linked.focus.callId);
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    const visibleTarget=await pane(page,'primary').locator('[data-occurrence-chip="true"][data-visit-id="lost-response:visit:1"]').evaluate(node=>{
      const target=node.getBoundingClientRect(),host=node.closest('[data-pane]').querySelector('.canvas').getBoundingClientRect();
      return target.right>host.left&&target.left<host.right&&target.bottom>host.top&&target.top<host.bottom;
    });
    assert(visibleTarget,'Revealed pane must show the retained exact occurrence, not only store its ID');
    const player=await page.getByTestId('scenario-player').boundingBox();assert(player.y>=0&&player.y+player.height<=844.5,'Player stays inside narrow workspace');
    await capture(page,'08-mobile-collaboration');
  },{mobile:true});
  if(technical)await run('technical-consumer',async page=>{
    await scenario(page,'tech.scenario');await seek(page,2);await chooseOccurrence(page,'primary','tech.visit.pause');
    const s=await state(page);assert.equal(s.linked.focus.callId,'tech.call.pause');assert.equal(s.checkpoint.states['tech.player-1'].phase,'paused');assert.equal(s.checkpoint.states['tech.player-2'].phase,'playing');
    assert.equal(await pane(page,'secondary').locator('[data-message-kind="request"]').count(),1);
    await close(page);await capture(page,'09-technical');
  },{source:technical});
  if(repeated)await run('repeated-occurrence-projection',async page=>{
    await scenario(page,'scenario.repeated-ui');const m=await model(page),sc=m.scenarios.find(s=>s.id==='scenario.repeated-ui');await last(page);
    const visits=sc.visits.filter(v=>v.stepId==='reserve');assert.equal(visits.length,2);
    for(const visit of visits){await chooseOccurrence(page,'secondary',visit.id);const s=await state(page);assert.equal(s.linked.focus.visitId,visit.id);assert.equal(s.linked.focus.callId,visit.callId);await close(page);}
    const header=pane(page,'primary').locator('[data-step-id="reserve"][data-ref-kind="step"]').first();await header.focus();await page.keyboard.press('Enter');
    const focus=(await state(page)).linked.focus;assert.equal(focus.stepId,'reserve');assert.equal(focus.visitId??null,null,'A repeated definition click cannot guess an occurrence');assert.equal(focus.callId??null,null);
    await capture(page,'10-repeated-projection');
  },{source:repeated});
  if(nativeArtifact)await run('native-builder-composition',async page=>{
    await scenario(page,'scenario.success');await seek(page,5);await chooseOccurrence(page,'primary','success:visit:1');await close(page);
    assert.equal((await state(page)).linked.focus.callId,'success:call:1');await capture(page,'11-native-composition');
  },{source:nativeArtifact});
}finally{await browser.close();}
console.log(JSON.stringify({evidence,results},null,2));
if(results.some(result=>result.status==='failed'))process.exitCode=1;
