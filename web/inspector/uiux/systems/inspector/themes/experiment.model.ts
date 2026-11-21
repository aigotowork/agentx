/** @aiPurpose Validate portable visual experiments; playback preferences and domain actions are deliberately outside this contract. */
import {resolveTokens,type TokenValues} from '../../../framework/framework.public';import {inspectorTokens} from './theme.resolve';
export function validateInspectorSettings(value:unknown):string|null{
 if(!value||typeof value!=='object'||Array.isArray(value))return '主题方案必须是对象';const settings=value as Record<string,unknown>;
 if(Object.keys(settings).some(key=>!['style','theme','overrides','tokens'].includes(key))||!['pixel','original'].includes(String(settings.style))||!['light','dark'].includes(String(settings.theme))||!settings.overrides||typeof settings.overrides!=='object'||Array.isArray(settings.overrides))return 'Inspector 主题方案无效';
 const allowed=['accent','step','shadow','press','icon','pressDepth','rhythm','chart'];
 for(const [name,current]of Object.entries(settings.overrides))if(!allowed.includes(name)||(name==='accent'?typeof current!=='string'||!/^#[0-9a-f]{6}$/i.test(current):typeof current!=='number'||!Number.isFinite(current)))return '存在无效语义变量：'+name;
 if(settings.tokens!==undefined){if(!settings.tokens||typeof settings.tokens!=='object'||Array.isArray(settings.tokens))return '无效变量方案';try{resolveTokens(inspectorTokens,settings.tokens as TokenValues,settings.style+'.'+settings.theme);}catch{return '无效变量方案';}}
 return null;
}
