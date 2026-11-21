/* Offline teaching UI. Never executes command text or contacts AgentX services. */
const NAV = [ ['overview','命令关系'], ['work','协作过程'], ['modes','入口与模式'], ['observe','诊断路径'], ['catalog','全部命令'] ];
const state = { view:'overview', node:'work', mode:'workbench-write', branch:'success', step:0, diagnostic:0, command:'work', query:'', category:'全部' };
const main = document.querySelector('#main');
const escapeHTML = value => String(value).replace(/[&<>"']/g, char => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
const badge = (label,tone='') => `<span class="badge ${tone}">${escapeHTML(label)}</span>`;
const sourceLinks = keys => `<div class="links">${keys.map(key => `<a href="${SOURCES[key][1]}">${SOURCES[key][0]} ↗</a>`).join('')}</div>`;
const commandBlock = command => `<div class="command-row"><pre class="command">${escapeHTML(command)}</pre><button class="copy" data-copy="${escapeHTML(command)}" aria-label="复制命令示例">复制</button></div>`;
const heading = (title,lede,badgeText='场景 → 对象 → 操作') => `<div class="page-heading"><div><h1>${title}</h1><p class="lede">${lede}</p></div>${badge(badgeText)}</div>`;
const panel = (title,content,decoration='') => `<section class="panel"><div class="panel-heading"><h2>${title}</h2>${decoration}</div><div class="panel-body">${content}</div></section>`;
const definition = rows => `<dl class="definition">${rows.map(([label,value])=>`<dt>${label}</dt><dd>${value}</dd>`).join('')}</dl>`;
function renderNavigation(){
  document.querySelector('#navigation').innerHTML = NAV.map(([id,label],i)=>`<a class="nav-link ${state.view===id?'active':''}" href="#${id}" ${state.view===id?'aria-current="page"':''}><span class="nav-index">0${i+1}</span>${label}</a>`).join('');
}
function svgNode(id,x,y,w,title,caption,active){
  return `<g class="map-node ${active?'active':''}" tabindex="0" role="button" aria-label="${escapeHTML(title+'：'+caption)}" data-node="${id}" transform="translate(${x} ${y})"><rect width="${w}" height="64"/><text x="16" y="27">${title}</text><text class="caption" x="16" y="47">${caption}</text></g>`;
}
function overviewMap(){
  const nodes = [
    ['launch',24,42,160,'launch / profile','选择工具与启动配方'],
    ['agent',240,42,160,'Agent / session','独立执行者与会话'],
    ['communication',456,42,170,'send / events','消息、活动与上下文'],
    ['work',240,178,160,'work','目标、分工、证据、接受'],
    ['workbench',24,178,160,'workbench','面向操作者的任务视图'],
    ['observe',456,178,170,'observe / web','服务、请求与诊断证据'],
    ['daemon',240,303,160,'daemon','作用域内的原生服务']
  ];
  const paths = [['launch','agent','M184 74H240'],['agent','communication','M400 74H456'],['agent','work','M320 106V178'],['workbench','work','M184 210H240'],['work','daemon','M320 242V303'],['communication','daemon','M541 106V278H400V303'],['observe','daemon','M541 242V335H400']];
  return `<svg class="map" viewBox="0 0 650 395" role="group" aria-label="命令责任关系图；可选择节点查看解释"><defs><marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M0 0L10 5L0 10Z" fill="currentColor"/></marker></defs>${paths.map(([from,to,path])=>`<path class="edge ${[from,to].includes(state.node)?'selected':''}" d="${path}" marker-end="url(#arrow)"/>`).join('')}<text class="map-label" x="25" y="25">创建与运行</text><text class="map-label" x="25" y="163">业务协作与诊断</text>${nodes.map(([id,x,y,w,title,caption])=>svgNode(id,x,y,w,title,caption,state.node===id)).join('')}</svg>`;
}
function nodeDetail(){
  const special = {
    agent:['Agent / session','执行者','真正运行的工具进程、身份和会话。一个 Agent 可以接收消息与角色分派，Work 则保留任务事实。','同一 Agent 的进程状态与具体任务的结果状态是两个维度。','agentx list','launch'],
    communication:['send / events','通信与活动','send 发送消息，listen 等待，events 查询或订阅业务活动，bundle 提供上下文，transcript 读取对话。','消息中说“完成”不会自动生成 WorkResult；普通 events 也不是 observe events。','agentx events --follow','router'],
    launch:['launch / profile','创建与配置','launch 新建 Agent，profile 选择保存的工具与原生参数。resume / fork 则从历史会话继续或分叉。','--count 创建多个执行者，并不自动绑定工作角色。','agentx launch --profile codex-deep --count 2 --headless','profile']
  };
  let values=special[state.node];
  if(!values){const row=COMMANDS[state.node];values=[state.node,row[0],row[1],row[3],row[4],state.node==='workbench'?'web':state.node];}
  const [title,category,purpose,boundary,example,source]=values;
  return `<h2>${title}</h2>${badge(category,'green')}<div class="detail-copy"><p>${purpose}</p><p class="subtle">${boundary}</p></div>${commandBlock(example)}<div class="focus-strip"><button data-jump="catalog" data-command="${state.node==='agent'?'list':state.node==='communication'?'send':state.node}">查看命令 →</button>${state.node==='work'||state.node==='workbench'?'<button data-jump="work">演示协作过程 →</button>':''}${state.node==='observe'||state.node==='daemon'?'<button data-jump="observe">跟随诊断路径 →</button>':''}</div>${sourceLinks([SOURCES[source]?source:'router'])}`;
}
function renderOverview(){
 main.innerHTML=heading('同一套 Agent，三个观察角度','先选你要操作的对象：执行者、任务结果，还是系统运行证据。')+`<div class="two-column"><section class="panel"><div class="panel-heading"><h2>命令责任地图</h2>${badge('点击节点展开')}</div>${overviewMap()}<div class="panel-body subtle">箭头表示创建、使用或观察关系；不是网络拓扑，也不表示所有命令都经过同一条执行路径。</div></section><section class="panel"><div class="panel-body" id="node-detail">${nodeDetail()}</div></section></div>`+`<section class="panel section-gap"><div class="panel-heading"><h2>你现在要做什么？</h2></div><div class="table-scroll"><table class="comparison"><thead><tr><th>问题</th><th>入口</th><th>真正操作的对象</th><th>下一步</th></tr></thead><tbody><tr><td>让一个 Agent 开始工作</td><td><code>launch / start</code></td><td>进程、身份、会话</td><td><a href="#catalog/launch">启动选项 →</a></td></tr><tr><td>把工作交出去并验收</td><td><code>work / workbench</code></td><td>目标、角色、结果、检查、决定</td><td><a href="#work">完整过程 →</a></td></tr><tr><td>为什么请求卡住或失败</td><td><code>observe / observe-web</code></td><td>服务、请求、配置、事件证据</td><td><a href="#observe">诊断路径 →</a></td></tr><tr><td>谁在忙，最近做了什么</td><td><code>list / events / transcript</code></td><td>参与者状态与协作活动</td><td><a href="#catalog/events">活动工具 →</a></td></tr></tbody></table></div></section>`;
}
function workDiagram(step){
 const complete=state.step>= (state.branch==='failure'?11:7);
 return `<svg class="map" viewBox="0 0 650 165" role="img" aria-label="操作方与两个独立角色围绕同一工作记录协作"><path class="edge selected" d="M169 80H240M410 80H480M325 57V23H565V57"/><text class="map-label" x="205" y="70">操作</text><text class="map-label" x="427" y="70">报告</text><text class="map-label" x="404" y="18">实现 / 检查独立</text>${svgStatic(15,48,154,'操作方','apply / 浏览器')}${svgStatic(240,48,170,'Work · CSV 导出',step.object)}${svgStatic(480,48,154,'实现者 / 检查者','report · 精确分派')}<text class="map-label" x="244" y="137">${complete?'明确接受已保留，工作关闭':'教学状态：'+escapeHTML(step.status)}</text></svg>`;
}
function svgStatic(x,y,w,title,caption){return `<g transform="translate(${x} ${y})" class="map-node active"><rect width="${w}" height="64"/><text x="12" y="27">${title}</text><text class="caption" x="12" y="47">${escapeHTML(caption)}</text></g>`;}
function renderWork(){
 const steps=state.branch==='failure'?FAILURE_STEPS:STEPS;
 state.step=Math.min(state.step,steps.length-1);
 const step=steps[state.step];
 const last=state.step===steps.length-1;
 const tone=step.status.includes('未通过')?'red':last?'green':'blue';
 main.innerHTML=heading('一次协作，怎样成为可接受的结果','沿同一个 CSV 导出任务看分派、回报、检查和接受。每一步可直接选择。','确定性教学演示')+
 `<div class="toolbar section-gap"><label for="branch">场景</label><select id="branch"><option value="success" ${state.branch==='success'?'selected':''}>正常检查通过</option><option value="failure" ${state.branch==='failure'?'selected':''}>检查失败 → 退回 → 修复</option></select><button data-action="previous" ${state.step===0?'disabled':''}>← 上一步</button><button data-action="next" ${last?'disabled':''}>下一步 →</button><button data-action="reset">重置</button><span class="subtle">${state.step+1} / ${steps.length}</span></div>`+
 `<div class="two-column section-gap"><section class="panel"><div class="panel-heading"><h2>工作记录 · CSV 导出</h2>${badge(step.status,tone)}</div>${workDiagram(step)}<div class="panel-body"><div class="facts"><div class="fact"><span class="fact-label">当前依据</span><span class="fact-value">${step.object}</span></div><div class="fact"><span class="fact-label">发布结果</span><span class="fact-value">${step.result}</span></div><div class="fact"><span class="fact-label">独立检查</span><span class="fact-value">${step.check}</span></div></div><div class="detail-copy"><h3>${state.step+1}. ${step.title}</h3><p>${step.text}</p></div><div class="callout ${step.status.includes('未通过')?'amber':''}">${step.meaning}</div><div class="step-command">${badge(step.actor)} <code>${step.operation}</code></div><details class="details"><summary>对应 CLI 与字段为何这么多</summary><p><code>work apply</code> 接受 OperatorCommand；<code>work report</code> 接受 AgentReport。页面操作也调用同一协作领域，浏览器只提供有限 Operator 操作。</p>${definition([['scope / project','找到数据空间和项目'],['generation','防止把旧数据库代际引用用于现在'],['revision','并发保护；不是 Git 版本'],['assignment / brief','报告的是哪份当前分派与任务依据'],['command-id','同一逻辑写入的持久身份；用于查询未知结果'],['source revision','产物的不可变源码身份；与 Work revision 分开']])}<a href="#catalog/work%20report">查看 report 语法与完整帮助 →</a></details>${sourceLinks(['service','model','work'])}</div></section><section class="panel"><div class="panel-heading"><h2>保留的过程</h2>${badge('可回看')}</div><div class="stage-list">${steps.map((s,i)=>`<button class="stage ${i===state.step?'active':i<state.step?'done':''}" data-step="${i}" aria-pressed="${i===state.step}"><span class="stage-number">${String(i+1).padStart(2,'0')}</span><span>${s.title}</span></button>`).join('')}</div></section></div>`+
 `<section class="panel section-gap"><div class="panel-heading"><h2>三种“完成”要分开</h2></div><div class="table-scroll"><table class="comparison"><thead><tr><th>你看到的信号</th><th>可以说明</th><th>还需要</th></tr></thead><tbody><tr><td>消息已入队</td><td>提交已进入消息路径</td><td>实际收到／角色报告；不能推断执行开始</td></tr><tr><td>Check · Passed</td><td>这份准确结果有检查报告</td><td>核对资格：角色、依据、源码、标准、未答问题</td></tr><tr><td>Accepted</td><td>操作方明确接受了所选结果、检查、brief</td><td>若需合并或发布，另走对应工程操作</td></tr></tbody></table></div></section>`;
}
function renderModes(){
 const mode=MODES.find(m=>m.id===state.mode);
 main.innerHTML=heading('只读，不等于不会启动服务','命令做什么、页面能改什么、启动时是否获取 daemon，是三个独立问题。')+
 `<div class="mode-buttons">${MODES.map(m=>`<button data-mode="${m.id}" aria-pressed="${m.id===state.mode}">${m.label}</button>`).join('')}</div>`+
 panel(mode.label,commandBlock(mode.command)+`<div class="mode-flow"><div class="flow-box"><strong>${mode.entry}</strong><span>当前调用</span></div><span class="flow-arrow">→</span><div class="flow-box"><strong>${mode.target}</strong><span>固定作用域</span></div><span class="flow-arrow">→</span><div class="flow-box"><strong>${mode.write.startsWith('否')?'只读':'有限操作'}</strong><span>原生领域契约</span></div></div><div class="mode-facts">${definition([['使用场景',mode.purpose],['写入能力',mode.write],['本地启动',mode.acquire]])}${definition([['路径选择',mode.scope],['退出之后',mode.stop],['页面资源','两个 Web 入口均支持嵌入资源；通常不需要 Node 或外部资源服务器。']])}</div>`+sourceLinks([mode.source]))+
 `<section class="panel section-gap"><div class="panel-heading"><h2>作用域与项目：两个不同坐标</h2>${badge('不是同一个目录概念')}</div><div class="panel-body"><div class="mode-flow"><div class="flow-box"><strong>scope</strong><span>数据库 / 配置 / 服务</span></div><span class="flow-arrow">⊃</span><div class="flow-box"><strong>project</strong><span>规范化项目目录</span></div><span class="flow-arrow">⊃</span><div class="flow-box"><strong>Work</strong><span>目标 / 角色 / 证据</span></div></div><p>scope 选择一套 AgentX 数据空间；project 在其中界定当前项目的协作记录与候选参与者。目录可位于不同位置，图示为逻辑归属，不是文件系统嵌套要求。</p><div class="callout section-gap">使用 <code>work</code> 前，把 <code>AGENTX_DIR</code> 设为要操作的已存在数据目录，或直接将示例中的 <code>"$AGENTX_DIR"</code> 替换为实际路径。当前 <code>work</code> 不继承 workbench 的省略参数默认行为。</div><details class="details"><summary>端口、LAN、退出和版本</summary><p>Web 默认监听 127.0.0.1、端口 0（系统选择可用端口）。可显式设 <code>--host</code> / <code>--port</code>，例如 <code>--host 0.0.0.0 --port 18973</code>。协作模式允许能访问该监听器的客户端执行有限写操作；没有独立登录流程。</p><p>Ctrl-C 结束 Web 适配器，不是 kill Agent。若原生服务构建不匹配，按实际诊断使用 daemon restart；工作台不会自行结束旧服务。</p><p>当前 .bin 帮助与开发构建存在 profile 差异；以所用二进制的 --help-all 确认语法，不只看版本号。</p></details>${sourceLinks(['web','daemon','env'])}</div></section>`;
}
function renderObserve(){
 const d=DIAGNOSTICS[state.diagnostic];
 main.innerHTML=heading('从“卡住了”走到可解释的证据','诊断路径帮助定位事实与未知；恢复动作由对应领域负责。','被动观察')+
 `<div class="two-column"><section class="panel"><div class="panel-heading"><h2>${d.title}</h2>${badge(`${state.diagnostic+1} / ${DIAGNOSTICS.length}`,'blue')}</div><div class="panel-body"><div class="callout">${d.fact}</div><div class="detail-copy"><p>${d.text}</p></div>${commandBlock(d.cmd)}<div class="detail-copy"><p class="subtle">${d.boundary}</p></div>${sourceLinks([d.source])}<div class="toolbar section-gap"><button data-diagnostic="${Math.max(0,state.diagnostic-1)}" ${state.diagnostic===0?'disabled':''}>← 上一步</button><button data-diagnostic="${Math.min(4,state.diagnostic+1)}" ${state.diagnostic===4?'disabled':''}>下一步 →</button></div></div></section><section class="panel"><div class="panel-heading"><h2>证据路径</h2></div><div class="stage-list">${DIAGNOSTICS.map((s,i)=>`<button class="stage ${i===state.diagnostic?'active':''}" data-diagnostic="${i}" aria-pressed="${i===state.diagnostic}"><span class="stage-number">0${i+1}</span>${s.title}</button>`).join('')}</div><div class="panel-body"><h3>两个事件流</h3>${definition([['events','消息、状态、生命周期与业务活动；面向协作与自动化'],['observe events','原生诊断窗口；保留缺口、代际与流边界']])}<a href="#catalog/observe%20events">查看窗口与参数 →</a></div></section></div>`+
 `<section class="panel section-gap"><div class="panel-heading"><h2>两个经常误读的结果</h2></div><div class="table-scroll"><table class="comparison"><thead><tr><th>看到的结果</th><th>准确解释</th><th>不能据此推断</th></tr></thead><tbody><tr><td>ReturnedOk</td><td>请求处理／返回过程的一个观察维度</td><td>不代表业务成功、调用方收到或目标执行完成</td></tr><tr><td>OutcomeUnknown</td><td>写操作的实际结果尚不能确定</td><td>不代表可以换一个 command ID 重新提交</td></tr><tr><td>单次诊断退出 0</td><td>诊断本身完成，可能明确返回缺失或缺口</td><td>不代表原业务已成功</td></tr></tbody></table></div></section>`;
}
function commandData(path){
 const base=path.split(' ')[0];
 const row=COMMANDS[base]||COMMANDS[''];
 const sub=SUBCOMMANDS[path];
 return {category:row[0],purpose:sub?sub[0]:row[1],scenario:sub?sub[1]:row[2],boundary:row[3],example:sub?sub[2]:path.includes(' ')?`agentx ${path} --help`:row[4]};
}
function renderCatalogList(){
 const query=state.query.trim().toLowerCase();
 const paths=Object.keys(HELP).filter(path=>{
 const d=commandData(path);
 return (state.category==='全部'||d.category===state.category)&&(!query||`${path} ${d.purpose} ${d.scenario} ${d.category} ${HELP[path]}`.toLowerCase().includes(query));
 });
 document.querySelector('#catalog-count').textContent=`${paths.length} / ${Object.keys(HELP).length}`;
 document.querySelector('#catalog-list').innerHTML=paths.map(path=>`<button class="catalog-item ${path.includes(' ')?'child':''} ${state.command===path?'active':''}" data-command="${escapeHTML(path)}" aria-pressed="${state.command===path}">${escapeHTML(path||'agentx（TUI / 帮助）')}</button>`).join('')||'<p class="panel-body subtle">没有匹配的命令。试试 work、检查或 --scope。</p>';
}
function renderCommandDetail(){
 const path=Object.hasOwn(HELP,state.command)?state.command:'work';
 const d=commandData(path);
 let special='';
 if(path==='work'||path.startsWith('work '))special=`<details class="details" open><summary>apply 和 report 的角色边界</summary><p><strong>Operator：</strong>CreateWork、UpdateBrief、BindParticipant、DispatchAssignment、SendNote、AnswerQuestion、RequestCheck、AcceptResult、RequestChanges。</p><p><strong>Agent：</strong>Progress、Question、Result、Check。报告要求当前角色与分派匹配；<code>--iam</code> 选择已有参与者声明，不是任意伪造新身份，也不单凭它证明某 Provider 是作者。</p><p>返回 Rejected 表示领域拒绝，退出 2；传输／获取失败退出 1；成功退出 0。参数为示意占位值，真实 ID、代际、版本、分派与 brief 必须从权威响应中读取。</p></details>`;
 if(path==='launch'||path.startsWith('profile'))special=`<details class="details" open><summary>启动选择与参数边界</summary><p>显式 profile 优先；显式 tool 屏蔽 AGENTX_PROFILE。无显式选择时，非空 AGENTX_PROFILE 优先于全局默认。tool 与 profile 同时指定必须匹配。<code>--replace-args</code> 整体替换继承参数，<code>--no-profile</code> 要求明确 tool 并忽略 profile 及旧默认参数。</p><p>新启动读取当前选择一次；resume / fork 使用历史参数。远端 profile 在目标设备解析。同一版本号的不同构建可能有不同语法，完整帮助快照来自当前开发构建。</p></details>`;
 if(path==='observe'||path.startsWith('observe '))special=`<details class="details" open><summary>引用、输出与退出</summary><p>引用文件为完整原生 JSON 对象，最多 32 KiB；拒绝未知字段。单次 --json 输出一个完整值，follow 输出 JSON Lines。窗口缺口不会自动被抹平。</p><p>退出 0：本次读取完成；1：范围、传输或原生拒绝等失败，或 follow 遇到已显示边界；2：参数／文件输入不合法；130：SIGINT。停止观察不取消原业务。</p></details>`;
 const base=path.split(' ')[0];
 const source=base==='work'?'work':base==='workbench'?'web':base.startsWith('observe')?'observe':base==='profile'||base==='launch'?'profile':base==='daemon'?'daemon':base==='config'?'env':'router';
 document.querySelector('#command-detail').innerHTML=`<div class="panel-heading"><h2>agentx${path?' '+escapeHTML(path):''}</h2>${badge(d.category,'green')}</div><div class="panel-body"><p>${d.purpose}</p>${definition([['适用场景',d.scenario],['重要边界',d.boundary]])}${commandBlock(d.example)}${special}<details class="details" ${['work','work report','workbench','observe'].includes(path)?'open':''}><summary>当前开发构建的完整帮助</summary><pre class="command help-text">${escapeHTML(HELP[path])}</pre></details>${sourceLinks([source])}</div>`;
}
function renderCatalog(){
 main.innerHTML=heading('全部命令，按意图查找','搜索命令、中文含义或参数；展开帮助查看精确语法。示例中的大写标识需要替换。',`${Object.keys(HELP).length} 个帮助节点`)+`<div class="toolbar"><label class="visually-hidden" for="query">搜索命令和参数</label><input id="query" type="search" placeholder="搜索 work、检查、--scope…" value="${escapeHTML(state.query)}"><label class="visually-hidden" for="category">命令类别</label><select id="category">${CATEGORIES.map(c=>`<option ${c===state.category?'selected':''}>${c}</option>`).join('')}</select><span class="subtle" id="catalog-count"></span></div><div class="catalog-layout section-gap"><section class="panel"><div class="catalog-list" id="catalog-list" aria-label="命令列表"></div></section><section class="panel" id="command-detail"></section></div>`;
 renderCatalogList();renderCommandDetail();
}
function route(){
 const [view,command]=location.hash.slice(1).split('/');
 state.view=NAV.some(([id])=>id===view)?view:'overview';
 if(command!==undefined){try{state.command=decodeURIComponent(command)}catch{state.command='work'}}
 renderNavigation();
 ({overview:renderOverview,work:renderWork,modes:renderModes,observe:renderObserve,catalog:renderCatalog})[state.view]();
 document.title=`AgentX · ${NAV.find(([id])=>id===state.view)[1]}`;
}
function selectNode(id){state.node=id;renderOverview();const selected=main.querySelector(`[data-node="${id}"]`);selected?.focus({preventScroll:true});}
main.addEventListener('click',async event=>{
 const target=event.target.closest('button,[data-node]');if(!target)return;
 if(target.dataset.copy!==undefined){
  try{await navigator.clipboard.writeText(target.dataset.copy);target.textContent='已复制';}
  catch{const pre=target.previousElementSibling;const range=document.createRange();range.selectNodeContents(pre);const selection=window.getSelection();selection.removeAllRanges();selection.addRange(range);target.textContent='已选中';}
  return;
 }
 if(target.dataset.jump){const cmd=target.dataset.command;location.hash=target.dataset.jump+(cmd?'/'+encodeURIComponent(cmd):'');return;}
 if(target.dataset.node){selectNode(target.dataset.node);return;}
 if(target.dataset.mode){state.mode=target.dataset.mode;renderModes();main.querySelector(`[data-mode="${state.mode}"]`)?.focus({preventScroll:true});return;}
 if(target.dataset.command!==undefined){state.command=target.dataset.command;history.replaceState(null,'','#catalog/'+encodeURIComponent(state.command));renderCatalogList();renderCommandDetail();main.querySelector('.catalog-item.active')?.focus({preventScroll:true});return;}
 if(target.dataset.diagnostic!==undefined){state.diagnostic=Number(target.dataset.diagnostic);renderObserve();main.querySelector(`.stage[data-diagnostic="${state.diagnostic}"]`)?.focus({preventScroll:true});return;}
 if(target.dataset.step!==undefined){state.step=Number(target.dataset.step);renderWork();main.querySelector(`[data-step="${state.step}"]`)?.focus({preventScroll:true});return;}
 if(target.dataset.action){const action=target.dataset.action;state.step=action==='reset'?0:state.step+(action==='next'?1:-1);renderWork();const control=main.querySelector(`[data-action="${action}"]`);if(control&&!control.disabled)control.focus({preventScroll:true});else main.querySelector(`[data-action="${action==='next'?'previous':'next'}"]`)?.focus({preventScroll:true});}
});
main.addEventListener('keydown',event=>{const node=event.target.closest('[data-node]');if(node&&(event.key==='Enter'||event.key===' ')){event.preventDefault();selectNode(node.dataset.node);}});
main.addEventListener('input',event=>{if(event.target.id==='query'){state.query=event.target.value;renderCatalogList();}});
main.addEventListener('change',event=>{
 if(event.target.id==='branch'){state.branch=event.target.value;state.step=0;renderWork();main.querySelector('#branch').focus({preventScroll:true});}
 if(event.target.id==='category'){state.category=event.target.value;renderCatalogList();}
});
window.addEventListener('hashchange',route);
route();
