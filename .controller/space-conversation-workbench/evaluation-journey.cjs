// Run on the Controller-designated isolated candidate. Baseline writes are never allowed.
// FORK_ONCE=1 permits one exact Luna Fork; REAL_ARRIVAL=1 permits one isolated fixture
// arrival in its dedicated case. Other arrivals and draft uploads are browser-local.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const fs=require('node:fs/promises');
const path=require('node:path');
const {randomUUID}=require('node:crypto');
const [target,out]=process.argv.slice(2);
if(!target||!out||new URL(target).port==='18983') throw Error('Give the distinct, Controller-designated isolated candidate URL and output path.');
const expectedGeneration='space-workspace-fixture';
const refs={luna:'101',mora:'102',sira:'103',tobi:'104'};
const endpoint=new URL(target);
const targetOrigin=endpoint.origin;
const authorizedForkOrigin=endpoint.hostname==='127.0.0.1'&&['19043','19044','19045','19046'].includes(endpoint.port);
const report={target, generatedAt:new Date().toISOString(), runs:{}};
const pause=(p,ms)=>p.waitForTimeout(ms);
const history=(p,name)=>p.getByLabel('会话历史',{exact:true}).filter({hasText:new RegExp(`\\[${name} / `)});
const panel=(p,name)=>history(p,name).locator('xpath=ancestor::section[@aria-label="当前会话"][1]');
const named=(p,s)=>p.getByRole('button',{name:s,exact:typeof s==='string'});
const summary=(p,text)=>p.locator('summary').filter({hasText:text});
const relation=(p)=>p.locator('button[aria-label="启动关系"],summary:has-text("启动关系"),button:has-text("启动关系")').filter({visible:true}).first();
const visible=(p,text)=>p.getByText(text,{exact:false}).filter({visible:true}).first();
const relationLayer=(marker)=>marker.locator('xpath=ancestor::*[.//*[normalize-space(text())="我启动的 Agent"]][1]');
const lunaChip=(p)=>named(p,/^(@luna|@luna（已最小化）|恢复会话：@luna)$/).filter({visible:true}).first();
const exactRoute=(p)=>Object.fromEntries(new URL(p.url()).searchParams);
const localFile={name:'原稿-最后一段.txt',mimeType:'text/plain',buffer:Buffer.from('仅用于独立浏览器评估，不发送。')};
const lunaDraft='Luna 专属草稿：保留阅读位置。\n中文续写：比较代码与附件。';
const moraDraft='Mora 专属草稿：不要带入另一窗。';

async function open(p,name) {
  if(exactRoute(p).agent!==name) await named(p,new RegExp(`^@${name} · `)).first().click({timeout:30000});
  await history(p,name).waitFor({timeout:14000});
  await p.evaluate(()=>document.fonts.ready);
}

async function reading(p,name) {
  return history(p,name).evaluate(h=>{
    const rect=h.getBoundingClientRect();
    const rows=[...h.querySelectorAll('article')].map(a=>{
      const x=a.getBoundingClientRect();
      return {id:Number(a.textContent.match(/\[[a-z]+ \/ (\d+)\]/)?.[1])||null,
        offset:Math.round((x.top-rect.top)*10)/10,visible:x.top<rect.bottom&&x.bottom>rect.top,
        complete:x.top>=rect.top-.5&&x.bottom<=rect.bottom+.5};
    }).filter(x=>x.id!==null);
    return {scrollTop:h.scrollTop,height:h.clientHeight,scrollHeight:h.scrollHeight,
      ids:rows.map(x=>x.id), first:rows.find(x=>x.visible), firstFull:rows.find(x=>x.complete), last:rows.findLast(x=>x.visible)};
  });
}

const screenshot=(p,key)=>p.screenshot({path:path.join(out,`journey-${key}.png`),animations:'disabled',timeout:7500})
  .catch(err=>{(report.screenshotErrors??={})[key]=String(err).slice(0,600);});

async function attempt(entry,key,fn) {
  try { entry[key]=await fn(); } catch(e) { entry[key]={error:String(e).slice(0,2000)}; }
  return entry[key];
}

async function stage(b,key,{touch=false,fault=false,fork=false,shortPage=false,direct,loseForkReceipt=false}={},fn) {
  if(process.env.EVAL_CASES&&!process.env.EVAL_CASES.split(',').includes(key)) return;
  const c=await b.newContext({viewport:touch?{width:375,height:812}:{width:1440,height:900},
    isMobile:touch,hasTouch:touch,locale:'zh-CN',timezoneId:'America/New_York'});
  const p=await c.newPage();
  p.setDefaultTimeout(6000);
  const state={ inject:false, failed:false, forkSent:false, newest:null, newestRecord:null };
  const traffic=[], e={traffic};
  report.runs[key]=e;
  const sample=async (route,q)=>{
    const res=await route.fetch();
    q.status=res.status();
    if(!res.ok()) return route.fulfill({response:res});
    const raw=await res.json();
    if(Array.isArray(raw.messages)){
      q.ids=raw.messages.map(m=>m.id);
      q.nextBefore=raw.next_before;
      q.liveAfter=raw.live_after;
      q.nextAfter=raw.next_after;
      if(raw.live_after!=null && !new URL(route.request().url()).searchParams.has('before')){
        state.newest=raw.live_after;
        state.newestRecord=raw.messages.at(-1);
      }
    }
    return route.fulfill({response:res});
  };
  await p.route('**/*',async r=>{
    const u=new URL(r.request().url()),method=r.request().method();
    if(!u.pathname.startsWith('/api/')) return ['GET','OPTIONS','HEAD'].includes(method)?r.continue():r.abort();
    const q={at:Date.now(),method,path:u.pathname,query:Object.fromEntries(u.searchParams)};
    traffic.push(q);
    const luna= u.origin===targetOrigin&&u.pathname.startsWith('/api/v1/agents/luna/');
    if(method==='POST'&&u.origin===targetOrigin&&u.pathname==='/api/v1/attachments'){
      q.local='attachment'; q.status=201;
      return r.fulfill({status:201,json:{id:'journey-draft-attachment',name:localFile.name,media_type:localFile.mimeType,
        size:localFile.buffer.length,url:'/api/v1/attachments/journey-draft-attachment'}});
    }
    const forkTarget = method==='POST'&&luna&&u.pathname.endsWith('/fork') ? r.request().postDataJSON() : null;
    if(forkTarget) q.request={agent:forkTarget.agent,generation:forkTarget.generation,request_id:forkTarget.request_id};
    if(forkTarget&&fork&&authorizedForkOrigin&&process.env.FORK_ONCE==='1'&&!state.forkSent
        &&forkTarget.agent?.name==='luna'&&String(forkTarget.agent.incarnation)==='101'&&forkTarget.generation===expectedGeneration){
      state.forkSent=true;
      q.liveFork=true;
      const res=await r.fetch({maxRedirects:0,maxRetries:0});
      q.status=res.status(); q.receipt=await res.json().catch(()=>null);
      if(loseForkReceipt){
        q.local='single real Fork submitted; response lost in browser';
        return r.abort('connectionreset');
      }
      return r.fulfill({response:res});
    }
    if(!['GET','HEAD','OPTIONS'].includes(method)){q.blocked=true;return r.abort();}
    if(fault&&luna&&u.pathname.endsWith('/message-history')&&u.searchParams.has('before')&&!state.failed){
      state.failed=true; q.local='first older request failed'; return r.abort('failed');
    }
    if(luna&&state.inject&&state.newestRecord&&u.pathname.endsWith('/messages')&&u.searchParams.has('after')){
      const id=String(BigInt(state.newest)+1n);
      const after=u.searchParams.get('after');
      q.local='one injected arrival';
      q.ids=after===id?[]:[id];
      return r.fulfill({json:{generation:u.searchParams.get('generation'),next_after:id,has_more:false,
        messages:after===id?[]:[{...state.newestRecord,id,attachments:[],text:`[luna / ${id}] 浏览器注入的新消息，旧阅读位置应保持不动。` }]}});
    }
    if(shortPage&&u.pathname.endsWith('/agents/tobi/message-history')&&!u.searchParams.has('before')){
      const res=await r.fetch();const body=await res.json();
      if(!res.ok()||body.messages.length<2) return r.fulfill({response:res});
      const last=body.messages.at(-1);
      q.local='one short record with true earlier records';q.ids=[last.id];
      return r.fulfill({json:{...body,messages:[{...last,text:`[tobi / ${last.id}] 短消息。`,attachments:[]}],next_before:last.id}});
    }
    if((luna || u.pathname.includes('/agents/tobi/'))&&/\/(messages|message-history)$/.test(u.pathname)) return sample(r,q);
    return r.continue();
  });
  p.on('response',r=>{
    const u=new URL(r.url());
    const q=traffic.findLast(x=>x.path===u.pathname&&x.status===undefined&&!x.local&&!x.blocked&&
      Object.keys(x.query).length===u.searchParams.size&&Object.entries(x.query).every(([k,v])=>u.searchParams.get(k)===v));
    if(q) q.status=r.status();
  });
  p.on('pageerror',err=>(e.pageErrors??=[]).push(String(err)));
  try {
    const url=new URL(target);
    if(direct){url.search=new URLSearchParams({office:'',scope:'/fixture/space-workspace',generation:expectedGeneration,agent:direct,incarnation:refs[direct],record:'current'});e.entry='exact URL';}
    await p.goto(url.href,{waitUntil:'domcontentloaded',timeout:30000}); await fn(p,e,state);
  }
  catch(err){ e.error=String(err).slice(0,2000); await screenshot(p,`${key}-error`).catch(()=>{});}
  finally { await c.close(); await fs.writeFile(path.join(out,'journey.json'),JSON.stringify(report,null,2)+'\n'); }
}

async function appendDraft(p,name,text){
  const box=panel(p,name).locator('textarea').first();
  await box.fill(text);
  return box;
}

async function scroll(p,name,where){
  const h=history(p,name);
  await h.evaluate((el,w)=>{el.scrollTop=w==='edge'?0:w==='bottom'?el.scrollHeight:Math.floor(el.scrollHeight*.40);},where);
  await p.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
}

(async()=>{
  await fs.mkdir(out,{recursive:true});
  const b=await chromium.launch({executablePath:process.env.CHROMIUM||'/usr/bin/google-chrome',args:['--disable-gpu']});
  try {
    await stage(b,'older-retry-arrival',{fault:true,direct:'luna'},async(p,e,s)=>{
      await open(p,'luna');
      e.first=await reading(p,'luna');
      await screenshot(p,'first');
      await scroll(p,'luna','edge');
      const retry=named(panel(p,'luna'),/重试/).first();
      await retry.waitFor();
      e.failed=await reading(p,'luna');
      await screenshot(p,'older-failure');
      const anchored=e.failed.first?.id;
      e.retryBefore=await retry.evaluate((button,id)=>{
        const h=button.closest('[aria-label="会话历史"]') || button.closest('section[aria-label="当前会话"]').querySelector('[aria-label="会话历史"]');
        const data=()=>{
          const hr=h.getBoundingClientRect(), br=button.getBoundingClientRect();
          const anchor=[...h.querySelectorAll('article')].find(a=>a.textContent.includes(`[luna / ${id}]`));
          return {scrollTop:h.scrollTop, retryTop:br.top-hr.top, retryBottom:br.bottom-hr.top,
            anchorOffset:anchor?anchor.getBoundingClientRect().top-hr.top:null,
            centerHit:button.contains(document.elementFromPoint(br.x+br.width/2,br.y+br.height/2))};
        };
        button.addEventListener('click',()=>{window.__evaluationAtRetryClick=data();},{once:true,capture:true});
        return data();
      },anchored);
      await retry.click();
      e.atRetryClick=await p.evaluate(()=>window.__evaluationAtRetryClick);
      await screenshot(p,'older-after-retry-click');
      await p.waitForFunction(old=>[...document.querySelectorAll('article')].some(a=>Number(a.textContent.match(/\[luna \/ (\d+)\]/)?.[1])<old),anchored);
      e.prepended=await reading(p,'luna');
      const cursor=e.traffic.filter(q=>q.path.endsWith('/message-history')&&q.query.before);
      e.olderRequests=cursor.map(x=>({before:x.query.before,result:x.local||x.status,ids:x.ids}));
      if(process.env.RETRY_ONLY==='1') return;
      if(process.env.REAL_ARRIVAL==='1'){
        if(new URL(target).hostname!=='127.0.0.1'||!['19044','19045'].includes(new URL(target).port)) throw Error('Real arrival scoped to the authorized isolated fixture.');
        const id=String(BigInt(s.newest)+1n);
        const request_id=randomUUID();
        const resp=await p.request.post(new URL('/api/v1/agents/luna/messages',target).href,{
          data:{agent:{name:'luna',incarnation:'101',stopped_event:null},generation:expectedGeneration,request_id,
            message:{text:`[luna / ${id}] 独立验收：读旧记录时收到的合成消息。`}},
          maxRedirects:0,maxRetries:0,timeout:15000});
        e.arrival={request_id,status:resp.status(),receipt:await resp.json().catch(()=>null)};
        if(!resp.ok()) throw Error('Isolated arrival rejected by fixture.');
      } else s.inject=true;
      const cue=visible(panel(p,'luna'),/条新消息|回到最新/);
      await cue.waitFor({timeout:17000});
      e.cue=await cue.innerText();
      e.cueTarget=await cue.evaluate(el=>{
        const r=el.getBoundingClientRect(), point=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
        return {x:r.x,y:r.y,width:r.width,height:r.height,centerHit:el.contains(point),hitLabel:point?.getAttribute('aria-label')||point?.textContent?.trim().slice(0,120)};
      });
      e.afterArrival=await reading(p,'luna');
      await screenshot(p,'away-from-newest');
      await cue.click();
      e.back=await reading(p,'luna');
      await screenshot(p,'back-to-newest');
      await attempt(e,'ordinaryPrepend',async()=>{
        const h=history(p,'luna'), first=e.back.ids[0];
        await h.evaluate(el=>{el.scrollTop=120;});
        await p.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
        const before=await reading(p,'luna');
        await h.evaluate(el=>{el.scrollTop=0;});
        await p.waitForFunction(old=>[...document.querySelectorAll('article')].some(a=>Number(a.textContent.match(/\[luna \/ (\d+)\]/)?.[1])<old),first);
        const after=await reading(p,'luna');
        return {before,after};
      });
    });
    await stage(b,'short-first-page',{shortPage:true,direct:'tobi',touch:process.env.SHORT_MOBILE==='1'},async(p,e)=>{
      await open(p,'tobi');
      e.first=await reading(p,'tobi');
      e.viewport=await p.evaluate(()=>[innerWidth,innerHeight,navigator.maxTouchPoints]);
      await screenshot(p,'short-first-page-before');
      const older=named(panel(p,'tobi'),/查看更早消息/).first();
      e.older=await older.isVisible();
      if(e.older){
        if(process.env.SHORT_MOBILE==='1') await older.tap(); else await older.click();
        await visible(panel(p,'tobi'),/\[tobi \/ 1\]/).waitFor();
        await p.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
        e.after=await reading(p,'tobi');
        if(process.env.SHORT_MOBILE==='1'){
          await pause(p,600);
          e.settled=await reading(p,'tobi');
        }
      }
      await screenshot(p,'short-first-page');
    });
    await stage(b,'desktop-continuity',{},async(p,e)=>{
      await open(p,'luna');
      await open(p,'mora');
      await scroll(p,'luna','middle');
      await scroll(p,'mora','middle');
      e.before={luna:await reading(p,'luna'),mora:await reading(p,'mora')};
      await appendDraft(p,'luna',lunaDraft);
      await appendDraft(p,'mora',moraDraft);
      await attempt(e,'attachment',async()=>{
        await panel(p,'luna').locator('input[type=file]').first().setInputFiles(localFile);
        const summary=visible(panel(p,'luna'),/^附件\s*1$/);
        await summary.waitFor();
        const hasDetails=await summary.evaluate(e=>!!e.closest('summary'));
        if(hasDetails) await summary.click();
        const file=await panel(p,'luna').locator(`[title="${localFile.name}"], [aria-label*="${localFile.name}"]`).count();
        const fileText=await visible(panel(p,'luna'),localFile.name).isVisible();
        if(hasDetails) await summary.click();
        return {count:await summary.innerText(),file:!!file||fileText};
      });
      e.beforeHide={luna:await reading(p,'luna'),mora:await reading(p,'mora')};
      await named(panel(p,'luna'),'最小化会话：@luna').click();
      e.hidden=!(await history(p,'luna').isVisible());
      const start=Date.now();
      await pause(p,12000);
      e.hiddenReads=e.traffic.filter(q=>q.at>=start&&/\/agents\/(luna|mora)\/(messages|message-history)$/.test(q.path));
      await lunaChip(p).click();
      await history(p,'luna').waitFor();
      e.after={luna:await reading(p,'luna'),mora:await reading(p,'mora')};
      e.restored={route:exactRoute(p),luna:await panel(p,'luna').locator('textarea').inputValue(),
        mora:await panel(p,'mora').locator('textarea').inputValue(),attachment:await visible(panel(p,'luna'),/^附件\s*1$/).isVisible()};
      await attempt(e,'transcript',async()=>{
        const before=e.traffic.length;
        await named(panel(p,'luna'),'Transcript').click();
        const transcript=p.getByLabel('会话历史',{exact:true}).filter({visible:true});
        const text=(await transcript.allTextContents()).map(s=>s.slice(0,500));
        await named(p,'消息').filter({visible:true}).first().click();
        return {text,requests:e.traffic.slice(before),recovered:await reading(p,'luna')};
      });
      await screenshot(p,'desktop-continuity');
    });
    await stage(b,'relation-and-fork',{fork:true},async(p,e)=>{
      await open(p,'luna');
      await scroll(p,'luna','middle');
      e.before=await reading(p,'luna');
      await appendDraft(p,'luna',lunaDraft);
      const trigger=relation(panel(p,'luna'));
      const openMarker=visible(p,'谁启动了我');
      await attempt(e,'keyboardRelation',async()=>{
        const search=named(panel(p,'luna'),'查找已读入记录');
        await search.focus();
        await p.keyboard.press('Tab');
        e.focusBeforePopup=await p.evaluate(()=>[document.activeElement?.getAttribute('aria-label'),document.activeElement?.matches(':focus-visible')]);
        await openMarker.waitFor();
        const isFocused=await trigger.evaluate(x=>x===document.activeElement);
        await p.keyboard.press('Escape');
        await pause(p,450);
        return {isFocused, dismissed:!(await openMarker.isVisible()),focus:await p.evaluate(()=>document.activeElement?.textContent.trim().slice(0,120))};
      });
      await attempt(e,'hoverRelation',async()=>{
        await trigger.hover();await openMarker.waitFor();
        const layer=relationLayer(openMarker);
        const more=named(layer,/查看更多|加载更多|更多.*关系/).filter({visible:true}).first();
        const initial=await layer.innerText();
        await screenshot(p,'relation-hover');
        const exact=visible(layer,/^@?mora(?:\s|$)/i);
        await exact.hover();
        const still=await openMarker.isVisible();
        await exact.click();
        await history(p,'mora').waitFor();
        const destination=exactRoute(p);
        await lunaChip(p).click();
        const source=exactRoute(p), draft=await panel(p,'luna').locator('textarea').inputValue(), readingPosition=await reading(p,'luna');
        await trigger.hover(); await openMarker.waitFor();
        let disclosed=null;
        if(await more.isVisible()){
          await more.click();
          await pause(p,800);
          if(await openMarker.isVisible()) {
            disclosed={text:await layer.innerText(),controls:await layer.locator('button,a[href]').evaluateAll(xs=>xs.map(x=>[x.innerText,x.getAttribute('aria-label'),x.disabled]))};
            await screenshot(p,'relation-more');
          } else disclosed={closed:true};
        }
        return {initial,disclosed,still,destination,source,draft,reading:readingPosition};
      });
      await attempt(e,'fork',async()=>{
        const action=named(panel(p,'luna'),'Fork 新 Agent').filter({visible:true}).first();
        await action.click();
        const dialog=p.getByRole('dialog').filter({visible:true}).last();
        const confirmation=(await dialog.innerText()).slice(0,1600);
        await screenshot(p,'fork-confirmation');
        if(process.env.FORK_ONCE!=='1') return {confirmation,submitted:false};
        await named(dialog,/确认|Fork|创建/).filter({visible:true}).last().click();
        const result=p.getByText('打开分叉会话',{exact:true}).filter({visible:true}).first();
        await attempt(e,'forkResultWait',async()=>{await result.waitFor({timeout:15000});return true;});
        await screenshot(p,'fork-receipt');
        if(e.forkResultWait!==true) return {confirmation,receiptVisible:false,body:(await p.locator('body').innerText()).split('\n').filter(s=>/fork|分叉|回执/i.test(s)).slice(-18)};
        const beforeOpen=exactRoute(p);
        await result.click();
        await p.waitForFunction(()=>new URL(location.href).searchParams.get('agent')!=='luna');
        const destination=exactRoute(p);
        await screenshot(p,'fork-result');
        await lunaChip(p).click();
        const source=exactRoute(p);
        return {confirmation,beforeOpen,destination,source,draft:await panel(p,'luna').locator('textarea').inputValue(),reading:await reading(p,'luna')};
      });
    });
    if(process.env.EVAL_CASES?.split(',').includes('relation-escape')){
      await stage(b,'relation-escape',{direct:'luna'},async(p,e)=>{
        await open(p,'luna');
        const content=panel(p,'luna'), trigger=relation(content), marker=visible(p,'谁启动了我');
        const focus=()=>p.evaluate(()=>[document.activeElement?.getAttribute('aria-label'),document.activeElement?.textContent.trim().slice(0,100)]);
        const entry=await reading(p,'luna');
        await trigger.hover();
        await marker.waitFor();
        await p.keyboard.press('Escape');
        await pause(p,600);
        e.pointerEscape={closed:!await marker.isVisible(), focus:await focus()};
        await p.mouse.move(5,5);
        const search=named(content,'查找已读入记录');
        await search.focus();
        await p.keyboard.press('Tab');
        await marker.waitFor();
        e.tabFocus=await focus();
        const row=visible(relationLayer(marker),/^@?mora(?:\s|$)/i);
        await row.locator('xpath=ancestor::button[1]').focus();
        e.rowFocus=await focus();
        const start=e.traffic.length;
        await p.keyboard.press('Escape');
        await pause(p,650);
        e.keyboardEscape={closed:!await marker.isVisible(),focus:await focus(),firstBefore:entry.first,firstAfter:(await reading(p,'luna')).first};
        await pause(p,6400);
        e.closedTraffic=e.traffic.slice(start).filter(q=>/launch-relations|messages/.test(q.path));
        await screenshot(p,'relation-escape');
      });
    }
    if(process.env.EVAL_CASES?.split(',').includes('uncertain-fork')){
      if(new URL(target).hostname!=='127.0.0.1'||!['19045','19046'].includes(new URL(target).port)||process.env.FORK_ONCE!=='1') throw Error('Uncertain Fork scoped to the explicitly authorized disposable fixture.');
      await stage(b,'uncertain-fork',{fork:true,loseForkReceipt:true,direct:'luna'},async(p,e)=>{
        await open(p,'luna');
        await scroll(p,'luna','middle');
        await appendDraft(p,'luna',lunaDraft);
        e.before=await reading(p,'luna');
        await named(panel(p,'luna'),'Fork 新 Agent').filter({visible:true}).first().click();
        const dialog=p.getByRole('dialog').filter({visible:true}).last();
        await named(dialog,/确认|Fork|创建/).filter({visible:true}).last().click();
        const result=p.getByText('打开分叉会话',{exact:true}).filter({visible:true}).first();
        const query=named(dialog,'查询原回执');
        await query.waitFor({timeout:10000});
        const confirm=named(dialog,/确认.*Fork/);
        e.confirmDisabled=await confirm.isDisabled();
        e.uncertainText=(await p.locator('body').innerText()).split('\n').filter(s=>/fork|分叉|回执|原请求/i.test(s)).slice(-25);
        await screenshot(p,'original-fork-receipt-unknown');
        if(!await result.isVisible()&&await query.isVisible()){
          e.manualQueryAttempted=true;
          try { await query.click({timeout:4000}); }
          catch(e) { if(!await result.isVisible()) throw e; }
        }
        await result.waitFor({timeout:12000}).catch(()=>{});
        e.recovered=await result.isVisible();
        e.beforeOpen=exactRoute(p);
        await screenshot(p,'uncertain-fork');
        if(e.recovered){
          await result.click();
          await p.waitForFunction(()=>new URL(location.href).searchParams.get('agent')!=='luna');
          e.destination=exactRoute(p);
          await lunaChip(p).click();
          e.source=exactRoute(p);
          e.restored={reading:await reading(p,'luna'),draft:await panel(p,'luna').locator('textarea').inputValue()};
        }
        e.forkPosts=e.traffic.filter(q=>q.method==='POST'&&q.path.endsWith('/fork')).length;
        e.receiptQueries=e.traffic.filter(q=>q.path.startsWith('/api/v1/operations/'));
      });
    }
    await stage(b,'unsupported-fork',{direct:'sira'},async(p,e)=>{
      await open(p,'sira');
      e.header=named(panel(p,'sira'),'Fork 新 Agent').filter({visible:true}).first();
      const shown=await e.header.isVisible();
      delete e.header;
      e.headerShown=shown;
      const details=panel(p,'sira').locator('summary[aria-label="会话详情"],button[aria-label="会话详情"],button[aria-label="会话操作"]').filter({visible:true}).first();
      await details.click();
      e.forkText=(await panel(p,'sira').innerText()).split('\n').filter(s=>/fork|分叉/i.test(s));
      await screenshot(p,'unsupported-fork');
    });
    await stage(b,'mobile-touch',{touch:true,direct:'luna'},async(p,e)=>{
      await open(p,'luna');
      await scroll(p,'luna','middle');
      e.before=await reading(p,'luna');
      const editor=await appendDraft(p,'luna',lunaDraft);
      await editor.blur();
      const trigger=relation(panel(p,'luna'));
      await trigger.tap();
      const openMarker=visible(p,'谁启动了我');
      await openMarker.waitFor();
      await screenshot(p,'mobile-relation');
      await panel(p,'luna').locator('textarea').tap();
      e.outsideDismissed=!(await openMarker.isVisible());
      if(!e.outsideDismissed) await trigger.tap();
      await trigger.tap();
      await openMarker.waitFor();
      await visible(relationLayer(openMarker),/^@?mora(?:\s|$)/i).tap();
      await history(p,'mora').waitFor();
      e.mora=exactRoute(p);
      await lunaChip(p).tap();
      await history(p,'luna').waitFor();
      e.restored={route:exactRoute(p),reading:await reading(p,'luna'),draft:await panel(p,'luna').locator('textarea').inputValue()};
      await screenshot(p,'mobile-return');
    });
  } finally {await b.close();await fs.writeFile(path.join(out,'journey.json'),JSON.stringify(report,null,2)+'\n');}
})().catch(err=>{console.error(err);process.exitCode=1;});
