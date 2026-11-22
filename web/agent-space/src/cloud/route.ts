import type { CloudScopeTypes } from '@agentx/client';

export type Access = 'admin' | 'scope';
export type CloudAgent = CloudScopeTypes['AgentView'];
export type ExactSelection = CloudAgent & { generation?: string };
export type CloudRoute = { view: 'scene' | 'list'; selection: ExactSelection | null; invalid: boolean };

const scopePattern = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;
const hostPattern = /^[a-z0-9][a-z0-9_.-]{0,252}$/;
const namePattern = /^[a-z0-9_-]{1,64}$/;
const opaquePattern = /^[A-Za-z0-9_.:-]{1,128}$/;
const exactKeys = ['host', 'name', 'incarnation', 'generation'] as const;

export function cloudEntry(path: string): string | null {
  return /^\/scopes\/([^/]+)\/space\/?$/.exec(path)?.[1] ?? null;
}
export function validScope(id: string) { return scopePattern.test(id); }
export function accessHint(search: string): Access {
  return new URLSearchParams(search).get('access') === 'admin' ? 'admin' : 'scope';
}
export function exactKey(agent: CloudAgent) { return JSON.stringify([agent.host, agent.name, agent.incarnation]); }
export function sameAgent(first: CloudAgent, second: CloudAgent) {
  return first.host === second.host && first.name === second.name && first.incarnation === second.incarnation;
}
export function exactLabel(agent: CloudAgent) { return `@${agent.name} · ${agent.host} · ${agent.incarnation}`; }

export function readCloudRoute(search: string): CloudRoute {
  const query = new URLSearchParams(search);
  const view = query.get('view') === 'list' ? 'list' : 'scene';
  if (!exactKeys.some(key => query.has(key))) return { view, selection: null, invalid: false };
  const host = query.get('host'), name = query.get('name'), incarnation = query.get('incarnation'), generation = query.get('generation');
  if (exactKeys.some(key => query.getAll(key).length > 1) || !host || !hostPattern.test(host) || !name || !namePattern.test(name)
    || !incarnation || !opaquePattern.test(incarnation) || (generation !== null && !opaquePattern.test(generation))) {
    return { view, selection: null, invalid: true };
  }
  return { view, selection: { host, name, incarnation, generation: generation ?? undefined }, invalid: false };
}

export function writeCloudRoute(route: CloudRoute, replace = false) {
  const url = new URL(location.href);
  for (const key of exactKeys) {
    const value = route.selection?.[key];
    if (value) url.searchParams.set(key, value);
    else url.searchParams.delete(key);
  }
  if (route.view === 'list') url.searchParams.set('view', 'list');
  else url.searchParams.delete('view');
  if (url.href !== location.href) history[replace ? 'replaceState' : 'pushState'](history.state, '', url);
}
