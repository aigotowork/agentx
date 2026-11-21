// Run with PLAYWRIGHT_MODULE=/path/to/playwright node evaluation-browser.cjs baseline|candidate URL OUT_DIR [FIXTURE_JSON].
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const fs = require('node:fs/promises');
const path = require('node:path');
const [mode, target, out, fixtureInput] = process.argv.slice(2);
const baseline = 'http://192.168.2.178:18983/space/';
if (!['baseline', 'candidate'].includes(mode) || !target || !out ||
    (mode === 'baseline' && new URL(target).href !== baseline) ||
    (mode === 'candidate' && new URL(target).origin === new URL(baseline).origin)) {
  throw new Error('Use baseline and the fixed LAN URL, or candidate and a distinct isolated origin.');
}
const viewports = [
  ['dual', 1440, 900, false],
  ['single', 1280, 800, false],
  ['mobile', 375, 812, true],
  ['short', 1280, 540, false],
].filter(([name])=>!process.env.EVAL_VIEWPORTS||process.env.EVAL_VIEWPORTS.split(',').includes(name));
const draft = Array.from({ length: 18 }, (_, i) =>
  `第 ${i+1} 行：请核对上次回执以后产生的完整记录，标出仍需决定的问题、输出文件和下一步，不要覆盖已有草稿。`).join('\n');
const attachment = { id:'evaluation-attachment', name:'复核记录-原始摘录.txt', media_type:'text/plain',
  size:27, url:'/api/v1/attachments/evaluation-attachment' };
const attachmentBytes = 'Synthetic review excerpt.\n\n';
const fixture = fixtureInput ? require(path.resolve(fixtureInput)) : {};
const run = { mode, target, generatedAt: new Date().toISOString(), passes: {} };
const button = (root, name) => root.getByRole('button', { name, exact: true });
const history = (p, name) => p.getByLabel('会话历史', {exact:true}).filter({hasText:new RegExp(`\\[${name} / `)});
const section = (p, name) => history(p, name).locator('xpath=ancestor::section[@aria-label="当前会话"][1]');
const message = (p, name, id) => history(p, name).getByText(`[${name} / ${id}]`, { exact:false }).first().locator('xpath=ancestor::article[1]');
const relation = (root) => root.getByText('启动关系', {exact:true}).first();

async function captureCoords(h) {
  return h.evaluate(el => {
    const round = n => Math.round(n*10)/10;
    const h = el.getBoundingClientRect();
    const top = Math.max(0, h.top), bottom = Math.min(innerHeight, h.bottom);
    const left = Math.max(0,h.left), right = Math.min(innerWidth,h.right);
    const wholly = rect => rect.top >= top - .5 && rect.bottom <= bottom + .5 && rect.left >= left - .5 && rect.right <= right + .5;
    const intersects = rect => rect.top < bottom && rect.bottom > top && rect.left < right && rect.right > left;
    const scroller = el.closest('section');
    const geometry = x => x ? Object.fromEntries(['x','y','width','height'].map(k => [k,round(x.getBoundingClientRect()[k])])) : null;
    const editor=scroller?.querySelector('textarea');
    const messages=[...el.querySelectorAll('article')].map(a => {
      const rect=a.getBoundingClientRect();
      if (!intersects(rect)) return null;
      const body=a.querySelector('.markdown') || a;
      const lines=new Map(), walker=document.createTreeWalker(body,NodeFilter.SHOW_TEXT);
      const range=document.createRange();
      for(let t=walker.nextNode();t;t=walker.nextNode()){
        if(t.parentElement.closest('button, [role=status], .markdown-code-toolbar')) continue;
        for(let i=0;i<t.length;i++){
          if(/\s/.test(t.data[i])) continue;
          range.setStart(t,i);range.setEnd(t,i+1);
          const r=range.getBoundingClientRect();
          const box=t.parentElement.closest('pre')?.getBoundingClientRect();
          const local=r.width>0&&r.height>0&&(!box||(r.left>=box.left-.5&&r.right<=box.right+.5));
          if(r.width===0) continue;
          const key=round(r.top);
          const prev=lines.get(key)||{full:true,visible:false};
          prev.full &&= wholly(r)&&local;
          prev.visible ||= intersects(r)&&local;
          lines.set(key,prev);
        }
      }
      const counts=[...lines.values()];
      return {id:a.textContent.match(/\[([a-z]+ \/ \d+)\]/)?.[1], offset:round(rect.top-h.top),
        full:wholly(rect)&&counts.every(x=>x.full), fullLines:counts.filter(x=>x.full).length,
        partialLines:counts.filter(x=>x.visible&&!x.full).length, code:!!a.querySelector('pre'),
        attachment:a.textContent.includes('复核记录-原始摘录.txt')};
    }).filter(Boolean);
    return { reader:geometry(el), editor:geometry(editor),
      editorScroll:editor?[editor.clientHeight,editor.scrollHeight]:null,
      scrollTop:round(el.scrollTop), scrollHeight:el.scrollHeight, clientHeight:el.clientHeight,
      pageWidth:document.documentElement.scrollWidth, viewportWidth:innerWidth,
      full:messages.filter(x=>x.full).length, fullLines:messages.reduce((n,m)=>n+m.fullLines,0),
      partialLines:messages.reduce((n,m)=>n+m.partialLines,0), messages };
  });
}

async function safePhase(entry,key,fn) {
  try { entry[key]=await fn(); } catch(e) { entry[key]={error:String(e).slice(0,1600)}; }
}

async function stage(browser, name, width, height, touch, overlay) {
  const context=await browser.newContext({ viewport:{width,height}, isMobile:touch, hasTouch:touch,
    locale:'zh-CN', timezoneId:'America/New_York' });
  const p=await context.newPage();
  p.setDefaultTimeout(5500);
  const traffic=[], pending=[], entry={traffic};
  p.on('pageerror', e => (entry.pageErrors ??= []).push(String(e)));
  await p.route('**/*',async route=>{
    const req=route.request(), u=new URL(req.url());
    const method=req.method();
    const match=u.pathname.match(/\/api\/v1\/agents\/([^/]+)\/(messages|message-history)$/);
    let record;
    if(u.pathname.startsWith('/api/')) { record={method,url:u.pathname+u.search,at:Date.now()}; traffic.push(record); }
    if(!['GET','HEAD','OPTIONS'].includes(method)) {
      if(record) record.result='blocked in browser';
      return route.abort();
    }
    if(overlay&&match&&fixture[match[1]]) {
      const spec=fixture[match[1]];
      const messages=spec.messages.map(m=>m.id==='4'?{...m,attachments:[attachment]}:m);
      const data=match[2]==='message-history'
        ? {generation:u.searchParams.get('generation'), messages, next_before:null, live_after:messages.at(-1).id}
        : {...spec,generation:u.searchParams.get('generation'),messages:u.searchParams.has('after')?[]:messages,has_more:false};
      record.result='local density overlay';
      return route.fulfill({json:data});
    }
    if(overlay&&u.pathname===attachment.url) {
      record.result='local attachment bytes';
      return route.fulfill({contentType:attachment.media_type,body:attachmentBytes});
    }
    return route.continue();
  });
  p.on('response',r=>{
    const u=new URL(r.url()),match=u.pathname.match(/\/api\/v1\/agents\/([^/]+)\/messages$/);
    if(!overlay&&mode==='baseline'&&match&&!u.searchParams.has('after')) {
      pending.push(r.json().then(data=>{fixture[match[1]]??=data;}).catch(()=>{}));
    }
    const last=traffic.findLast(q=>q.url===u.pathname+u.search&&q.status===undefined);
    if(last) last.status=r.status();
  });
  const open = async name => {
    await p.getByRole('button',{name:new RegExp(`^@${name} · `)}).first().click({timeout:30000});
    await history(p,name).waitFor({timeout:12000});
    await history(p,name).getByText(new RegExp(`\\[${name} / `)).first().waitFor();
    await p.evaluate(()=>document.fonts.ready);
  };
  const shot = part => p.screenshot({path:path.join(out,`${overlay?'fixture':'live'}-${name}-${part}.png`),animations:'disabled',timeout:8500}).catch(e=>{(entry.screenshotErrors??={})[part]=String(e).slice(0,800);});
  const align = async (agent,id) => {
    await message(p,agent,id).evaluate(a=>{
      const h=a.closest('[aria-label="会话历史"]');
      h.scrollTop += a.getBoundingClientRect().top-h.getBoundingClientRect().top-1;
    });
    await p.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
  };
  try {
    await p.goto(target,{waitUntil:'domcontentloaded',timeout:22000});
    await open('luna');
    if(mode==='candidate'&&!overlay) await shot('luna-first');
    if(name==='dual') await open('mora');
    entry.route=p.url();
    entry.initial={luna:await captureCoords(history(p,'luna'))};
    if(name==='dual') entry.initial.mora=await captureCoords(history(p,'mora'));
    await shot('initial');
    if(overlay) {
      await align('luna','4');
      if(name==='dual') await align('mora','4');
      entry.sameAnchor={luna:await captureCoords(history(p,'luna'))};
      if(name==='dual') entry.sameAnchor.mora=await captureCoords(history(p,'mora'));
      await shot('same-anchor');
      await safePhase(entry,'densityControls',async()=>{
        const scope=section(p,name==='dual'?'mora':'luna');
        const labels=await scope.locator('button,summary,textarea,a[href]').evaluateAll(nodes=>nodes.map(e=>{
          const r=e.getBoundingClientRect(), occlusion=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
          return {name:e.getAttribute('aria-label')||e.title||e.textContent.trim()||e.placeholder,
            ok:!e.disabled, hit:!!occlusion&&(e===occlusion||e.contains(occlusion)), width:r.width,height:r.height};
        }).filter(x=>x.width&&x.height&&x.hit));
        return labels;
      });
      await safePhase(entry,'longDraft', async()=>{
        const scope=section(p,'luna'), editor=scope.locator('textarea').first();
        await editor.fill(draft);
        await editor.press('End');
        await editor.press('Enter');
        const newline=(await editor.inputValue()).length===draft.length+1;
        const end=traffic.length;
        await editor.evaluate(e=>{
          e.dispatchEvent(new CompositionEvent('compositionstart',{bubbles:true,data:'确认'}));
          e.dispatchEvent(new KeyboardEvent('keydown',{bubbles:true,key:'Enter',ctrlKey:true,isComposing:true,keyCode:229}));
          e.dispatchEvent(new CompositionEvent('compositionend',{bubbles:true,data:'确认'}));
        });
        const send=button(scope,'发送 AgentX 消息');
        const sendVisible=await send.evaluate(e=>{
          const r=e.getBoundingClientRect();
          return e.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2));
        });
        await shot('long-draft');
        return {newline, sendVisible, sends:traffic.slice(end).filter(x=>x.method==='POST'), coords:await captureCoords(history(p,'luna'))};
      });
      await safePhase(entry,'heightChoices',async()=>{
        const scope=section(p,'luna'), editor=scope.locator('textarea').first();
        const original=await editor.inputValue();
        if(touch) await scope.getByLabel('调整草稿高度',{exact:true}).tap();
        else {
          const handle=scope.getByRole('button',{name:'拖动调整草稿高度',exact:true});
          await handle.focus(); await p.keyboard.press('Enter');
        }
        const compact=scope.getByRole('button',{name:'收紧',exact:true});
        await compact.waitFor();
        const controls=await compact.evaluate(x=>x.parentElement.innerText);
        if(touch) await compact.tap(); else { await compact.focus(); await p.keyboard.press('Enter'); }
        return {controls,draftPreserved:(await editor.inputValue())===original,coords:await captureCoords(history(p,'luna'))};
      });
    } else if(mode==='baseline'&&name==='dual') {
      await safePhase(entry,'continuity',async()=>{
        await align('luna','5');
        await align('mora','7');
        const before=await captureCoords(history(p,'luna'));
        await section(p,'luna').locator('textarea').fill('评估 Luna 独有草稿\n第二行');
        await section(p,'mora').locator('textarea').fill('评估 Mora 独有草稿');
        await button(section(p,'luna'),'最小化会话：@luna').click();
        const min=await history(p,'luna').isVisible();
        await p.getByRole('button',{name:/^(@luna|恢复会话：@luna|恢复 @luna|@luna（已最小化）)$/}).filter({visible:true}).first().click();
        await history(p,'luna').waitFor();
        return {before,min,after:await captureCoords(history(p,'luna')),
          lunaDraft:await section(p,'luna').locator('textarea').inputValue(),
          moraDraft:await section(p,'mora').locator('textarea').inputValue()};
      });
    } else if(mode==='baseline'&&(name==='single'||name==='mobile')) {
      await safePhase(entry,'disclosures',async()=>{
        const scope=section(p,'luna'), trigger=relation(scope);
        const opened=scope.getByText('谁启动了我',{exact:true});
        const result={};
        if(!touch){
          await trigger.hover(); await p.waitForTimeout(450); result.hover=await opened.isVisible();
          await trigger.focus(); await p.waitForTimeout(450); result.focus=await opened.isVisible();
          await trigger.click();
        } else await trigger.tap();
        await opened.waitFor();
        result.rows=(await opened.evaluate(e=>e.closest('details').innerText));
        await shot('relation-open');
        if(touch) await scope.locator('textarea').first().tap();
        else await p.keyboard.press('Escape');
        result.dismissed=!(await opened.isVisible());
        if(await opened.isVisible()) await trigger.click();
        await scope.getByLabel('会话详情',{exact:true}).click();
        result.forkLabels=(await scope.getByText(/Fork|分叉|复刻|复制会话/i).allInnerTexts()).filter(Boolean);
        await shot('details');
        return result;
      });
    }
  } catch(e) { entry.error=String(e).slice(0,2200); await shot('error').catch(()=>{}); }
  finally { await Promise.all(pending); await context.close(); }
  return entry;
}

(async()=>{
  await fs.mkdir(out,{recursive:true});
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM||'/usr/bin/google-chrome',args:['--disable-gpu']});
  try {
    // Baseline live data seeds the identical browser-only density overlay for both builds.
    if (!process.env.DENSITY_ONLY) {
      run.passes.live={};
      for(const dims of viewports) {run.passes.live[dims[0]]=await stage(browser,...dims,false); await fs.writeFile(path.join(out,'matrix.json'),JSON.stringify(run,null,2)+'\n');}
    }
    if(mode==='baseline'&&!fixtureInput) await fs.writeFile(path.join(out,'fixture.json'),JSON.stringify(fixture,null,2)+'\n');
    if(!process.env.LIVE_ONLY&&fixture.luna&&fixture.mora) {
      run.passes.fixture={};
      for(const dims of viewports) {run.passes.fixture[dims[0]]=await stage(browser,...dims,true); await fs.writeFile(path.join(out,'matrix.json'),JSON.stringify(run,null,2)+'\n');}
    }
  } finally { await browser.close(); await fs.writeFile(path.join(out,'matrix.json'),JSON.stringify(run,null,2)+'\n'); }
})().catch(e=>{console.error(e);process.exitCode=1;});
