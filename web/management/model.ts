/** @aiPurpose Own exact identities and the single durable browser write journal. */
import { parseJson, stringifyJson, parseManagementJson } from '@agentx/client';
import type { AgentRef, Operation, SubmitLaunch, SubmitMessage, SubmitTarget, SubmitDirectoryMessage, SubmitLabels } from './api/native';

export type WriteBody = SubmitLaunch | SubmitMessage | SubmitTarget | SubmitDirectoryMessage | SubmitLabels;
export type WriteKind = 'launch' | 'directory-send' | 'send' | 'stop' | 'resume' | 'transcript' | 'labels';
export interface Receipt {
  id: string;
  path: string;
  body: WriteBody;
  kind: WriteKind;
  label: string;
  at: string;
  operation?: Operation;
  issue?: string;
  missing?: boolean;
  rejected?: boolean;
}
export const recoveryKey = 'agentx.management.operations.v1';
export const refKey = (agent: AgentRef): string => JSON.stringify([agent.name, agent.incarnation, agent.stopped_event]);
export const activeOperation = (receipt: Receipt): boolean => !receipt.rejected && (!receipt.operation || ['Accepted', 'Running'].includes(receipt.operation.status));
export const needsAttention = (receipt: Receipt): boolean => activeOperation(receipt) || receipt.operation?.status === 'Unknown';
export const messageOf = (error: unknown): string => error instanceof Error ? error.message : String(error);

export function readStored<T>(key: string, fallback: T): T {
  try { return JSON.parse(localStorage.getItem(key) ?? 'null') ?? fallback; } catch { return fallback; }
}

export function loadReceipts(suffix = ''): Receipt[] {
  let value: unknown;
  try { value = parseJson(localStorage.getItem(recoveryKey + suffix) ?? '[]'); } catch { return []; }
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is Receipt => !!item && typeof item.id === 'string' && typeof item.path === 'string' && item.body?.request_id === item.id && typeof item.body?.generation === 'string' && (item.path.startsWith('/api/v1/agents') || item.path === '/api/v1/directory-messages')).map(item => {
    try { return { ...item, operation: item.operation ? parseManagementJson('Operation', stringifyJson(item.operation)) : undefined }; }
    catch { return { ...item, operation: undefined, issue: '已保存的回执无法解析；请查询原操作。' }; }
  });
}

export function saveReceipts(receipts: Receipt[], suffix = ''): void {
  // Retain all unresolved work; bound only old terminal receipts.
  let terminal = 0;
  const bounded = receipts.filter(receipt => activeOperation(receipt) || receipt.operation?.status === 'Unknown' || ++terminal <= 40);
  localStorage.setItem(recoveryKey + suffix, stringifyJson(bounded));
}

/** Generate an RFC 4122 v4 ID on trusted LAN HTTP as well as secure origins. */
export function requestId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const value = [...bytes].map(byte => byte.toString(16).padStart(2, '0')).join('');
  return `${value.slice(0, 8)}-${value.slice(8, 12)}-${value.slice(12, 16)}-${value.slice(16, 20)}-${value.slice(20)}`;
}
