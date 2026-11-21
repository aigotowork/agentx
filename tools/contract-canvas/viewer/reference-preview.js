/* ReferencePreview owns transient peeks and the bounded comparison shelf.
 * Canonical facts and main navigation remain with ContractWorkbench. */
window.createContractReferences=function(api){
  'use strict';
  const {state,nodeFor,isMethod,portsFor,esc,refMarkup,supportText,renderSources,navigate,toggleDrawer,publicFields,implementationFields,publicPorts}=api;
  const array=v=>Array.isArray(v)?v:[];
  const limit=3,pins=[];
  const popup=document.createElement('section');
  popup.id='reference-preview';popup.className='reference-preview';popup.hidden=true;
  popup.setAttribute('role','dialog');popup.setAttribute('aria-modal','false');popup.setAttribute('aria-labelledby','reference-preview-title');
  popup.dataset.uiBlock='ReferencePreview';document.body.append(popup);
  const shelf=document.createElement('section');
  shelf.className='comparison-shelf';shelf.hidden=true;shelf.setAttribute('aria-label','固定对照');shelf.dataset.uiBlock='ContractComparison';
  document.querySelector('#inspector').append(shelf);
  let current=null,origin=null,trail=[],sticky=false,openTimer,closeTimer;
  const referenceSelector='[data-reference],.reference-row[data-type],#inspector-content .related-chip,#inspector-content .method-button';
  const referenceFor=target=>target.closest(referenceSelector);
  const idFor=el=>el?.dataset.reference||el?.dataset.type||el?.dataset.method;
  function announce(message){document.querySelector('#announcer').textContent=message;}
  function contextFor(el){
    const card=el.closest('[data-pinned]');
    if(card)return card.dataset.pinned;
    if(popup.contains(el))return current?.id||state.selected;
    return el.dataset.field||state.selected;
  }
  function fieldRows(fields){
    return fields.map(f=>`<div class="field-row"><span class="field-name" title="${esc(f.type||'')}">${esc(f.name||'return')}${f.optional?'?':''}${f.nullable?' ∅':''}</span><span class="field-value">${refMarkup(f)}</span></div>`).join('');
  }
  function structure(n,compact){
    let html='';
    if(isMethod(n)){
      html+=`<div class="reference-signature">${esc(n.signature||n.id)}</div>`;
      if(n.owner)html+=`<div class="reference-section-label">OWNER</div>${refMarkup({typeIds:[n.owner]})}`;
      if(array(n.inputs).length)html+=`<div class="reference-section-label">INPUTS</div>${fieldRows(n.inputs)}`;
      if(n.output)html+=`<div class="reference-section-label">RETURNS</div>${fieldRows([{...n.output,name:'return'}])}`;
      const exits=[...array(n.errors),...array(n.outcomes)].map(e=>typeof e==='string'?e:e.id).filter(id=>nodeFor(id));
      if(exits.length)html+=`<div class="reference-section-label">ERRORS / OUTCOMES</div><div class="related-list">${exits.map(id=>refMarkup({typeIds:[id]})).join(' · ')}</div>`;
    }else{
      const fields=publicFields(n),internal=implementationFields(n),shown=compact?fields.slice(0,4):fields;
      if(fields.length)html+=`<div class="reference-section-label">FIELDS <span>${fields.length}</span></div>${fieldRows(shown)}`;
      if(fields.length>shown.length)html+=`<details class="reference-more"><summary>+ ${fields.length-shown.length} 字段</summary>${fieldRows(fields.slice(shown.length))}</details>`;
      const ports=publicPorts(n),internalPorts=portsFor(n).filter(p=>!ports.includes(p)),visible=compact?ports.slice(0,3):ports;
      if(ports.length)html+=`<div class="reference-section-label">METHOD PORTS <span>${ports.length}</span></div>${visible.map(p=>`<button class="reference-method" data-reference="${esc(p.id)}" aria-haspopup="dialog">ƒ ${esc(p.name)}() <span>↗</span></button>`).join('')}`;
      if(ports.length>visible.length)html+=`<details class="reference-more"><summary>+ ${ports.length-visible.length} 方法</summary>${ports.slice(visible.length).map(p=>`<button class="reference-method" data-reference="${esc(p.id)}" aria-haspopup="dialog">ƒ ${esc(p.name)}() ↗</button>`).join('')}</details>`;
      if(internalPorts.length)html+=`<details class="reference-more"><summary>实现端口 · ${internalPorts.length}</summary>${internalPorts.map(p=>refMarkup({typeIds:[p.id]})).join(' · ')}</details>`;
      if(internal.length)html+=`<details class="reference-more"><summary>实现字段 · ${internal.length}</summary>${fieldRows(internal)}</details>`;
      if(!fields.length&&!ports.length&&!internal.length&&!internalPorts.length)html+=`<div class="reference-signature">${esc(n.signature||n.summary||n.kind)}</div>`;
    }
    if(!compact&&n.support?.note)html+=`<details class="reference-capability"><summary>能力边界</summary><p class="scope-note">${esc(n.support.note)}</p></details>`;
    return html;
  }
  function heading(n,id=''){
    return `<span class="reference-kind">${esc(n.kind)} / ${esc(n.domain)}</span><h2 ${id?`id="${id}"`:''}>${esc(isMethod(n)?n.id:n.name||n.id)}</h2>`;
  }
  function drawPopup(){
    const n=nodeFor(current.id),pinned=pins.some(p=>p.id===n.id),full=pins.length===limit;
    popup.innerHTML=`<div class="reference-topline"><span>引用预览</span><div>${trail.length?'<button class="reference-icon" data-ref-action="back" aria-label="返回上一个引用" title="返回上一个引用">←</button>':''}<button class="reference-icon" data-ref-action="close" aria-label="关闭引用预览" title="关闭引用预览">×</button></div></div><div class="reference-body">${heading(n,'reference-preview-title')}<div class="reference-origin" title="${esc(current.from)}">${esc(current.from)} <span>→</span> ${esc(n.name||n.id)}</div><span class="reference-support">${esc(supportText(n))}</span>${structure(n,true)}</div><div class="reference-actions"><button data-ref-action="pin" ${!pinned&&full?'disabled':''}>${pinned?'查看对照':full?'对照已满（3）':'⊞ 固定对照'}</button><button data-ref-action="enter" class="reference-enter">进入画布 ↗</button></div>`;
    position();
  }
  function position(){
    if(popup.hidden||!origin?.isConnected)return;
    const edge=12,gap=8,rect=origin.getBoundingClientRect();
    const box=popup.getBoundingClientRect();
    let left=rect.right+gap;
    if(left+box.width>innerWidth-edge)left=rect.left-box.width-gap;
    left=Math.max(edge,Math.min(left,innerWidth-box.width-edge));
    const top=Math.max(edge,Math.min(rect.top,innerHeight-box.height-edge));
    popup.style.setProperty('--peek-x',left+'px');popup.style.setProperty('--peek-y',top+'px');
  }
  function open(el,interactive=false){
    clearTimeout(openTimer);clearTimeout(closeTimer);
    const id=idFor(el);if(!nodeFor(id))return;
    if(!popup.hidden&&current.id===id&&origin===el){sticky=sticky||interactive;if(interactive)popup.querySelector('button:not([disabled])')?.focus();return;}
    const nested=popup.contains(el),from=contextFor(el);
    if(nested&&current)trail.push({...current});else trail=[];
    if(!nested){origin?.setAttribute('aria-expanded','false');origin=el;}
    current={id,from};sticky=interactive||nested;popup.hidden=false;
    origin.setAttribute('aria-expanded','true');origin.setAttribute('aria-controls',popup.id);
    drawPopup();
    if(interactive||nested)popup.querySelector('button:not([disabled])')?.focus();
  }
  function close(restore=false){
    clearTimeout(openTimer);clearTimeout(closeTimer);
    const returnTo=origin;
    popup.hidden=true;sticky=false;current=null;trail=[];
    origin?.setAttribute('aria-expanded','false');origin=null;
    if(restore&&returnTo?.isConnected){returnTo.focus({preventScroll:true});clearTimeout(openTimer);}
  }
  function scheduleClose(){
    clearTimeout(closeTimer);
    closeTimer=setTimeout(()=>{if(!sticky&&!popup.contains(document.activeElement)&&document.activeElement!==origin)close();},220);
  }
  function renderShelf(){
    const positions=new Map([...shelf.querySelectorAll('[data-pinned]')].map(card=>[card.dataset.pinned,card.querySelector('.comparison-body')?.scrollTop||0]));
    shelf.hidden=!pins.length;
    shelf.innerHTML=pins.length?`<header class="comparison-heading"><span>固定对照 <b>${pins.length} / ${limit}</b></span><button class="reference-icon" data-ref-action="clear" aria-label="清空固定对照" title="清空固定对照">×</button></header><div class="comparison-cards">${pins.map(p=>{
      const n=nodeFor(p.id);
      return `<article class="comparison-card" data-pinned="${esc(n.id)}"><button class="comparison-toggle" data-ref-action="collapse" data-ref-id="${esc(n.id)}" aria-expanded="${p.open}" aria-controls="comparison-body-${pins.indexOf(p)}"><span class="comparison-title">${esc(isMethod(n)?n.id:n.name||n.id)}</span><span class="reference-kind">${esc(n.kind)}</span></button><div class="comparison-body" id="comparison-body-${pins.indexOf(p)}" ${p.open?'':'hidden'}><div class="reference-origin" title="${esc(p.from)}">← ${esc(p.from)}</div>${structure(n,false)}${renderSources(n)}</div><footer class="comparison-actions"><button data-ref-action="enter-pin" data-ref-id="${esc(n.id)}">进入画布 ↗</button><button data-ref-action="remove" data-ref-id="${esc(n.id)}" aria-label="移除 ${esc(n.id)} 对照">移除</button></footer></article>`;
    }).join('')}</div>`:'';
    document.querySelector('#inspector').classList.toggle('has-comparison',!!pins.length);
    shelf.querySelectorAll('[data-pinned]').forEach(card=>{card.querySelector('.comparison-body').scrollTop=positions.get(card.dataset.pinned)||0;});
  }
  function showPinned(id){
    close(true);
    if(state.narrow&&!document.querySelector('#inspector').classList.contains('open'))toggleDrawer('inspector');
    const card=[...shelf.querySelectorAll('[data-pinned]')].find(el=>el.dataset.pinned===id);
    if(card){setExpanded(id,true);card.scrollIntoView({block:'nearest'});card.querySelector('.comparison-toggle').focus({preventScroll:true});}
  }
  function setExpanded(id,open){
    const pin=pins.find(p=>p.id===id),card=[...shelf.querySelectorAll('[data-pinned]')].find(el=>el.dataset.pinned===id);
    if(!pin||!card)return;
    pin.open=open;card.querySelector('.comparison-toggle').setAttribute('aria-expanded',String(open));card.querySelector('.comparison-body').hidden=!open;
  }
  function handleClick(event){
    const action=event.target.closest('[data-ref-action]');
    if(action){
      const kind=action.dataset.refAction,id=action.dataset.refId;
      if(kind==='close')close(true);
      if(kind==='collapse')setExpanded(id,!pins.find(p=>p.id===id)?.open);
      if(kind==='back'){current=trail.pop();drawPopup();popup.querySelector('button')?.focus();}
      if(kind==='enter'){const target=current.id;close();navigate(target);}
      if(kind==='enter-pin'){close();navigate(id);}
      if(kind==='pin'){
        const target=current.id;
        if(!pins.some(p=>p.id===target)){if(pins.length>=limit)return true;pins.push({...current,open:true});renderShelf();announce('已固定 '+target);}
        showPinned(target);
      }
      if(kind==='remove'){
        const index=pins.findIndex(p=>p.id===id);if(index>=0)pins.splice(index,1);renderShelf();
        (shelf.querySelector('.comparison-toggle')||document.querySelector('#inspector-content button')||document.querySelector('#search')).focus({preventScroll:true});announce('已移除 '+id+' 对照');
      }
      if(kind==='clear'){pins.length=0;renderShelf();document.querySelector('#inspector-content button')?.focus({preventScroll:true});announce('已清空对照');}
      return true;
    }
    const ref=referenceFor(event.target);
    if(ref){open(ref,true);return true;}
    if(!popup.hidden&&!popup.contains(event.target))close();
    return false;
  }
  function handleKey(event){
    const ref=referenceFor(event.target);
    if(ref&&(event.key==='Enter'||event.key===' '||event.key==='ArrowDown')){event.preventDefault();open(ref,true);return true;}
    if(event.key==='Escape'&&!popup.hidden){event.preventDefault();close(true);return true;}
    if(event.key==='Tab'&&!popup.hidden){
      if(!popup.contains(event.target)){close();return false;}
      const buttons=[...popup.querySelectorAll('button:not([disabled]),summary')],first=buttons[0],last=buttons.at(-1);
      if((event.shiftKey&&event.target===first)||(!event.shiftKey&&event.target===last)){event.preventDefault();close(true);}
      return true;
    }
    return false;
  }
  document.addEventListener('pointerover',event=>{
    if(event.pointerType==='touch')return;
    const ref=referenceFor(event.target);
    if(popup.contains(event.target)){clearTimeout(closeTimer);return;}
    if(ref&&!ref.contains(event.relatedTarget)&&!sticky){clearTimeout(openTimer);openTimer=setTimeout(()=>open(ref),260);}
  });
  document.addEventListener('pointerout',event=>{
    const ref=referenceFor(event.target);
    if(ref&&!ref.contains(event.relatedTarget))clearTimeout(openTimer);
    if((ref||popup.contains(event.target))&&!popup.contains(event.relatedTarget)&&!origin?.contains(event.relatedTarget))scheduleClose();
  });
  document.addEventListener('focusin',event=>{
    clearTimeout(closeTimer);
    const ref=referenceFor(event.target);
    if(ref&&!popup.contains(ref)&&!sticky){clearTimeout(openTimer);openTimer=setTimeout(()=>open(ref),180);}
    else if(!ref&&!popup.contains(event.target))close();
  });
  addEventListener('resize',()=>close());
  document.addEventListener('scroll',()=>{if(!popup.hidden&&!popup.matches(':hover')){if(sticky)position();else close();}},true);
  return {close,handleClick,handleKey,clear(){close();pins.length=0;renderShelf();}};
};
