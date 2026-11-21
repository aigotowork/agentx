// All state is local to this document. No RPC, storage, network or real send.
const root = document.getElementById('app');
let state = JSON.parse(document.getElementById('initial-state').textContent);
let focusBeforeLayer = null;
let toastTimer;
let tipTimer;
let longPress;
let touchTipConsumed = false;
function hideTip(){const t=document.getElementById('tooltip');if(t)t.hidden=true;clearTimeout(tipTimer);}
function bindModal(){const modal=document.querySelector('[role="dialog"]');const shell=document.querySelector('.app-shell');if(shell)shell.inert=!!modal;if(modal)requestAnimationFrame(()=>document.getElementById('sheet-title')?.focus());}
function render(focusId){const pageScroll=document.getElementById('page-content')?.scrollTop||0;const detailScroll=document.querySelector('.detail-scroll')?.scrollTop||0;const queueScroll=document.querySelector('.agent-list')?.scrollTop||0;hideTip();document.documentElement.dataset.theme=state.theme;root.innerHTML=renderApp(state);document.getElementById('page-content')?.scrollTo(0,pageScroll);document.querySelector('.detail-scroll')?.scrollTo(0,detailScroll);document.querySelector('.agent-list')?.scrollTo(0,queueScroll);if(focusId)document.getElementById(focusId)?.focus();bindModal();}
function openLayer(name){focusBeforeLayer=document.activeElement?.dataset.action;state.layer=name;render();}
function closeLayer(){state.layer=null;render();if(focusBeforeLayer)document.querySelector(`[data-action="${focusBeforeLayer}"]`)?.focus();}
function toast(message){state.toast=message;render();clearTimeout(toastTimer);toastTimer=setTimeout(()=>{state.toast=null;const live=document.getElementById('live-status');if(live){live.classList.remove('visible');live.innerHTML='';}},4000);}
function back(){if(state.layer){closeLayer();return;}if(state.detail){state.detail=false;state.receipt=null;state.tab='evidence';if(state.returnView){state.view=state.returnView;state.returnView=null;}render();document.querySelector(`[data-action="${state.view==='usage'?'usage-agent:':'agent:'}${state.selected}"]`)?.focus();return;}}
function action(value){const [kind,arg]=value.split(':');switch(kind){
case 'nav':state.layer=null;state.view=arg;state.detail=false;state.tab='evidence';state.filter='all';state.query='';state.receipt=null;state.returnView=null;render();break;
case 'agent':state.selected=arg;state.detail=true;state.tab='evidence';state.receipt=state.receipts[arg]||null;render();break;
case 'usage-agent':state.returnView='usage';state.view='recovery';state.selected=arg;state.detail=true;state.tab='evidence';state.receipt=state.receipts[arg]||null;render();break;
case 'filter':state.filter=arg;render();document.querySelector(`[data-action="filter:${arg}"]`)?.focus();break;
case 'choose-filter':state.filter=arg;render();break;
case 'tab':state.tab=arg;render();document.querySelector(`[data-action="tab:${arg}"]`)?.focus();break;
case 'summary':if(arg==='0'){state.filter='attention';}else if(arg==='1'&&state.view==='recovery'){state.selected='atlas';state.receipt=state.receipts.atlas||null;state.detail=true;state.filter='all';}else if(arg==='1'&&state.view==='overview'){state.filter='running';}else if(arg==='2'){state.filter=state.view==='recovery'?'all':'done';if(state.view==='recovery'){state.selected='relay';state.receipt=state.receipts.relay||'unknown';state.detail=true;}}else {state.filter='all';}render();break;
case 'confirm':case 'demo-confirm':if(kind==='demo-confirm'){state.selected='atlas';}if(state.receipts[state.selected]){toast('原操作已保留。先查看操作记录与原会话。');break;}openLayer('confirm');break;
case 'send':{const branch=document.getElementById('demo-result').value;state.layer=null;state.receipt=branch;state.receipts[state.selected]=branch;state.tab='result';state.detail=true;state.view='recovery';state.busy=true;render();const target=document.querySelector('[data-dc-id="delivery-result"]');if(target)target.innerHTML=`<div class="loading-example">${icon('recovery')}正在核验并模拟提交…</div>`;setTimeout(()=>{state.busy=false;render();document.querySelector('[data-action="tab:result"]')?.focus();},450);break;}
case 'refresh':toast(state.view==='usage'?'采集依据：本地保留日志。离线样例没有新增记录。':state.receipt==='unknown'||state.selected==='relay'?'观察已刷新：原操作仍未核实，保留投递未知。':'观察已刷新：示例状态未变化。');break;
case 'evidence':state.expanded=!state.expanded;render();document.querySelector('[data-action="evidence"]')?.focus();break;
case 'menu-evidence':state.layer=null;state.expanded=true;state.tab='evidence';state.detail=true;render();break;
case 'demo-evidence':state.view='recovery';state.detail=true;state.expanded=true;render();break;
case 'coverage':state.coverage=!state.coverage;render();document.querySelector('[data-action="coverage"]')?.focus();break;
case 'day':state.chartDay=arg;render();document.querySelector(`[data-action="day:${arg}"]`)?.focus();break;
case 'theme':state.theme=state.theme==='light'?'dark':'light';render();break;
case 'clear':state.query='';state.filter='all';state.workspace='all';render('agent-search');break;
case 'selected':state.selected=state.selectedRows[0];state.receipt=state.receipts[state.selected]||null;state.detail=true;render();break;
case 'deselect':state.selectedRows=[];render();break;
case 'back':back();break;
case 'close':closeLayer();break;
case 'backdrop':closeLayer();break;
case 'glyph':toast('同一 16 × 16 网格 · 可编辑 SVG 见 icons/');break;
case 'range':break;
case 'none':break;
default:openLayer(kind);
}}
root.addEventListener('click',e=>{if(touchTipConsumed){touchTipConsumed=false;e.preventDefault();return;}const b=e.target.closest('[data-action]');if(!b||b.disabled)return;if(b.dataset.action==='backdrop'&&e.target!==b)return;hideTip();action(b.dataset.action);});
root.addEventListener('input',e=>{if(e.target.id==='followup')state.drafts[state.selected]=e.target.value;if(e.target.id==='agent-search'){const value=e.target.value;state.query=value;root.querySelector('#agent-list').innerHTML=filtered(state).length?filtered(state).map(a=>AgentRow(a,state)).join(''):`<div class="empty-state">${icon('search')}<h3>没有匹配的 Agent</h3><p>当前仅搜索 6 个已加载样例。</p>${button('清除条件','clear')}</div>`;root.querySelector('.list-label>span:last-child').textContent=`${filtered(state).length} 个对象`;}});
root.addEventListener('change',e=>{if(e.target.id==='workspace-scope'){state.workspace=e.target.value;render();}if(e.target.dataset.select){const id=e.target.dataset.select;state.selectedRows=e.target.checked?[...state.selectedRows,id]:state.selectedRows.filter(x=>x!==id);render();document.querySelector(`[data-select="${id}"]`)?.focus();}});
document.addEventListener('keydown',e=>{if(e.key==='Escape'){hideTip();back();return;}if(e.key==='/'&&!/INPUT|TEXTAREA|SELECT/.test(document.activeElement.tagName)&&!state.layer){e.preventDefault();document.getElementById('agent-search')?.focus();}const dialog=document.querySelector('[role="dialog"]');if(dialog&&e.key==='Tab'){const nodes=[...dialog.querySelectorAll('button:not(:disabled),input,textarea,select,[tabindex="0"]')];const first=nodes[0],last=nodes.at(-1);if(e.shiftKey&&(document.activeElement===first||document.activeElement.id==='sheet-title')){e.preventDefault();last?.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus();}return;}const tab=document.activeElement.closest('[role="tab"]');if(tab&&['ArrowRight','ArrowLeft','Home','End'].includes(e.key)){const tabs=[...tab.parentElement.querySelectorAll('[role="tab"]')];const i=tabs.indexOf(tab);const next=e.key==='Home'?tabs[0]:e.key==='End'?tabs.at(-1):tabs[(i+(e.key==='ArrowRight'?1:tabs.length-1))%tabs.length];e.preventDefault();action(next.dataset.action);}});
function showTip(el){if(!el||state.layer&& !el.closest('[role="dialog"]'))return;const t=document.getElementById('tooltip');if(!t)return;const text=el.dataset.tip;if(!text)return;t.textContent=text;t.hidden=false;const r=el.getBoundingClientRect(),b=t.getBoundingClientRect();const gap=parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--tooltip-gap'))||8;let y=r.bottom+gap;if(y+b.height>innerHeight-gap)y=r.top-b.height-gap;t.style.left=Math.max(gap,Math.min(r.left+r.width/2-b.width/2,innerWidth-b.width-gap))+'px';t.style.top=Math.max(gap,y)+'px';}
root.addEventListener('pointerover',e=>{const b=e.target.closest('[data-tip]');if(b&&e.pointerType!=='touch')tipTimer=setTimeout(()=>showTip(b),350);});
root.addEventListener('pointerout',e=>{if(e.target.closest('[data-tip]'))hideTip();});
root.addEventListener('focusin',e=>{const b=e.target.closest('[data-tip]');if(b)showTip(b);});
root.addEventListener('focusout',hideTip);
root.addEventListener('pointerdown',e=>{hideTip();if(e.pointerType==='touch'){const b=e.target.closest('[data-tip]');if(b){longPress=setTimeout(()=>{showTip(b);touchTipConsumed=true;},500);}}});
root.addEventListener('pointerup',()=>clearTimeout(longPress));root.addEventListener('pointercancel',()=>clearTimeout(longPress));
document.addEventListener('scroll',hideTip,true);window.addEventListener('resize',hideTip);
document.documentElement.dataset.theme=state.theme;
bindModal();
