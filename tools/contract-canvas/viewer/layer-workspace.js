/* LayerWorkspace consumes canonical facts without classifying by names or domains.
 * It owns structural layer composition, exact-edge boundary disclosure and local
 * exploration snapshots. Reference peeks/pins remain with reference-preview.js. */
window.createContractLayers=function(api){
  'use strict';
  const {model,view,state,types,isMethod,nodeFor,firstPort,viewportState,setting,typeRefs,esc,svgText,renderSources,render,closeDrawers,toggleDrawer,reachable,publicFields,publicPorts,isImplementation}=api;
  const $=s=>document.querySelector(s),array=v=>Array.isArray(v)?v:[];
  const catalog=window.CONTRACT_LAYERS;
  const layers=array(catalog?.layers||model.layers),memory=new Map(),returns=[];
  const viewLayer=id=>setting(view.layers,id);
  const layer=id=>layers.find(l=>l.id===id);
  const ownership=id=>catalog?.layerForNode(id)?.id||nodeFor(id)?.layerId;
  const contains=id=>!state.layerId||ownership(id)===state.layerId;
  const ownTypes=id=>types.filter(n=>n.layerId===id&&!isImplementation(n));
  const owner=id=>{const n=nodeFor(id);return isMethod(n)?n.owner:n?.id;};
  const cut=(s,n)=>String(s??'').length>n?String(s).slice(0,n-1)+'…':String(s??'');
  const provenance=n=>n?.provenance==='proposed'?'设计提案':'源码事实';
  const kindNames={extends:'继承 extends',implements:'实现 implements',wraps:'封装 wraps',delegates:'委托 delegates',composes:'组合 composes',references:'引用 references',returns:'返回 returns',parameter:'参数 parameter',owns:'拥有 owns',variant:'分支 variant',aliases:'别名 aliases'};
  const kindLabel=e=>kindNames[e.kind]||e.kind;
  const edgePriority=e=>({wraps:0,delegates:1,extends:2,implements:3,composes:4})[e.kind]??5;
  const unorderedEdges=()=>catalog?.layerMappings(state.layerId)||model.edges.filter(e=>ownership(e.from)&&ownership(e.to)&&ownership(e.from)!==ownership(e.to)&&(ownership(e.from)===state.layerId||ownership(e.to)===state.layerId));
  const boundaryEdges=()=>[...unorderedEdges()].sort((a,b)=>edgePriority(a)-edgePriority(b));
  const related=e=>{const n=nodeFor(state.selected);return isMethod(n)?e.from===n.id||e.to===n.id:owner(e.from)===owner(n?.id)||owner(e.to)===owner(n?.id);};
  const defaultNode=id=>viewLayer(id)?.initialSubject||ownTypes(id)[0].id;
  const disclosureQuery='#inspector-content details,#layer-boundary details';
  const focusAttributes=['layerEnter','layerSelect','type','reference','refId','layer','walkthroughSubject'];
  function capture(trigger=document.activeElement){
    const {scene,selected,method,history,focus,trace,query,zoom,pan,step,branch,layerId,boundaryOpen,boundaryScope,scenarioStep}=state;
    return {scene,selected,method,history:history.map(v=>({...v})),focus,trace,query,zoom,pan:{...pan},step,branch,layerId,boundaryOpen,boundaryScope,scenarioStep,drawer:$('.navigator.open,.inspector.open')?.id,scroll:{diagram:$('#diagram-viewport').scrollTop,inspector:$('#inspector-content').scrollTop,directory:$('#type-list').scrollTop,boundary:$('.boundary-rows')?.scrollTop||0},openDetails:[...document.querySelectorAll(disclosureQuery)].flatMap((el,i)=>el.open?[i]:[]),focusAnchor:trigger&&{values:focusAttributes.map(key=>trigger.dataset[key]),edge:trigger.closest('[data-edge]')?.dataset.edge,block:trigger.closest('[data-ui-block]')?.dataset.uiBlock}};
  }
  function restore(snapshot){
    const {scroll,openDetails,focusAnchor,drawer,...view}=snapshot;
    Object.assign(state,view);$('#search').value=state.query;closeDrawers();render();
    if(drawer&&state.narrow){$('.context-bar [data-action="'+drawer+'"]')?.focus({preventScroll:true});toggleDrawer(drawer);}
    const disclosures=document.querySelectorAll(disclosureQuery);for(const i of openDetails||[])if(disclosures[i])disclosures[i].open=true;
    $('#diagram-viewport').scrollTop=snapshot.scroll?.diagram||0;$('#inspector-content').scrollTop=snapshot.scroll?.inspector||0;$('#type-list').scrollTop=snapshot.scroll?.directory||0;
    if($('.boundary-rows'))$('.boundary-rows').scrollTop=snapshot.scroll?.boundary||0;
    const previousLayerButton=focusAnchor?.values[focusAttributes.indexOf('layer')];
    const origin=(!previousLayerButton||previousLayerButton===state.layerId)&&focusAnchor?.values.some(Boolean)&&[...document.querySelectorAll('button,svg [role=button]')].find(el=>el.checkVisibility({checkVisibilityCSS:true})&&focusAttributes.every((key,i)=>el.dataset[key]===focusAnchor.values[i])&&el.closest('[data-edge]')?.dataset.edge===focusAnchor.edge&&el.closest('[data-ui-block]')?.dataset.uiBlock===focusAnchor.block);
    (origin||$('[data-layer-action="return"]')||$('[data-layer="'+CSS.escape(state.layerId)+'"]'))?.focus({preventScroll:true});
  }
  function switchLayer(id){
    if(!layer(id)||id===state.layerId)return;
    memory.set(state.layerId,capture());
    if(memory.has(id)){restore(memory.get(id));return;}
    const selected=defaultNode(id);if(!selected)return;
    restore({...viewportState(),layerId:id,selected,scene:'layer',method:firstPort(nodeFor(selected))});
  }
  function enter(id,trigger){
    const targetLayer=ownership(id);
    if(!targetLayer||targetLayer===state.layerId)return false;
    const snapshot=capture(trigger);returns.push(snapshot);if(returns.length>12)returns.shift();memory.set(state.layerId,snapshot);
    state.layerId=targetLayer;state.query='';$('#search').value='';state.boundaryOpen=false;state.boundaryScope='subject';state.history=[];
    return true;
  }
  function reset(){
    memory.clear();returns.length=0;
    const layerId=view.initial?.layerId||layers[0].id,selected=view.initial?.subjectId||defaultNode(layerId);
    Object.assign(state,{layerId,selected,scene:'layer',method:firstPort(nodeFor(selected))});
  }
  function relationMarkup(e,boundary){
    const target=ownership(e.from)===state.layerId?e.to:e.from;
    const via=e.via?`<span class="boundary-via">via <code>${esc(typeof e.via==='string'?e.via:JSON.stringify(e.via))}</code></span>`:'';
    const note=e.mapping||e.note||e.summary||e.label;
    return `<article class="boundary-row ${e.provenance==='proposed'?'is-proposed':''}" data-edge="${esc(e.id)}"><div class="boundary-edge-heading"><b class="relationship-kind kind-${esc(e.kind)}">${esc(kindLabel(e))}</b><span class="provenance-tag ${e.provenance==='proposed'?'proposed':''}">${provenance(e)}</span>${boundary?`<span class="boundary-layer-label">${esc(layer(ownership(target))?.name||ownership(target))}</span>`:''}</div><div class="boundary-endpoints"><button class="boundary-reference" data-reference="${esc(e.from)}" data-field="${esc(e.from)}" aria-haspopup="dialog" title="${esc(e.from)}">${esc(e.from)}</button><span aria-hidden="true">→</span><button class="boundary-reference" data-reference="${esc(e.to)}" data-field="${esc(e.from)}" aria-haspopup="dialog" title="${esc(e.to)}">${esc(e.to)}</button></div>${via}<div class="boundary-row-actions">${boundary?`<button class="boundary-enter" data-layer-enter="${esc(target)}">进入 ${esc(layer(ownership(target))?.name||ownership(target))} ↗</button>`:''}<details class="boundary-evidence"><summary>映射与依据 · ${array(e.sourceRefs).length}</summary>${note?`<p>${esc(typeof note==='string'?note:JSON.stringify(note))}</p>`:''}${renderSources(e)}</details></div></article>`;
  }
  function preferPrecise(edges){
    const exact=new Set(edges.filter(e=>e.kind==='extends').map(e=>JSON.stringify([e.from,e.to])));
    return edges.filter(e=>e.kind!=='composes'||!exact.has(JSON.stringify([e.from,e.to])));
  }
  function inspectorRelations(n){
    if(!layers.length)return '';
    const currentLayer=layer(n.layerId),local=preferPrecise(model.edges.filter(e=>ownership(e.from)===n.layerId&&ownership(e.to)===n.layerId&&['extends','implements','wraps','composes','delegates'].includes(e.kind)&&(owner(e.from)===owner(n.id)||owner(e.to)===owner(n.id))));
    const external=boundaryEdges().filter(related);
    return `<div class="subject-layer"><span>${esc(currentLayer?.name||n.layerId)}</span><span class="provenance-tag ${n.provenance==='proposed'?'proposed':''}">${provenance(n)}</span></div>${local.length?`<section class="inspector-section"><h3 class="section-heading">本层关系 <span>${local.length}</span></h3>${local.map(e=>relationMarkup(e,false)).join('')}</section>`:''}${external.length?`<button class="subject-boundaries" data-layer-action="subject">跨层依赖 <b>${external.length}</b> <span>↗</span></button>`:''}`;
  }
  function paint(){
    if(!layers.length){$('#layer-picker').hidden=true;$('#layer-context').hidden=true;$('#layer-boundary').hidden=true;return;}
    $('#layer-picker').innerHTML=layers.map((l,i)=>`<button class="layer-choice ${state.layerId===l.id?'active':''}" data-layer="${esc(l.id)}" aria-current="${state.layerId===l.id}" aria-label="${esc(l.name)}" title="${esc(l.name+' — '+l.summary)}"><span class="layer-index">${String(i+1).padStart(2,'0')}</span><span>${esc(viewLayer(l.id)?.label||l.name)}</span><span class="layer-choice-count">${ownTypes(l.id).length}</span></button>`).join('');
    const l=layer(state.layerId),origin=returns.at(-1);
    $('#layer-context').innerHTML=`<span class="layer-ownership"><i></i>${esc(l?.name)} <span class="layer-domain">/ ${esc(nodeFor(state.selected)?.domain||'')}</span></span><span class="provenance-tag ${l?.status==='proposed'?'proposed':''}">${l?.status==='proposed'?'设计提案 · 非生产实现':'源码关联'}</span>${origin?`<button class="layer-return" data-layer-action="return" title="恢复对象、视图、相机与滚动">← 返回 ${esc(layer(origin.layerId)?.name)} · ${esc(nodeFor(origin.selected)?.name||origin.selected)}</button>`:''}`;
    const all=boundaryEdges(),subject=all.filter(related),rows=state.boundaryScope==='all'?all:subject;
    $('#layer-boundary').classList.toggle('expanded',!!state.boundaryOpen);
    $('#layer-boundary').classList.toggle('boundary-no-rows',!rows.length);
    $('#layer-boundary').innerHTML=`<div class="boundary-toolbar"><button class="boundary-toggle" data-layer-action="toggle" aria-expanded="${!!state.boundaryOpen}" aria-controls="boundary-content"><span aria-hidden="true">${state.boundaryOpen?'⌄':'↗'}</span> 跨层关系 <b>${all.length}</b></button><div class="boundary-filters"><button data-boundary-scope="subject" aria-pressed="${state.boundaryScope!=='all'}">当前对象 ${subject.length}</button><button data-boundary-scope="all" aria-pressed="${state.boundaryScope==='all'}">整层 ${all.length}</button></div></div><div id="boundary-content" class="boundary-rows" ${state.boundaryOpen?'':'hidden'}>${(state.boundaryOpen?rows:[]).map(e=>relationMarkup(e,true)).join('')||'<div class="boundary-empty">当前对象没有跨层边。<button data-boundary-scope="all">查看整层关系 →</button></div>'}</div>`;
    document.body.dataset.layer=state.layerId;
    $('#diagram').classList.toggle('layer-diagram',state.scene==='layer');
    $('[data-scene="method"]').disabled=!isMethod(nodeFor(state.selected))&&!firstPort(nodeFor(state.selected));
  }
  function scene(){
    const narrow=state.narrow,width=narrow?390:1080,cardWidth=narrow?340:452,columns=narrow?1:2,positions=[30,30],boxes=new Map();
    const owned=ownTypes(state.layerId),cards=[];
    const preferred=viewLayer(state.layerId)?.order??[];
    const rank=id=>preferred.includes(id)?preferred.indexOf(id):preferred.length;
    owned.sort((a,b)=>rank(a.id)-rank(b.id));
    for(const [i,n] of owned.entries()){
      const column=i%columns,fields=publicFields(n).slice(0,3),ports=publicPorts(n).slice(0,2),h=100+fields.length*27+ports.length*30;
      const b={x:narrow?25:35+column*555,y:positions[column],w:cardWidth,h};positions[column]+=h+55;boxes.set(n.id,b);cards.push({n,b,fields,ports});
    }
    const structure=['extends','implements','composes','wraps'];
    const local=preferPrecise(model.edges.filter(e=>ownership(e.from)===state.layerId&&ownership(e.to)===state.layerId&&(structure.includes(e.kind)||(owned.length<=16&&['references','returns'].includes(e.kind)))&&owner(e.from)!==owner(e.to)));
    const trace=state.trace?reachable(state.selected,state.trace,true):null;
    let html='';
    for(const [i,e] of local.entries()){
      const a=boxes.get(owner(e.from)),b=boxes.get(owner(e.to));if(!a||!b)continue;
      const selected=related(e);if((state.focus&&!selected)||(trace&&(!trace.has(owner(e.from))||!trace.has(owner(e.to)))))continue;
      const forward=a.x<b.x,ax=forward?a.x+a.w:a.x,bx=forward?b.x:b.x+b.w,ay=a.y+39,by=b.y+39;
      let d,lx,ly;
      if(a.x===b.x){
        const left=narrow||a.x<100;
        const x=left?a.x:a.x+a.w,channel=x+(left?-1:1)*(9+i%4*4);
        d=`M${x},${ay} H${channel} V${by} H${x}`;lx=channel;ly=(ay+by)/2;
      }
      else{const channel=(ax+bx)/2+(i%5-2)*10;d=`M${ax},${ay} H${channel} V${by} H${bx}`;lx=channel;ly=(ay+by)/2;}
      html+=`<g class="layer-relation ${selected?'selected':''} kind-${esc(e.kind)}" data-layer-edge="${esc(e.id)}"><title>${esc(e.from+' — '+e.kind+' → '+e.to)}</title><path d="${d}" marker-end="url(#layer-${e.kind==='extends'||e.kind==='implements'?'inherit':'arrow'})"/>${!narrow&&a.x!==b.x&&(selected||structure.includes(e.kind))?svgText(lx,ly-8,e.kind,'layer-relation-label','text-anchor="middle"'):''}</g>`;
    }
    for(const {n,b,fields,ports} of cards){
      const selected=state.selected===n.id;
      html+=`<g class="layer-type ${selected?'selected':''} ${trace&&!trace.has(n.id)?'dimmed':''} ${n.provenance==='proposed'?'is-proposed':''}" data-layer-node="${esc(n.id)}"><rect class="layer-card-surface" x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}"/><g role="button" tabindex="0" data-layer-select="${esc(n.id)}" aria-label="选择 ${esc(n.name)}"><title>${esc(n.summary)}</title><rect class="layer-card-hit" x="${b.x}" y="${b.y}" width="${b.w}" height="68"/>${svgText(b.x+16,b.y+20,n.kind.toUpperCase()+' / '+provenance(n),'node-kind')}${svgText(b.x+16,b.y+46,cut(n.name,38),'layer-card-title')}</g>`;
      fields.forEach((f,i)=>{
        const y=b.y+83+i*27,refs=typeRefs(f),id=refs[0];
        html+=svgText(b.x+16,y,cut(f.name+(f.optional?'?':''),narrow?14:20),'node-field-name');
        if(id){const external=ownership(id)!==state.layerId;
          html+=`<g class="reference-row" data-reference="${esc(id)}" data-field="${esc(f.id)}" tabindex="0" role="button" aria-haspopup="dialog" aria-label="${esc(f.name+' 引用 '+id)}"><title>${esc(f.type+' / '+layer(ownership(id))?.name)}</title><rect class="row-hit" x="${b.x+(narrow?125:175)}" y="${y-17}" width="${b.w-(narrow?140:190)}" height="26"/>${svgText(b.x+(narrow?130:180),y,cut(f.type||nodes.get(id)?.name,narrow?24:35)+(refs.length>1?' +'+(refs.length-1):'')+(external?' ↗':' →'),'reference-type')}</g>`;
        }else html+=svgText(b.x+(narrow?130:180),y,cut(f.type,narrow?25:36),'node-field');
      });
      ports.forEach((p,i)=>{const y=b.y+97+fields.length*27+i*30;html+=`<g class="layer-port" data-reference="${esc(p.id)}" tabindex="0" role="button" aria-haspopup="dialog" aria-label="方法 ${esc(p.id)}"><title>${esc(p.signature||p.id)}</title><rect x="${b.x+12}" y="${y-18}" width="${b.w-24}" height="26"/>${svgText(b.x+23,y,'ƒ '+cut(p.name,34)+'()','port-title')}${svgText(b.x+b.w-25,y,'↗','port-title','text-anchor="end"')}</g>`;});
      html+=`<g role="button" tabindex="0" class="layer-card-footer" data-type="${esc(n.id)}" aria-label="展开 ${esc(n.name)} 全部字段与端口"><rect class="layer-card-hit" x="${b.x}" y="${b.y+b.h-24}" width="${b.w}" height="24"/>${svgText(b.x+16,b.y+b.h-9,`${publicFields(n).length} 字段 · ${publicPorts(n).length} 端口`,'node-kind')}${svgText(b.x+b.w-16,b.y+b.h-9,'完整契约 →','reference-type','text-anchor="end"')}</g></g>`;
    }
    return {html,width,height:Math.max(...positions)+10};
  }
  function handleClick(event){
    const t=event.target.closest('button[data-layer],[data-layer-action],[data-layer-select],[data-layer-enter],[data-boundary-scope]');if(!t)return false;
    if(t.dataset.layer){switchLayer(t.dataset.layer);return true;}
    if(t.dataset.layerSelect){const scroll=$('#diagram-viewport').scrollTop;state.selected=t.dataset.layerSelect;state.trace=null;render();$('#diagram-viewport').scrollTop=scroll;$('#diagram [data-layer-select="'+CSS.escape(state.selected)+'"]')?.focus({preventScroll:true});return true;}
    if(t.dataset.layerEnter){api.navigate(t.dataset.layerEnter,undefined,{origin:t});return true;}
    if(t.dataset.boundaryScope){state.boundaryScope=t.dataset.boundaryScope;state.boundaryOpen=true;paint();$('[data-boundary-scope="'+state.boundaryScope+'"]')?.focus({preventScroll:true});return true;}
    if(t.dataset.layerAction==='return'){const snapshot=returns.pop();if(snapshot){memory.set(state.layerId,capture());restore(snapshot);}return true;}
    if(t.dataset.layerAction==='subject'){state.boundaryScope='subject';state.boundaryOpen=true;paint();$('.boundary-toggle')?.focus();return true;}
    if(t.dataset.layerAction==='toggle'){state.boundaryOpen=!state.boundaryOpen;paint();$('.boundary-toggle')?.focus();return true;}
    return false;
  }
  return {layers,ownership,contains,scene,render:paint,handleClick,enter,reset,inspectorRelations};
};
