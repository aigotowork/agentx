/* Workspace composition; reads canonical portable artifacts/checkpoints without semantic reduction. */
(function(){
  'use strict';
  const D=globalThis.WorkbenchDOM,V=globalThis.WorkbenchViews;
  const artifact=JSON.parse(document.getElementById('workbench-model').textContent),index=D.index(artifact),byId=id=>document.getElementById(id);
  const host=byId('canvas'),details=byId('details'),navigation=byId('navigation');
  const state={view:'process',processId:artifact.definition.processes[0]?.id??'',scenarioId:'',selection:null,returnReference:null,returnPane:'primary',query:'',presentation:null};
  let projection=null,layout=null,viewport=null,linked=null,painting=false;
  function context(){const process=artifact.definition.processes.find(p=>p.id===state.processId),scenario=artifact.scenarios.find(s=>s.id===state.scenarioId);return {artifact,index,view:state.view,paneId:'primary',process,scenario,checkpoint:state.presentation?.checkpoint??null,presentation:state.presentation,selection:state.selection,onSelect:select};}
  function notice(message){byId('notice').textContent=message;}
  const player=globalThis.WorkbenchPlayer.create({onChange(presentation,applied){state.presentation=presentation;updatePlayer();if(applied&&layout){paint();if(!details.hidden)paintDetails();}else if(linked)linked.animate(presentation);}});
  layout=globalThis.WorkbenchLayout.create({host,panel:byId('layout-panel'),onChange:()=>paint(),onPause:()=>player.pause(),onNotice:notice,artifact});
  viewport=globalThis.FlowAtlasViewport.create({host,controls:byId('canvas-controls'),beforeNavigate:()=>layout.cancel()});
  linked=globalThis.WorkbenchLinked.create({host,controls:byId('canvas-controls'),beforeNavigate:()=>layout.cancel(),onChange:()=>paint()});
  function refFromNode(node){return node?.dataset.refKind?globalThis.WorkbenchLinked.reference(context(),{kind:node.dataset.refKind,id:node.dataset.refId,scope:node.dataset.refScope},node):null;}
  function setView(view){layout.cancel();state.view=view;byId('view-select').value=view;paint();}
  function updateChoices(){const process=context().process;byId('scenario-select').replaceChildren();const design=D.el('option','设计 · 无场景');design.value='';byId('scenario-select').append(design);artifact.scenarios.filter(s=>s.processId===process?.id).forEach(s=>{const option=D.el('option',s.label);option.value=s.id;byId('scenario-select').append(option);});byId('scenario-select').value=state.scenarioId;byId('scenario-select').disabled=!artifact.scenarios.some(s=>s.processId===process?.id);byId('process-select').value=state.processId;}
  function changeContext(processId,scenarioId=''){layout.cancel();if(state.processId!==processId||state.scenarioId!==scenarioId){state.selection=null;state.returnReference=null;details.hidden=true;}state.processId=processId;state.scenarioId=scenarioId;const scenario=artifact.scenarios.find(s=>s.id===scenarioId)??null;updateChoices();player.context(scenario);paintNavigation();if(!details.hidden)paintDetails();}
  function select(reference,origin){
    reference=globalThis.WorkbenchLinked.reference(context(),reference,origin);
    const entry=D.lookup(context(),reference);if(reference.kind==='scenario'&&entry){const scenario=entry.item;if(state.scenarioId!==scenario.id)changeContext(scenario.processId,scenario.id);}
    else if(['instance','expectation','visit'].includes(reference.kind)&&entry?.owner&&state.scenarioId!==entry.owner.id)changeContext(entry.owner.processId,entry.owner.id);
    else if(reference.kind==='process'&&entry&&state.processId!==entry.item.id)changeContext(entry.item.id);
    else if(['step','binding','flow'].includes(reference.kind)&&entry){const process=reference.kind==='binding'?entry.owner.process:entry.owner;if(process.id!==state.processId)changeContext(process.id);}
    state.selection=reference;if(origin&&!details.contains(origin)){state.returnReference=refFromNode(origin);state.returnPane=origin.closest('[data-pane]')?.dataset.pane??'primary';}details.hidden=false;
    if(['object','instance'].includes(reference.kind)){const nodeId=reference.kind==='instance'?reference.id:(context().scenario?.instances.find(i=>i.objectId===reference.id)?.id??reference.id);layout.select(nodeId);}
    byId('view-select').value=state.view;paint();paintDetails();byId('details-close').focus({preventScroll:true});
    linked.reveal();
    if(window.innerWidth<=700)closeNavigation(false);paintNavigation();
  }
  function paintDetails(){if(!state.selection||details.hidden)return;const result=globalThis.WorkbenchDetails.render(context(),state.selection);byId('details-title').textContent=result.title;byId('details-body').replaceChildren(result.fragment);details.dataset.refKind=state.selection.kind;details.dataset.refId=state.selection.id;details.dataset.refScope=state.selection.scope??'';}
  function closeDetails(){details.hidden=true;paint();const target=state.returnReference;const node=target?Array.from(document.querySelectorAll('[data-ref-id]')).find(n=>n!==details&&n.dataset.refKind===target.kind&&n.dataset.refId===target.id&&(!target.scope||n.dataset.refScope===target.scope)&&(!target.visitId||n.dataset.visitId===target.visitId)&&(!n.closest('[data-pane]')||n.closest('[data-pane]').dataset.pane===state.returnPane)&&!n.closest('[hidden]')):null;(node??linked.activeHost()).focus({preventScroll:true});}
  function paint(){
    if(painting)return;painting=true;
    try{
      const c=context(),positions=layout.context(state.processId,state.scenarioId,V.defaults(c),state.view==='objects');c.positions=positions;
      projection=V.render(c);host.replaceChildren(projection.svg);
      linked.render(c,projection,viewport);
      layout.mount(projection);
      byId('canvas-title').textContent=V.names[state.view];
      byId('checkpoint-label').textContent=c.scenario?'#'+(c.checkpoint?.index??0)+' · '+(c.checkpoint?.phase??'initial'):c.artifact.definition.objects.length+' 对象 · '+c.artifact.definition.operations.length+' 方法';
      byId('layout-toggle').disabled=state.view!=='objects';if(state.view!=='objects')byId('layout-panel').hidden=true;
      const diagnostics=layout.diagnostics(),status=byId('route-status');status.replaceChildren();status.hidden=!diagnostics.length;
      if(diagnostics.length){const disclosure=D.el('details'),summary=D.el('summary','布局提示 '+diagnostics.length);disclosure.append(summary,...diagnostics.map(d=>{const row=D.el('div',undefined,'diagnostic');row.dataset.diagnosticCode=d.code;row.append(D.el('span',d.message));return row;}));status.append(disclosure);}
      Object.assign(host.dataset,{view:state.view,scenarioId:state.scenarioId,checkpointIndex:String(c.checkpoint?.index??0),checkpointPhase:c.checkpoint?.phase??'design'});
    }finally{painting=false;}
  }
  function resourceRow(kind,item,scope,description){const node=D.ref(context(),kind,item.id,item.label,scope);node.className='resource-row';if(state.selection?.kind===kind&&state.selection.id===item.id&&(!scope||state.selection.scope===scope))node.setAttribute('aria-pressed','true');if(description)node.append(D.el('small',description));return node;}
  function paintNavigation(){const c=context(),root=byId('resource-list'),query=state.query.trim().toLocaleLowerCase();root.replaceChildren();const match=item=>!query||[item.id,item.label,item.responsibility,item.description].filter(Boolean).join(' ').toLocaleLowerCase().includes(query);
    function group(title,rows){if(!rows.length)return;const node=D.el('details',undefined,'resource-group');node.open=true;node.append(D.el('summary',title),...rows);root.append(node);}
    const contexts=[];for(const boundary of artifact.definition.contexts){const groupRows=[];for(const module of artifact.definition.modules.filter(m=>m.contextId===boundary.id)){const moduleRows=[];for(const object of artifact.definition.objects.filter(o=>o.moduleId===module.id)){const operations=artifact.definition.operations.filter(op=>op.ownerId===object.id),fields=object.fields.filter(match),methods=operations.filter(match);if(match(object)||methods.length||fields.length){moduleRows.push(resourceRow('object',object,undefined,object.kind));if(query){const nested=D.el('div',undefined,'resource-children');nested.append(...fields.map(f=>resourceRow('field',f,object.id)),...methods.map(op=>resourceRow('operation',op,undefined,op.kind)));moduleRows.push(nested);}}}if(moduleRows.length||match(module)){groupRows.push(resourceRow('module',module));const nested=D.el('div',undefined,'resource-children');nested.append(...moduleRows);groupRows.push(nested);}}if(groupRows.length||match(boundary)){contexts.push(resourceRow('context',boundary));const nested=D.el('div',undefined,'resource-children');nested.append(...groupRows);contexts.push(nested);}}
    group('领域与模块',contexts);group('规则',artifact.definition.rules.filter(match).map(rule=>resourceRow('rule',rule)));group('语义值',artifact.definition.values.filter(match).map(value=>resourceRow('value',value,undefined,value.unit)));group('用例',artifact.definition.processes.filter(match).map(process=>resourceRow('process',process)));group('场景',artifact.scenarios.filter(match).map(scenario=>resourceRow('scenario',scenario)));if(!root.childNodes.length)root.append(D.el('p','没有匹配的资源','muted'));
  }
  function updatePlayer(){
    const value=state.presentation??player.state(),scenario=context().scenario,cp=value.checkpoint;
    const phaseNames={call:'请求传入',effect:'拥有者处理中',reply:'回复传回',outcome:'本地结果 / 无返回',terminal:'用例结束',initial:'初态'};
    const element=byId('player');element.hidden=!scenario;
    Object.assign(element.dataset,{playing:String(value.playing),checkpointIndex:String(value.index),elapsed:String(value.elapsed),progress:String(value.progress),targetPhase:value.target?.phase??''});
    const play=byId('player-play');play.textContent=value.playing?'Ⅱ':'▶';play.setAttribute('aria-label',value.playing?'暂停':'播放');play.title=value.playing?'暂停':'播放';play.disabled=!scenario||(!value.playing&&value.index>=scenario.checkpoints.length-1);
    byId('player-previous').disabled=!scenario||value.index===0;byId('player-next').disabled=!scenario||value.index>=scenario.checkpoints.length-1;
    byId('player-position').max=String(Math.max(0,(scenario?.checkpoints.length??1)-1));byId('player-position').value=String(value.index);
    byId('player-phase').textContent='#'+value.index+' · '+(value.elapsed>0?(phaseNames[value.phase]??value.phase)+' '+Math.round(value.progress*100)+'%':'已应用 · '+(cp?.phase??'design'));
    const callId=(value.elapsed>0?value.target?.callId:null)??cp?.callId;
    // The in-flight target supplies request input; caller knowledge stays at the applied checkpoint.
    const known=cp?.calls.find(c=>c.callId===callId),upcoming=value.target?.calls.find(c=>c.callId===callId),call=known??upcoming;
    byId('player-call').textContent=call?D.label(context(),'operation',call.operationId)+' · '+(known?.phase??'pending'):'';
    byId('player-call').disabled=!call;byId('player-call').dataset.callId=call?.callId??'';
  }
  function closeNavigation(restore=true){navigation.classList.remove('is-open');byId('navigation-toggle').setAttribute('aria-expanded','false');if(restore)byId('navigation-toggle').focus({preventScroll:true});}
  artifact.definition.processes.forEach(process=>{const option=D.el('option',process.label);option.value=process.id;byId('process-select').append(option);});byId('process-select').disabled=!artifact.definition.processes.length;
  byId('model-title').textContent=artifact.label;byId('model-title').title=artifact.label;document.title=artifact.label+' · Flow Atlas';byId('diagnostic-count').textContent=String(artifact.diagnostics.length);
  byId('view-select').addEventListener('change',event=>setView(event.target.value));byId('process-select').addEventListener('change',event=>changeContext(event.target.value));byId('scenario-select').addEventListener('change',event=>changeContext(state.processId,event.target.value));
  byId('resource-search').addEventListener('input',event=>{state.query=event.target.value;paintNavigation();});byId('navigation-toggle').addEventListener('click',()=>{const opened=navigation.classList.toggle('is-open');byId('navigation-toggle').setAttribute('aria-expanded',String(opened));if(opened)byId('resource-search').focus();});byId('navigation-close').addEventListener('click',()=>closeNavigation());
  byId('details-close').addEventListener('click',closeDetails);byId('diagnostics-toggle').addEventListener('click',event=>select({kind:'diagnostics',id:'diagnostics'},event.target));byId('compare-toggle').addEventListener('click',event=>select({kind:'comparison',id:'comparison'},event.target));
  byId('layout-toggle').addEventListener('click',()=>{byId('layout-panel').hidden=!byId('layout-panel').hidden;if(!byId('layout-panel').hidden)byId('layout-panel').querySelector('button:not(:disabled)').focus();});
  byId('player-play').addEventListener('click',()=>player.state().playing?player.pause():player.play());byId('player-previous').addEventListener('click',()=>player.previous());byId('player-next').addEventListener('click',()=>player.next());byId('player-replay').addEventListener('click',()=>player.replay());byId('player-speed').addEventListener('change',event=>player.speed(Number(event.target.value)));byId('player-position').addEventListener('input',event=>player.seek(Number(event.target.value)));byId('player-expectations').addEventListener('click',event=>select({kind:'expectations',id:state.scenarioId},event.target));byId('player-call').addEventListener('click',event=>select({kind:'call',id:byId('player-call').dataset.callId},event.target));
  document.addEventListener('keydown',event=>{if(event.key!=='Escape')return;if(!byId('layout-panel').hidden){byId('layout-panel').hidden=true;byId('layout-toggle').focus();}else if(!details.hidden)closeDetails();else if(navigation.classList.contains('is-open'))closeNavigation();else{const open=document.querySelector('details[open].linked-legend,details[open].context-help');if(open){open.open=false;open.querySelector('summary').focus();}}event.preventDefault();});
  document.addEventListener('visibilitychange',()=>{if(document.hidden)player.pause();});window.addEventListener('pagehide',()=>player.pause());
  document.addEventListener('workbench-reference',event=>{if(event.detail?.reference)select(event.detail.reference,event.detail.node);});
  // Read-only detached presentation snapshot for independent acceptance/consumer inspection.
  globalThis.FlowAtlasWorkbench=Object.freeze({state(){const c=context(),p=player.state();return JSON.parse(JSON.stringify({view:state.view,processId:state.processId,scenarioId:state.scenarioId,selection:state.selection,checkpoint:c.checkpoint,presentation:{index:p.index,elapsed:p.elapsed,progress:p.progress,phase:p.phase,playing:p.playing,speed:p.speed,targetId:p.target?.id},camera:viewport.state(),linked:linked.snapshot(),layout:layout.state()}));}});
  byId('view-select').value=state.view;updateChoices();paintNavigation();player.context(null);
}());
