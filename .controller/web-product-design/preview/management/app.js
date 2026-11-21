import {openDialog, closeDialog} from '../system/disclosure.js';
import {AGENTS, OFFICES, ROOT, VERSION, key} from './fixtures.js';
import {load, save, seed, STORE, preview, capability, offices, submit, recover, setScenario} from './model.js';
import {e, btn, icon, badge, field, notice, refTitle} from './ui.js';
import * as B from './blocks.js';

let m=load();
const ui={filter:'all',query:'',officeFilter:'',labelFilter:'',limit:8,opFilter:'all',views:{},routes:{},directoryDrafts:{},mobilePicker:false,scroll:{},stale:false,pathId:''};
let route, pane, previousRoute='', layerKind='', layerData={}, returnFocus='';
const app=document.getElementById('app');
const dialog=document.getElementById('layer');
const say=message=>{ document.getElementById('announcer').textContent=message; };
const href=(page,id)=>'#'+page+(id?'/'+encodeURIComponent(id):'');
function readRoute(){
  const [page,raw='']=location.hash.slice(1).split('/');
  let id='';
  try { id=decodeURIComponent(raw); } catch { /* Invalid share target stays unselected. */ }
  return {page:['agents','workspaces','operations'].includes(page)?page:'agents',id};
}
function navigate(page,id){
  const next=href(page,id);
  if(location.hash===next) render();
  else location.hash=next;
}
function selector(el){
  if(!el || el===document.body || !(app.contains(el)||dialog.contains(el)))return '';
  const attr=(name,value)=>'['+name+'="'+CSS.escape(value)+'"]';
  if(el.id) return '#'+CSS.escape(el.id);
  if(el.matches('a[href]')) return 'a'+attr('href',el.getAttribute('href'));
  if(el.dataset.cmd) return 'button'+attr('data-cmd',el.dataset.cmd)+(el.dataset.value?attr('data-value',el.dataset.value):'');
  if(el.name) return (dialog.contains(el)?'#layer ':'#app ')+attr('name',el.name);
  if(el.hasAttribute('popovertarget')) return 'button'+attr('popovertarget',el.getAttribute('popovertarget'));
  return '';
}
function rememberScroll(){
  if(!previousRoute) return;
  app.querySelectorAll('[data-scroll]').forEach(el=>{
    const id=el.dataset.scroll;
    const k=['agents','workspaces','operations'].includes(id)?id:previousRoute+'/'+id;
    ui.scroll[k]=el.scrollTop;
  });
}
function restoreScroll(){
  app.querySelectorAll('[data-scroll]').forEach(el=>{
    const id=el.dataset.scroll;
    const k=['agents','workspaces','operations'].includes(id)?id:previousRoute+'/'+id;
    el.scrollTop=ui.scroll[k]||0;
  });
}
function demoMenu(){
  return '<div id="demo-settings" class="ax-popover demo-menu" popover><strong>合成数据演示</strong><small>'+VERSION+' · atlas-sandbox · 无实时连接</small><div class="demo-buttons">'+[
    ['refresh','刷新 Agent','refresh'],['stale','切换刷新失败','clock'],['simulate-lost','下次发送丢失回执','operations'],
    ['simulate-conflict','下次保存遇到标签冲突','branch'],['next-generation','切换数据代次','shield'],['reset','重置演示','refresh']
  ].map(([cmd,label,glyph])=>btn(label,cmd,{glyph})).join('')+'</div>'+
  (m.scenario==='lost'?badge('下次发送将丢失回执','warning'):m.scenario==='conflict'?badge('下次保存将遇到冲突','warning'):'')+'</div>';
}
const titles={agents:['Agent 实例', '受管资源','agents'],workspaces:['工作区与角色','协作目录','workspaces'],operations:['操作回执','恢复与追踪','operations']};
function render(){
  rememberScroll();
  const focus=app.contains(document.activeElement)?selector(document.activeElement):'';
  const cursor=focus&&document.activeElement.name==='query'?[document.activeElement.selectionStart,document.activeElement.selectionEnd]:null;
  route=readRoute();
  ui.pathId=route.id;
  const {page,id}=route;
  previousRoute=page+'/'+id;
  let left,detail,summary='';
  if(page==='agents'){
    pane=id?m.agents.find(a=>key(a)===id):ui.suppressAuto?undefined:m.agents[0];
    left=B.AgentIndex(m,ui,key(pane));detail=B.AgentWorkArea(m,ui,pane);summary=B.AgentSummary(m,ui);
  }else if(page==='workspaces'){
    pane=offices[id||'atlas'];
    left=B.WorkspaceIndex(pane?.id); detail=B.WorkspaceWorkArea(m,ui,pane);
  }else{
    pane=id?m.ops.find(x=>x.id===id):m.ops[0];
    left=B.OperationIndex(m,ui,pane?.id);detail='<section class="detail-panel operations-detail" data-scroll="receipt">'+B.ReceiptBlock(m,pane)+'</section>';
  }
  const [title,section,glyph]=titles[page];
  const rightLabel=page==='agents'?(pane?'@'+pane.name:'Agent'):page==='workspaces'?(pane?.name||'工作区'):(pane?.label||'回执');
  app.innerHTML=B.ScopeBar(m,page)+'<main class="workspace-main" id="workspace-main"><div class="workspace-heading"><div class="page-heading"><div class="eyebrow">'+icon(glyph)+e(section)+'</div><h1 tabindex="-1" id="page-title">'+e(title)+'</h1></div>'+
    '<div class="preview-badge"><span class="state-dot"></span> 合成预览</div></div>'+summary+
    '<div class="mobile-picker"><button type="button" class="ax-button" id="picker" data-cmd="picker" aria-expanded="'+ui.mobilePicker+'">'+icon('list')+(ui.mobilePicker?'返回详情':'选择 · '+e(rightLabel))+icon('collapse')+'</button></div>'+
    '<div class="resource-layout" data-ui-layout="'+page+'-workspace" data-picker="'+ui.mobilePicker+'">'+left+detail+'</div>'+
    '<footer class="workspace-footer"><span>AgentX <span class="footer-divider">/</span> '+e(section)+'</span><span>'+e(m.gen)+' · 合成数据</span></footer></main>'+demoMenu();
  restoreScroll();
  if(focus && !dialog.open){
    const el=document.querySelector(focus);
    if(el?.getClientRects().length){el.focus({preventScroll:true});if(cursor)el.setSelectionRange(...cursor);}
  }
}
function layerBody(kind,data){
  switch(kind){
    case 'facts':return B.AgentFacts(data.a);
    case 'receipt':return B.ReceiptBlock(m,m.ops.find(o=>o.id===data.id),true);
    case 'labels':return B.LabelsForm(data.a,m.labelRev[key(data.a)]||0,data.draft);
    case 'launch':return B.LaunchForm(data.prefill,data.values);
    case 'directory':return B.DirectoryForm(data.preview,ui.directoryDrafts[data.preview.path]||{});
    case 'stop':
    case 'resume':return '<dl class="fact-grid"><div><dt>确切目标</dt><dd class="mono">'+e(refTitle(key(data.a)))+'</dd></div></dl>'+
      notice(kind==='stop'?'终止该实例的进程。操作结果不代表任务已经完成。':'由此停止事件恢复为新的执行。当前同名实例不会被替换。')+
      '<form data-form="'+kind+'" class="ax-dialog-actions">'+btn('取消','dismiss')+'<button class="ax-button" data-variant="'+(kind==='stop'?'danger':'primary')+'" type="submit">'+(kind==='stop'?'终止进程':'恢复执行')+'</button></form>';
    case 'reset':return '<p>清除当前浏览器中的合成操作、标签和草稿，恢复初始样本。</p><form data-form="reset" class="ax-dialog-actions">'+btn('取消','dismiss')+'<button class="ax-button" data-variant="danger" type="submit">重置</button></form>';
  }
}
const layerTitle={facts:'实例详情',receipt:'操作回执',labels:'编辑标签',launch:'启动 Agent',directory:'联系目录',stop:'终止进程？',resume:'恢复执行？',reset:'重置演示？'};
function paintLayer(){
  if(['facts','receipt'].includes(layerKind)) dialog.dataset.placement='drawer'; else delete dialog.dataset.placement;
  dialog.innerHTML='<header class="ax-dialog-head"><h2 id="layer-title">'+layerTitle[layerKind]+'</h2><form method="dialog"><button type="submit" class="ax-button ax-icon-button" data-variant="quiet" aria-label="关闭">'+icon('close')+'</button></form></header><div class="ax-dialog-body">'+layerBody(layerKind,layerData)+'</div>';
}
function layer(kind,data,opener,keepTrigger=false){
  if(!keepTrigger) returnFocus=selector(opener)||selector(document.activeElement);
  layerKind=kind;layerData=data;
  paintLayer();
  if(!openDialog(dialog,opener)) (dialog.querySelector('[autofocus]')||dialog.querySelector('button'))?.focus();
}
dialog.addEventListener('close',()=>{
  if(dialog.open) return;
  const el=returnFocus?document.querySelector(returnFocus):null;
  if(el?.getClientRects().length) el.focus({preventScroll:true});
  else document.getElementById('page-title')?.focus({preventScroll:true});
});
function authorizedSubmit(...args){
  try {return submit(m,...args);}
  catch (error) {say(error.message);window.alert(error.message);return null;}
}
function showOp(op,trigger){
  if(!op)return;
  save(m);
  render();
  const restored=trigger&&document.querySelector(trigger);
  layer('receipt',{id:op.id},restored||undefined,dialog.open);
  say('操作回执已更新');
}

document.addEventListener('click',ev=>{
  const button=ev.target.closest('[data-cmd]');
  if(!button||button.disabled) return;
  const {cmd,value}=button.dataset;
  const current=route.page==='agents'?pane:null;
  switch(cmd){
    case 'picker': ui.mobilePicker=!ui.mobilePicker;render();break;
    case 'agent-filter':
      ui.filter=value;ui.limit=8;ui.scroll.agents=0;render();break;
    case 'more-agents':ui.limit+=8;render();break;
    case 'reply':
    case 'clear-reply': {
      if(!capability(current,'send')||ui.stale)break;
      const k=key(current);
      const draft=(m.drafts[k]??={});
      if(cmd==='reply') draft.reply_to=value;else delete draft.reply_to;
      save(m);render();app.querySelector('.composer textarea')?.focus();break;
    }
    case 'agent-tab': if(current){ui.views[key(current)]=value;render();} break;
    case 'op-filter':ui.opFilter=value;render();break;
    case 'facts': if(current) layer('facts',{a:current},button);break;
    case 'labels': if(capability(current,'labels')&&!ui.stale) layer('labels',{a:current,base:{...current.labels}},button);break;
    case 'stop':
    case 'resume': if(capability(current,cmd,m)&&!ui.stale) layer(cmd,{a:current},button);break;
    case 'launch': layer('launch',{prefill:value},button);break;
    case 'receipt':layer('receipt',{id:value},button);break;
    case 'transcript':
      if(capability(current,'transcript')&&!ui.stale) showOp(authorizedSubmit('transcript',key(current),{},'@'+current.name),selector(button));
      break;
    case 'directory': {
      const data=ui.routes[pane?.id];
      if(data?.preview&&!data.changed&&data.preview.state!=='error') layer('directory',{preview:data.preview},button);
      break;
    }
    case 'lookup':
    case 'replay': {
      const op=m.ops.find(x=>x.id===value);
      if(!op)break;
      recover(m,op,cmd);
      if(op.kind==='send'&&op.state==='accepted'&&m.drafts[op.target]?.message===op.input.message&&
          (m.drafts[op.target]?.intent||'request')===op.input.intent&&m.drafts[op.target]?.reply_to===op.input.reply_to) delete m.drafts[op.target];
      if(op.kind==='directory'&&op.state==='accepted'&&ui.directoryDrafts[op.target]?.message===op.input.message) delete ui.directoryDrafts[op.target];
      save(m); render();
      if(dialog.open) {
        paintLayer();
        (dialog.querySelector('#receipt-replay')||dialog.querySelector('button'))?.focus();
      }
      say(cmd==='lookup'?'原请求回执已查询':'已处理原请求');
      break;
    }
    case 'find-operation': {
      const op=m.ops.find(x=>x.id===value);
      if(!op||op.gen!==m.gen) break;
      ui.query=op.label.replace(/^@/,'');ui.filter=op.kind==='stop'?'stopped':'live';ui.officeFilter='';ui.labelFilter='';ui.limit=8;
      ui.suppressAuto=true;
      closeDialog(dialog);ui.mobilePicker=true; navigate('agents');break;
    }
    case 'dismiss':closeDialog(dialog);break;
    case 'refresh':ui.stale=false;render();say('合成数据已刷新');break;
    case 'stale':ui.stale=!ui.stale;render();say(ui.stale?'已模拟刷新失败':'刷新已恢复');break;
    case 'simulate-lost': case 'simulate-conflict': case 'next-generation': {
      setScenario(m,({'simulate-lost':'lost','simulate-conflict':'conflict','next-generation':'generation'})[cmd]);save(m);
      if(cmd==='next-generation'){ui.routes={};ui.views={};ui.directoryDrafts={};}
      render();say(({'simulate-lost':'下次发送将丢失回执','simulate-conflict':'下次保存将遇到标签冲突','next-generation':'已切换数据代次'})[cmd]);break;
    }
    case 'reset': document.getElementById('demo-settings')?.hidePopover();layer('reset',{},document.querySelector('[popovertarget="demo-settings"]'));break;
    case 'label-rebase': {
      const input=dialog.querySelector('[name=revision]');
      const editor=dialog.querySelector('[name=labels]');
      const desired=layerData.desired;
      const staged=labelChanges(layerData.base,desired);
      const latest={...layerData.a.labels};
      const rebased={...latest,...staged.set};
      for(const k of staged.remove) delete rebased[k];
      layerData.base=latest;
      if(editor) editor.value=Object.entries(rebased).map(([k,v])=>k+'='+v).join('\n');
      if(input) input.value=String(m.labelRev[key(layerData.a)]||0);
      dialog.querySelector('#label-error').innerHTML='<span>已按当前版本重新准备；请核对草稿后保存。</span>';
      dialog.querySelector('[name=labels]')?.focus();
      break;
    }
  }
});
document.addEventListener('input',ev=>{
  const el=ev.target;
  const f=el.form?.dataset.form;
  if(el.name==='query'){ui.query=el.value;ui.limit=8;render();return;}
  if(f==='send'&&pane){const k=key(pane); (m.drafts[k]??={})[el.name]=el.value;save(m);}
  if(f==='route'){
    const draft=(ui.routes[pane.id]??={});
    draft.directory=el.value;draft.changed=!!draft.preview && el.value.trim().replace(/\/+$/,'')!==draft.preview.path;
    const result=app.querySelector('[data-route-result]');
    if(result) result.hidden=draft.changed;
  }
  if(f==='directory-send') (ui.directoryDrafts[layerData.preview.path]??={})[el.name]=el.value;
});
document.addEventListener('change',ev=>{
  const el=ev.target;
  if(['officeFilter','labelFilter'].includes(el.name)){ui[el.name]=el.value;ui.limit=8;render();}
  if(el.form?.dataset.form==='launch'&&['office','role'].includes(el.name)){
    const vals=Object.fromEntries(new FormData(el.form));
    if(el.name==='office') vals.role='';
    const focusName=el.name;layerData.values=vals;paintLayer();dialog.querySelector('[name='+focusName+']')?.focus();
  }
});
const labelChanges=(base,desired)=>({
  set:Object.fromEntries(Object.entries(desired).filter(([k,v])=>base[k]!==v)),
  remove:Object.keys(base).filter(k=>!Object.hasOwn(desired,k))
});
document.addEventListener('submit',ev=>{
  const f=ev.target.dataset.form;
  if(!f)return;
  ev.preventDefault();
  const values=Object.fromEntries(new FormData(ev.target));
  const trigger=selector(ev.submitter);
  switch(f){
    case 'send': {
      const k=key(pane);
      if(!capability(pane,'send')||ui.stale||!values.message.trim())return;
      const reply=(m.messages[k]||[]).find(msg=>msg.id===values.reply_to);
      if(reply?.thread) values.thread=reply.thread;
      const op=authorizedSubmit('send',k,values,'@'+pane.name);
      if(op?.state==='accepted')delete m.drafts[k];
      showOp(op,trigger);break;
    }
    case 'route': {
      ui.routes[pane.id]={directory:values.directory,preview:preview(values.directory)};
      render();say('只读路由预览已更新');break;
    }
    case 'workspace-path': {
      const path=values.path.trim().replace(/\/+$/,'');
      const found=OFFICES.find(x=>x.path===path);
      navigate('workspaces',found?.id||path);break;
    }
    case 'directory-send': {
      const r=layerData.preview;
      if(!values.message.trim()) return;
      const op=authorizedSubmit('directory',r.path,values,r.office?.name||r.path);
      if(op?.state==='accepted') delete ui.directoryDrafts[r.path];
      showOp(op,trigger);break;
    }
    case 'launch': {
      const [type,runtime]=(values.runtime||'').split(':');
      delete values.runtime;
      if(!values.role){if(type==='tool')values.tool=runtime;else if(type==='profile')values.profile=runtime;}
      showOp(authorizedSubmit('launch',offices[values.office]?.path||values.office,values,values.title||'启动 Agent'),trigger);break;
    }
    case 'labels': {
      const current=m.agents.find(a=>key(a)===key(layerData.a));
      if(!capability(current,'labels')) return;
      const labels={};
      for(const line of values.labels.split(/\n/).map(x=>x.trim()).filter(Boolean)){
        const match=/^([a-zA-Z][\w.-]{0,31})=(.{1,48})$/.exec(line);
        if(!match || Object.hasOwn(labels,match[1])){
          dialog.querySelector('#label-error').textContent='每行填写一个不重复的 key=value。';return;
        }
        labels[match[1]]=match[2];
      }
      if(m.scenario==='conflict'){
        const k=key(current);
        m.labelRev[k]=(m.labelRev[k]||0)+1;current.labels.review='pending';m.scenario='normal';
      }
      const changes=labelChanges(layerData.base,labels);
      const op=authorizedSubmit('labels',key(current),{...changes,revision:Number(values.revision)},'@'+current.name);
      if(!op)return;
      if(op.state==='conflict'){
        layerData.desired=labels;
        save(m);
        dialog.querySelector('#label-error').innerHTML=notice('另一处已更新标签。草稿仍在；核对现值后可按新版本编辑。','danger')+
          '<span>现值：'+e(Object.entries(current.labels).map(([k,v])=>k+'='+v).join('、'))+'</span>'+btn('采用当前版本','label-rebase');
      }else showOp(op,trigger);
      break;
    }
    case 'stop':
    case 'resume': {
      const a=layerData.a;
      showOp(authorizedSubmit(f,key(a),{},'@'+a.name),trigger);break;
    }
    case 'reset':
      closeDialog(dialog);localStorage.removeItem(STORE);location.replace(location.pathname);break;
  }
});

document.addEventListener('click',ev=>{
  const a=ev.target.closest('a[href^="#"]');
  if(!a) return;
  if(a.getAttribute('href')==='#agents') ui.suppressAuto=false;
  if(a.matches('.resource-row')&&location.hash===a.hash){
    ui.mobilePicker=false;render();
    if(matchMedia('(max-width: 720px)').matches)document.getElementById('picker').focus();
  }
  if(dialog.contains(a)) closeDialog(dialog);
});
window.addEventListener('hashchange',()=>{
  const current=readRoute();
  if(current.id) ui.mobilePicker=false;
  closeDialog(dialog); render();
  if(!ui.mobilePicker && matchMedia('(max-width: 720px)').matches) document.getElementById('picker')?.focus();
});
render();
