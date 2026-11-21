import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(await fs.readFile(path.join(root, 'design.json'), 'utf8'));
const palettes = {
  light: { bg:'#F3F6FF', paper:'#FFFFFF', soft:'#F6F8FF', ink:'#19254B', muted:'#62708D', line:'#D9E0F0', edge:'#26355A', accent:'#315BFF', blue:'#315BFF', blueTint:'#E8EDFF', green:'#008563', greenTint:'#D7FFF0', orange:'#C94B0A', orangeTint:'#FFF0DD', yellow:'#FFC938', red:'#D53050', redTint:'#FFE5EC', violet:'#713CE0', violetTint:'#F0E8FF', pink:'#DB248E', cyan:'#007F98', white:'#FFFFFF', shadow:'#CFD8F4' },
  dark: { bg:'#10162B', paper:'#18213B', soft:'#202B48', ink:'#F1F5FF', muted:'#9AAACB', line:'#354464', edge:'#778DB4', accent:'#7096FF', blue:'#7096FF', blueTint:'#263C71', green:'#43E4AD', greenTint:'#15493F', orange:'#FFB349', orangeTint:'#573A24', yellow:'#FFD64A', red:'#FF738F', redTint:'#53293D', violet:'#BB95FF', violetTint:'#3A2C60', pink:'#FF7ED0', cyan:'#4CD9F0', white:'#FFFFFF', shadow:'#070C19' }
};
const esc = x => String(x).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
palettes.light.pinkTint='#FFE5F5'; palettes.light.cyanTint='#DDF8FF';
palettes.dark.pinkTint='#522442'; palettes.dark.cyanTint='#164552';
palettes.light.onAccent='#FFFFFF'; palettes.dark.onAccent='#10162B';
const v = key => `var(--${key})`;
const rect = (x,y,w,h,fill='paper',extra='') => `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${fill.startsWith('#') || fill==='none' ? fill : v(fill)}" ${extra}/>`;
const line = (x1,y1,x2,y2,key='line',width=1) => `<path d="M${x1} ${y1}L${x2} ${y2}" fill="none" stroke="${v(key)}" stroke-width="${width}"/>`;
const text = (x,y,value,size=13,key='ink',weight=400,extra='') => `<text x="${x}" y="${y}" font-size="${size}" font-weight="${weight}" fill="${v(key)}" ${extra}>${esc(value)}</text>`;
const group = (id,body) => `<g data-dc-id="${id}" id="${id}">${body}</g>`;
function stepPath(x,y,w,h,c=3){ return `M${x+c} ${y}H${x+w-c}V${y+c}H${x+w}V${y+h-c}H${x+w-c}V${y+h}H${x+c}V${y+h-c}H${x}V${y+c}H${x+c}Z`; }
function panel(x,y,w,h,{fill='paper',stroke='line',shadow=false,cut=4}={}){
  return `${shadow?`<path d="${stepPath(x+3,y+3,w,h,cut)}" fill="${v('shadow')}"/>`:''}<path d="${stepPath(x,y,w,h,cut)}" fill="${v(fill)}" stroke="${v(stroke)}" stroke-width="1.5"/>`;
}
const icons = {
  chat:'M2 2h11v8H7v3H5v-3H2V2zm2 2v4h7V4H4zm1 1h5v1H5V5z',
  recovery:'M3 1h2v3h7v2h2v6h-2v2H5v-2H3v-2h2v2h7V6H5v3H3V7H1V5h2V1z',
  grid:'M1 1h6v6H1V1zm2 2v2h2V3H3zm6-2h6v6H9V1zm2 2v2h2V3h-2zM1 9h6v6H1V9zm2 2v2h2v-2H3zm6-2h6v6H9V9zm2 2v2h2v-2h-2z',
  token:'M5 1h6v2h2v2h2v6h-2v2h-2v2H5v-2H3v-2H1V5h2V3h2V1zm0 4v6h2V9h2v2h2V5H9v2H7V5H5z',
  chart:'M1 2h2v11h12v2H1V2zm4 5h2v4H5V7zm4-4h2v8H9V3zm4 2h2v6h-2V5z',
  compare:'M1 2h4v2h2v2h2V4h2V2h4v2h-4v2H9v4h2v2h4v2h-4v-2H9v-2H7v2H5v2H1v-2h4v-2h2V6H5V4H1V2z',
  pulse:'M0 8h4l2-6h2l3 10 1-4h4v2h-3l-1 5h-2L7 5 6 10H0V8z',
  sparkle:'M7 0h2v5h2v2h5v2h-5v2H9v5H7v-5H5V9H0V7h5V5h2V0zm0 7v2h2V7H7z',
  search:'M3 1h6v2H3V1zM1 3h2v6H1V3zm8 0h2v6H9V3zM3 9h6v2H3V9zm6 2h2v2h2v2h-2v-2H9v-2z',
  chevron:'M3 5h2v2h2v2h2V7h2V5h2v2h-2v2H9v2H7V9H5V7H3V5z',
  arrow:'M3 3h10v10h-2V7H9V5H3V3zm4 4h2v2H7v2H5v2H3v-2h2V9h2V7z',
  folder:'M1 3h5v2h9v9H1V3zm2 4v5h10V7H3z',
  layers:'M7 1h2v2h3v2h3v2h-3v2H9v2H7V9H4V7H1V5h3V3h3V1zM1 9h2v2h4v2h2v-2h4V9h2v2h-2v2H9v2H7v-2H3v-2H1V9z',
  clock:'M5 1h6v2H5V1zM3 3h2v2H3V3zM1 5h2v6H1V5zm2 6h2v2H3v-2zm2 2h6v2H5v-2zm6-2h2v2h-2v-2zm2-6h2v6h-2V5zm-2-2h2v2h-2V3zM7 4h2v4h3v2H7V4z',
  check:'M1 8h2v2h2v2h2v-2h2V8h2V6h2V4h2v3h-2v2h-2v2H9v2H7v2H5v-2H3v-2H1V8z',
  warning:'M7 1h2v2h2v3h2v3h2v5H1V9h2V6h2V3h2V1zm0 5v4h2V6H7zm0 5v2h2v-2H7z',
  info:'M5 1h6v2H5V1zM1 5h2v6H1V5zm12 0h2v6h-2V5zM3 3h2v2H3V3zm8 0h2v2h-2V3zM3 11h2v2H3v-2zm8 0h2v2h-2v-2zm-6 2h6v2H5v-2zm2-8h2v2H7V5zm0 3h2v4H7V8z',
  filter:'M1 2h14v2H1V2zm3 5h8v2H4V7zm3 5h2v2H7v-2z',
  more:'M1 7h3v3H1V7zm6 0h3v3H7V7zm6 0h3v3h-3V7z',
  sun:'M6 0h3v3H6V0zm0 12h3v3H6v-3zM0 6h3v3H0V6zm12 0h3v3h-3V6zM4 4h7v7H4V4zM1 1h2v2H1V1zm11 0h2v2h-2V1zM1 12h2v2H1v-2zm11 0h2v2h-2v-2z',
  bolt:'M7 0h6L9 6h5L5 16l2-7H2L7 0z',
  settings:'M3 1h2v4h10v2H5v8H3V7H1V5h2V1zm8 8h2v2h2v2h-2v2h-2v-2H1v-2h10V9z',
  play:'M4 1h2v2h3v2h3v2h3v2h-3v2H9v2H6v2H4V1z',
  terminal:'M1 2h14v12H1V2zm2 2v8h10V4H3zm1 1h2v1h2v2H6v1H4V7h2V6H4V5zm5 4h3v2H9V9z',
  split:'M1 2h14v12H1V2zm2 2v8h4V4H3zm6 0v8h4V4H9z',
  document:'M2 1h8v2h3v2h2v10H2V1zm2 2v10h9V7H9V3H4zm1 4h3v2H5V7zm0 3h6v2H5v-2z',
  shield:'M1 2h4V0h6v2h4v8h-2v2h-2v2H9v2H7v-2H5v-2H3v-2H1V2zm4 5v2h2v2h2V9h2V7H9v2H7V7H5z',
  bell:'M5 1h6v2h2v7h2v2H1v-2h2V3h2V1zm0 3v6h6V4H5zm1 9h4v2H6v-2z'
};
function icon(name,x,y,key='muted',size=16){return `<svg x="${x}" y="${y}" width="${size}" height="${size}" viewBox="0 0 16 16" aria-hidden="true"><path d="${icons[name]||icons.grid}" fill="${v(key)}" shape-rendering="crispEdges"/></svg>`;}
function chip(x,y,w,label,{key='blue',selected=false,symbol=null,h=28,disabled=false}={}){
  const fill=selected?key:`${key}Tint`;
  return `<g opacity="${disabled?.45:1}">${selected?`<path d="${stepPath(x+2,y+2,w,h,3)}" fill="${v('shadow')}"/>`:''}<path d="${stepPath(x,y,w,h,3)}" fill="${v(fill)}" stroke="${v(key)}" stroke-width="${selected?1.8:1}"/>${symbol?icon(symbol,x+9,y+(h-14)/2,selected?'onAccent':key,14):''}${text(x+(symbol?29:11),y+h/2+4.5,label,12,selected?'onAccent':key,selected?650:500)}</g>`;
}
function squareButton(x,y,name,key='muted',selected=false){return `${panel(x,y,30,30,{fill:selected?'blueTint':'paper',stroke:selected?'blue':'line',cut:2})}${icon(name,x+7,y+7,key)}`;}
function checkbox(x,y,checked=false){return `${rect(x,y,16,16,checked?'accent':'paper',`stroke="${v(checked?'accent':'muted')}" stroke-width="1.4"`)}${checked?icon('check',x+1,y+1,'white',13):''}`;}

const tabs=[['chat','会话','blue',94],['recovery','恢复','orange',68],['grid','看板','pink',68],['token','Token','orange',77],['chart','明细','violet',68],['compare','横评','cyan',68],['pulse','事件流','orange',81],['sparkle','智能分析','green',94]];
function header(active='恢复',mobile=false){
  const w=mobile?390:1440;
  let out=rect(0,0,w,102,'paper')+line(0,48,w,48)+line(0,101,w,101);
  if(mobile){out+=text(16,29,'Agent Inspector',14,'ink',750)+text(144,29,'Paseo…',11,'muted')+chip(200,10,70,'Inspector',{key:'blue',h:28})+chip(278,10,47,'Bots',{key:'violet',h:28})+icon('sun',344,17);}
  else out+=icon('terminal',24,15,'accent',20)+text(54,29,'Agent Inspector',14,'ink',750)+text(180,29,'Paseo · 观测模式',11,'muted')+chip(1249,10,73,'Inspector',{key:'blue',h:28})+chip(1330,10,46,'Bots',{key:'violet',h:28})+icon('sun',1392,17);
  let x=14;
  tabs.forEach(([name,label,key,width])=>{
    const selected=label===active;
    const tw=mobile?(selected?68:32):width;
    if(selected)out+=`<path d="${stepPath(x+2,62,tw,30,3)}" fill="${v('shadow')}"/><path d="${stepPath(x,59,tw,30,3)}" fill="${v(key==='pink'?'pink':key==='blue'?'accent':key)}" stroke="${v('edge')}" stroke-width="1.5"/>`;
    out+=icon(name,x+9,66,selected?'onAccent':key,14);
    if(!mobile||selected)out+=text(x+29,79,label,13,selected?'onAccent':'muted',selected?650:500);
    if(label==='会话'&&!mobile)out+=text(x+65,78,'347',11,selected?'onAccent':'muted',600);
    x+=tw+4;
  });
  if(!mobile)out+=rect(1353,72,6,6,'green')+text(1365,79,'实时',11,'muted')+icon('more',1404,68);
  else out+=rect(333,72,5,5,'green')+icon('more',355,68);
  return group('inspector-existing-header',out);
}
const error='We’re currently experiencing high demand, which may cause temporary errors.';
function recoveryToolbar(mobile=false){
  const w=mobile?390:1440;
  let out=rect(0,102,w,172,'paper')+text(12,135,'自动恢复中',12,'muted');
  out+=icon('recovery',mobile?240:1284,120,'muted',16)+chip(mobile?269:1313,116,79,'自动开',{key:'green',symbol:'bolt'})+icon('settings',mobile?360:1404,122);
  out+=chip(12,156,54,'28',{key:'blue',symbol:'layers',selected:true})+chip(72,156,62,'403',{key:'violet',symbol:'clock'})+icon('filter',148,162);
  out+=panel(mobile?302:1344,154,30,30,{cut:2})+checkbox(mobile?309:1351,161)+chip(mobile?338:1381,156,47,'0',{key:'green',symbol:'play',disabled:true});
  out+=chip(12,194,47,'0',{key:'orange',symbol:'clock'})+chip(64,194,47,'0',{key:'blue',symbol:'pulse'})+chip(116,194,54,'24',{key:'green',symbol:'check'})+chip(176,194,54,'10',{key:'red',symbol:'warning'})+text(237,212,'5m → 20m → 1h',11,'muted');
  out+=panel(12,230,w-60,32,{fill:'soft',cut:2})+text(21,251,'搜索 Agent / 报错',12,'muted')+icon('info',w-35,239,'muted',14)+line(0,274,w,274);
  return group('recovery-original-toolbar',out);
}
function projectRow(y,name,count,mobile=false){const w=mobile?390:1440;return group(`project-${y}`,rect(0,y,w,43,'soft')+icon('chevron',22,y+15,'muted',12)+icon('folder',48,y+14,'accent',14)+text(67,y+26,name,13,'ink',650)+text(70+name.length*7.5,y+26,count,11,'muted'));}
function workspaceRow(y,label,count,mobile=false){return group(`workspace-${y}`,icon('chevron',38,y+13,'muted',12)+icon('layers',64,y+12,'violet',14)+text(84,y+24,label,12,'muted')+text(mobile?371:84+label.length*6.8,y+24,count,11,'muted'));}
function recoveryAgent(y,title,{status='待核实',provider='codex · 12:55 PM',mobile=false}={}){
  const w=mobile?390:1440;
  const body=checkbox(20,y+18)+text(48,y+25,title,13,'ink',650)+text(48,y+44,provider,11,'muted')+icon('warning',mobile?296:1340,y+22,'red',12)+text(mobile?311:1357,y+32,status,11,'red')+icon('chevron',w-34,y+23,'muted',12)+icon('recovery',12,y+63,'orange',12)+text(29,y+73,mobile?'We’re currently experiencing high demand…':error,12,'muted')+text(w-62,y+73,'#1',11,'muted')+icon('arrow',w-34,y+62,'muted',12)+line(12,y+91,w-12,y+91);
  return group(`agent-${y}`,body);
}
function footer(w=1440,h=1000){return group('original-footer',rect(0,h-26,w,26,'paper')+line(0,h-26,w,h-26)+icon('check',12,h-18,'green',12)+text(29,h-8,'347 · 11:54 AM',11,'muted')+text(w-122,h-8,'60s · 当前主机',11,'muted'));}
function recovery(mobile=false){
  let out=header('恢复',mobile)+recoveryToolbar(mobile);
  out+=projectRow(282,'Agent Inspector','1',mobile)+workspaceRow(325,mobile?'Design and implement Paseo bulk agent…':'Design and implement Paseo bulk agent recovery plugin','1',mobile);
  out+=recoveryAgent(358,mobile?'工作台体验领域负责人':'Agent Inspector · 工作台体验领域负责人',{mobile});
  out+=projectRow(459,'paseo-bots','4',mobile)+workspaceRow(502,mobile?'调研 Paseo SDK 创建 Bot 时的工具暴露…':'调研 Paseo SDK 创建 Bot 时的工具暴露控制能力','4',mobile);
  ['bots · 插件内核与服务端契约','bots · 客户端插件与场景扩展','bots 统一接入 · 旧插件契约与应用桥','bots 独立化 · 工程、构建与真实场景验收'].forEach((title,i)=>{out+=recoveryAgent(535+i*89,title,{mobile});});
  if(!mobile)out+=projectRow(901,'hypit','1')+workspaceRow(944,'梳理 docs/planning/ai-video-platform-requirements.zh-CN.md 的 CLI 与 API 全景','1');
  return out+footer(mobile?390:1440,mobile?844:1000);
}

function sessionItem(y,title,{key='blue',subtitle='codex · gpt-6.1',depth=0,selected=false,badge=null,meta=null}={}){
  const x=12+depth*24,w=338-depth*24;
  return group(`session-${y}`,`${selected?panel(x,y,w,90,{fill:'violetTint',stroke:'violet',cut:2}):''}${rect(x,y+7,3,68,key)}${icon(key==='violet'?'info':'pulse',x+11,y+13,key,14)}${text(x+34,y+26,title,13,'ink',650)}${meta?chip(x+34,y+34,Math.min(238,w-40),meta,{key,h:20,symbol:'pulse'}):''}${text(x+34,y+(meta?72:49),subtitle,11,'muted')}${badge?chip(x+w-53,y+11,45,badge,{key:'violet',h:20}):''}`);
}
function sessions(){
  let out=header('会话');
  out+=group('session-filters',rect(0,102,1440,44,'paper')+chip(12,110,105,'需要处理 8',{key:'orange',symbol:'bell'})+chip(125,110,96,'执行中 10',{key:'blue',symbol:'pulse'})+chip(229,110,96,'未读 244',{key:'pink',symbol:'chat'})+chip(333,110,89,'待回复 1',{key:'violet',symbol:'chat'})+chip(431,110,91,'全部项目',{key:'blue',symbol:'folder'})+chip(531,110,91,'最近 7 天',{key:'blue',symbol:'clock'})+chip(631,110,57,'筛选',{key:'violet'})+chip(697,110,65,'图例',{key:'violet'}));
  out+=rect(0,146,360,854,'paper')+`<path d="M359 146V1000" stroke="${v('line')}" stroke-width="2"/>`;
  out+=icon('bell',16,160,'orange',12)+text(34,171,'需要处理 1',11,'orange',650);
  out+=sessionItem(180,'整体设计与实现负责人',{key:'violet',selected:true,badge:'提问',subtitle:'hypit · main · 6 分 · codex',meta:'等待回复 · 构建环境容量方案'});
  out+=text(42,290,'⌄ 9 个子 Agent · 1 需处理 · 9 执行中',11,'muted');
  out+=sessionItem(305,'D6主控｜任务连续性与可信复用',{subtitle:'codex · gpt-6.1',meta:'后台 · D6 任务持久状态'});
  out+=text(63,398,'⌄ 1 个子 Agent · 1 执行中',11,'muted');
  out+=sessionItem(414,'D6 任务持久状态与可信复用',{depth:1});
  out+=sessionItem(503,'D5主控｜素材采用与参考改编',{meta:'后台 · 资源准入、采用与交接'});
  out+=text(63,597,'⌄ 1 个子 Agent · 1 执行中',11,'muted');
  out+=sessionItem(613,'D5 素材采用与交接设计',{depth:1});
  out+=sessionItem(702,'D1主控｜真实 Plan 与 React 主线',{meta:'后台 · 2 个子任务'});
  out+=text(63,795,'⌄ 2 个子 Agent · 2 执行中',11,'muted');
  out+=sessionItem(813,'D1 真实执行｜正式客户端',{depth:1});
  out+=sessionItem(899,'D1 React 基础与产品装配',{depth:1});
  out+=group('original-empty-detail',panel(879,523,42,42,{fill:'blueTint',stroke:'blue',cut:4})+icon('split',890,534,'blue',20)+text(900,594,'选择一个会话',14,'ink',500,'text-anchor="middle"')+text(900,620,'结果、时间线和子任务会显示在这里',12,'muted',400,'text-anchor="middle"'));
  out+=group('original-session-search',panel(1104,59,239,32,{fill:'soft',cut:3})+icon('search',1116,67,'muted',13)+text(1137,80,'搜索任务、项目或分支',12,'muted'));
  return out;
}

function metricCard(x,y,w,title,value,note,key,symbol,bars){
  let out=panel(x,y,w,148,{fill:'paper',stroke:'line',shadow:false,cut:5})+rect(x+2,y+6,4,136,key)+rect(x+14,y+14,24,24,`${key}Tint`)+icon(symbol,x+18,y+18,key,16)+text(x+47,y+31,title,13,'muted',650)+text(x+14,y+72,value,25,'ink',750)+text(x+14,y+101,note,11,'muted');
  bars.forEach((height,i)=>{out+=rect(x+14+i*19,y+134-height,16,Math.max(2,height),height>3?key:'line');});
  return group(`metric-${title}`,out);
}
function dashboard(){
  let out=header('看板');
  out+=group('dashboard-scope',rect(130,115,143,28,'soft')+text(140,134,'7 天',12,'muted')+chip(177,117,44,'30 天',{key:'blue',h:25,selected:true})+text(232,134,'90 天',12,'muted')+text(130,164,'历史统计 · 全主机',12,'muted')+icon('check',238,152,'green',13)+text(257,164,'目录已同步',12,'green')+text(344,164,'保留范围内的统计　前期覆盖不足，暂不显示增减',12,'muted')+text(1210,134,'更新于 刚刚',11,'muted')+icon('info',1291,122,'muted',12));
  let hero=panel(130,184,1180,180,{fill:'accent',stroke:'edge',shadow:false,cut:7})+rect(1275,188,28,28,'#FFC938')+rect(1251,216,28,28,'#FF7BAA')+rect(1275,244,28,28,'#43E4AD')+text(146,217,'中午好，今天已派出 44 个 Agent',13,'white',650)+text(146,249,'1,107 轮对话 · 145 小时',27,'white',750)+text(146,276,'近 30 天 488 个会话 · 1,107 轮',12,'white')+text(146,305,'实时 · 全主机',12,'white');
  const heroChip=(x,width,label)=>`<path d="${stepPath(x,320,width,28,3)}" fill="#FFFFFF" fill-opacity=".15" stroke="#FFFFFF" stroke-opacity=".4"/>${text(x+12,339,label,12,'white',600)}`;
  hero+=heroChip(146,99,'执行中 10')+heroChip(253,92,'待审批 0')+heroChip(353,98,'未读 244')+heroChip(459,98,'待回复 1')+text(1219,224,'7',28,'white',750)+text(1194,249,'连续天数',12,'white')+text(1192,273,'Lv.5 · 12/18',11,'white',650);
  out+=group('existing-dashboard-summary',hero);
  const data=[['会话','488','其中子 Agent 420','blue','chat',[1,1,1,1,1,1,13,16,9,14,23,8,5]],['对话轮次','1,107','每会话 2.3 轮','violet','recovery',[1,1,1,1,1,1,1,1,1,1,1,1,28]],['成功率','95.5%','失败 50 · 取消 0','green','check',[1,1,1,1,1,1,1,1,1,1,1,1,25]],['工作时长','145 小时','平均每轮 8 分','red','clock',[1,1,1,1,1,1,1,1,1,1,1,1,28]],['并行时长','9.8 小时','','orange','layers',[1,1,1,1,1,1,1,1,1,1,1,1,20]],['工具调用','1.1万','失败率 4.0%','cyan','settings',[1,1,1,1,1,1,1,1,1,1,1,1,25]],['代码行','+2.7万','−2,711 · 1,293 个文件','green','document',[1,1,1,1,1,1,1,1,1,1,1,1,28]],['审批等待','9 分','4 次审批','orange','shield',[1,1,1,1,1,1,1,1,1,1,1,1,26]]];
  data.forEach((d,i)=>out+=metricCard(130+(i%4)*298,380+Math.floor(i/4)*159,286,...d));
  let heat=panel(130,706,716,294,{cut:5})+rect(134,707,708,3,'green')+icon('grid',147,729,'green',18)+text(177,746,'年度活跃',16,'ink',700)+chip(739,726,44,'会话',{key:'green',h:25})+text(793,744,'轮次',12,'muted')+text(147,781,'488 个会话',14,'ink',600)+text(247,781,'保留范围内的统计',12,'muted')+text(147,807,'2/9',11,'muted')+text(793,807,'11/13',11,'muted');
  for(let col=0;col<40;col++)for(let row=0;row<7;row++){
    const x=148+col*17,y=819+row*17;
    if(col<38)heat+=rect(x,y,11,11,'soft',`stroke="${v('muted')}" stroke-width=".65" stroke-dasharray="2 2"`);
    else heat+=rect(x,y,11,11,col===39&&row<4?'green':'greenTint');
  }
  heat+=text(147,958,'虚线格表示覆盖未知；已记录值不代表全天完整。',12,'muted')+text(147,986,'点格子看当天',12,'muted');
  out+=group('original-annual-activity',heat);
  let badges=panel(862,706,448,214,{cut:5})+rect(866,707,440,3,'orange')+icon('shield',881,730,'orange',18)+text(910,746,'成长档案',16,'ink',700)+text(1214,745,'全部徽章 ↗',12,'orange',600)+chip(880,763,56,'Lv.5',{key:'orange',selected:true})+rect(952,765,340,8,'line')+rect(952,765,76,8,'orange')+text(952,791,'已点亮 12 / 18 枚 · 每 3 枚升一级',11,'muted')+text(880,819,'长期累计从本次观测边界起算；明细只代表实际保留范围。',11,'muted');
  [['第一次点灯','orange',100],['今日开工','red',88],['三日连线','orange',88],['七日连线','pink',88],['千轮对话','blue',88],['多线程','violet',76],['并发狂魔','violet',90]].forEach(([label,key,width],i)=>{const xs=[880,988,1084,880,976,1072,1156],ys=[838,838,838,875,875,875,875];badges+=chip(xs[i],ys[i],width,label,{key,selected:true,h:28});});
  return out+group('original-growth-profile',badges);
}

function swatch(x,y,key,name,hex){return group(`swatch-${key}`,panel(x,y,180,118,{fill:'paper',cut:4})+rect(x+8,y+8,164,49,key)+text(x+12,y+81,name,13,'ink',650)+text(x+12,y+102,hex,11,'muted',500,'font-family="monospace"'));}
function visualSystem(){
  let out=rect(0,0,1440,1000,'bg')+text(48,60,'鲜彩像素',34,'ink',750)+text(48,89,'沿用现有 Inspector 的结构与操作，调整视觉表达。',14,'muted')+text(1392,57,'VISUAL STUDY / 02',11,'muted',600,'text-anchor="end" letter-spacing="2"');
  out+=text(48,141,'01 / 色彩',12,'muted',650);
  const colors=[['accent','宝蓝 · 主操作','#315BFF'],['green','青绿 · 完成 / 自动','#008563'],['yellow','亮黄 · 注意','#FFC938'],['red','珊瑚 · 异常','#D53050'],['violet','紫色 · 未知 / 分析','#713CE0'],['ink','深蓝 · 文字 / 轮廓','#19254B']];
  colors.forEach(([key,name,hex],i)=>out+=swatch(48+i*224,159,key,name,hex));
  out+=text(48,325,'02 / 同样的控件，明确的像素轮廓',12,'muted',650);
  let controls=panel(48,344,832,248,{cut:5})+text(72,376,'操作',12,'muted')+chip(72,394,137,'继续原 Agent',{key:'blue',selected:true,symbol:'play',h:34})+chip(223,394,112,'刷新观察',{key:'blue',symbol:'recovery',h:34})+chip(349,394,112,'继续 0 个',{key:'blue',symbol:'play',disabled:true,h:34})+squareButton(482,396,'settings')+squareButton(526,396,'filter')+squareButton(570,396,'more')+text(72,466,'状态',12,'muted')+chip(72,483,101,'等待重试',{key:'orange',symbol:'clock'})+chip(185,483,89,'核验中',{key:'blue',symbol:'pulse'})+chip(286,483,89,'已恢复',{key:'green',symbol:'check'})+chip(387,483,89,'待核实',{key:'red',symbol:'warning'})+chip(488,483,113,'投递未知',{key:'violet',symbol:'info'})+text(72,561,'填充与轮廓区分操作层级；状态同时保留文字与符号。',13,'muted');
  out+=group('pixel-control-examples',controls);
  let grammar=panel(904,344,488,248,{cut:5})+text(928,379,'像素感来自细节',18,'ink',700)+icon('terminal',929,408,'blue',32)+icon('folder',985,408,'orange',32)+icon('grid',1041,408,'pink',32)+icon('shield',1097,408,'green',32)+icon('recovery',1153,408,'blue',32)+icon('sparkle',1209,408,'violet',32)+text(929,480,'16 × 16 整数网格图标',14,'ink',600)+text(929,511,'阶梯转角 · 2px 控件轮廓 · 2px 硬阴影',13,'muted')+text(929,543,'中文保留正常字形，数字清楚、界面紧凑。',13,'muted');
  out+=group('pixel-grammar',grammar);
  out+=text(48,637,'03 / 像素换肤的范围',12,'muted',650);
  let compare=panel(48,655,650,278,{cut:5})+text(72,693,'保留结构与功能',20,'ink',700)+text(72,727,'会话 · 恢复 · 看板 · Token · 明细 · 横评 · 事件流 · 智能分析',13,'muted')+line(72,748,674,748)+icon('check',74,771,'green',16)+text(102,785,'原有页签、页面顺序和各页布局',15,'ink',600)+icon('check',74,816,'green',16)+text(102,830,'项目 / Workspace / Agent 分组与展开',15,'ink',600)+icon('check',74,861,'green',16)+text(102,875,'原有按钮、筛选、详情、确认和结果反馈',15,'ink',600);
  out+=group('preserved-structure',compare);
  let night=panel(722,655,670,278,{fill:'ink',stroke:'edge',cut:5})+text(748,693,'同一结构，也可以有深色版',20,'white',700)+text(748,725,'深蓝底色 + 更明亮的强调色',13,'white');
  [['#7096FF',748],['#43E4AD',849],['#FFD64A',950],['#FF738F',1051],['#BB95FF',1152],['#4CD9F0',1253]].forEach(([color,x])=>night+=rect(x,751,78,34,color));
  night+=`<path d="${stepPath(748,813,185,38,3)}" fill="#7096FF" stroke="#9BB5FF" stroke-width="2"/>${text(768,838,'▶ 继续原 Agent',14,'ink',650)}<path d="${stepPath(957,813,155,38,3)}" fill="#15493F" stroke="#43E4AD" stroke-width="1.5"/><text x="974" y="838" fill="#43E4AD" font-size="14">✓ 已观察完成</text>`+text(748,895,'明亮与深色两张恢复画板使用完全相同的构图。',13,'white');
  return out+group('dark-palette-study',night)+text(48,973,'本轮交付：SVG 视觉候选。示例数据固定；未替换产品组件或连接业务操作。',12,'muted');
}

const pages={'recovery-light':()=>recovery(),'recovery-dark':()=>recovery(),'sessions-light':sessions,'dashboard-light':dashboard,'visual-system':visualSystem,'recovery-mobile':()=>recovery(true)};
const output=path.join(root,'svg');await fs.mkdir(output,{recursive:true});
let previous={};try{previous=JSON.parse(await fs.readFile(path.join(root,'.generated.json'),'utf8'));}catch{}
const generated={};
const {createHash}=await import('node:crypto');
const hash=s=>createHash('sha256').update(s).digest('hex');
async function writeGenerated(file,contents){
  const target=path.join(root,file);
  if(previous[file]){const existing=await fs.readFile(target,'utf8').catch(()=>null);if(existing&&hash(existing)!==previous[file])throw new Error(`用户编辑尚未合并，停止覆盖 ${file}`);}
  await fs.writeFile(target,contents);generated[file]=hash(contents);
}
for(const board of manifest.artboards){
  const theme=board.id==='recovery-dark'?'dark':'light';
  const vars=Object.entries(palettes[theme]).map(([key,value])=>`--${key}:${value}`).join(';');
  const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="${board.width}" height="${board.height}" viewBox="0 0 ${board.width} ${board.height}" role="img" aria-labelledby="title-${board.id} desc-${board.id}"><title id="title-${board.id}">${esc(board.title)}</title><desc id="desc-${board.id}">沿用原 Inspector 的结构与操作。可编辑 SVG 视觉候选，固定样例，无业务交互。</desc><style>:root{${vars}}text{font-family:Inter,"Noto Sans CJK SC","Microsoft YaHei",sans-serif;font-variant-numeric:tabular-nums}svg{background:var(--bg)}</style>${rect(0,0,board.width,board.height,'bg')}${pages[board.id]()}</svg>`;
  await writeGenerated(`svg/${board.id}.svg`,svg);
  const inlineSvg=svg.replace(`:root{${vars}}`,'');
  const html=`<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${esc(board.title)}</title><style>:root{${vars}}*{box-sizing:border-box}html,body{margin:0;background:var(--bg)}body>svg{display:block;width:100%;height:auto}text{user-select:text}</style><link rel="stylesheet" href="tweaks.css"></head><body>${inlineSvg}</body></html>`;
  await writeGenerated(board.file,html);
}
await fs.writeFile(path.join(root,'.generated.json'),JSON.stringify(generated,null,2)+'\n');
console.log(`已生成 ${manifest.artboards.length} 张 SVG 与 Canvas 画板。`);
