/* Pane composition contract: one app context/player, independent presentation cameras.
 * create({host,controls,beforeNavigate,onChange}) -> {primaryHost,render,animate,reveal,snapshot}.
 * render(context,primaryProjection) never reduces facts; snapshot() returns detached presentation state.
 * See .controller/domain-daemon/tasks/flow-atlas-linked-ui.md for hooks and ownership.
 */
(function(global){
  'use strict';
  const D=global.WorkbenchDOM,byId=id=>document.getElementById(id);
  /** Add occurrence identity to native references without manufacturing a visit for an ambiguous step. */
  function reference(c,ref,origin){
    const result={...ref},entry=D.lookup(c,ref);
    for(const key of ['processId','stepId','scenarioId','visitId','callId','phase'])if(origin?.dataset[key]&&!result[key])result[key]=origin.dataset[key];
    let stepId=ref.kind==='step'?ref.id:ref.kind==='binding'?entry?.owner.step.id:result.stepId;
    const visit=ref.kind==='visit'?entry?.item:c.scenario?.visits.find(v=>result.visitId?v.id===result.visitId:result.callId?v.callId===result.callId:ref.kind==='call'&&v.callId===ref.id);
    if(visit){stepId=visit.stepId;result.visitId=visit.id;result.callId=visit.callId;result.scenarioId=entry?.kind==='visit'?entry.owner.id:c.scenario.id;}
    if(stepId){
      result.processId=ref.kind==='step'?entry?.owner.id??c.process?.id:ref.kind==='binding'?entry?.owner.process.id??c.process?.id:ref.kind==='visit'?entry?.owner.processId:c.process?.id;result.stepId=stepId;
      // References followed inside the inspector retain the exact occurrence of that same step.
      if(!result.visitId&&origin?.closest('#details')&&c.selection?.stepId===stepId){for(const key of ['scenarioId','visitId','callId','phase'])if(c.selection[key])result[key]=c.selection[key];}
      const visits=c.scenario?.processId===result.processId?c.scenario.visits.filter(v=>v.stepId===stepId):[];
      if(!result.visitId&&visits.length===1){result.scenarioId=c.scenario.id;result.visitId=visits[0].id;result.callId=visits[0].callId;}
      if(result.visitId&&!result.phase)result.phase=c.checkpoint?.calls.find(call=>call.callId===result.callId&&call.visitId===result.visitId)?.phase??'call';
    }
    return result;
  }
  function focus(c){
    const ref=c.selection;if(!ref)return null;
    const result={processId:ref.processId??c.process?.id};for(const key of ['stepId','scenarioId','visitId','callId','phase'])if(ref[key])result[key]=ref[key];
    if(ref.kind==='step')result.stepId=ref.id;
    return result;
  }
  /** Composition only. The caller retains primary layout/camera and the unique player. */
  function create({host,controls,beforeNavigate,onChange}){
    const section=host.closest('.canvas-section'),root=byId('workbench'),narrow=matchMedia('(max-width:1000px)');
    let mode='paired',secondaryView='sequence',activePane='primary',current=null,secondaryProjection=null,primaryProjection=null,primaryViewport=null,lastFocus='',latestPresentation=null;
    const entered=new Set(),mountedScopes=new Map(),renderedVisible=new Set();section.classList.add('linked-workspace');root.classList.add('has-linked-workspace');
    function selectControl(id,label,options,value){
      const wrapper=D.el('label',undefined,'linked-choice'),name=D.el('span',label),select=D.el('select');select.id=id;select.dataset.testid=id;select.setAttribute('aria-label',label);
      for(const [key,title]of options){const option=D.el('option',title);option.value=key;select.append(option);}select.value=value;wrapper.append(name,select);return {wrapper,select};
    }
    const toolbar=D.el('div',undefined,'linked-toolbar'),modeControl=selectControl('linked-mode','布局',[['paired','双窗'],['single','单窗']],mode),secondaryControl=selectControl('linked-secondary-view','辅助',[['sequence','场景时序'],['state','对象状态']],secondaryView),focusLabel=D.el('span','未选择步骤','linked-focus');
    focusLabel.dataset.testid='linked-focus';focusLabel.id='linked-focus';focusLabel.setAttribute('role','status');
    const tabs=D.el('div',undefined,'linked-pane-tabs');tabs.setAttribute('role','tablist');tabs.setAttribute('aria-label','工作窗');
    function tab(pane,title){const button=D.el('button',title,'compact-button');button.id='linked-show-'+pane;button.dataset.testid=button.id;button.type='button';button.setAttribute('role','tab');button.setAttribute('aria-controls','linked-pane-'+pane);button.addEventListener('click',()=>{activePane=pane;visibility();renderCurrent();button.focus({preventScroll:true});});tabs.append(button);return button;}
    const primaryTab=tab('primary','协作'),secondaryTab=tab('secondary','时序');
    const legend=D.el('details',undefined,'linked-legend'),legendToggle=D.el('summary','ⓘ'),legendBody=D.el('div');legendToggle.setAttribute('aria-label','分支与场景路径图例');legendToggle.title='分支与场景路径图例';
    for(const [kind,title]of [['declared','声明分支'],['path','场景路径'],['applied','已应用前缀']]){const row=D.el('span',title);row.dataset.legendKind=kind;legendBody.append(row);}legend.append(legendToggle,legendBody);
    toolbar.append(modeControl.wrapper,secondaryControl.wrapper,tabs,focusLabel,legend);
    const deck=D.el('div',undefined,'linked-deck');
    function pane(id,title){const pane=D.el('section',undefined,'linked-pane'),heading=D.el('div',undefined,'linked-pane-heading'),label=D.el('strong',title),checkpoint=D.el('span',undefined,'muted');pane.id='linked-pane-'+id;pane.dataset.testid=pane.id;pane.dataset.pane=id;pane.setAttribute('aria-label',title);heading.append(label,checkpoint);pane.append(heading);deck.append(pane);return {root:pane,heading,label,checkpoint};}
    const primary=pane('primary','协作流程'),secondary=pane('secondary','场景时序'),secondaryHost=D.el('div',undefined,'canvas'),secondaryControls=D.el('div');secondaryHost.dataset.testid='linked-canvas-secondary';secondaryControls.className='linked-viewport-controls';
    host.before(toolbar,deck);primary.root.append(host,controls);secondary.root.append(secondaryHost,secondaryControls);
    primary.heading.replaceChildren(byId('canvas-title'),byId('checkpoint-label'));primary.label=byId('canvas-title');primary.checkpoint=byId('checkpoint-label');
    function payloadCue(pane){
      const disclosure=D.el('details',undefined,'linked-payload'),summary=D.el('summary'),body=D.el('div',undefined,'linked-payload-body'),caption=D.el('strong'),payload=D.el('pre');
      disclosure.hidden=true;disclosure.dataset.testid='linked-payload-cue';payload.dataset.testid='flow-payload';body.append(caption,payload);disclosure.append(summary,body);pane.checkpoint.before(disclosure);
      disclosure.addEventListener('keydown',event=>{if(event.key==='Escape'&&disclosure.open){disclosure.open=false;summary.focus({preventScroll:true});event.preventDefault();event.stopPropagation();}});
      return {disclosure,summary,caption,payload};
    }
    const payloadCues=[payloadCue(primary),payloadCue(secondary)];
    const secondaryViewport=global.FlowAtlasViewport.create({host:secondaryHost,controls:secondaryControls,beforeNavigate});
    function visibility(){
      root.dataset.linkedMode=mode;root.dataset.linkedActivePane=activePane;root.dataset.linkedNarrow=String(narrow.matches);
      primary.root.hidden=narrow.matches&&mode==='paired'&&activePane!=='primary';secondary.root.hidden=mode==='single'||(narrow.matches&&activePane!=='secondary');
      secondaryControl.wrapper.hidden=mode==='single';tabs.hidden=mode==='single';
      for(const [id,button]of [['primary',primaryTab],['secondary',secondaryTab]]){button.setAttribute('aria-selected',String(activePane===id));button.tabIndex=activePane===id?0:-1;}
    }
    tabs.addEventListener('keydown',event=>{if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;event.preventDefault();(event.key==='Home'?primaryTab:event.key==='End'?secondaryTab:activePane==='primary'?secondaryTab:primaryTab).click();});
    modeControl.select.addEventListener('change',()=>{beforeNavigate();mode=modeControl.select.value;if(mode==='single')activePane='primary';visibility();onChange();});
    secondaryControl.select.addEventListener('change',()=>{beforeNavigate();secondaryView=secondaryControl.select.value;visibility();onChange();});
    narrow.addEventListener('change',()=>{visibility();renderCurrent();});
    function mount(pane,projection,viewport){
      if(pane.root.hidden)return;
      const c=projection.context,key=pane.root.dataset.pane+'/'+c.view+'/'+c.process?.id+'/'+(c.scenario?.id??'design');viewport.mount(projection.svg,key);mountedScopes.set(pane.root.dataset.pane,key);
      if(entered.has(key))return;entered.add(key);
      // Start with readable text and the participant/owner headers, not a deep reveal of a long message.
      if(viewport.state().zoom<0.85){pane.root.querySelector('[data-testid="viewport-actual"]').click();const first=projection.svg.querySelector('[data-responsibility-lane]')??projection.nodes.values().next().value;if(first)viewport.reveal(first);}
    }
    function renderCurrent(){if(current&&primaryProjection)render({...current,presentation:latestPresentation??current.presentation},primaryProjection,primaryViewport);}
    function render(c,p,viewport){
      current=c;primaryProjection=p;primaryViewport=viewport;visibility();
      const newlyVisible=[];for(const pane of [primary,secondary]){const id=pane.root.dataset.pane;if(pane.root.hidden)renderedVisible.delete(id);else{if(!renderedVisible.has(id))newlyVisible.push(pane);renderedVisible.add(id);}}
      for(const [pane,view]of [[primary,c.view],[secondary,secondaryView]])Object.assign(pane.root.dataset,{view,scenarioId:c.scenario?.id??'',checkpointIndex:String(c.checkpoint?.index??0)});
      primaryTab.textContent=c.view==='process'?'协作':global.WorkbenchViews.names[c.view];secondaryTab.textContent=secondaryView==='sequence'?'时序':'状态';
      secondary.label.textContent=secondaryView==='sequence'?'场景时序':'对象状态';secondary.checkpoint.textContent=c.scenario?'#'+(c.checkpoint?.index??0)+' 已应用':'选择场景';
      mount(primary,p,viewport);
      secondaryProjection=global.WorkbenchViews.render({...c,view:secondaryView,paneId:'secondary'});secondaryHost.replaceChildren(secondaryProjection.svg);mount(secondary,secondaryProjection,secondaryViewport);
      const f=focus(c),step=c.process?.steps.find(s=>s.id===f?.stepId);focusLabel.textContent=step?(step.label+(f.visitId?' / '+f.visitId:'')):'未选择步骤';focusLabel.title=focusLabel.textContent;
      animate(c.presentation??{progress:0});const key=JSON.stringify(f);if(key!==lastFocus){lastFocus=key;reveal();}else for(const pane of newlyVisible)revealPane(pane);
    }
    function target(p){
      const f=focus(current),ref=current?.selection;if(!p||!ref)return null;
      const marks=[...p.svg.querySelectorAll('[data-ref-id], [data-step-id]')];
      return (f?.visitId?marks.find(n=>n.dataset.visitId===f.visitId&&(!f.callId||n.dataset.callId===f.callId)):null)??(f?.stepId?marks.find(n=>n.dataset.stepId===f.stepId):null)??marks.find(n=>n.dataset.refKind===ref.kind&&n.dataset.refId===ref.id&&(!ref.scope||n.dataset.refScope===ref.scope));
    }
    function revealPane(pane){const isPrimary=pane===primary,p=isPrimary?primaryProjection:secondaryProjection,viewport=isPrimary?primaryViewport:secondaryViewport,node=target(p);if(node&&!pane.root.hidden)viewport?.reveal(node);}
    function reveal(){revealPane(primary);revealPane(secondary);}
    function updatePayload(presentation){
      const target=presentation.target,call=target?.calls.find(c=>c.callId===target.callId),phase=target?.phase,active=presentation.progress>0&&(phase==='call'||(phase==='reply'&&call?.completion==='reply'));
      const title=phase==='call'?'示意请求参数':'示意回复数据',value=D.json(phase==='call'?call?.input:call?.payload),key=(target?.callId??'')+'/'+(target?.visitId??'')+'/'+phase;
      for(const cue of payloadCues){cue.disclosure.hidden=!active;if(!active){cue.disclosure.open=false;continue;}if(cue.disclosure.dataset.payloadKey!==key)cue.disclosure.open=false;Object.assign(cue.disclosure.dataset,{payloadKey:key,callId:target.callId??'',visitId:target.visitId??'',phase});cue.summary.textContent=phase==='call'?'⇢ 参数':'⇠ 数据';cue.summary.setAttribute('aria-label',title+' · '+(target.callId??''));cue.summary.title=title;cue.caption.textContent=title+' · '+(target.callId??'');if(cue.payload.textContent!==value)cue.payload.textContent=value;}
    }
    function animate(presentation){latestPresentation=presentation;if(primaryProjection&&!primary.root.hidden)global.WorkbenchViews.animate(primaryProjection,presentation);if(secondaryProjection&&!secondary.root.hidden)global.WorkbenchViews.animate(secondaryProjection,presentation);updatePayload(presentation);}
    function snapshot(){
      const f=current?focus(current):null;
      const pane=(element,view,viewport)=>{const key=element.dataset.pane+'/'+view+'/'+current?.process?.id+'/'+(current?.scenario?.id??'design');return {view,visible:!element.hidden,camera:mountedScopes.get(element.dataset.pane)===key?viewport?.state()??null:null,focus:f,scenarioId:current?.scenario?.id??'',checkpointIndex:current?.checkpoint?.index??0};};
      return {mode,secondaryView,activePane,focus:f,panes:{primary:pane(primary.root,current?.view??'process',primaryViewport),secondary:pane(secondary.root,secondaryView,secondaryViewport)}};
    }
    visibility();return Object.freeze({primaryHost:host,render,animate,reveal,snapshot,activeHost(){return activePane==='secondary'?secondaryHost:host;}});
  }
  global.WorkbenchLinked=Object.freeze({create,reference,focus});
}(globalThis));
