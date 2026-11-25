/** @aiPurpose Adapt authenticated Namespace reads and launch membership to the shared Space supply. */
import { CloudAdminError, createCloudAdminClient, createCloudNamespaceClient, type CloudAdminClient, type CloudNamespaceClient, type CloudNamespaceTypes } from '@agentx/client';
import { exactAgentKey, projectOffice, type AgentResource, type AgentState, type CloudOffice, type Membership } from '../offline/model';
import { sameAgent, type Access, type CloudAgent } from './route';

export type CloudHost = CloudNamespaceTypes['HostView'];
export type CloudWorkspace = CloudNamespaceTypes['WorkspaceView'];
export type CloudRosterAgent = CloudNamespaceTypes['CloudAgentSummary'];
export type CloudAgentFacts = CloudNamespaceTypes['CloudAgentFacts'];
export type CloudSubject = CloudAgent & Partial<Pick<CloudRosterAgent, 'facts' | 'workspace'>>;
export type Session = { namespace: string; generation: string } & (
  { access: 'admin'; client: CloudAdminClient } | { access: 'namespace'; client: CloudNamespaceClient }
);
export type PageQuery = { after?: string; limit: number };
export const cloudPageSize = 20;

export class NamespaceChanged extends Error {}
export class NamespaceUnavailable extends Error {}

function originFetch(input: RequestInfo | URL, init?: RequestInit) {
  const url = new URL(input instanceof Request ? input.url : input, location.origin);
  if (url.origin !== location.origin) throw new TypeError('Cloud Space only connects to its own origin');
  const caller = init?.signal ?? (input instanceof Request ? input.signal : undefined);
  const timeout = AbortSignal.timeout(url.pathname.endsWith("/changes") ? 35_000 : 15_000);
  return fetch(input, { ...init, signal: caller ? AbortSignal.any([caller, timeout]) : timeout,
    mode: 'same-origin', credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer' });
}

async function adminGeneration(client: CloudAdminClient, namespace: string, signal: AbortSignal) {
  const row = await client.getNamespace(namespace, { signal });
  if (row.namespace_id !== namespace) throw new NamespaceChanged();
  if (!row.available) throw new NamespaceUnavailable();
  return row.generation;
}

export async function connectCloud(namespace: string, access: Access, token: string, signal: AbortSignal): Promise<Session> {
  if (access === 'admin') {
    const client = createCloudAdminClient({ baseUrl: location.origin, token, fetch: originFetch });
    return { namespace, access, client, generation: await adminGeneration(client, namespace, signal) };
  }
  const client = createCloudNamespaceClient({ baseUrl: `${location.origin}/namespaces/${namespace}/api/v1`, token, fetch: originFetch });
  return { namespace, access, client, generation: (await client.describe({ signal })).generation };
}

export function hostsPage(session: Session, query: PageQuery, signal: AbortSignal) {
  const request = { ...query, generation: session.generation };
  return session.access === 'admin'
    ? session.client.listHosts(session.namespace, request, { signal })
    : session.client.listHosts(request, { signal });
}

export function workspacesPage(session: Session, query: PageQuery & { host?: string }, signal: AbortSignal) {
  const request = { ...query, generation: session.generation };
  return session.access === 'admin'
    ? session.client.listWorkspaces(session.namespace, request, { signal })
    : session.client.listWorkspaces(request, { signal });
}

export function agentsPage(session: Session, query: PageQuery & { host?: string; workspace_id?: string }, signal: AbortSignal) {
  const request = { ...query, facts: true, generation: session.generation };
  return session.access === 'admin'
    ? session.client.listAgents(session.namespace, request, { signal })
    : session.client.listAgents(request, { signal });
}

export async function resolveAgent(session: Session, selected: CloudAgent, signal: AbortSignal) {
  let after: string | undefined;
  for (;;) {
    const page = await agentsPage(session, { host: selected.host, after, limit: 100 }, signal);
    signal.throwIfAborted();
    const found = page.items.find(agent => sameAgent(agent, selected));
    if (found) return found;
    // The cloud directory contains only one active tuple per name and pages by name.
    if (!page.next || page.items.some(row => row.name >= selected.name)) return null;
    if (after && page.next === after) throw new TypeError('Directory cursor did not advance');
    after = page.next;
  }
}

const factStates: Record<CloudAgentFacts['status'], AgentState> = { active: 'running', idle: 'idle', error: 'failed', unknown: 'unknown', stopped: 'closed' };

/** @aiPurpose Build an exact project office from the published catalogue. @aiUsage Use the tuple key internally and the display name in the UI. */
export function projectCloudOffice(row: CloudWorkspace): CloudOffice {
  const { host, workspace_id, name, online, roles } = row;
  return { id: projectOffice(host, workspace_id), title: name, host, workspaceId: workspace_id, origin: 'cloud-project', online, roles };
}

export function projectCloudAgent(agent: CloudSubject, session: Session, supplied?: CloudAgentFacts | null, retired = false): AgentResource {
  const { host, name, incarnation } = agent;
  const facts = supplied ?? agent.facts;
  const ref = { scope: session.namespace, generation: session.generation, host, name, incarnation, record: 'current' as const };
  const state = retired ? 'closed' : facts ? factStates[facts.status] : 'unknown';
  const workspace = agent.workspace;
  const membership: Membership = workspace?.kind === 'project' ? { kind: 'project', host, workspaceId: workspace.workspace_id }
    : { kind: workspace?.kind ?? 'unknown' };
  return {
    id: exactAgentKey(ref), name, title: facts?.title || `@${name}`, launchTitle: facts?.title, state, attention: state === 'failed',
    ref,
    membership, role: facts?.role ? { kind: 'adopted', name: facts.role, source: '', profile: '', revision: '' }
      : facts ? { kind: 'none' } : { kind: 'unknown' },
    description: host, cwd: facts?.cwd ?? null, model: null, tool: facts ? 'codex' : '', profile: null,
    source: { id: host, title: host }, createdAt: '',
  };
}

export function unsupportedCloudOperation(error: unknown) {
  return error instanceof CloudAdminError && error.code === 'unsupported_operation';
}

export function cloudProblem(error: unknown, fallback: string): string {
  if (error instanceof NamespaceChanged) return 'Namespace 代次已变化，请断开并重新连接核对。';
  if (error instanceof NamespaceUnavailable) return '此 Namespace 当前不可用。';
  if (error instanceof CloudAdminError) {
    if (error.code === 'unsupported_operation') return '此服务不支持这项操作。';
    if (error.code === 'stale_generation') return 'Namespace 代次已变化，请断开并重新连接核对。';
    if (error.status === 401 || error.status === 403) return '令牌无效或权限不足，请核对访问方式与令牌。';
    if (error.status === 404) return '此 Namespace 或精确实例不存在。';
    if (error.code === 'capability_unavailable') return '所选主机能力已变化，请重新选择工作区或岗位。';
    if (error.status === 409) return 'Namespace 或实例已变化，请重新连接核对。';
  }
  return fallback;
}

export function cloudTime(seconds: bigint) {
  const value = new Date(Number(seconds) * 1000);
  return Number.isNaN(value.getTime()) ? null : value;
}
