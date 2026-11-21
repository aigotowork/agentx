/** Typed project-owned values, semantic references and independent mode overrides. */
export type TokenScalar=string|number|boolean;
export type TokenValue=TokenScalar|{ref:string};
export type TokenValues=Record<string,TokenValue>;
export type TokenDefinition={label:string;type:'color'|'dimension'|'duration'|'number'|'boolean'|'select';value:TokenValue;group:'foundation'|'semantic'|'component';unit?:'px'|'ms'|'%';min?:number;max?:number;step?:number;options?:{label:string;value:string}[]};
export type TokenDocument={schemaVersion:1;system:string;tokens:Record<string,TokenDefinition>;modes?:Record<string,{label:string;values:TokenValues}>};
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const tokenId=/^[a-z][a-zA-Z0-9.-]*$/;
export function tokenValues(document:TokenDocument,overrides:TokenValues={},mode?:string):TokenValues {
 if(mode!==undefined&&!Object.hasOwn(document.modes??{},mode))throw new Error('未知变量模式：'+mode);
 return {...Object.fromEntries(Object.entries(document.tokens).map(([id,token])=>[id,token.value])),...(mode?document.modes?.[mode].values:{}),...overrides};
}
export function resolveTokens(document:TokenDocument,overrides:TokenValues={},mode?:string):Record<string,TokenScalar> {
 const values=tokenValues(document,overrides,mode),resolved:Record<string,TokenScalar>={},visiting=new Set<string>();
 for(const id of Object.keys(values))if(!Object.hasOwn(document.tokens,id))throw new Error('未知变量：'+id);
 function visit(id:string):TokenScalar {
  if(Object.hasOwn(resolved,id))return resolved[id];
  if(visiting.has(id))throw new Error('变量引用循环：'+id);
  const definition=document.tokens[id];if(!definition)throw new Error('引用不存在：'+id);
  visiting.add(id);const raw=values[id];let value:TokenScalar;
  if(object(raw)){
   if(Object.keys(raw).length!==1||typeof raw.ref!=='string'||!Object.hasOwn(document.tokens,raw.ref))throw new Error('引用不存在：'+id);
   const target=document.tokens[raw.ref];if(target.type!==definition.type||target.unit!==definition.unit)throw new Error('引用类型或单位不符：'+id);
   value=visit(raw.ref);
  }else value=raw;
  if(definition.type==='color'&&(typeof value!=='string'||!/^#[0-9a-f]{6}$/i.test(value)))throw new Error(id+' 需要六位HEX颜色');
  if(definition.type==='boolean'&&typeof value!=='boolean')throw new Error(id+' 需要布尔值');
  if(definition.type==='select'&&(typeof value!=='string'||!definition.options?.some(option=>option.value===value)))throw new Error(id+' 选项无效');
  if(['dimension','duration','number'].includes(definition.type)){
   if(typeof value!=='number'||!Number.isFinite(value))throw new Error(id+' 需要有限数值');
   if(definition.min!==undefined&&value<definition.min||definition.max!==undefined&&value>definition.max)throw new Error(id+' 超出范围');
   if(definition.step!==undefined&&Math.abs((value-(definition.min??0))/definition.step-Math.round((value-(definition.min??0))/definition.step))>1e-7)throw new Error(id+' 不符合步长');
  }
  visiting.delete(id);resolved[id]=value;return value;
 }
 for(const id of Object.keys(document.tokens))visit(id);return resolved;
}
/** Validate metadata once at the ownership boundary; mode validation also catches reference cycles. */
export function readTokens(value:unknown):TokenDocument {
 if(!object(value)||value.schemaVersion!==1||typeof value.system!=='string'||!/^[a-z][a-z0-9-]*$/.test(value.system)||!object(value.tokens)||!Object.keys(value.tokens).length)throw new Error('变量文档格式无效');
 for(const [id,token]of Object.entries(value.tokens)){
  if(!tokenId.test(id)||['__proto__','constructor','prototype'].includes(id)||!object(token)||typeof token.label!=='string'||!token.label.trim()||!['color','dimension','duration','number','boolean','select'].includes(String(token.type))||!['foundation','semantic','component'].includes(String(token.group)))throw new Error('变量声明无效：'+id);
  if(token.type==='dimension'&&!['px','%'].includes(String(token.unit))||token.type==='duration'&&token.unit!=='ms'||!['dimension','duration'].includes(String(token.type))&&token.unit!==undefined)throw new Error('变量单位无效：'+id);
  for(const bound of ['min','max','step'])if(token[bound]!==undefined&&(typeof token[bound]!=='number'||!Number.isFinite(token[bound])||bound==='step'&&token[bound]<=0))throw new Error('变量边界无效：'+id);
  if(typeof token.min==='number'&&typeof token.max==='number'&&token.min>token.max)throw new Error('变量范围无效：'+id);
  if(token.type==='select'&&(!Array.isArray(token.options)||!token.options.length||!token.options.every(option=>object(option)&&typeof option.label==='string'&&typeof option.value==='string')||new Set(token.options.map(option=>option.value)).size!==token.options.length))throw new Error('变量选项无效：'+id);
 }
 if(value.modes!==undefined){if(!object(value.modes))throw new Error('变量模式无效');for(const [id,mode]of Object.entries(value.modes))if(!tokenId.test(id)||!object(mode)||typeof mode.label!=='string'||!mode.label.trim()||!object(mode.values))throw new Error('变量模式无效：'+id);}
 const document=value as TokenDocument;resolveTokens(document);for(const mode of Object.keys(document.modes??{}))resolveTokens(document,{},mode);return document;
}
/** Source operations may change declared values; definitions and other modes remain intact. */
export function updateTokens(document:TokenDocument,values:TokenValues,mode?:string):TokenDocument {
 resolveTokens(document,values,mode);const next=structuredClone(document);
 if(mode){const initial=tokenValues(document,{},mode);for(const [id,value]of Object.entries(values))if(JSON.stringify(initial[id])!==JSON.stringify(value))next.modes![mode].values[id]=value;}
 else for(const [id,value]of Object.entries(values))next.tokens[id].value=value;
 return readTokens(next);
}
