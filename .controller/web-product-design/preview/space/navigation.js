import {agents, offices, posts, SCOPE, GENERATION, keyOf} from './fixtures.js';

export const defaults = {mode:'chat', office:'', q:'', status:'', tab:'members', channel:'messages', key:'', panel:'', detail:'', post:'', mail:'pending', before:''};
const routeKeys = ['view','office','q','state','tab','channel','scope','generation','agent','incarnation','stopped_event','record','panel','detail','post','mail','before'];

export function readLocation(href = location.href) {
  const q = new URL(href).searchParams;
  const next = {...defaults};
  let issue = '';
  const fail = (message = '这个链接包含无法识别的选项。') => { issue ||= message; };
  if (routeKeys.some(key => q.getAll(key).length > 1)) return {next, issue:'链接中的选项重复，已返回会话列表。'};
  function enumParam(url, field, options) {
    if (!q.has(url)) return;
    const val = q.get(url);
    if (options.includes(val)) next[field] = val; else fail();
  }
  enumParam('view','mode',['chat','scene']);
  enumParam('office','office',['',...offices.map(o=>o.id)]);
  enumParam('state','status',['','working','waiting','offline','unknown']);
  enumParam('tab','tab',['members','history','posts','skills']);
  enumParam('channel','channel',['messages','model','notes']);
  enumParam('panel','panel',['directory','mailbox']);
  enumParam('detail','detail',['agent','work','relations','actions']);
  enumParam('mail','mail',['pending','all']);
  enumParam('before','before',['h03']);
  next.q = q.get('q')?.slice(0,100) || '';
  if (q.has('post')) {
    const p = posts.find(item=>item.id===q.get('post') && (!next.office || item.office===next.office));
    if (p) next.post = p.id;
    else fail('找不到该办公室下的岗位。');
  }
  if (['scope','generation','agent','incarnation','stopped_event','record'].some(k=>q.has(k))) {
    const name = q.get('agent');
    const inc = q.get('incarnation') || '';
    const stop = q.get('stopped_event') || '';
    const kind = q.get('record') || (stop ? 'stopped' : 'current');
    const exact = q.get('scope')===SCOPE && q.get('generation')===GENERATION && name && inc && kind===(stop?'stopped':'current');
    const target = exact && agents.find(a=>a.key===keyOf({name,inc,stop}));
    if (target) {
      next.key = target.key;
      if (next.office && next.office!==target.office) next.office = target.office;
    } else {
      next.detail = '';
      fail('链接指向的确切实例不可用。请选择会话列表中的实例。');
    }
  }
  return {next,issue};
}

export function locationFor(state) {
  const url = new URL(location.href);
  const p = url.searchParams;
  routeKeys.forEach(k=>p.delete(k));
  // Emit an explicit view for compatibility with the installed route default.
  p.set('view',state.mode);
  for (const [key,val] of [['office',state.office],['q',state.q],['state',state.status],['tab',state.tab==='members'?'':state.tab],['panel',state.panel],['detail',state.detail],['post',state.post],['mail',state.panel==='mailbox' && state.mail==='all'?'all':''],['before',state.panel==='mailbox'?state.before:'']]) if(val) p.set(key,val);
  const target = agents.find(a=>a.key===state.key);
  if (target) {
    for (const [k,v] of [['scope',SCOPE],['generation',GENERATION],['agent',target.name],['incarnation',target.inc],['record',target.stop?'stopped':'current'],['stopped_event',target.stop],['channel',state.channel==='messages'?'':state.channel]]) if(v) p.set(k,v);
  }
  return url;
}
