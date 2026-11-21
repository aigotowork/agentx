/** Browser acceptance for actual capability selection, call/reply/result and shared stages. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
const [origin, example, evidence] = process.argv.slice(2);
assert(origin && example && evidence);
const require = createRequire(import.meta.url);
const { chromium } = require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({ executablePath:'/usr/bin/google-chrome', headless:true, args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server'] });
await mkdir(evidence,{recursive:true});
const errors=[], requests=[], checks=[];
let page;
const shot = async name => page.screenshot({path:join(evidence,name+'.png')});
const phase = async value => page.locator(`[data-testid="event-phase"][data-phase="${value}"]`).waitFor({timeout:20000});
async function runtime(scenario,step=-1,view='relations',speed=2) {
 await page.goto(`${origin}#mode=appserver&scenario=${scenario}&step=${step}&view=${view}&speed=${speed}`);
 await page.getByTestId('event-phase').waitFor();
}
async function generic(scenario,step=-1,view='sequence') {
 await page.goto(`${pathToFileURL(example).href}#mode=lookup&scenario=${scenario}&step=${step}&view=${view}&speed=2`);
 await page.getByTestId('event-phase').waitFor();
}
async function geom() {
 const geometry=await page.evaluate(()=>({width:innerWidth,documentWidth:document.documentElement.scrollWidth,map:document.getElementById('atlas-map').getBoundingClientRect().toJSON(),facts:document.getElementById('facts').getBoundingClientRect().toJSON(),dock:document.getElementById('step-detail').getBoundingClientRect().toJSON()}));
 assert(geometry.documentWidth<=geometry.width+1);
 assert(geometry.facts.bottom<=844 || geometry.width>600);
 return geometry;
}
try {
 const context=await browser.newContext({viewport:{width:1366,height:768},reducedMotion:'no-preference'});
 page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>requests.push(r.url()));
 await runtime('appserver-mail');
 await page.getByTestId('map-ports-daemon').focus();await page.keyboard.press('Enter');
 await page.getByTestId('port-select-inbox-submit').click();
 await page.getByTestId('port-contract').waitFor();
 assert.equal(await page.locator('#inspector-content').evaluate(element=>element.scrollTop),0,'A selected contract starts at its signature');
 assert.match(await page.getByTestId('inspector').innerText(),/Messaging.Submit.*requestId/s);
 assert.match(await page.getByTestId('inspector').innerText(),/已持久接受/);
 assert.match(await page.getByTestId('inspector').innerText(),/提交被拒绝/);
 assert.match(await page.getByTestId('inspector').innerText(),/接受结果待查询/);
 await shot('01-port-contract');await page.keyboard.press('Escape');
 assert.equal(await page.getByTestId('inspector').isVisible(),false);
 checks.push('Object capability inventory, keyboard entry, input/result/source contract, local Escape close');

 await page.getByTestId('speed-select').selectOption('0.25');
 const before=await page.getByTestId('facts').innerText();
 await page.getByTestId('step-next').click();await phase('flow');
 await page.getByTestId('flow-marker').waitFor();
 assert.equal(await page.getByTestId('current-call').getAttribute('data-result-known'),'false');
 assert.match(await page.getByTestId('current-call').innerText(),/Messaging.Submit.*调用中/);
 await page.getByTestId('step-play').click();
 const position=await page.getByTestId('flow-marker').getAttribute('data-progress');
 await page.getByTestId('current-call').click();
 assert.equal(await page.getByTestId('call-result-submit-message').count(),0,'No completed result while the request is in flight');
 assert.match(await page.getByTestId('inspector').innerText(),/结果尚未可知/);
 await shot('02-request-pending');await page.keyboard.press('Escape');
 await page.getByTestId('view-flow').click();await page.getByTestId('view-sequence').click();
 assert.equal(await page.getByTestId('flow-marker').getAttribute('data-progress'),position);
 assert.equal(await page.getByTestId('facts').innerText(),before);
 await page.getByTestId('sequence-port-persist').focus();await page.keyboard.press('Enter');
 assert.match(await page.getByTestId('port-contract').innerText(),/Messaging.Submit/);
 await page.keyboard.press('Escape');
 await page.getByTestId('speed-select').selectOption('2');await page.getByTestId('step-play').click();
 await page.locator('[data-testid="current-call"][data-call-stage="returning"]').waitFor({timeout:12000});
 assert.equal(await page.getByTestId('current-call').getAttribute('data-result-known'),'false');
 const replyDirection=await page.getByTestId('flow-marker').getAttribute('data-direction');
 assert.equal(replyDirection,'reverse');await shot('03-reply-in-progress');
 await phase('applied');
 if(await page.getByTestId('step-play').getAttribute('aria-pressed')==='true')await page.getByTestId('step-play').click();
 assert.equal(await page.getByTestId('current-call').getAttribute('data-result-known'),'true');
 assert.match(await page.getByTestId('current-call').innerText(),/已持久接受/);
 assert.match(await page.getByTestId('facts').innerText(),/1 条待收/);
 await page.getByTestId('current-call').click();
 assert.match(await page.getByTestId('call-result-submit-message').innerText(),/尚未交给 Provider/);
 assert.equal(await page.getByTestId('call-effect-inbox').getAttribute('data-applied'),'true');
 await shot('04-accepted-submission');await page.keyboard.press('Escape');
 checks.push('Visible request/reverse reply; result withheld until applied; state stays prior while paused view/rate/contract selection');

 await runtime('appserver-mail',2,'sequence');await page.getByTestId('step-next').click();await phase('applied');
 assert.equal(await page.getByTestId('current-call').getAttribute('data-call-stage'),'unknown');
 assert.match(await page.getByTestId('facts').innerText(),/Issuing/);
 assert.equal(await page.locator('[data-trace-step="3"][data-call-role="response"]').count(),0);
 await page.getByTestId('current-call').click();
 assert.match(await page.getByTestId('call-result-issue-input').innerText(),/尚无匹配/);
 await shot('05-input-unknown');await page.keyboard.press('Escape');
 await page.getByTestId('step-next').click();await phase('applied');
 assert.match(await page.getByTestId('current-call').innerText(),/已观察/);
 assert.match(await page.getByTestId('facts').innerText(),/Issuing/,'Observation alone cannot advance receipt');
 await page.getByTestId('step-next').click();await phase('applied');
 assert.match(await page.getByTestId('facts').innerText(),/本批次交付已确认/);
 await page.getByTestId('current-call').click();
 assert.match(await page.getByTestId('call-result-confirm-batch').innerText(),/工作完成另计/);
 await shot('06-confirmed-delivery');
 checks.push('Native issuance unknown, exact event observation, later inbox confirmation remain separate facts');

 await runtime('appserver-disconnect',-1,'sequence');await page.getByTestId('step-next').click();await phase('applied');
 assert.equal(await page.getByTestId('current-call').count(),0,'EOF is a disconnect, not an invented method');
 assert.equal(await page.getByTestId('sequence-break').count()>0,true,'The applied disconnection retains its static break symbol');
 await page.getByTestId('step-next').click();await phase('applied');
 assert.match(await page.getByTestId('current-call').innerText(),/ReleaseNativeRoot/);
 await page.getByTestId('step-next').click();await phase('applied');
 assert.equal(await page.getByTestId('current-call').count(),0);
 assert.match(await page.getByTestId('facts').innerText(),/没有请求停止共享后端/);
 checks.push('Disconnect stays non-call; exact root release does not imply stopping shared backend');

 await generic('read-record',3);await page.getByTestId('step-next').click();await phase('applied');
 assert.equal(await page.getByTestId('current-call').getAttribute('data-call-stage'),'accepted');
 await shot('07-generic-accepted');
 await generic('reject-selection');await page.getByTestId('step-next').click();await phase('applied');
 assert.equal(await page.getByTestId('current-call').getAttribute('data-call-stage'),'rejected');
 await page.getByTestId('current-call').click();
 assert.equal(await page.locator('.call-effect').count()>0,true);
 await shot('08-generic-rejected');await page.keyboard.press('Escape');
 await generic('unknown-selection');await page.getByTestId('step-next').click();await phase('applied');
 assert.equal(await page.getByTestId('current-call').getAttribute('data-call-stage'),'unknown');
 assert.equal(await page.locator('[data-call-role="response"]').count(),0);
 await shot('09-generic-unknown');
 checks.push('Same generic port reused in accepted/rejected/unknown scenarios; unknown has no fabricated response');

 await runtime('appserver-mail',0,'flow');await shot('10-flow-desktop');
 await page.getByTestId('view-relations').click();await shot('11-relations-desktop');
 const geometry=[await geom()];
 const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,reducedMotion:'reduce'});
 const desktop=page;page=await mobile.newPage();page.on('pageerror',e=>errors.push(e.message));
 await runtime('appserver-mail',-1,'flow');geometry.push(await geom());
 await page.getByTestId('step-next').tap();await phase('applied');
 await page.getByTestId('current-call').tap();await page.getByTestId('call-result-submit-message').waitFor();
 await shot('12-mobile-call');await page.getByTestId('detail-close').tap();
 await page.getByTestId('view-relations').tap();
 await page.getByTestId('map-ports-daemon').tap();await page.getByTestId('port-select-inbox-confirm').tap();
 await page.getByTestId('port-contract').waitFor();await shot('13-mobile-contract');
 await mobile.close();page=desktop;
 await writeFile(join(evidence,'geometry.json'),JSON.stringify(geometry,null,2));
 assert.deepEqual(errors,[]);
 assert(requests.filter(url=>/^https?:/.test(url)).every(url=>new URL(url).origin===new URL(origin).origin));
 await writeFile(join(evidence,'result.json'),JSON.stringify({result:'passed',browser:browser.version(),checks,errors},null,2));
} catch(error) {
 if(page&&!page.isClosed())await shot('failure');
 await writeFile(join(evidence,'failure.json'),JSON.stringify({message:error.message,checks,errors},null,2));throw error;
} finally {await browser.close();await writeFile(join(evidence,'browser-closed.json'),JSON.stringify({closed:true}));}
