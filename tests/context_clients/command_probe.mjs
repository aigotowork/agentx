/** Diagnose actual browser accelerator delivery without native services or product edits. */
import {createRequire} from 'node:module';
import {mkdtemp,rm,writeFile,mkdir} from 'node:fs/promises';
import {execFileSync,spawn} from 'node:child_process';
import {createServer} from 'node:net';
const require=createRequire(import.meta.url);
const {chromium}=require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const root=await mkdtemp('/tmp/context-command-probe-'),evidence=process.argv[2];await mkdir(evidence);
const stage=async(name,value={})=>writeFile(evidence+'/stage.json',JSON.stringify({name,root,...value},null,2));await stage('launch');
const installed=process.env.CONTEXT_INSTALLED_CHROME==='1';
const debugSocket=createServer();await new Promise(r=>debugSocket.listen(0,'127.0.0.1',r));const debugPort=debugSocket.address().port;await new Promise(r=>debugSocket.close(r));
let raw,browser,context;
if(process.env.CONTEXT_RAW_CHROME==='1'){
  const socket=createServer();await new Promise(r=>socket.listen(0,'127.0.0.1',r));const port=socket.address().port;await new Promise(r=>socket.close(r));
  raw=spawn('/home/long/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome',['--no-sandbox','--disable-dev-shm-usage','--enable-unsafe-extension-debugging','--no-first-run','--no-default-browser-check','--disable-extensions-except='+process.cwd()+'/.artifacts/context-clients/agentx-browser-context-1.0.0','--load-extension='+process.cwd()+'/.artifacts/context-clients/agentx-browser-context-1.0.0',`--user-data-dir=${root}`,`--remote-debugging-port=${port}`,'about:blank'],{stdio:'ignore'});
  const deadline=Date.now()+15000;let endpoint;while(Date.now()<deadline){try{endpoint=await(await fetch(`http://127.0.0.1:${port}/json/version`)).json();break;}catch{await new Promise(r=>setTimeout(r,100));}}
  browser=await chromium.connectOverCDP(endpoint.webSocketDebuggerUrl);context=browser.contexts()[0];
}else context=await chromium.launchPersistentContext(root,{executablePath:installed?'/usr/bin/google-chrome':'/home/long/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome',headless:false,ignoreDefaultArgs:['--disable-extensions'],args:['--no-sandbox','--disable-dev-shm-usage','--enable-unsafe-extension-debugging',`--remote-debugging-port=${debugPort}`,...installed?[]:['--disable-extensions-except='+process.cwd()+'/.artifacts/context-clients/agentx-browser-context-1.0.0','--load-extension='+process.cwd()+'/.artifacts/context-clients/agentx-browser-context-1.0.0']]});
try{
  await stage('browser-started');
  if(installed){const cdp=await context.browser().newBrowserCDPSession();console.log(await cdp.send('Extensions.loadUnpacked',{path:process.cwd()+'/.artifacts/context-clients/agentx-browser-context-1.0.0'}));}
  const productWorker='chrome-extension://fjikfhjcmfmfgkbhpjlfonbenichgakc/background.js';
  const worker=context.serviceWorkers().find(w=>w.url()===productWorker)??await context.waitForEvent('serviceworker',{predicate:w=>w.url()===productWorker});
  const capabilities=await worker.evaluate(async()=>{for(let i=0;i<50&&!chrome.commands;i++)await new Promise(r=>setTimeout(r,100));return{keys:Object.keys(chrome),runtime:chrome.runtime?.id,manifest:chrome.runtime?.getManifest?.()};});
  const commands=await worker.evaluate(()=>chrome.commands?.getAll());const behavior=await worker.evaluate(()=>chrome.sidePanel?.getPanelBehavior());
  await stage('worker',{commands,behavior});
  let configured;
  if(process.env.CONTEXT_CONFIGURE_COMMAND==='1'){
    const settings=await context.newPage();await settings.goto('chrome://extensions/shortcuts');
    configured=await settings.evaluate(async()=>{await chrome.developerPrivate.updateExtensionCommand({extensionId:'fjikfhjcmfmfgkbhpjlfonbenichgakc',commandName:'_execute_action',keybinding:'Ctrl+Shift+Y'});return true;});
    await settings.close();await stage('configured',{commands:await worker.evaluate(()=>chrome.commands.getAll())});
  }
  const page=await context.newPage();await page.goto('about:blank');await page.evaluate(()=>{window.keyLog=[];window.addEventListener('keydown',e=>keyLog.push({key:e.key,alt:e.altKey,shift:e.shiftKey}));});await page.bringToFront();
  const ordinary=execFileSync('python3',['tests/context_clients/x11_keys.py','b']).toString();
  const shortcut=execFileSync('python3',['tests/context_clients/x11_keys.py',...configured?['Control_L','Shift_L','y']:['Alt_L','Shift_L','a']]).toString();
  let actualPanel;
  if(configured){const puppeteer=require('puppeteer-core');const connection=await puppeteer.connect({browserURL:`http://127.0.0.1:${debugPort}`});try{const target=await connection.waitForTarget(t=>t.url()==='chrome-extension://fjikfhjcmfmfgkbhpjlfonbenichgakc/panel.html',{timeout:15000});const panel=await target.asPage();actualPanel={url:panel.url(),runtime:await panel.evaluate(()=>chrome.runtime.id),body:await panel.locator('body').waitHandle().then(h=>h.evaluate(e=>e.innerText))};}finally{await connection.disconnect();}}
  await page.waitForFunction(()=>window.keyLog.length>0,{timeout:2000}).catch(()=>{});
  const session=await context.newCDPSession(page);const targets=await session.send('Target.getTargets');const commandLine=await session.send('Browser.getBrowserCommandLine').catch(e=>({error:e.message}));const version=await session.send('Browser.getVersion');const bounds=await session.send('Browser.getWindowForTarget');
  execFileSync('python3',['-c',`from PIL import ImageGrab; ImageGrab.grab().save('${evidence}/host.png')`]);
  await writeFile(evidence+'/probe.json',JSON.stringify({worker:worker.url(),capabilities,commands,configured,actualPanel,afterCommands:await worker.evaluate(()=>chrome.commands?.getAll()),behavior,ordinary,shortcut,commandLine,version,bounds,keys:await page.evaluate(()=>keyLog),targets,pages:context.pages().map(p=>p.url())},null,2));
}finally{if(raw){await browser.close();if(raw.exitCode==null){raw.kill();await new Promise(r=>raw.once('exit',r));}}else await context.close();await rm(root,{recursive:true});}
