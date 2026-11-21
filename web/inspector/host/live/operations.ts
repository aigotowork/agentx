import { parseJson, stringifyJson, parseManagementJson } from '@agentx/client';
/** Browser command journal stores reconciliation identity before any POST. Native receipts own results. */
import type { ManagementTypes } from '@agentx/client';
import type { AgentRef, Operation } from './client';
import type { Dataset } from './model';
import { datasetKey, isPending, referenceKey } from './model';
import { NativeError, descriptor, receipt, submit, type TargetAction } from './client';
export interface CommandEntry {
  requestId: string;
  agent: AgentRef;
  action: TargetAction;
  operation: Operation | null;
  input?: ManagementTypes['SubmitMessage'] | ManagementTypes['SubmitTarget'];
  error: string | null;
  refused?: boolean;
}
export interface CommandStorage { getItem(key: string): string | null; setItem(key: string, value: string): void }
const storageKey = (dataset: Dataset) => `agentx-inspector:commands:v1:${datasetKey(dataset)}`;
export function loadCommands(dataset: Dataset, storage: CommandStorage): CommandEntry[] {
  try {
    const entries: unknown = parseJson(storage.getItem(storageKey(dataset)) ?? '[]');
    if (!Array.isArray(entries)) return [];
    return entries.filter((item): item is CommandEntry => !!item && typeof item.requestId === 'string' && typeof item.agent?.name === 'string' && ['messages', 'stop', 'resume', 'transcript'].includes(item.action)).map(item => {
      try { return { ...item, operation: item.operation ? parseManagementJson('Operation', stringifyJson(item.operation)) : null }; }
      catch { return { ...item, operation: null, error: '已保存的回执无法解析；请查询原操作。' }; }
    });
  } catch { return []; }
}
export function saveCommands(dataset: Dataset, entries: CommandEntry[], storage: CommandStorage): void {
  storage.setItem(storageKey(dataset), stringifyJson(entries));
}
export function blocksTarget(entry: CommandEntry, agent: AgentRef): boolean {
  return !entry.refused && referenceKey(entry.agent) === referenceKey(agent) && (!entry.operation || isPending(entry.operation.status));
}
export function requestId(): string {
  if (typeof crypto.randomUUID === 'function') return crypto.randomUUID();
  // Plain LAN HTTP may lack randomUUID; getRandomValues is still available there.
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
export async function runCommand(dataset: Dataset, agent: AgentRef, action: TargetAction, text: string | undefined, journal: (entry: CommandEntry) => void): Promise<CommandEntry> {
  const current = await descriptor();
  if (datasetKey(current) !== datasetKey(dataset)) throw new Error('数据集已改变，请刷新后重新选择对象');
  const id = requestId();
  const target = { generation: dataset.generation, request_id: id, agent: structuredClone(agent) };
  const input = action === 'messages' ? { ...target, message: { attachments: [], text: text ?? '', intent: null, reply_to: null, thread: null } } : target;
  const entry: CommandEntry = { requestId: id, agent: target.agent, action, input, operation: null, error: null };
  journal(entry); // Failure to persist aborts submission.
  try {
    const operation = await submit(dataset.generation, agent, action, entry.requestId, text);
    if (operation.generation !== dataset.generation || operation.request_id !== entry.requestId) throw new Error('操作回执身份不匹配');
    const result = { ...entry, operation };
    journal(result);
    return result;
  } catch (error) {
    const refused = error instanceof NativeError && error.status >= 400 && error.status < 500;
    const result = { ...entry, refused, error: refused ? `原生请求已拒绝：${String(error)}` : `${String(error)} · 结果未知，仅可查询原回执` };
    journal(result);
    return result;
  }
}
export async function reconcile(dataset: Dataset, entry: CommandEntry): Promise<CommandEntry> {
  const operation = await receipt(dataset.generation, entry.requestId);
  if (operation.generation !== dataset.generation || operation.request_id !== entry.requestId) throw new Error('操作回执身份不匹配');
  return { ...entry, operation, error: null };
}
