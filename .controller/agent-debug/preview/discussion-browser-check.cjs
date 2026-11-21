// Bounded explanation checks only; this does not exercise any AgentX socket or Provider.
const fs = require('node:fs');
const path = require('node:path');
const {chromium} = require('/home/long/.local/lib/node_modules/@longwork/uiux/node_modules/playwright-core');
const out = __dirname;
const url = process.env.AGENT_SOCKET_PREVIEW_URL || 'http://192.168.2.178:39731/';
(async () => {
  const browser = await chromium.launch({executablePath: process.env.AGENT_SOCKET_BROWSER_PATH || '/home/long/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome', headless: true, args: ['--no-sandbox']});
  try {
  const page = await browser.newPage({viewport: {width:1440,height:1000}, deviceScaleFactor:1});
  const errors = [], observations = [];
  page.on('pageerror', error => errors.push(error.message));
  const check = (name, passed, detail) => {observations.push({name, passed, detail}); if(!passed) throw new Error(name + ': ' + detail)};
  await page.goto(url);
  check('initial panorama', await page.locator('.architecture').count()===2 && await page.locator('.seq-row').count()===7, 'Paired ownership maps and all seven sequence rows present without interaction');
  await page.screenshot({path:path.join(out,'discussion-desktop.png'),fullPage:true});
  for(const [id,label] of [['disconnect','daemon 断线'],['mismatch','协议不兼容'],['provider','Provider 停滞'],['lost','确认响应丢失'],['host','宿主崩溃']]) {
    await page.getByRole('button',{name:label,exact:true}).click();
    check('linked failure '+id, (await page.locator('#proposedMap .edge.broken').count())>0 && await page.locator('#failureRows tr.current').getAttribute('data-scene')===id, await page.locator('#states').innerText());
  }
  await page.getByRole('button',{name:'Provider 停滞',exact:true}).click();
  await page.locator('#sequence [data-step="4"]').click();
  check('step highlights actual objects in both maps',await page.locator('.architecture .node.selected').count()===4 && await page.locator('#proposedMap .edge.selected').count()===1, 'Both adapter and Provider are selected, along with the input edge');
  await page.screenshot({path:path.join(out,'discussion-provider.png'),fullPage:true});
  await page.getByRole('button',{name:'协议不兼容',exact:true}).click();
  await page.screenshot({path:path.join(out,'discussion-mismatch.png'),fullPage:true});
  await page.getByRole('button',{name:'确认响应丢失',exact:true}).click();
  const socket = page.locator('#proposedMap [data-object="socket"]');
  await socket.focus(); await page.keyboard.press('Enter');
  check('keyboard object selection and focus retained', await socket.evaluate(el=>el===document.activeElement) && await page.locator('.architecture .node.selected').count()===2, await page.locator('#focusNote').innerText());
  await page.locator('#sequence [data-step="6"]').focus(); await page.keyboard.press('Enter');
  check('receipt response path selection',await page.locator('#proposedMap .edge.selected').count()>=1,await page.locator('#focusNote').innerText());
  await page.screenshot({path:path.join(out,'discussion-lost-reply.png'),fullPage:true});
  await page.setViewportSize({width:390,height:844});
  await page.getByRole('button',{name:'正常交付',exact:true}).click();
  await page.screenshot({path:path.join(out,'discussion-mobile.png'),fullPage:true});
  check('narrow page remains contained',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'390px page; diagrams retain readable type in local horizontal scrollers');
  const portable = await browser.newPage({viewport:{width:1280,height:900}});
  const externalRequests=[];
  portable.on('request',r=>{if(!r.url().startsWith('file:'))externalRequests.push(r.url())});
  portable.on('pageerror',error=>errors.push(error.message));
  await portable.goto('file://'+path.join(out,'index.html'));
  await portable.getByRole('button',{name:'daemon 断线',exact:true}).click();
  check('portable offline HTML',await portable.locator('#proposedMap .edge.broken').count()>0 && externalRequests.length===0,'Direct file open, central interaction works, no external resource requests');
  check('no browser script errors',errors.length===0,errors);
  fs.writeFileSync(path.join(out,'discussion-check.json'), JSON.stringify({url,viewports:[{width:1440,height:1000},{width:390,height:844}],observations,errors,externalRequests,limits:'Static simulated explanation only. No production socket, daemon, Provider or remote-device execution.'},null,2)+'\n');
  console.log(JSON.stringify({passed:observations.length,errors,artifact:path.join(out,'discussion-check.json')}));
  } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exitCode=1});
