/* Safe DOM primitives and exact semantic references. Presentation only. */
(function (global) {
  'use strict';
  const ns = 'http://www.w3.org/2000/svg';
  function el(tag, text, className) { const node = document.createElement(tag); if (text !== undefined) node.textContent = String(text); if (className) node.className = className; return node; }
  function svg(tag, attributes = {}, text) { const node = document.createElementNS(ns, tag); Object.entries(attributes).forEach(([key,value]) => node.setAttribute(key,String(value))); if (text !== undefined) node.textContent = String(text); return node; }
  function short(value, length = 38) { const text = typeof value === 'string' ? value : JSON.stringify(value); return text === undefined ? '—' : text.length > length ? text.slice(0,length-1)+'…' : text; }
  function json(value) { return value === undefined ? '—' : JSON.stringify(value,null,2); }
  function at(value, path) { for (const part of path ?? []) { if (value === null || typeof value !== 'object' || !Object.prototype.hasOwnProperty.call(value,part)) return undefined; value = value[part]; } return value; }
  function path(value) { return value?.length ? value.join('.') : '$'; }
  function index(artifact) {
    const result = new Map(), definition = artifact.definition;
    const add = (kind,item,owner,scope) => { const entry={kind,item,owner,scope};result.set(kind+':'+item.id+':'+(scope??''),entry); };
    for (const [collection,kind] of [['contexts','context'],['modules','module'],['values','value'],['objects','object'],['rules','rule'],['operations','operation'],['relationships','relationship'],['processes','process'],['schemas','schema'],['sources','source']]) (definition[collection] ?? []).forEach(item=>add(kind,item));
    definition.objects.forEach(object=>{object.fields.forEach(field=>add('field',field,object,object.id));(object.lifecycle?.transitions??[]).forEach(transition=>add('transition',transition,object,object.id));});
    definition.operations.forEach(operation=>{(operation.inputFields??[]).forEach(field=>add('parameter',field,operation,operation.id));operation.outcomes.forEach(outcome=>{add('outcome',outcome,operation,operation.id);(outcome.fields??[]).forEach(field=>add('parameter',field,outcome,operation.id+'/'+outcome.id));});});
    definition.processes.forEach(process=>{(process.inputFields??[]).forEach(field=>add('parameter',field,process,process.id));process.steps.forEach(step=>{add('step',step,process,process.id);(step.bindings??[]).forEach(binding=>add('binding',binding,{step,process},process.id+'/'+step.id));});process.flows.forEach(flow=>add('flow',flow,process,process.id));});
    artifact.scenarios.forEach(scenario=>{add('scenario',scenario);scenario.instances.forEach(instance=>add('instance',instance,scenario,scenario.id));scenario.expectations.forEach(expectation=>add('expectation',expectation,scenario,scenario.id));scenario.visits.forEach(visit=>add('visit',visit,scenario,scenario.id));});
    return result;
  }
  function lookup(context,{kind,id,scope}) {const scoped=scope??(['step','flow'].includes(kind)?context.process?.id:['instance','expectation','visit'].includes(kind)?context.scenario?.id:undefined);return context.index.get(kind+':'+id+':'+(scoped??''));}
  function find(context,kind,id,scope) { return lookup(context,{kind,id,scope})?.item; }
  function label(context,kind,id,scope) { return find(context,kind,id,scope)?.label ?? id ?? '—'; }
  function activate(node,kind,id,name,onSelect,scope) {
    node.dataset.refKind=kind;node.dataset.refId=id;node.setAttribute('role','button');node.setAttribute('tabindex','0');node.setAttribute('aria-label',name);
    if(scope!==undefined)node.dataset.refScope=scope;
    function reference(){const ref={kind,id,scope};for(const key of ['processId','stepId','scenarioId','visitId','callId','phase'])if(node.dataset[key])ref[key]=node.dataset[key];return ref;}
    node.addEventListener('click',event=>{event.stopPropagation();onSelect(reference(),node);});
    node.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();event.stopPropagation();onSelect(reference(),node);}});
    return node;
  }
  function ref(context,kind,id,text,scope) { const node=el('button',text??label(context,kind,id,scope),'ref-link');node.type='button';return activate(node,kind,id,node.textContent,context.onSelect,scope); }
  function badge(status) { const names={declared:'已声明', 'source-bound':'源码绑定','checked-authored':'声明案例已检查','checked-observed':'观察记录已检查',unsupported:'不支持',passed:'符合预期',failed:'偏离预期',tested:'已测',resolved:'已解析',unavailable:'不可用',mismatch:'不匹配'}; const node=el('span',names[status]??status,'badge');node.dataset.status=status;return node; }
  function facts(entries) {const node=el('dl',undefined,'fact-grid');entries.forEach(([name,value])=>{node.append(el('dt',name),el('dd',typeof value==='string'?value:json(value)));});return node;}
  function section(title,...children) {const node=el('section',undefined,'detail-section');node.append(el('h3',title),...children);return node;}
  function safeLink(link) {
    const node=el('div',undefined,'detail-row'),uri=String(link.uri??'');
    if (/^https?:\/\/[^\s]+$/i.test(uri)) {try {const url=new URL(uri);if(url.protocol==='http:'||url.protocol==='https:'){const anchor=el('a',link.label??uri);anchor.href=url.href;anchor.target='_blank';anchor.rel='noopener noreferrer';node.append(anchor);}}catch(_){/* Keep invalid URI inert. */}}
    if (!node.childNodes.length) node.append(el('strong',link.label??'来源'));
    node.append(el('div',uri,'muted'));return node;
  }
  function metadata(context,item) {
    const fragment=document.createDocumentFragment(),meta=item.metadata??{};
    for(const key of ['summary','responsibility'])if(meta[key])fragment.append(section(key==='summary'?'说明':'职责',el('p',meta[key])));
    if(item.explanation) fragment.append(section('设计依据',el('p',item.explanation.summary),...(item.explanation.detail?[el('p',item.explanation.detail)]:[])));
    if(meta.tags?.length)fragment.append(el('p',meta.tags.join(' · '),'muted'));
    const sourceIds=new Set([...(item.sourceIds??[]),...(item.binding?.sourceIds??[]),...(item.explanation?.sourceIds??[])]);
    const sources=[...sourceIds].map(id=>find(context,'source',id)).filter(Boolean);
    if(sources.length)fragment.append(section('来源',...sources.map(source=>{const node=safeLink(source);node.dataset.sourceId=source.id;node.append(badge(source.basis));return node;})));
    for(const [key,title]of [['sourceLinks','源码链接'],['documents','设计文档']])if(meta[key]?.length)fragment.append(section(title,...meta[key].map(safeLink)));
    if(meta.extensions)fragment.append(section('扩展元数据',el('pre',json(meta.extensions))));
    return fragment;
  }
  global.WorkbenchDOM = Object.freeze({el,svg,short,json,at,path,index,lookup,find,label,activate,ref,badge,facts,section,safeLink,metadata});
}(globalThis));
