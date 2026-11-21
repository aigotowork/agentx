/** Infrastructure-only real extension probe; never accepted as product evidence. */
import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {dirname,join} from 'node:path';
import {chromium,config,browserExecutable,save,until} from './common.mjs';
const probe=join(dirname(config.profile),'host-probe');await mkdir(probe);
await writeFile(join(probe,'manifest.json'),JSON.stringify({manifest_version:3,name:'Disposable host probe',version:'1.0',permissions:['sidePanel','activeTab','scripting'],host_permissions:['http://127.0.0.1/*'],background:{service_worker:'worker.js'},side_panel:{default_path:'panel.html'},action:{default_title:'probe'},commands:{_execute_action:{suggested_key:{default:'Alt+Shift+A'}}}}));
await writeFile(join(probe,'worker.js'),"chrome.sidePanel.setPanelBehavior({openPanelOnActionClick:true}); chrome.runtime.onInstalled.addListener(()=>{});");
await writeFile(join(probe,'panel.html'),'<button id="open">Open actual side panel</button><div>Disposable host probe</div><script src="panel.js"></script>');
await writeFile(join(probe,'panel.js'),"document.querySelector('#open').onclick=async()=>{try{await chrome.sidePanel.open({windowId:(await chrome.windows.getCurrent()).id});document.body.dataset.open='ok'}catch(e){document.body.dataset.open=e.message}};");
let browser;
try{
  browser=await chromium.launchPersistentContext(config.profile,{executablePath:browserExecutable,headless:false,ignoreDefaultArgs:['--disable-extensions'],args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server',`--disable-extensions-except=${probe}`,`--load-extension=${probe}`],viewport:{width:1100,height:820}});
  const worker=browser.serviceWorkers()[0]??await browser.waitForEvent('serviceworker');
  const extensionId=new URL(worker.url()).host;
  const page=await browser.newPage();await page.goto(`chrome-extension://${extensionId}/panel.html`);
  const discovery=await page.evaluate(async url=>{try{const response=await fetch(url);return{status:response.status,text:await response.text(),runtimeId:chrome.runtime.id}}catch(error){return{error:error.message}}},config.url+'/api/v1/management');
  await page.locator('#open').click();
  const sidePanel=await until('sidePanel actual API completion',()=>page.locator('body').getAttribute('data-open'));
  const session=await browser.newCDPSession(page);const targets=await session.send('Target.getTargets');
  await save('host-probe',{purpose:'Infrastructure only; not packaged product acceptance',version:browser.browser()?.version(),extensionId,worker:worker.url(),discovery,sidePanel,targets,executable:browserExecutable});
  assert.equal(discovery.status,200,JSON.stringify(discovery));assert.equal(sidePanel,'ok');
}finally{await browser?.close();}
