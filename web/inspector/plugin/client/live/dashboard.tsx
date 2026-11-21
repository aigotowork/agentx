import { useEffect, useState, type CSSProperties } from 'react';
import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import type { PluginTheme } from '../../../host/contracts';
import { descriptor, directory, directoryAgent, directoryReference, type AgentView, type Descriptor } from '../../../host/live/client';
import { currentCounts, datasetKey, referenceKey, selectionKey } from '../../../host/live/model';
import { inspection } from '../../../host/live/inspection';
import { Button, ViewTabs, type ViewTab } from '../../../uiux/systems/inspector/inspector.native';
import { OverlayBoundary } from '../../../uiux/systems/inspector/inspector.native';
import { WorkspaceLayout } from './layouts/workspace';
import { NativeSession } from './session';
import { CollectionBlock, ObservationTable, TokenTable } from './collection';
import { AnalysisBlock } from './analysis';
import { RecoveryBlock } from './recovery';

type Tab = 'agents' | 'board' | 'tokens' | 'analytics' | 'compare' | 'activity' | 'insights' | 'recovery';
const tabs: ViewTab<Tab>[] = [
  { value: 'agents', label: '会话', icon: 'MessagesSquare', colors: ['sky', 'blue'] },
  { value: 'board', label: '看板', icon: 'LayoutDashboard', colors: ['violet', 'indigo'] },
  { value: 'tokens', label: 'Token', icon: 'Coins', colors: ['amber', 'orange'] },
  { value: 'analytics', label: '明细', icon: 'ChartNoAxesCombined', colors: ['cyan', 'teal'] },
  { value: 'compare', label: '横评', icon: 'GitCompareArrows', colors: ['pink', 'fuchsia'] },
  { value: 'activity', label: '事件', icon: 'Activity', colors: ['teal', 'green'] },
  { value: 'insights', label: '分析', icon: 'Sparkles', colors: ['indigo', 'violet'] },
  { value: 'recovery', label: '恢复', icon: 'HeartPulse', colors: ['orange', 'red'] },
];
const initialTab = (route: { screenId: string; params?: Record<string, string> }): Tab => {
  const params = new URLSearchParams(location.search);
  const requested = route.params?.tab ?? (params.get('experience') === 'recovery' ? 'recovery' : route.screenId === 'insight-jobs' ? 'insights' : route.screenId === 'diagnostics' ? 'analytics' : 'agents');
  return tabs.find(tab => tab.value === requested)?.value ?? 'agents';
};

/** @aiPurpose Connect the existing Inspector shell to its authoritative same-origin scope and reset fence.
 * @aiUsage Live entry only; explicit demo routes retain their independent historical provider.
 * @aiBehavior Scope/generation replacement unmounts all selections and pending effects before new actions. */
export function LiveDashboard({ theme, compact, route }: { theme: PluginTheme; compact: boolean; route: { screenId: string; params?: Record<string, string> } }) {
  const host = useQuery({ queryKey: ['native-descriptor'], queryFn: ({ signal }) => descriptor(signal), refetchInterval: 5000 });
  if (host.error) return <div className="host-connection" role="alert"><span>{String(host.error)}</span><Button theme={theme} label="重新连接" onPress={() => void host.refetch()} /></div>;
  if (!host.data) return <div className="host-connection" role="status">正在连接 AgentX…</div>;
  return <OverlayBoundary theme={theme} scopeKey={datasetKey(host.data)}><NativeWorkspace key={datasetKey(host.data)} dataset={host.data} theme={theme} compact={compact} route={route} /></OverlayBoundary>;
}

function NativeWorkspace({ dataset, theme, compact, route }: { dataset: Descriptor; theme: PluginTheme; compact: boolean; route: { screenId: string; params?: Record<string, string> } }) {
  const [tab, setTab] = useState<Tab>(() => initialTab(route));
  const [filter, setFilter] = useState('');
  const [selected, setSelected] = useState<string | null>(() => new URLSearchParams(location.search).get('ref'));
  const [historyMode, setHistoryMode] = useState(false);
  const needsDirectory = tab === 'agents' || tab === 'board' || tab === 'compare';
  const needsCollection = tab !== 'agents';
  const listing = useInfiniteQuery({
    queryKey: ['native-directory', datasetKey(dataset), historyMode ? 'all' : 'current'],
    initialPageParam: undefined as string | undefined,
    queryFn: ({ signal, pageParam }) => directory(dataset.generation, signal, !historyMode, pageParam),
    getNextPageParam: (page, _pages, cursor, cursors) => {
      if (page.next && (page.next === cursor || cursors.includes(page.next))) return undefined;
      return page.next ?? undefined;
    },
    enabled: needsDirectory,
    refetchInterval: needsDirectory && !historyMode ? 5000 : false,
    refetchOnWindowFocus: !historyMode,
    refetchIntervalInBackground: false,
    staleTime: historyMode ? Infinity : 0,
    retry: false,
  });
  const collection = useQuery({ queryKey: ['native-inspection', datasetKey(dataset)], queryFn: ({ signal }) => inspection(dataset.generation, signal), enabled: needsCollection, refetchInterval: needsCollection ? 4000 : false });
  useEffect(() => { setTab(initialTab(route)); }, [route]);
  // Exact references deduplicate pages while keeping native current/history identities distinct.
  const agents = [...new Map((listing.data?.pages.flatMap(page => page.agents) ?? []).map(row => [referenceKey(row.agent), row])).values()];
  const listedSelection = agents.find(item => referenceKey(item.agent) === selected);
  const selectedReference = directoryReference(selected);
  const exactSelection = useQuery({
    queryKey: ['native-selected-agent', datasetKey(dataset), selected],
    queryFn: ({ signal }) => directoryAgent(dataset.generation, selectedReference!, signal),
    enabled: tab === 'agents' && !!selectedReference && !listedSelection,
    retry: false,
    staleTime: 5000,
  });
  const lastPage = listing.data?.pages.at(-1);
  const cursorCycle = !!lastPage?.next && listing.data!.pageParams.includes(lastPage.next);
  const coverage = !listing.data ? '目录覆盖未知' : cursorCycle ? '游标重复 · 部分覆盖' : listing.hasNextPage ? historyMode && lastPage?.currentComplete ? '当前已覆盖 · 历史部分覆盖' : '部分覆盖' : historyMode ? '目录分页结束' : '当前成员分页结束';
  const refresh = () => {
    if (needsDirectory) void listing.refetch();
    if (needsCollection) void collection.refetch();
  };
  const visible = agents.filter(row => `${row.agent.name} ${row.directory} ${row.collaboration.workspace ?? ''} ${row.collaboration.role?.name ?? ''}`.toLowerCase().includes(filter.toLowerCase()));
  const row = listedSelection ?? exactSelection.data;
  const select = (item: AgentView) => {
    const key = referenceKey(item.agent); setSelected(key);
    const url = new URL(location.href); url.searchParams.set('ref', key); history.replaceState(null, '', url);
  };
  const navigate = (next: Tab) => { setTab(next); const url = new URL(location.href); url.searchParams.set('tab', next); history.replaceState(null, '', url); };
  const style = { '--native-border': theme.colors.border, '--native-surface': theme.colors.surface1, '--native-selected': theme.colors.surface2, '--native-accent': theme.colors.accent, '--native-danger': theme.colors.statusDanger, color: theme.colors.foreground, backgroundColor: theme.colors.surface0, flex: 1, minWidth: 0 } as CSSProperties;
  const directoryControls = needsDirectory ? <><span className="native-muted">已加载 {agents.length} · {coverage}</span><Button theme={theme} size="sm" label={historyMode ? '查看当前成员' : '全部（含停止历史）'} onPress={() => setHistoryMode(value => !value)} />{listing.hasNextPage && <Button theme={theme} size="sm" label={historyMode ? '加载更多记录' : '加载更多当前成员'} busy={listing.isFetching} onPress={() => { if (!listing.isFetching) void listing.fetchNextPage(); }} />}</> : null;
  const toolbar = <><ViewTabs theme={theme} tabs={tabs} value={tab} onChange={navigate} iconOnly={compact} accessibilityLabel="Inspector 视图" /><div className="native-actions"><span className="native-muted" title={`${dataset.scope}\n${dataset.generation}`}>{dataset.project}</span>{directoryControls}<Button theme={theme} size="sm" label="刷新" busy={(needsDirectory && listing.isFetching) || (needsCollection && collection.isFetching)} onPress={refresh} /></div></>;
  const sidebar = tab === 'agents' ? <><input className="native-search" aria-label="筛选会话" placeholder="名称 / 目录 / Role…" value={filter} onChange={event => setFilter(event.target.value)} />{listing.isPending && <p role="status">正在读取会话…</p>}{!visible.length && !listing.isPending && <p className="native-muted">没有匹配的已加载会话</p>}{visible.map(item => <button key={referenceKey(item.agent)} className="native-resource" aria-pressed={selected === referenceKey(item.agent)} onClick={() => select(item)}><strong>{item.agent.name}</strong> · {item.status}<small>{item.tool} · {item.agent.stopped_event ? `停止 #${item.agent.stopped_event}` : `实例 ${item.agent.incarnation ?? '未知'}`}</small><small>{item.collaboration.role?.name ?? '—'} · {item.collaboration.workspace ?? '全局 / 未知'}</small></button>)}</> : undefined;
  let content;
  if (tab === 'agents') content = row ? <NativeSession key={selectionKey(dataset, row.agent)} dataset={dataset} row={row} theme={theme} /> : <p className="native-muted">{selected ? '原引用当前未加载或已失效，请重新选择。' : '选择会话查看精确实例、消息与模型对话。'}</p>;
  else if (tab === 'board') content = <><h2>{historyMode ? '全部目录快照（含停止历史）' : '当前成员快照'}</h2><table><thead><tr><th>原生状态</th><th>已加载记录</th></tr></thead><tbody>{currentCounts(agents).map(count => <tr key={count.status}><td>{count.status}</td><td>{count.count}</td></tr>)}</tbody></table><p className="native-muted">当前与停止记录分别计数；不代表模型轮次或任务完成率。</p><CollectionBlock theme={theme} dataset={dataset} snapshot={collection.data} refresh={() => collection.refetch()} /></>;
  else if (tab === 'analytics') content = <><h2>采集与覆盖</h2><CollectionBlock theme={theme} dataset={dataset} snapshot={collection.data} refresh={() => collection.refetch()} /><ObservationTable snapshot={collection.data} /></>;
  else if (tab === 'tokens') content = <><h2>Token 观测</h2><TokenTable snapshot={collection.data} /></>;
  else if (tab === 'compare') content = <><h2>{historyMode ? '已加载会话对照（含停止历史）' : '当前会话对照'}</h2><ComparisonTable agents={agents} /><TokenTable snapshot={collection.data} /></>;
  else if (tab === 'activity') content = <><h2>采集事件</h2><ObservationTable snapshot={collection.data} /></>;
  else if (tab === 'insights') content = <AnalysisBlock theme={theme} dataset={dataset} snapshot={collection.data} refresh={() => collection.refetch()} />;
  else content = <RecoveryBlock theme={theme} dataset={dataset} snapshot={collection.data} refresh={() => collection.refetch()} />;
  return <div style={style}><WorkspaceLayout toolbar={toolbar} sidebar={sidebar}>{tab === 'agents' && exactSelection.isFetching && !row && <p role="status">正在读取所选会话…</p>}{tab === 'agents' && exactSelection.error && <p className="native-error" role="alert">所选会话：{String(exactSelection.error)}</p>}{needsDirectory && listing.error && <p className="native-error" role="alert">目录：{String(listing.error)}</p>}{collection.error && needsCollection && <p className="native-error" role="alert">采集：{String(collection.error)} <Button theme={theme} label="重新读取采集" size="sm" onPress={() => void collection.refetch()} /></p>}{!collection.data && collection.isPending && needsCollection && <p role="status">正在读取采集状态…</p>}{cursorCycle && needsDirectory && <p className="native-error" role="alert">目录游标重复，覆盖未知；请刷新。</p>}{content}</WorkspaceLayout></div>;
}

function ComparisonTable({ agents }: { agents: AgentView[] }) {
  return <table><thead><tr><th>实例</th><th>工具 / Role</th><th>原生状态</th><th>来源归属</th></tr></thead><tbody>{agents.map(row => <tr key={referenceKey(row.agent)}><td>{row.agent.name}<br /><span className="native-muted">{row.agent.stopped_event ? `停止 #${row.agent.stopped_event}` : row.agent.incarnation}</span></td><td>{row.tool} / {row.collaboration.role?.name ?? '未知'}</td><td>{row.status}</td><td>{row.collaboration.status} · {row.collaboration.workspace ?? '全局 / 未知'}</td></tr>)}</tbody></table>;
}
