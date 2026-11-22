import { forgetLogin, rememberLogin, useRememberedLogin, type TokenRetention } from '../../../management/deployment/remember';
import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from 'react';
import { DeploymentControlDialog, DeploymentReceiptLookup, type ControlTarget } from '../../../management/deployment/control';
import { useCursor, useRead } from '../../../management/deployment/model';
import { SpaceInteractions, SpaceStyleSwitch, SpaceTheme, type SpaceStyle } from '../../uiux/systems/space/space.web';
import { usesStudioArt } from '../../uiux/systems/space/components/presentation';
import { OfficeLayout } from '../layouts/OfficeLayout';
import { readAppearance, saveAppearance } from '../navigation/appearance';
import { SceneSpace, type SceneMemory } from '../scene/SceneSpace';
import type { HallLayoutMemory } from '../scene/population';
import { CloudMessages } from './CloudMessages';
import { agentsPage, cloudPageSize, cloudProblem, cloudTime, connectCloud, hostsPage, projectCloudAgent, resolveAdminAgent, tokenPattern, type Session } from './model';
import { accessHint, exactKey, readCloudRoute, validScope, writeCloudRoute, type Access, type CloudAgent, type CloudRoute } from './route';
import './cloud.css';

type Rail = 'agents' | 'hosts' | 'receipts';
type AdminSession = Session & { access: 'admin' };
const permissions: Record<Access, string> = { admin: '清单、主机目录、启动、精确停止和回执', scope: '清单与只读云端消息' };

/**
 * @aiPurpose Mount the existing Space presentation for a cloud scope, with explicit audience admission.
 * @aiUsage The Space entry calls this only for its scoped cloud route; do not mount the native Space connection beside it.
 */
export function CloudSpace({ scope }: { scope: string }) {
  const [session, setSession] = useState<Session>();
  const [appearance, setAppearance] = useState(() => readAppearance(false));
  const studio = usesStudioArt(appearance.style) || !!appearance.legacyStudio;
  function chooseStyle(style: SpaceStyle) {
    saveAppearance(style);
    setAppearance({ style });
  }
  useEffect(() => {
    const sync = () => setAppearance(readAppearance(false));
    function restore(event: PageTransitionEvent) { if (event.persisted) setSession(undefined); }
    window.addEventListener('popstate', sync);
    window.addEventListener('pageshow', restore);
    return () => { window.removeEventListener('popstate', sync); window.removeEventListener('pageshow', restore); };
  }, []);
  const styles = <SpaceStyleSwitch value={appearance.style} onChange={chooseStyle} compact />;
  const header = <><div className="cloud-brand"><h1>云端 Space</h1><code>{scope}</code></div>
    <div className="cloud-header-actions">{styles}<a href="/deployment">部署管理</a>
      {session && <button type="button" title={permissions[session.access]} onClick={() => { forgetLogin(session.access === 'admin' ? 'admin' : scopeAudience(scope)); setSession(undefined); }}>断开</button>}
    </div></>;
  return <SpaceTheme style={appearance.style}><SpaceInteractions><OfficeLayout studio={studio} header={header}
    context={session ? <div className="cloud-session"><strong>{session.access === 'admin' ? '管理员访问' : 'Scope 访问'}</strong><span title={permissions[session.access]}>{permissions[session.access]}</span><code title="当前 Scope 代次">{session.generation}</code></div> : null}>
    {!validScope(scope) ? <p className="cloud-empty" role="alert">Scope 标识无效。</p>
      : session ? <CloudWorkspace session={session} studio={studio} />
      : <Connect scope={scope} onConnect={setSession} />}
  </OfficeLayout></SpaceInteractions></SpaceTheme>;
}

const scopeAudience = (scope: string) => 'scope.' + scope;

function Connect({ scope, onConnect }: { scope: string; onConnect(session: Session): void }) {
  const [access, setAccess] = useState(() => accessHint(location.search));
  const [token, setToken] = useState('');
  const [retention, setRetention] = useState<TokenRetention>(90);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const audience = access === 'admin' ? 'admin' : scopeAudience(scope);
  const validate = useCallback((token: string, signal: AbortSignal) => connectCloud(scope, access, token, signal), [scope, access]);
  const restoring = useRememberedLogin(audience, validate, onConnect);
  const controller = useRef<AbortController>(null);
  useEffect(() => () => controller.current?.abort(), []);
  async function connect(event: FormEvent) {
    event.preventDefault();
    if (controller.current) return;
    const bearer = token.trim();
    if (!tokenPattern.test(bearer)) { setError('令牌应包含 32–256 个字母、数字、下划线或连字符。'); return; }
    const request = new AbortController();
    controller.current = request;
    setBusy(true);
    setError('');
    try {
      const session = await connectCloud(scope, access, bearer, request.signal);
      if (!request.signal.aborted) { rememberLogin(audience, bearer, retention);
        const url = new URL(location.href);
        url.searchParams.set('access', access);
        history.replaceState(history.state, '', url); onConnect(session); }
    } catch (failure) {
      if (!request.signal.aborted) setError(cloudProblem(failure, '无法连接云端 Space，请重试。'));
    } finally {
      controller.current = null;
      if (!request.signal.aborted) setBusy(false);
    }
  }
  return <div className="cloud-entry"><form className="cloud-connect" onSubmit={connect} autoComplete="off">
    <div className="cloud-connect-mark" aria-hidden="true"><svg viewBox="0 0 32 32" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="m3 14 13-8 13 8v12H3zM12 26v-9h8v9M1 15 16 5l15 10" /></svg></div>
    <h2>连接此 Scope</h2>
    <label>访问方式<select value={access} disabled={busy || restoring} onChange={event => { const next = event.target.value as Access; setAccess(next); setToken(''); setError(''); }}>
      <option value="scope">Scope 令牌</option><option value="admin">管理员令牌</option>
    </select></label>
    <p className="cloud-note">{permissions[access]}</p>
    <label>访问令牌<input value={token} onChange={event => setToken(event.target.value)} type="password" required maxLength={256} autoComplete="off" spellCheck={false} aria-describedby={error ? 'cloud-connect-error' : undefined} aria-invalid={!!error} /></label>
    <label>记住令牌<select value={retention} onChange={event => setRetention(Number(event.target.value) as TokenRetention)} disabled={busy || restoring}>
      <option value="0">仅本次</option><option value="30">30 天</option><option value="90">90 天</option>
    </select></label>
    {error && <p id="cloud-connect-error" role="alert">{error}</p>}
    <button className="cloud-primary" type="submit" disabled={busy || restoring}>{busy ? '正在连接…' : '连接'}</button>
  </form></div>;
}

function CloudWorkspace({ session, studio }: { session: Session; studio: boolean }) {
  const [route, setRoute] = useState(() => readCloudRoute(location.search));
  const [rail, setRail] = useState<Rail>('agents');
  const [hostFilter, setHostFilter] = useState<string>();
  const [target, setTarget] = useState<ControlTarget | null>(null);
  const hostCursor = useCursor();
  const agentCursor = useCursor();
  const loadHosts = useCallback((signal: AbortSignal) => hostsPage(session, { after: hostCursor.after, limit: cloudPageSize }, signal), [session, hostCursor.after]);
  const loadAgents = useCallback((signal: AbortSignal) => agentsPage(session, { after: agentCursor.after, host: hostFilter, limit: cloudPageSize }, signal), [session, agentCursor.after, hostFilter]);
  const hosts = useRead(loadHosts);
  const agents = useRead(loadAgents);
  const [freshness, setFreshness] = useState(0);
  const room = `/cloud/${session.scope}`;
  const names = useMemo(() => new Map([[room, session.scope]]), [room, session.scope]);
  const agentPage = agents.read.kind === 'ready' ? agents.read.data : undefined;
  const rows = agentPage?.items;
  const complete = !!agentPage && !agentPage.next && !agentCursor.after && !hostFilter;
  const actors = useMemo(() => rows?.map(agent => projectCloudAgent(agent, session, room)) ?? [], [rows, session, room]);
  const visibleKeys = useMemo(() => new Set(actors.map(agent => agent.id)), [actors]);
  const memory = useRef<SceneMemory>(null);
  const layoutMemory = useMemo<HallLayoutMemory>(() => new Map(), [hostFilter, agentCursor.after]);
  const railRoot = useRef<HTMLElement>(null);
  const readerTitle = useRef<HTMLHeadingElement>(null);
  const selectionTrigger = useRef<HTMLElement | null>(null);
  const stale = !!route.selection?.generation && route.selection.generation !== session.generation;
  const selected = stale ? null : route.selection;
  const activeKey = selected && exactKey(selected);

  useEffect(() => {
    function sync() { setRoute(readCloudRoute(location.search)); }
    window.addEventListener('popstate', sync);
    return () => window.removeEventListener('popstate', sync);
  }, []);
  // Pushes only come from deliberate navigation. A generationless shared link gets pinned after admission.
  useEffect(() => {
    if (route.selection && !route.selection.generation) {
      const pinned = { ...route, selection: { ...route.selection, generation: session.generation } };
      writeCloudRoute(pinned, true);
      setRoute(pinned);
    }
  }, [route, session.generation]);

  function navigate(next: CloudRoute) { writeCloudRoute(next); setRoute(next); }
  function select(agent: CloudAgent) {
    selectionTrigger.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    navigate({ ...route, invalid: false, selection: { ...agent, generation: session.generation } });
    requestAnimationFrame(() => readerTitle.current?.focus());
  }
  function closeReader() {
    navigate({ ...route, invalid: false, selection: null });
    requestAnimationFrame(() => {
      const trigger = selectionTrigger.current;
      if (trigger?.isConnected && trigger.getClientRects().length) trigger.focus();
      else openRoster();
    });
  }
  function selectActor(key: string) {
    const agent = rows?.find(item => exactKey(item) === key);
    if (!agent) return false;
    select(agent);
    return true;
  }
  function filterHost(host?: string) {
    if (hostFilter !== host) {
      setHostFilter(host);
      agentCursor.reset();
    }
    setRail('agents');
  }
  function refresh() {
    hosts.retry();
    agents.retry();
    setFreshness(value => value + 1);
  }
  function launch(host: string) {
    setTarget({ kind: 'launch', scope: session.scope, generation: session.generation, host });
  }
  function stop(agent: CloudAgent) {
    setTarget({ kind: 'stop', scope: session.scope, generation: session.generation, ...agent });
  }
  function closeControl() {
    setTarget(null);
    refresh();
  }
  function openRoster() {
    setRail('agents');
    railRoot.current?.querySelector<HTMLButtonElement>('[data-agents-tab]')?.focus({ preventScroll: true });
  }
  const hasReader = !!selected || stale || route.invalid;
  const gridClass = `cloud-grid${route.view === 'list' ? ' cloud-list-view' : ''}${hasReader ? ' cloud-with-reader' : ''}`;
  const panel = <aside ref={railRoot} className="cloud-rail" aria-label="云端清单与回执">
    <nav className="cloud-rail-nav" aria-label="Scope 资源">
      <button type="button" data-agents-tab aria-pressed={rail === 'agents'} onClick={() => setRail('agents')}>Agents</button>
      <button type="button" aria-pressed={rail === 'hosts'} onClick={() => setRail('hosts')}>主机</button>
      {session.access === 'admin' && <button type="button" aria-pressed={rail === 'receipts'} onClick={() => setRail('receipts')}>回执</button>}
    </nav>
    {rail === 'agents' && <div className="cloud-rail-content">
      {hostFilter && <div className="cloud-filter"><code>{hostFilter}</code><button type="button" onClick={() => filterHost()}>全部主机</button></div>}
      <ReadPage value={agents.read} cursor={agentCursor} retry={agents.retry} empty="没有已注册的 Agent。" error="无法读取 Agent 清单。">
        {items => <ul className="cloud-inventory" aria-label="已注册的 Agents">{items.map(agent => <li key={exactKey(agent)}>
          <button type="button" className="cloud-agent-row" aria-pressed={activeKey === exactKey(agent)} onClick={() => select(agent)}>
            <strong>@{agent.name}</strong><code>{agent.host} · {agent.incarnation}</code>
          </button>
        </li>)}</ul>}
      </ReadPage>
    </div>}
    {rail === 'hosts' && <div className="cloud-rail-content">
      <ReadPage value={hosts.read} cursor={hostCursor} retry={hosts.retry} empty="没有已注册的主机。" error="无法读取主机清单。">
        {items => <ul className="cloud-inventory" aria-label="已注册的主机">{items.map(host => {
          const seen = cloudTime(host.last_seen);
          return <li key={host.name} className="cloud-host-row"><div className="cloud-host-facts"><strong>{host.name}</strong>
            <span data-online={host.online}>{host.online ? '心跳在线' : '心跳离线'}</span></div>
            {seen && <time dateTime={seen.toISOString()} title="最后心跳">{seen.toLocaleString()}</time>}
            <div className="cloud-row-actions"><button type="button" onClick={() => filterHost(host.name)}>查看 Agents</button>
              {session.access === 'admin' && <button type="button" aria-label={`查看 ${host.name} 的目录与启动选项`} onClick={() => launch(host.name)}>目录与启动</button>}
            </div>
          </li>;
        })}</ul>}
      </ReadPage>
    </div>}
    {rail === 'receipts' && session.access === 'admin' && <div className="cloud-rail-content cloud-control">
      <DeploymentReceiptLookup client={session.client} scope={session.scope} generation={session.generation} />
    </div>}
  </aside>;
  return <div className="cloud-workbench">
    <div className="cloud-toolbar">
      <nav className="presentation-switch" aria-label="呈现方式">
        <button type="button" aria-pressed={route.view === 'scene'} onClick={() => navigate({ ...route, view: 'scene' })}>场景</button>
        <button type="button" aria-pressed={route.view === 'list'} onClick={() => navigate({ ...route, view: 'list' })}>名册</button>
      </nav>
      <button type="button" onClick={refresh}>刷新清单</button>
    </div>
    <div className={gridClass}>{panel}
      {route.view === 'scene' && <section className="cloud-scene" aria-label="云端办公室场景">
        {(rows || memory.current) && <SceneSpace studio={studio} scope={room} identityScope={JSON.stringify([session.scope, session.generation])}
          memory={memory} layoutMemory={layoutMemory} names={names} agents={actors} visibleKeys={visibleKeys}
          selectedId={activeKey} reading={!!activeKey} workspace={<></>} reader={null} directory={null} hideHumanMailbox
          population={{ revision: 0, complete }}
          panel="" onPanel={value => { if (value === 'directory') openRoster(); }} onClose={closeReader}
          onSelect={selectActor} onCabinet={openRoster} onReset={() => filterHost()} showEmpty={false}
          awaitStudioRoster={agents.read.kind === 'loading'} />}
      </section>}
      {hasReader && <section className="cloud-reader" aria-label="精确 Agent 实例">
        <header className="cloud-reader-heading"><div><h2 ref={readerTitle} tabIndex={-1}>{selected ? `@${selected.name}` : '精确实例链接'}</h2>
          {selected && <code>{selected.host} · {selected.incarnation}</code>}</div>
          <button type="button" onClick={closeReader} aria-label="关闭实例详情">×</button>
        </header>
        {stale && <p className="cloud-empty" role="alert">链接中的 Scope 代次已变化。请从清单选择实例。</p>}
        {route.invalid && <p className="cloud-empty" role="alert">链接中的精确实例标识不完整或无效。</p>}
        {selected && session.access === 'scope' && <CloudMessages key={exactKey(selected)} session={session} agent={selected} />}
        {selected && session.access === 'admin' && <AdminAgent key={JSON.stringify([freshness, exactKey(selected)])} session={session} agent={selected} onStop={stop} />}
      </section>}
      {!hasReader && route.view === 'list' && <div className="cloud-reader-placeholder">从名册选择一个 Agent 实例</div>}
    </div>
    {session.access === 'admin' && <div className="cloud-control">
      <DeploymentControlDialog client={session.client} target={target} onClose={closeControl} />
    </div>}
  </div>;
}

function AdminAgent({ session, agent, onStop }: { session: AdminSession; agent: CloudAgent; onStop(agent: CloudAgent): void }) {
  const load = useCallback((signal: AbortSignal) => resolveAdminAgent(session, agent, signal), [session, agent.host, agent.name, agent.incarnation]);
  const { read, retry } = useRead(load);
  if (read.kind === 'loading') return <p className="cloud-note" role="status">正在核对精确实例…</p>;
  if (read.kind === 'error') return <div className="cloud-empty" role="alert">{cloudProblem(read.error, '无法核对实例。')} <button type="button" onClick={retry}>重试</button></div>;
  if (!read.data) return <p className="cloud-empty">此实例不在当前清单中。</p>;
  return <div className="cloud-admin-agent"><div><strong>已注册</strong><p className="cloud-note">云端消息需要 Scope 令牌。</p></div>
    <button className="cloud-danger" type="button" onClick={() => onStop(agent)}>停止此实例</button></div>;
}

type Page<T> = { items: T[]; next: string | null };
function ReadPage<T>({ value, cursor, retry, empty, error, children }: {
  value: { kind: 'loading' } | { kind: 'error'; error: unknown } | { kind: 'ready'; data: Page<T> };
  cursor: ReturnType<typeof useCursor>; retry(): void; empty: string; error: string; children(items: T[]): ReactNode;
}) {
  const next = value.kind === 'ready' ? value.data.next : null;
  return <>
    {value.kind === 'loading' && <p className="cloud-empty" role="status">正在读取清单…</p>}
    {value.kind === 'error' && <div className="cloud-empty" role="alert">{cloudProblem(value.error, error)} <button type="button" onClick={retry}>重试</button></div>}
    {value.kind === 'ready' && (value.data.items.length ? children(value.data.items) : <p className="cloud-empty">{empty}</p>)}
    {(cursor.index > 0 || next) && <nav className="cloud-pager" aria-label="清单分页">
      <button type="button" onClick={cursor.back} disabled={!cursor.index}>上一页</button><span>第 {cursor.index + 1} 页</span>
      <button type="button" onClick={() => { if (next) cursor.next(next); }} disabled={!next || next === cursor.after}>下一页</button>
    </nav>}
  </>;
}
