/** Final origin/embedding check for the exact published bytes under offline and LAN hosting. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {writeFile,mkdir} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
const [artifact,evidence]=process.argv.slice(2);
await mkdir(evidence,{recursive:true});
const {chromium}=createRequire(import.meta.url)('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
const results=[];
try{
  for(const url of [pathToFileURL(artifact).href,'http://192.168.2.178:18959/']){
    const context=await browser.newContext({viewport:{width:1440,height:900}}),page=await context.newPage(),errors=[],requests=[];
    page.on('pageerror',error=>errors.push(error.message));page.on('request',request=>requests.push(request.url()));
    await page.goto(url);await page.waitForFunction(()=>globalThis.FlowAtlasWorkbench);
    const model=await page.locator('#workbench-model').evaluate(node=>JSON.parse(node.textContent));
    assert.equal(model.format,'flow-atlas/workbench-1');assert(model.definition.operations.every(op=>op.nativeMethodId&&op.nativeOutputSchemaId));
    await page.getByTestId('scenario-select').selectOption('scenario.lost-response');
    await page.getByTestId('player-position').evaluate(node=>{node.value=node.max;node.dispatchEvent(new Event('input',{bubbles:true}));});
    const current=await page.evaluate(()=>FlowAtlasWorkbench.state());
    assert.equal(current.checkpoint.states['order-42-view'].status,'awaiting');assert.equal(current.checkpoint.derived['stock-book']['field.available'],18);
    assert(current.checkpoint.calls.some(call=>call.phase==='unknown'&&call.effectOutcomeId==='reserved'&&call.completion==='local-outcome'));
    assert.deepEqual(errors,[]);assert(requests.every(request=>request===url),'No external resource or API request');
    if(url.startsWith('http:'))await page.screenshot({path:join(evidence,'lan-final.png')});
    results.push({url,errors,requests,checkpointIndex:current.checkpoint.index,phase:current.checkpoint.phase});
    await context.close();
  }
  await writeFile(join(evidence,'result.json'),JSON.stringify({result:'passed',results,browser:browser.version()},null,2)+'\n');
}finally{
  await browser.close();await writeFile(join(evidence,'closed.json'),JSON.stringify({closed:true})+'\n');
}
