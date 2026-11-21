/* Small presentation composition over native controls, references and read-only state(). No clock. */
(function () {
  'use strict';
  const D=globalThis.WorkbenchDOM,B=globalThis.FlowAtlasWorkbench,byId=id=>document.getElementById(id);
  const artifact=JSON.parse(byId('workbench-model').textContent),root=byId('workbench'),navigation=byId('navigation'),inspector=byId('details'),body=byId('details-body'),rail=byId('experience-checkpoints');
  const narrow=window.matchMedia('(max-width:700px)'),phaseNames={initial:'初态',call:'请求',effect:'效果',reply:'回复',outcome:'本地结果',terminal:'终点'};
  const kindNames={object:'对象定义',instance:'场景实例',operation:'公开方法 / Port',field:'状态字段',rule:'领域规则',schema:'原生 Schema','schema-field':'Schema 字段',parameter:'命名参数',outcome:'结果分支',binding:'参数来源',process:'用例',step:'流程步骤',flow:'控制分支',scenario:'场景与预期',call:'调用证据',transition:'生命周期',relationship:'领域关系',module:'领域模块',context:'限界上下文',value:'语义值',diagnostics:'诊断与覆盖',expectations:'预期与检查',comparison:'终点比较'};
  let category='model',lastSelection='',currentEntry=null,pendingOrigin=null,returning=false,railScenario='',lastEvidence='',lastBinding='';
  const history=[],openGroups=new Map();
  function state(){return B.state();}
  function change(id,value,eventName='change'){const node=byId(id);node.value=String(value);node.dispatchEvent(new Event(eventName,{bubbles:true}));}
  function setText(id,value){const n=byId(id),text=String(value??'');if(n.textContent!==text)n.textContent=text;}
  function refKey(ref){return ref?[ref.kind,ref.id,ref.scope??'',ref.scenarioId??'',ref.visitId??'',ref.callId??'',ref.phase??''].join('|'):'';}
  function rememberOrigin(event){
    const target=event.target.closest?.('[data-ref-kind],#diagnostics-toggle,#compare-toggle,#player-expectations,#player-call');
    if(target&&target!==inspector)pendingOrigin=target;
  }
  document.addEventListener('click',rememberOrigin,true);
  document.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' ')rememberOrigin(event);},true);
  function resourceGroups(){
    const query=byId('resource-search').value.trim();
    for(const group of byId('resource-list').children){
      if(!group.matches('details'))continue;
      const summary=group.querySelector(':scope > summary'),cases=!!group.querySelector('[data-ref-kind="process"],[data-ref-kind="scenario"]');
      const key=summary.dataset.groupKey??summary.textContent;
      if(!summary.dataset.groupKey){summary.dataset.groupKey=key;const count=D.el('span',group.querySelectorAll('.resource-row').length,'group-count');summary.append(count);group.addEventListener('toggle',()=>{if(!byId('resource-search').value.trim())openGroups.set(key,group.open);});}
      group.hidden=!query&&(cases?category!=='cases':category!=='model');
      const primary=!!group.querySelector('[data-ref-kind="context"]');group.open=query?true:(openGroups.get(key)??(primary||cases));
    }
    document.querySelectorAll('[data-nav-category]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.navCategory===category)));
  }
  function enhanceNavigation(){
    for(const row of byId('resource-list').querySelectorAll('.resource-row')){
      if(row.querySelector('.resource-label'))continue;
      const label=D.el('span',undefined,'resource-label');for(const child of [...row.childNodes])if(child.nodeType===Node.TEXT_NODE)label.append(child);row.prepend(label);
      const kindLabel=row.querySelector('small'),names={entity:'实体',process:'过程',technical:'技术对象',actor:'参与者',value:'值对象',service:'服务',command:'命令',query:'查询'};
      if(kindLabel&&names[kindLabel.textContent])kindLabel.textContent=names[kindLabel.textContent];
    }
    resourceGroups();
  }
  document.querySelectorAll('[data-nav-category]').forEach(button=>button.addEventListener('click',()=>{category=button.dataset.navCategory;resourceGroups();}));
  const resourceObserver=new MutationObserver(enhanceNavigation);resourceObserver.observe(byId('resource-list'),{childList:true});
  setText('experience-resource-count',artifact.definition.objects.length+' 对象');
  byId('experience-resource-summary').append(D.el('span',artifact.definition.contexts.length+' 领域'),D.el('span',artifact.definition.modules.length+' 模块'),D.el('span',artifact.definition.operations.length+' 方法'));
  function syncWorkspace(){
    const s=state();document.querySelectorAll('button[data-view]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.view===s.view)));
    const binding=refKey(s.selection);if(s.selection?.kind==='binding'&&binding!==lastBinding){lastBinding=binding;root.dataset.bindingFocus='selected';}
    const focusButton=byId('experience-binding-focus');focusButton.hidden=s.view!=='dataflow'||s.selection?.kind!=='binding';focusButton.textContent=root.dataset.bindingFocus==='all'?'聚焦参数':'关联全图';
    setText('experience-basis',s.scenarioId?'场景快照':'定义');root.dataset.experienceView=s.view;
    syncInspector();syncEvidence();syncDrawers();
  }
  document.querySelectorAll('button[data-view]').forEach(button=>button.addEventListener('click',()=>change('view-select',button.dataset.view)));
  byId('experience-binding-focus').addEventListener('click',()=>{root.dataset.bindingFocus=root.dataset.bindingFocus==='all'?'selected':'all';change('view-select','dataflow');});
  const canvasObserver=new MutationObserver(syncWorkspace);canvasObserver.observe(byId('canvas'),{childList:true});
  function syncInspector(){
    const s=state(),key=refKey(s.selection);if(!key)return;
    if(key!==lastSelection){
      if(!returning&&currentEntry){history.push(currentEntry);if(history.length>16)history.shift();}
      currentEntry={reference:s.selection,origin:pendingOrigin};lastSelection=key;pendingOrigin=null;
    }
    byId('experience-back').disabled=!history.length;setText('experience-subject-kind',kindNames[s.selection.kind]??s.selection.kind);
    bodyObserver.disconnect();
    for(const tree of body.querySelectorAll('.schema-tree details'))if(!tree.dataset.experienceEnhanced){tree.open=false;tree.dataset.experienceEnhanced='true';}
    for(const constraints of body.querySelectorAll('.schema-tree > pre')){
      const disclosure=D.el('details',undefined,'schema-constraints');disclosure.dataset.experienceEnhanced='true';disclosure.append(D.el('summary','约束'));constraints.replaceWith(disclosure);disclosure.append(constraints);
    }
    for(const section of [...body.children]){
      if(!section.classList.contains('detail-section'))continue;
      const title=section.querySelector(':scope > h3')?.textContent??'';
      if(!/^(来源|源码链接|设计文档|扩展元数据|原生完整输出|导出的 JSON Schema|编译器派生投影)$/.test(title.split(' · ')[0]))continue;
      const disclosure=D.el('details',undefined,'inspector-disclosure'),summary=D.el('summary',title);disclosure.append(summary);section.querySelector(':scope > h3')?.remove();section.replaceWith(disclosure);disclosure.append(section);
    }
    bodyObserver.observe(body,{childList:true});syncDrawers();
  }
  const bodyObserver=new MutationObserver(syncInspector);bodyObserver.observe(body,{childList:true});
  byId('experience-back').addEventListener('click',()=>{
    const previous=history.pop();if(!previous)return;
    let origin=previous.origin?.isConnected?previous.origin:null;
    if(!origin)origin=[...document.querySelectorAll('[data-ref-kind]')].find(node=>node!==inspector&&node.dataset.refKind===previous.reference.kind&&node.dataset.refId===previous.reference.id&&(!previous.reference.scope||node.dataset.refScope===previous.reference.scope)&&(!previous.reference.visitId||node.dataset.visitId===previous.reference.visitId)&&!node.closest('[hidden]'));
    if(!origin){origin=D.ref({onSelect(reference,node){document.dispatchEvent(new CustomEvent('workbench-reference',{detail:{reference,node}}));}},previous.reference.kind,previous.reference.id,previous.reference.id,previous.reference.scope);Object.assign(origin.dataset,previous.reference);}
    returning=true;pendingOrigin=origin;origin.dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true,view:window}));syncInspector();returning=false;
    byId('experience-back').focus({preventScroll:true});
  });
  function outcomeLabel(call){
    if(!call)return '初态';if(call.phase==='pending')return '等待结果';
    if(call.phase==='unknown'){
      if(call.completion==='reply')return '已收到回复 · 结果未知';
      if(call.completion==='local-outcome')return '未知 · 无返回';
      return '未知 · 返回方式未记录';
    }
    const operation=artifact.definition.operations.find(op=>op.id===call.operationId),label=operation?.outcomes.find(o=>o.id===call.outcomeId)?.label;
    const names={returned:'已收到回复',rejected:'已收到拒绝',failed:'失败',disconnected:'连接中断'};
    return (names[call.phase]??call.phase)+(label?' / '+label:'');
  }
  function witnessBasis(scenario){const bases=[...new Set(scenario.visits.map(v=>v.evidence?.kind).filter(Boolean))];return bases.map(kind=>({authored:'声明案例',observed:'观察记录',declared:'声明'}[kind]??kind)).join(' + ')||'声明案例';}
  function syncEvidence(){
    const s=state(),scenario=artifact.scenarios.find(item=>item.id===s.scenarioId),cp=s.checkpoint,presentation=s.presentation;
    if(!scenario||!cp){railScenario='';lastEvidence='';return;}
    if(railScenario!==scenario.id){
      railScenario=scenario.id;rail.replaceChildren();setText('experience-scenario-name',scenario.label);setText('experience-witness-basis',witnessBasis(scenario));
      for(const checkpoint of scenario.checkpoints){
        const button=D.el('button',undefined,'checkpoint-button');button.type='button';button.dataset.testid='experience-checkpoint-'+checkpoint.index;button.dataset.checkpointIndex=checkpoint.index;
        button.setAttribute('aria-label','#'+checkpoint.index+' '+(phaseNames[checkpoint.phase]??checkpoint.phase)+(checkpoint.stepId?' / '+checkpoint.stepId:''));
        button.append(D.el('span',checkpoint.index.toString().padStart(2,'0'),'checkpoint-number'),D.el('span',phaseNames[checkpoint.phase]??checkpoint.phase),D.el('span',checkpoint.stepId??'—','checkpoint-step'));
        button.addEventListener('click',()=>change('player-position',checkpoint.index,'input'));rail.append(button);
      }
    }
    const call=cp.calls.find(item=>item.callId===cp.callId)??cp.calls[cp.calls.length-1];
    const activeTarget=presentation.playing||presentation.elapsed>0?scenario.checkpoints[presentation.index+1]:null;
    for(const button of rail.children){const index=Number(button.dataset.checkpointIndex);button.dataset.applied=String(index===cp.index);button.dataset.past=String(index<cp.index);button.dataset.target=String(index===activeTarget?.index);button.setAttribute('aria-pressed',String(index===cp.index));if(index===cp.index)button.setAttribute('aria-current','step');else button.removeAttribute('aria-current');}
    byId('experience-caller-value').parentElement.dataset.meaning=call?.phase??'initial';setText('experience-caller-value',outcomeLabel(call));
    const effect=call?.effectOutcomeId,operation=artifact.definition.operations.find(op=>op.id===call?.operationId),effectName=operation?.outcomes.find(o=>o.id===effect)?.label??effect;
    byId('experience-effect-value').parentElement.dataset.meaning=effect?'applied':'initial';setText('experience-effect-value',effect?'已提交 / '+effectName:call?'当前未记录':'尚无记录');
    setText('experience-applied','#'+cp.index+' 已应用'+(activeTarget?' · 示意 → '+(phaseNames[activeTarget.phase]??activeTarget.phase):''));
    const evidenceKey=scenario.id+'/'+cp.index;if(lastEvidence!==evidenceKey){lastEvidence=evidenceKey;const current=rail.querySelector('[data-applied="true"]');if(current&&narrow.matches)current.scrollIntoView({block:'nearest',inline:'nearest',behavior:'instant'});}
    root.dataset.experienceCheckpoint=cp.id;
  }
  const playerObserver=new MutationObserver(syncEvidence);playerObserver.observe(byId('player'),{attributes:true,attributeFilter:['data-checkpoint-index','data-playing','data-target-phase','hidden']});
  function syncDrawers(){
    const navOpen=narrow.matches&&navigation.classList.contains('is-open'),detailsOpen=narrow.matches&&!inspector.hidden;
    for(const [drawer,opened] of [[navigation,navOpen],[inspector,detailsOpen]]){if(opened){drawer.setAttribute('role','dialog');drawer.setAttribute('aria-modal','true');}else{drawer.removeAttribute('role');drawer.removeAttribute('aria-modal');}}
  }
  const drawerObserver=new MutationObserver(syncDrawers);drawerObserver.observe(inspector,{attributes:true,attributeFilter:['hidden']});drawerObserver.observe(navigation,{attributes:true,attributeFilter:['class']});narrow.addEventListener('change',syncDrawers);
  byId('navigation-toggle').addEventListener('click',()=>{if(narrow.matches&&!inspector.hidden)byId('details-close').click();});
  document.addEventListener('keydown',event=>{
    if(event.key==='/'&&!event.ctrlKey&&!event.metaKey&&!event.altKey&&!event.target.closest('input,textarea,select,[contenteditable]')){if(narrow.matches&&!navigation.classList.contains('is-open'))byId('navigation-toggle').click();byId('resource-search').focus();event.preventDefault();return;}
    if(event.key!=='Tab'||!narrow.matches)return;
    const drawer=navigation.classList.contains('is-open')?navigation:!inspector.hidden?inspector:null;if(!drawer||!drawer.contains(document.activeElement))return;
    const focusables=[...drawer.querySelectorAll('button:not(:disabled),input:not(:disabled),select:not(:disabled),summary,a,[tabindex="0"]')].filter(node=>!node.closest('[hidden]')&&node.getClientRects().length);
    const first=focusables[0],last=focusables[focusables.length-1];if(event.shiftKey&&document.activeElement===first){last?.focus();event.preventDefault();}else if(!event.shiftKey&&document.activeElement===last){first?.focus();event.preventDefault();}
  },true);
  // One visual icon vocabulary; native controls retain their original event handlers and names.
  const iconPaths={
    'navigation-toggle':'M4 6h16M4 12h16M4 18h16','navigation-close':'m6 6 12 12M18 6 6 18','details-close':'m6 6 12 12M18 6 6 18',
    'layout-toggle':'M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z',
    'player-previous':'M5 5v14M18 5l-9 7 9 7z','player-next':'M19 5v14M6 5l9 7-9 7z','player-replay':'M4 11a8 8 0 1 1 2 7M4 4v7h7'
  };
  for(const [id,path]of Object.entries(iconPaths)){const svg=D.svg('svg',{viewBox:'0 0 24 24',class:'control-icon','aria-hidden':'true'});svg.append(D.svg('path',{d:path}));byId(id).replaceChildren(svg);}
  const tooltip=byId('experience-tooltip');let tooltipOwner=null;
  function showTooltip(target){
    const owner=target.closest?.('[title]');if(!owner||!owner.getAttribute('title'))return;
    tooltipOwner=owner;tooltip.textContent=owner.getAttribute('title');tooltip.hidden=false;owner.setAttribute('aria-describedby','experience-tooltip');
    const rect=owner.getBoundingClientRect(),style=getComputedStyle(root),gap=parseFloat(style.getPropertyValue('--ex-s2')),max=parseFloat(style.getPropertyValue('--ex-tooltip-width'));
    tooltip.style.setProperty('--ex-tooltip-x',Math.max(gap,Math.min(rect.left,window.innerWidth-max-gap))+'px');tooltip.style.setProperty('--ex-tooltip-y',Math.min(window.innerHeight-tooltip.offsetHeight-gap,rect.bottom+gap)+'px');
  }
  function hideTooltip(){tooltip.hidden=true;tooltipOwner?.removeAttribute('aria-describedby');tooltipOwner=null;}
  document.addEventListener('pointerover',event=>{if(event.pointerType!=='touch')showTooltip(event.target);});
  document.addEventListener('pointerout',hideTooltip);document.addEventListener('focusin',event=>showTooltip(event.target));document.addEventListener('focusout',hideTooltip);
  document.addEventListener('pointerdown',event=>{hideTooltip();if(event.pointerType==='touch')showTooltip(event.target);});
  document.addEventListener('keydown',event=>{if(event.key==='Escape')hideTooltip();});
  enhanceNavigation();syncWorkspace();
}());
