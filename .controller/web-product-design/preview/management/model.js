/** Small deterministic browser-only scenario model. Reset removes every mutation. */
import {AGENTS, INITIAL_GENERATION, NEXT_GENERATION, MESSAGES, OPERATIONS, ROOT, OFFICES, VERSION, key} from './fixtures.js';

export const STORE = 'agentx-preview:' + VERSION;
export const SCOPE = 'atlas-sandbox';
export const agentStates = {failed:['异常','danger'], active:['运行中','success'], waiting:['等待','info'], pending:['启动中','warning'], stopped:['已停止','neutral'], unknown:['未知','warning']};
export const operationStates = {lost:['回执丢失','warning'], missing:['未查到原请求','warning'], unknown:['结果未知','danger'], accepted:['已受理','success'], succeeded:['已完成','success'], rejected:['已拒绝','danger'], conflict:['标识冲突','danger']};
export const kinds = {send:'发送消息', directory:'联系目录', transcript:'读取 Transcript', stop:'停止实例', resume:'恢复实例', launch:'启动 Agent', labels:'更新标签'};
export const offices = Object.fromEntries(OFFICES.map(x => [x.id,x]));
export const seed = () => {
  const m = ({
  gen:INITIAL_GENERATION, agents:structuredClone(AGENTS), messages:structuredClone(MESSAGES), ops:structuredClone(OPERATIONS),
  drafts:{}, snapshots:{[key(AGENTS[9])]:'已读取'}, scenario:'normal', seq:0, labelRev:{}
  });
  admitWithoutReply(m,m.ops[0]);
  return m;
};
export function load() {
  try { return JSON.parse(localStorage.getItem(STORE)) || seed(); } catch { return seed(); }
}
export function save(m) {
  localStorage.setItem(STORE,JSON.stringify(m));
}
export function capability(a, kind, m) {
  if (!a) return false;
  return ({send:['active','waiting','failed'].includes(a.status),stop:['active','waiting','failed','pending'].includes(a.status),
    resume:a.status==='stopped' && a.resume!==false && !m?.agents.some(other=>other.name===a.name&&other.status!=='stopped'), transcript:a.status!=='pending'&&a.status!=='unknown',labels:!['unknown','stopped'].includes(a.status)})[kind] ?? false;
}
export const alerting = a => a.status==='failed'||a.status==='unknown';
export function preview(directory) {
  const path=directory.trim().replace(/\/+$/,'');
  if (!path || !path.startsWith('/') || path==='/tmp' || path.startsWith('/missing')) return {state:'error',path:directory,reason:'无法读取此目录。请输入工作区下的绝对路径。'};
  const office=[...OFFICES].filter(x=>x.path!=='global').sort((a,b)=>b.path.length-a.path.length).find(x=>path===x.path||path.startsWith(x.path+'/'));
  if (!office || office.error || path.endsWith('/no-default')) return {state:'error',path,office,reason:office?.error || '此路径没有可解析的默认 Role。'};
  const role=office.roles.find(x=>x.default);
  if(path.endsWith('/unknown')) return {state:'unknown',path,office,role,reason:'无法核实原绑定的确切执行。'};
  if(path.endsWith('/role-changed')) return {state:'role_changed',path,office,role,reason:'当前默认 Role 与既有绑定不同。'};
  if(path.endsWith('/adoptable')) return {state:'adoptable',path,office,role,reason:'找到一个可采纳的候选；预览没有绑定端点。',recipient:key(office.id==='web'?AGENTS[2]:AGENTS[1])};
  if(path.endsWith('/ambiguous')) return {state:'ambiguous',path,office,role,reason:'2 个候选实例。无法唯一确认收件人。'};
  if(path.endsWith('/stale')) return {state:'stale',path,office,role,reason:'绑定的执行已过期。预览不会更新绑定。'};
  if(office.id==='docs') return {state:'absent',path,office,role,reason:'暂无已绑定实例。发送可能启动默认 Role。'};
  return {state:'bound',path,office,role,recipient:key(office.id==='web'?AGENTS[2]:AGENTS[1])};
}
function id(m) {
  return 'a0859d02-77d6-4787-8149-ce30'+String(++m.seq).padStart(8,'0');
}
function newAgent(m,op,defaults={}) {
  const i=op.input;
  const a={name:i.name,id:'208284720434928'+String(m.seq).padStart(4,'0'),ref:'incarnation',status:'waiting',
    office:i.office||'global',role:i.role||null,tool:i.tool||'codex',profile:i.profile||'codex-build',cwd:i.cwd||ROOT,time:'11:28',
    title:i.title,adoption:'adopted',labels:{},...defaults};
  m.agents.unshift(a);
  return key(a);
}
export function execute(m,op) {
  if(op.gen!==m.gen) return;
  const a=m.agents.find(x=>key(x)===op.target);
  op.state='succeeded';
  switch(op.kind) {
    case 'send':
      if(!capability(a,'send')) return reject(op,'原实例不可用。未改投同名实例。');
      op.state='accepted'; op.result='消息已受理，等待实际通信。';
      op.messageId='118200000000000'+String(m.seq).padStart(3,'0');
      (m.messages[op.target]??=[]).push({by:'bigboss',time:'11:28',text:op.input.message,intent:op.input.intent,id:op.messageId,...(op.input.reply_to?{reply_to:op.input.reply_to}:{}),...(op.input.thread?{thread:op.input.thread}:{})});
      break;
    case 'directory': {
      const route=preview(op.target);
      if(!['bound','absent','adoptable'].includes(route.state)) return reject(op,route.reason);
      let recipient=route.recipient;
      let created=false;
      if(route.state==='absent'||(route.office.id==='web'&&route.path.endsWith('/src'))) {
        const override=route.office.id==='web';
        const name=override?'reni':'wena';
        const existing=m.agents.find(x=>x.name===name&&x.office===route.office.id&&x.status!=='stopped');
        recipient=existing?key(existing):newAgent(m,{input:{name,office:route.office.id,role:route.role.name,cwd:route.path,profile:route.role.profile}});
        created=!existing;
      }
      if(!capability(m.agents.find(x=>key(x)===recipient),'send')) return reject(op,'执行时的确切收件人已不可用。');
      op.subject=recipient;
      op.state='accepted'; op.result=created?'执行时解析到新的默认端点，已启动并受理消息。':'已向执行时解析的端点受理消息。';
      op.route={workspace:route.office.path,role:route.role.name,created};
      op.messageId='118200000000000'+String(m.seq).padStart(3,'0');
      (m.messages[recipient]??=[]).push({by:'bigboss',time:'11:28',text:op.input.message,intent:op.input.intent,id:op.messageId,...(op.input.reply_to?{reply_to:op.input.reply_to}:{}),...(op.input.thread?{thread:op.input.thread}:{})});
      break;
    }
    case 'transcript':
      if(!capability(a,'transcript')) return reject(op,'当前实例没有可读 Transcript。');
      if(a.transcript==='failed') {m.snapshots[op.target]='读取失败';return reject(op,'Provider 快照任务执行失败。');}
      if(a.transcript==='unknown') {m.snapshots[op.target]='结果未知';op.state='unknown';op.result='Provider 快照读取结果未知。';break;}
      m.snapshots[op.target]=a.transcript==='unavailable'?'不可用':m.messages[op.target]?.length?'已读取':'空快照';
      op.result=m.snapshots[op.target]==='不可用'?'Provider 记录不可用：未发现会话文件。':m.snapshots[op.target]==='空快照'?'已读取；当前没有模型交换。':'已读取有界 Provider 快照。';
      break;
    case 'launch': {
      const name='demo-'+String(m.seq).padStart(2,'0');
      const o=offices[op.input.office];
      if(o?.error) return reject(op,o.error);
      const role=op.input.role&&o?.roles.find(r=>r.name===op.input.role);
      newAgent(m,{input:{...op.input,name,profile:role?.profile||op.input.profile}});
      op.label='@'+name;
      op.result='已启动新实例。';
      break;
    }
    case 'stop': {
      if(!capability(a,'stop')) return reject(op,'只允许停止选中的活动实例。');
      const stopped={...a,ref:'stopped_event',id:'900719925474'+String(2000+m.seq),time:'11:28',status:'stopped',issue:null};
      m.agents.splice(m.agents.indexOf(a),1,stopped);
      m.messages[key(stopped)]=[...(m.messages[op.target]||[])];
      op.result='已停止确切实例；历史快照可查。';
      break;
    }
    case 'resume':
      if(!capability(a,'resume',m)) return reject(op,'此停止快照不支持恢复。');
      newAgent(m,op,{...a,id:'308284720434928'+String(m.seq).padStart(4,'0'),status:'waiting',ref:'incarnation',time:'11:28'});
      op.result='从原快照恢复为新执行。';
      break;
    case 'labels':
      if(!capability(a,'labels')) return reject(op,'此确切实例不支持编辑标签。');
      if(op.input.revision !== (m.labelRev[op.target]||0)) { op.state='conflict'; op.result='标签版本已变化。请核对现值后重新提交。'; return; }
      a.labels={...a.labels,...op.input.set};
      for(const name of op.input.remove) delete a.labels[name];
      m.labelRev[op.target]=op.input.revision+1; op.result='已更新标签。';
      break;
  }
}
function reject(op,reason) {
  op.state='rejected';op.result=reason;
}
export function submit(m,kind,target,input,label) {
  const op={id:id(m),gen:m.gen,kind,target,input,label,time:'11:28',state:'lost'};
  m.ops.unshift(op);
  try {save(m);} catch {m.ops.shift();throw new Error('无法保存原请求。未执行操作。');}
  if(m.scenario==='lost' && ['send','directory'].includes(kind)) {
    admitWithoutReply(m,op);m.scenario='normal';return op;
  }
  execute(m,op);
  return op;
}
// The mock POST runs once. Its client-facing receipt is hidden until GET.
function admitWithoutReply(m,op) {
  const admitted=structuredClone(op);
  execute(m,admitted);
  delete admitted.next;
  op.hiddenReceipt=admitted;
  delete op.next;
}
export function recover(m,op,step) {
  if(op.gen!==m.gen) return;
  if(step==='replay') {
    if(op.state==='missing') execute(m,op);
    return;
  }
  if(op.state!=='lost') return;
  if(op.hiddenReceipt) {
    Object.assign(op,op.hiddenReceipt);
    delete op.hiddenReceipt;
  } else op.state=op.next||'unknown';
}
export function setScenario(m,scenario) {
  if(scenario==='generation' && m.gen!==NEXT_GENERATION) {
    m.gen=NEXT_GENERATION; m.agents=structuredClone(AGENTS).filter(a=>a.name!=='luma'||a.status==='stopped');
    m.messages=structuredClone(MESSAGES); delete m.messages[key(AGENTS[0])]; m.drafts={}; m.snapshots={};m.labelRev={};
  }
  m.scenario=scenario;
}
