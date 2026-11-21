/** Persistence boundary: preserve Space v1 entries without using them as transport calls. */
import { parseJson, parseManagementJson, stringifyJson, type AgentAddress, type CurrentReference, type OperationKind, type OperationReceipt, type PreparedOperation, type ScopeAddress } from '@agentx/client';
import type { SubmitDirectoryMessage, SubmitLaunch, SubmitMessage, SubmitTarget } from '../../../management/api/native';
import { domainAgentKey, domainReference, nativeReference } from './projection';

type SpaceOperation = Exclude<OperationKind, 'patch-labels'>;
export type Kind = SpaceOperation | 'stop';
export type SpaceCommand = PreparedOperation<SpaceOperation>;
type Body = SubmitLaunch | SubmitMessage | SubmitTarget | SubmitDirectoryMessage;
export interface Receipt {
  scope: string; listener?: string; id: string; kind: Kind; key: string; path: string; body: Body; operation?: OperationReceipt; issue?: string;
}
const journalKey = 'agentx.space.operations.v1';

export function loadJournal(): Receipt[] {
  try {
    const value: unknown = parseJson(localStorage.getItem(journalKey) ?? '[]');
    return Array.isArray(value) ? value.filter((item): item is Receipt => item && typeof item.scope === 'string' && typeof item.key === 'string' && typeof item.id === 'string' && item.body?.request_id === item.id && typeof item.body.generation === 'string' && typeof item.path === 'string' && item.path.startsWith('/api/v1/')).map(item => {
      try { return { ...item, operation: item.operation ? parseManagementJson('Operation', stringifyJson(item.operation)) : undefined }; }
      catch { return { ...item, operation: undefined, issue: '已保存的回执无法解析；请查询原操作。' }; }
    }) : [];
  } catch { return []; }
}

export function persistJournal(receipts: readonly Receipt[]) {
  localStorage.setItem(journalKey, stringifyJson(receipts));
}

function persistableMessage(message: PreparedOperation<'send'>['message']) {
  return { ...message, attachments: message.attachments && [...message.attachments] };
}

export function makeReceipt(key: string, command: SpaceCommand): Receipt {
  const { kind, operation } = command;
  const { scope, baseUrl: listener, generation } = operation.scope;
  const id = operation.requestId;
  const original = { request_id: id, generation };
  const details = { scope, listener, id, kind, key };
  if (kind === 'launch') return { ...details, path: '/api/v1/agents', body: { ...original, launch: command.launch } };
  if (kind === 'directory-send') return { ...details, path: '/api/v1/directory-messages', body: { ...original, directory: command.directory, message: persistableMessage(command.message) } };
  const agent = nativeReference(command.target.reference);
  if (domainAgentKey(command.target) !== key) throw new Error('原操作与精确会话不匹配');
  const body = { ...original, agent };
  const path = `/api/v1/agents/${encodeURIComponent(agent.name)}/${kind === 'send' ? 'messages' : kind}`;
  return { ...details, path, body: kind === 'send' ? { ...body, message: persistableMessage(command.message) } : body };
}

export function restoreCommand(receipt: Receipt, scope: ScopeAddress): SpaceCommand {
  const { body } = receipt;
  if (receipt.scope !== scope.scope || body.generation !== scope.generation || (receipt.listener !== undefined && receipt.listener !== scope.baseUrl)) throw new Error('原操作属于不同的监听器、作用域或代次');
  const operation = { scope, requestId: receipt.id };
  if (receipt.kind === 'launch' && 'launch' in body) return { kind: 'launch', operation, launch: body.launch };
  if (receipt.kind === 'directory-send' && 'directory' in body) return { kind: 'directory-send', operation, directory: body.directory, message: body.message };
  if ('agent' in body) {
    const reference = domainReference(body.agent);
    const target = { scope, reference };
    if (domainAgentKey(target) !== receipt.key) throw new Error('已保存的操作与精确会话不匹配');
    switch (receipt.kind) {
      case 'transcript': case 'fork': case 'resume': return { kind: receipt.kind, operation, target };
    }
    if (reference.kind === 'current') {
      const current: AgentAddress<CurrentReference> = { scope, reference };
      switch (receipt.kind) {
        case 'send': if ('message' in body) return { kind: 'send', operation, target: current, message: body.message }; break;
        case 'stop': case 'kill': return { kind: 'kill', operation, target: current };
        case 'stop-participation': return { kind: 'stop-participation', operation, target: current };
      }
    }
  }
  throw new Error('未保留原操作的完整信息');
}
