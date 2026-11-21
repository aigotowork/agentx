/* World-coordinate object gestures and explicit presentation-only JSON exchange. */
(function (global) {
  'use strict';
  const engine=global.FlowAtlasLayout,D=global.WorkbenchDOM;
  /** Shared pure engine owns positions/history; this adapter owns borrowed DOM gestures. */
  function create({host,panel,onChange,onPause,onNotice,artifact}) {
    const sessions=new Map(),digest=engine.identity(artifact);let key=null,session=null,selected=null,enabled=false,gesture=null,scene=null,suppressClick=false;
    const title=D.el('p','对象布局','muted'),actions=D.el('div',undefined,'layout-actions'),json=D.el('textarea',undefined,'layout-json');
    json.setAttribute('aria-label','布局 JSON');json.dataset.testid='layout-json';json.spellcheck=false;
    function button(id,text,action){const node=D.el('button',text,'compact-button');node.type='button';node.dataset.testid=id;node.addEventListener('click',action);actions.append(node);return node;}
    function persist(){if(session)sessions.set(key,session);}
    function changed(){persist();refresh();onChange();}
    function cancel(){if(!gesture)return;session=engine.cancel(session);const previous=gesture;gesture=null;persist();if(host.hasPointerCapture(previous.pointerId))host.releasePointerCapture(previous.pointerId);onChange();}
    function mutate(action){if(!enabled||!session)return;cancel();onPause();session=action(session);changed();}
    const pin=button('layout-pin','固定',()=>{if(selected)mutate(s=>engine.togglePin(s,selected));});
    const undo=button('layout-undo','撤销',()=>mutate(engine.undo));button('layout-reset','重置',()=>mutate(engine.reset));
    const deltas={left:[-10,0],up:[0,-10],down:[0,10],right:[10,0]};
    for(const [direction,delta]of Object.entries(deltas))button('layout-'+direction,{left:'←',up:'↑',down:'↓',right:'→'}[direction],()=>move(delta));
    button('layout-export','导出 JSON',()=>{if(session){json.value=JSON.stringify(engine.exportDocument(session),null,2);json.focus();json.select();}});
    button('layout-import','导入 JSON',()=>{if(!session)return;try{const imported=engine.importDocument(session,json.value);onPause();session=imported;changed();onNotice('布局已导入');}catch(error){onNotice(error.message);}});
    panel.replaceChildren(title,actions,json);
    function refresh(){title.textContent=enabled?'对象布局 · '+(selected??'选择对象标题'):'在对象与关系视图调整布局';actions.querySelectorAll('button').forEach(node=>node.disabled=!enabled);pin.disabled=!enabled||!selected;pin.textContent=selected&&session&&engine.pinned(session,selected)?'取消固定':'固定';undo.disabled=!enabled||!session?.history.length;json.disabled=!enabled;}
    function context(processId,scenarioId,positions,isEnabled){enabled=isEnabled;const next=processId+'/'+(scenarioId||'design');if(key!==next){cancel();key=next;selected=null;session=sessions.get(key)??engine.create({contentDigest:digest,scope:{view:'relations',modeId:processId||'definition',scenarioId:scenarioId||'design'},positions});persist();}refresh();return session?.document.positions??positions;}
    function point(event){const root=host.querySelector('svg');if(!root?.getScreenCTM())return null;return new DOMPoint(event.clientX,event.clientY).matrixTransform(root.getScreenCTM().inverse());}
    function move(delta){if(!selected||!enabled||!session||engine.pinned(session,selected))return;mutate(s=>{const p=s.document.positions[selected];return engine.move(s,selected,{x:p.x+delta[0],y:p.y+delta[1]});});}
    function down(event){if(!enabled||event.button!==0)return;const handle=event.target.closest('[data-layout-handle]');if(!handle)return;selected=handle.dataset.nodeId;refresh();if(!Object.hasOwn(session.document.positions,selected)||engine.pinned(session,selected))return;const origin=point(event);if(!origin)return;gesture={pointerId:event.pointerId,nodeId:selected,origin,start:{...session.document.positions[selected]},client:{x:event.clientX,y:event.clientY},moved:false};host.setPointerCapture(event.pointerId);event.preventDefault();event.stopPropagation();}
    function pointermove(event){if(!gesture||event.pointerId!==gesture.pointerId)return;if(!gesture.moved&&Math.hypot(event.clientX-gesture.client.x,event.clientY-gesture.client.y)<4)return;const current=point(event);if(!current)return;if(!gesture.moved){onPause();session=engine.begin(session,gesture.nodeId);gesture.moved=true;}session=engine.update(session,{x:gesture.start.x+current.x-gesture.origin.x,y:gesture.start.y+current.y-gesture.origin.y});persist();onChange();event.preventDefault();}
    function up(event){if(!gesture||event.pointerId!==gesture.pointerId)return;const previous=gesture;gesture=null;if(previous.moved){session=engine.commit(session);suppressClick=true;changed();}if(host.hasPointerCapture(event.pointerId))host.releasePointerCapture(event.pointerId);event.preventDefault();}
    host.addEventListener('pointerdown',down);host.addEventListener('pointermove',pointermove);host.addEventListener('pointerup',up);
    host.addEventListener('pointercancel',cancel);host.addEventListener('lostpointercapture',()=>{if(gesture)cancel();});window.addEventListener('blur',cancel);
    host.addEventListener('click',event=>{if(suppressClick){suppressClick=false;event.preventDefault();event.stopImmediatePropagation();}},true);
    host.addEventListener('keydown',event=>{if(event.key==='Escape'&&gesture){cancel();event.preventDefault();event.stopImmediatePropagation();return;}const handle=event.target.closest('[data-layout-handle]');if(!enabled||!handle)return;selected=handle.dataset.nodeId;const arrows={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,-1],ArrowDown:[0,1]};if(arrows[event.key])move(arrows[event.key].map(n=>n*(event.shiftKey?1:10)));else if(event.key.toLowerCase()==='p')mutate(s=>engine.togglePin(s,selected));else return;refresh();Array.from(host.querySelectorAll('[data-layout-handle]')).find(node=>node.dataset.nodeId===selected)?.focus({preventScroll:true});event.preventDefault();event.stopImmediatePropagation();},true);
    return Object.freeze({context,cancel,select(id){selected=id;refresh();},mount(projection){scene=projection;for(const handle of host.querySelectorAll('[data-layout-handle]'))handle.dataset.pinned=String(session&&engine.pinned(session,handle.dataset.nodeId));},positions(){return session?.document.positions;},state(){return session?engine.exportDocument(session):null;},diagnostics(){if(!scene||!enabled)return [];return [...engine.overlaps(scene.boxes),...scene.routes.flatMap(route=>engine.routeDiagnostics(route.id,route.points,scene.boxes))];}});
  }
  global.WorkbenchLayout = Object.freeze({create});
}(globalThis));
