/* Contract walkthrough: read canonical checkpoints and select their exact anchors. */
window.createContractWalkthrough=function(api){
  'use strict';
  const {model,view,state,nodeFor,fieldFor,isMethod,esc,render}=api;
  const binding=view.walkthrough;
  const scenario=model.scenarios.find(s=>s.id===binding?.scenarioId);
  const $=s=>document.querySelector(s),element=$('#contract-walkthrough');
  const steps=scenario?.steps||[];
  const tab=binding?.tab||scenario?.name;
  const label=(step,index)=>binding?.stepTitles?.[index]||step.label;
  const available=()=>!!scenario&&state.layerId===binding.layerId;
  const current=()=>steps[state.scenarioStep]||steps[0];
  const ownIds=step=>step.nodeIds.filter(id=>nodeFor(id)?.layerId===state.layerId);
  function selectStep(index){
    if(!steps[index])return;
    state.scenarioStep=index;const ids=ownIds(steps[index]);
    state.selected=ids[0]||steps[index].nodeIds[0];state.trace=null;
    state.history=[{id:state.selected,scene:'scenario',label:tab}];
    render();const checkpoint=$('#walkthrough-step-'+index);checkpoint?.focus({preventScroll:true});checkpoint?.scrollIntoView({block:'nearest',inline:'nearest'});
  }
  function contractEntry(id){
    const n=nodeFor(id);if(!n)return '';
    const method=isMethod(n),selected=id===state.selected;
    const external=n.layerId!==state.layerId;
    const field=fieldFor(id);
    const detail=field?field.type+' · '+(field.summary||''):method?n.signature:n.summary;
    const title=id!==n.id?id:method?n.id:n.name||n.id;
    const select=external?`data-reference="${esc(id)}" aria-haspopup="dialog"`:`data-walkthrough-subject="${esc(id)}" aria-pressed="${selected}"`;
    return `<li class="walkthrough-contract ${selected?'selected':''}" data-walkthrough-contract="${esc(id)}"><button class="walkthrough-subject" ${select}><span class="walkthrough-kind ${method?'method':''}">${esc(field?'FIELD':method?'ƒ PORT':n.kind.toUpperCase())}</span><strong>${esc(title)}</strong>${detail?`<span class="walkthrough-contract-detail" title="${esc(detail)}">${esc(detail)}</span>`:''}</button>${!external?`<button class="icon-button walkthrough-peek" data-reference="${esc(id)}" aria-haspopup="dialog" title="预览 ${esc(title)}" aria-label="预览 ${esc(title)}">↗</button>`:''}</li>`;
  }
  function draw(){
    const step=current(),active=state.scene==='scenario'&&!!step;
    const trigger=$('#application-scenario');
    trigger.hidden=!available();
    if(tab)trigger.textContent=tab;
    trigger.classList.toggle('active',active);
    element.hidden=!active;
    if(!active){element.innerHTML='';return;}
    const inside=ownIds(step),outside=step.nodeIds.filter(id=>!inside.includes(id));
    element.innerHTML=`<nav class="walkthrough-steps" style="--checkpoint-count:${steps.length}" aria-label="应用契约步骤">${steps.map((s,i)=>`<button id="walkthrough-step-${i}" data-walkthrough-step="${i}" aria-current="${i===state.scenarioStep?'step':'false'}" title="${esc(s.label)}"><span>${String(i+1).padStart(2,'0')}</span> ${esc(label(s,i))}</button>`).join('')}</nav><div class="walkthrough-checkpoint"><div class="walkthrough-heading"><h2>${esc(label(step,state.scenarioStep))}</h2><span class="provenance-tag ${scenario.provenance==='proposed'?'proposed':''}">${scenario.provenance==='proposed'?'设计提案':'源码事实'}</span></div><p class="walkthrough-label">${esc(step.label)}</p><ul aria-label="本层契约" class="walkthrough-contracts">${inside.map(contractEntry).join('')}</ul>${outside.length?`<details class="walkthrough-external"><summary>跨层契约 · ${outside.length}</summary><ul class="walkthrough-contracts">${outside.map(contractEntry).join('')}</ul></details>`:''}<div class="walkthrough-actions"><button data-walkthrough-step="${state.scenarioStep-1}" ${state.scenarioStep===0?'disabled':''}>← 上一步</button><button data-walkthrough-step="${state.scenarioStep+1}" ${state.scenarioStep===steps.length-1?'disabled':''}>下一步 →</button></div></div>`;
  }
  function handleClick(event){
    const button=event.target.closest('[data-walkthrough-action],[data-walkthrough-step],[data-walkthrough-subject]');if(!button)return false;
    if(button.dataset.walkthroughAction==='open'&&available()){state.scene='scenario';state.boundaryOpen=false;selectStep(state.scenarioStep);$('#diagram-viewport').scrollTop=0;return true;}
    if(button.dataset.walkthroughStep!==undefined){selectStep(Number(button.dataset.walkthroughStep));return true;}
    if(button.dataset.walkthroughSubject){const id=button.dataset.walkthroughSubject,scroll=$('#diagram-viewport').scrollTop;state.selected=id;state.trace=null;state.history=[{id,scene:'scenario',label:tab}];render();$('#diagram-viewport').scrollTop=scroll;$('[data-walkthrough-subject="'+CSS.escape(id)+'"]')?.focus({preventScroll:true});return true;}
    return false;
  }
  return {scenario,available,render:draw,handleClick,stepCount:steps.length};
};
