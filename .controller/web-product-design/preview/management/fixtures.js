/** Presentation fixtures only. Native Service remains authoritative in the real product. */
export const VERSION = 'management-2026-11-18-r2';
export const INITIAL_GENERATION = 'demo-g7';
export const NEXT_GENERATION = 'demo-g8';
export const NOW = '2026-11-18T11:28:00Z';
export const ROOT = '/work/atlas';
export const OFFICES = [
  {id:'atlas', name:'Atlas', path:ROOT, source:'Git 根 · 已采纳', coverage:'采样已截断', roles:[
    {name:'controller', origin:'项目', profile:'codex-review', default:true, note:'安排发布、追踪阻塞与验收'},
    {name:'reviewer', origin:'项目', profile:'codex-review', note:'审阅构建和发布结果'},
    {name:'researcher', origin:'全局', profile:null, note:'收集证据和验证假设'}]},
  {id:'web', name:'Web Console', path:'/work/atlas/web-console', source:'Git 根 · 已采纳', roles:[
    {name:'fronter', origin:'项目', profile:'codex-build', default:true, note:'构建控制台工作区'},
    {name:'reviewer', origin:'继承', profile:'codex-review', note:'检查变更和浏览器证据'}]},
  {id:'docs', name:'Docs', path:'/work/atlas/docs', source:'AgentX 标记', roles:[
    {name:'writer', origin:'项目', profile:null, default:true, note:'维护项目手册'}]},
  {id:'global', name:'全局', path:'global', source:'显式全局范围', roles:[
    {name:'researcher', origin:'全局', profile:null, default:true, note:'收集证据和验证假设'}]},
  {id:'broken', name:'Migration', path:'/work/atlas/migration', source:'历史采纳', error:'当前 Role 目录包含无效定义：migration/reviewer.md。'}];
const row = (name, id, status, office, role, time, extra={}) => ({
  name,id,status,office,role,time, ref:status==='stopped'?'stopped_event':'incarnation', tool:'codex', profile:role==='controller'||role==='reviewer'?'codex-review':'codex-build',
  cwd:OFFICES.find(x=>x.id===office && x.path!=='global')?.path || ROOT, adoption:'adopted', labels:{},...extra
});
export const AGENTS = [
  row('luma','1092847204349280141','failed','atlas','reviewer','11:26',{issue:'模型请求失败 · Provider 超时', labels:{lane:'release',priority:'high'}}),
  row('nori','1092847204349280142','active','atlas','controller','11:27',{labels:{lane:'release'}}),
  row('sora','1092847204349280143','waiting','web','fronter','11:25',{cwd:ROOT+'/web-console/src',labels:{lane:'frontend'}}),
  row('mika','1092847204349280144','active','web','reviewer','11:22',{labels:{lane:'frontend'}}),
  row('riko','1092847204349280145','waiting','atlas','researcher','11:18',{transcript:'failed'}),
  row('nami','1092847204349280146','pending','web','fronter','11:16'),
  row('yuna','1092847204349280147','waiting','global',null,'10:53',{transcript:'unknown'}),
  row('kiri','1092847204349280148','unknown',null,null,'10:27',{adoption:'unavailable',issue:'已采纳协作记录不可用'}),
  row('toma','1092847204349280149','waiting','global',null,'10:12',{adoption:'legacy'}),
  row('luma','9007199254741151','stopped','atlas','reviewer','10:02',{labels:{lane:'release'}}),
  row('kevi','9007199254741152','stopped','docs','writer','09:41',{resume:false}),
  row('noro','1092847204349280150','waiting','atlas','researcher','09:22',{tool:'claude',transcript:'unavailable'})
];
export const key = a => a ? a.name+'~'+a.ref+'~'+a.id : '';
const MESSAGE_ROWS = {
  [key(AGENTS[0])]: [
    {by:'bigboss',time:'11:04',text:'请核对 Atlas v2.8 的发布清单，并回报 Linux 验证结果。',intent:'request'},
    {by:'luma',time:'11:12',text:'源码检查通过。正在复核 Linux 浏览器流程，构建记录留在 docs/release/2.8.md。',intent:'inform'},
    {by:'bigboss',time:'11:15',text:'先确认只读预览可以打开；若 Provider 超时，请保留已采集证据。',intent:'request'}
  ],
  [key(AGENTS[9])]: [
    {by:'bigboss',time:'09:35',text:'检查 v2.7 基线截图，并整理已知差异。',intent:'request'},
    {by:'luma',time:'09:49',text:'基线已记录：移动端资源选择需要多一步。其余三条路径可完成。',intent:'inform'}
  ],
  [key(AGENTS[1])]: [{by:'nori',time:'11:23',text:'发布安排已更新。等待 Linux 验证结果。',intent:'inform'}]
};
export const MESSAGES=Object.fromEntries(Object.entries(MESSAGE_ROWS).map(([agent,rows],group)=>[agent,
  rows.map((msg,row)=>({...msg,id:'117200000000000'+String(group*100+row).padStart(3,'0'),...(group===0?{thread:'atlas-release'}:{})}))
]));
export const OPERATIONS = [
  {id:'d1859d02-77d6-4787-8149-ce30a7902101', gen:INITIAL_GENERATION, kind:'send', target:key(AGENTS[0]), label:'@luma', time:'11:24', state:'lost', input:{message:'请回报 Linux 验证的当前进展。', intent:'request'}, next:'accepted'},
  {id:'d1859d02-77d6-4787-8149-ce30a7902102', gen:INITIAL_GENERATION, kind:'directory', target:ROOT+'/web-console/src', label:'Web Console', time:'11:07', state:'unknown', input:{message:'请检查目录路由预览。',intent:'request'}},
  {id:'d1859d02-77d6-4787-8149-ce30a7902103', gen:INITIAL_GENERATION, kind:'transcript', target:key(AGENTS[9]), label:'@luma', time:'10:02', state:'succeeded', input:{}, result:'已读取有界 Transcript 快照。'},
  {id:'d1859d02-77d6-4787-8149-ce30a7902104', gen:INITIAL_GENERATION, kind:'send', target:key(AGENTS[2]), label:'@sora', time:'11:09', state:'lost', input:{message:'请整理 Web Console 截图。',intent:'request'}, next:'missing'}
];
export const TOOLS = ['codex','claude']; 
export const PROFILES = ['codex-build','codex-review','claude-research'];
