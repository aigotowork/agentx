/** Actual UI outcome: move/cancel/pin/exchange without changing semantic playback. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
const [artifact,evidence]=process.argv.slice(2);assert(artifact&&evidence);
const require=createRequire(import.meta.url),{chromium}=require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
await mkdir(evidence,{recursive:true});
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
let page;const errors=[],checks=[];
const url=pathToFileURL(artifact).href+'#mode=appserver&scenario=appserver-mail&step=-1&view=relations&speed=0.25';
const position=id=>page.getByTestId('map-node-'+id).locator('.map-node__body').evaluate(n=>({x:Number(n.getAttribute('x')),y:Number(n.getAttribute('y'))}));
const handle=id=>page.getByTestId('layout-handle-'+id);
async function drag(id,dx,dy,cancel=false){const h=handle(id);await h.scrollIntoViewIfNeeded();const b=await h.boundingBox();const x=b.x+b.width/2,y=b.y+b.height/2;await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+dx,y+dy,{steps:6});if(cancel)await page.keyboard.press('Escape');await page.mouse.up();}
const menu=async()=>{if(await page.getByTestId('layout-menu').getAttribute('open')===null)await page.getByTestId('layout-toggle').click();};
try{
  const context=await browser.newContext({viewport:{width:1366,height:768},acceptDownloads:true,reducedMotion:'no-preference'});
  page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));await page.goto(url);await handle('daemon').waitFor();
  const start=await position('daemon');
  await drag('daemon',75,35);const moved=await position('daemon');assert(moved.x>start.x+60);assert(moved.y>start.y+25);
  assert.equal(await page.getByTestId('atlas-map').getAttribute('data-layout-undo-count'),'1');
  const endpoints=await page.evaluate(()=>Array.from(document.querySelectorAll('.canvas-call')).map(group=>{const path=group.querySelector('.canvas-call__line'),p=path.getPointAtLength(path.getTotalLength()),pin=document.querySelector(`[data-testid="canvas-port-${group.dataset.targetPort}"]`);return Math.hypot(p.x-Number(pin.dataset.pinX),p.y-Number(pin.dataset.pinY));}));assert(endpoints.every(x=>x<0.02));
  await drag('daemon',55,-15,true);assert.deepEqual(await position('daemon'),moved);assert.equal(await page.getByTestId('atlas-map').getAttribute('data-layout-undo-count'),'1');
  await menu();await page.getByTestId('layout-undo').click();assert.deepEqual(await position('daemon'),start);
  await handle('daemon').focus();await page.keyboard.press('ArrowRight');assert.equal((await position('daemon')).x,start.x+10);
  await page.keyboard.press('Shift+ArrowDown');assert.equal((await position('daemon')).y,start.y+1);
  await page.keyboard.press('p');assert.equal(await page.getByTestId('map-node-daemon').getAttribute('data-layout-pinned'),'true');
  const pinned=await position('daemon');await drag('daemon',50,30);assert.deepEqual(await position('daemon'),pinned);
  await handle('daemon').focus();await page.keyboard.press('p');
  await page.getByTestId('canvas-port-inbox-submit').click();assert(await page.getByTestId('port-contract').isVisible());await page.keyboard.press('Escape');
  await menu();const downloadEvent=page.waitForEvent('download');await page.getByTestId('layout-export').click();const download=await downloadEvent;const path=join(evidence,'layout.json');await download.saveAs(path);const exported=JSON.parse(await readFile(path,'utf8'));
  assert.equal(exported.scope.scenarioId,'appserver-mail');assert.equal(exported.positions.daemon.x,pinned.x);
  await page.getByTestId('layout-reset').click();assert.deepEqual(await position('daemon'),start);
  await page.getByTestId('layout-import-input').setInputFiles(path);await page.getByTestId('layout-feedback').filter({hasText:'布局已导入'}).waitFor();assert.deepEqual(await position('daemon'),pinned);
  const stale={...exported,contentDigest:'sha256:'+'0'.repeat(64)};await page.getByTestId('layout-import-input').setInputFiles({name:'stale.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(stale))});await page.getByTestId('layout-feedback').filter({hasText:'导入失败'}).waitFor();assert.deepEqual(await position('daemon'),pinned);
  await page.keyboard.press('Escape');
  checks.push('Real pointer move/cancel/one undo, exact Port endpoints, keyboard precision, pin, direct Port access, download/import/stale refusal');
  await page.getByTestId('step-next').click();await page.getByTestId('flow-marker').waitFor();
  await drag('daemon',18,8);assert.equal(await page.getByTestId('step-play').getAttribute('aria-pressed'),'false');
  const phase=await page.getByTestId('event-phase').getAttribute('data-phase'),progress=await page.getByTestId('flow-marker').getAttribute('data-progress');
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  assert.equal(await page.getByTestId('event-phase').getAttribute('data-phase'),phase);assert.equal(await page.getByTestId('flow-marker').getAttribute('data-progress'),progress);
  await page.screenshot({path:join(evidence,'layout-desktop.png')});checks.push('Drag during transfer pauses and keeps exact marker progress and checkpoint');
  await page.getByTestId('view-flow').click();await page.getByTestId('view-relations').click();assert.equal(await page.getByTestId('flow-marker').getAttribute('data-progress'),progress);
  const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,reducedMotion:'reduce'});
  page=await mobile.newPage();page.on('pageerror',e=>errors.push(e.message));await page.goto(url);await handle('daemon').waitFor();
  // Touch users can move without a precision gesture through local directional controls.
  await handle('daemon').scrollIntoViewIfNeeded();await handle('daemon').tap();if(await page.getByTestId('inspector').isVisible())await page.getByTestId('detail-close').tap();
  const mobileStart=await position('daemon');await page.getByTestId('layout-toggle').tap();await page.getByTestId('layout-move-right').tap();assert.equal((await position('daemon')).x,mobileStart.x+10);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);
  await page.screenshot({path:join(evidence,'layout-narrow.png')});checks.push('390px touch selection and local move controls; document width bounded');
  assert.deepEqual(errors,[]);await writeFile(join(evidence,'result.json'),JSON.stringify({result:'passed',checks,errors,endpoints,browser:browser.version()},null,2));
}catch(error){if(page&&!page.isClosed())await page.screenshot({path:join(evidence,'failure.png')});await writeFile(join(evidence,'failure.json'),JSON.stringify({message:error.message,checks,errors},null,2));throw error;}
finally{await browser.close();await writeFile(join(evidence,'closed.json'),JSON.stringify({closed:true}));}
