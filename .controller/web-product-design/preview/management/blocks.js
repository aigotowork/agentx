import {e, icon, badge, btn, ibtn, option, notice, field, input, select, empty, refTitle} from './ui.js';
import {OFFICES, ROOT, TOOLS, PROFILES, key} from './fixtures.js';
import {agentStates,operationStates,kinds,offices,capability,alerting,SCOPE} from './model.js';

const navDefs=[['agents','Agents'],['workspaces','Workspaces'],['operations','Operations']];
const stateBadge = a => badge(...agentStates[a.status]);
const opBadge = op => badge(...operationStates[op.state]);
const workspaceName = a => a.adoption==='unavailable'?'归属不可用':a.adoption==='legacy'?'全局 · 旧记录':offices[a.office]?.name||'—';
const routeLink=(path,label,glyph,cls='')=>'<a class="inline-link '+cls+'" href="#'+path+'">'+e(label)+(glyph?icon(glyph):'')+'</a>';
const dataPair=(label,value,mono=false)=>'<div><dt>'+e(label)+'</dt><dd'+(mono?' class="mono"':'')+'>'+e(value)+'</dd></div>';
const agentToken=a=>'<span class="agent-token" data-state="'+a.status+'">'+e(a.name.slice(0,2).toUpperCase())+'</span>';
const tab=(value,label,current,cmd)=>'<button type="button" class="segment" data-cmd="'+cmd+'" data-value="'+value+'" aria-pressed="'+String(current===value)+'">'+e(label)+'</button>';

export function ScopeBar(m,page){
  const unresolved=m.ops.filter(x=>['lost','missing','unknown','conflict'].includes(x.state)).length;
  const nav=navDefs.map(([id,label])=>'<a href="#'+id+'" class="global-nav-link"'+(id===page?' aria-current="page"':'')+'>'+icon(id)+'<span>'+label+'</span>'+(id==='operations'&&unresolved?'<span class="unresolved-count" aria-label="'+unresolved+' 条需关注">'+unresolved+'</span>':'')+'</a>').join('');
  return '<header class="scope-bar" data-ui-block="scope-bar"><div class="product-brand">'+icon('logo')+'<div><b>AgentX</b><span>Management</span></div></div><div class="scope-context">'+icon('shield')+'<span>'+SCOPE+'</span><code>'+e(m.gen)+'</code></div><div class="scope-actions"><button type="button" class="ax-button demo-toggle" popovertarget="demo-settings" aria-label="演示" title="演示">'+icon('more')+'<span>演示</span></button>'+btn('启动 Agent','launch',{variant:'primary',glyph:'plus'})+'</div></header><nav class="global-nav" aria-label="资源导航">'+nav+'<a class="all-previews" href="../">全部设计 '+icon('up')+'</a></nav>';
}

export function AgentSummary(m, ui){
  const counts={
    all:m.agents.length, attention:m.agents.filter(alerting).length,
    live:m.agents.filter(a=>a.status!=='stopped').length, stopped:m.agents.filter(a=>a.status==='stopped').length
  };
  return '<div class="agent-summary" aria-label="已加载 Agent 的状态摘要" data-ui-block="loaded-status">'+[
    ['all','全部','neutral'],['attention','需关注','danger'],['live','当前','info'],['stopped','已停止','neutral']
  ].map(([name,label,tone])=>'<button type="button" class="summary-item" data-tone="'+tone+'" data-cmd="agent-filter" data-value="'+name+'" aria-pressed="'+String(ui.filter===name)+'"><span>'+e(label)+'</span><strong>'+counts[name]+'</strong></button>').join('')+'</div>';
}

export function matchingAgents(m, ui){
  const q=ui.query.toLowerCase().trim();
  return m.agents.filter(a=>{
    if(ui.filter==='attention'&&!alerting(a)) return false;
    if(ui.filter==='live'&&a.status==='stopped') return false;
    if(ui.filter==='stopped'&&a.status!=='stopped') return false;
    if(ui.officeFilter&&a.office!==ui.officeFilter) return false;
    if(ui.labelFilter&&!Object.entries(a.labels).some(([k,v])=>k+'='+v===ui.labelFilter)) return false;
    return !q||[a.name,a.role,a.id,workspaceName(a)].some(x=>x?.toLowerCase().includes(q));
  });
}

export function AgentIndex(m,ui,selected){
  const choices=matchingAgents(m,ui);
  const labels=[...new Set(m.agents.flatMap(a=>Object.entries(a.labels).map(([k,v])=>k+'='+v)))];
  const rows=choices.slice(0,ui.limit).map(a=>{
    const match=key(a)===selected;
    return '<li><a class="resource-row agent-row" href="#agents/'+encodeURIComponent(key(a))+'"'+(match?' aria-current="true"':'')+'>'+agentToken(a)+'<div class="resource-name"><strong>@'+e(a.name)+'</strong><span>'+e(workspaceName(a))+(a.role?' / '+e(a.role):'')+'</span></div>'+stateBadge(a)+'<time>'+e(a.time)+'</time></a></li>';
  }).join('');
  return '<aside class="index-panel" data-ui-block="agent-index"><div class="index-head"><div class="index-title"><h2>Agent 列表</h2><span>'+choices.length+' 条匹配 · 已加载</span></div><div class="search-wrap">'+icon('search')+input('query',ui.query,'type="search" placeholder="搜索名称、Role 或 ID" aria-label="搜索 Agent"')+'</div><div class="filter-controls">'+
    '<label><span class="sr-only">工作区筛选</span>'+select('officeFilter',option('','所有工作区',ui.officeFilter)+OFFICES.map(o=>option(o.id,o.name,ui.officeFilter)).join(''))+'</label>'+
    '<label><span class="sr-only">标签筛选</span>'+select('labelFilter',option('','所有标签',ui.labelFilter)+labels.map(o=>option(o,o,ui.labelFilter)).join(''))+'</label>'+
    '</div></div><div class="index-scroll" data-scroll="agents">'+(rows?'<ul class="resource-list">'+rows+'</ul>':empty('没有匹配的 Agent','试试调整当前筛选。'))+'</div><div class="index-foot">'+(choices.length>ui.limit?btn('显示更多','more-agents',{glyph:'collapse'}):'<span>'+Math.min(choices.length,ui.limit)+' / '+choices.length+' 条匹配</span>')+'<button type="button" class="ax-button ax-icon-button" data-variant="quiet" popovertarget="coverage" aria-label="列表覆盖范围" title="列表覆盖范围">'+icon('info')+'</button><div id="coverage" class="ax-popover coverage" popover>仅统计本地已加载记录。实际 daemon 名录可能有更多分页。</div></div></aside>';
}

const labelsLine = a => Object.entries(a.labels).map(([k,v])=>'<code class="label-chip">'+e(k)+'='+e(v)+'</code>').join('');
export function AgentWorkArea(m,ui,a){
  if(!a) return '<section class="detail-panel" data-ui-block="agent-work-area">'+empty(ui.pathId?'原实例不可用':'选择 Agent',ui.pathId?'没有自动改选同名实例。请从列表选择确切引用。':'选择一行查看确切执行与消息。','agents')+'</section>';
  const k=key(a);
  const tabId=ui.views[k]||'messages';
  const issue=ui.stale?notice('刷新失败 · 正在显示上次读取结果。写操作暂不可用。'):a.issue?notice(a.issue,a.status==='failed'?'danger':'warning'):'';
  return '<section class="detail-panel agent-detail" data-ui-block="agent-work-area"><div class="detail-head">'+
    '<div class="subject-line">'+agentToken(a)+'<div class="subject-text"><div class="subject-title"><h2>@'+e(a.name)+'</h2>'+stateBadge(a)+'</div><code class="exact-ref">'+(a.ref==='incarnation'?'执行':'停止事件')+' #'+e(a.id)+'</code></div><div class="subject-actions">'+ibtn('实例详情','facts','info')+btn('标签','labels',{disabled:!capability(a,'labels')||ui.stale})+
    (a.status==='stopped'?btn('恢复执行','resume',{disabled:!capability(a,'resume',m)||ui.stale}):btn('终止进程','stop',{variant:'danger',disabled:!capability(a,'stop')||ui.stale}))+'</div></div>'+
    '<div class="subject-meta">'+icon('workspaces')+(a.office?routeLink('workspaces/'+a.office,workspaceName(a),'up'):e(workspaceName(a)))+'<span class="meta-divider"></span><span>'+e(a.role||'无已记录 Role')+'</span><span class="meta-divider"></span><span>'+e(a.tool)+' / '+e(a.profile)+'</span>'+labelsLine(a)+'</div>'+issue+
    '<div class="view-tabs" role="group" aria-label="Agent 内容">'+tab('messages','消息',tabId,'agent-tab')+tab('transcript','Transcript',tabId,'agent-tab')+'</div></div>'+
    (tabId==='messages'?MessagesBlock(m,ui,a):TranscriptBlock(m,a,ui))+'</section>';
}

function MessagesBlock(m,ui,a){
  const k=key(a);
  const messages=m.messages[k]||[];
  const draft=m.drafts[k]||{};
  const canSend=capability(a,'send')&&!ui.stale;
  const linked=m.ops.find(o=>o.gen===m.gen&&o.target===k&&o.kind==='send');
  return '<div class="channel" data-ui-block="messages"><div class="channel-head"><span>'+icon('envelope')+'AgentX 通信</span><span>'+messages.length+' 条 · 当前快照</span></div>'+
    '<div class="channel-scroll" data-scroll="messages">'+(messages.length?'<ol class="message-list">'+messages.map(msg=>'<li class="message-row"><div class="message-avatar">'+e(msg.by.slice(0,1).toUpperCase())+'</div><div><div class="message-meta"><strong>@'+e(msg.by)+'</strong><code>'+e(msg.intent)+'</code><time>'+e(msg.time)+'</time>'+
    (msg.thread?'<code class="thread-label">'+e(msg.thread)+'</code>':'')+
    (canSend?btn('回复','reply',{value:msg.id,id:'reply-'+msg.id}):'')+
    '</div>'+(msg.reply_to?'<span class="reply-reference">回复 #'+e(msg.reply_to)+'</span>':'')+'<p>'+e(msg.text)+'</p></div></li>').join('')+'</ol>':empty('暂无消息','当前确切实例未读取到消息。','envelope'))+'</div>'+
    (linked?'<div class="inline-receipt"><span>'+opBadge(linked)+' '+e(linked.state==='accepted'?'消息已受理':'最近发送')+'</span>'+btn('查看回执','receipt',{value:linked.id,glyph:'chevron'})+'</div>':'')+
    (canSend?'<form class="composer" data-form="send">'+
    (draft.reply_to?'<div class="reply-context"><span>回复 #'+e(draft.reply_to)+'</span><input type="hidden" name="reply_to" value="'+e(draft.reply_to)+'">'+btn('取消回复','clear-reply')+'</div>':'')+
    '<label><span class="sr-only">给 '+e(a.name)+' 的消息</span><textarea class="ax-input" name="message" rows="2" placeholder="给 @'+e(a.name)+' 写消息…" required maxlength="500">'+e(draft.message)+'</textarea></label><div class="composer-actions"><label class="intent-label">意图 '+select('intent',['request','inform','ack'].map(x=>option(x,x,draft.intent||'request')).join(''))+'</label><button class="ax-button" data-variant="primary" type="submit">'+icon('send')+'发送消息</button></div></form>':
    '<div class="composer-disabled">'+(a.status==='stopped'?'停止快照 · 只读消息':ui.stale?'刷新成功后可以继续操作。':'该实例目前不可发送。')+'</div>')+'</div>';
}

function TranscriptBlock(m,a,ui){
  const state=m.snapshots[key(a)];
  const unavailable=state==='不可用';
  return '<div class="channel" data-ui-block="transcript"><div class="channel-head"><span>'+icon('type')+'Provider 会话记录</span>'+btn(state?'重新读取':'读取快照','transcript',{disabled:!capability(a,'transcript')||ui.stale,glyph:'refresh'})+'</div><div class="channel-scroll" data-scroll="transcript">'+
    (state==='已读取'?'<div class="snapshot"><div class="snapshot-banner">'+badge('有界快照','info')+'<span>Provider 记录 · 并非 AgentX 通信</span></div><ol class="message-list"><li class="model-exchange"><b>请求</b><p>核对发布清单，并标注验证边界。</p></li><li class="model-exchange"><b>模型回复</b><p>先核对版本、构建产物和浏览器路径，再记录覆盖范围。</p></li></ol></div>':
    empty(unavailable?'快照不可用':state||'尚未读取快照',unavailable?'未找到 Provider 会话文件。':state==='空快照'?'此快照没有可显示的交换。':state==='读取失败'?'Provider 任务失败，详见操作回执。':state==='结果未知'?'查看原回执；未知操作不提供重放。':'用「读取快照」查看有限的模型交换。','type'))+'</div></div>';
}

export function AgentFacts(a){
  const office=offices[a.office];
  return '<div class="fact-intro">'+agentToken(a)+'<div><strong>@'+e(a.name)+'</strong> '+stateBadge(a)+'<p class="mono">'+e(refTitle(key(a)))+'</p></div></div><dl class="fact-grid">'+
  dataPair('引用类型',a.ref,true)+dataPair('更新',a.time)+(a.title?dataPair('启动标题',a.title):'')+dataPair('协作采纳状态',({adopted:'当前记录已采纳',legacy:'旧记录 · 未保存采纳信息',unavailable:'不可用'})[a.adoption])+
  dataPair('采纳工作区',a.adoption==='adopted'?office?.path||'不可用':'未记录',true)+dataPair('采纳 Role',a.role||'无记录')+dataPair('启动档案',a.profile,true)+dataPair('进程目录 cwd',a.cwd,true)+
  dataPair('标签',Object.entries(a.labels).map(([k,v])=>k+'='+v).join(', ')||'—',true)+'</dl>';
}

export function WorkspaceIndex(selected){
  return '<aside class="index-panel" data-ui-block="workspace-index"><div class="index-head"><div class="index-title"><h2>已知工作区</h2><span>有限发现</span></div><form class="explicit-path" data-form="workspace-path"><label class="sr-only" for="office-path">显式工作区路径</label>'+input('path','','id="office-path" placeholder="绝对路径或 global" required')+'<button class="ax-button" type="submit" title="打开工作区" aria-label="打开工作区">'+icon('arrow')+'</button></form></div><div class="index-scroll" data-scroll="workspaces"><ul class="resource-list">'+OFFICES.map(o=>'<li><a class="resource-row workspace-row" href="#workspaces/'+o.id+'"'+(selected===o.id?' aria-current="true"':'')+'><span class="resource-icon">'+icon('workspaces')+'</span><div class="resource-name"><strong>'+e(o.name)+'</strong><span class="mono">'+e(o.path)+'</span></div>'+icon('chevron')+'</a></li>').join('')+'</ul></div><div class="index-foot"><span>已知路径 '+OFFICES.length+' 条</span><button type="button" class="ax-button ax-icon-button" data-variant="quiet" popovertarget="workspace-coverage" aria-label="工作区发现覆盖范围">'+icon('info')+'</button><div id="workspace-coverage" class="ax-popover coverage" popover>本地样本：已采纳记录与有界标记发现。该集合不是完整的目录索引。</div></div></aside>';
}

const routeStates={bound:'已绑定',absent:'无绑定端点',ambiguous:'候选不唯一',stale:'绑定已过期',unknown:'绑定未知',role_changed:'Role 已变化',adoptable:'有可采纳候选',error:'无法预览'};
export function RouteBlock(m,ui,o){
  const draft=ui.routes[o.id]||{};
  const route=draft.preview;
  const associated=m.ops.find(op=>op.kind==='directory'&&op.target===route?.path);
  const tone=s=>({bound:'success',absent:'warning',ambiguous:'danger',stale:'warning',unknown:'warning',role_changed:'warning',adoptable:'info',error:'danger'})[s];
  return '<section class="workspace-section" data-ui-block="directory-route"><div class="section-title"><div class="section-identity">'+icon('branch')+'<h3>目录路由</h3></div>'+badge('只读预览','info')+'</div>'+
  '<form class="directory-input" data-form="route"><label class="sr-only" for="route-directory">联系目录 · 绝对路径</label>'+input('directory',draft.directory??(o.id==='web'?o.path+'/src':o.path==='global'?'':o.path),'id="route-directory" placeholder="联系目录 · 绝对路径" required')+'<button class="ax-button" data-variant="primary" type="submit">检查路由</button></form>'+
  (route?'<div data-route-result'+(draft.changed?' hidden':'')+'><p class="route-fact mono">已检查 '+e(route.path)+'</p><div class="route-steps" aria-label="预览路由"><div><span>选择的工作区</span><strong>'+e(route.office?.path||'—')+'</strong></div>'+icon('chevron')+'<div><span>声明来源 / 默认 Role</span><strong>'+e(route.role?route.office.path+' / '+route.role.name:'—')+'</strong></div>'+icon('chevron')+'<div><span>端点</span>'+badge(routeStates[route.state],tone(route.state))+'</div></div>'+
    (route.recipient?'<p class="route-fact mono">'+e(refTitle(route.recipient))+'</p>':'')+
    (route.reason?notice(route.reason,tone(route.state)):'')+
    '<div class="route-bottom">'+(associated?'<span>'+opBadge(associated)+' '+btn('查看回执','receipt',{value:associated.id})+'</span>':'<span></span>')+
    btn('联系目录','directory',{glyph:'send',disabled:route.state==='error'||draft.changed})+'</div></div>':'')+'</section>';
}

export function WorkspaceWorkArea(m,ui,o) {
  if(!o) return '<section class="detail-panel">'+empty('未知工作区','此显式路径不在模拟数据内。')+'</section>';
  const members=m.agents.filter(a=>a.office===o.id&&a.adoption==='adopted');
  return '<section class="detail-panel workspace-detail" data-ui-block="workspace-work-area"><div class="detail-head"><div class="subject-line"><span class="resource-icon large">'+icon('workspaces')+'</span><div class="subject-text"><div class="subject-title"><h2>'+e(o.name)+'</h2>'+badge(o.source)+'</div><code class="exact-ref">'+e(o.path)+'</code></div></div></div><div class="workspace-scroll" data-scroll="office-detail">'+
    '<section class="workspace-section" data-ui-block="effective-roles"><div class="section-title"><div class="section-identity">'+icon('shield')+'<h3>当前有效 Roles</h3></div><span>'+(o.roles?.length??'—')+'</span></div>'+
    (o.error?notice(o.error,'danger'):'<ul class="role-list">'+o.roles.map(r=>'<li><div><strong>'+e(r.name)+'</strong> '+(r.default?badge('默认','info'):'')+'<p>'+e(r.note)+'</p></div><span class="role-origin">'+e(r.origin)+'</span><code>'+e(r.profile||'启动时选择')+'</code>'+btn('启动','launch',{value:o.id+'~'+r.name,glyph:'plus'})+'</li>').join('')+'</ul>')+'</section>'+
    '<section class="workspace-section" data-ui-block="workspace-members"><div class="section-title"><div class="section-identity">'+icon('agents')+'<h3>已采纳的 Agent</h3></div><span>'+members.length+' 条已加载</span></div>'+
    (members.length?'<div class="member-list">'+members.map(a=>'<a class="member-chip" href="#agents/'+encodeURIComponent(key(a))+'"><b>@'+e(a.name)+'</b> '+stateBadge(a)+'</a>').join('')+'</div>':'<p class="empty-inline">当前没有已加载的采纳成员</p>')+
    '</section>'+RouteBlock(m,ui,o)+'</div></section>';
}

export function OperationIndex(m,ui,selected){
  const unresolved=o=>['lost','missing','unknown','conflict'].includes(o.state);
  const ops=m.ops.filter(o=>ui.opFilter==='all'||unresolved(o));
  return '<aside class="index-panel" data-ui-block="browser-journal"><div class="index-head"><div class="index-title"><h2>浏览器日志</h2><span>'+m.ops.length+' 条本地记录</span></div><div class="filter-tabs">'+tab('all','全部',ui.opFilter,'op-filter')+tab('unresolved','需关注',ui.opFilter,'op-filter')+'</div></div><div class="index-scroll" data-scroll="operations">'+
    (ops.length?'<ul class="resource-list">'+ops.map(op=>'<li><a class="resource-row operation-row" href="#operations/'+op.id+'"'+(selected===op.id?' aria-current="true"':'')+'><span class="resource-icon">'+icon('operations')+'</span><div class="resource-name"><strong>'+e(kinds[op.kind])+'</strong><span>'+e(op.label)+' · '+e(op.time)+'</span></div>'+opBadge(op)+'</a></li>').join('')+'</ul>':empty('没有匹配的回执','切换筛选查看本地日志。','operations'))+'</div><div class="index-foot">仅显示此浏览器保存的请求</div></aside>';
}

export function ReceiptBlock(m,op,inline=false){
  if(!op) return empty('选择请求','查看原始输入、结果与恢复操作。','operations');
  const old=op.gen!==m.gen;
  const linked=(op.kind==='directory'?op.subject:op.target);
  const linkExists=op.gen===m.gen&&m.agents.some(a=>key(a)===linked);
  const follow=!old&&['launch','stop','resume'].includes(op.kind)&&op.state==='succeeded'?
    btn('在 Agents 中查找','find-operation',{value:op.id,glyph:'search'}):linkExists?routeLink('agents/'+encodeURIComponent(linked),op.kind==='directory'?'查看实际收件人':'查看确切实例','up'):'';
  let recovery='';
  if(old) recovery=notice('原请求属于 '+op.gen+'，当前为 '+m.gen+'。此范围不能查询或重放该请求。','danger');
  else if(op.state==='lost') recovery=notice('尚未确认请求是否受理。先按原请求 ID 查询。')+btn('查询原回执','lookup',{value:op.id,variant:'primary',glyph:'search',id:'receipt-lookup'});
  else if(op.state==='missing') recovery=notice('未查到原请求。可以明确重放同一请求，输入和 ID 会保持一致。')+btn('重放原请求','replay',{value:op.id,variant:'primary',glyph:'refresh',id:'receipt-replay'});
  else if(op.state==='unknown') recovery=notice('原操作可能已产生效果。当前结果未知，不能重放。','danger');
  else if(op.state==='conflict') recovery=notice('请求存在冲突，不能重放。','danger');
  return '<div class="receipt-content" data-ui-block="'+(inline?'contextual-receipt':'receipt-work-area')+'">'+
    '<div class="receipt-heading"><div><div class="subject-title"><h2>'+e(kinds[op.kind])+'</h2>'+opBadge(op)+'</div><span>'+e(op.label)+' · '+e(op.time)+'</span></div>'+follow+'</div>'+
    (op.result?notice(op.result,['accepted','succeeded'].includes(op.state)?'success':'danger'):'')+
    '<dl class="fact-grid">'+dataPair('请求 ID',op.id,true)+dataPair('范围 / 代次',SCOPE+' / '+op.gen,true)+dataPair('请求目标',refTitle(op.target)||op.label,true)+(op.messageId?dataPair('消息 ID',op.messageId,true):'')+
    (op.kind==='directory'&&op.subject?dataPair('实际收件人',refTitle(op.subject),true)+dataPair('实际 Role / 工作区',op.route.role+' / '+op.route.workspace,true)+dataPair('端点',op.route.created?'本次启动':'复用现有端点'):'')+'</dl>'+
    '<div class="receipt-recovery">'+recovery+'</div><details class="payload"><summary>原始请求输入</summary><pre>'+e(JSON.stringify(op.input,null,2))+'</pre></details></div>';
}

export function LaunchForm(prefill,values={}){
  const selectedOffice=values.office||prefill?.split('~')[0]||'atlas';
  const o=offices[selectedOffice]||offices.atlas;
  const role=values.role??prefill?.split('~')[1]??'';
  return '<form data-form="launch" class="dialog-form"><div class="form-grid">'+
    field('标题（可选）',input('title',values.title,'maxlength="64" placeholder="例如 发布复核" autofocus'))+
    field('采纳工作区',select('office',OFFICES.map(x=>option(x.id,x.path,selectedOffice)).join('')))+
    field('Role',select('role',option('','不选择 Role',role)+(o.roles||[]).map(x=>option(x.name,x.name,role)).join('')))+
    field('工具 / 启动档案',select('runtime',option('tool:codex','codex',values.runtime||'tool:codex')+TOOLS.filter(t=>t!=='codex').map(t=>option('tool:'+t,t,values.runtime)).join('')+PROFILES.map(p=>option('profile:'+p,p,values.runtime)).join('')).replace('<select ',role?' <select disabled ':'<select '))+
    field('进程目录 cwd',input('cwd',values.cwd??(o.path==='global'?ROOT:o.path),'required pattern="/.+"'),true)+
    field('初始提示词（可选）','<textarea class="ax-input" name="prompt" rows="3" maxlength="500">'+e(values.prompt)+'</textarea>',true)+
    '</div><div class="ax-dialog-actions">'+btn('取消','dismiss')+'<button type="submit" class="ax-button" data-variant="primary">启动</button></div></form>';
}

export function LabelsForm(a,revision,draft){
  const text=draft??Object.entries(a.labels).map(([key,val])=>key+'='+val).join('\n');
  return '<form class="dialog-form" data-form="labels"><p class="mono">'+e(refTitle(key(a)))+'</p><div class="form-grid">'+field('标签 · 每行 key=value','<textarea class="ax-input" name="labels" rows="5" maxlength="300" autofocus>'+e(text)+'</textarea>',true)+'</div>'+
  '<input type="hidden" name="revision" value="'+revision+'"><div id="label-error" role="status"></div><div class="ax-dialog-actions">'+btn('取消','dismiss')+'<button class="ax-button" data-variant="primary" type="submit">保存标签</button></div></form>';
}

export function DirectoryForm(route,draft){
  return '<form data-form="directory-send" class="dialog-form"><dl class="fact-grid">'+dataPair('目录',route.path,true)+dataPair('预览端点',routeStates[route.state],false)+'</dl>'+
    notice('执行时会重新解析目录，可能采用或启动其他端点。查阅回执确认实际收件人。')+
    '<div class="form-grid">'+field('消息','<textarea class="ax-input" name="message" rows="3" required maxlength="500" autofocus>'+e(draft.message)+'</textarea>',true)+
    field('意图',select('intent',['request','inform','ack'].map(x=>option(x,x,draft.intent||'request')).join('')))+
    '</div><div class="ax-dialog-actions">'+btn('取消','dismiss')+'<button class="ax-button" data-variant="primary" type="submit">发送到目录</button></div></form>';
}
