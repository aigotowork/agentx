/** Native contracts own facts; this module only projects them into the existing scene. */
import type { AgentAddress, AgentReference, AgentSnapshot, Message, ReadonlyValue, RoleCatalog, Transcript, WorkspaceCatalog as DomainCatalog } from '@agentx/client';
import type { AgentRef, AgentView, Descriptor } from '../../../management/api/native';
import { exactAgentKey, type AgentResource, type AgentSpaceSupply, type HistoryRecord, type Post, type SuppliedRecords } from '../offline/model';

export function projectAgent(native: ReadonlyValue<AgentView>, descriptor: Pick<Descriptor, 'scope' | 'generation'>): AgentResource {
  const stopped = native.agent.stopped_event != null;
  const ref: AgentResource['ref'] = { scope: descriptor.scope, generation: descriptor.generation, ...native.agent, incarnation: native.agent.incarnation ?? null, stopped_event: native.agent.stopped_event ?? null, record: stopped ? 'stopped' : 'current' };
  const collaboration = native.collaboration, role = collaboration.role;
  const states: Record<string, AgentResource['state']> = { active: 'running', working: 'running', listening: 'idle', idle: 'idle', ready: 'ready', inactive: 'idle', failed: 'failed', error: 'failed', blocked: 'failed', stopped: 'closed' };
  return {
    fileBindings: native.file_bindings,
    id: exactAgentKey(ref), name: native.agent.name, title: `@${native.agent.name}`, launchTitle: native.title, nativeStatus: native.status, statusContext: native.status_context, storedStatus: native.stored_status, requestError: native.request_error, pendingMessages: native.pending_messages, description: [native.status, native.detail, collaboration.reason].filter(Boolean).join(' · '),
    state: stopped ? 'closed' : states[native.status] ?? 'unknown', attention: native.status === 'failed' || native.status === 'blocked', ref,
    membership: collaboration.status === 'adopted' ? collaboration.workspace == null ? { kind: 'global' } : { kind: 'directory', path: collaboration.workspace } : { kind: collaboration.status === 'legacy' ? 'legacy' : 'unknown' },
    role: collaboration.status === 'adopted' ? role ? { kind: 'adopted', name: role.name, source: role.path, profile: role.profile, revision: '未提供', description: role.description ?? null } : { kind: 'none' } : { kind: 'unknown' },
    cwd: native.directory || null, model: null, tool: native.tool, profile: null,
    source: { id: descriptor.scope, title: 'AgentX 原生 API' }, createdAt: '',
  };
}
export function projectPosts(catalog: RoleCatalog): Post[] {
  const workspace = catalog.workspace.address.key;
  const office = workspace === 'global' ? '$global' : workspace;
  return catalog.roles.map(({ definition: role }) => ({ id: JSON.stringify([office, role.path]), office, name: role.name, origin: role.namespace.startsWith('global:') ? '全局' : role.namespace === `local:${workspace}/.agentx/roles` ? '本地' : '继承', source: role.path, description: role.description ?? '', profile: role.profile ?? '默认启动配置', revision: '未提供' }));
}
export function messageRecords(messages: readonly Message[]): SuppliedRecords {
  return { status: 'available', provenance: 'AgentX 原生通信记录；接受消息不代表模型已完成。', items: messages.map(message => ({ id: message.id, kind: 'message', native: true, role: 'user', text: message.text, attachments: message.attachments, at: message.at, author: message.from, recipients: message.recipients, intent: message.intent ?? undefined, thread: message.thread ?? undefined, replyTo: message.reply_to ?? undefined })) };
}
export function transcriptRecords(page: Transcript): SuppliedRecords {
  if (!page.available) return { status: 'unavailable', reason: page.reason ?? 'Provider 未提供记录' };
  const items: HistoryRecord[] = [];
  for (const exchange of page.exchanges) {
    if (exchange.user) items.push({ id: `${exchange.position}:user`, kind: 'message', native: true, role: 'user', text: exchange.user, at: exchange.at, author: 'Provider 用户' });
    if (exchange.assistant) items.push({ id: `${exchange.position}:assistant`, kind: 'message', native: true, role: 'assistant', text: exchange.assistant, at: exchange.at, author: 'Provider 助手' });
    if (exchange.action || exchange.files.length) items.push({ id: `${exchange.position}:action`, kind: 'message', native: true, role: 'assistant', text: [exchange.action, ...exchange.files].filter(Boolean).join('\n'), at: exchange.at, author: 'Provider 操作记录' });
  }
  return { status: 'available', provenance: 'Provider 原生有界 exchanges；不包含未供给的思考或工具事件。', items };
}
export function emptySupply(): AgentSpaceSupply {
  return { mode: 'live', scope: { id: '', generation: '', coverage: '连接中' }, workspaces: [], posts: [], agents: [], records: () => ({ status: 'unavailable', reason: '尚未读取' }) };
}
export function domainReference(ref: AgentRef): AgentReference {
  if (ref.incarnation != null && ref.stopped_event == null) return { kind: 'current', name: ref.name, incarnation: ref.incarnation };
  if (ref.stopped_event != null && ref.incarnation == null) return { kind: 'stopped', name: ref.name, stopped_event: ref.stopped_event };
  throw new Error('需要一个精确 incarnation 或停止事件');
}

export function nativeReference(ref: AgentReference): AgentRef {
  return { name: ref.name, incarnation: ref.kind === 'current' ? ref.incarnation : null, stopped_event: ref.kind === 'stopped' ? ref.stopped_event : null };
}

export function domainAgentKey({ scope, reference }: AgentAddress) {
  const incarnation = reference.kind === 'current' ? reference.incarnation : null;
  const stopped_event = reference.kind === 'stopped' ? reference.stopped_event : null;
  return exactAgentKey({ scope: scope.scope, generation: scope.generation, name: reference.name, incarnation, stopped_event, record: reference.kind });
}

/** Give the existing capability consumers the native facts represented by a verified SDK snapshot. */
export function nativeSnapshot({ address, facts, membership }: AgentSnapshot): ReadonlyValue<AgentView> {
  const agent = nativeReference(address.reference);
  const collaboration = membership.kind === 'adopted' ? { status: 'adopted' as const, workspace: membership.workspace === 'global' ? null : membership.workspace, role: membership.role }
    : membership.kind === 'legacy' ? { status: 'legacy' as const } : { status: 'unavailable' as const, reason: membership.reason };
  return { ...facts, agent, collaboration };
}

export function domainCoverage(catalog: DomainCatalog | undefined, error: string) {
  if (!catalog) return error ? `办公室读取失败：${error}` : '办公室读取中';
  const coverage = catalog.coverage;
  return [coverage.agent_limit_reached ? '目录的成员扫描达到上限' : '有界目录扫描', coverage.scan_limit_reached ? '目录扫描达到上限' : '', ...coverage.errors].filter(Boolean).join(' · ');
}
