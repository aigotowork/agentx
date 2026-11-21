/** A native TS design is understandable through the same exported modules/operations/processes. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
const [artifact,evidence]=process.argv.slice(2);assert(artifact&&evidence);
const require=createRequire(import.meta.url),{chromium}=require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
await mkdir(evidence,{recursive:true});let page;const errors=[];
try{
 const context=await browser.newContext({viewport:{width:1366,height:768}});page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));await page.goto(pathToFileURL(artifact).href);
 assert.equal(await page.locator('.design-port').count(),2);
 await page.getByTestId('operation:inventory.reserve').focus();await page.keyboard.press('Enter');assert.match(await page.getByTestId('design-details').innerText(),/quantity/);assert.match(await page.getByTestId('design-details').innerText(),/Design\/code correspondence/);
 await page.keyboard.press('Escape');assert.equal(await page.getByTestId('design-details').isVisible(),false);assert.equal(await page.evaluate(()=>document.activeElement.dataset.testid),'operation:inventory.reserve');
 await page.getByTestId('design-process').click();assert.equal(await page.locator('.design-node').count(),7);
 await page.getByTestId('step:reserve-stock').click();assert.match(await page.getByTestId('design-details').innerText(),/已绑定/);assert.match(await page.getByTestId('design-details').innerText(),/Rejected/);await page.keyboard.press('Escape');
 await page.screenshot({path:join(evidence,'design-process.png')});await page.getByTestId('design-lint').click();assert.match(await page.getByTestId('design-details').innerText(),/arbitrary source is not inspected/);await page.keyboard.press('Escape');
 const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});page=await mobile.newPage();page.on('pageerror',e=>errors.push(e.message));await page.goto(pathToFileURL(artifact).href);await page.getByTestId('operation:orders.prepare').tap();assert(await page.getByTestId('design-details').isVisible());assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);await page.screenshot({path:join(evidence,'design-narrow.png')});assert.deepEqual(errors,[]);
 await writeFile(join(evidence,'result.json'),JSON.stringify({result:'passed',errors,checks:['Native Schema and documents local inspection','Declared branches and exact step bindings','Keyboard/Escape restores same target','390px touch/internal scroll','Coverage disclosure'],browser:browser.version()},null,2));
}catch(error){if(page&&!page.isClosed())await page.screenshot({path:join(evidence,'failure.png')});await writeFile(join(evidence,'failure.json'),JSON.stringify({message:error.message,errors},null,2));throw error;}finally{await browser.close();await writeFile(join(evidence,'closed.json'),JSON.stringify({closed:true}));}
