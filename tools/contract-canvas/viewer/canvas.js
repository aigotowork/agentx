/* ContractWorkbench: a local, source-linked reading surface. No service calls. */
(() => {
  'use strict';
  const model=window.CONTRACT_MODEL,view=window.CONTRACT_VIEW;
  if(!model||!view){document.querySelector('#empty-model').textContent='契约模型尚未接入 · 请保留生成的数据文件及画布资源';return;}
  const $=selector=>document.querySelector(selector);
  const esc=value=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const array=value=>Array.isArray(value)?value:[];
  const nodes=new Map(model.nodes.map(n=>[n.id,n]));
  const methods=model.nodes.filter(n=>n.kind==='method');
  const isMethod=n=>n?.kind==='method';
  const types=model.nodes.filter(n=>!isMethod(n));
  const sources=new Map(array(model.sources).map(s=>[s.id,s]));
  const fieldOwners=new Map(model.nodes.flatMap(n=>array(n.fields).map(f=>[f.id,n.id])));
  const setting=(record,key)=>record&&Object.hasOwn(record,key)?record[key]:undefined;
  const domainColors=['blue','teal','violet','amber'];
  const colorValues={blue:'#3764db',teal:'#197e7b',violet:'#7951b7',amber:'#ab6d19'};
  const domains=[...new Set(types.map(n=>n.domain))];
  const accent=id=>colorValues[setting(view.domains,id)?.color||domainColors[Math.max(0,domains.indexOf(id))%4]];
  const viewportState=()=>({history:[],focus:false,trace:null,query:'',zoom:1,pan:{x:0,y:0},step:0,branch:null,boundaryOpen:false,boundaryScope:'subject',scenarioStep:0});
  const state={...viewportState(),scene:'layer',narrow:innerWidth<=760};
  const overview=view.overview,sequence=view.sequence;
  const sceneNames={layer:'层模型',scenario:view.walkthrough?.tab||'契约路径',overview:'契约总览',type:'类型详情',method:'方法端口',sequence:sequence?.tab||'交互序列'};
  const lenses=new Map(array(view.lenses).map(lens=>[lens.id,lens]));
  function applyView(){
    const brand=view.brand;
    document.title=brand?.title||model.title+' · Contract Canvas';
    $('#brand-name').textContent=brand?.name?brand.name+' ':'';
    $('.brand').setAttribute('aria-label',brand?.ariaLabel||'契约画布总览');
    if(brand?.edition)$('#edition-name').textContent=brand.edition;
    if(brand?.editionNumber){$('#edition-number').textContent=brand.editionNumber;$('#edition-number').hidden=false;}
    if(brand?.footer)$('#project-footer').textContent=brand.footer;
    if(overview?.tab)$('[data-scene="overview"]').textContent=overview.tab;
    if(sequence?.tab)$('[data-scene="sequence"]').textContent=sequence.tab;
    $('.scenario-lens').insertAdjacentHTML('beforeend',[...lenses.values()].map(lens=>`<button data-lens="${esc(lens.id)}">${esc(lens.label)}${lens.detail?` <span>→</span> ${esc(lens.detail)}`:''}</button>`).join(''));
  }
  const normalized=id=>nodes.has(id)?id:fieldOwners.get(id)||id;
  const nodeFor=id=>nodes.get(id)||nodes.get(normalized(id));
  const fieldFor=id=>model.nodes.flatMap(n=>array(n.fields)).find(f=>f.id===id);
  const labelFor=id=>nodes.get(id)?.name||fieldFor(id)?.name||id;
  const portsFor=n=>array(n?.ports).map(p=>typeof p==='string'?nodes.get(p):nodes.get(p.id)||p).filter(Boolean);
  const typeRefs=field=>array(field?.typeIds).filter(id=>nodes.has(id));
  const relationIds=id=>{
    const n=nodeFor(id), refs=new Set(array(n?.fields).flatMap(typeRefs));
    model.edges.forEach(e=>{if(normalized(e.from)===id)refs.add(normalized(e.to));if(normalized(e.to)===id)refs.add(normalized(e.from));});
    refs.delete(id);return [...refs].filter(v=>nodes.has(v));
  };
  function reachable(id,direction,layerOnly=false){
    const visited=new Set([id]),queue=[id];
    while(queue.length){const current=queue.shift();for(const e of model.edges){const from=normalized(e.from),to=normalized(e.to);if(layerOnly&&(!layers.contains(from)||!layers.contains(to)))continue;const candidate=direction==='upstream'?(to===current?from:null):(from===current?to:null);if(candidate&&!visited.has(candidate)){visited.add(candidate);queue.push(candidate);}}}
    return visited;
  }
  function supportText(n){
    const support=n?.support??{};
    const keys=Object.keys(support).filter(key=>key!=='note');
    const proposed=n?.provenance==='proposed';
    const shown=proposed?(view.proposedSupportKeys??[]):keys;
    const status=shown.filter(key=>keys.includes(key)).map(key=>`${setting(view.supportLabels,key)||key} ${support[key]}`);
    return [...(proposed?['设计提案 · 未实现']:[]),...status].join(' · ')||'源关联契约';
  }
  function typeLink(id,label){return `<button class="type-link" data-type="${esc(id)}" data-reference="${esc(id)}" aria-haspopup="dialog">${esc(label||labelFor(id))}</button>`;}
  const isImplementation=item=>item?.visibility==='implementation';
  const publicFields=n=>array(n?.fields).filter(f=>!isImplementation(f));
  const implementationFields=n=>array(n?.fields).filter(isImplementation);
  const publicPorts=n=>portsFor(n).filter(p=>!isImplementation(p));
  const firstPort=n=>portsFor(n)[0]?.id;
  function fieldRow(f){return `<div class="field-row ${state.selected===f.id?'field-selected':''}" data-contract-field="${esc(f.id)}"><span class="field-name" title="${esc(f.summary||f.type)}">${esc(f.name)}${f.optional?'?':''}${f.nullable?' ∅':''}</span><span class="field-value">${refMarkup(f)}</span></div>`;}
  function refMarkup(field){
    const refs=typeRefs(field);
    return refs.length?refs.map(id=>typeLink(id)).join('<span class="field-punctuation"> · </span>'):`<span>${esc(field.type||'—')}</span>`;
  }
  let referenceOrigin=null;
  function navigate(id,scene,options={}){
    const n=nodeFor(id);if(!n)return;
    const crossed=layers.enter(id,options.origin||referenceOrigin||document.activeElement);
    scene=scene||(isMethod(n)?'method':'type');
    if(options.push!==false){
      if(!state.history.length&&!crossed)state.history.push({id:state.selected,scene:state.scene,label:state.scene==='layer'?'层模型':labelFor(state.selected)});
      const last=state.history.at(-1);if(last?.id!==id||last.scene!==scene)state.history.push({id,scene,label:isMethod(n)?n.id:n.name||id});
      if(state.history.length>12)state.history.splice(1,1);
    }
    state.selected=id;state.scene=scene;
    if(isMethod(n))state.method=n.id;
    state.zoom=1;state.pan={x:0,y:0};state.trace=null;
    closeDrawers();render();
    const selected=$('#diagram [data-id="'+CSS.escape(n.id)+'"],#diagram [data-layer-select="'+CSS.escape(n.id)+'"]');
    if(selected)selected.focus({preventScroll:true});
    $('#announcer').textContent=`${sceneNames[scene]}，${n.name||id}`;
  }
  function openSequence(reset){
    if(!sequence)return;
    const id=sequence.steps[0].id,crossed=layers.enter(id);
    if(reset&&!state.history.length&&!crossed)state.history.push({id:state.selected,scene:state.scene,label:state.scene==='layer'?sceneNames.layer:labelFor(state.selected)});
    if(reset){state.step=0;state.branch=null;}
    state.selected=id;state.scene='sequence';
    state.history.push({id,scene:state.scene,label:sceneNames.sequence});
    state.zoom=1;state.pan={x:0,y:0};render();
  }
  function changeScene(scene){
    if(scene==='layer'){state.scene=scene;state.zoom=1;state.pan={x:0,y:0};render();return;}
    if(scene==='overview'){
      if(!overview){changeScene('layer');return;}
      if(layers.enter(overview.subjectId)){state.selected=overview.subjectId;state.method=firstPort(nodeFor(state.selected));}
    }
    if(scene==='sequence'){openSequence(false);return;}
    if(scene==='method'){const current=nodeFor(state.selected);const port=isMethod(current)?current:portsFor(current)[0]||nodes.get(state.method);if(port)navigate(port.id,'method');return;}
    if(scene==='type'){const n=nodeFor(state.selected);navigate(isMethod(n)?n.owner:n.id,'type');return;}
    state.scene='overview';state.zoom=1;state.pan={x:0,y:0};render();
  }
  function renderNavigator(){
    $('#type-count').textContent=types.filter(n=>layers.contains(n.id)&&!isImplementation(n)).length;
    const query=state.query.trim().toLowerCase();
    let html='',implementation='';
    for(const domain of domains){
      const list=types.filter(n=>layers.contains(n.id)&&n.domain===domain&&(!query||`${n.name} ${n.id} ${n.summary} ${array(n.fields).map(f=>f.type).join(' ')}`.toLowerCase().includes(query)));
      const methodList=query?methods.filter(n=>layers.contains(n.id)&&n.domain===domain&&`${n.id} ${n.name}`.toLowerCase().includes(query)):[];
      if(!list.length&&!methodList.length)continue;
      const entry=n=>isMethod(n)?`<button class="type-item method-result" data-method="${esc(n.id)}"><span class="type-symbol">ƒ</span><span class="item-name">${esc(n.id)}</span></button>`:`<button class="type-item ${nodeFor(state.selected)?.id===n.id?'active':''}" data-type="${esc(n.id)}" aria-current="${nodeFor(state.selected)?.id===n.id}" style="--accent:${accent(domain)}"><span class="type-symbol">${n.kind==='interface'?'I':n.kind==='enum'?'E':n.kind==='union'?'U':'T'}</span><span class="item-name">${esc(n.name)}</span></button>`;
      const matches=[...list,...methodList],shown=matches.filter(n=>!isImplementation(n));
      if(shown.length)html+=`<div class="domain-heading" style="--accent:${accent(domain)}"><i class="domain-dot"></i>${esc(setting(view.domains,domain)?.label||domain)}</div>${shown.map(entry).join('')}`;
      implementation+=matches.filter(isImplementation).map(entry).join('');
    }
    if(implementation)html+=`<details class="navigator-implementation" ${query||isImplementation(nodeFor(state.selected))?'open':''}><summary>实现契约</summary>${implementation}</details>`;
    $('#type-list').innerHTML=html||'<p class="scope-note">没有匹配的类型或方法。</p>';
  }
  function renderBreadcrumbs(){
    const trail=state.history.length?state.history:[{id:state.selected,scene:state.scene,label:state.scene==='layer'?'层模型':labelFor(state.selected)}];
    $('#breadcrumbs').innerHTML=trail.map((entry,i)=>`${i?'<span class="crumb-separator">/</span>':''}<button data-crumb="${i}" title="${esc(entry.label)}">${esc(entry.label)}</button>`).join('');
    $('#breadcrumbs').scrollLeft=9999;
  }
  function renderSources(n){
    const refs=array(n?.sourceRefs);
    return `<section class="inspector-section"><h3 class="section-heading">SOURCE EVIDENCE <span>${refs.length}</span></h3>${refs.map((ref,i)=>{
      const s=typeof ref==='string'?sources.get(ref):ref;
      if(!s)return `<div class="source-location">${esc(ref)}</div>`;
      const path=s.path||s.file||s.location||s.href||s.url||s.id;
      const line=s.line||s.lines?.[0]||s.startLine;
      return `<details class="source-disclosure"><summary>${esc(s.label||s.title||s.symbol||s.id||`source ${i+1}`)}</summary><div class="source-location">${esc(path)}${line?':'+esc(line):''}</div><p>${esc(s.excerpt||s.summary||s.note||'源声明拥有契约语义；提案文件只描述设计，均不代表运行采用。')}</p><button data-copy="${esc(path+(line?':'+line:''))}">复制源位置</button></details>`;
    }).join('')}</section>`;
  }
  function renderInspector(){
    const n=nodeFor(state.selected), method=isMethod(n);
    const fields=publicFields(n), internal=implementationFields(n), ports=publicPorts(n), internalPorts=portsFor(n).filter(isImplementation), relatives=relationIds(n.id).filter(id=>layers.contains(id)&&!isMethod(nodes.get(id)));
    const note=n.support?.note;
    let html=`<span class="inspector-kind" style="--accent:${accent(n.domain)}">${esc(n.kind)} / ${esc(n.domain)}${isImplementation(n)?' / 实现':''}</span><h2 class="inspector-title">${esc(method?n.id:n.name||n.id)}</h2><span class="status-badge">${esc(supportText(n))}</span><p class="inspector-summary">${esc(n.summary||'')}</p>`;
    if(state.scene==='sequence')html+=renderSequenceInspector();
    if(method){
      html+=`<section class="inspector-section"><h3 class="section-heading">OWNER</h3>${typeLink(n.owner)}</section>`;
      html+=`<section class="inspector-section"><h3 class="section-heading">INPUTS <span>${array(n.inputs).length}</span></h3>${array(n.inputs).map(f=>`<div class="field-row"><span class="field-name">${esc(f.name)}${f.optional?'?':''}</span><span class="field-value">${refMarkup(f)}</span></div>`).join('')||'<p class="scope-note">无额外输入</p>'}</section>`;
      if(n.output)html+=`<section class="inspector-section"><h3 class="section-heading">RETURNS</h3><div class="field-row"><span class="field-name">${esc(n.output.type||'return')}</span><span class="field-value">${refMarkup(n.output)}</span></div></section>`;
      const exits=[...array(n.errors),...array(n.outcomes)].map(e=>typeof e==='string'?e:e.id).filter(Boolean);
      if(exits.length)html+=`<section class="inspector-section"><h3 class="section-heading">ERRORS / OUTCOMES</h3><div class="related-list">${exits.map(id=>`<button class="related-chip" data-type="${esc(id)}">${esc(labelFor(id))}</button>`).join('')}</div></section>`;
    } else {
      if(fields.length)html+=`<section class="inspector-section"><h3 class="section-heading">FIELDS <span>${fields.length}</span></h3>${fields.map(fieldRow).join('')}</section>`;
      if(ports.length)html+=`<section class="inspector-section"><h3 class="section-heading">METHOD PORTS <span>${ports.length}</span></h3>${ports.map(p=>`<button class="method-button" data-method="${esc(p.id)}"><span>ƒ ${esc(p.name+'()')}</span><small>→</small></button>`).join('')}</section>`;
    }
    if(internalPorts.length)html+=`<details class="implementation-disclosure"><summary>实现端口 · ${internalPorts.length}</summary>${internalPorts.map(p=>typeLink(p.id)).join(' · ')}</details>`;
    if(internal.length)html+=`<details class="implementation-disclosure"><summary>实现字段 · ${internal.length}</summary>${internal.map(fieldRow).join('')}</details>`;
    if(note&&n.provenance!=='proposed')html+=`<section class="inspector-section"><h3 class="section-heading">CAPABILITY</h3><p class="scope-note">${esc(note)}</p></section>`;
    html+=`<section class="inspector-section"><h3 class="section-heading">DEPENDENCY TRACE</h3><div class="trace-controls"><button data-trace="upstream" class="${state.trace==='upstream'?'active':''}">← 上游</button><button data-trace="downstream" class="${state.trace==='downstream'?'active':''}">下游 →</button></div></section>`;
    if(relatives.length)html+=`<section class="inspector-section"><h3 class="section-heading">RELATED TYPES <span>${relatives.length}</span></h3><div class="related-list">${relatives.map(id=>`<button class="related-chip" data-type="${esc(id)}">${esc(labelFor(id))}</button>`).join('')}</div></section>`;
    html+=layers.inspectorRelations(n)+renderSources(n);
    $('#inspector-content').innerHTML=html;
  }
  function svgText(x,y,text,className,extra=''){return `<text x="${x}" y="${y}" class="${className}" ${extra}>${esc(text)}</text>`;}
  const shorten=(text,max)=>String(text??'').length>max?String(text).slice(0,max-1)+'…':String(text??'');
  function svgPort(id,box,label){
    const n=nodes.get(id), selected=state.selected===id;
    return `<g class="port-node ${selected?'selected':''}" data-id="${esc(id)}" data-method="${esc(id)}" ${state.scene!=='overview'&&state.selected!==id?`data-reference="${esc(id)}" aria-haspopup="dialog"`:''} role="button" tabindex="0" aria-label="方法端口 ${esc(id)}"><title>${esc(n?.signature||id)}</title><rect class="port-surface" x="${box.x}" y="${box.y}" width="${box.w}" height="${box.h}"/>${svgText(box.x+12,box.y+box.h/2+4,`ƒ ${shorten(label||n?.name||id,Math.floor((box.w-30)/7))}`,'port-title')}${box.h>45?svgText(box.x+12,box.y+box.h-11,'METHOD PORT','port-tag'):''}<circle class="port-dot" cx="${box.x}" cy="${box.y+box.h/2}" r="4"/><circle class="port-dot" cx="${box.x+box.w}" cy="${box.y+box.h/2}" r="4"/></g>`;
  }
  function svgNode(id,box,options={}){
    const n=nodeFor(id);if(!n)return '';
    if(isMethod(n))return svgPort(id,box);
    const selected=state.selected===id;
    const active=state.trace?reachable(state.selected,state.trace):null;
    const dimmed=active&&!active.has(id);
    const ports=portsFor(n), max=Math.floor((box.w-34)/8.1);
    const detail=options.detail||array(n.fields).slice(0,2).map(f=>f.name).join(' · ')||n.kind;
    const card=`<g class="type-node ${selected?'selected':''} ${dimmed?'dimmed':''}" data-id="${esc(id)}" data-type="${esc(id)}" ${state.scene!=='overview'?`data-reference="${esc(id)}" aria-haspopup="dialog"`:''} tabindex="0" role="button" aria-label="类型 ${esc(n.name)}" style="--accent:${accent(n.domain)}"><title>${esc(n.name+' — '+n.summary)}</title><rect class="node-surface" x="${box.x}" y="${box.y}" width="${box.w}" height="${box.h}"/><rect class="node-accent" x="${box.x}" y="${box.y+12}" width="3" height="22"/>${svgText(box.x+14,box.y+20,shorten(n.kind.toUpperCase(),25),'node-kind')}${svgText(box.x+14,box.y+43,shorten(n.name,max),'node-title')}${svgText(box.x+14,box.y+63,shorten(detail,Math.floor((box.w-28)/6)),'node-detail')}</g>`;
    return card+(options.port&&ports.length?svgPort(ports[0].id,{x:box.x+10,y:box.y+box.h-24,w:box.w-20,h:28}):'');
  }
  let routingObstacles=[];
  function edge(a,b,label,options={}){
    const vertical=options.vertical||state.narrow||(Math.abs((a.x+a.w/2)-(b.x+b.w/2))<Math.min(a.w,b.w)/2);
    let sx,sy,tx,ty,d,lx,ly;
    if(vertical){sx=a.x+a.w/2;sy=a.y+a.h;tx=b.x+b.w/2;ty=b.y;const mid=(sy+ty)/2;d=`M${sx},${sy} V${mid} H${tx} V${ty}`;lx=(sx+tx)/2;ly=mid-5;}
    else {const forward=b.x>=a.x+a.w;sx=forward?a.x+a.w:a.x;sy=a.y+a.h/2;tx=forward?b.x:b.x+b.w;ty=b.y+b.h/2;const mid=(sx+tx)/2;d=`M${sx},${sy} H${mid} V${ty} H${tx}`;lx=mid;ly=(sy+ty)/2-6;}
    if(!state.narrow&&routingObstacles.length){
      const routed=ContractRoutes.route(a,b,routingObstacles,vertical);d=routed.d;lx=routed.label.x;ly=routed.label.y;
    }
    const marker=options.error?'error':options.active?'active':'default';
    const text=shorten(label,15),textW=text.length*5.8+12;
    return `<g class="typed-edge"><path class="edge-path ${options.error?'rejects':''} ${options.active?'active':''}" d="${d}" marker-end="url(#arrow-${marker})"/>${label?`<rect class="edge-label-bg" x="${lx-textW/2}" y="${ly-11}" width="${textW}" height="16"/>${svgText(lx,ly,text,'edge-label','text-anchor="middle"')}`:''}</g>`;
  }
  function overviewScene(){
    const groups=overview.groups.map(({label,nodeIds})=>({label,types:nodeIds.map(id=>nodes.get(id))}));
    const l=ContractLayout.overview(groups,state.narrow);
    const visible=new Map(l.nodes.map(n=>[n.id,n]));
    routingObstacles=l.nodes;
    let html=l.boundaries.map(b=>`<rect class="domain-scope" x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}"/>${svgText(b.x+15,b.y+23,b.label,'domain-label')}${svgText(b.x+b.w-17,b.y+23,String(b.count).padStart(2,'0'),'domain-number','text-anchor="end"')}`).join('');
    const portBoxes=[];
    for(const box of l.nodes){
      const n=nodes.get(box.id),ports=portsFor(n),p=ports.find(p=>overview.ports?.includes(p.id))||ports[0];
      if(p){const pb={id:p.id,x:box.x+9,y:box.y+box.h-25,w:box.w-18,h:28};visible.set(p.id,pb);portBoxes.push(pb);}
    }
    if(!state.narrow){
      const candidates=model.edges.filter(e=>visible.has(e.from)&&visible.has(e.to)&&e.from!==e.to&&e.kind!=='owns');
      html+=candidates.slice(0,10).map(e=>edge(visible.get(e.from),visible.get(e.to),'',{active:e.from===overview.highlightFrom})).join('');
    }
    html+=l.nodes.map(box=>svgNode(box.id,box,{detail:portsFor(nodes.get(box.id)).length?'':undefined})).join('');
    html+=portBoxes.map(box=>svgPort(box.id,box,nodes.get(box.id).name+'()')).join('');
    return {html,...l};
  }
  function typeScene(){
    let n=nodeFor(state.selected);if(isMethod(n))n=nodes.get(n.owner);
    const fields=publicFields(n),ports=publicPorts(n);
    const preferred=view.neighborOrder??[];
    let neighbors=relationIds(n.id).filter(id=>layers.contains(id)&&!isMethod(nodes.get(id)));
    neighbors=[...new Set(neighbors)].sort((a,b)=>{const ai=preferred.indexOf(a),bi=preferred.indexOf(b);return(ai<0?999:ai)-(bi<0?999:bi);});
    if(state.trace){const reachableIds=reachable(n.id,state.trace);neighbors=neighbors.filter(id=>reachableIds.has(id));}
    const fieldHeights=fields.map(f=>Math.max(36,typeRefs(f).length*17+4));
    const fieldPixels=fieldHeights.reduce((sum,h)=>sum+h,0);
    const l=ContractLayout.typeDetail(fieldPixels/36,ports.length,neighbors,state.narrow),b=l.center;
    let html='';
    routingObstacles=[b,...l.nodes];
    for(const other of l.nodes){
      const direct=model.edges.find(e=>normalized(e.from)===n.id&&normalized(e.to)===other.id)||model.edges.find(e=>normalized(e.to)===n.id&&normalized(e.from)===other.id);
      if(!direct)continue;
      const outgoing=normalized(direct.from)===n.id;
      if(state.narrow){
        const trunk=b.x+b.w/2, targetY=other.y+other.h/2;
        const targetX=other.x<trunk?other.x+other.w:other.x;
        const d=outgoing?`M${trunk},${b.y+b.h} V${targetY} H${targetX}`:`M${targetX},${targetY} H${trunk} V${b.y+b.h}`;
        html+=`<path class="edge-path ${state.focus?'active':''}" d="${d}" marker-end="url(#arrow-${state.focus?'active':'default'})"/>`;
      } else html+=edge(outgoing?b:other,outgoing?other:b,direct.kind,{active:state.focus});
    }
    html+=`<g data-id="${esc(n.id)}" class="class-detail" tabindex="0" role="group" aria-label="${esc(n.name)} 类型详情" style="--accent:${accent(n.domain)}"><rect class="section-surface" x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}"/>${svgText(b.x+18,b.y+25,n.kind.toUpperCase()+' / '+n.domain.toUpperCase(),'node-kind')}${svgText(b.x+18,b.y+53,shorten(n.name,32),'section-title')}<line class="node-separator" x1="${b.x}" x2="${b.x+b.w}" y1="${b.y+72}" y2="${b.y+72}"/>`;
    fields.forEach((f,i)=>{
      const fy=b.y+84+fieldHeights.slice(0,i).reduce((sum,h)=>sum+h,0),refs=typeRefs(f), labelWidth=state.narrow?112:145;
      html+=svgText(b.x+15,fy+18,shorten(f.name+(f.optional?'?':''),Math.floor(labelWidth/6.6)),'node-field-name');
      if(refs.length){
        let rx=b.x+labelWidth;
        refs.forEach((id,j)=>{
          const title=labelFor(id),max=Math.floor((b.w-labelWidth-22)/6.7), text=shorten(title,max);
          const rowY=refs.length>1?fy+j*17:fy+5;
          html+=`<g class="reference-row" data-type="${esc(id)}" data-field="${esc(f.id)}" tabindex="0" role="button" aria-label="${esc(f.name)} 引用 ${esc(title)}"><title>${esc(f.type)}</title><rect class="row-hit" x="${rx-5}" y="${rowY}" width="${b.w-labelWidth-12}" height="19"/>${svgText(rx,rowY+13,text+' ↗','reference-type')}</g>`;
        });
      }else html+=svgText(b.x+labelWidth,fy+18,shorten(f.type,Math.floor((b.w-labelWidth-16)/6.4)),'node-field');
      html+=`<line class="node-separator" x1="${b.x+14}" x2="${b.x+b.w-14}" y1="${fy+fieldHeights[i]-3}" y2="${fy+fieldHeights[i]-3}"/>`;
    });
    ports.forEach((port,i)=>{html+=svgPort(port.id,{x:b.x+13,y:b.y+88+fieldPixels+i*37,w:b.w-26,h:29},port.name);});
    html+='</g>';
    html+=l.nodes.map(box=>svgNode(box.id,box)).join('');
    return {html,...l};
  }
  function methodScene(){
    const n=nodes.get(state.method);
    const inputs=array(n.inputs).map(f=>({...f,ref:typeRefs(f)[0]}));
    const outputs=typeRefs(n.output).map(id=>({ref:id,type:n.output.type,label:'returns'}));
    const errors=array(n.errors).map(v=>nodes.get(typeof v==='string'?v:v.id)).filter(Boolean);
    const outcomes=array(n.outcomes).map(v=>nodes.get(typeof v==='string'?v:v.id)).filter(Boolean);
    const groups=[];if(errors.length)groups.push({title:'REJECTS',items:errors,error:true});if(outcomes.length)groups.push({title:'OUTCOMES',items:outcomes});
    const narrow=state.narrow,width=narrow?390:1080;
    const p={x:narrow?35:385,y:narrow?75+inputs.length*112:195,w:narrow?320:300,h:65};
    const inputBoxes=inputs.map((f,i)=>({x:narrow?45:30,y:65+i*125,w:narrow?300:285,h:88}));
    let nextY=narrow?p.y+133:60;
    const outputBoxes=outputs.map(f=>{const box={x:narrow?45:765,y:nextY,w:narrow?300:285,h:88};nextY+=125;return box;});
    const groupBoxes=groups.map(group=>{const box={x:narrow?45:765,y:nextY,w:narrow?300:285,h:47+group.items.length*28};nextY+=box.h+28;return box;});
    routingObstacles=[p,...inputBoxes,...outputBoxes,...groupBoxes];
    let html=svgText(narrow?25:30,28,'INPUT CONTRACTS','domain-label')+svgText(narrow?25:765,narrow?p.y+108:28,'RETURN / ERROR / OUTCOME','domain-label');
    inputBoxes.forEach((b,i)=>{
      if(narrow){const channel=15+i*5;html+=`<path class="edge-path active" d="M${b.x},${b.y+b.h/2} H${channel} V${p.y+p.h/2} H${p.x}" marker-end="url(#arrow-active)"/>`;}
      else html+=edge(b,p,inputs[i].name+(inputs[i].optional?'?':''),{active:true});
    });
    outputBoxes.forEach(b=>html+=edge(p,b,'returns',{active:true}));
    groupBoxes.forEach((b,i)=>{
      if(narrow){const channel=372+i*4;html+=`<path class="edge-path ${groups[i].error?'rejects':'active'}" d="M${p.x+p.w},${p.y+p.h/2} H${channel} V${b.y+b.h/2} H${b.x+b.w}" marker-end="url(#arrow-${groups[i].error?'error':'active'})"/>`;}
      else html+=edge(p,b,groups[i].title.toLowerCase(),{error:groups[i].error,active:!groups[i].error});
    });
    html+=svgPort(n.id,p,n.id);
    inputBoxes.forEach((b,i)=>html+=inputs[i].ref?svgNode(inputs[i].ref,b,{detail:inputs[i].type}):valueCard(b,inputs[i].name,inputs[i].type));
    outputBoxes.forEach((b,i)=>html+=svgNode(outputs[i].ref,b,{detail:outputs[i].type}));
    groupBoxes.forEach((b,i)=>{
      const group=groups[i];
      html+=`<g><rect class="section-surface" x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}"/>${svgText(b.x+14,b.y+23,group.title,'node-kind')}`;
      group.items.forEach((item,j)=>{const y=b.y+37+j*28;html+=`<g class="reference-row" data-type="${esc(item.id)}" role="button" tabindex="0" aria-label="${esc(group.title+' '+item.name)}"><rect class="row-hit" x="${b.x+7}" y="${y-5}" width="${b.w-14}" height="27"/>${svgText(b.x+15,y+12,shorten(item.name,34)+' ↗','reference-type')}</g>`;});html+='</g>';
    });
    let height=Math.max(narrow?nextY+20:500,nextY+55,...inputBoxes.map(b=>b.y+b.h+25));
    const ids=setting(view.methodAnnotations,n.id);
    if(ids?.length){
      const y=narrow?height-10:Math.max(470,nextY+15,...inputBoxes.map(b=>b.y+b.h+15));
      ids.forEach((id,i)=>{const item=nodes.get(id),bx=narrow?25:30+(i%2)*520,by=y+(narrow?i:Math.floor(i/2))*80,bw=narrow?340:500;html+=`<g class="reference-row" data-type="${esc(id)}" tabindex="0" role="button" aria-label="${esc(item.name+' '+supportText(item))}"><title>${esc(item.summary+' '+(item.support?.note||''))}</title><rect class="capability-box" x="${bx}" y="${by}" width="${bw}" height="61"/>${svgText(bx+12,by+21,shorten(item.id,narrow?38:58),'capability-label')}${svgText(bx+12,by+43,supportText(item)+' ↗','capability-label')}</g>`;});
      height=y+(narrow?ids.length*80:Math.ceil(ids.length/2)*80+5);
    }
    return {html,width,height};
  }
  function valueCard(b,name,type){return `<g><rect class="section-surface" x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}"/>${svgText(b.x+14,b.y+22,'VALUE / PARAMETER','node-kind')}${svgText(b.x+14,b.y+46,shorten(name,30),'node-title')}${svgText(b.x+14,b.y+67,shorten(type,Math.floor((b.w-25)/6)),'node-detail')}</g>`;}
  function sequenceBranches(){
    return model.edges.filter(e=>e.from===sequence.branchSource&&e.kind==='variant').map(e=>nodes.get(e.to));
  }
  function renderSequenceInspector(){
    const steps=sequence.steps,current=steps[Math.min(state.step,steps.length-1)];
    return `<section class="inspector-section"><h3 class="section-heading">${esc(sequence.stepHeading||'SEQUENCE')} <span>${state.step+1} / ${steps.length}</span></h3><p class="inspector-summary">${esc(current?.caption)}</p><div class="trace-controls"><button data-step-delta="-1" ${state.step===0?'disabled':''}>← 上一步</button><button data-step-delta="1" ${state.step>=steps.length-1?'disabled':''}>下一步 →</button></div></section>${sequence.branchSource?`<section class="inspector-section"><h3 class="section-heading">${esc(sequence.branchHeading||'BRANCHES')}</h3><div class="branch-list">${sequenceBranches().map(n=>`<button class="branch-button ${state.branch===n.id?'active':''}" data-branch="${esc(n.id)}">${esc(n.name)} <span>↗</span></button>`).join('')}</div></section>`:''}`;
  }
  function sequenceScene(){
    const narrow=state.narrow,width=narrow?390:1080;
    const lanes=narrow?(sequence.lanes.length===3?[42,187,337]:[70,320]):(sequence.lanes.length===3?[125,520,950]:[180,900]);
    const labels=sequence.lanes.map(lane=>narrow?(lane.narrowLabel||lane.label):lane.label);
    const steps=sequence.steps,branchY=135+steps.length*65;
    let html=labels.map((label,i)=>svgText(lanes[i],35,label,'lane-title','text-anchor="middle"')+`<line class="lane-line" x1="${lanes[i]}" y1="55" x2="${lanes[i]}" y2="${branchY-30}"/>`).join('');
    steps.forEach((step,i)=>{
      const y=75+i*65,from=lanes[step.from],to=lanes[step.to],active=state.step===i;
      const left=Math.min(from,to),right=Math.max(from,to);
      html+=`<path class="edge-path ${active?'active':''} ${step.error?'rejects':''}" d="M${from},${y+38} H${to}" marker-end="url(#arrow-${active?'active':step.error?'error':'default'})"/>`;
      const bx=narrow?left+5:left+16,bw=narrow?Math.max(right-left-10,120):Math.min(right-left-32,370);
      html+=`<g class="sequence-step ${active?'active':''}" data-step="${i}" role="button" tabindex="0" aria-label="${esc(sequence.stepLabel||'步骤')} ${i+1} ${esc(step.label)}"><title>${esc(step.caption)}</title><rect x="${bx}" y="${y-2}" width="${bw}" height="33"/>${svgText(bx+7,y+11,String(i+1).padStart(2,'0'),'step-index')}${svgText(bx+26,y+11,shorten(step.label,narrow?18:43),'step-label')}${!narrow?svgText(bx+26,y+25,shorten(step.caption,48),'step-description'):''}</g>`;
    });
    const branches=sequenceBranches();
    const lastStepY=75+(steps.length-1)*65+38,branchOrigin=lanes[steps.at(-1).to];
    branches.forEach((n,i)=>{
      const bx=narrow?25:35+i*(1010/Math.max(branches.length,1)),by=narrow?branchY+i*84:branchY,bw=narrow?340:1010/Math.max(branches.length,1)-12;
      const d=narrow?`M${branchOrigin},${lastStepY} H14 V${by+33} H${bx}`:`M${branchOrigin},${lastStepY} V${branchY-35} H${bx+bw/2} V${by}`;
      html+=`<path class="edge-path ${state.branch===n.id?'active':''}" d="${d}" marker-end="url(#arrow-${state.branch===n.id?'active':'default'})"/>`;
    });
    branches.forEach((n,i)=>{
      const bx=narrow?25:35+i*(1010/Math.max(branches.length,1)),by=narrow?branchY+i*84:branchY,bw=narrow?340:1010/Math.max(branches.length,1)-12;
      html+=`<g class="sequence-step" data-branch="${esc(n.id)}" tabindex="0" role="button" aria-label="${esc(sequence.branchLabel||'分支')} ${esc(n.name)}"><rect class="branch-surface ${state.branch===n.id?'active':''}" x="${bx}" y="${by}" width="${bw}" height="66"/>${svgText(bx+11,by+24,shorten(n.name,Math.floor((bw-20)/7)),'branch-title')}${svgText(bx+11,by+46,shorten(n.summary,Math.floor((bw-20)/10)),'branch-note')}</g>`;
    });
    const annotation=sequence.annotation;
    const stopY=narrow?branchY+branches.length*84+18:branchY+95;
    if(annotation){
      const label=annotation.label||labelFor(annotation.id);
      html+=`<g class="reference-row" data-type="${esc(annotation.id)}" tabindex="0" role="button" aria-label="${esc(annotation.ariaLabel||label)}"><rect class="capability-box" x="25" y="${stopY}" width="${width-50}" height="43"/>${svgText(38,stopY+17,(narrow?annotation.narrowLabel||label:label)+' ↗','capability-label')}${svgText(38,stopY+32,narrow?annotation.narrowCaption||annotation.caption:annotation.caption,'capability-label')}</g>`;
    }
    return {html,width,height:annotation?stopY+62:branches.length?stopY+10:branchY};
  }
  function renderDiagram(){
    const svg=$('#diagram'),scenario=state.scene==='scenario';
    svg.classList.toggle('is-hidden',scenario);
    if(scenario)return;
    let result;
    if(state.scene==='layer')result=layers.scene();
    else if(state.scene==='overview')result=overviewScene();
    else if(state.scene==='type')result=typeScene();
    else if(state.scene==='method')result=methodScene();
    else result=sequenceScene();
    state.geometry=result;
    const defs='<defs><marker id="layer-inherit" markerWidth="10" markerHeight="10" refX="9" refY="5" orient="auto"><path class="layer-inherit-marker" d="M1,1 L9,5 L1,9 Z"/></marker><marker id="layer-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path class="edge-arrow" d="M0,0 L8,4 L0,8 Z"/></marker><marker id="arrow-default" markerWidth="7" markerHeight="7" refX="6" refY="3.5" orient="auto"><path class="edge-arrow" d="M0,0 L7,3.5 L0,7 Z"/></marker><marker id="arrow-active" markerWidth="7" markerHeight="7" refX="6" refY="3.5" orient="auto"><path class="edge-arrow-active" d="M0,0 L7,3.5 L0,7 Z"/></marker><marker id="arrow-error" markerWidth="7" markerHeight="7" refX="6" refY="3.5" orient="auto"><path class="edge-arrow-error" d="M0,0 L7,3.5 L0,7 Z"/></marker></defs>';
    $('#diagram').innerHTML=defs+result.html;
    $('#diagram').setAttribute('aria-label',sceneNames[state.scene]+' · '+labelFor(state.selected));
    $('#diagram').style.aspectRatio=state.narrow||state.scene==='layer'?`${result.width} / ${result.height}`:'';
    setCamera();
  }
  function setCamera(){
    const g=state.geometry;if(!g)return;
    const w=g.width/state.zoom,h=g.height/state.zoom;
    $('#diagram').setAttribute('viewBox',`${(g.width-w)/2+state.pan.x} ${(g.height-h)/2+state.pan.y} ${w} ${h}`);
    $('.fit-button').textContent=state.zoom===1?'适应':Math.round(state.zoom*100)+'%';
  }
  function render(){
    references.close();
    $('#empty-model').hidden=true;$('#empty-model').style.display='none';
    renderNavigator();renderBreadcrumbs();renderInspector();renderDiagram();layers.render();walkthrough.render();
    document.body.dataset.workbenchScene=state.scene;
    const n=nodeFor(state.selected);
    const titles={scenario:view.walkthrough?.title||walkthrough.scenario?.name,layer:layers.layers.find(l=>l.id===state.layerId)?.name||'层模型',overview:overview?.title||model.title,type:n?.name||state.selected,method:state.method,sequence:sequence?.title||sceneNames.sequence};
    const kickers={scenario:view.walkthrough?.kicker||'CONTRACT WALKTHROUGH',layer:'DECLARATION LAYER / OBJECT MODEL',overview:'DOMAIN OVERVIEW',type:'TYPE / CONTRACT',method:'METHOD PORT / TYPED FLOW',sequence:sequence?.kicker||'CONTRACT SEQUENCE'};
    $('#scene-title').textContent=titles[state.scene];$('#scene-kicker').textContent=kickers[state.scene];
    $('#scene-total').textContent=state.scene==='scenario'?`${state.scenarioStep+1} / ${walkthrough.stepCount}`:state.scene==='layer'?`${types.filter(n=>layers.contains(n.id)&&!isImplementation(n)).length} TYPES\n${methods.filter(n=>layers.contains(n.id)&&!isImplementation(n)).length} PORTS`:state.scene==='overview'?`${types.length} TYPES\n${methods.length} PORTS`:state.scene==='sequence'?'READ-ONLY TRACE':n?.kind?.toUpperCase()||'CONTRACT';
    document.querySelectorAll('[data-scene]').forEach(b=>{const selected=b.dataset.scene===state.scene;b.classList.toggle('active',selected);b.setAttribute('aria-current',String(selected));});
    $('[data-scene="overview"]').hidden=!overview||!layers.contains(overview.subjectId);
    $('[data-scene="sequence"]').hidden=!sequence||!layers.contains(sequence.steps[0].id);
    $('.scenario-lens').hidden=!lenses.size||walkthrough.available();
    document.querySelectorAll('[data-lens]').forEach(b=>{
      const lens=lenses.get(b.dataset.lens);
      b.classList.toggle('active',lens.scene===state.scene&&(lens.scene!=='method'||state.method===lens.subjectId));
    });
    $('[data-action="focus"]').setAttribute('aria-pressed',String(state.focus));
    if(state.focus&&state.scene==='overview'){
      const related=new Set([state.selected,...relationIds(state.selected)]);
      document.querySelectorAll('#diagram .type-node').forEach(el=>el.classList.toggle('dimmed',!related.has(el.dataset.id)));
    }
  }
  let drawerOrigin=null;
  function toggleDrawer(id){
    const open=!$('#'+id).classList.contains('open');closeDrawers();
    if(open){drawerOrigin=document.activeElement;$('#'+id).classList.add('open');$('#drawer-shade').hidden=false;$('#'+id).querySelector('button,input')?.focus();}
  }
  function closeDrawers(){
    document.querySelectorAll('.navigator.open,.inspector.open').forEach(el=>el.classList.remove('open'));$('#drawer-shade').hidden=true;
    if(drawerOrigin?.isConnected){drawerOrigin.focus({preventScroll:true});drawerOrigin=null;}
  }
  function selectStep(index){
    const steps=sequence.steps;state.step=Math.max(0,Math.min(steps.length-1,index));state.selected=steps[state.step].id;
    if(isMethod(nodes.get(state.selected)))state.method=state.selected;
    render();
  }
  async function copyLocation(button){
    const value=button.dataset.copy;
    try{await navigator.clipboard.writeText(value);button.textContent='已复制';}
    catch{const text=document.createElement('textarea');text.value=value;document.body.appendChild(text);text.select();document.execCommand('copy');text.remove();button.textContent='已复制';}
  }
  document.addEventListener('click',event=>{
    const action=event.target.closest('[data-ref-action]')?.dataset.refAction;
    if(action==='enter')referenceOrigin=$('[aria-controls="reference-preview"][aria-expanded="true"]');
    if(action==='enter-pin')referenceOrigin=event.target.closest('[data-ref-action]');
    const handled=references.handleClick(event);referenceOrigin=null;
    if(handled)return;
    if(layers.handleClick(event)||walkthrough.handleClick(event))return;
    const t=event.target.closest('[data-type],[data-method],[data-scene],[data-crumb],[data-action],[data-trace],[data-lens],[data-step],[data-step-delta],[data-branch],[data-copy]');if(!t)return;
    if(t.dataset.type){navigate(t.dataset.type);return;}
    if(t.dataset.method){navigate(t.dataset.method,'method');return;}
    if(t.dataset.scene){changeScene(t.dataset.scene);return;}
    if(t.dataset.crumb!==undefined){const index=Number(t.dataset.crumb),entry=state.history[index]||{id:state.selected,scene:state.scene};state.history=state.history.slice(0,index+1);navigate(entry.id,entry.scene,{push:false});return;}
    if(t.dataset.trace){state.trace=state.trace===t.dataset.trace?null:t.dataset.trace;render();$('[data-trace="'+t.dataset.trace+'"]')?.focus({preventScroll:true});return;}
    if(t.dataset.lens){const lens=lenses.get(t.dataset.lens);if(lens?.scene==='method')navigate(lens.subjectId,'method');else if(lens?.scene==='sequence')openSequence(true);return;}
    if(t.dataset.step!==undefined){selectStep(Number(t.dataset.step));return;}
    if(t.dataset.stepDelta){selectStep(state.step+Number(t.dataset.stepDelta));return;}
    if(t.dataset.branch){state.branch=t.dataset.branch;state.selected=t.dataset.branch;render();$('#announcer').textContent=labelFor(state.selected)+'，'+nodeFor(state.selected).summary;return;}
    if(t.dataset.copy){copyLocation(t);return;}
    switch(t.dataset.action){
      case 'navigator':toggleDrawer('navigator');break;
      case 'inspector':toggleDrawer('inspector');break;
      case 'help':$('#help-dialog').showModal();break;
      case 'close-help':$('#help-dialog').close();break;
      case 'focus':state.focus=!state.focus;render();break;
      case 'fit':state.zoom=1;state.pan={x:0,y:0};setCamera();break;
      case 'zoom-in':state.zoom=Math.min(2.5,state.zoom+.25);setCamera();break;
      case 'zoom-out':state.zoom=Math.max(.5,state.zoom-.25);setCamera();break;
      case 'reset':references.clear();Object.assign(state,viewportState());layers.reset();$('#search').value='';closeDrawers();render();break;
    }
  });
  document.addEventListener('keydown',event=>{
    if(references.handleKey(event))return;
    const target=event.target;
    if((event.key==='Enter'||event.key===' ')&&target.matches('svg [role="button"]')){event.preventDefault();target.dispatchEvent(new MouseEvent('click',{bubbles:true}));}
    if(event.key==='/'&&!target.matches('input,textarea')){event.preventDefault();if(state.narrow)toggleDrawer('navigator');$('#search').focus();}
    if(event.key==='Escape')closeDrawers();
    const open=document.querySelector('.navigator.open,.inspector.open');
    if(event.key==='Tab'&&open){const tabbables=[...open.querySelectorAll('button,input,summary,a[href]')].filter(el=>el.getClientRects().length);const first=tabbables[0],last=tabbables.at(-1);if(event.shiftKey&&target===first){event.preventDefault();last.focus();}else if(!event.shiftKey&&target===last){event.preventDefault();first.focus();}}
  });
  $('#search').addEventListener('input',event=>{state.query=event.target.value;renderNavigator();});
  $('#search-form').addEventListener('submit',event=>{event.preventDefault();const first=$('#type-list [data-type],#type-list [data-method]');if(first)first.click();});
  $('#drawer-shade').addEventListener('click',closeDrawers);
  $('#help-dialog').addEventListener('click',event=>{if(event.target===$('#help-dialog'))$('#help-dialog').close();});
  let drag=null;
  $('#diagram').addEventListener('pointerdown',event=>{if(event.target.closest('[role="button"]')||state.narrow)return;drag={x:event.clientX,y:event.clientY,px:state.pan.x,py:state.pan.y};$('#diagram').setPointerCapture(event.pointerId);});
  $('#diagram').addEventListener('pointermove',event=>{if(!drag)return;const rect=$('#diagram').getBoundingClientRect(),scale=state.geometry.width/rect.width/state.zoom;state.pan={x:drag.px-(event.clientX-drag.x)*scale,y:drag.py-(event.clientY-drag.y)*scale};setCamera();});
  $('#diagram').addEventListener('pointerup',()=>{drag=null;});
  $('#diagram').addEventListener('pointercancel',()=>{drag=null;});
  $('#diagram').addEventListener('lostpointercapture',()=>{drag=null;});
  addEventListener('resize',()=>{const narrow=innerWidth<=760;if(narrow!==state.narrow){state.narrow=narrow;state.zoom=1;state.pan={x:0,y:0};closeDrawers();render();}});
  $('.brand').addEventListener('click',event=>{event.preventDefault();changeScene('overview');});
  window.ContractWorkbench={model,state,navigate,render};
  const references=window.createContractReferences({model,state,nodeFor,isMethod,portsFor,esc,refMarkup,supportText,renderSources,navigate,toggleDrawer,publicFields,implementationFields,publicPorts});
  const layers=window.createContractLayers({model,view,state,types,isMethod,nodeFor,firstPort,viewportState,setting,typeRefs,esc,svgText,renderSources,render,closeDrawers,toggleDrawer,navigate,reachable,publicFields,publicPorts,isImplementation});
  const walkthrough=window.createContractWalkthrough({model,view,state,nodeFor,fieldFor,isMethod,esc,render});
  applyView();
  layers.reset();
  render();
})();
