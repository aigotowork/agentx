/** @aiPurpose Bind manifest scene identifiers to React instances through a product-neutral adapter. */
import type {ComponentType,ReactNode} from 'react';
import type {Resource} from './resource.types';
import type {TokenDocument,TokenValues} from './token.model';
export type SceneProps={args:Record<string,string|boolean>;emit:(name:string,payload?:unknown)=>void;updateArgs?:(patch:Record<string,string|boolean>)=>void;inspect?:(part:string)=>void;selection?:string};
export type SceneModule={scenes:Record<string,ComponentType<SceneProps>>};
export type SystemAdapter={id:string;title:string;Provider:ComponentType<{children:ReactNode}>;Controls:ComponentType;TokenEditor:ComponentType;tokens?:TokenDocument;useTokens?:()=>{mode?:string;values:TokenValues;apply:(values:TokenValues)=>void};canvas?:{resource:string;scene?:string;wide?:boolean}[];aliases?:Record<string,string>;composition?:{resource:string;scene:string};useSettings?:()=>{value:Record<string,unknown>;apply:(value:Record<string,unknown>)=>void};validateSettings?:(value:unknown)=>string|null};
export type LoadedResource={manifest:Resource;path:string;documentation:string;load:()=>Promise<SceneModule>};
