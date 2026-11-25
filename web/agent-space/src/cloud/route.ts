/** @aiPurpose Translate public Namespace links into the shared Office and exact-reader model. */
import { cloudIncarnationPattern, cloudWorkspaceIdPattern, type CloudNamespaceTypes } from '@agentx/client';
import { defaultRoute, parseRoute, routeSearch, type NavigationCodec } from '../navigation/route';
import { hostGroup, projectOffice } from '../offline/model';

export type Access = 'admin' | 'namespace';
export type CloudAgent = Pick<CloudNamespaceTypes['AgentView'], 'host' | 'name' | 'incarnation'>;

const namespacePattern = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;
const hostPattern = /^[a-z0-9][a-z0-9_.-]{0,252}$/;
const namePattern = /^[a-z0-9_-]{1,64}$/;
const opaquePattern = /^[A-Za-z0-9_.:-]{1,128}$/;
const exactKeys = ['name', 'incarnation', 'generation'] as const;
const cloudKeys = ['host', 'workspace_id', 'group', ...exactKeys] as const;

export function cloudEntry(path: string): string | null {
  return /^\/namespaces\/([^/]+)\/space\/?$/.exec(path)?.[1] ?? null;
}
export function validNamespace(id: string) { return namespacePattern.test(id); }
export function accessHint(search: string): Access {
  return new URLSearchParams(search).get('access') === 'admin' ? 'admin' : 'namespace';
}
export function sameAgent(first: CloudAgent, second: CloudAgent) {
  return first.host === second.host && first.name === second.name && first.incarnation === second.incarnation;
}
export function exactLabel(agent: CloudAgent) { return `@${agent.name} · ${agent.host} · ${agent.incarnation}`; }

/** Decode an internal project key or the distinct host overview into network query fields. */
export function cloudOfficeQuery(office: string): { host?: string; workspace_id?: string } {
  const project = /^project:([^:]+):(.+)$/.exec(office);
  return project ? { host: project[1], workspace_id: project[2] } : office.startsWith('host:') ? { host: office.slice(5) } : {};
}

/** Decode nonsecret Namespace URLs into shared navigation; no local paths are accepted. */
export function cloudNavigation(namespace: string, currentGeneration: string): NavigationCodec {
  return {
    parse(search) {
      const query = new URLSearchParams(search);
      const hasExact = exactKeys.some(key => query.has(key));
      const host = query.get('host'), name = query.get('name'), incarnation = query.get('incarnation'), generation = query.get('generation');
      const workspace = query.get('workspace_id'), group = query.get('group');
      if (cloudKeys.some(key => query.getAll(key).length > 1) || host !== null && !hostPattern.test(host)
        || workspace !== null && (!host || !cloudWorkspaceIdPattern.test(workspace))
        || group !== null && (group !== 'host' || !host || !!workspace)) {
        return { route: defaultRoute(), issue: '云端项目链接需要有效的主机和工作区标识' };
      }
      if (hasExact && (!host || !name || !namePattern.test(name)
        || !incarnation || !cloudIncarnationPattern.test(incarnation) || (generation !== null && !opaquePattern.test(generation)))) {
        return { route: defaultRoute(), issue: '需要完整有效的云端 Agent 精确引用' };
      }
      cloudKeys.forEach(key => query.delete(key));
      if (['office', 'scope', 'agent', 'record', 'stopped_event'].some(key => query.has(key))) return { route: defaultRoute(), issue: '云端链接需要主机与工作区或实例引用' };
      // The common parser validates detail with a target, so defer that flag to this caller.
      const detail = query.get('detail');
      if (detail === 'agent' && hasExact && query.getAll('detail').length === 1) query.delete('detail');
      if (query.get('view') === 'list' && query.getAll('view').length === 1) query.set('view', 'chat');
      const parsed = parseRoute(query.toString());
      if (parsed.issue) return parsed;
      if (host) {
        if (workspace) parsed.route.workspaceId = projectOffice(host, workspace);
        else if (!hasExact || group === 'host') parsed.route.workspaceId = hostGroup(host);
      }
      if (host && name && incarnation) {
        if (!['members', 'history'].includes(parsed.route.officeView)) return { route: defaultRoute(), issue: '导航面板与资源不匹配' };
        parsed.route.target = { scope: namespace, generation: generation ?? currentGeneration, host, name, incarnation, stopped_event: null, record: 'current' };
        parsed.route.detail = detail === 'agent';
      }
      return parsed;
    },
    search(search, route) {
      const query = new URLSearchParams(search);
      cloudKeys.forEach(key => query.delete(key));
      const common = new URLSearchParams(routeSearch(query.toString(), { ...route, workspaceId: '', target: null }));
      const office = cloudOfficeQuery(route.workspaceId);
      if (office.host) common.set('host', office.host);
      if (office.workspace_id) common.set('workspace_id', office.workspace_id);
      if (route.presentation === 'chat') common.set('view', 'list');
      const target = route.target;
      if (target?.host && target.incarnation) {
        common.set('host', target.host);
        if (!office.workspace_id && office.host === target.host) common.set('group', 'host');
        common.set('name', target.name);
        common.set('incarnation', target.incarnation);
        common.set('generation', target.generation);
        if (route.channel !== 'messages') common.set('channel', route.channel);
        if (route.detail) common.set('detail', 'agent');
      }
      return common.size ? `?${common}` : '';
    },
    normalize(search, route) {
      const query = new URLSearchParams(search);
      if (route.target && !query.has('generation')) {
        query.set('generation', route.target.generation);
        return `?${query}`;
      }
      return search;
    },
  };
}
