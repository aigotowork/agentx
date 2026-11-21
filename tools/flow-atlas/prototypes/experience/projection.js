/* Bounded presentation adapter. Native app/player/layout still own all behavior and facts. */
(function (global) {
  'use strict';
  const Base=global.WorkbenchViews,D=global.WorkbenchDOM;
  const G=Object.freeze({width:336,left:48,right:480,top:96,inset:18,header:92,section:28,field:28,port:36,rule:26,gap:32});
  function brief(value,budget=42) {const full=String(value??'—');let result='',units=0;for(const character of full){units+=/[\u2e80-\uffff]/.test(character)?2:1;if(units>budget)return result+'…';result+=character;}return result;}
  function text(parent,value,x,y,className='domain-text',extra={}) {const n=D.svg('text',{x,y,class:className,...extra},value);parent.append(n);return n;}
  function activate(c,node,kind,id,label,scope) {D.activate(node,kind,id,label,c.onSelect,scope);node.append(D.svg('title',{},label));return node;}
  function root(c) {
    const svg=D.svg('svg',{role:'group','aria-label':Base.names[c.view],class:'domain-scene experience-scene'}),defs=D.svg('defs');
    const marker=D.svg('marker',{id:'wb-arrow',viewBox:'0 0 10 10',refX:9,refY:5,markerWidth:6,markerHeight:6,orient:'auto-start-reverse'});
    marker.append(D.svg('path',{d:'M0 0 L10 5 L0 10z',fill:'currentColor'}));defs.append(marker);svg.append(defs);
    return {svg,nodes:new Map(),ports:new Map(),callPaths:new Map(),replyGeometry:new Map(),boxes:Object.create(null),routes:[],markers:[],context:c};
  }
  function chosen(c,kind,id,scope) {const ref=c.selection;return ref?.kind===kind&&ref.id===id&&(!scope||!ref.scope||ref.scope===scope);}
  function nodes(c) {return c.scenario?c.scenario.instances.map(instance=>({id:instance.id,instance,object:D.find(c,'object',instance.objectId)})).filter(n=>n.object):c.artifact.definition.objects.map(object=>({id:object.id,object}));}
  function members(c,object) {return {operations:c.artifact.definition.operations.filter(o=>o.ownerId===object.id),rules:c.artifact.definition.rules.filter(r=>r.ownerId===object.id)};}
  function height(c,object) {const {operations,rules}=members(c,object);return G.header+G.inset+(object.fields.length?G.section+object.fields.length*G.field:0)+(operations.length?G.section+operations.length*G.port:0)+(rules.length?G.section+rules.length*G.rule:0);}
  function defaults(c) {
    const items=nodes(c),first=c.process?.steps.find(s=>s.kind==='operation'),owner=c.process?.roles.find(r=>r.id===first?.roleId)?.objectId;
    const primary=c.scenario?c.scenario.roles[first?.roleId]:owner,positions=Object.create(null),cursors=[G.top,G.top];
    for(const item of items) {const column=item.id===primary||item.object.id===owner?0:1;positions[item.id]={x:column?G.right:G.left,y:cursors[column]};cursors[column]+=height(c,item.object)+G.gap;}
    return positions;
  }
  function box(p,id,pos,w,h,classes='domain-card') {p.boxes[id]={x:pos.x,y:pos.y,width:w,height:h};return D.svg('rect',{x:pos.x,y:pos.y,width:w,height:h,rx:12,class:classes});}
  function route(p,id,points,label,reference,className='domain-edge ex-edge') {
    const d=points.map((point,i)=>(i?'L':'M')+point.x+' '+point.y).join(' '),path=D.svg('path',{d,class:className,'marker-end':'url(#wb-arrow)','data-route-id':id});
    path.append(D.svg('title',{},label));p.svg.append(path);p.routes.push({id,points});
    if(reference) {const hit=activate(p.context,D.svg('path',{d,class:'domain-hit'}),reference.kind,reference.id,label,reference.scope);p.svg.append(hit);}
    return path;
  }
  function section(group,label,count,x,y) {group.append(D.svg('line',{x1:x+G.inset,y1:y-17,x2:x+G.width-G.inset,y2:y-17,class:'ex-divider'}));text(group,label,x+G.inset,y,'ex-section-label');text(group,String(count),x+G.width-G.inset,y,'ex-section-count',{'text-anchor':'end'});}
  function fieldType(c,object,field) {
    if(field.derived)return 'ƒ derived';let schema=D.find(c,'schema',object.stateSchemaId)?.jsonSchema;
    for(const part of field.path) schema=typeof part==='number'?schema?.items:schema?.properties?.[part];
    return schema?.type??(schema?.anyOf?'union':'—');
  }
  function card(c,p,item,pos) {
    const {id,object,instance}=item,{operations,rules}=members(c,object),h=height(c,object),group=D.svg('g',{'data-node-id':id,class:'ex-node'});
    const selected=chosen(c,instance?'instance':'object',id,c.scenario?.id)||chosen(c,'object',object.id),changed=c.checkpoint?.differences.some(d=>d.instanceId===id);
    group.append(box(p,id,pos,G.width,h,'domain-card ex-card'+(selected?' is-selected':'')+(changed?' is-changed':'')));
    const header=activate(c,D.svg('g',{'data-layout-handle':id,'data-node-id':id}),instance?'instance':'object',id,instance?.label??object.label,instance?c.scenario.id:undefined);
    header.append(D.svg('rect',{x:pos.x,y:pos.y,width:G.width,height:G.header,rx:12,fill:'transparent'}));
    text(header,brief(instance?.label??object.label,34),pos.x+G.inset,pos.y+30,'domain-title');
    const module=D.find(c,'module',object.moduleId),boundary=D.find(c,'context',module?.contextId);
    text(header,brief([boundary?.label,module?.label,object.kind].filter(Boolean).join(' / '),46),pos.x+G.inset,pos.y+50,'ex-module-label');
    text(header,brief(object.responsibility,46),pos.x+G.inset,pos.y+73,'domain-muted');group.append(header);
    let y=pos.y+G.header;
    if(object.fields.length) {
      y+=G.section;section(group,instance?'状态 · 已应用检查点':'状态字段',object.fields.length,pos.x,y);
      for(const field of object.fields) {
        y+=G.field;const value=instance?(field.derived?c.checkpoint?.derived?.[id]?.[field.id]:D.at(c.checkpoint?.states[id]??instance.initial,field.path)):undefined;
        const difference=c.checkpoint?.differences.find(diff=>diff.instanceId===id&&JSON.stringify(diff.path)===JSON.stringify(field.path));
        const row=activate(c,D.svg('g',{class:'ex-field-row'+(chosen(c,'field',field.id,object.id)?' is-selected':''),'data-field-id':field.id,'data-instance-id':instance?.id??''}),'field',field.id,field.label+' · '+D.path(field.path),object.id);
        row.append(D.svg('rect',{x:pos.x+8,y:y-20,width:G.width-16,height:26,rx:5,class:'ex-row-background'}));
        text(row,brief(field.label,24)+(field.derived?' ƒ':''),pos.x+G.inset,y,'ex-field-label'+(field.derived?' ex-field-derived':''));
        text(row,instance?D.short(value,19):fieldType(c,object,field),pos.x+G.width-G.inset,y,difference?'domain-diff ex-field-value':'ex-field-value');
        if(value!==undefined)row.dataset.value=JSON.stringify(value);if(difference){row.dataset.before=JSON.stringify(difference.before);row.dataset.after=JSON.stringify(difference.after);}
        group.append(row);
      }
    }
    if(operations.length) {
      y+=G.section;section(group,'公开方法 / PORTS',operations.length,pos.x,y);
      for(const operation of operations) {
        y+=G.port;const row=activate(c,D.svg('g',{class:'ex-port-row'+(chosen(c,'operation',operation.id)?' is-selected':''),'data-port-id':operation.id,'data-node-id':id}),'operation',operation.id,operation.label+' · '+operation.kind);
        row.append(D.svg('rect',{x:pos.x+8,y:y-24,width:G.width-16,height:30,rx:6,class:'ex-method-surface'}));
        const side=pos.x<(G.left+G.right)/2?'right':'left',anchor={x:pos.x+(side==='right'?G.width:0),y:y-7};
        row.append(D.svg('circle',{cx:anchor.x,cy:anchor.y,r:5,class:'domain-port','data-kind':operation.kind,'data-port-id':operation.id,'data-node-id':id}));
        text(row,D.short(operation.label,27),pos.x+G.inset,y-2,'ex-method-label');text(row,operation.kind==='query'?'查询':'命令',pos.x+G.width-G.inset,y-2,'ex-method-kind');
        p.ports.set(id+'/'+operation.id,anchor);group.append(row);
      }
    }
    if(rules.length) {
      y+=G.section;section(group,'拥有的规则',rules.length,pos.x,y);
      for(const rule of rules) {
        y+=G.rule;const row=activate(c,D.svg('g',{class:'ex-rule-row'+(chosen(c,'rule',rule.id)?' is-selected':'')}),'rule',rule.id,rule.label);
        row.append(D.svg('rect',{x:pos.x+8,y:y-19,width:G.width-16,height:25,rx:5,class:'ex-row-background'}));
        row.append(D.svg('rect',{x:pos.x+G.inset,y:y-10,width:7,height:7,rx:2,class:'ex-rule-mark'}));text(row,brief(rule.label,44),pos.x+G.inset+16,y,'ex-rule-label');group.append(row);
      }
    }
    if(!object.fields.length&&!operations.length&&!rules.length)text(group,'无公开状态与方法',pos.x+G.inset,pos.y+G.header,'ex-identity');
    p.nodes.set(id,group);return group;
  }
  function objects(c) {
    const p=root(c),items=nodes(c),positions=c.positions??defaults(c),cards=items.map(item=>card(c,p,item,positions[item.id])),focus=c.selection;
    const focusedNode=focus?.kind==='instance'?focus.id:focus?.kind==='object'?items.find(n=>n.object.id===focus.id)?.id:null;
    const connected=new Set();
    for(const [i,step] of (c.process?.steps??[]).filter(s=>s.kind==='operation').entries()) {
      const from=c.scenario?c.scenario.roles[step.callerRoleId]:c.process.roles.find(r=>r.id===step.callerRoleId)?.objectId,to=c.scenario?c.scenario.roles[step.roleId]:c.process.roles.find(r=>r.id===step.roleId)?.objectId;
      const a=p.boxes[from],b=p.boxes[to],port=p.ports.get(to+'/'+step.operationId);if(!a||!b||!port||from===to)continue;
      const activeStep=c.checkpoint?.stepId??c.checkpoint?.calls[c.checkpoint.calls.length-1]?.stepId;
      const hasFocus=focus?.kind==='operation'||focusedNode||activeStep;
      const relevant=focus?.kind==='operation'?focus.id===step.operationId:focusedNode?[from,to].includes(focusedNode):activeStep?step.id===activeStep:true;
      const targetRight=port.x===b.x+b.width;
      const lane=targetRight?b.x+b.width+34+i%3*8:b.x-34-i%3*8;
      const start={x:lane<a.x+a.width/2?a.x:a.x+a.width,y:a.y+36+i%4*12};
      const points=[start,{x:lane,y:start.y},{x:lane,y:port.y},port];
      const path=route(p,'invoke:'+step.id,points,step.label+' → '+D.label(c,'operation',step.operationId),{kind:'step',id:step.id,scope:c.process.id},'domain-edge ex-edge'+(relevant&&hasFocus?' is-emphasized':'')+(!relevant?' is-muted':''));
      Object.assign(path.dataset,{stepId:step.id,targetPort:step.operationId,targetNode:to});p.callPaths.set(step.id,path);connected.add(from+'/'+to);
      p.svg.append(D.svg('circle',{cx:start.x,cy:start.y,r:4,class:'ex-caller-origin'}));
    }
    for(const relation of c.artifact.definition.relationships)for(const from of items.filter(n=>n.object.id===relation.fromObjectId))for(const to of items.filter(n=>n.object.id===relation.toObjectId)) {
      if(from.id===to.id||connected.has(from.id+'/'+to.id))continue;
      const a=p.boxes[from.id],b=p.boxes[to.id],right=b.x>a.x,start={x:right?a.x+a.width:a.x,y:a.y+50},end={x:right?b.x:b.x+b.width,y:b.y+54},lane=(start.x+end.x)/2;
      route(p,'relation:'+relation.id+':'+from.id+':'+to.id,[start,{x:lane,y:start.y},{x:lane,y:end.y},end],relation.label+' · '+relation.kind,{kind:'relationship',id:relation.id},'domain-edge ex-edge is-muted');
    }
    p.svg.append(...cards);return p;
  }
  /** One selected binding gets a compact source/consumer lens; the full dataflow remains native. */
  function bindingLens(c) {
    const entry=D.lookup(c,c.selection);if(!entry||entry.kind!=='binding')return Base.render(c);
    const p=root(c),{item,owner:{step,process}}=entry,source=item.source,targetOperation=D.find(c,'operation',step.operationId);
    const sourceStep=source.kind==='result'?process.steps.find(s=>s.id===source.stepId):null,sourceOperation=sourceStep?D.find(c,'operation',sourceStep.operationId):null;
    const originLabel=source.kind==='result'?(sourceOperation?.label??source.stepId)+' / '+source.outcomeId:source.kind==='input'?process.label+' / 输入':source.kind==='identity'?D.label(c,'object',process.roles.find(r=>r.id===source.roleId)?.objectId)+' / 身份':'常量';
    const provenance=global.WorkbenchDetails.parameterCheckpoint(c,step.id)?.parameters.find(param=>param.id===item.id);
    const compact=window.matchMedia('(max-width:700px)').matches,positions=compact?[{x:28,y:90},{x:28,y:410}]:[{x:36,y:112},{x:456,y:112}],w=320,h=250;
    const sourceSchema=source.kind==='input'?D.find(c,'schema',process.inputSchemaId):sourceOperation?D.find(c,'schema',sourceOperation.outcomes.find(o=>o.id===source.outcomeId)?.payloadSchemaId):null;
    const targetSchema=D.find(c,'schema',targetOperation?.inputSchemaId);
    function lensCard(id,pos,title,subtitle,reference,schema,value,side) {
      const group=D.svg('g',{class:'ex-binding-card','data-step-id':id});group.append(box(p,id,pos,w,h));
      text(group,title,pos.x+18,pos.y+28,'ex-section-label');
      const heading=activate(c,D.svg('g'),reference.kind,reference.id,subtitle,reference.scope);heading.append(D.svg('rect',{x:pos.x+10,y:pos.y+40,width:w-20,height:36,rx:6,class:'ex-method-surface'}));text(heading,D.short(subtitle,33),pos.x+18,pos.y+64,'domain-link');group.append(heading);
      text(group,(side==='source'?'来源路径 ':'目标参数 ')+D.path(side==='source'?source.path:item.targetPath),pos.x+18,pos.y+100,'domain-muted');
      const rows=value&&typeof value==='object'?Object.entries(value):value!==undefined?[['$',value]]:Object.entries(schema?.jsonSchema?.properties??{}).map(([key,s])=>[key,s.type??'Schema']);
      rows.slice(0,5).forEach(([key,value],i)=>{text(group,D.short(key,22),pos.x+18,pos.y+132+i*23,'domain-text');text(group,D.short(value,19),pos.x+w-18,pos.y+132+i*23,'ex-field-value');});
      if(!rows.length)text(group,provenance?'无字段值':'定义来源 · 当前无调用证据',pos.x+18,pos.y+132,'domain-muted');
      p.nodes.set(id,group);return group;
    }
    const originRef=sourceStep?{kind:'step',id:sourceStep.id,scope:process.id}:source.kind==='identity'?{kind:'object',id:process.roles.find(r=>r.id===source.roleId)?.objectId}:{kind:'process',id:process.id};
    const a=positions[0],b=positions[1],start=compact?{x:a.x+w/2,y:a.y+h}:{x:a.x+w,y:a.y+100},end=compact?{x:b.x+w/2,y:b.y}:{x:b.x,y:b.y+100};
    const line=route(p,'binding:'+step.id+':'+item.id,[start,end],originLabel+' → '+D.path(item.targetPath),{kind:'binding',id:item.id,scope:process.id+'/'+step.id},'domain-edge is-data is-active');
    Object.assign(line.dataset,{bindingId:item.id,sourceKind:source.kind,targetPath:JSON.stringify(item.targetPath)});
    if(source.kind==='result')Object.assign(line.dataset,{sourceStepId:source.stepId,sourceOutcomeId:source.outcomeId,sourcePath:JSON.stringify(source.path)});
    p.callPaths.set(step.id,line);
    p.svg.append(lensCard(sourceStep?.id??'process-input',a,'来源 / '+source.kind,originLabel,originRef,sourceSchema,provenance?.value,'source'),lensCard(step.id,b,'消费方 / '+(provenance?.status??'定义'),targetOperation?.label??step.label,{kind:'operation',id:step.operationId},targetSchema,provenance?.actual,'target'));
    p.svg.dataset.bindingLens='true';return p;
  }
  let lensSelection='';
  global.WorkbenchViews=Object.freeze({...Base,defaults,render(c){
    if(c.selection?.kind==='binding'){const key=c.selection.id+'/'+c.selection.scope;if(key!==lensSelection){lensSelection=key;document.getElementById('workbench').dataset.bindingFocus='selected';}}
    return c.view==='objects'?objects(c):c.view==='dataflow'&&c.selection?.kind==='binding'&&document.getElementById('workbench').dataset.bindingFocus!=='all'?bindingLens(c):Base.render(c);
  }});
}(globalThis));
