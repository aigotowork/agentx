/** Static import boundary for the migrated Inspector; remote executable chunks are unsupported. */
import type {ComponentType} from 'react';
import type {PluginClientContext} from '../../host/contracts';
import type {TextSource} from '../shared/i18n';
export interface ChunkExtras {text?:TextSource}
export interface ChunkLoaderOptions {label:string|(()=>string);text?:TextSource}
export function createChunkLoader(client:PluginClientContext,options:ChunkLoaderOptions) {
 let ready:Record<string,ComponentType<any>>|undefined;
 return {peek:()=>ready,label:()=>typeof options.label==='function'?options.label():options.label,
 async load(){return ready??=(await import('./surfaces')).default().createSurfaces(client,{text:options.text});}};
}
