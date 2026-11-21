/* Design-only projections. No business execution, source importing or trace inference. */
(function () {
  'use strict';
  const model = JSON.parse(document.getElementById('design-model').textContent);
  const byId = items => new Map(items.map(item => [item.id, item]));
  const modules = byId(model.modules), objects = byId(model.objects), operations = byId(model.operations), schemas = byId(model.schemas);
  const graph = document.getElementById('graph'), details = document.getElementById('details'), body = document.getElementById('details-body');
  const choice = document.getElementById('process');
  const viewport = FlowAtlasViewport.create({host:graph,controls:graph.parentNode});
  let view = 'modules', selected, returnTarget;
  const el = (tag, text, className) => { const node = document.createElement(tag); if (text !== undefined) node.textContent = text; if (className) node.className = className; return node; };
  const svg = (tag, attrs = {}, text) => { const node = document.createElementNS('http://www.w3.org/2000/svg', tag); for (const [key,value] of Object.entries(attrs)) node.setAttribute(key, String(value)); if (text !== undefined) node.textContent = text; return node; };
  const short = (text, limit = 30) => { text = String(text ?? ''); return text.length > limit ? text.slice(0,limit - 1) + '…' : text; };
  const label = (map, id) => map.get(id)?.label ?? id;
  function text(parent, value, x, y, className = 'design-meta') { parent.append(svg('text', {x, y, class: className}, value)); }
  function activeLink(uri) { return typeof uri === 'string' && /^https?:\/\/[^\s]+$/i.test(uri); }
  function link(ref) {
    const row = el('p');
    if (activeLink(ref.uri)) { const a = el('a', ref.label); a.href = ref.uri; a.target = '_blank'; a.rel = 'noopener noreferrer'; row.append(a); }
    else row.append(el('span', ref.label));
    row.append(el('span', ' · ' + ref.uri, 'muted')); body.append(row);
  }
  function metadata(item) {
    const meta = item.metadata ?? {};
    for (const key of ['summary','responsibility']) if (meta[key]) body.append(el('p',meta[key]));
    if (meta.tags?.length) body.append(el('p',meta.tags.join(' · '),'muted'));
    for (const [field,title] of [['sourceLinks','源码'],['documents','设计文档']]) if (meta[field]?.length) {
      body.append(el('h3',title)); meta[field].forEach(link);
    }
    if (meta.extensions) { body.append(el('h3','扩展元信息')); body.append(el('pre',JSON.stringify(meta.extensions,null,2))); }
    if (item.explanation) body.append(el('p',item.explanation.summary));
  }
  function open(title, target) {
    returnTarget = target;
    document.getElementById('details-title').textContent = title;
    body.replaceChildren(); details.hidden = false;
    selected = target; paint();
    document.getElementById('close').focus({preventScroll:true});
  }
  function close() { details.hidden = true; selected = undefined; paint(); const target = Array.from(graph.querySelectorAll('[data-testid]')).find(node=>node.dataset.testid===returnTarget); if (target) target.focus({preventScroll:true}); }
  function button(title, fn) { const node = el('button',title); node.addEventListener('click',fn); body.append(node); return node; }
  function schemaSection(title, id) { body.append(el('h3',title + ' · ' + label(schemas,id))); body.append(el('pre',JSON.stringify(schemas.get(id)?.jsonSchema ?? {error:'Schema unavailable'},null,2))); }
  function operationDetail(item) {
    open(item.label, 'operation:' + item.id);
    body.append(el('p',label(objects,item.ownerId) + ' · ' + item.kind)); metadata(item);
    schemaSection('输入',item.inputSchemaId); schemaSection('输出',item.outputSchemaId);
    body.append(el('h3','声明结果')); item.outcomes.forEach(outcome => { body.append(el('p',outcome.label + ' · ' + outcome.meaning)); metadata(outcome); });
  }
  function objectDetail(item) {
    open(item.label,'object:' + item.id); body.append(el('p',label(modules,item.moduleId) + ' · ' + item.kind)); metadata(item);
    const owner = modules.get(item.moduleId); if (owner) { body.append(el('h3','模块职责')); metadata(owner); }
    body.append(el('h3','公开方法')); model.operations.filter(op=>op.ownerId === item.id).forEach(op=>button(op.label,()=>operationDetail(op)));
  }
  function stepDetail(item, process) {
    open(item.label, 'step:' + item.id); metadata(item);
    body.append(el('p',item.kind === 'operation' ? (item.bound ? '实现步骤已绑定' : '缺少实现绑定') : item.kind === 'end' ? '终点 · ' + item.outcome : '入口'));
    if (item.operationId && operations.has(item.operationId)) button(label(operations,item.operationId) + ' · 查看方法契约',()=>operationDetail(operations.get(item.operationId)));
    const outgoing = process.flows.filter(flow=>flow.from===item.id);
    if (outgoing.length) { body.append(el('h3','可能的下一步')); outgoing.forEach(flow=>body.append(el('p',flow.label + ' → ' + (process.steps.find(s=>s.id===flow.to)?.label ?? flow.to)))); }
  }
  function diagnostics() {
    open('设计检查','diagnostics');
    if (!model.diagnostics.length) body.append(el('p','当前定义没有结构诊断。'));
    model.diagnostics.forEach(item=>{ const row=el('div',undefined,'diagnostic'); row.dataset.testid='design-diagnostic'; row.append(el('strong',item.code+' · '+item.severity),el('p',item.message),el('code',item.path.join('.'))); body.append(row); });
    body.append(el('h3','检查范围')); model.coverage.forEach(value=>body.append(el('p',value)));
  }
  function selectable(group, key, name, action) {
    group.setAttribute('role','button'); group.setAttribute('tabindex','0'); group.setAttribute('aria-label',name); group.dataset.testid = key;
    group.addEventListener('click',event=>{event.stopPropagation();action();}); group.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();event.stopPropagation();action();}});
    group.append(svg('title',{},name)); return group;
  }
  function base(width,height) {
    const root = svg('svg',{viewBox:`0 0 ${width} ${height}`,width,height,role:'group','aria-label':view === 'modules'?'模块、对象及原生方法':'声明的流程及结果分支'});
    const defs=svg('defs'),marker=svg('marker',{id:'design-arrow',viewBox:'0 0 10 10',refX:9,refY:5,markerWidth:6,markerHeight:6,orient:'auto'}); marker.append(svg('path',{d:'M0 0 L10 5 L0 10z',class:'design-arrow'})); defs.append(marker);root.append(defs); return root;
  }
  function card(parent,item,p,rows,key,action) {
    const group=selectable(svg('g',{class:'design-node'+(selected===key?' is-selected':'')}),key,item.label,action);
    group.append(svg('rect',{x:p.x,y:p.y,width:260,height:p.height,rx:9,class:'design-card'}));
    text(group,short(item.label),p.x+14,p.y+25,'design-title'); rows.forEach((row,i)=>text(group,short(row,36),p.x+14,p.y+46+i*18)); parent.append(group); return group;
  }
  function modulesView() {
    const width=Math.max(650,model.modules.length*360+32),positions=new Map();
    const heights=model.modules.map(module=>model.objects.filter(obj=>obj.moduleId===module.id).reduce((h,obj)=>h+model.operations.filter(op=>op.ownerId===obj.id).length*32+94,0)+64);
    const root=base(width,Math.max(360,...heights)+32);
    model.modules.forEach((module,index)=>{
      const x=32+index*360; let y=68;
      root.append(svg('rect',{x:x-12,y:18,width:310,height:Math.max(286,heights[index]),rx:9,class:'design-module'}));
      const heading=selectable(svg('g'),'module:'+module.id,module.label,()=>{open(module.label,'module:'+module.id);metadata(module);body.append(el('h3','允许的直接依赖'));body.append(el('p',module.dependencies.map(id=>label(modules,id)).join(' · ')||'无'));});
      text(heading,module.label,x,42,'design-title');root.append(heading);
      model.objects.filter(obj=>obj.moduleId===module.id).forEach(obj=>{
        const ops=model.operations.filter(op=>op.ownerId===obj.id),p={x,y,height:58+ops.length*32};positions.set(obj.id,p);
        const group=card(root,obj,p,[obj.kind],'object:'+obj.id,()=>objectDetail(obj));
        ops.forEach((op,opIndex)=>{
          const py=y+64+opIndex*32;
          const port=selectable(svg('g',{class:'design-port'}),'operation:'+op.id,op.label+' · '+op.kind,()=>operationDetail(op));
          port.append(svg('rect',{x:x+6,y:py-11,width:248,height:28,rx:4,class:'design-port__body'}));port.append(svg('circle',{cx:x,cy:py+3,r:5,class:'design-port__pin'}));
          text(port,short(op.label,26),x+14,py+7,'design-port__label'); group.append(port);
        }); y+=p.height+36;
      });
    });
    // Module dependencies are declarations, not recorded runtime calls.
    model.modules.forEach((module,index)=>module.dependencies.forEach(id=>{
      const target=model.modules.findIndex(m=>m.id===id);if(target<0)return;
      const x1=32+index*360+310-12,x2=32+target*360-12;
      root.append(svg('path',{d:`M${x1} 50 H${x2}`,class:'design-wire','marker-end':'url(#design-arrow)'}));
    }));
    return root;
  }
  function processView() {
    const process=model.processes.find(item=>item.id===choice.value);if(!process)return base(650,360);
    // Compute bounded graph depth from references, independently of input array order.
    const depth=new Map([[process.startId,0]]),queue=[process.startId];
    for(let i=0;i<queue.length;i++)for(const flow of process.flows.filter(f=>f.from===queue[i]))if(!depth.has(flow.to)){depth.set(flow.to,depth.get(flow.from)+1);queue.push(flow.to);}
    const layers=new Map();process.steps.forEach(step=>{const d=depth.get(step.id)??Math.max(0,...depth.values())+1;if(!layers.has(d))layers.set(d,[]);layers.get(d).push(step);});
    const outgoing = id => process.flows.filter(flow=>flow.from===id);
    const positions=new Map();for(const [d,steps]of layers){let y=36;steps.forEach(step=>{const height=step.kind==='end'?86:96+outgoing(step.id).length*24;positions.set(step.id,{x:32+d*332,y,height});y+=height+44;});}
    const root=base(Math.max(650,...Array.from(positions.values(),p=>p.x+292)),Math.max(360,...Array.from(positions.values(),p=>p.y+p.height+32)));
    process.flows.forEach((flow,index)=>{
      const a=positions.get(flow.from),b=positions.get(flow.to);if(!a||!b)return;
      const row=outgoing(flow.from).indexOf(flow),incoming=process.flows.filter(f=>f.to===flow.to),entry=incoming.indexOf(flow);
      const forward=b.x>a.x,x1=a.x+260,x2=forward?b.x:b.x+260,y1=a.y+88+row*24,y2=b.y+24+(entry+1)*Math.min(14,(b.height-48)/(incoming.length+1)),middle=forward?x1+38+row*16:Math.max(x1,x2)+30+(index%3)*16;
      const wire=svg('path',{d:`M${x1} ${y1} H${middle} V${y2} H${x2}`,class:'design-wire','marker-end':'url(#design-arrow)'});wire.append(svg('title',{},flow.label));root.append(wire);
    });
    process.steps.forEach(step=>{
      const rows=step.kind==='operation'?[label(objects,step.callerId)+' → '+label(operations,step.operationId),step.bound?'已绑定实现步骤':'未绑定实现步骤']:step.kind==='end'?['终点 · '+step.outcome]:['入口'];
      const p=positions.get(step.id),group=card(root,step,p,rows,'step:'+step.id,()=>stepDetail(step,process));
      outgoing(step.id).forEach((flow,index)=>{
        const y=p.y+88+index*24;
        text(group,short(flow.label,32),p.x+16,y+4,'design-port__label');group.append(svg('circle',{cx:p.x+260,cy:y,r:4,class:'design-port__pin'}));
      });
    });return root;
  }
  function paint() {
    const scene=view==='modules'?modulesView():processView();
    graph.replaceChildren(scene);viewport.mount(scene,view==='modules'?'modules':'process/'+choice.value);
    document.getElementById('graph-title').textContent=view==='modules'?'模块边界 → 所属对象 → 原生公开方法':'可能的流程路径 · 结果分支';
    document.getElementById('graph-count').textContent=view==='modules'?`${model.modules.length} 模块 · ${model.operations.length} 方法`:'声明图 · 非执行轨迹';
  }
  model.processes.forEach(process=>{const option=el('option',process.label);option.value=process.id;choice.append(option);});choice.disabled=!model.processes.length;
  document.getElementById('model-title').textContent=model.label;document.title=model.label+' · Flow Atlas';
  document.getElementById('design-summary').textContent=`${model.objects.length} 对象 · ${model.operations.length} 原生方法 · ${model.processes.length} 流程`;
  const errors=model.diagnostics.filter(d=>d.severity==='error').length;
  document.getElementById('lint').textContent=`检查 ${errors} 错误 / ${model.diagnostics.length-errors} 警告`;
  document.querySelectorAll('[data-view]').forEach(button=>button.addEventListener('click',()=>{view=button.dataset.view;document.querySelectorAll('[data-view]').forEach(b=>b.setAttribute('aria-pressed',String(b===button)));close();paint();}));
  choice.addEventListener('change',()=>{view='process';document.querySelectorAll('[data-view]').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.view===view)));close();paint();});
  document.getElementById('lint').addEventListener('click',diagnostics);document.getElementById('coverage').addEventListener('click',diagnostics);
  document.getElementById('close').addEventListener('click',close);document.addEventListener('keydown',event=>{if(event.key==='Escape'&&!details.hidden)close();});
  paint();
}());
