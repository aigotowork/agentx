/** Shared viewport outcome: container ownership, unbounded pan, anchored zoom and node editing. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
const [runtime,design,evidence]=process.argv.slice(2);assert(runtime&&design&&evidence);
const require=createRequire(import.meta.url),{chromium}=require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
await mkdir(evidence,{recursive:true});
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
const results=[],errors=[];let page;
const read=()=>page.locator('.spatial-viewport').evaluate(host=>{const s=host.querySelector('svg'),b=s.getBoundingClientRect(),r=host.getBoundingClientRect(),v=s.viewBox.baseVal;return{x:Number(host.dataset.cameraX),y:Number(host.dataset.cameraY),zoom:Number(host.dataset.cameraZoom),width:r.width,height:r.height,left:r.left,top:r.top,svg:{x:b.x,y:b.y,w:b.width,h:b.height},ratio:v.width/v.height,scroll:host.scrollWidth-host.clientWidth,body:document.documentElement.scrollWidth,viewport:innerWidth};});
const near=(a,b,limit=0.05)=>assert(Math.abs(a-b)<limit,`${a} differs from ${b}`);
async function pan(dx,dy,button='left',cancel=false){const r=await read();const x=r.left+9,y=r.top+9;await page.mouse.move(x,y);await page.mouse.down({button});await page.mouse.move(x+dx,y+dy,{steps:5});if(cancel)await page.keyboard.press('Escape');await page.mouse.up({button});}
try{
 for(const [kind,artifact]of [['runtime',runtime],['design',design]]){
  const context=await browser.newContext({viewport:{width:1366,height:768}});page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
  await page.goto(pathToFileURL(artifact).href+(kind==='runtime'?'#mode=appserver&scenario=appserver-mail&view=relations&step=-1&speed=0.25':''));
  await page.getByTestId('viewport-fit').waitFor();
  let r=await read();near(r.svg.x,r.left);near(r.svg.y,r.top);near(r.svg.w,r.width);near(r.svg.h,r.height);near(r.ratio,r.width/r.height,0.0001);assert.equal(r.scroll,0);assert.equal(r.body,r.viewport);
  const initial=r;await pan(120,60);r=await read();near(r.x,initial.x-120/initial.zoom);near(r.y,initial.y-60/initial.zoom);
  const committed=r;await pan(90,25,'middle',true);r=await read();near(r.x,committed.x);near(r.y,committed.y);
  // Zoom preserves the world point under the cursor, regardless of content bounds.
  await page.locator('.spatial-viewport').evaluate(host=>host.addEventListener('wheel',event=>{const rect=host.getBoundingClientRect();window.testWheelAnchor={x:event.clientX-rect.left,y:event.clientY-rect.top};},{once:true,capture:true}));
  let point={x:170,y:100};await page.mouse.move(r.left+point.x,r.top+point.y);await page.mouse.wheel(0,340);
  await page.waitForFunction(previous=>Number(document.querySelector('.spatial-viewport').dataset.cameraZoom)!==previous,r.zoom);
  point=await page.evaluate(()=>window.testWheelAnchor);
  const zoomed=await read();near(r.x+point.x/r.zoom,zoomed.x+point.x/zoomed.zoom);near(r.y+point.y/r.zoom,zoomed.y+point.y/zoomed.zoom);
  await page.locator('.spatial-viewport').focus();await page.keyboard.press('ArrowLeft');const keyboard=await read();near(keyboard.x,zoomed.x-60/zoomed.zoom);
  await page.getByTestId('viewport-actual').click();near((await read()).zoom,1);
  await pan(1400,0,'middle');assert((await read()).x<initial.x-1000,'Pan is not clamped at the old content boundary');
  await page.getByTestId('viewport-fit').click();r=await read();near(r.x,initial.x);near(r.zoom,initial.zoom);
  if(kind==='runtime'){
   const handle=page.getByTestId('layout-handle-daemon'),box=await handle.boundingBox();
   const before=await page.getByTestId('map-node-daemon').locator('.map-node__body').evaluate(n=>Number(n.getAttribute('x')));
   await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2+30,box.y+box.height/2+12,{steps:5});await page.mouse.up();
   const after=await page.getByTestId('map-node-daemon').locator('.map-node__body').evaluate(n=>Number(n.getAttribute('x')));near(after-before,30/r.zoom,0.2);
   const endpoints=await page.evaluate(()=>Array.from(document.querySelectorAll('.canvas-call')).map(group=>{const path=group.querySelector('.canvas-call__line'),p=path.getPointAtLength(path.getTotalLength()),pin=document.querySelector(`[data-testid="canvas-port-${group.dataset.targetPort}"]`);return Math.hypot(p.x-Number(pin.dataset.pinX),p.y-Number(pin.dataset.pinY));}));assert(endpoints.every(distance=>distance<0.02));
   await page.getByTestId('step-next').click();await page.getByTestId('flow-marker').waitFor();await page.getByTestId('step-play').click();
   const progress=await page.getByTestId('flow-marker').getAttribute('data-progress');await pan(25,15,'middle');assert.equal(await page.getByTestId('flow-marker').getAttribute('data-progress'),progress);
   const saved=await read();await page.getByTestId('view-flow').click();await page.getByTestId('view-relations').click();r=await read();near(r.x,saved.x);near(r.y,saved.y);near(r.zoom,saved.zoom);
   await page.getByTestId('viewport-fit').click();
  }else{
   await page.getByTestId('design-process').click();await page.getByTestId('viewport-fit').click();await pan(30,15,'middle');const saved=await read();
   await page.getByTestId('step:reserve-stock').focus();await page.keyboard.press('Enter');await page.keyboard.press('Escape');r=await read();near(r.x,saved.x);near(r.zoom,saved.zoom);
  }
  await page.screenshot({path:join(evidence,kind+'-desktop.png')});
  const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});page=await mobile.newPage();page.on('pageerror',error=>errors.push(error.message));
  await page.goto(pathToFileURL(artifact).href+(kind==='runtime'?'#mode=appserver&scenario=appserver-mail&view=relations&step=-1':''));await page.getByTestId('viewport-fit').waitFor();r=await read();near(r.svg.w,r.width);near(r.svg.h,r.height);assert.equal(r.body,390);
  assert(r.zoom>=0.85,'Phone initial view keeps object text readable; full overview is explicit');
  await page.screenshot({path:join(evidence,kind+'-narrow.png')});
  const cdp=await page.context().newCDPSession(page),touch=(x,y,id)=>({x,y,id,radiusX:2,radiusY:2});
  const x=r.left+8,y=r.top+8;
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch(x,y,1)]});
  await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[touch(x+70,y+25,1)]});
  await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});const touchPanned=await read();assert(touchPanned.x<r.x-50/r.zoom);
  await page.getByTestId('viewport-fit').tap();r=await read();
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch(x+70,y+8,1)]});
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch(x+70,y+8,1),touch(x+170,y+8,2)]});
  await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[touch(x+45,y+8,1),touch(x+195,y+8,2)]});
  await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});assert((await read()).zoom>r.zoom*1.35,'Two-finger pinch changes the camera');
  await page.getByTestId('viewport-fit').tap();
  if(kind==='runtime'){await page.getByTestId('canvas-port-inbox-submit').tap();assert(await page.getByTestId('port-contract').isVisible());await page.getByTestId('detail-close').tap();}
  else{await page.getByTestId('operation:orders.prepare').tap();assert.match(await page.getByTestId('design-details').innerText(),/quantity/);await page.getByRole('button',{name:'关闭详情'}).tap();}
  await page.screenshot({path:join(evidence,kind+'-narrow-overview.png')});results.push({kind,containerAligned:true,unboundedPan:true,cursorAnchoredZoom:true,retainedCamera:true,touchPanPinch:true,readablePhoneInitial:true,errors:[]});await context.close();await mobile.close();
 }
 assert.deepEqual(errors,[]);await writeFile(join(evidence,'result.json'),JSON.stringify({result:'passed',results,errors,browser:browser.version()},null,2));
}catch(error){if(page&&!page.isClosed())await page.screenshot({path:join(evidence,'failure.png')});await writeFile(join(evidence,'failure.json'),JSON.stringify({message:error.message,results,errors},null,2));throw error;}
finally{await browser.close();await writeFile(join(evidence,'closed.json'),JSON.stringify({closed:true}));}
