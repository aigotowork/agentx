/** @aiPurpose Compose the preserved office scene over native supply or explicitly selected demo data. */
import { createPortal } from "react-dom";
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore, type ReactNode } from "react";
import { SpaceController, type ConversationOrigin, type Presentation } from "./offline/controller";
import { createOfflineSupply } from "./offline/fixture";
import { createStudioSupply } from "./offline/studio";
import { SpaceTheme, SpaceInteractions, SpaceStyleSwitch, type SpaceStyle } from "../uiux/systems/space/space.web";
import { usesStudioArt } from "../uiux/systems/space/components/presentation";
import { AgentCommands, NewAgentButton } from "./interactions/AgentCommands";
import { findConversation, GLOBAL_OFFICE, UNKNOWN_OFFICE, LEGACY_OFFICE } from "./offline/model";
import { officeMembers, visibleMembers } from "./offline/projection";
import type { HallLayoutMemory } from "./scene/population";
import { SceneSpace, type SceneMemory } from "./scene/SceneSpace";
import { Conversation } from "./conversation/Conversation";
import { Directory } from "./Directory";
import { OfficeLayout } from "./layouts/OfficeLayout";
import { SpaceLayout } from "./layouts/SpaceLayout";
import { ChatLayout } from "./layouts/ChatLayout";
import { OfficeNavigation, OfficeContext } from "./OfficeNavigation";
import { OfficeResources } from "./OfficeResources";
import type { SpaceHostBinding } from './live/host';
import { SpaceReadStatus } from './live/ReadStatus';
import { defaultRoute, routeSearch } from './navigation/route';
import { readAppearance, saveAppearance } from "./navigation/appearance";
import { LiveSpace } from "./live/controller";
import { emptySupply } from "./live/projection";
import { installNavigation } from "./navigation/browser";
import type { SpaceInspection } from "./inspection";
import { installContext } from "./live/context";
import { DirectoryContact, LiveStatus } from "./live/Actions";
import { WorkspaceWindows } from "./workspace/WorkspaceWindows";
import { ResourceWindow } from "./workspace/ResourceWindow";
import { visibleWindows, type ConversationWindows } from "./workspace/windows";
import { WorkspaceSoundToggle, useWorkspaceFeedback } from "./workspace/feedback";
import "./workspace/integration.css";

/** @aiPurpose Present actual offices, named Agent instances, posts and separate record channels.
 * @aiUsage The selected style changes presentation independently of the trusted SDK host. Without a host, supply=studio selects a synthetic office.
 */
export function App({ inspection, host, previewCaption }: { inspection?: SpaceInspection; host?: SpaceHostBinding; previewCaption?: string }) {
  const [session, setSession] = useState(0);
  const allowSample = !host;
  const [appearance, setAppearance] = useState(() => readAppearance(allowSample));
  const { style, legacyStudio } = appearance;
  const studio = !!legacyStudio || usesStudioArt(style);
  useEffect(() => {
    const sync = () => setAppearance(readAppearance(allowSample));
    window.addEventListener("popstate", sync);
    return () => window.removeEventListener("popstate", sync);
  }, [allowSample]);
  function chooseStyle(value: SpaceStyle) {
    if (value === style) return;
    saveAppearance(value);
    setAppearance({ style: value });
    window.dispatchEvent(new Event("agentx:location-changed"));
  }
  function reconnect(presentation: Presentation) {
    const search = routeSearch(location.search, { ...defaultRoute(), presentation });
    history.replaceState(null, '', `${location.pathname}${search}${location.hash}`);
    window.dispatchEvent(new Event('agentx:location-changed'));
    setSession(value => value + 1);
  }
  const styleControls = <SpaceStyleSwitch value={style} onChange={chooseStyle} />;
  const readerStyleControls = <SpaceStyleSwitch value={style} onChange={chooseStyle} compact />;
  return <SpaceTheme style={style}><SpaceInteractions><SpaceApp key={session} studio={studio} inspection={inspection} host={host} previewCaption={previewCaption} reconnect={reconnect} styleControls={styleControls} readerStyleControls={readerStyleControls} /></SpaceInteractions></SpaceTheme>;
}
function SpaceApp({ studio, inspection, host, previewCaption, reconnect, styleControls, readerStyleControls }: { studio: boolean; inspection?: SpaceInspection; host?: SpaceHostBinding; previewCaption?: string; reconnect(view: Presentation): void; styleControls: ReactNode; readerStyleControls: ReactNode }) {
  const [entry] = useState(() => new URLSearchParams(location.search));
  const sample = !host && entry.get("supply");
  const windowed = entry.get("workspace") !== "classic";
  const studioSample = sample === "studio";
  const [controller] = useState(() => new SpaceController(studioSample ? createStudioSupply() : sample === "demo" ? createOfflineSupply() : emptySupply(), windowed));
  const [live] = useState(() => controller.supply.mode === "live" ? new LiveSpace(controller.replaceSupply, host) : undefined);
  const [contextIssue, setContextIssue] = useState('');
  const [filtersOpen, setFiltersOpen] = useState(false);
  const contextId = useId();
  const contentRef = useRef<HTMLDivElement>(null);
  const contentWidth = useRef(0);
  const [width, setWidth] = useState(0);
  const feedback = useWorkspaceFeedback();
  const [windowCue, setWindowCue] = useState<{ id: string; event: "open" | "minimize" | "restore"; windows: ConversationWindows }>();
  useEffect(() => {
    if (!windowCue) return;
    const timer = window.setTimeout(() => setWindowCue(undefined), 240);
    return () => window.clearTimeout(timer);
  }, [windowCue]);
  useLayoutEffect(() => {
    const node = contentRef.current;
    if (!windowed || !node) return;
    const measure = (value: number) => {
      contentWidth.current = value;
      setWidth(value);
    };
    measure(node.clientWidth);
    const observer = new ResizeObserver(([entry]) => measure(entry.contentRect.width));
    observer.observe(node);
    return () => observer.disconnect();
  }, [windowed]);
  useEffect(() => {
    const uninstall = installNavigation(controller, setContextIssue, live ? (target, signal, office) => live.openReference({ name: target.name, incarnation: target.incarnation, stopped_event: target.stopped_event }, target, signal, office) : undefined);
    function sync() {
      if (!live) return;
      const state = controller.getSnapshot();
      live.chooseOffice(state.workspaceId);
      const active = controller.getSnapshot();
      if (windowed) live.selectReaders(active.reading ? visibleWindows(active.windows, paneCapacity(contentWidth.current, active.presentation)) : [], active.reading ? active.selectedId : null);
      else live.select(active.reading ? active.selectedId : null);
    }
    const unsubscribe = controller.subscribe(sync);
    sync();
    const stop = live?.start();
    return () => { uninstall(); unsubscribe(); stop?.(); };
  }, [controller, live, windowed]);
  useEffect(() => installContext(controller, live, setContextIssue), [controller, live]);
  const state = useSyncExternalStore(controller.subscribe, controller.getSnapshot), supply = controller.supply;
  const capacity = paneCapacity(width, state.presentation);
  useEffect(() => {
    if (!windowed || !live) return;
    const current = controller.getSnapshot();
    live.selectReaders(current.reading ? visibleWindows(current.windows, capacity) : [], current.reading ? current.selectedId : null);
  }, [capacity, controller, live, windowed]);
  useEffect(() => { live?.mailbox(state.mail === 'pending', state.before); }, [live, state.mail, state.before, supply]);
  const human = supply.human ? { ...supply.human, history: live ? (pending: boolean) => controller.mailbox(pending ? 'pending' : 'all', '') : supply.human.history, page: live ? (older: boolean) => controller.mailbox(state.mail, older ? supply.human?.nextBefore ?? '' : '') : supply.human.page } : undefined;
  const memories = useRef(new Map<string, { current: SceneMemory | null }>());
  if (!memories.current.has(state.workspaceId)) memories.current.set(state.workspaceId, { current: null });
  const memory = memories.current.get(state.workspaceId)!;
  const names = useMemo(() => {
    const entries = supply.workspaces.filter(office => !state.workspaceId || office.id === state.workspaceId).map(office => [office.id, studio ? office.title : `${office.title} · ${office.id}`] as const);
    const map = new Map<string, string>(entries);
    if (!studioSample) {
      if (!state.workspaceId || state.workspaceId === GLOBAL_OFFICE) map.set(GLOBAL_OFFICE, "全局成员");
      if (!state.workspaceId || state.workspaceId === UNKNOWN_OFFICE) map.set(UNKNOWN_OFFICE, "归属未知");
      if (!state.workspaceId || state.workspaceId === LEGACY_OFFICE) map.set(LEGACY_OFFICE, "旧记录 · 未记录归属");
    }
    return map;
  }, [supply, state.workspaceId, studio, studioSample]);
  const layoutMemory = useRef<HallLayoutMemory>(new Map());
  const scoped = useMemo(() => officeMembers(supply, state.workspaceId), [supply, state.workspaceId]);
  const filtered = visibleMembers(supply, state), visibleKeys = new Set(filtered.map(agent => agent.id));
  const selected = findConversation(supply, state.selectedId);
  const channel = selected ? state.channels[selected.id] ?? "messages" : "messages";
  const scrollPositions = useRef(new Map<string, number>());
  const roster = supply.roster?.office === state.workspaceId ? supply.roster : undefined;
  const isPinnedOffice = !!host && (state.workspaceId.startsWith("/") || state.workspaceId === GLOBAL_OFFICE);
  const awaitStudioRoster = studio && isPinnedOffice && (roster?.phase !== "ready" || !roster.exhausted && !scoped.some(agent => agent.ref.record === "current"));
  function cue(id: string, event: "open" | "minimize" | "restore" | "close") {
    feedback.play(event);
    setWindowCue(event === "close" ? undefined : { id, event, windows: controller.getSnapshot().windows });
  }
  function cueOpen(id: string, previous: ConversationWindows, visible: readonly string[]) {
    if (!windowed || visible.includes(id) || previous === controller.getSnapshot().windows) return;
    cue(id, previous.panes.some(pane => pane.id === id) ? "restore" : "open");
  }
  const shown = windowed && state.reading ? visibleWindows(state.windows, capacity) : [];
  const open = (id: string, origin: ConversationOrigin) => {
    const previous = controller.getSnapshot().windows;
    const opened = live?.connection !== 'reset' && controller.openConversation(id, origin);
    if (opened) cueOpen(id, previous, shown);
    return opened;
  };
  const openRelated = (id: string, sourceId?: string) => {
    const previous = controller.getSnapshot().windows;
    controller.openExact(id, sourceId);
    cueOpen(id, previous, shown);
  };
  const focusWorkspace = () => document.querySelector<HTMLSelectElement>('[aria-label="工作区范围"]')?.focus({ preventScroll: true });
  const focusSearch = () => {
    setFiltersOpen(true);
    requestAnimationFrame(() => document.getElementById(contextId)?.querySelector<HTMLInputElement>('input[type="search"]')?.focus({ preventScroll: true }));
  };
  function closePane(id: string) {
    controller.closeWindow(id);
    cue(id, "close");
    if (controller.getSnapshot().windows.panes.length) return;
    requestAnimationFrame(() => {
      const candidates = contentRef.current?.querySelectorAll<HTMLButtonElement>("[data-hall-object], [data-agent-key]");
      const opener = [...candidates ?? []].find(button => (button.dataset.hallObject === id || button.dataset.agentKey === id) && button.getClientRects().length > 0);
      if (opener) opener.focus({ preventScroll: true });
      else focusWorkspace();
    });
  }
  const directory = <Directory studio={studio} agents={filtered} selectedId={state.selectedId} drafts={state.drafts} onSelect={id => open(id, state.presentation === "scene" ? "directory" : "list")} onReset={controller.clearFilters} scrollPositions={scrollPositions.current} scrollKey={JSON.stringify(['roster', state.workspaceId, state.presentation, state.officeView])} showEmpty={!roster || scoped.length > 0 && !filtered.length && !!(state.query || state.state)} />;
  const backLabel = studio ? (state.presentation === "scene" ? "返回办公室" : "返回名册") : undefined;
  const reader = selected ? <Conversation studio={studio} backLabel={backLabel} presentationControl={studio ? readerStyleControls : undefined} controller={controller} key={selected.id} agent={selected} records={supply.records(selected.id, channel)} channel={channel} onChannel={value => controller.channel(selected.id, value)} draft={state.drafts[selected.id] ?? ""} onDraft={text => controller.writeDraft(selected.id, text)} onBack={controller.close} live={live} scrollPositions={scrollPositions.current} /> : <ReaderEmpty />;
  const isResources = state.officeView === "posts" || state.officeView === "skills";
  const windowAgents = windowed ? state.windows.panes.flatMap(pane => {
    const agent = findConversation(supply, pane.id);
    return agent ? [{ pane, agent }] : [];
  }) : [];
  const panes = windowAgents.map(({ pane, agent }) => {
    const recordChannel = state.channels[pane.id] ?? "messages";
    const visible = shown.includes(pane.id);
    const reader = visible ? (controls: ReactNode) => <Conversation workspace workspaceControls={controls} studio={studio} key={pane.id} agent={agent} records={supply.records(pane.id, recordChannel)} channel={recordChannel} onChannel={value => controller.channel(pane.id, value)} draft={state.drafts[pane.id] ?? ""} onDraft={text => controller.writeDraft(pane.id, text)} onBack={() => closePane(pane.id)} onOpenRelated={id => openRelated(id, pane.id)} live={live} controller={controller} scrollPositions={scrollPositions.current} /> : undefined;
    const duplicate = windowAgents.some(other => other.pane.id !== pane.id && other.agent.title === agent.title);
    const title = duplicate ? `${agent.title} · ${agent.ref.record === "stopped" ? "历史" : "当前"} ${agent.ref.stopped_event ?? agent.ref.incarnation ?? ""}` : agent.title;
    return { ...pane, title, visible, reader };
  });
  const windows = windowed ? <WorkspaceWindows panes={panes} activeId={state.windows.activeId} focused={state.windows.focused} overlay={state.presentation === "scene"}
    motion={windowCue?.windows === state.windows ? windowCue : undefined}
    onActivate={id => { const previous = controller.getSnapshot().windows; controller.activateWindow(id); cueOpen(id, previous, shown); }}
    onFocus={controller.focusWindow}
    onMinimize={id => { controller.minimizeWindow(id); cue(id, "minimize"); }}
    onClose={closePane} /> : undefined;
  const resources = isResources && <OfficeResources controller={controller} live={live} />;
  const content = !windowed && resources ? resources : state.presentation === "scene"
    ? <SceneSpace studio={studio} studioSample={studioSample} awaitStudioRoster={awaitStudioRoster} key={state.workspaceId} memory={memory} layoutMemory={layoutMemory.current} population={supply.population} identityScope={`${supply.scope.id}:${supply.scope.generation}`} messages={supply.activity} human={human} agents={scoped} names={names} visibleKeys={visibleKeys} selectedId={state.selectedId} reading={state.reading} reader={reader} workspace={windows} directory={directory} scope={state.workspaceId} showEmpty={!roster || !!(state.query || state.state) || studio && roster.phase === "ready" && roster.exhausted} onSelect={id => open(id, "actor")} onClose={controller.close} onCabinet={() => controller.historyDirectory()} onReset={controller.clearFilters} onHumanNavigate={openRelated} panel={state.panel} onPanel={controller.panel} />
    : <ChatLayout directory={directory} reader={state.reading ? reader : <ReaderEmpty />} workspace={windows} hasWindows={panes.length > 0} reading={state.reading} onReturnFallback={focusWorkspace} />;
  const contextStatus = contextIssue ? <div className="context-error" role="alert">{contextIssue}{live?.more && !roster && <button type="button" onClick={live.loadMore}>加载更多成员</button>}{host && contextIssue.startsWith('精确目标不可用') && <button type="button" onClick={() => window.dispatchEvent(new PopStateEvent('popstate'))}>重试精确会话</button>}</div> : null;
  const sound = windowed && <WorkspaceSoundToggle enabled={feedback.enabled} onToggle={feedback.toggle} />;
  const compactReading = windowed && state.reading;
  const filters = compactReading && <button className="workspace-filter-trigger" type="button" aria-expanded={filtersOpen} aria-controls={contextId} onClick={() => setFiltersOpen(value => !value)}>筛选</button>;
  const header = <SpaceHeader studio={studioSample} previewCaption={studio && host ? previewCaption : undefined} presentation={state.presentation} onPresentation={controller.presentation} live={live} inspection={inspection} onReconnect={() => reconnect(state.presentation)} styleControls={<>{styleControls}{sound}</>} windowed={windowed} filters={filters} />;
  const context = <>{contextStatus}<OfficeContext studio={studioSample} controller={controller} matchedCount={filtered.length} actions={live && <DirectoryContact key={state.workspaceId} live={live} office={state.workspaceId} />} />{live && <SpaceReadStatus live={live} roster={roster} rows={scoped} />}</>;
  const studioReader = !windowed && studio && !isResources && state.reading && selected ? state.presentation : undefined;
  const resourceWindow = windowed && resources && <ResourceWindow title={state.officeView === "skills" ? "办公室技能" : "办公室岗位"} compact={state.officeView === "skills"} onClose={controller.closeResources}>{resources}</ResourceWindow>;
  return <AgentCommands live={live} controller={controller} onSearch={windowed ? focusSearch : undefined}><OfficeLayout studio={studio} studioSample={studioSample} studioReader={studioReader} windowed={windowed} compactReading={compactReading} filtersOpen={filtersOpen} header={header} context={context} contextId={contextId}><SpaceLayout contentRef={contentRef} navigation={<OfficeNavigation studio={studioSample} controller={controller} />}>{content}{resourceWindow}</SpaceLayout></OfficeLayout></AgentCommands>;
}
function paneCapacity(width: number, presentation: Presentation): 1 | 2 {
  return width >= (presentation === "scene" ? 800 : 1000) ? 2 : 1;
}
function SpaceHeader({ studio, previewCaption, presentation, onPresentation, live, inspection, onReconnect, styleControls, windowed, filters }: { studio: boolean; previewCaption?: string; presentation: Presentation; onPresentation(value: Presentation): void; live?: LiveSpace; inspection?: SpaceInspection; onReconnect(): void; styleControls?: ReactNode; windowed?: boolean; filters?: ReactNode }) {
  const [inspecting, setInspecting] = useState(false);
  function toggleInspection() { const visible = !inspecting; inspection?.setVisible(visible); setInspecting(visible); }
  const inspectionDismiss = inspecting && createPortal(<button type="button" className="space-inspection-dismiss" aria-label="收起页面检查" onClick={toggleInspection}>收起检查 ×</button>, document.body);
  const inspectionButton = inspection && <button type="button" aria-label="检查页面" aria-pressed={inspecting} title="打开或收起页面检查工具" onClick={toggleInspection}>检查</button>;
  const displayOptions = windowed ? <details className="workspace-display-menu"><summary aria-label="显示与反馈" title="显示与反馈"><svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round"><path d="M4 6h16M4 12h16M4 18h16M8 4v4m8 2v4m-6 2v4" /></svg></summary><div>{styleControls}{inspectionButton}</div></details> : <>{styleControls}{inspectionButton}</>;
  return <>{inspectionDismiss}<div className="office-brand"><strong>{previewCaption || "Agent 空间"}</strong></div><nav className="presentation-switch" aria-label="呈现方式"><button type="button" aria-pressed={presentation === "scene"} onClick={() => onPresentation("scene")}>场景</button><button type="button" aria-pressed={presentation === "chat"} onClick={() => onPresentation("chat")}>聊天</button></nav>{filters}{displayOptions}{live ? <><NewAgentButton /><LiveStatus live={live} onReconnect={onReconnect} /></> : <details className="offline-info"><summary aria-label="离线样例说明">{studio ? "工作室样例 ⓘ" : "离线样例 ⓘ"}</summary><div><strong>合成样例</strong><p>成员、目录和消息均为样例。草稿不会发送。</p><a href="./NOTICE.md" target="_blank" rel="noreferrer">素材与许可</a></div></details>}</>;
}
function ReaderEmpty() {
  return <div className="reader-empty"><span className="reader-empty-glyph" aria-hidden="true">▤</span><strong>选择一位 Agent</strong></div>;
}
