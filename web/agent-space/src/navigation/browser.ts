/** Page-lifetime URL/state synchronization; supply retries never create history. */
import { conversationSubjects, GLOBAL_OFFICE, matchesOffice, membershipKey } from '../offline/model';
import type { SpaceController } from '../offline/controller';
import { defaultRoute, parseRoute, routeFromState, routeSearch, type SpaceRoute, type RouteTarget, type NavigationCodec } from './route';
/** Install once per page; user navigation cancels pending targets, drafts/supply do not. */
export function installNavigation(controller: SpaceController, report: (issue: string) => void, resolveTarget?: (target: RouteTarget, signal: AbortSignal, office: string) => Promise<unknown>, codec: NavigationCodec = { parse: parseRoute, search: routeSearch }): () => void {
  let applying = false, revision = controller.navigationRevision, pending: SpaceRoute | null = null;
  let previous = defaultRoute(), syntaxIssue = '';
  let supplied = controller.supply;
  type RouteRead = { controller: AbortController; phase: 'loading' | 'ready' | 'error'; error?: string };
  let request: RouteRead | undefined;
  function cancel() { request?.controller.abort(); request = undefined; }
  function restore() {
    if (!pending || applying) return;
    const route = pending, supply = controller.supply;
    let issue = syntaxIssue, selectedId: string | null = null;
    let workspaceId = route.workspaceId;
    const domainOffice = !!supply.scope.listener && (workspaceId === GLOBAL_OFFICE || workspaceId.startsWith('/'));
    const known = !domainOffice || workspaceId === GLOBAL_OFFICE || supply.workspaces.some(office => office.id === workspaceId);
    if (workspaceId && !supply.scope.cloud && !['$global', '$legacy', '$unknown'].includes(workspaceId) && !supply.workspaces.some(office => office.id === workspaceId)) issue ||= '办公室尚未加载或不存在';
    if (!route.target && workspaceId.startsWith('project:') && supply.scope.cloud && !supply.workspaces.some(office => office.id === workspaceId)) {
      issue ||= supply.cloudDirectory?.phase === 'ready' ? '此项目办公室不存在' : '正在等待项目目录';
    }
    if (route.target) {
      const target = route.target;
      // Synthetic demo records can explicitly retain an older generation; native targets cannot.
      if (!supply.scope.id) issue ||= '正在等待精确会话供给';
      else if (target.scope !== supply.scope.id || supply.mode !== 'offline' && target.generation !== supply.scope.generation) issue ||= supply.scope.cloud ? '目标 Namespace 代次已失效；不会转向同名实例' : '目标 scope / generation 已失效；不会转向同名实例';
      else {
        const candidates = domainOffice && supply.roster?.office !== workspaceId ? supply.subjects ?? [] : conversationSubjects(supply);
        const matches = candidates.filter(agent => agent.ref.scope === target.scope && agent.ref.generation === target.generation && agent.ref.host === target.host && agent.name === target.name && agent.ref.incarnation === target.incarnation && (agent.ref.stopped_event ?? null) === target.stopped_event && agent.ref.record === target.record);
        if ((supply.scope.listener || supply.scope.cloud) && request?.phase === 'loading') issue ||= '正在读取精确会话';
        else if (request?.phase === 'error') issue ||= `精确目标不可用：@${target.name} · ${target.record === 'stopped' ? '停止历史' : '当前实例'}（${target.incarnation ?? target.stopped_event}） · ${request.error}；不会转向同名实例`;
        else if (matches.length !== 1) issue ||= '精确目标未加载、已失效或不唯一；不会转向同名实例';
        else if (!matchesOffice(matches[0], workspaceId)) issue ||= '办公室与精确 Agent 归属不匹配';
        else if (known) { selectedId = matches[0].id; if (route.inferOffice) workspaceId = membershipKey(matches[0]); }
      }
    }
    if (route.postId && !supply.posts.some(post => post.id === route.postId && (!workspaceId || post.office === workspaceId))) issue ||= '岗位尚未加载或不存在';
    applying = true;
    const state = controller.getSnapshot();
    controller.restoreNavigation({ workspaceId, presentation: route.presentation, officeView: route.officeView, postId: route.postId, query: route.query, state: route.state, panel: route.panel, detail: route.detail, mail: route.mail, before: route.before, selectedId, reading: !!selectedId, channels: selectedId ? { ...state.channels, [selectedId]: route.channel } : state.channels });
    applying = false; previous = route; report(issue);
    // Keep unresolved resource intent until supply arrives or a deliberate navigation supersedes it.
    if (!issue) {
      pending = null;
      const search = codec.normalize?.(location.search, route);
      if (search !== undefined && search !== location.search) {
        history.replaceState(history.state, '', `${location.pathname}${search}${location.hash}`);
      }
    }
    const target = route.target;
    if (target && known && !selectedId && resolveTarget && target.scope === supply.scope.id && target.generation === supply.scope.generation) {
      if (!request) {
        const controller = new AbortController();
        const read: RouteRead = { controller, phase: 'loading' };
        request = read;
        // Finish the route's synchronous office transition before addressing its exact read.
        void Promise.resolve().then(() => {
          if (controller.signal.aborted) return;
          return resolveTarget(target, controller.signal, route.workspaceId);
        }).then(() => {
          if (request !== read) return;
          read.phase = 'ready'; restore();
        }, error => {
          if (request !== read) return;
          read.phase = 'error'; read.error = error instanceof Error ? error.message : String(error);
          restore();
        });
      }
    }
  }
  function read() {
    revision = controller.noteBrowserNavigation();
    const parsed = codec.parse(location.search); syntaxIssue = parsed.issue; pending = parsed.route; cancel(); restore();
  }
  read();
  const unsubscribe = controller.subscribe(() => {
    if (applying) return;
    if (controller.navigationRevision === revision) {
      if (controller.supply !== supplied) {
        const next = controller.supply;
        if (supplied.scope.id !== next.scope.id || supplied.scope.generation !== next.scope.generation || supplied.scope.listener !== next.scope.listener) { cancel(); pending ??= previous; }
        supplied = next;
      }
      if (pending && !syntaxIssue) restore(); return;
    }
    revision = controller.navigationRevision; pending = null; cancel(); syntaxIssue = ''; report('');
    const route = routeFromState(controller.getSnapshot(), controller.supply);
    const search = codec.search(location.search, route);
    if (search !== location.search) {
      const onlyFilters = JSON.stringify({ ...previous, query: '', state: '' }) === JSON.stringify({ ...route, query: '', state: '' });
      window.history[onlyFilters ? 'replaceState' : 'pushState'](null, '', `${location.pathname}${search}${location.hash}`);
      window.dispatchEvent(new Event('agentx:location-changed'));
    }
    previous = route;
  });
  window.addEventListener('popstate', read);
  return () => { cancel(); unsubscribe(); window.removeEventListener('popstate', read); };
}
