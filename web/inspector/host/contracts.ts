/** Consumer-owned presentation and schema contracts, independent of runtime SDKs. */
import type { z } from 'zod';
import type { ReactNode, ComponentType } from 'react';
import type { AgentSource } from '../plugin/shared/model';

export interface PluginTheme { readonly colors: { readonly surface0: string; readonly surface1: string; readonly surface2: string; readonly border: string; readonly foreground: string; readonly foregroundMuted: string; readonly accent: string; readonly accentForeground: string; readonly statusSuccess: string; readonly statusWarning: string; readonly statusDanger: string } }
export interface PluginSurfaceProps { theme: PluginTheme; host: {id:string;label:string}; layout:{compact:boolean;platform:'web'|'ios'|'android'}; navigation?:{openAgent(input:{agentId:string;serverId?:string}):void;openWorkspace(input:{workspaceId:string;serverId?:string}):void;openBrowser?(input:{url:string;workspaceId:string;serverId?:string}):void} }
export interface PluginScreenProps extends PluginSurfaceProps { params: Record<string,string> }
export interface PluginWorkspacePanelProps extends PluginScreenProps {workspaceId:string;context?:'workspace'|'agent';revision?:number;tab?:{state:unknown;setState(value:unknown):void;setModified(value:boolean):void}}
export interface PluginAgentPanelProps extends PluginWorkspacePanelProps {agentId:string}
export interface PluginPopoverProps extends Omit<PluginSurfaceProps,'navigation'> {close():void;openScreen(input:{screenId:string;params?:Record<string,string>}):void}
export interface PluginSidebarItemProps extends PluginSurfaceProps {currentScreen:{screenId:string;params:Record<string,string>}|null;openScreen(input:{screenId:string;params?:Record<string,string>}):void;openPopover(Content:ComponentType<PluginPopoverProps>):void}
export interface PluginClientContext {openScreen(input:{screenId:string;params?:Record<string,string>}):void;playAudio(source:{base64:string;mimeType:string}):Promise<void>}
export interface InspectorAgent extends Omit<AgentSource,'activeTurn'|'pendingPermissions'|'attentionReason'> {status:'running'|'initializing'|'idle'|'error'|'closed';provider:string;cwd:string;createdAt:string;updatedAt:string;activeTurn:{turnId?:string;startedAt?:string|null}|null;pendingPermissions:unknown[];attentionReason?:'permission'|'error'|'finished'|null;lastActivityAt?:string|null}
export interface Operation<I extends z.ZodType=z.ZodType,O extends z.ZodType=z.ZodType>{name:string;input:I;output:O}
export type RpcInput<C extends Operation> = z.output<C['input']>;
export type RpcOutput<C extends Operation> = z.input<C['output']>;
/** Schema descriptors are inert; the host chooses the finite local provider. */
export function defineRpc<I extends z.ZodType,O extends z.ZodType>(definition:Operation<I,O>):Operation<I,O>{return definition;}
export interface SettingsDefinition<S extends z.ZodType=z.ZodType>{id:string;scope:'host';version:number;schema:S;migrate?:(values:unknown,fromVersion:number)=>unknown|Promise<unknown>}
export function defineSettings<S extends z.ZodType>(definition:SettingsDefinition<S>):SettingsDefinition<S>{return definition;}

import type { AgentDirectoryPage, AgentDirectoryListOptions, AgentDirectorySubscription } from '../plugin/shared/directory-store';
import type { PermissionResponse } from '../plugin/shared/permissions';
export interface ProviderEntry {provider:string;available:boolean;configured:boolean;displayName:string;modes?:Array<{id:string;isDefault?:boolean}>;models?:Array<{id:string;isDefault?:boolean}>;[key:string]:unknown}
export interface AgentHandle {
 current():InspectorAgent|null;
 refresh():Promise<{agent:InspectorAgent;project:null}|null>;
 archive():Promise<unknown>;
 send(input:string|{text:string},options?:{messageId?:string;activeTurnBehavior?:string}):Promise<void>;
 respondToPermission(input:{requestId:string;response:PermissionResponse}):Promise<void>;
 timeline:{refetch(input?:unknown):Promise<unknown>;subscribe(listener:(update:unknown)=>void):(()=>void)&{ready:Promise<void>}};
}
export interface AgentXMockHost {
 agents:{list(options?:Omit<Partial<AgentDirectoryListOptions>,'filter'>&{subscribe?:object;filter?:{includeArchived?:boolean;requiresAttention?:boolean;labels?:Record<string,string>}}):Promise<AgentDirectoryPage&{subscription:AgentDirectorySubscription}>;subscribe(listener:(event:unknown)=>void):()=>void;ref(id:string):AgentHandle;create(input:unknown):Promise<{id:string}>};
 providers:{snapshot():Promise<{entries:ProviderEntry[]}>};
 workspaces:{ref(id:string):{agents:{create(input:unknown):Promise<{id:string}>}}};
 config:{get():Promise<unknown>};
}
