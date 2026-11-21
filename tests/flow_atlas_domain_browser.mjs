/** Independent offline workbench acceptance. Bind scenario assertions to the released core artifact. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const require = createRequire(import.meta.url);
const {chromium} = require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const viewNames = ['objects', 'process', 'dataflow', 'state', 'sequence'];

async function capture(page, evidence, name) {
  const snapshot = await page.evaluate(() => {
    const ids = ['view-select', 'process-select', 'scenario-select', 'player-position', 'player-phase', 'player-call', 'checkpoint-label', 'details'];
    const controls = Object.fromEntries(ids.map(id => {
      const element = document.getElementById(id);
      return [id, {value: element?.value, text: element?.textContent, hidden: element?.hidden}];
    }));
    const host = document.querySelector('.spatial-viewport');
    const canvas = document.querySelector('[data-testid="workbench-canvas"]');
    const bounds = element => {
      if (!element) return null;
      const box = element.getBoundingClientRect();
      return {x:box.x, y:box.y, width:box.width, height:box.height};
    };
    return {
      capturedAt:Date.now(), viewport:{width:innerWidth, height:innerHeight},
      bodyWidth:document.documentElement.scrollWidth, controls,
      camera:host ? {...host.dataset} : null,
      regions:{canvas:bounds(canvas), details:bounds(document.getElementById('details')), player:bounds(document.getElementById('player'))},
      references:Array.from(canvas?.querySelectorAll('[data-ref-id]') ?? [], n => ({kind:n.dataset.refKind, id:n.dataset.refId, text:n.textContent})),
      visibleText:document.body.innerText,
    };
  });
  await writeFile(join(evidence, name+'.json'), JSON.stringify(snapshot, null, 2)+'\n');
  await page.screenshot({path:join(evidence, name+'.png')});
  return snapshot;
}

async function camera(page) {
  return page.locator('.spatial-viewport').evaluate(host => {
    const svg = host.querySelector('svg'), box = host.getBoundingClientRect(), graphic = svg.getBoundingClientRect();
    return {
      x:Number(host.dataset.cameraX), y:Number(host.dataset.cameraY), zoom:Number(host.dataset.cameraZoom),
      host:{x:box.x,y:box.y,width:box.width,height:box.height},
      svg:{x:graphic.x,y:graphic.y,width:graphic.width,height:graphic.height},
      ratio:svg.viewBox.baseVal.width/svg.viewBox.baseVal.height,
    };
  });
}

function near(actual, expected, tolerance=0.1) {
  assert(Math.abs(actual-expected)<tolerance, `${actual} differs from ${expected}`);
}

async function checkShell(page) {
  const options = await page.getByTestId('view-select').locator('option').evaluateAll(nodes => nodes.map(n=>n.value));
  assert.deepEqual(options, viewNames, 'All projections share the same workspace');
  assert.equal(await page.getByTestId('player-speed').inputValue(), '0.5');
  assert.deepEqual(await page.getByTestId('player-speed').locator('option').evaluateAll(nodes=>nodes.map(n=>n.value)), ['0.25','0.5','1','2']);
  assert.equal(await page.getByTestId('scenario-player').isVisible(), false, 'Design mode has no scenario player');
  const dimensions = await camera(page);
  for (const axis of ['x','y','width','height']) near(dimensions.host[axis], dimensions.svg[axis]);
  near(dimensions.ratio, dimensions.host.width/dimensions.host.height, 0.0001);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth), await page.evaluate(()=>innerWidth));
}

// Coherent releases must be received before invoking this runner. More behavioral cases are
// attached after the actual core/checkpoint contract arrives; this early shell is not acceptance.
const [artifact, evidence, technicalArtifact, hostileArtifact, diagnosticArtifact] = process.argv.slice(2);
assert(artifact && evidence, 'Usage: node tests/flow_atlas_domain_browser.mjs artifact.html evidence-directory');
await mkdir(evidence, {recursive:true});
const browser = await chromium.launch({executablePath:'/usr/bin/google-chrome', headless:true, args:['--no-sandbox','--disable-dev-shm-usage']});
const errors = [];
const results=[];
const included=process.env.FLOW_ATLAS_CASES?.split(',');
const read=page=>page.evaluate(()=>globalThis.FlowAtlasWorkbench.state());
const ref=(host,kind,id)=>host.locator(`[data-ref-kind="${kind}"][data-ref-id="${id}"]`);
const choose=(page,id)=>page.getByTestId('scenario-select').selectOption(id);
async function seek(page,index) {
  await page.getByTestId('player-position').evaluate((node,value)=>{node.value=String(value);node.dispatchEvent(new Event('input',{bubbles:true}));},index);
  assert.equal((await read(page)).presentation.index,index);
}
async function closeDetails(page) {
  if(await page.getByTestId('subject-details').isVisible())await page.getByTestId('details-close').click();
}
async function run(name,check,{mobile=false,source=artifact}={}) {
  if(included&&!included.includes(name))return;
  const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1440,height:900},isMobile:mobile,hasTouch:mobile});
  const page=await context.newPage();
  const pageErrors=[];
  page.on('pageerror',error=>pageErrors.push(error.message));
  page.setDefaultTimeout(6000);
  try {
    await page.goto(pathToFileURL(source).href);
    await page.getByTestId('viewport-fit').waitFor();
    await check(page);
    assert.deepEqual(pageErrors,[]);
    results.push({name,status:'passed'});
  }catch(error){
    await capture(page,evidence,name+'-failure');
    results.push({name,status:'failed',message:error.message,stack:error.stack,pageErrors});
  }finally{
    errors.push(...pageErrors);await context.close();
    await writeFile(join(evidence,'result.json'),JSON.stringify({browser:browser.version(),results,errors},null,2)+'\n');
  }
}
try {
  await run('navigation',async page=>{
    await checkShell(page);await capture(page,evidence,'desktop-initial');
    const navigation=page.getByTestId('resource-navigation'), details=page.getByTestId('subject-details');
    await ref(navigation,'context','context.sales').click();assert.match(await details.innerText(),/销售/);
    await ref(navigation,'object','object.order').click();assert.match(await details.innerText(),/业务身份|orderId/);
    await ref(details,'field','field.order-quantity').click();assert.match(await details.innerText(),/quantity|Quantity/);
    await ref(navigation,'object','object.order').click();await ref(details,'operation','operation.confirm').click();
    assert.match(await details.innerText(),/reservationId/);assert.match(await details.innerText(),/Schema/);
    assert.match(await details.innerText(),/native\.order\.confirm/,'Method details expose the real native binding');
    await capture(page,evidence,'desktop-method-schema');
    await ref(details,'rule','rule.receipt-quantity').click();assert.match(await details.innerText(),/native.ts/);
    await page.getByTestId('resource-search').fill('confirm');
    const operation=ref(navigation,'operation','operation.confirm');await operation.focus();await page.keyboard.press('Enter');
    assert.equal((await read(page)).selection.id,'operation.confirm');
    await page.keyboard.press('Escape');assert.equal(await details.isVisible(),false);
    await page.getByTestId('resource-search').fill('');
    await ref(navigation,'object','object.order').click();await ref(details,'operation','operation.confirm').click();
    await ref(details,'step','confirm').click();assert.equal((await read(page)).view,'process');
    await ref(details,'binding','binding.confirm').click();assert.equal((await read(page)).view,'dataflow');
    assert.match(await details.innerText(),/reserve.*reserved/s);
    await capture(page,evidence,'desktop-binding-source');
  });
  await run('views',async page=>{
    await choose(page,'scenario.success');
    for(const view of viewNames) {
      await page.getByTestId('view-select').selectOption(view);
      assert.equal((await read(page)).view,view);
      assert(await page.getByTestId('workbench-canvas').locator('svg text').count()>0);
    }
    await seek(page,7);await page.getByTestId('view-select').selectOption('dataflow');
    const line=page.locator('path[data-binding-id="binding.confirm"]');
    assert.equal(await line.getAttribute('data-source-step-id'),'reserve');assert.equal(await line.getAttribute('data-source-outcome-id'),'reserved');
    assert.equal(await line.getAttribute('data-source-path'),'[]');
    const binding=page.locator('g[data-binding-id="binding.confirm"]');await binding.focus();await page.keyboard.press('Enter');
    assert.match(await page.getByTestId('subject-details').innerText(),/resolved|已解析/);
    assert.match(await page.getByTestId('subject-details').innerText(),/reservation:|reservationId/);
    await capture(page,evidence,'dataflow-provenance');
    await closeDetails(page);await page.getByTestId('view-select').selectOption('state');await seek(page,8);
    assert(await page.locator('[data-transition-id="transition.confirm"][data-visited="true"]').count()>0);
    await capture(page,evidence,'state-applied');
  });
  await run('playback',async page=>{
    await choose(page,'scenario.success');
    const initial=(await read(page)).checkpoint;
    await page.getByTestId('player-play').click();
    await page.waitForFunction(()=>FlowAtlasWorkbench.state().presentation.progress>0.08);
    const active=await read(page);assert.equal(active.presentation.index,0);assert.deepEqual(active.checkpoint.states,initial.states);
    assert(await page.getByTestId('flow-marker').count()>0);
    const first=await page.getByTestId('flow-marker').getAttribute('cx');
    await page.waitForFunction(previous=>document.querySelector('[data-testid="flow-marker"]')?.getAttribute('cx')!==previous,first);
    await page.getByTestId('player-play').click();
    const paused=await read(page);assert.equal(paused.presentation.playing,false);
    await capture(page,evidence,'playback-paused-request');
    for(const view of ['dataflow','sequence','objects'])await page.getByTestId('view-select').selectOption(view);
    const retained=await read(page);near(retained.presentation.elapsed,paused.presentation.elapsed,0.001);assert.equal(retained.presentation.index,paused.presentation.index);
    await page.getByTestId('player-play').click();await page.waitForFunction(()=>FlowAtlasWorkbench.state().presentation.index===1);
    await page.getByTestId('player-play').click();await seek(page,1);
    assert.equal((await read(page)).checkpoint.states['order-42-view'].status,'draft');
    await page.getByTestId('player-play').click();await page.waitForFunction(()=>FlowAtlasWorkbench.state().presentation.phase==='effect'&&FlowAtlasWorkbench.state().presentation.progress>0.08);
    assert(await page.getByTestId('processing-marker').count()>0);assert.equal((await read(page)).checkpoint.states['order-42-view'].status,'draft');
    await page.getByTestId('player-play').click();await capture(page,evidence,'playback-paused-processing');
    await page.getByTestId('player-next').click();
    assert.equal((await read(page)).checkpoint.states['order-42-view'].status,'awaiting');
    assert.equal(await page.locator('[data-instance-id="order-42-view"][data-field-id="field.order-status"]').getAttribute('data-after'),'"awaiting"');
    await capture(page,evidence,'playback-applied-effect');
    await page.getByTestId('player-next').click();assert.equal(await page.locator('[data-after]').count(),0);
    await page.getByTestId('player-previous').click();assert.equal((await read(page)).presentation.index,2);
    await page.getByTestId('player-replay').click();assert.equal((await read(page)).presentation.index,0);
    await page.getByTestId('player-play').click();await choose(page,'scenario.refusal');assert.equal((await read(page)).presentation.index,0);assert.equal((await read(page)).presentation.playing,false);
  });
  await run('lost-response',async page=>{
    await choose(page,'scenario.lost-response');await seek(page,5);
    const committed=await read(page);assert.equal(committed.checkpoint.states['order-42-view'].status,'awaiting');
    assert.equal(committed.checkpoint.states['stock-book'].reservations.length,1);assert.equal(committed.checkpoint.derived['stock-book']['field.available'],18);
    assert.equal(await page.locator('[data-instance-id="stock-book"][data-field-id="field.available"]').getAttribute('data-value'),'18');
    await page.getByTestId('view-select').selectOption('sequence');await page.getByTestId('player-play').click();
    await page.waitForFunction(()=>FlowAtlasWorkbench.state().presentation.phase==='outcome'&&FlowAtlasWorkbench.state().presentation.progress>0.1);
    assert.equal(await page.getByTestId('flow-marker').count(),0,'Local uncertainty cannot draw a return packet');
    await page.getByTestId('player-play').click();await page.getByTestId('player-next').click();
    const observed=await read(page),call=observed.checkpoint.calls.find(call=>call.stepId==='reserve');
    assert.equal(call.phase,'unknown');assert.equal(call.effectOutcomeId,'reserved');assert.equal(call.payload,undefined);
    assert.equal(await page.locator('[data-reply-call-id="'+call.callId+'"]').count(),0);assert.match(await page.getByTestId('workbench-canvas').innerText(),/本地结果.*unknown/);
    await capture(page,evidence,'committed-unknown-sequence');
    await page.getByTestId('view-select').selectOption('objects');await capture(page,evidence,'committed-unknown-objects');
    await page.getByTestId('player-next').click();assert.equal((await read(page)).checkpoint.states['stock-book'].reservations.length,1);
  });
  await run('outcomes',async page=>{
    for(const [scenario,status,available] of [['scenario.success','confirmed',18],['scenario.refusal','rejected',20],['scenario.wrong-receipt','awaiting',18],['scenario.stale-receipt','awaiting',18],['scenario.quantity-mismatch','awaiting',18]]){
      await choose(page,scenario);const max=Number(await page.getByTestId('player-position').getAttribute('max'));await seek(page,max);
      const current=await read(page);assert.equal(current.checkpoint.states['order-42-view'].status,status);assert.equal(current.checkpoint.states['order-43-view'].status,'draft');
      assert.equal(current.checkpoint.derived['stock-book']['field.available'],available);
      await page.getByTestId('player-expectations').click();assert.match(await page.getByTestId('subject-details').innerText(),/预期值|预期/);
      assert.equal(await page.getByTestId('subject-details').locator('[data-status="failed"]').count(),0);
      await closeDetails(page);
    }
    await page.getByTestId('compare-toggle').click();assert.equal(await page.getByTestId('subject-details').locator('[data-scenario-id]').count(),6);
    await capture(page,evidence,'comparison-expectations');
  });
  await run('readable-entry',async page=>{
    await choose(page,'scenario.lost-response');
    assert((await camera(page)).zoom>=0.85,'Entering a scenario preserves readable fields');
    await capture(page,evidence,'desktop-readable-scenario');
    await page.getByTestId('view-select').selectOption('dataflow');assert((await camera(page)).zoom>=0.85,'Initial dataflow does not shrink labels into an unreadable overview');
    await capture(page,evidence,'desktop-readable-dataflow');
    await page.getByTestId('viewport-fit').click();const overview=await camera(page);assert(overview.zoom<0.85,'Explicit fit remains available as a full overview');
    await page.getByTestId('view-select').selectOption('objects');await page.getByTestId('view-select').selectOption('dataflow');near((await camera(page)).zoom,overview.zoom,0.001);
    await choose(page,'scenario.success');assert((await camera(page)).zoom>=0.85,'A fresh scenario gets a fresh readable camera');
  });
  await run('layout-camera',async page=>{
    const before=await read(page),cam=await camera(page);
    await page.mouse.move(cam.host.x+8,cam.host.y+8);await page.mouse.down();await page.mouse.move(cam.host.x+88,cam.host.y+33,{steps:5});await page.mouse.up();
    near((await camera(page)).x,cam.x-80/cam.zoom,0.2);
    await page.getByTestId('viewport-fit').click();
    await page.mouse.move(cam.host.x+200,cam.host.y+180);await page.mouse.wheel(0,180);
    await page.waitForFunction(previous=>Number(document.querySelector('.spatial-viewport').dataset.cameraZoom)!==previous,cam.zoom);
    const scaled=await camera(page),state=await read(page),id='object.stock',handle=page.locator('[data-layout-handle="'+id+'"]'),box=await handle.boundingBox();
    await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2+36,box.y+box.height/2+18,{steps:6});await page.mouse.up();
    const moved=await read(page);near(moved.layout.positions[id].x-state.layout.positions[id].x,36/scaled.zoom,0.3);near(moved.layout.positions[id].y-state.layout.positions[id].y,18/scaled.zoom,0.3);
    const endpoints=await page.evaluate(()=>Array.from(document.querySelectorAll('path[data-target-port]')).map(path=>{const p=path.getPointAtLength(path.getTotalLength()),pin=Array.from(document.querySelectorAll('circle[data-port-id]')).find(pin=>pin.dataset.portId===path.dataset.targetPort&&pin.dataset.nodeId===path.dataset.targetNode);return Math.hypot(p.x-Number(pin.getAttribute('cx')),p.y-Number(pin.getAttribute('cy')));}));
    assert(endpoints.length>0&&endpoints.every(distance=>distance<0.01),'All call endpoints remain actual Port centers');
    await page.getByTestId('layout-toggle').click();await page.getByTestId('layout-undo').click();assert.deepEqual((await read(page)).layout.positions,state.layout.positions);
    await page.getByTestId('layout-export').click();const exported=await page.getByTestId('layout-json').inputValue(),doc=JSON.parse(exported);doc.positions[id].x+=23;
    await page.getByTestId('layout-json').fill(JSON.stringify(doc));await page.getByTestId('layout-import').click();near((await read(page)).layout.positions[id].x,state.layout.positions[id].x+23);
    const good=await read(page);doc.contentDigest='wrong';await page.getByTestId('layout-json').fill(JSON.stringify(doc));await page.getByTestId('layout-import').click();assert.deepEqual((await read(page)).layout,good.layout,'Invalid layout import is atomic');
    await page.keyboard.press('Escape');await page.getByTestId('viewport-fit').click();
    await handle.focus();await page.keyboard.press('p');const pinned=await read(page);await page.keyboard.press('ArrowRight');assert.deepEqual((await read(page)).layout.positions,pinned.layout.positions);await page.keyboard.press('p');
    await page.keyboard.press('ArrowRight');near((await read(page)).layout.positions[id].x,pinned.layout.positions[id].x+10);
    assert.equal((await read(page)).checkpoint,before.checkpoint,'Layout cannot create a semantic checkpoint');
    await capture(page,evidence,'layout-scaled-ports');
  });
  await run('mobile',async page=>{
    await checkShell(page);assert((await camera(page)).zoom>=0.85);
    await capture(page,evidence,'mobile-initial');
    await page.getByTestId('navigation-toggle').tap();await page.getByTestId('resource-search').fill('confirm');
    const operation=ref(page.getByTestId('resource-navigation'),'operation','operation.confirm');await operation.tap();
    assert.equal((await read(page)).selection.id,'operation.confirm');assert(await page.getByTestId('subject-details').isVisible());
    assert.equal(await page.getByTestId('resource-navigation').isVisible(),false);await capture(page,evidence,'mobile-method');
    await page.keyboard.press('Escape');assert.equal(await page.getByTestId('subject-details').isVisible(),false);
    await choose(page,'scenario.lost-response');await page.getByTestId('player-next').tap();assert.equal((await read(page)).presentation.index,1);
    await page.getByTestId('player-expectations').tap();await capture(page,evidence,'mobile-player-expectations');await page.getByTestId('details-close').tap();
    const cam=await camera(page),cdp=await page.context().newCDPSession(page),touch=(x,y,id)=>({x,y,id,radiusX:2,radiusY:2});
    const x=cam.host.x+8,y=cam.host.y+8;
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch(x,y,1)]});await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[touch(x+60,y+25,1)]});await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});assert((await camera(page)).x<cam.x-40/cam.zoom);
    await page.getByTestId('viewport-fit').tap();
    const start=await camera(page);
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch(x+50,y+8,1),touch(x+150,y+8,2)]});await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[touch(x+25,y+8,1),touch(x+175,y+8,2)]});await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});assert((await camera(page)).zoom>start.zoom*1.3);
    await page.getByTestId('viewport-actual').tap();await page.getByTestId('layout-toggle').tap();await page.getByTestId('layout-export').tap();assert(JSON.parse(await page.getByTestId('layout-json').inputValue()).positions);
    await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);
    await capture(page,evidence,'mobile-workspace');
  },{mobile:true});
  if(technicalArtifact)await run('technical',async page=>{
    await checkShell(page);await choose(page,'tech.scenario');await seek(page,2);
    const s=await read(page);assert.equal(s.checkpoint.states['tech.player-1'].phase,'paused');assert.equal(s.checkpoint.states['tech.player-2'].phase,'playing');
    for(const view of viewNames)await page.getByTestId('view-select').selectOption(view);
    await capture(page,evidence,'technical-consumer');
  },{source:technicalArtifact});
  if(hostileArtifact)await run('safety',async page=>{
    assert.equal(await page.evaluate(()=>globalThis.AUTHORED_EXECUTED),undefined);assert.equal(await page.locator('script').count(),2);
    await ref(page.getByTestId('resource-navigation'),'object','object.order').click();
    const details=page.getByTestId('subject-details');assert.match(await details.innerText(),/<img src=x/);
    assert.equal(await details.locator('img').count(),0);
    const links=await details.locator('a').evaluateAll(nodes=>nodes.map(node=>node.getAttribute('href')));
    assert(links.includes('https://example.com/design'));assert(links.every(href=>/^https?:\/\//.test(href)));
    assert.equal(await page.evaluate(()=>globalThis.AUTHORED_EXECUTED),undefined);
    await capture(page,evidence,'inert-authored-metadata');
  },{source:hostileArtifact});
  if(diagnosticArtifact)await run('diagnostics',async page=>{
    await page.getByTestId('diagnostics-toggle').click();
    const details=page.getByTestId('subject-details'),target=ref(details,'binding','binding.confirm');assert(await target.count()>0);
    await target.first().click();const s=await read(page);assert.equal(s.selection.kind,'binding');assert.equal(s.selection.id,'binding.confirm');assert.equal(s.selection.scope,'process.checkout/confirm');
    assert.equal(s.view,'dataflow');await capture(page,evidence,'diagnostic-exact-target');
  },{source:diagnosticArtifact});
  assert.equal(results.filter(item=>item.status==='failed').length,0,JSON.stringify(results.filter(item=>item.status==='failed'),null,2));
}finally{
  await browser.close();
  await writeFile(join(evidence,'closed.json'), JSON.stringify({closed:true})+'\n');
}
