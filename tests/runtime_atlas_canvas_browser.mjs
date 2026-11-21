/** Outcome checks: object-owned pins, data on real call paths and local state. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
const [origin,example,evidence,selectedPhase]=process.argv.slice(2);assert(origin&&example&&evidence);
const require=createRequire(import.meta.url),{chromium}=require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
await mkdir(evidence,{recursive:true});
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server']});
const model=JSON.parse(await readFile(new URL('../.docs/architecture/runtime-atlas/model.json',import.meta.url),'utf8'));
let page;const errors=[],checks=[];
const shot=name=>page.screenshot({path:join(evidence,name+'.png')});
const applied=()=>page.locator('[data-testid="event-phase"][data-phase="applied"]').waitFor({timeout:20000});
async function open(scenario='appserver-mail',step=-1,speed=2){
 const mode=model.scenarios.find(s=>s.id===scenario).modeId;
 await page.goto(`${origin}#mode=${mode}&scenario=${scenario}&step=${step}&view=relations&speed=${speed}`);
 await page.getByTestId('event-phase').waitFor();
}
try{
 const desktop=await browser.newContext({viewport:{width:1366,height:768},reducedMotion:'no-preference'});
 page=await desktop.newPage();page.on('pageerror',e=>errors.push(e.message));
 await open();
 if(selectedPhase==='details'){
  for(const viewport of [{width:1366,height:768},{width:390,height:844}]){
   await page.setViewportSize(viewport);await open();
   await page.getByTestId('canvas-state-daemon-inbox').focus();await page.keyboard.press('Enter');
   assert.match(await page.getByTestId('inspector').innerText(),/当前 · 无待收消息/);
   await page.keyboard.press('Escape');
   assert.equal(await page.getByTestId('inspector').isVisible(),false);
   await shot('state-detail-return-'+viewport.width);
  }
  const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,reducedMotion:'reduce'});
  page=await mobile.newPage();page.on('pageerror',e=>errors.push(e.message));await open();
  await page.getByTestId('canvas-state-daemon-inbox').tap();
  assert.match(await page.getByTestId('inspector').innerText(),/当前 · 无待收消息/);await shot('mobile-state-detail');
  await mobile.close();
  assert.deepEqual(errors,[]);
  await writeFile(join(evidence,'result.json'),JSON.stringify({result:'passed',checks:['Exact object-state field opens its current value, desktop/narrow keyboard/Escape and touch'],errors,browser:browser.version()},null,2));
  await browser.close();await writeFile(join(evidence,'browser-closed.json'),JSON.stringify({closed:true}));process.exit(0);
 }
 assert.equal(await page.locator('.canvas-port[data-port-id]').count(),7);
 assert.equal(await page.locator('[data-target-port="inbox-submit"]').count(),1);
 assert.equal(await page.getByTestId('inspector').isVisible(),false);
 assert.equal(await page.getByTestId('canvas-state-daemon-inbox').getAttribute('data-current-value'),'无待收消息');
 const endpoints=await page.evaluate(()=>Array.from(document.querySelectorAll('.canvas-call')).map(group=>{
  const path=group.querySelector('.canvas-call__line'),point=path.getPointAtLength(path.getTotalLength());
  const pin=document.querySelector(`[data-testid="canvas-port-${group.dataset.targetPort}"]`);
  return {port:group.dataset.targetPort,error:Math.hypot(point.x-Number(pin.dataset.pinX),point.y-Number(pin.dataset.pinY))};
 }));assert(endpoints.every(e=>e.error<0.01));
 await shot('01-object-ports-state');
 await page.getByTestId('canvas-port-inbox-prepare').focus();await page.keyboard.press('Enter');
 assert.match(await page.getByTestId('port-contract').innerText(),/Inbox.Prepare/);await page.keyboard.press('Escape');
 checks.push('Seven visible named pins; every scenario call terminates at its exact Port; direct keyboard contract entry');
 await page.getByTestId('speed-select').selectOption('0.5');await page.getByTestId('step-next').click();
 await page.getByTestId('flow-marker').waitFor();
 assert.match(await page.getByTestId('flow-marker').getAttribute('data-payload'),/requestId=/);
 assert.match(await page.getByTestId('canvas-payload').textContent(),/text: 待处理的业务消息/,'Business payload remains readable on the canvas');
 const samples=await page.evaluate(()=>new Promise(resolve=>{
  const values=[];const start=performance.now();function frame(){const marker=document.querySelector('[data-testid="flow-marker"]');const p=marker.transform.baseVal.consolidate().matrix;values.push({x:p.e,y:p.f});if(performance.now()-start>180)resolve(values);else requestAnimationFrame(frame);}requestAnimationFrame(frame);
 }));assert(Math.hypot(samples.at(-1).x-samples[0].x,samples.at(-1).y-samples[0].y)>2);
 await page.getByTestId('step-play').click();
 assert.equal(await page.getByTestId('canvas-state-daemon-inbox').getAttribute('data-current-value'),'无待收消息');
 assert.equal(await page.getByTestId('canvas-state-daemon-inbox').getAttribute('data-applied'),'false');
 await shot('02-payload-prior-state');
 const progress=await page.getByTestId('flow-marker').getAttribute('data-progress');
 await page.getByTestId('view-sequence').click();await page.getByTestId('view-relations').click();
 assert.equal(await page.getByTestId('flow-marker').getAttribute('data-progress'),progress);
 await page.getByTestId('speed-select').selectOption('2');await page.getByTestId('step-play').click();
 await page.locator('[data-testid="current-call"][data-call-stage="returning"]').waitFor({timeout:12000});
 assert.equal(await page.getByTestId('flow-marker').getAttribute('data-path-direction'),'reverse');
 assert.match(await page.getByTestId('flow-marker').getAttribute('data-payload'),/待确认/);
 await shot('03-return-to-caller');await applied();
 assert.equal(await page.getByTestId('canvas-state-daemon-inbox').getAttribute('data-current-value'),'1 条待收');
 assert.match(await page.getByTestId('canvas-state-daemon-inbox').textContent(),/无待收消息.*→.*1 条待收/s);
 await shot('04-applied-object-state');
 checks.push('Real payload motion and return on the Port path, frozen prior state, pause/view continuity, exact applied state difference');
 await open('appserver-mail',2);await page.getByTestId('step-next').click();await applied();
 assert.match(await page.getByTestId('canvas-state-daemon-inbox').getAttribute('data-current-value'),/Issuing/);
 assert.equal(await page.getByTestId('current-call').getAttribute('data-call-stage'),'unknown');
 await open('appserver-disconnect');await page.getByTestId('step-next').click();await applied();assert(await page.getByTestId('flow-break').count()>0);
 checks.push('Unknown native input and non-call disconnect keep their distinct meanings');
 for(const scenario of model.scenarios){
  await open(scenario.id);
  const geometry=await page.evaluate(()=>({width:innerWidth,document:document.documentElement.scrollWidth,boxes:Array.from(document.querySelectorAll('.map-node__body')).map(n=>{const b=n.getBBox();return{x:b.x,y:b.y,w:b.width,h:b.height};})}));
  assert(geometry.document<=geometry.width+1);
  for(let i=0;i<geometry.boxes.length;i++)for(let j=i+1;j<geometry.boxes.length;j++){
   const a=geometry.boxes[i],b=geometry.boxes[j];assert(a.x+a.w<=b.x||b.x+b.w<=a.x||a.y+a.h<=b.y||b.y+b.h<=a.y,`Object bodies overlap in ${scenario.id}`);
  }
 }
 checks.push('All14 runtime scenario canvases keep object bodies separate and document width bounded');
 await page.goto(`${pathToFileURL(example).href}#mode=lookup&scenario=read-record&step=4&view=relations&speed=0.5`);
 await page.getByTestId('event-phase').waitFor();assert.equal(await page.locator('.canvas-state').count(),3);await shot('05-generic-object-surface');
 const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,reducedMotion:'reduce'});
 page=await mobile.newPage();page.on('pageerror',e=>errors.push(e.message));await open();
 await page.getByTestId('canvas-port-inbox-submit').tap();await page.getByTestId('port-contract').waitFor();await page.getByTestId('detail-close').tap();
 await page.getByTestId('canvas-state-daemon-inbox').tap();assert.match(await page.getByTestId('inspector').innerText(),/当前 · 无待收消息/);await page.getByTestId('detail-close').tap();
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);await shot('06-mobile-object-surface');
 await mobile.close();checks.push('Offline generic state projections and390px direct pin/state touch access');
 assert.deepEqual(errors,[]);await writeFile(join(evidence,'result.json'),JSON.stringify({result:'passed',checks,errors,endpoints,samples,browser:browser.version()},null,2));
}catch(error){if(page&&!page.isClosed())await shot('failure');await writeFile(join(evidence,'failure.json'),JSON.stringify({message:error.message,checks,errors},null,2));throw error;}
finally{await browser.close();await writeFile(join(evidence,'browser-closed.json'),JSON.stringify({closed:true}));}
