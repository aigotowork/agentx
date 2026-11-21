import { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { DeploymentControlDialog, DeploymentReceiptLookup, type ControlTarget } from './control';
import { EmptyState, Icon, Timestamp } from '../presentation';
import { explainError, pageSize, useCursor, useRead, type AdminClient, type AdminOverview, type Read, type Scope, type ScopeWriteAction } from './model';
import { ScopeWriteDialog } from './write-dialog';

function NativeTime({ value }: { value: string | number | bigint }) {
  const raw = String(value);
  const date = /^\d+$/.test(raw) ? new Date(Number(raw) * 1000) : new Date(raw);
  return Number.isNaN(date.valueOf()) ? <code>{raw}</code> : <Timestamp value={date.toISOString()} />;
}

function StatusDot({ active, label }: { active: boolean; label: string }) {
  return <span className="status-label" data-tone={active ? 'ready' : 'warn'}><i aria-hidden="true" />{label}</span>;
}

function ReadFeedback({ read, label, retry }: { read: Exclude<Read<unknown>, { kind: 'ready' }>; label: string; retry: () => void }) {
  if (read.kind === 'loading') return <p className="deployment-feedback muted" role="status">正在读取{label}…</p>;
  return <div className="deployment-feedback"><p className="deployment-error" role="alert">{explainError(read.error, `无法读取${label}。`)}</p><button onClick={retry}>重试</button></div>;
}

function Overview({ client, initial, revision, refresh }: { client: AdminClient; initial: AdminOverview; revision: number; refresh: () => void }) {
  const load = useCallback((signal: AbortSignal) => revision ? client.overview({ signal }) : Promise.resolve(initial), [client, initial, revision]);
  const { read, retry } = useRead(load);
  const stats = read.kind === 'ready' ? read.data : null;
  return <section className="deployment-overview" aria-label="部署概况">
    <div className="deployment-overview-heading"><h1>部署概况</h1>{stats && <NativeTime value={stats.observed_at} />}</div>
    {stats ? <dl className="deployment-metrics">
      <div><dt>Scopes</dt><dd>{String(stats.scopes)}</dd></div>
      <div><dt>主机在线</dt><dd>{String(stats.online_hosts)} <small>/ {String(stats.hosts)}</small></dd></div>
      <div><dt>Agents</dt><dd>{String(stats.agents)}</dd></div>
      <div><dt>不可用 Scopes</dt><dd data-unavailable={Boolean(stats.unavailable)}>{String(stats.unavailable)}</dd></div>
    </dl> : read.kind !== 'ready' && <ReadFeedback read={read} label="部署概况" retry={retry} />}
    <div className="deployment-overview-actions">
      {stats && <details className="scope-control deployment-identity">
        <summary aria-label="部署标识" title="部署标识"><Icon name="info" /></summary>
        <dl><dt>部署 ID</dt><dd><code>{stats.deployment_id}</code></dd><dt>协议版本</dt><dd><code>v{String(stats.protocol_version)}</code></dd></dl>
      </details>}
      <button className="quiet icon-control" onClick={refresh} aria-label="刷新部署" title="刷新部署"><Icon name="refresh" /></button>
    </div>
  </section>;
}

type Cursor = ReturnType<typeof useCursor>;

function Pages({ cursor, next, label }: { cursor: Cursor; next?: string | null; label: string }) {
  if (cursor.index === 0 && !next) return null;
  return <nav className="deployment-pages" aria-label={`${label}分页`}>
    <button type="button" className="quiet icon-control" onClick={cursor.back} disabled={!cursor.index} aria-label={`${label}上一页`} title="上一页"><Icon name="back" /></button>
    <span className="muted">第 {cursor.index + 1} 页</span>
    <button type="button" className="quiet icon-control" onClick={() => { if (next) cursor.next(next); }} disabled={!next} aria-label={`${label}下一页`} title="下一页"><Icon name="chevron" /></button>
  </nav>;
}

function ScopeCollection({ client, selected, onSelect, onCreate }: { client: AdminClient; selected: string | null; onSelect: (id: string) => void; onCreate: () => void }) {
  const cursor = useCursor();
  const { after } = cursor;
  const load = useCallback((signal: AbortSignal) => client.listScopes({ after, limit: pageSize }, { signal }), [client, after]);
  const { read, retry } = useRead(load);
  const rows = read.kind === 'ready' ? read.data.items : [];
  return <aside className="collection-rail deployment-collection" aria-labelledby="deployment-list-title">
    <div className="collection-header section-heading">
      <h2 id="deployment-list-title" tabIndex={-1}>Scopes</h2>
      <button className="primary icon-button" onClick={onCreate}><Icon name="plus" />新建 Scope</button>
    </div>
    <div className="collection-list">
      {read.kind !== 'ready' ? <ReadFeedback read={read} label="Scopes" retry={retry} /> : rows.length ? rows.map(scope =>
        <button type="button" className="collection-row deployment-scope-row" aria-pressed={selected === scope.id} key={scope.id} onClick={() => onSelect(scope.id)}>
          <strong>{scope.id}</strong>
          <StatusDot active={scope.available} label={scope.available ? '可用' : '不可用'} />
          <small>{String(scope.online_hosts)} / {String(scope.hosts)} 主机在线 · {String(scope.agents)} Agents</small>
        </button>) : <EmptyState icon="workspace" title="暂无 Scope" />}
    </div>
    <Pages cursor={cursor} next={read.kind === 'ready' ? read.data.next : null} label="Scopes" />
  </aside>;
}

type InventoryKind = 'hosts' | 'agents';
type InventorySelection = { kind: 'hosts' | 'receipts' } | { kind: 'agents'; host?: string };
type Host = Awaited<ReturnType<AdminClient['listHosts']>>['items'][number];
type Agent = Awaited<ReturnType<AdminClient['listAgents']>>['items'][number];

type InventoryPage<T> = { items: T[]; next?: string | null };
type InventoryRecord = { kind: 'hosts'; page: InventoryPage<Host> } | { kind: 'agents'; page: InventoryPage<Agent> };

function InventoryTable({ inventory, onInspectHost, onLaunch, onStop }: {
  inventory: InventoryRecord; onInspectHost: (host: string) => void; onLaunch: (host: Host) => void; onStop: (agent: Agent) => void;
}) {
  if (inventory.kind === 'hosts') return <table className="deployment-table" aria-label="主机清单">
    <thead><tr><th scope="col">主机</th><th scope="col">状态</th><th scope="col">最后在线</th><th className="deployment-row-action" scope="col">操作</th></tr></thead>
    <tbody>{inventory.page.items.map(host => <tr key={host.name}>
      <th scope="row"><button className="quiet deployment-host-drill" onClick={() => onInspectHost(host.name)} aria-label={`查看 ${host.name} 的 Agents`} title="查看此主机的 Agents"><code>{host.name}</code><Icon name="chevron" /></button></th>
      <td><StatusDot active={host.online} label={host.online ? '在线' : '离线'} /></td><td><NativeTime value={host.last_seen} /></td>
      <td className="deployment-row-action"><button className="quiet" onClick={() => onLaunch(host)} aria-label={`在 ${host.name} 启动 Agent`}>启动</button></td>
    </tr>)}</tbody>
  </table>;
  return <table className="deployment-table" aria-label="Agent 清单">
    <thead><tr><th scope="col">Agent</th><th scope="col">主机</th><th scope="col">实例标识</th><th className="deployment-row-action" scope="col">操作</th></tr></thead>
    <tbody>{inventory.page.items.map(agent => <tr key={agent.name}><th scope="row"><code>{agent.name}</code></th><td><code>{agent.host}</code></td><td><code>{String(agent.incarnation)}</code></td>
      <td className="deployment-row-action"><button className="quiet" onClick={() => onStop(agent)} aria-label={`停止 ${agent.name}`}>停止</button></td></tr>)}</tbody>
  </table>;
}

function Inventory({ client, id, kind, host, onInspectHost, onLaunch, onStop }: {
  client: AdminClient; id: string; kind: InventoryKind; host?: string;
  onInspectHost: (host: string) => void; onLaunch: (host: Host) => void; onStop: (agent: Agent) => void;
}) {
  const cursor = useCursor();
  const { after } = cursor;
  const load = useCallback(async (signal: AbortSignal): Promise<InventoryRecord> => {
    const query = { after, limit: pageSize };
    return kind === 'hosts' ? { kind, page: await client.listHosts(id, query, { signal }) } : { kind, page: await client.listAgents(id, { ...query, host }, { signal }) };
  }, [client, id, kind, host, after]);
  const { read, retry } = useRead(load);
  const label = kind === 'hosts' ? '主机' : 'Agents';
  const next = read.kind === 'ready' ? read.data.page.next : null;
  return <>
    <div className="deployment-inventory">
      {read.kind !== 'ready' ? <ReadFeedback read={read} label={`${label}清单`} retry={retry} /> : read.data.page.items.length ? <InventoryTable inventory={read.data} onInspectHost={onInspectHost} onLaunch={onLaunch} onStop={onStop} /> : <EmptyState icon={kind === 'hosts' ? 'workspace' : 'agents'} title={kind === 'hosts' ? '暂无主机' : '暂无 Agent'} />}
    </div>
    <Pages cursor={cursor} next={next} label={label} />
  </>;
}

function ScopeDetails({ client, id, onBack, onRotate, onControl }: {
  client: AdminClient; id: string; onBack: () => void; onRotate: (scope: Scope) => void; onControl: (target: ControlTarget) => void;
}) {
  const [selection, setSelection] = useState<InventorySelection>({ kind: 'hosts' });
  const { kind } = selection;
  const host = kind === 'agents' ? selection.host : undefined;
  const hostFilter = useRef<HTMLDivElement>(null);
  const agentsTab = useRef<HTMLButtonElement>(null);
  useLayoutEffect(() => { if (host) hostFilter.current?.focus(); }, [host]);
  const clearHost = () => { setSelection({ kind: 'agents' }); agentsTab.current?.focus(); };
  const inspectHost = (name: string) => setSelection({ kind: 'agents', host: name });
  const load = useCallback((signal: AbortSignal) => client.getScope(id, { signal }), [client, id]);
  const { read, retry } = useRead(load);
  const scope = read.kind === 'ready' ? read.data : null;
  const launch = (row: Host) => { if (scope) onControl({ scope: id, generation: scope.generation, host: row.name, kind: 'launch' }); };
  const stop = (agent: Agent) => { if (scope) onControl({ scope: id, generation: scope.generation, host: agent.host, kind: 'stop', name: agent.name, incarnation: agent.incarnation }); };
  return <section className="deployment-detail" aria-labelledby="deployment-scope-title">
    <header className="deployment-detail-header">
      <button className="quiet icon-button deployment-back" onClick={onBack}><Icon name="back" />Scopes</button>
      <div className="deployment-scope-title"><h2 id="deployment-scope-title" tabIndex={-1}>{id}</h2>{scope && <StatusDot active={scope.available} label={scope.available ? '可用' : '不可用'} />}</div>
      {scope && <>
        <div className="deployment-credential"><span className="muted">令牌版本 <code>v{String(scope.credential_version)}</code></span><button onClick={() => onRotate(scope)}>轮换令牌</button></div>
        <p className="deployment-scope-url"><code>{scope.scope_url}</code></p>
        <details className="deployment-generation"><summary>代次</summary><code>{scope.generation}</code></details>
      </>}
    </header>
    {read.kind !== 'ready' ? <ReadFeedback read={read} label="Scope" retry={retry} /> : <>
        <div className="detail-tabs" role="group" aria-label="Scope 清单">
          <button aria-pressed={kind === 'hosts'} onClick={() => setSelection({ kind: 'hosts' })}>主机 <span className="tab-count">{String(read.data.hosts)}</span></button>
          <button ref={agentsTab} aria-pressed={kind === 'agents'} onClick={clearHost}>Agents <span className="tab-count">{String(read.data.agents)}</span></button>
          <button aria-pressed={kind === 'receipts'} onClick={() => setSelection({ kind: 'receipts' })}>操作回执</button>
        </div>
        {host && <div className="deployment-host-filter" ref={hostFilter} tabIndex={-1}><span>主机 <code>{host}</code></span><button className="quiet" onClick={clearHost}>查看所有 Agents</button></div>}
        {kind === 'receipts' ? <DeploymentReceiptLookup client={client} scope={id} generation={read.data.generation} /> : read.data.available ?
          <Inventory key={JSON.stringify([kind, host])} client={client} id={id} kind={kind} host={host} onInspectHost={inspectHost} onLaunch={launch} onStop={stop} /> :
          <EmptyState icon="warning" title="Scope 清单暂不可用" />}
      </>}
  </section>;
}

/**
 * @aiPurpose Show deployment aggregates, paged scopes, and the exact selected inventory and credential controls.
 * @aiUsage Mount for one authenticated deployment session; remount to disconnect and reset all selection and drafts.
 */
export function DeploymentWorkspace({ client, initialOverview }: { client: AdminClient; initialOverview: AdminOverview }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const [write, setWrite] = useState<ScopeWriteAction | null>(null);
  const [control, setControl] = useState<ControlTarget | null>(null);
  const main = useRef<HTMLElement>(null);
  const requestedFocus = useRef(false);
  const refresh = () => setRevision(value => value + 1);
  const select = (id: string | null) => { requestedFocus.current = true; setSelected(id); };
  useLayoutEffect(() => {
    if (!requestedFocus.current) return;
    requestedFocus.current = false;
    if (matchMedia('(max-width: 700px)').matches) main.current?.querySelector<HTMLElement>(selected ? '#deployment-scope-title' : '#deployment-list-title')?.focus();
  }, [selected]);

  return <main ref={main} className="console-main deployment-main" tabIndex={-1}>
    <Overview client={client} initial={initialOverview} revision={revision} refresh={refresh} />
    <div className="workspace deployment-workspace" data-selecting={!selected}>
      <ScopeCollection key={revision} client={client} selected={selected} onSelect={select} onCreate={() => setWrite({ kind: 'create' })} />
      {selected ? <ScopeDetails key={`${revision}:${selected}`} client={client} id={selected} onBack={() => select(null)} onRotate={scope => setWrite({ kind: 'rotate', scope })} onControl={setControl} /> :
        <section className="deployment-detail deployment-unselected"><EmptyState icon="route" title="选择一个 Scope" /></section>}
    </div>
    <ScopeWriteDialog client={client} action={write} onDismiss={() => setWrite(null)} onRefresh={refresh} onSaved={id => { setSelected(id); refresh(); }} />
    <DeploymentControlDialog client={client} target={control} onClose={() => setControl(null)} />
  </main>;
}
