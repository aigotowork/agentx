/** @aiPurpose Compact known-office navigation and local members/posts/Skills/history controls. */
import type { MouseEvent, ReactNode } from "react";
import type { SpaceController } from "./offline/controller";
import { findConversation, GLOBAL_OFFICE, hostGroup, UNKNOWN_OFFICE, LEGACY_OFFICE, projectDisplayName, type CloudOfficeDirectory, type OfficeView } from "./offline/model";
import { currentMembers, officeMembers, visibleMembers } from "./offline/projection";
import { HALL_STATE_LABEL } from "./scene/model";
import { cloudNavigation } from "./cloud/route";
import { defaultRoute } from "./navigation/route";
const views: { id: OfficeView; label: string }[] = [{ id: "members", label: "成员" }, { id: "posts", label: "岗位" }, { id: "skills", label: "技能" }, { id: "history", label: "历史" }];
function officeEntries(controller: SpaceController, studio?: boolean) {
  const other = studio ? [] : [{ id: GLOBAL_OFFICE, title: "全局成员" }, { id: UNKNOWN_OFFICE, title: "归属未知" }, { id: LEGACY_OFFICE, title: "旧记录 · 未记录归属" }];
  return [{ id: "", title: "全部办公室" }, ...controller.supply.workspaces, ...other].map(office => ({ ...office, count: currentMembers(officeMembers(controller.supply, office.id)).length }));
}

function cloudGroups(directory?: CloudOfficeDirectory) {
  const projects = directory?.projects ?? [];
  const hosts = new Map(directory?.hosts.map(host => [host.name, host.online]));
  for (const project of projects) if (!hosts.has(project.host)) hosts.set(project.host, project.online);
  return [...hosts].sort(([a], [b]) => a.localeCompare(b)).map(([host, online]) => {
    const offices = projects.filter(project => project.host === host).sort((a, b) => a.title.localeCompare(b.title) || a.id.localeCompare(b.id));
    return { host, online, offices: offices.map(office => ({ ...office, label: projectDisplayName(office.title, office.workspaceId, offices.filter(other => other.title === office.title).map(other => other.workspaceId)) })) };
  });
}

function CloudOfficeNavigation({ controller }: { controller: SpaceController }) {
  const { cloudDirectory, scope } = controller.supply;
  const { workspaceId, presentation } = controller.getSnapshot();
  const codec = cloudNavigation(scope.id, scope.generation);
  const current = new URLSearchParams(location.search);
  const preferences = new URLSearchParams();
  for (const key of ['access', 'style', 'workspace']) if (current.has(key)) preferences.set(key, current.get(key)!);
  function href(office: string) { return location.pathname + codec.search(preferences.toString(), { ...defaultRoute(), presentation, workspaceId: office }); }
  function choose(event: MouseEvent<HTMLAnchorElement>, office: string) {
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    controller.office(office);
  }
  const groups = cloudGroups(cloudDirectory);
  return <nav className="office-tree cloud-office-tree" aria-label="目录办公室">
    <header><strong>Namespace</strong><code title={scope.id}>{scope.id}</code></header>
    <a className="cloud-office-link" href={href('')} aria-current={!workspaceId ? 'page' : undefined} onClick={event => choose(event, '')}>总览</a>
    {groups.map(({ host, online, offices }) => {
      const group = hostGroup(host);
      return <section className="cloud-office-group" key={host} aria-label={host}>
        <a className="cloud-office-link cloud-host-link" href={href(group)} aria-current={workspaceId === group ? 'page' : undefined} onClick={event => choose(event, group)}><strong>{host}</strong>{!online && <small>离线</small>}</a>
        {offices.map(office => <a className="cloud-office-link cloud-project-link" key={office.id} title={office.workspaceId} href={href(office.id)}
          aria-current={workspaceId === office.id ? 'page' : undefined} onClick={event => choose(event, office.id)}><span>{office.label}</span>{!office.online && online && <small>离线</small>}</a>)}
        {!offices.length && cloudDirectory?.phase === 'ready' && <small className="cloud-no-projects">没有项目</small>}
      </section>;
    })}
  </nav>;
}

function CloudOfficeOptions({ controller }: { controller: SpaceController }) {
  const selected = controller.getSnapshot().workspaceId;
  const groups = cloudGroups(controller.supply.cloudDirectory);
  const found = groups.some(group => selected === hostGroup(group.host) || group.offices.some(office => selected === office.id));
  return <>
    <option value="">Namespace 总览</option>
    {selected && !found && <option value={selected}>当前范围 · 目录未提供</option>}
    {groups.map(({ host, online, offices }) => <optgroup key={host} label={online ? host : `${host} · 离线`}>
      <option value={hostGroup(host)}>{host} · 全部成员</option>
      {offices.map(office => <option key={office.id} value={office.id}>{office.label} · {host}{!office.online ? ' · 离线' : ''}</option>)}
    </optgroup>)}
  </>;
}
/** @aiPurpose Show canonical office boundaries, known empty offices and separate global/unknown areas.
 * @aiUsage Desktop companion to the scope select; choosing an office changes only local context.
 */
export function OfficeNavigation({ controller, studio }: { controller: SpaceController; studio?: boolean }) {
  if (controller.supply.scope.cloud) return <CloudOfficeNavigation controller={controller} />;
  const selected = controller.getSnapshot().workspaceId;
  const entries = officeEntries(controller, studio);
  return <nav className="office-tree" aria-label="目录办公室"><header><strong>办公室</strong><small>{controller.supply.mode === "offline" ? "已知集 · 样例" : "已知集 · 原生"}</small></header>{entries.map(entry => <button type="button" key={entry.id} aria-current={selected === entry.id ? "page" : undefined} onClick={() => controller.office(entry.id)}><span><strong>{entry.title}</strong>{entry.id.startsWith("/") && <small title={controller.supply.workspaces.find(office => office.id === entry.id)?.sources?.join(" · ")}>{entry.id}</small>}</span><output>{entry.count}</output></button>)}<details className="scope-facts"><summary>数据范围 ⓘ</summary><code>{controller.supply.scope.id}</code><small>{controller.supply.scope.generation}</small><small>{controller.supply.scope.coverage}</small><small>成员数不包含停止历史；归属未知独立列出。</small></details></nav>;
}
/** @aiPurpose Keep scope selection, search and local navigation reachable in the working viewport.
 * @aiUsage Compose above scene/chat/resources; all controls are page-local and preserve exact drafts.
 */
export function OfficeContext({ controller, matchedCount, actions, studio }: { controller: SpaceController; matchedCount: number; actions?: ReactNode; studio?: boolean }) {
  const cloud = controller.supply.scope.cloud;
  const state = controller.getSnapshot();
  const members = officeMembers(controller.supply, state.workspaceId);
  const current = currentMembers(members), stopped = members.length - current.length;
  const posts = controller.supply.posts.filter(post => !state.workspaceId || post.office === state.workspaceId);
  const selected = findConversation(controller.supply, state.selectedId);
  const canFindSelected = !!controller.supply.roster && selected &&
    visibleMembers(controller.supply, state).some(member => member.id === selected.id) &&
    (state.presentation === "chat" || selected.ref.record === "current");
  const isPeople = state.officeView === "members" || state.officeView === "history";
  const count = state.officeView === "history" ? stopped : current.length;
  const project = controller.supply.cloudDirectory?.projects.find(office => office.id === state.workspaceId);
  const officeOptions = cloud ? <CloudOfficeOptions controller={controller} />
    : officeEntries(controller, studio).map(entry => <option value={entry.id} key={entry.id}>{entry.title} · {entry.count}</option>);
  const availableViews = cloud ? views.filter(view => view.id === "members" || view.id === "posts" && !!project) : views;
  const counts: Partial<Record<OfficeView, number>> = { members: current.length, history: stopped, posts: project?.roles.length ?? posts.length };
  return <><div className="overview-fields"><label className="scope-field"><span className="visually-hidden">目录办公室</span><select aria-label="工作区范围" value={state.workspaceId} onChange={event => controller.office(event.target.value)}>{officeOptions}</select></label><div className="search-field"><input type="search" aria-label="搜索 Agent 或工作区" placeholder={cloud ? "搜索成员…" : "搜索成员、岗位…"} value={state.query} onChange={event => controller.filter({ query: event.target.value })} />{state.query && <button type="button" aria-label="清除搜索" onClick={() => controller.filter({ query: "" })}>×</button>}</div>{isPeople && <output className="match-count" aria-label="匹配会话数">{matchedCount}/{count}</output>}<details className="context-provenance"><summary aria-label="范围与覆盖说明">ⓘ</summary><div><code>{state.workspaceId || controller.supply.scope.id}</code><span>{controller.supply.scope.coverage}</span></div></details></div><div className="office-local-controls"><nav className="local-views" aria-label="办公室内容">{availableViews.map(view => <button type="button" key={view.id} aria-pressed={state.officeView === view.id} onClick={() => controller.view(view.id)}>{view.label}{counts[view.id] === undefined ? "" : ` ${counts[view.id]}`}</button>)}</nav>{isPeople && !cloud && <select className="state-select" aria-label="状态筛选" value={state.state} onChange={event => controller.filter({ state: event.target.value as typeof state.state })}><option value="">全部状态</option>{Object.entries(HALL_STATE_LABEL).map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select>}{selected && !state.reading && !canFindSelected && <button type="button" className="resume-reader" aria-label={`继续阅读 ${selected.title} ${selected.ref.record === "stopped" ? "停止历史" : "当前实例"}`} onClick={() => controller.openConversation(selected.id, "resume")}>↩ {selected.title}</button>}{actions}</div></>;
}
