import { CloudAdminError, createCloudAdminClient, createCloudScopeClient, type CloudAdminClient, type CloudScopeClient, type CloudScopeTypes } from '@agentx/client';
import { exactAgentKey, type AgentResource, type AgentState } from '../offline/model';
import { sameAgent, type Access, type CloudAgent } from './route';

export type CloudHost = CloudScopeTypes['HostView'];
export type CloudRosterAgent = CloudScopeTypes['CloudAgentSummary'];
export type CloudAgentFacts = CloudScopeTypes['CloudAgentFacts'];
export type Session = { scope: string; generation: string } & (
  { access: 'admin'; client: CloudAdminClient } | { access: 'scope'; client: CloudScopeClient }
);
export type PageQuery = { after?: string; limit: number };
export const cloudPageSize = 20;
export const tokenPattern = /^[A-Za-z0-9_-]{32,256}$/;

export class ScopeChanged extends Error {}
export class ScopeUnavailable extends Error {}

function originFetch(input: RequestInfo | URL, init?: RequestInit) {
  const url = new URL(input instanceof Request ? input.url : input, location.origin);
  if (url.origin !== location.origin) throw new TypeError('Cloud Space only connects to its own origin');
  const caller = init?.signal ?? (input instanceof Request ? input.signal : undefined);
  const timeout = AbortSignal.timeout(url.pathname.endsWith("/changes") ? 35_000 : 15_000);
  return fetch(input, { ...init, signal: caller ? AbortSignal.any([caller, timeout]) : timeout,
    mode: 'same-origin', credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer' });
}

async function adminGeneration(client: CloudAdminClient, scope: string, signal: AbortSignal) {
  const row = await client.getScope(scope, { signal });
  if (row.id !== scope) throw new ScopeChanged();
  if (!row.available) throw new ScopeUnavailable();
  return row.generation;
}

export async function connectCloud(scope: string, access: Access, token: string, signal: AbortSignal): Promise<Session> {
  if (access === 'admin') {
    const client = createCloudAdminClient({ baseUrl: location.origin, token, fetch: originFetch });
    return { scope, access, client, generation: await adminGeneration(client, scope, signal) };
  }
  const client = createCloudScopeClient({ baseUrl: `${location.origin}/scopes/${scope}/api/v1`, token, fetch: originFetch });
  return { scope, access, client, generation: (await client.describe({ signal })).generation };
}

export async function fenced<T>(session: Session, signal: AbortSignal, run: () => Promise<T>) {
  const generation = () => session.access === 'admin'
    ? adminGeneration(session.client, session.scope, signal)
    : session.client.describe({ signal }).then(row => row.generation);
  if (await generation() !== session.generation) throw new ScopeChanged();
  const data = await run();
  if (await generation() !== session.generation) throw new ScopeChanged();
  return data;
}

export function hostsPage(session: Session, query: PageQuery, signal: AbortSignal) {
  return session.access === 'admin'
    ? session.client.listHosts(session.scope, query, { signal })
    : session.client.listHosts(query, { signal });
}

const legacyRoster = new WeakSet<Session>();
export async function agentsPage(session: Session, query: PageQuery & { host?: string }, signal: AbortSignal) {
  const read = (facts?: true) => session.access === 'admin'
    ? session.client.listAgents(session.scope, { ...query, facts }, { signal })
    : session.client.listAgents({ ...query, facts }, { signal });
  if (legacyRoster.has(session)) return read();
  try { return await read(true); }
  catch (error) {
    // Old deployments reject unknown roster query fields. The exact read identity is unchanged.
    if (!(error instanceof CloudAdminError) || error.status !== 400) throw error;
    const page = await read();
    legacyRoster.add(session);
    return page;
  }
}

export function resolveAgent(session: Session, selected: CloudAgent, signal: AbortSignal) {
  return fenced(session, signal, async () => {
    let after: string | undefined;
    do {
      const query = { host: selected.host, after, limit: 100 };
      const page = await agentsPage(session, query, signal);
      const found = page.items.find(agent => sameAgent(agent, selected));
      if (found) return found;
      // The cloud directory contains only one active tuple per name and pages by name.
      if (!page.next || page.items.some(row => row.name >= selected.name)) return null;
      if (after && page.next === after) throw new TypeError('Directory cursor did not advance');
      after = page.next;
    } while (!signal.aborted);
    signal.throwIfAborted();
    return null;
  });
}

const factStates: Record<CloudAgentFacts['status'], AgentState> = { active: 'running', idle: 'idle', error: 'failed', unknown: 'unknown', stopped: 'closed' };

export function projectCloudAgent(agent: CloudRosterAgent, session: Session, supplied?: CloudAgentFacts | null, retired = false): AgentResource {
  const { host, name, incarnation } = agent;
  const facts = supplied ?? agent.facts;
  const ref = { scope: session.scope, generation: session.generation, host, name, incarnation, record: 'current' as const };
  const state = retired ? 'closed' : facts ? factStates[facts.status] : 'unknown';
  return {
    id: exactAgentKey(ref), name, title: facts?.title || `@${name}`, launchTitle: facts?.title, state, attention: state === 'failed',
    ref,
    membership: { kind: 'host', host }, role: facts?.role ? { kind: 'adopted', name: facts.role, source: '', profile: '', revision: '' }
      : facts ? { kind: 'none' } : { kind: 'unknown' },
    description: host, cwd: facts?.cwd ?? null, model: null, tool: facts ? 'codex' : '', profile: null,
    source: { id: host, title: host }, createdAt: '',
  };
}

export function cloudProblem(error: unknown, fallback: string): string {
  if (error instanceof ScopeChanged) return 'Scope 代次已变化，请断开并重新连接核对。';
  if (error instanceof ScopeUnavailable) return '此 Scope 当前不可用。';
  if (error instanceof CloudAdminError) {
    if (error.status === 401 || error.status === 403) return '令牌无效或权限不足，请核对访问方式与令牌。';
    if (error.status === 404) return '此 Scope 或精确实例不存在。';
    if (error.code === 'workspace_alias_taken') return '此工作区名称已被使用，请换一个名称。';
    if (error.code === 'capability_unavailable') return '所选主机能力已变化，请重新选择工作区或岗位。';
    if (error.status === 409) return 'Scope 或实例已变化，请重新连接核对。';
  }
  return fallback;
}

export function cloudTime(seconds: bigint) {
  const value = new Date(Number(seconds) * 1000);
  return Number.isNaN(value.getTime()) ? null : value;
}
