/** @aiPurpose Validate portable design work, preserving stable instance IDs while refusing stale or foreign resources. */
import type {Resource} from './resource.types';
export type SceneArgs=Record<string,string|boolean>;
export type CanvasItem={id:string;resource:string;scene:string;wide?:boolean;args?:SceneArgs};
export type WorkspaceSnapshot={schemaVersion:1;system:string;items:CanvasItem[];composition?:CanvasItem;view:'overview'|'composition';selection:string;part?:string;settings:Record<string,unknown>};
export type DesignPlan={id:string;name:string;updatedAt:string;stage:'experiment'|'review';snapshot:WorkspaceSnapshot};
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const id=/^[a-zA-Z0-9_-]{1,100}$/;
export function sceneDefaults(resource:Resource,sceneId:string):SceneArgs{return Object.fromEntries((resource.scenes.find(scene=>scene.id===sceneId)?.controls??[]).map(control=>[control.id,control.default]));}
/** All imported settings are checked by the selected system before applying any portion of the workspace. */
export function parseWorkspace(value:unknown,system:string,resources:Resource[],validateSettings?:(value:unknown)=>string|null):{snapshot?:WorkspaceSnapshot;error?:string}{
 if(!object(value)||value.schemaVersion!==1||value.system!==system)return {error:'方案版本或设计系统不符'};
 if(!Array.isArray(value.items)||value.items.length>60||!['overview','composition'].includes(String(value.view))||typeof value.selection!=='string'||!object(value.settings))return {error:'画布结构无效'};
 if(value.part!==undefined&&(typeof value.part!=='string'||!id.test(value.part)))return {error:'局部选择无效'};
 const settingsError=validateSettings?validateSettings(value.settings):(Object.keys(value.settings).length?'当前系统不支持此主题方案':null);if(settingsError)return {error:settingsError};
 const ids=new Set<string>(),items:CanvasItem[]=[];let composition:CanvasItem|undefined;
 const source=[...value.items,...(value.composition===undefined?[]:[value.composition])];
 for(const entry of source){
  if(!object(entry)||typeof entry.id!=='string'||!id.test(entry.id)||ids.has(entry.id)||typeof entry.resource!=='string'||typeof entry.scene!=='string'||entry.wide!==undefined&&typeof entry.wide!=='boolean'||entry.args!==undefined&&!object(entry.args))return {error:'画布实例无效或重复'};
  ids.add(entry.id);const resource=resources.find(resource=>resource.id===entry.resource&&resource.system===system),scene=resource?.scenes.find(scene=>scene.id===entry.scene);if(!resource||!scene)return {error:'资源或场景已不可用：'+entry.resource+'/'+entry.scene};
  const args:SceneArgs=sceneDefaults(resource,scene.id);
  for(const [name,current]of Object.entries(entry.args??{})){const control=scene.controls?.find(control=>control.id===name);if(!control||control.type==='boolean'&&typeof current!=='boolean'||control.type!=='boolean'&&(typeof current!=='string'||current.length>4000)||control.type==='select'&&!control.options?.some(option=>option.value===current))return {error:'无效场景参数：'+name};args[name]=current as string|boolean;}
  const parsed={id:entry.id,resource:resource.id,scene:scene.id,args,...(entry.wide!==undefined?{wide:entry.wide}:{})};if(entry===value.composition)composition=parsed;else items.push(parsed);
 }
 if(value.view==='composition'&&!composition)return {error:'方案缺少整体场景'};
 if(value.selection&&!ids.has(value.selection))return {error:'选中实例已不存在'};
 if(value.part&&(!composition||value.selection!==composition.id||!resources.find(resource=>resource.id===composition.resource)?.scenes.find(scene=>scene.id===composition.scene)?.parts?.some(part=>part.id===value.part)))return {error:'场景局部已不存在'};
 return {snapshot:{schemaVersion:1,system,items,composition,view:value.view as WorkspaceSnapshot['view'],selection:value.selection,part:value.part as string|undefined,settings:structuredClone(value.settings)}};
}
/** Recursively compare semantic settings and scene parameters without relying on object insertion order. */
export function snapshotChanges(before:WorkspaceSnapshot,after:WorkspaceSnapshot):{field:string;before:string;after:string}[]{
 const flatten=(value:unknown,prefix='',output:Record<string,string>={})=>{if(object(value)){for(const key of Object.keys(value).sort())flatten(value[key],prefix?prefix+'.'+key:key,output);}else if(Array.isArray(value)){value.forEach((entry,index)=>flatten(entry,prefix+'.'+index,output));}else if(value!==undefined)output[prefix]=String(value);return output;};
 const a=flatten({theme:before.settings,canvas:before.items,composition:before.composition}),b=flatten({theme:after.settings,canvas:after.items,composition:after.composition});return [...new Set([...Object.keys(a),...Object.keys(b)])].filter(key=>a[key]!==b[key]).map(field=>({field,before:a[field]??'—',after:b[field]??'—'}));
}
export function workspaceStorageKey(system:string){return 'uiux.workspace.v1.'+system;}
export type StoredWorkspace={schemaVersion:1;draft:WorkspaceSnapshot;plans:DesignPlan[];recoveryArchive?:true};
export type WorkspaceRecoverySource={id:string;raw:string;paths:string[]};
export type WorkspaceRecoveryRecord={id:string;sourceId:string;kind:'draft'|'plan'|'workspace';name:string;value:unknown;error:string};
export type WorkspaceRecovery={sources:WorkspaceRecoverySource[];records:WorkspaceRecoveryRecord[];pending?:string;archiveError?:string};
export type WorkspaceReadResult={draft?:WorkspaceSnapshot;plans:DesignPlan[];error?:string;recovery?:WorkspaceRecovery};
export function workspaceRecoveryKey(system:string){return workspaceStorageKey(system)+'.recovery';}
function planMetadata(value:unknown):value is DesignPlan{return object(value)&&typeof value.id==='string'&&id.test(value.id)&&typeof value.name==='string'&&!!value.name.trim()&&value.name.length<=80&&typeof value.updatedAt==='string'&&Number.isFinite(Date.parse(value.updatedAt))&&['experiment','review'].includes(String(value.stage));}
function checkedWorkspace(value:unknown,system:string,resources:Resource[],validateSettings?:(value:unknown)=>string|null){try{return parseWorkspace(value,system,resources,validateSettings);}catch{return {error:'设计系统无法验证此方案'};}}
function recoverySources(raw:string|null):WorkspaceRecoverySource[]{
 if(raw===null)return [];
 let value:unknown;try{value=JSON.parse(raw);}catch{throw new Error('恢复存档格式损坏，原件未被覆盖');}
 if(!object(value)||value.schemaVersion!==1||!Array.isArray(value.sources)||value.sources.some(source=>!object(source)||typeof source.id!=='string'||!id.test(source.id)||typeof source.raw!=='string'||!Array.isArray(source.paths)||source.paths.some(path=>typeof path!=='string'||!/^(draft|workspace|plans\.\d+)$/.test(path))))throw new Error('恢复存档格式损坏，原件未被覆盖');
 const sources=value.sources as WorkspaceRecoverySource[];
 if(new Set(sources.map(source=>source.id)).size!==sources.length)throw new Error('恢复存档标识重复，原件未被覆盖');
 return sources;
}
function recoveryRecords(sources:WorkspaceRecoverySource[],system:string,resources:Resource[],validateSettings?:(value:unknown)=>string|null):WorkspaceRecoveryRecord[]{
 return sources.flatMap(source=>{
  let parsed:unknown;try{parsed=JSON.parse(source.raw);}catch{/* The original bytes remain downloadable even when JSON cannot be repaired. */}
  return source.paths.map(path=>{
   const kind=path==='draft'?'draft':path.startsWith('plans.')?'plan':'workspace';
   const value=kind==='workspace'?parsed:object(parsed)?kind==='draft'?parsed.draft:Array.isArray(parsed.plans)?parsed.plans[Number(path.slice(6))]:undefined:undefined;
   const name=kind==='draft'?'自动草稿':kind==='workspace'?'工作区原始记录':object(value)&&typeof value.name==='string'&&value.name.trim()?value.name.slice(0,80):'未命名方案';
   const snapshot=kind==='plan'&&object(value)?value.snapshot:value;
   const result=kind==='workspace'?{error:'工作区格式无法自动修复，请导出原件检查'}:checkedWorkspace(snapshot,system,resources,validateSettings);
   return {id:source.id+':'+path,sourceId:source.id,kind,name,value,error:result.error??'原始记录已隔离，需显式恢复'};
  });
 });
}
/** Read valid drafts and plans independently; archived originals remain available after subsequent saves. */
export function readWorkspace(storage:Pick<Storage,'getItem'>,system:string,resources:Resource[],validateSettings?:(value:unknown)=>string|null):WorkspaceReadResult{
 let raw:string|null;try{raw=storage.getItem(workspaceStorageKey(system));}catch{return {plans:[],error:'本地存储不可读取，仍可编辑与导出'};}
 if(raw===null)return {plans:[]};
 let parsed:unknown;try{if(raw.length<=1_000_000)parsed=JSON.parse(raw);}catch{/* Quarantine the whole document below. */}
 const plans:DesignPlan[]=[],paths:string[]=[],errors:string[]=[];let draft:WorkspaceSnapshot|undefined;
 if(!object(parsed)||parsed.schemaVersion!==1){paths.push('workspace');errors.push(raw.length>1_000_000?'本地方案过大，未自动载入':'本地方案格式损坏');}
 else{
  const result=checkedWorkspace(parsed.draft,system,resources,validateSettings);draft=result.snapshot;if(!draft){paths.push('draft');errors.push(result.error??'草稿无效');}
  const ids=new Set<string>();
  if(!Array.isArray(parsed.plans)){paths.push('workspace');errors.push('本地方案列表格式损坏');}
  else parsed.plans.forEach((plan,index)=>{
   const result=checkedWorkspace(object(plan)?plan.snapshot:undefined,system,resources,validateSettings);
   const error=!planMetadata(plan)?'本地方案信息无效':ids.has(plan.id)?'本地方案标识重复':plans.length>=16?'本机方案超出16个':result.error;
   if(error||!result.snapshot){paths.push('plans.'+index);errors.push(error??'方案无效');return;}
   ids.add((plan as DesignPlan).id);plans.push({...plan as DesignPlan,snapshot:result.snapshot});
  });
 }
 let sources:WorkspaceRecoverySource[]=[],archiveError:string|undefined;
 if(object(parsed)&&parsed.recoveryArchive===true){
  let archived:string|null=null;
  try{archived=storage.getItem(workspaceRecoveryKey(system));if(archived===null)throw new Error('恢复存档缺失，已暂停自动保存');sources=recoverySources(archived);}
  catch(error){archiveError=error instanceof Error&&error.message.startsWith('恢复存档')?error.message:'恢复存档不可读取，已暂停自动保存';if(archived!==null)sources=[{id:workspaceId('damaged-archive'),raw:archived,paths:['workspace']}];}
 }
 let pending:string|undefined;
 if(paths.length){
  const existing=sources.find(source=>source.raw===raw);
  if(existing){if(paths.some(path=>!existing.paths.includes(path))){existing.paths=[...new Set([...existing.paths,...paths])];pending=existing.id;}}
  else{const source={id:workspaceId('recovery'),raw,paths};sources.push(source);pending=source.id;}
 }
 const recovery=sources.length||archiveError?{sources,records:recoveryRecords(sources,system,resources,validateSettings),pending,archiveError}:undefined;
 return {draft,plans,error:archiveError??(errors.length?errors[0]+'；已保留原记录':undefined),...(recovery?{recovery}:{})};
}
/** Persist and verify every original before callers may replace the damaged workspace; quota failures keep writes disabled. */
export function archiveWorkspaceRecovery(storage:Pick<Storage,'getItem'|'setItem'>,system:string,recovery:WorkspaceRecovery,resources:Resource[],validateSettings?:(value:unknown)=>string|null):{recovery?:WorkspaceRecovery;error?:string}{
 if(recovery.archiveError)return {error:recovery.archiveError};
 try{
  const sources=recoverySources(storage.getItem(workspaceRecoveryKey(system)));
  for(const source of recovery.sources){const existing=sources.find(entry=>entry.id===source.id||entry.raw===source.raw);if(existing){if(existing.raw!==source.raw)throw new Error('恢复存档冲突');existing.paths=[...new Set([...existing.paths,...source.paths])];}else sources.push(source);}
  const raw=JSON.stringify({schemaVersion:1,sources});storage.setItem(workspaceRecoveryKey(system),raw);
  if(storage.getItem(workspaceRecoveryKey(system))!==raw)throw new Error('恢复存档未能确认写入');
  return {recovery:{sources,records:recoveryRecords(sources,system,resources,validateSettings)}};
 }catch{return {error:'原始记录未能存档，自动保存仍已暂停；可下载原件和当前设计'};}
}
export function writeWorkspace(storage:Pick<Storage,'setItem'>,system:string,value:StoredWorkspace):string|null{try{const json=JSON.stringify(value);if(json.length>1_000_000)return '本地方案过大，请导出后精简';storage.setItem(workspaceStorageKey(system),json);return null;}catch{return '无法写入本地存储，请导出方案保留修改';}}

/** Browser-local identity works on LAN HTTP as well as secure contexts; it is not an authentication token. */
export function workspaceId(prefix:string){
 const bytes=new Uint32Array(3);
 if(globalThis.crypto?.getRandomValues)globalThis.crypto.getRandomValues(bytes);
 else for(let index=0;index<bytes.length;index++)bytes[index]=Math.floor(Math.random()*0x100000000);
 return prefix+'-'+Date.now().toString(36)+'-'+Array.from(bytes,value=>value.toString(36)).join('-');
}
