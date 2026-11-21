import {openDialog, closeDialog} from '../system/disclosure.js';
import {agents, offices, statuses, posts, recent, older, records, mail, SCOPE, GENERATION, FIXTURE} from './fixtures.js';
import {defaults, readLocation, locationFor} from './navigation.js';
import {setupScene} from './scene.js';

const $ = id => document.getElementById(id);
const byKey = key => agents.find(a=>a.key===key);
const officeOf = id => offices.find(o=>o.id===id);
const officeLabel = id => officeOf(id)?.short || '全部办公室';
const escapeHTML = (value='') => String(value).replace(/[&<>"']/g, c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const E = escapeHTML;
const sharedIcons = new Set(['search','info','close','back','arrow-right','list','grid','send','check','warning','panel']);
const localIcons = {
  space:'M4 5h16v14H4zm0 7h16m-9-7v7m4 0v7',
  folder:'M3 7h7l2 2h9v10H3zm0 0V5h7l2 2h7v2',
  filter:'M4 6h16M7 12h10m-7 6h4',
  map:'m3 6 6-3 6 3 6-3v15l-6 3-6-3-6 3zm6-3v15m6-12v15',
  mail:'M3 5h18v14H3zm0 1 9 7 9-7',
  flask:'M10 3h4m-3 0v7l-5 8q-1 3 2 3h8q3 0 2-3l-5-8V3M8 15h8',
  plus:'M12 5v14M5 12h14', minus:'M5 12h14',
  focus:'M4 9V4h5m6 0h5v5m0 6v5h-5m-6 0H4v-5m8-5v4m-2-2h4',
  share:'M12 15V3M8 7l4-4 4 4M6 10H4v10h16V10h-2',
  clip:'M8 12l6-6a3 3 0 0 1 4 4l-8 8a4 4 0 0 1-6-6l9-9m-4 12 6-6',
  reply:'M9 5 3 11l6 6m-6-6h9q7 0 8 7',
  more:'M4 12h.1m7.9 0h.1m7.9 0h.1'
};
const renderIcon = name => {
  if (name==='forward') name='arrow-right';
  const body = sharedIcons.has(name) ? `<use href="../system/icons.svg#${name}"/>` : `<path d="${localIcons[name]}" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>`;
  return `<svg aria-hidden="true" viewBox="0 0 24 24" focusable="false">${body}</svg>`;
};
function icon(name) { return `<span data-icon="${name}">${renderIcon(name)}</span>`; }
document.querySelectorAll('[data-icon]').forEach(node=>{node.innerHTML=renderIcon(node.dataset.icon);});
function badge(text,tone='neutral') { return `<span class="ax-badge" data-tone="${tone}">${E(text)}</span>`; }
function stateBadge(a) { return badge(...statuses[a.status]); }
function avatar(agent, self=false) { return `<span class="avatar" aria-hidden="true" data-avatar="${self?'blue':agent.tone}">${self?'你':E(agent.name.slice(0,2).toUpperCase())}</span>`; }
function button(action,text,value='',opts={}) {
  const title=opts.title?` title="${E(opts.title)}" aria-label="${E(opts.title)}"`:'';
  return `<button type="button" class="ax-button${opts.ic?' ax-icon-button':''}"${title} data-action="${action}" data-value="${E(value)}"${opts.primary?' data-variant="primary"':''}${opts.disabled?' disabled':''}>${opts.icon?icon(opts.icon):''}${E(text)}</button>`;
}
const pressed = (action,label,value,current) => `<button type="button" data-action="${action}" data-value="${E(value)}" aria-pressed="${value===current}">${E(label)}</button>`;
function empty(head,text,action='') { return `<div class="empty-state"><strong>${E(head)}</strong><p>${E(text)}</p>${action}</div>`; }

let {next:nav, issue:routeIssue} = readLocation();
let transient = '', dialogType = '', modalTrigger, announcementTimer, serial = 0, readerRenderKey = '';
const returns = new Map();
const dialogs = $('disclosure');
const sizes = new ResizeObserver(()=> {
  if (nav.mode==='scene' && byKey(nav.key)) requestAnimationFrame(()=>scene.bringIntoView(byKey(nav.key)));
});
sizes.observe($('scene'));
const scene = setupScene($('map-viewport'),$('map-plan'),$('map-markers'),$('camera-label'));
const indexScroll = new Map();
const sessions = new Map();
const mailHandled = new Set(mail.filter(m=>m.read).map(m=>m.id));
const expandedMail = new Set();
const channels = {messages:'消息', model:'Provider 记录', notes:'本地笔记'};
const messageSessions = key=>{
  if (!sessions.has(key)) sessions.set(key,{text:'',attach:[],reply:'',intent:'',topic:'',rev:0, operation:null, older:false, added:[], incoming:0, provider:'idle', notes:[], noteDraft:'', read:{}, tool:new Set()});
  return sessions.get(key);
};
const restoreKey = s=>JSON.stringify([s.office,s.tab,s.q,s.status]);
const subject = ()=>byKey(nav.key);
const current = ()=>nav.key?messageSessions(nav.key):null;
const currentDraft = s=>Boolean(s?.text.trim() || s?.attach.length);
const matches = a=>{
  const q=nav.q.trim().toLowerCase();
  return (!nav.office || a.office===nav.office) && (!nav.status || a.stop || a.status===nav.status) &&
    (!q || [a.name, '@'+a.name, a.role, officeLabel(a.office), a.office].some(x=>x.toLowerCase().includes(q)));
};
const exactLabel = a=>`@${a.name} · ${a.stop?'历史实例':'当前实例'} · ${officeLabel(a.office)}`;
function announce(text) {
  const node=$('announcement');
  node.hidden=false;
  node.textContent=text;
  clearTimeout(announcementTimer);
  announcementTimer=setTimeout(()=>{node.hidden=true;},3000);
}
function captureRead() {
  if (!nav.key || $('reader').hidden || nav.channel==='notes') return;
  const thread = $('thread'), top = thread.getBoundingClientRect().top;
  const visible = [...thread.querySelectorAll('[data-record-id]')].find(node=>node.getBoundingClientRect().bottom>top+1);
  current().read[nav.channel] = {
    id:visible?.dataset.recordId, dy:visible?.getBoundingClientRect().top-top, scroll:thread.scrollTop,
    latest:(thread.querySelector('.thread-inner')?.getBoundingClientRect().bottom || 0)-thread.getBoundingClientRect().bottom<12
  };
}
function restoreRead(override) {
  const thread=$('thread'), memory=override || current()?.read[nav.channel];
  if (!memory && nav.channel==='messages') return toLatest();
  if (!memory) {thread.scrollTop=0;return;}
  const tail = thread.querySelector('.thread-anchor-space');
  if (tail) tail.style.blockSize='0px';
  const node=[...thread.querySelectorAll('[data-record-id]')].find(n=>n.dataset.recordId===memory.id);
  const top = node?thread.scrollTop+node.getBoundingClientRect().top-thread.getBoundingClientRect().top-memory.dy:memory.scroll;
  // A large viewport can have too little content below the restored message.
  // Add only the space needed to keep its offset instead of snapping to older rows.
  const missing = Math.ceil(top - Math.max(0,thread.scrollHeight-thread.clientHeight));
  if (tail && node && missing>0) tail.style.blockSize=missing+'px';
  thread.scrollTop=top;
}
function saveDiscovery() {
  if (!$('index').hidden && $('index').offsetWidth) indexScroll.set(restoreKey(nav),$('index').scrollTop);
}
function focusDiscovery(key) {
  const selector=nav.mode==='scene'?'.map-actor:not([hidden]):not([inert]), .directory-agent':'.agent-row';
  const exact=[...document.querySelectorAll(selector)].find(n=>n.dataset.value===key && n.getClientRects().length);
  if (exact) exact.focus({preventScroll:true});
  else if (nav.mode==='scene' && !$('scene').hidden) document.querySelector('[data-action="directory"]').focus({preventScroll:true});
  else $('search').focus({preventScroll:true});
}
function transition(patch, opts={}) {
  captureRead();
  saveDiscovery();
  const old=nav;
  nav={...nav,...patch};
  if (JSON.stringify(old)===JSON.stringify(nav) && !routeIssue && !opts.force) return;
  if (old.key && !nav.key) scene.unreveal();
  routeIssue='';
  if (!opts.pop) {
    const url=locationFor(nav);
    if (url.href!==location.href) history[opts.replace?'replaceState':'pushState'](null,'',url);
  }
  render();
  if (nav.mode==='scene' && nav.key) requestAnimationFrame(()=>scene.bringIntoView(subject()));
  if (opts.reader) $('reader-identity').focus({preventScroll:true});
  if (opts.focus) focusDiscovery(opts.focus);
}
window.addEventListener('popstate',()=>{
  const prev=nav;
  const {next, issue}=readLocation();
  transition(next,{pop:true,force:true});
  routeIssue=issue;
  renderRoute();
  if (prev.key && !nav.key) focusDiscovery(prev.key);
});

function renderRoute() {
  $('route-notice').hidden=!routeIssue;
  $('route-notice').innerHTML=routeIssue
    ? `<span>${E(routeIssue)}</span> ${button('clear-route','返回可用列表')}`:'';
}
function renderLayout() {
  const a=subject();
  const isScene=nav.mode==='scene' && nav.tab==='members';
  $('space-body').dataset.mode=isScene?'scene':'chat';
  $('space-body').dataset.reading=Boolean(a);
  $('reader').hidden=!a; $('welcome').hidden=Boolean(a)||isScene;
  $('index').hidden=isScene; $('scene').hidden=!isScene;
  document.querySelectorAll('.mode-control button').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.value===nav.mode)));
  $('scope').value=nav.office;
  if ($('search').value!==nav.q) $('search').value=nav.q;
  if ($('status').value!==nav.status) $('status').value=nav.status;
  $('status').disabled=nav.tab!=='members';
  const matchesCount=agents.filter(a=>matches(a) && (nav.tab==='history'?a.stop:!a.stop)).length;
  const title=nav.tab==='posts'?'岗位':nav.tab==='skills'?'技能':nav.tab==='history'?'历史实例':nav.mode==='scene'?'办公室':'会话';
  $('collection-label').innerHTML=`<h1>${title}</h1><small>${nav.tab==='members'||nav.tab==='history'?`${matchesCount} ${nav.tab==='history'?'个历史实例':'位成员'}`:'合成样本'}</small>`;
  $('resource-tabs').innerHTML=pressed('tab',nav.mode==='scene'?'楼层':'当前','members',nav.tab)+pressed('tab','历史','history',nav.tab)+pressed('tab','岗位','posts',nav.tab)+pressed('tab','技能','skills',nav.tab)+button('workspace','更多','',{icon:'more',title:'工作区操作'});
  $('mail-count').textContent=mail.length-mailHandled.size;
  $('mail-count').setAttribute('aria-label',`${mail.length-mailHandled.size} 封未处理`);
}
function row(a) {
  const picked=a.key===nav.key, draft=currentDraft(sessions.get(a.key)), unknown=['unknown','checking'].includes(sessions.get(a.key)?.operation?.kind);
  return `<button class="agent-row" type="button" data-action="select" data-value="${E(a.key)}" data-ui-component="ConversationRow" aria-current="${picked}">
    ${avatar(a)}
    <span class="agent-row-main"><span class="agent-row-name"><b>@${E(a.name)}</b> ${stateBadge(a)}</span>
    <span class="agent-row-preview">${E(a.preview)}</span>
    <span class="agent-row-context">${E(officeLabel(a.office))} · ${E(a.role)}${unknown?' · 发送待核对':draft?' · 草稿':''}</span></span>
    <span class="agent-row-side"><time>${E(a.time)}</time>${a.unread?`<span class="unread-count" aria-label="${a.unread} 条样本未读">${a.unread}</span>`:''}</span>
  </button>`;
}
function renderIndex() {
  const index = $('index');
  if(index.hidden) return;
  if(nav.tab==='skills') {index.innerHTML=empty('技能目录未提供','当前样本未提供办公室技能目录。',button('tab','返回发现','members'));return;}
  if(nav.tab==='posts') {
    const list=posts.filter(p=>(!nav.office || p.office===nav.office) && (!nav.q || [p.title,p.path,officeLabel(p.office)].some(x=>x.toLowerCase().includes(nav.q.trim().toLowerCase()))));
    index.innerHTML=list.length?`<div class="resource-list">${list.map(p=>`<button class="ax-button resource-row" data-action="post" data-value="${p.id}"><span><strong>${E(p.title)}</strong> · ${E(officeLabel(p.office))}</span>${icon('forward')}</button>`).join('')}</div>`:empty('没有可显示的岗位','当前办公室或检索条件下没有岗位定义。',button('clear-filters','清除筛选'));
    return;
  }
  const list=agents.filter(a=>matches(a) && (nav.tab==='history'?a.stop:!a.stop));
  if(!list.length) {
    index.innerHTML=nav.q || (nav.tab==='members' && nav.status)?empty('没有匹配的实例','调整检索范围后再试。',button('clear-filters','清除筛选')):
      empty(nav.tab==='history'?'没有历史实例':'此办公室暂无成员','可查看岗位定义，或切换办公室。',button('tab','查看岗位','posts'));
    return;
  }
  const order=nav.office?[nav.office]:offices.map(o=>o.id);
  index.innerHTML=order.map(o=>{
    const group=list.filter(a=>a.office===o);
    return group.length?`<section aria-label="${E(officeLabel(o))}">${nav.office?'':`<h2 class="index-caption">${E(officeLabel(o))} <span>· ${group.length}</span></h2>`}${group.map(row).join('')}</section>`:'';
  }).join('');
  index.scrollTop=indexScroll.get(restoreKey(nav))||0;
}
function renderScene() {
  if ($('scene').hidden) return;
  scene.selectOffice(nav.office);
  const roomTags=offices.filter(x=>x.x!=null).map(o=>{
    const count=agents.filter(a=>!a.stop && a.office===o.id).length;
    return `<button class="room-marker" data-ui-component="RoomMarker" data-action="office" data-value="${E(o.id)}" data-x="${o.x}" data-y="${o.y}" aria-current="${nav.office===o.id}" aria-label="${E(o.title)}，${count} 位成员"><small>${E(o.code)}</small>${E(o.title)}<small>${count}</small></button>`;
  }).join('');
  const markers=agents.filter(a=>a.x!=null && !a.stop && matches(a)).map(a=>
    `<button class="map-actor" data-ui-component="ActorMarker" data-action="select" data-value="${E(a.key)}" data-x="${a.x}" data-y="${a.y}" data-selected="${nav.key===a.key}" aria-current="${nav.key===a.key}" aria-label="${E(exactLabel(a)+' · '+a.role+' · '+statuses[a.status][0])}" title="${E(a.role+' · '+statuses[a.status][0])}">${avatar(a)}@${E(a.name)}<i class="scene-dot" data-status="${a.status}"></i></button>`).join('');
  $('map-markers').innerHTML=roomTags+markers+`<button class="map-mail" data-action="mail" data-x="50" data-y="82" aria-label="我的信箱">${icon('mail')}信箱 · ${mail.length-mailHandled.size}</button>`;
  $('map-directory').hidden=nav.panel!=='directory';
  document.querySelector('[data-action="directory"]').setAttribute('aria-expanded',String(nav.panel==='directory'));
  if(nav.panel==='directory'){
    const groups=nav.office?[officeOf(nav.office)]:offices;
    $('map-directory').innerHTML=`<div class="directory-head"><strong>楼层目录</strong>${button('directory','','',{icon:'close',ic:true,title:'关闭楼层目录'})}</div>`+groups.map(o=>{
      const people=agents.filter(a=>!a.stop && a.office===o.id && matches(a));
      return `<div class="directory-group">${button('office',`${o.title} · ${people.length}`,o.id)}${people.map(a=>`<button class="directory-agent" data-action="select" data-value="${E(a.key)}" aria-current="${a.key===nav.key}"><span>@${E(a.name)} · ${E(a.role)}</span>${stateBadge(a)}</button>`).join('') || `<p class="index-caption">${nav.q || nav.status?'没有匹配的成员':'暂无成员'}</p>`}</div>`;
    }).join('')+button('tab','浏览历史实例','history');
  }
  scene.project();
}

function envelopes(a,s) {
  if(a.key===agents[0].key) return [...(s.older?older:[]),...recent,...s.added];
  const seed=a.name==='kiri'||a.name==='taro'?[]:[{id:'seed-'+a.inc,who:a.name,time:a.time,text:a.preview}];
  return [...seed,...s.added];
}
function envelopeBlock(msg,a) {
  const self=msg.who==='你';
  const reply=msg.reply?`<span class="envelope-reply">引用 ${E(msg.reply)}${msg.thread?' · '+E(msg.thread):''}</span>`:'';
  const names=msg.files || (msg.file?[msg.file]:[]);
  const file=names.map(n=>button('file',n,n,{icon:'clip'})).join('');
  return `<article class="envelope" data-record-id="${E(msg.id)}" data-ui-component="MessageEnvelope" data-self="${self}">
    ${avatar(a,self)}<div class="envelope-content"><header class="envelope-meta"><b>${self?'你':'@'+E(msg.who)}</b><time>${E(msg.time)}</time></header>
    <div class="envelope-bubble">${reply}${E(msg.text)}${msg.intent?`<div class="envelope-reply">意图 · ${E(msg.intent)}</div>`:''}</div>
    ${file? `<div class="envelope-file">${file}</div>`:''}
    ${msg.thread&&!msg.reply?`<small class="muted">主题 · ${E(msg.thread)}</small>`:''}</div>
  </article>`;
}
function messageThread(a,s) {
  const messages=envelopes(a,s);
  if(!messages.length) return empty('暂无 AgentX 消息',a.stop?'这个历史实例没有可用消息。':'选择下方输入框写一条新消息。');
  const hasOlder=a.key===agents[0].key;
  const head=hasOlder?(s.older?'<div class="thread-divider">样本记录起点</div>':`<div class="history-loader">${button('older','加载更早的消息')}</div>`):'';
  const olderLink=hasOlder&&s.older?`<div class="history-loader">${button('history-start','查看已加载的旧消息')}</div>`:'';
  return head+messages.map((m,i)=>`${i===(hasOlder&&s.older?older.length:0)?olderLink+'<div class="thread-divider">'+(a.stop?'历史 · 11 月':'今天 · 11 月 18 日')+'</div>':''}${envelopeBlock(m,a)}`).join('');
}
function providerThread(a,s) {
  if(a.status==='unknown') return empty('Provider 记录未提供','这个实例没有可获取的 Provider 记录。');
  if(s.provider==='idle') return empty('尚未获取 Provider 记录','Provider 会话与 AgentX 消息分开读取。',button('load-provider','加载 Provider 记录','',{primary:true}));
  if(s.provider==='loading') return empty('正在读取 Provider 记录…','');
  if(s.provider==='error') return empty('Provider 读取失败','样本读取超时。可以重新发起读取。',button('load-provider','重新读取'));
  if(s.provider==='empty') return empty('没有 Provider 记录','读取已完成，当前实例没有可显示的样本。',button('load-provider','重新读取'));
  return `<div class="thread-divider">Provider · 合成会话 ${E(a.inc)}</div>`+records.map(r=>`<article class="ax-panel record-card" data-record-id="${r.id}" data-ui-component="ProviderRecord"><header><b>${E(r.role)}</b><time class="muted">${E(r.time)}</time></header><p>${E(r.text)}</p>${r.tool?`<details data-tool="${r.id}-tool"${s.tool.has(r.id+'-tool')?' open':''}><summary>工具调用</summary><pre>${E(r.tool)}</pre></details><details data-tool="${r.id}-reason"${s.tool.has(r.id+'-reason')?' open':''}><summary>推理摘录</summary><p>${E(r.reason)}</p></details>`:''}</article>`).join('');
}
function noteThread(s) {
  return `<div class="thread-divider">当前页面中的笔记 · 不会发给 Agent</div>${s.notes.map(n=>`<article class="ax-panel record-card"><p>${E(n)}</p></article>`).join('')}
    <form id="note-form" class="ax-stack"><label for="note">本地笔记</label><textarea id="note" class="ax-input" rows="3" maxlength="3000">${E(s.noteDraft)}</textarea><button class="ax-button" type="submit">添加本地笔记</button></form>`;
}
function renderThread(override) {
  const a=subject(),s=current();
  if(!a)return;
  const stream=nav.channel==='messages'?messageThread(a,s):nav.channel==='model'?providerThread(a,s):noteThread(s);
  $('thread').innerHTML=`<div class="thread-inner">${stream}</div><div class="thread-anchor-space" aria-hidden="true"></div>`;
  $('thread').querySelectorAll('[data-tool]').forEach(n=>n.addEventListener('toggle',()=>n.open?s.tool.add(n.dataset.tool):s.tool.delete(n.dataset.tool)));
  restoreRead(override);
  $('latest-cue').hidden=nav.channel!=='messages'||!s.incoming;
  if(s.incoming) $('latest-cue').textContent=`${s.incoming} 条新消息 · 跳到最新`;
}
function toLatest() {
  const thread=$('thread'),tail=thread.querySelector('.thread-anchor-space');
  if(tail) tail.style.blockSize='0px';
  thread.scrollTop=thread.scrollHeight;
}
function renderOperation(s) {
  const op=s.operation;
  const fields={
    pending:['正在发送…','info'], rejected:['发送已拒绝 · 草稿已保留','danger'],
    unknown:['发送结果未知 · 草稿已保留','warning'], checking:['正在核对原回执…','info'],
    accepted:['消息已受理','success']
  };
  $('operation').innerHTML=op?`<div class="ax-notice operation-feedback" data-tone="${fields[op.kind][1]}" data-ui-component="SendOutcome"><strong>${fields[op.kind][0]}</strong>
  ${op.kind==='unknown'? `<small>回执 ${E(op.id)}${op.checked?' · 仍未确认':''}</small>${button('check','核对原回执')}`:''}
  </div>`:'';
}
function composerState(sync=false) {
  const a=subject(),s=current();
  if(!a)return;
  const blocked=['pending','unknown','checking'].includes(s.operation?.kind);
  $('compose-block').hidden=nav.channel!=='messages'||Boolean(a.stop);
  $('composer').hidden=a.status==='unknown';
  if(sync || $('draft').value!==s.text) $('draft').value=s.text;
  $('draft').readOnly=['unknown','checking'].includes(s.operation?.kind);
  $('draft').placeholder=a.status==='unknown'?'发送能力未确认':'撰写消息…';
  $('composer').querySelector('button[type="submit"]').disabled=blocked||!currentDraft(s);
  $('composer').querySelectorAll('button[type="button"]').forEach(b=>{b.disabled=blocked;});
  $('compose-recipient').textContent=a.status==='unknown'?'发送能力未确认':`发给 @${a.name}`;
  $('composer-chips').innerHTML=[...s.attach.map(n=>button('remove-chip',n,n,{icon:'close',disabled:blocked})),s.reply?button('remove-chip',`引用 ${s.reply}`,'reply',{disabled:blocked,icon:'close'}):'',s.intent?badge(s.intent):'',s.topic?badge(s.topic):''].join('');
  renderOperation(s);
}
function renderReader() {
  const a=subject();
  if(!a){readerRenderKey='';return;}
  $('reader-identity').innerHTML=`${avatar(a)}<div><h2>@${E(a.name)}</h2><div class="reader-ident-meta">${stateBadge(a)} <small>${E(officeLabel(a.office))} · ${E(a.role)}${a.stop?'':' · 当前实例'}</small></div></div>`;
  const locus=returns.get(a.key);
  const contextReturn=locus?.dialogType==='mailbox'?'返回信箱':locus?.dialogType==='relations'?'返回启动关系':
    locus?.dialogType==='post'?'返回岗位':locus?.dialogType==='contact'?'返回目录路由':
    locus?.dialogType==='connection'?'返回连接与回执':
    locus?.nav.panel==='directory' && locus.nav.mode===nav.mode?'返回楼层目录':'';
  const targetReturn=contextReturn || (nav.mode==='scene'?'返回办公室':'返回会话列表');
  const returnButton=document.querySelector('.reader-return');
  returnButton.title=targetReturn;returnButton.setAttribute('aria-label',targetReturn);
  returnButton.querySelector('.return-locus').textContent=contextReturn;
  returnButton.querySelector('.return-locus').hidden=!contextReturn;
  $('channels').innerHTML=Object.entries(channels).map(([v,l])=>pressed('channel',l,v,nav.channel)).join('');
  const hiddenByFilter=!matches(a);
  const noActor=nav.mode==='scene'&&(a.stop || a.x==null);
  const cue=a.stop?'历史实例 · 只读消息':a.status==='unknown'?'发送能力未确认':hiddenByFilter?'所选实例被当前筛选隐藏':noActor?'此实例没有确认的地图位置':'';
  $('reader-cue').hidden=!cue;
  $('reader-cue').innerHTML=cue?`<span>${cue}</span> ${hiddenByFilter?button('clear-filters','清除筛选'):''}`:'';
  const viewKey=a.key+'\n'+nav.channel;
  if(readerRenderKey!==viewKey) {
    renderThread();readerRenderKey=viewKey;
  } else if(nav.channel==='messages') restoreRead();
  composerState();
}

const detailGrid = fields => `<dl class="detail-grid">${fields.map(([label,value])=>`<dt>${E(label)}</dt><dd>${E(value)}</dd>`).join('')}</dl>`;
const resourceAgent = a=>button('follow',`${exactLabel(a)}`,a.key,{icon:'forward'});
const mailLetters = mail.length;
const legacyLetter = {id:'h00',from:'mora',inc:null,time:'11/12',text:'旧版收件记录，未保留发件实例身份。',read:true};
function mailboxContent() {
  const page = nav.mail==='pending'?mail.filter(m=>!mailHandled.has(m.id)):
    (nav.before?[...mail.slice(2),legacyLetter]:mail.slice(0,2));
  const pending=page.filter(m=>!mailHandled.has(m.id) && m.id!=='h00');
  return `<nav class="resource-tabs" aria-label="信箱类型">${pressed('mail-tab','待处理','pending',nav.mail)}${pressed('mail-tab','历史','all',nav.mail)}</nav>
    <div class="mail-summary"><span class="muted">${mailLetters-mailHandled.size} 封未处理</span>${pending.length?button('handle-page','处理本页待办'):''}</div>
    ${page.length?page.map(m=>{
      const contact=agents.find(a=>a.name===m.from&&a.inc===m.inc&&!a.stop);
      const handled=m.id==='h00'||mailHandled.has(m.id);
      return `<article class="ax-panel mail-item" data-ui-component="MailboxLetter">
        <header><b>@${E(m.from)}</b>${badge(handled?'已处理':'待处理',handled?'neutral':'info')} <time class="muted">${E(m.time)}</time></header>
        <details data-letter="${m.id}"${expandedMail.has(m.id)?' open':''}><summary>读信 · ${m.id}</summary><p>${E(m.text)}</p><small class="muted">${contact?'精确发件人 · 实例 '+contact.inc:'历史名录仅保留名称'}</small></details>
        <footer>${contact?button('follow','打开会话',contact.key,{icon:'forward'}):badge('无法定位发件实例')}${!handled?button('handle','标为已处理',m.id):''}</footer></article>`;
    }).join(''):empty('没有待处理信件','可在历史中查看其他信件。')}
    ${nav.mail==='all'?button('mail-page',nav.before?'返回最新页':'更早的信件',nav.before?'':'h03'):''}`;
}
let routePreview='';
let tempValue='';
const workspaceOptions = ()=> `<div class="ax-stack">${button('tab','查看岗位','posts')}${button('tab','查看技能','skills')}${button('local','联系目录','contact')}${button('local','连接与回执','connection')}${button('local','新建 Agent','launch')}</div>`;
function dialogContent(type) {
  const a=subject();
  if(type==='mailbox') return ['我的信箱',mailboxContent()];
  if(type==='workspace') return ['工作区操作',workspaceOptions()];
  if(type==='post'){
    const p=posts.find(item=>item.id===nav.post);
    if(!p)return ['岗位',empty('该岗位不可用','请选择当前办公室下的定义。')];
    const adopted=agents.filter(x=>!x.stop&&x.post===p.id&&x.office===p.office);
    return [p.title,`<section data-ui-block="PostDefinition" class="ax-stack">${detailGrid([['办公室',officeLabel(p.office)],['当前定义源',p.path],['职责',p.desc]])}<h3>采纳该定义的当前成员</h3>${adopted.length?adopted.map(resourceAgent).join(''):'<p class="muted">暂无成员</p>'}</section>`];
  }
  if(type==='agent' && a) return ['会话详情',`<section data-ui-block="ConversationFacts" class="ax-stack">${detailGrid([
    ['对象',exactLabel(a)],['目录归属',a.office===':unknown'?'未确认':a.office],['身份作用域',SCOPE],['代次',GENERATION],
    ['实例',a.inc],...(a.stop?[['停止事件',a.stop]]:[]),['采纳岗位',a.role],['当前工作目录', a.office.startsWith('/')?a.office+'/scratch':'未提供'],
    ['Provider',a.status==='unknown'?'未提供':'样本 · Codex']
  ])}<div class="ax-cluster">${button('detail','工作绑定','work')}${button('detail','启动关系','relations')}${button('detail','对象操作','actions')}</div></section>`];
  if(type==='work' && a) return ['工作绑定',`<section data-ui-block="BoundWork">${a.inc==='104'?detailGrid([['绑定类型','Task · 合成'],['定义','tasks/web-navigation.task.md'],['声明进度','待复核'],['最近读取','样本 · 10:42'],['Agent 状态',statuses[a.status][0]]]):empty('未提供绑定事实','')}${button('detail','返回会话详情','agent')}</section>`];
  if(type==='relations' && a) {
    const other=a.inc==='104'?agents.find(x=>x.inc==='192'):a.inc==='192'?agents[0]:null;
    return ['启动关系',`<section data-ui-block="LaunchRelations" class="ax-stack">${other?`<strong>${a.inc==='104'?'直接发起者':'直接启动的实例'}</strong>${resourceAgent(other)}`:empty('未提供直接关系','')}${button('detail','返回会话详情','agent')}</section>`];
  }
  if(type==='actions' && a) return ['对象操作',`<div class="ax-stack" data-ui-block="TargetActions"><strong>${E(exactLabel(a))}</strong>${(a.stop?['恢复会话','分支会话']:a.status==='unknown'?[]:['停止参与','终止进程','分支会话']).map(t=>button('confirm',t,t)).join('')||'<p class="muted">能力未确认</p>'}${button('detail','返回会话详情','agent')}</div>`];
  if(type==='confirm') return [tempValue,`<div class="ax-stack"><strong>${E(a?exactLabel(a):'当前办公室')}</strong><p>原型只展示确认与取消，操作不会提交。</p>${button('detail','取消并返回','actions')}</div>`];
  if(type==='attachment') return ['添加样本附件',`<div class="ax-stack">${button('attach','Web 空状态 · 评审摘录.txt','Web 空状态 · 评审摘录.txt',{icon:'clip'})}${button('attach','导航验收清单.txt','导航验收清单.txt',{icon:'clip'})}${button('attach-failure','模拟附件读取失败')}</div>`];
  if(type==='composer-options') {
    const s=current();
    return ['消息选项',`<form class="ax-stack" id="options-form"><label for="draft-intent">意图</label><select id="draft-intent" class="ax-input"><option value="">未指定</option>${['请求','告知','回复'].map(t=>`<option${s.intent===t?' selected':''}>${t}</option>`).join('')}</select><label for="draft-topic">主题</label><input id="draft-topic" class="ax-input" maxlength="60" value="${E(s.topic)}"><button type="submit" class="ax-button" data-variant="primary">应用到草稿</button></form>`];
  }
  if(type==='file') return ['样本附件',`<div class="ax-stack"><strong>${E(tempValue)}</strong><p>这份合成附件用于演示会话引用。内容：进入 · 判定空状态 · 返回到原会话。</p></div>`];
  if(type==='share') return ['精确会话链接',`<label for="share-address">复制地址</label><input class="ax-input" id="share-address" readonly value="${E(locationFor({...nav,panel:'',detail:''}).href)}">`];
  if(type==='contact') {
    const known=routePreview==='/projects/atlas/web'||routePreview==='/projects/atlas';
    const target=routePreview==='/projects/atlas/web'?agents[0]:agents[2];
    const result=known?detailGrid([['解析目录',routePreview],['目录入口',exactLabel(target)],['岗位',target.role]]):
      routePreview?empty(routePreview.includes('ambiguous')?'路由有歧义':'未找到可联系的目录入口','不会选择其他对象。'):'';
    return ['联系目录',`<form id="route-form" class="ax-stack"><label for="dir-path">目录路径</label><input class="ax-input" id="dir-path" placeholder="/projects/atlas/web" value="${E(routePreview)}" required><button type="submit" class="ax-button">预览路由</button></form><div class="ax-stack" id="route-result">${result}${known?resourceAgent(target):''}</div>`];
  }
  if(type==='connection') {
    const pending=[...sessions.entries()].filter(([,s])=>['pending','unknown','checking'].includes(s.operation?.kind));
    return ['连接与回执',`<div class="ax-stack">${detailGrid([['数据源','合成预览 · '+FIXTURE],['范围',SCOPE],['代次',GENERATION],['原回执待核对',pending.length]])}${pending.map(([key,s])=>`<div class="ax-cluster">${badge(s.operation.id,'warning')}${resourceAgent(byKey(key))}</div>`).join('')}</div>`];
  }
  if(type==='launch') return ['新建 Agent',empty('预览不启动 Agent','正式入口将使用当前办公室和实际可用启动配置。',button('local','返回工作区操作','workspace'))];
  return ['详情',''];
}
let suppressClose=0;
function renderDialog() {
  const desired=transient || (nav.detail && nav.key?nav.detail:'') || (nav.post?'post':'') || (nav.panel==='mailbox'?'mailbox':'');
  if(!desired){
    if(dialogs.open){suppressClose++;closeDialog(dialogs);}
    dialogType='';
    return;
  }
  const [title,body]=dialogContent(desired);
  dialogType=desired;
  dialogs.dataset.form=desired;
  $('dialog-title').textContent=title;
  $('dialog-body').innerHTML=body;
  $('dialog-body').querySelectorAll('[data-letter]').forEach(n=>n.addEventListener('toggle',()=>n.open?expandedMail.add(n.dataset.letter):expandedMail.delete(n.dataset.letter)));
  if(!dialogs.open) openDialog(dialogs,modalTrigger);
  if(desired==='share'){
    $('share-address').focus();
    $('share-address').select();
  }
}
function openLocal(type, trigger, value='') {
  modalTrigger=trigger;
  tempValue=value;
  transient=type;
  renderDialog();
}
dialogs.addEventListener('close',()=>{
  if(suppressClose){suppressClose--;return;}
  if(dialogs.open)return;
  if(transient) {transient='';renderDialog();return;}
  if(nav.detail || nav.post || nav.panel==='mailbox') transition({detail:'',post:'',panel:nav.panel==='mailbox'?'':nav.panel},{replace:true});
});
function render() {
  renderRoute();renderLayout();renderIndex();renderScene();renderReader();renderDialog();
}

function selectAgent(key, source, fromDisclosure=false){
  const a=byKey(key);
  if(!a)return;
  const previous={...nav};
  const from=fromDisclosure?{nav:previous,transient,dialogType}:null;
  if(from) returns.set(key,from);
  else {
    returns.delete(key);
    if(nav.mode==='scene' && !a.stop && nav.panel==='directory') returns.set(key,{nav:{...previous,key:'',channel:'messages'},transient:'',dialogType:''});
  }
  transient='';
  const nextTab=fromDisclosure||!matches(a)?a.stop?'history':'members':nav.tab;
  transition({key,channel:key===nav.key?nav.channel:'messages',office:fromDisclosure?a.office:nav.office,
    tab:nextTab,detail:'',post:'',panel:''},{reader:true,force:!!source});
}
function returnReader(){
  const prev=nav.key, saved=returns.get(prev);
  returns.delete(prev);
  if(saved && !(saved.nav.panel==='directory' && saved.nav.mode!==nav.mode)){
    transient=saved.transient;
    transition(saved.nav,{force:true,reader:!!saved.nav.key,focus:saved.nav.key?null:prev});
  }else transition({key:'',channel:'messages',detail:''},{focus:prev});
}
function sendMessage() {
  const a=subject(),s=current();
  if(!a||a.stop||a.status==='unknown'||nav.channel!=='messages'||!currentDraft(s)||['pending','unknown','checking'].includes(s.operation?.kind))return;
  const result=$('send-mode').value;
  const op={kind:'pending',id:`S-${++serial}`,result,text:s.text,attach:[...s.attach],reply:s.reply,intent:s.intent,topic:s.topic,rev:s.rev};
  s.operation=op;
  composerState();
  setTimeout(()=>{
    if(s.operation!==op)return;
    if(result==='accepted') accept(a,s,op);
    else {
      op.kind=result==='rejected'?'rejected':'unknown';
      if(nav.key===a.key)composerState();
      renderIndex();
    }
  },360);
}
function accept(a,s,op){
  if(op.kind==='accepted')return;
  op.kind='accepted';
  s.added.push({id:op.id,who:'你',time:'10:50',text:op.text,reply:op.reply,file:op.attach[0],files:op.attach,thread:op.topic,intent:op.intent});
  if(s.rev===op.rev){s.text='';s.attach=[];s.reply='';s.intent='';s.topic='';s.rev++;}
  renderIndex();
  if(nav.key===a.key){
    composerState(true);
    if(nav.channel==='messages'){renderThread();toLatest();s.incoming=0;$('latest-cue').hidden=true;}
  }
}
function checkReceipt(){
  const a=subject(),s=current(),op=s?.operation;
  if(op?.kind!=='unknown')return;
  op.kind='checking';composerState();
  setTimeout(()=>{
    if(s.operation!==op)return;
    if(op.result==='unknown')accept(a,s,op);
    else {op.kind='unknown';op.checked=true;if(nav.key===a.key)composerState();}
  },320);
}
function loadProvider(){
  const a=subject(),s=current();
  if (!a || s.provider==='loading' || a.status==='unknown')return;
  const choice=$('read-mode').value;
  s.provider='loading';renderThread();
  setTimeout(()=>{
    s.provider=choice==='normal'?'ready':choice;
    if(a.key===nav.key&&nav.channel==='model'){
      renderThread();
      $('thread').focus({preventScroll:true});
    }
  },300);
}
function receive(){
  const a=subject()||agents[0],s=messageSessions(a.key);
  if(a.stop)return announce('历史实例未生成新样本');
  const atMessages=nav.key===a.key&&nav.channel==='messages';
  if(atMessages)captureRead();
  const stay=atMessages&&!s.read.messages?.latest;
  s.added.push({id:'incoming-'+(++serial),who:a.name,time:'10:54',text:'我补了一份复核记录：如果你还在看旧消息，先留在原处，稍后可以跳回这里。'});
  if(stay)s.incoming++;
  if(atMessages){renderThread();if(!stay)toLatest();}
  announce('收到一条合成 AgentX 消息');
}
async function share(trigger){
  const url=locationFor({...nav,panel:'',detail:''}).href;
  try {
    if(navigator.clipboard && window.isSecureContext) await navigator.clipboard.writeText(url);
    else{
      const el=document.createElement('textarea');
      el.className='sr-only';el.value=url;document.body.append(el);
      let copied;
      try {el.select();copied=document.execCommand('copy');} finally {el.remove();trigger.focus();}
      if(!copied) throw new Error('clipboard unavailable');
    }
    announce('已复制精确会话链接');
  } catch {openLocal('share',trigger);}
}
function reset(){
  const url=new URL(location.href);
  const clean=locationFor(defaults);
  // Keep only unrelated host hints and hash. Navigation/drafts/fixture effects reset.
  url.search=clean.search;
  location.replace(url.href);
  if(url.href===location.href)location.reload();
}

$('scope').innerHTML='<option value="">全部办公室</option>'+offices.map(o=>`<option value="${E(o.id)}">${E(o.short)}</option>`).join('');
$('scope').addEventListener('change',e=>setOffice(e.target.value));
function setOffice(id){
  transient='';
  transition({office:id,key:'',channel:'messages',post:'',detail:'',panel:'',mail:'pending',before:'',tab:'members'});
}
$('search').addEventListener('input',e=>transition({q:e.target.value.slice(0,100)},{replace:true}));
$('status').addEventListener('change',e=>transition({status:e.target.value},{replace:true}));
$('draft').addEventListener('input',e=>{
  const s=current();
  if(!s)return;
  s.text=e.target.value;s.rev++;
  $('composer').querySelector('[type="submit"]').disabled=['pending','unknown','checking'].includes(s.operation?.kind)||!currentDraft(s);
});
$('composer').addEventListener('submit',e=>{e.preventDefault();sendMessage();});
$('thread').addEventListener('input',e=>{if(e.target.id==='note')current().noteDraft=e.target.value;});
$('thread').addEventListener('submit',e=>{
  if(e.target.id!=='note-form')return;
  e.preventDefault();
  const s=current();
  if(s.noteDraft.trim()){s.notes.push(s.noteDraft);s.noteDraft='';renderThread();$('note').focus();}
});
$('dialog-body').addEventListener('input',e=>{
  if(e.target.id==='dir-path' && routePreview && e.target.value.trim()!==routePreview){
    routePreview='';
    $('route-result').replaceChildren();
  }
});
$('dialog-body').addEventListener('submit',e=>{
  if(e.target.id==='route-form'){e.preventDefault();routePreview=$('dir-path').value.trim();renderDialog();$('dir-path').focus();}
  if(e.target.id==='options-form'){
    e.preventDefault();const s=current();
    s.intent=$('draft-intent').value;s.topic=$('draft-topic').value.trim();s.rev++;
    closeDialog(dialogs);composerState();
  }
});
document.addEventListener('click',event=>{
  const b=event.target.closest('[data-action]');
  if(!b || b.disabled)return;
  const action=b.dataset.action, val=b.dataset.value||'';
  switch(action){
    case 'mode':
      transition({mode:val,tab:nav.tab==='history'&&!subject()?.stop?'members':nav.tab,panel:nav.panel==='directory'?'':nav.panel});break;
    case 'office':setOffice(val);$('scope').focus();break;
    case 'tab':
      transient='';
      transition({tab:val,post:'',panel:'',detail:''},{force:true});
      document.querySelector('#resource-tabs [aria-pressed="true"]')?.focus({preventScroll:true});break;
    case 'select':selectAgent(val,b);break;
    case 'follow':selectAgent(val,b,true);break;
    case 'return':returnReader();break;
    case 'channel':
      transition({channel:val});$('channels').querySelector('[aria-pressed="true"]')?.focus({preventScroll:true});break;
    case 'clear-route':transition({...defaults});$('search').focus();break;
    case 'clear-filters':transition({q:'',status:''},{replace:true});$('search').focus();break;
    case 'directory':
      transition({panel:nav.panel==='directory'?'':'directory'});
      if(nav.panel==='directory')$('map-directory').querySelector('button')?.focus({preventScroll:true});
      else document.querySelector('.map-actions [data-action="directory"]').focus();break;
    case 'zoom':scene.zoom(Number(val));break;
    case 'pan': { const [x,y]=val.split(',').map(Number);scene.move(x,y);break; }
    case 'reframe':scene.reset();break;
    case 'facts':modalTrigger=b;transition({detail:'agent'});break;
    case 'detail':transient='';transition({detail:val},{force:true});dialogs.querySelector('[autofocus]').focus();break;
    case 'share':share(b);break;
    case 'workspace':openLocal('workspace',b);break;
    case 'local':openLocal(val,b);break;
    case 'confirm':openLocal('confirm',b,val);break;
    case 'post':modalTrigger=b;transition({post:val});break;
    case 'mail':modalTrigger=b;transition({panel:'mailbox'});break;
    case 'mail-tab':
      transition({mail:val,before:''});$('dialog-body').querySelector('[aria-pressed="true"]')?.focus();break;
    case 'mail-page':transition({before:val});$('dialog-body').querySelector('[data-action="mail-page"]')?.focus();break;
    case 'handle':mailHandled.add(val);renderLayout();renderScene();renderDialog();$('dialog-title').parentElement.querySelector('button').focus();announce('信件已标为处理');break;
    case 'handle-page':
      (nav.mail==='pending'?mail:nav.before?mail.slice(2):mail.slice(0,2)).forEach(m=>mailHandled.add(m.id));
      renderLayout();renderScene();renderDialog();$('dialog-title').parentElement.querySelector('button').focus();announce('本页明确列出的待办已处理');break;
    case 'attachment-picker':openLocal('attachment',b);break;
    case 'composer-options':openLocal('composer-options',b);break;
    case 'attach':{
      const s=current();
      if(!s.attach.includes(val)) {s.attach.push(val);s.rev++;}
      closeDialog(dialogs);composerState();break;
    }
    case 'attach-failure':closeDialog(dialogs);announce('样本附件读取失败 · 现有草稿已保留');break;
    case 'remove-chip':{
      const s=current();
      if(val==='reply')s.reply='';else s.attach=s.attach.filter(n=>n!==val);
      s.rev++;composerState();$('draft').focus();break;
    }
    case 'reply':{
      const a=subject(),s=current();
      const latest=envelopes(a,s).at(-1);
      if(latest){s.reply=latest.id;s.rev++;composerState();}
      $('draft').focus();break;
    }
    case 'file':openLocal('file',b,val);break;
    case 'older':{
      captureRead();
      const s=current(), prev=s.read.messages;
      s.older=true;renderThread(prev?{...prev,latest:false}:null);
      $('thread').focus({preventScroll:true});break;
    }
    case 'history-start':$('thread').scrollTop=0;$('thread').focus({preventScroll:true});break;
    case 'latest':
      toLatest();current().incoming=0;$('latest-cue').hidden=true;$('thread').focus({preventScroll:true});break;
    case 'load-provider':loadProvider();break;
    case 'check':checkReceipt();break;
    case 'receive':$('demo-menu').hidePopover();receive();break;
    case 'reset':reset();break;
  }
});
document.addEventListener('keydown',e=>{
  if(e.key!=='Escape'||dialogs.open||$('demo-menu').matches(':popover-open'))return;
  if(nav.panel==='directory'){e.preventDefault();transition({panel:''});document.querySelector('.map-actions [data-action="directory"]').focus();}
});
render();
if(nav.mode==='scene' && nav.key)requestAnimationFrame(()=>scene.bringIntoView(subject()));
