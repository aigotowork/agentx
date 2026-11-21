/** Named public management SDK operations; no parallel wire transport. */
import type { AgentPage, AgentRef, MessagePage, Operation, SubmitMessage, SubmitTarget } from '../../../management/api/native';
import { management } from '../../../management/api/client';
export type { AgentView, AgentRef, Descriptor, MessagePage, Operation, TranscriptPage } from '../../../management/api/native';
export { ManagementError as NativeError } from '@agentx/client';
export const descriptor = (signal?: AbortSignal) => management.describe({ signal });
export interface DirectoryPage { agents: AgentPage['agents']; next: string | null; currentComplete: boolean }
/** Read one selector-bound page; callers decide when additional/history data is needed. */
export async function directory(generation: string, signal?: AbortSignal, currentOnly = true, after?: string): Promise<DirectoryPage> {
  const page = await management.agents.list({ generation, limit: 100, current_only: currentOnly, after }, { signal });
  if (page.generation !== generation) throw new Error('数据代次已改变，请刷新');
  if (page.next && page.next === after) throw new Error('目录游标重复，覆盖未知');
  return { agents: page.agents, next: page.next ?? null, currentComplete: page.current_complete };
}
/** Decode only an exact URL reference; never resolve a stopped execution by name. */
export function directoryReference(key: string | null): AgentRef | null {
  try {
    const value: unknown = JSON.parse(key ?? 'null');
    if (!Array.isArray(value) || value.length !== 3) return null;
    const [name, incarnation, stopped] = value;
    if (typeof name !== 'string' || !/^[A-Za-z0-9_-]{1,128}$/.test(name)) return null;
    if (typeof incarnation === 'string' && /^\d+$/.test(incarnation) && stopped === null) return { name, incarnation, stopped_event: null };
    if (incarnation === null && typeof stopped === 'string' && /^[1-9]\d*$/.test(stopped)) return { name, incarnation: null, stopped_event: stopped };
    return null;
  } catch { return null; }
}
/** Read the requested instance directly when pagination has not loaded its row. */
export async function directoryAgent(generation: string, agent: AgentRef, signal?: AbortSignal) {
  const row = await management.agents.get({ generation, agent }, { signal });
  if (row.agent.name !== agent.name || row.agent.incarnation !== agent.incarnation || row.agent.stopped_event !== agent.stopped_event) throw new Error('会话引用已变化，请重新选择');
  return row;
}
export const messages = async (generation: string, agent: AgentRef, after?: string, signal?: AbortSignal): Promise<MessagePage> => {
  const page = await management.agents.messages.list({ generation, agent, limit: 100, after }, { signal });
  if (page.generation !== generation) throw new Error('消息数据代次已改变，请重新连接');
  return page;
};
export const receipt = (generation: string, requestId: string, signal?: AbortSignal) => management.getOperation({ generation, request_id: requestId }, { signal });
export type TargetAction = 'messages' | 'stop' | 'resume' | 'transcript';
export async function submit(generation: string, agent: AgentRef, action: TargetAction, requestId: string, text?: string): Promise<Operation> {
  const target: SubmitTarget = { generation, request_id: requestId, agent };
  if (action === 'messages') {
    const input: SubmitMessage = { ...target, message: { attachments: [], text: text ?? '', intent: null, reply_to: null, thread: null } };
    return management.send(input);
  }
  if (action === 'stop') return management.agents.kill(target);
  if (action === 'resume') return management.agents.resume(target);
  return management.agents.transcript(target);
}
