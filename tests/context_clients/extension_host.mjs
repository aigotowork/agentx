/** Genuine packaged sidePanel host, extracted from the successful attachment probe. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createServer} from 'node:net';
import {readFile,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {chromium,browserExecutable} from './common.mjs';
const require=createRequire(import.meta.url);
const puppeteer=require('puppeteer-core');

/** Load immutable unpacked bytes in an owned profile and invoke the actual browser action. */
export async function openExtensionHost(config,sourceUrl){
  assert.equal(new URL(sourceUrl).hostname,'127.0.0.1');
  const manifest=JSON.parse(await readFile(config.extension+'/manifest.json','utf8'));
  assert.equal(manifest.manifest_version,3);
  const socket=createServer();await new Promise(resolve=>socket.listen(0,'127.0.0.1',resolve));
  const port=socket.address().port;await new Promise(resolve=>socket.close(resolve));
  let context,connection;
  const close=async()=>{await connection?.disconnect();await context?.close();};
  try{
    context=await chromium.launchPersistentContext(config.profile,{executablePath:browserExecutable,headless:false,
      ignoreDefaultArgs:['--disable-extensions'],viewport:{width:1200,height:860},locale:'zh-CN',
      args:['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server',`--remote-debugging-port=${port}`,
        '--remote-debugging-address=127.0.0.1',`--disable-extensions-except=${config.extension}`,`--load-extension=${config.extension}`]});
    const suffix='/'+manifest.background.service_worker;
    const worker=context.serviceWorkers().find(worker=>worker.url().endsWith(suffix))??await context.waitForEvent('serviceworker',{predicate:worker=>worker.url().endsWith(suffix)});
    const actual=await worker.evaluate(()=>({id:chrome.runtime.id,manifest:chrome.runtime.getManifest()}));
    assert.equal(actual.manifest.name,manifest.name);assert.equal(actual.manifest.version,manifest.version);
    const originalCommands=await worker.evaluate(()=>chrome.commands.getAll());
    const settings=await context.newPage();await settings.goto('chrome://extensions/shortcuts');
    // Actual browser-owned settings in this disposable profile; never a chrome.* mock.
    await settings.evaluate(async id=>{await chrome.developerPrivate.updateExtensionCommand({extensionId:id,commandName:'_execute_action',keybinding:'Ctrl+Shift+Y'});},actual.id);
    await settings.close();
    const source=await context.newPage();await source.goto(sourceUrl);await source.bringToFront();
    const invocation=execFileSync('python3',['tests/context_clients/x11_keys.py','Control_L','Shift_L','y'],{env:process.env}).toString();
    connection=await puppeteer.connect({browserURL:`http://127.0.0.1:${port}`});
    const target=await connection.waitForTarget(target=>target.url()===`chrome-extension://${actual.id}/panel.html`,{timeout:30000});
    const panel=await target.asPage();
    assert.equal(await panel.evaluate(()=>chrome.runtime.id),actual.id);
    const evidence={extension:config.extension,manifest,version:context.browser()?.version(),id:actual.id,
      worker:worker.url(),panel:panel.url(),originalCommands,commands:await worker.evaluate(()=>chrome.commands.getAll()),
      invocation,method:'Browser-owned shortcut configuration in fresh profile, actual X11 action, genuine native sidePanel attached with Puppeteer Target.asPage',
      limitation:'Suggested shortcut may be absent/collide; this proves configured browser action, not a physical toolbar click or default shortcut assignment.'};
    await writeFile(`${config.evidence}/extension-host.json`,JSON.stringify(evidence,null,2)+'\n');
    return{context,connection,worker,panel,source,close};
  }catch(error){await close();throw error;}
}
