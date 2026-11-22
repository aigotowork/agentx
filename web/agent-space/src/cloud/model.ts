import { CloudAdminError, createCloudAdminClient, createCloudScopeClient, type CloudAdminClient, type CloudScopeClient, type CloudScopeTypes } from '@agentx/client';
import type { AgentResource } from '../offline/model';
import { exactKey, sameAgent, type Access, type CloudAgent } from './route';

export type CloudHost = CloudScopeTypes['HostView'];
export type Session = { scope: string; generation: string } & (
  { access: 'admin'; client: CloudAdminClient } | { access: 'scope'; client: CloudScopeClient }
);
export type PageQuery = { after?: string; limit: number };
export const cloudPageSize = 20;
export const tokenPattern = /^[A-Za-z0-9_-]{32,256}$/;

class ScopeChanged extends Error {}
class ScopeUnavailable extends Error {}

function originFetch(input: RequestInfo | URL, init?: RequestInit) {
  const url = new URL(input instanceof Request ? input.url : input, location.origin);
  if (url.origin !== location.origin) throw new TypeError('Cloud Space only connects to its own origin');
  const caller = init?.signal ?? (input instanceof Request ? input.signal : undefined);
  const timeout = AbortSignal.timeout(15_000);
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
  return fenced(session, signal, () => session.access === 'admin'
    ? session.client.listHosts(session.scope, query, { signal })
    : session.client.listHosts(query, { signal }));
}

export function agentsPage(session: Session, query: PageQuery & { host?: string }, signal: AbortSignal) {
  return fenced(session, signal, () => session.access === 'admin'
    ? session.client.listAgents(session.scope, query, { signal })
    : session.client.listAgents(query, { signal }));
}

export function resolveAdminAgent(session: Session & { access: 'admin' }, selected: CloudAgent, signal: AbortSignal) {
  return fenced(session, signal, async () => {
    let after: string | undefined;
    do {
      const page = await session.client.listAgents(session.scope, { host: selected.host, after, limit: 100 }, { signal });
      const found = page.items.find(agent => sameAgent(agent, selected));
      if (found) return found;
      // The cloud directory contains only one active tuple per name and pages by name.
      if (!page.next || page.items.some(row => row.name >= selected.name)) return null;
      if (after && page.next <= after) throw new TypeError('Directory cursor did not advance');
      after = page.next;
    } while (!signal.aborted);
    signal.throwIfAborted();
    return null;
  });
}

export function projectCloudAgent(agent: CloudAgent, session: Session, room: string): AgentResource {
  return {
    id: exactKey(agent), name: agent.name, title: agent.name, state: 'unknown', attention: false,
    ref: { scope: session.scope, generation: session.generation, name: agent.name, incarnation: agent.incarnation, record: 'current' },
    membership: { kind: 'directory', path: room }, role: { kind: 'unknown' },
    description: agent.host, cwd: null, model: null, tool: '', profile: null,
    source: { id: agent.host, title: agent.host }, createdAt: '',
  };
}

export function cloudProblem(error: unknown, fallback: string): string {
  if (error instanceof ScopeChanged) return 'Scope 代次已变化，请断开并重新连接核对。';
  if (error instanceof ScopeUnavailable) return '此 Scope 当前不可用。';
  if (error instanceof CloudAdminError) {
    if (error.status === 401 || error.status === 403) return '令牌无效或权限不足，请核对访问方式与令牌。';
    if (error.status === 404) return '此 Scope 或精确实例不存在。';
    if (error.status === 409) return 'Scope 或实例已变化，请重新连接核对。';
  }
  return fallback;
}

export function cloudTime(seconds: bigint) {
  const value = new Date(Number(seconds) * 1000);
  return Number.isNaN(value.getTime()) ? null : value;
}
