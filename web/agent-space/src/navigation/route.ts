/** Copyable navigation values; no drafts, effects or camera memory. */
import type { SpaceSnapshot } from '../offline/controller';
import { findConversation, type AgentSpaceSupply, type RecordChannel } from '../offline/model';
export interface RouteTarget { scope: string; generation: string; host?: string; name: string; incarnation: string | null; stopped_event: string | null; record: 'current' | 'stopped' }
export type SpaceRoute = Pick<SpaceSnapshot, 'workspaceId' | 'presentation' | 'officeView' | 'postId' | 'query' | 'state' | 'panel' | 'detail' | 'mail' | 'before'> & { inferOffice?: boolean; target: RouteTarget | null; channel: RecordChannel };
export type NavigationCodec = { parse(search: string): ParsedRoute; search(search: string, route: SpaceRoute): string; normalize?(search: string, route: SpaceRoute): string };
export type ParsedRoute = { route: SpaceRoute; issue: string };
const owned = ['office', 'view', 'tab', 'post', 'q', 'state', 'scope', 'generation', 'agent', 'incarnation', 'stopped_event', 'record', 'channel', 'panel', 'detail', 'mail', 'before'];
export const defaultRoute = (): SpaceRoute => ({ workspaceId: '', presentation: 'scene', officeView: 'members', postId: null, query: '', state: '', target: null, channel: 'messages', panel: '', detail: false, mail: 'pending', before: '' });
/** Validate bounded route syntax; resource existence is resolved against asynchronous supply. */
export function parseRoute(search: string): ParsedRoute {
  const route = defaultRoute(), params = new URLSearchParams(search);
  try {
    for (const key of owned) {
      if (params.getAll(key).length > 1) throw new Error('导航查询参数重复');
      if ((params.get(key)?.length ?? 0) > (key === 'q' ? 512 : 4096)) throw new Error('导航查询参数过长');
    }
    function choice<T extends string>(key: string, values: readonly T[], fallback: T): T {
      const value = params.get(key); if (value === null) return fallback;
      if (!values.includes(value as T)) throw new Error(`${key} 导航参数无效`);
      return value as T;
    }
    route.workspaceId = params.get('office') ?? '';
    route.presentation = choice('view', ['scene', 'chat'], 'scene');
    route.officeView = choice('tab', ['members', 'history', 'posts', 'skills'], 'members');
    route.postId = params.get('post'); route.query = params.get('q') ?? '';
    route.state = choice('state', ['', 'running', 'idle', 'ready', 'closed', 'failed', 'unknown'], '');
    route.channel = choice('channel', ['messages', 'model'], 'messages');
    route.panel = choice('panel', ['', 'directory', 'mailbox'], '');
    route.detail = choice('detail', ['', 'agent'], '') === 'agent';
    route.mail = choice('mail', ['pending', 'all'], 'pending'); route.before = params.get('before') ?? '';
    const identity = ['scope', 'generation', 'agent', 'incarnation', 'stopped_event', 'record'];
    if (identity.some(key => params.has(key))) {
      const scope = params.get('scope'), generation = params.get('generation'), name = params.get('agent');
      const incarnation = params.get('incarnation'), stopped_event = params.get('stopped_event');
      const record = choice('record', ['current', 'stopped'], stopped_event ? 'stopped' : 'current');
      const demo = params.get('supply') === 'demo';
      if (!scope || !generation || !name || name.length > 128 || generation.length > 128 || (!incarnation && !stopped_event) || (incarnation && stopped_event) || (!demo && !/^\d{1,24}$/.test((incarnation ?? stopped_event)!)) || (record === 'current' && !incarnation) || (!demo && record === 'stopped' && !stopped_event)) throw new Error('需要完整有效的 Agent 精确引用');
      route.inferOffice = !params.has('office') && !params.has('record');
      route.target = { scope, generation, name, incarnation, stopped_event, record };
      if (!params.has('tab')) route.officeView = record === 'stopped' ? 'history' : 'members';
    }
    if (route.target && !['members', 'history'].includes(route.officeView) || route.postId && route.officeView !== 'posts' || route.panel && (route.presentation !== 'scene' || !['members', 'history'].includes(route.officeView)) || route.detail && !route.target) throw new Error('导航面板与资源不匹配');
    return { route, issue: '' };
  } catch (error) { return { route: defaultRoute(), issue: error instanceof Error ? error.message : String(error) }; }
}
/** Snapshot exact supplied identity only while its reader is open. */
export function routeFromState(state: SpaceSnapshot, supply: AgentSpaceSupply): SpaceRoute {
  const agent = state.reading ? findConversation(supply, state.selectedId) : undefined;
  const target = agent ? { scope: agent.ref.scope, generation: agent.ref.generation, ...(agent.ref.host === undefined ? {} : { host: agent.ref.host }), name: agent.name, incarnation: agent.ref.incarnation, stopped_event: agent.ref.stopped_event ?? null, record: agent.ref.record } : null;
  return { workspaceId: state.workspaceId, presentation: state.presentation, officeView: state.officeView, postId: state.officeView === 'posts' ? state.postId : null, query: state.query, state: state.state, target, channel: agent ? state.channels[agent.id] ?? 'messages' : 'messages', panel: state.presentation === 'scene' ? state.panel : '', detail: !!target && state.detail, mail: state.mail, before: state.before };
}
/** Replace only owned keys; preserve embedding/supply/host hints and fragment at the caller. */
export function routeSearch(search: string, route: SpaceRoute): string {
  const params = new URLSearchParams(search); owned.forEach(key => params.delete(key));
  const put = (key: string, value: string | null | undefined) => { if (value) params.set(key, value); };
  put('office', route.workspaceId); if (route.presentation !== 'scene') put('view', route.presentation);
  if (route.officeView !== 'members') put('tab', route.officeView);
  put('post', route.postId); put('q', route.query); put('state', route.state); put('panel', route.panel);
  if (route.mail !== 'pending') put('mail', route.mail); put('before', route.before);
  if (route.target) {
    const target = route.target;
    params.set('office', route.workspaceId);
    put('scope', target.scope); put('generation', target.generation); put('agent', target.name);
    put('incarnation', target.incarnation); put('stopped_event', target.stopped_event); put('record', target.record);
    if (route.channel !== 'messages') put('channel', route.channel); if (route.detail) put('detail', 'agent');
  }
  const value = params.toString(); return value ? `?${value}` : '';
}
