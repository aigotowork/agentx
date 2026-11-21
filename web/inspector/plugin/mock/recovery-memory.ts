/** Memory/store and observation ports around the preserved recovery state machine. */
import { RecoveryService } from '../core/recovery';
import type { RecoveryState, Attempt } from '../shared/domain/recovery';
import type { RecoveryList } from '../shared/recovery';
import type { AgentPort, RecoveryStore } from '../shared/domain/recovery-ports';
import { rawAgents, updateFixtureAgent } from './fixtures.js';
import type { InspectorAgent } from '../../host/contracts';
import type { ContinueInput, ManageInput, Policy } from '../../web/shared/contracts';

export function memoryRecovery(seed:RecoveryList) {
 const state:RecoveryState={...structuredClone(seed),generations:{}};
 const agents=rawAgents as InspectorAgent[];
 for(const item of state.incidents) {
  state.generations[item.agentId]=item.generation;
  item.errorFingerprint=item.message;
  if(!agents.some(agent=>agent.id===item.agentId))agents.push({id:item.agentId,title:item.title,provider:item.provider,model:'gpt-6.1',cwd:`/mock/${item.workspaceId}`,status:'error',workspaceId:item.workspaceId,workspaceName:item.workspaceName,projectId:item.projectId,projectName:item.projectName,lastError:item.message,lastUserMessageAt:item.lastUserMessageAt,createdAt:item.observedAt,updatedAt:item.observedAt,activeTurn:null,pendingPermissions:item.blockedReason?[{id:'mock-permission',kind:'tool',title:'模拟待处理权限'}]:[],attentionReason:'error',requiresAttention:true});
 }
 // Store changes serialize and commit in memory before the effect port is called.
 let queue=Promise.resolve();
 const store:RecoveryStore={async read(){await queue;return structuredClone(state);},change(update){const task=queue.then(()=>update(state));queue=task.then(()=>{},()=>{});return task;},async close(){await queue;}};
 const observation=(id:string)=>{const agent=agents.find(item=>item.id===id);return agent?{id:agent.id,title:agent.title??null,provider:agent.provider,workspaceId:agent.workspaceId,status:agent.status,lastError:agent.lastError??null,lastUserMessageAt:agent.lastUserMessageAt??null,hasActiveTurn:!!agent.activeTurn,pendingPermissionCount:agent.pendingPermissions.length,archivedAt:agent.archivedAt??null,attentionReason:agent.attentionReason??null}:null;};
 const workspace=(id:string)=>{const item=state.incidents.find(item=>item.workspaceId===id);return item?{id,projectId:item.projectId??'mock-project',projectDisplayName:item.projectName,name:item.workspaceName,archivingAt:null}:null;};
 const sent=new Map<string,Attempt>();
 const scenario=new URLSearchParams(globalThis.location?.search??'').get('scenario');
 const port:AgentPort={async scan(){return {agents:state.incidents.map(item=>observation(item.agentId)!).filter(Boolean),workspaces:state.automation.catalog.workspaces.map(item=>workspace(item.id)!).filter(Boolean)};},async refresh(id){return observation(id);},async refreshWorkspace(id){return workspace(id);},async send(id,_text,attempt){
  const guard=await service.deliveryGuard(attempt.id,id,observation(id)!);
  if(guard||scenario==='recovery-not-sent')return {status:'not_sent',messageId:attempt.id,reasonCode:'guard_rejected'};
  if(scenario==='recovery-unknown')return {status:'unknown',messageId:attempt.id,reasonCode:'send_timeout'};
  sent.set(attempt.id,attempt);
  // A simulated receipt is deliberately not turned into execution/completion evidence.
  return {status:'acknowledged',messageId:attempt.id,reasonCode:'ledger_sent'};
 }};
 const service=new RecoveryService(store);
 return async(name:string,input:unknown):Promise<unknown>=>{
  if(name==='recovery.list')return service.list(port,true);
  if(name==='recovery.continue'){const command=input as ContinueInput;return service.continue(port,command.operationId,command.targets);}
  if(name==='recovery.configure')return service.configure(input as Policy);
  if(name==='recovery.manage'){const command=input as ManageInput;return service.manage(command.cycleId,command.action);}
  throw new Error('Unsupported simulated recovery operation');
 };
}
