import type { CloudScopeTypes } from '@agentx/client';
import { defaultRoute, parseRoute, routeSearch, type NavigationCodec } from '../navigation/route';

export type Access = 'admin' | 'scope';
export type CloudAgent = CloudScopeTypes['AgentView'];

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
export function sameAgent(first: CloudAgent, second: CloudAgent) {
  return first.host === second.host && first.name === second.name && first.incarnation === second.incarnation;
}
export function exactLabel(agent: CloudAgent) { return `@${agent.name} · ${agent.host} · ${agent.incarnation}`; }

/** Decode the existing scoped URLs into the shared navigation model. */
export function cloudNavigation(scope: string, currentGeneration: string): NavigationCodec {
  return {
    parse(search) {
      const query = new URLSearchParams(search);
      const hasExact = exactKeys.some(key => query.has(key));
      const host = query.get('host'), name = query.get('name'), incarnation = query.get('incarnation'), generation = query.get('generation');
      if (hasExact && (exactKeys.some(key => query.getAll(key).length > 1)
        || !host || !hostPattern.test(host) || !name || !namePattern.test(name)
        || !incarnation || !opaquePattern.test(incarnation) || (generation !== null && !opaquePattern.test(generation)))) {
        return { route: defaultRoute(), issue: '需要完整有效的云端 Agent 精确引用' };
      }
      exactKeys.forEach(key => query.delete(key));
      if (['scope', 'agent', 'record', 'stopped_event'].some(key => query.has(key))) return { route: defaultRoute(), issue: '云端会话链接需要主机与实例引用' };
      // The common parser validates detail with a target, so defer that flag to this caller.
      const detail = query.get('detail');
      if (detail === 'agent' && hasExact && query.getAll('detail').length === 1) query.delete('detail');
      if (query.get('view') === 'list' && query.getAll('view').length === 1) query.set('view', 'chat');
      const parsed = parseRoute(query.toString());
      if (parsed.issue) return parsed;
      if (host && name && incarnation) {
        parsed.route.target = { scope, generation: generation ?? currentGeneration, host, name, incarnation, stopped_event: null, record: 'current' };
        parsed.route.detail = detail === 'agent';
      }
      return parsed;
    },
    search(search, route) {
      const query = new URLSearchParams(search);
      exactKeys.forEach(key => query.delete(key));
      const common = new URLSearchParams(routeSearch(query.toString(), { ...route, target: null }));
      if (route.presentation === 'chat') common.set('view', 'list');
      const target = route.target;
      if (target?.host && target.incarnation) {
        common.set('host', target.host);
        common.set('name', target.name);
        common.set('incarnation', target.incarnation);
        common.set('generation', target.generation);
        if (route.channel !== 'messages') common.set('channel', route.channel);
        if (route.detail) common.set('detail', 'agent');
      }
      return common.size ? `?${common}` : '';
    },
    normalize(search, route) {
      if (!route.target || new URLSearchParams(search).has('generation')) return search;
      const query = new URLSearchParams(search);
      query.set('generation', route.target.generation);
      return `?${query}`;
    },
  };
}
