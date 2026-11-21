/* Responsibility-lane collaboration projection. Definitions own all branches; scenario visits
 * own occurrence chips; the applied checkpoint alone owns current evidence. No clock/reducer.
 * render(context) returns the native WorkbenchViews projection contract.
 */
(function(global){
  'use strict';
  const D=global.WorkbenchDOM,P=global.WorkbenchViews.projection;
  const G=Object.freeze({left:18,top:108,lane:218,width:188,operation:106,terminal:44,gap:38,label:18});
  function laneText(header,value,x,y,className,budget){
    const full=String(value??'—');let visible='',units=0;
    for(const character of full){const width=/[\u2e80-\uffff]/.test(character)?2:1;if(units+width>budget){visible+='…';break;}visible+=character;units+=width;}
    const node=P.text(header,visible,x,y,className);node.append(D.svg('title',{},full));return node;
  }
  function graphRows(model){
    const depths=new Map([[model.startId,0]]),queue=[model.startId];
    for(let i=0;i<queue.length;i++)for(const flow of model.flows.filter(f=>f.from===queue[i]))if(!depths.has(flow.to)){depths.set(flow.to,depths.get(flow.from)+1);queue.push(flow.to);}
    let orphan=Math.max(0,...depths.values())+1;for(const step of model.steps)if(!depths.has(step.id))depths.set(step.id,orphan++);
    return depths;
  }
  function render(c){
    const p=P.root(c),model=c.process;if(!model){P.text(p.svg,'暂无用例',24,44,'domain-muted');return p;}
    const operations=model.steps.filter(s=>s.kind==='operation'),usedRoles=new Set(operations.flatMap(s=>[s.roleId,s.callerRoleId]));
    const roleKey=roleId=>c.scenario?.roles[roleId]??roleId,lanes=[];
    for(const role of model.roles.filter(r=>usedRoles.has(r.id))){const id=roleKey(role.id);if(!lanes.some(l=>l.id===id))lanes.push({id,role,object:D.find(c,'object',role.objectId),instance:c.scenario?.instances.find(i=>i.id===id)});}
    if(!lanes.length)lanes.push({id:'process',role:{label:model.label}});
    const laneIndex=new Map(lanes.map((l,i)=>[l.id,i])),depths=graphRows(model),positions=new Map(),rowY=new Map(),rowHeights=new Map();
    const ownerLane=step=>step.kind==='operation'?(laneIndex.get(roleKey(step.roleId))??0):step.kind==='start'?0:(laneIndex.get(roleKey(model.steps.find(s=>s.id===model.flows.find(f=>f.to===step.id)?.from)?.roleId))??0);
    const groups=new Map();for(const step of model.steps){const depth=depths.get(step.id);if(!groups.has(depth))groups.set(depth,[]);groups.get(depth).push(step);}
    let y=G.top;
    for(const [depth,steps]of [...groups].sort((a,b)=>a[0]-b[0])){
      rowY.set(depth,y);const perLane=new Map();let rowHeight=0;
      for(const step of steps){const column=ownerLane(step),offset=perLane.get(column)??0,height=step.kind==='operation'?G.operation:G.terminal,fanout=model.flows.filter(f=>f.from===step.id).length;
        positions.set(step.id,{x:G.left+column*G.lane,y:y+offset,width:G.width,height});const occupied=offset+height+G.gap+Math.max(0,fanout-1)*G.label;perLane.set(column,occupied);rowHeight=Math.max(rowHeight,occupied);
      }rowHeights.set(depth,rowHeight);y+=rowHeight;
    }
    lanes.forEach((lane,i)=>{
      const x=G.left+i*G.lane,group=D.svg('g',{'data-responsibility-lane':lane.id});
      group.append(D.svg('rect',{x:x-7,y:12,width:G.lane-10,height:y+10,rx:8,class:'linked-lane'}));
      const kind=lane.instance?'instance':'object',id=lane.instance?.id??lane.object?.id;
      const clipId=p.markerId+'-lane-header-'+i,clip=D.svg('clipPath',{id:clipId}),name=lane.instance?.label??lane.object?.label??lane.role.label,responsibility=lane.object?.responsibility??lane.role.label;
      clip.append(D.svg('rect',{x:x+8,y:20,width:G.width-8,height:68}));p.svg.querySelector('defs').append(clip);
      const header=D.svg('g',{'data-node-id':lane.instance?.id??lane.object?.id??lane.id,'data-lane-header':lane.id,'clip-path':'url(#'+clipId+')'});let stateLabel='';
      if(lane.instance&&c.checkpoint){const field=lane.object.fields.find(f=>!f.derived),value=field?D.at(c.checkpoint.states[lane.id],field.path):undefined;stateLabel=field?field.label+' = '+D.json(value):'#'+c.checkpoint.index+' 已应用';}
      if(id)P.selectable(c,header,kind,id,[name,responsibility,stateLabel].filter(Boolean).join(' · '),lane.instance?c.scenario.id:undefined);
      laneText(header,name,x+8,38,'domain-title',25);laneText(header,responsibility,x+8,58,'domain-muted',27);
      if(stateLabel)laneText(header,stateLabel,x+8,80,'domain-muted',27);
      p.svg.append(group,header);p.nodes.set(lane.id,header);
    });
    const appliedCalls=c.checkpoint?.calls??[],scenarioVisits=c.scenario?.visits??[];
    function incomingFlow(visit,i){
      if(visit.viaFlowId)return visit.viaFlowId;
      // Project an already explicit scenario path through a unique declared edge; never invent a trace.
      const previous=scenarioVisits[i-1],candidates=model.flows.filter(f=>f.from===(previous?.stepId??model.startId)&&f.to===visit.stepId&&(!previous||!f.outcomeId||f.outcomeId===previous.outcomeId));
      return candidates.length===1?candidates[0].id:null;
    }
    const visitedFlows=new Set(scenarioVisits.map(incomingFlow).filter(Boolean)),appliedFlows=new Set(scenarioVisits.map((v,i)=>appliedCalls.some(call=>call.visitId===v.id&&call.callId===v.callId)?incomingFlow(v,i):null).filter(Boolean));
    for(const flow of model.flows){
      const a=positions.get(flow.from),b=positions.get(flow.to);if(!a||!b)continue;
      const siblings=model.flows.filter(f=>f.from===flow.from),i=siblings.indexOf(flow),ax=a.x+a.width/2,bx=b.x+b.width/2,bridge=a.y+a.height+24+i*G.label;
      const points=b.y>a.y?[{x:ax,y:a.y+a.height},{x:ax,y:bridge},{x:bx,y:bridge},{x:bx,y:b.y}]:[{x:a.x+a.width,y:a.y+a.height/2},{x:a.x+a.width+12+i*8,y:a.y+a.height/2},{x:a.x+a.width+12+i*8,y:b.y-10},{x:bx,y:b.y-10},{x:bx,y:b.y}];
      const path=P.wire(p,'flow:'+flow.id,points,flow.label+(flow.outcomeId?' · '+flow.outcomeId:''),'domain-edge linked-flow',{kind:'flow',id:flow.id,scope:model.id});
      // The terminal edge has no visit; only explicit terminal checkpoint proves its prefix.
      const lastVisit=scenarioVisits[scenarioVisits.length-1],terminalVisited=!!c.scenario&&flow.to===c.scenario.terminalStepId&&lastVisit?.stepId===flow.from&&(!flow.outcomeId||lastVisit.outcomeId===flow.outcomeId);
      Object.assign(path.dataset,{flowId:flow.id,scenarioPath:String(visitedFlows.has(flow.id)||terminalVisited),applied:String(appliedFlows.has(flow.id)||(terminalVisited&&c.checkpoint?.phase==='terminal')),scenarioId:c.scenario?.id??''});
      const label=P.text(p.svg,D.short(flow.label,25),Math.min(ax,bx)+8,bridge-6,'linked-flow-label');label.append(D.svg('title',{},flow.label+(flow.outcomeId?' / '+flow.outcomeId:'')));
      if(!p.callPaths.has(flow.to))p.callPaths.set(flow.to,path);
    }
    for(const step of model.steps){
      const pos=positions.get(step.id),visits=c.scenario?.visits.filter(v=>v.stepId===step.id)??[],applied=visits.some(v=>appliedCalls.some(call=>call.callId===v.callId&&call.visitId===v.id));
      const focus=c.selection,selected=focus?.stepId===step.id||(focus?.kind==='step'&&focus.id===step.id),group=P.selectable(c,D.svg('g',{'data-step-id':step.id}), 'step',step.id,step.label,model.id);
      Object.assign(group.dataset,{processId:model.id,scenarioId:c.scenario?.id??'',scenarioPath:String(!!visits.length||(step.kind==='start'&&!!c.scenario)||(step.kind==='end'&&step.id===c.scenario?.terminalStepId)),applied:String(applied||(step.kind==='start'&&!!c.checkpoint)||(step.kind==='end'&&c.checkpoint?.phase==='terminal'&&step.id===c.scenario?.terminalStepId)),selected:String(!!selected)});
      group.append(P.outline(p,step.id,pos.x,pos.y,pos.width,pos.height,'domain-card linked-step'+(selected?' is-selected':'')));
      if(step.kind==='end'){laneText(group,step.label,pos.x+12,pos.y+18,'domain-title',22);laneText(group,step.outcome,pos.x+12,pos.y+36,'domain-muted',24);}
      else P.text(group,D.short(step.label,20),pos.x+12,pos.y+25,'domain-title');
      if(step.kind==='operation'){
        P.text(group,D.short(D.label(c,'operation',step.operationId),24),pos.x+12,pos.y+46,'domain-muted');
        P.text(group,D.short((model.roles.find(r=>r.id===step.callerRoleId)?.label??step.callerRoleId)+' → '+(model.roles.find(r=>r.id===step.roleId)?.label??step.roleId),26),pos.x+12,pos.y+65,'domain-muted');
        const params=D.svg('g');P.selectable(c,params,'step',step.id,step.bindings.length+' 参数来源',model.id);params.append(D.svg('rect',{x:pos.x+8,y:pos.y+76,width:pos.width-16,height:24,rx:4,class:'linked-chip-surface'}));P.text(params,step.bindings.length+' 参数'+(visits.length?' · '+visits.length+' 次访问':''),pos.x+12,pos.y+93,'domain-link');group.append(params);
      }else if(step.kind==='start')P.text(group,'入口',pos.x+pos.width-12,pos.y+25,'linked-terminal-label');
      p.nodes.set(step.id,group);p.svg.append(group);
      // Chips remain exact and independently selectable when a step occurs multiple times.
      visits.forEach((visit,i)=>{
        const call=appliedCalls.find(call=>call.callId===visit.callId&&call.visitId===visit.id),chip=P.selectable(c,D.svg('g'), 'step',step.id,step.label+' / '+visit.id,model.id);
        P.occurrence(c,chip,visit,null,!!call);chip.dataset.phase=call?.phase??'call';chip.dataset.occurrenceChip='true';
        const cx=pos.x+pos.width+5,cy=pos.y+12+i*28;chip.append(D.svg('rect',{x:cx,y:cy,width:24,height:24,rx:6,class:'linked-chip-surface'}));P.text(chip,String(i+1),cx+8,cy+17,'domain-link');p.svg.append(chip);
      });
    }
    return p;
  }
  global.WorkbenchCollaboration=Object.freeze({render});
}(globalThis));
