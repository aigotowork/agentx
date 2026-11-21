import { useEffect, useRef, useState } from 'react';
import type { AgentRef, AgentView } from './api/native';
import type { Management } from './use-management';
import { MembershipFacts } from './offices';
import { matchesOffice, membershipLabel } from './use-offices';
import { messageOf, refKey } from './model';
import { LabelEditor } from './labels/editor';
import { LabelSelection } from './labels/controls';
import { groupAgents } from './labels/model';
import { EmptyState, Icon, State } from './presentation';
import { InteractionDialog } from './dialog';

function AgentRow({ agent, selected, onSelect }: { agent: AgentView; selected: boolean; onSelect: () => void }) {
  const membership = membershipLabel(agent.collaboration);
  return <button className="agent-row collection-row" onClick={onSelect} aria-pressed={selected}>
    <span className="agent-avatar" aria-hidden="true">{agent.agent.name.slice(0, 2)}</span>
    <span className="agent-row-copy"><span className="collection-row-title"><strong>{agent.agent.name}</strong><State value={agent.status} /></span>
    <span className="agent-meta ellipsis" title={agent.title || membership}>{agent.title || membership}</span></span>
  </button>;
}

/**
 * @aiPurpose Find and select exact current executions or stopped snapshots.
 * @aiUsage Search, office, lifecycle and grouping use loaded rows; label selectors use native filtering.
 * @aiLayout Desktop index and mobile picker share filter and scroll state.
 */
export function AgentResources({ model, onPick }: { model: Management; onPick: (agent: AgentRef) => void }) {
  const [filter, setFilter] = useState('all');
  const [search, setSearch] = useState('');
  const [expanded, setExpanded] = useState(false);
  const [groupKey, setGroupKey] = useState('');
  const searchInput = useRef<HTMLInputElement>(null);
  const picker = useRef<HTMLButtonElement>(null);
  const visible = model.agents.filter(agent => {
    const stopped = agent.agent.stopped_event != null;
    const matchesFilter = filter === 'all' || (filter === 'stopped' ? stopped : !stopped);
    const content = [agent.agent.name, agent.title, agent.tool, agent.directory, membershipLabel(agent.collaboration)].join(' ').toLowerCase();
    return matchesFilter && matchesOffice(agent, model.offices.filter) && content.includes(search.trim().toLowerCase());
  });
  const officePaths = [...new Set([...(model.offices.catalog?.workspaces.map(office => office.path) ?? []), ...model.agents.flatMap(agent => agent.collaboration.status === 'adopted' && agent.collaboration.workspace ? [agent.collaboration.workspace] : []), ...(model.offices.filter.startsWith('/') ? [model.offices.filter] : [])])];
  const groups = groupAgents(visible, groupKey);
  const filtered = !!search || filter !== 'all' || model.offices.filter !== 'all' || model.labelSelectors.length > 0;
  const pickerOpen = expanded || !model.selected;
  const pick = (agent: AgentView) => { onPick(agent.agent); setExpanded(false); };
  const reset = () => { setSearch(''); setFilter('all'); setGroupKey(''); model.offices.choose('all'); if (model.labelSelectors.length) model.applyLabelSelectors([]); };
  useEffect(() => {
    function shortcut(event: KeyboardEvent) {
      if (event.key !== '/' || event.ctrlKey || event.altKey || event.metaKey || event.defaultPrevented) return;
      if ((event.target as HTMLElement | null)?.closest('input,textarea,select,[contenteditable],dialog') || !picker.current?.closest('.destination:not([hidden])')) return;
      event.preventDefault();
      setExpanded(true);
      requestAnimationFrame(() => searchInput.current?.focus());
    }
    window.addEventListener('keydown', shortcut);
    return () => window.removeEventListener('keydown', shortcut);
  }, []);
  const railClass = pickerOpen ? 'resource-rail collection-rail expanded' : 'resource-rail collection-rail';
  return <aside className={railClass} aria-label="Agent 列表">
    <button ref={picker} className="mobile-picker icon-button" disabled={!model.selected} onClick={() => setExpanded(value => !value)} aria-expanded={pickerOpen}><Icon name="agents" />Agent 列表<Icon name={pickerOpen ? 'down' : 'chevron'} /></button>
    <div className="resource-controls collection-tools">
      <div className="section-heading"><h2>Agent 列表</h2><button className="quiet icon-control" onClick={model.refresh} aria-label="刷新 Agent 列表" title="刷新 Agent 列表"><Icon name="refresh" /></button></div>
      <div className="search-field"><Icon name="search" /><input ref={searchInput} aria-label="搜索已加载的 Agent" placeholder="搜索名称、标题、目录…" value={search} onChange={event => setSearch(event.target.value)} onKeyDown={event => { if (event.key === 'Escape') { if (search) setSearch(''); else { setExpanded(false); picker.current?.focus(); } } }} /><kbd aria-hidden="true">/</kbd></div>
      <div className="segmented" aria-label="Agent 范围">{[['all', '全部'], ['live', '当前'], ['stopped', '已停止']].map(([value, label]) => <button key={value} aria-pressed={filter === value} onClick={() => setFilter(value)}>{label}</button>)}</div>
      <details className="agent-filter-options"><summary>工作区与标签{filtered && <span className="filter-active" aria-label="已应用筛选" />}</summary>
        <label>已加载成员<select aria-label="办公室筛选" value={model.offices.filter} onChange={event => model.offices.choose(event.target.value)}><option value="all">全部工作区</option><option value="global">全局（明确归属）</option><option value="legacy">旧记录（未记录归属）</option><option value="unavailable">归属不可用</option>{officePaths.map(path => <option key={path} value={path}>{path}</option>)}</select></label>
        <LabelSelection supported={model.descriptor?.supports_labels === true} selectors={model.labelSelectors} onApply={model.applyLabelSelectors} groupKey={groupKey} onGroup={setGroupKey} />
      </details>
    </div>
    <div className="collection-meta"><span>{visible.length} 个匹配 · 已加载 {model.agents.length}{model.hasMore ? ' · 有更多' : ''}</span>{filtered && <button className="quiet reset-filter" onClick={reset}>清除筛选</button>}</div>
    <div className="agent-list collection-list">{groups.map(group => <section key={group.id} className="label-group">{groupKey && <h3>{groupKey} · {group.title} <span className="count">{group.agents.length} 已加载</span></h3>}{group.agents.map(agent => <AgentRow key={refKey(agent.agent)} agent={agent} selected={!!model.selected && refKey(agent.agent) === refKey(model.selected)} onSelect={() => pick(agent)} />)}</section>)}
      {!visible.length && <EmptyState icon={filtered ? 'search' : 'agents'} title={model.loading ? '读取 Agent…' : model.error && !model.agents.length ? '暂时无法读取' : filtered ? '没有匹配的已加载 Agent' : '还没有 Agent'}>{!model.loading && !model.error && filtered && '可清除筛选，或继续加载。'}</EmptyState>}
      {model.hasMore && <button className="load-more quiet" onClick={model.loadMore}>加载更多 Agent</button>}
    </div>
  </aside>;
}

/** @aiPurpose Show one exact Agent and its allowed lifecycle actions. @aiUsage Key by generation and exact reference; callbacks point to the original operation ID. */
export function AgentToolbar({ model, onOperation, onOpenOperation }: { model: Management; onOperation: (id: string) => void; onOpenOperation: (id: string) => void }) {
  const [stopOpen, setStopOpen] = useState(false);
  const [error, setError] = useState('');
  const agent = model.detail;
  const target = model.selected;
  const lifecycleKind = agent?.can_resume || target?.stopped_event ? 'resume' : 'stop';
  const uncertain = target && model.receipts.find(receipt => receipt.kind === lifecycleKind && receipt.operation?.status === 'Unknown' && receipt.body.generation === model.descriptor?.generation && 'agent' in receipt.body && refKey(receipt.body.agent) === refKey(target));
  const canStop = !!agent?.can_stop && !model.targetPending && !uncertain;
  function openStop() { setError(''); setStopOpen(true); }
  const closeStop = () => setStopOpen(false);
  async function act(kind: 'stop' | 'resume') {
    setError('');
    try {
      await model.submit(kind, undefined, id => { if (kind === 'stop') closeStop(); onOperation(id); });
    } catch (error) { setError(messageOf(error)); }
  }
  return <header className="agent-toolbar">
    <div className="selected-heading">
      <span className="agent-avatar subject-avatar" aria-hidden="true">{target?.name.slice(0, 2)}</span>
      <div className="subject-copy"><div className="subject-name"><h1>{target?.name}</h1>{agent && <State value={agent.status} />}</div>
        {agent?.title && <p className="subject-title">{agent.title}</p>}
        {agent && <p className="selected-meta ellipsis" title={agent.directory}>{agent.tool} <span>·</span> <code>{agent.directory}</code></p>}
      </div>
      {agent && <div className="lifecycle-actions">{uncertain ? <button className="icon-button" onClick={() => onOpenOperation(uncertain.id)}><Icon name="operations" />查原请求</button> : lifecycleKind === 'resume' ? <button className="icon-button" disabled={!agent.can_resume || model.targetPending} onClick={() => void act('resume')}><Icon name="resume" />恢复</button> : <button className="danger icon-button" disabled={!canStop} onClick={openStop}><Icon name="stop" />停止</button>}</div>}
    </div>
    {agent && <div className="subject-facts"><MembershipFacts facts={agent.collaboration} /><LabelEditor model={model} onOperation={onOperation} onOpenOperation={onOpenOperation} />
      <details className="identity-details"><summary aria-label="Agent 精确信息"><Icon name="info" />执行信息</summary><dl><dt>状态</dt><dd>{agent.status} · {agent.detail || '未附说明'}</dd>{agent.status_context && <><dt>状态上下文</dt><dd><code>{agent.status_context}</code></dd></>}{agent.pending_messages != null && <><dt>待投递消息</dt><dd>{agent.pending_messages.toString()}</dd></>}{agent.request_error && <><dt>模型请求异常</dt><dd>{agent.request_error.reason} · 连续失败 {agent.request_error.consecutive_failures.toString()} 次</dd></>}<dt>执行实例</dt><dd><code>{agent.agent.incarnation ?? '—'}</code></dd><dt>停止事件</dt><dd><code>{agent.agent.stopped_event ?? '—'}</code></dd><dt>会话</dt><dd><code>{agent.session_id ?? '—'}</code></dd></dl></details>
    </div>}
    {!agent && !model.detailError && <p className="muted" role="status">读取执行信息…</p>}
    {(model.detailError || (!stopOpen && error)) && <p role="alert" className="error">{model.detailError || error}</p>}
    <InteractionDialog open={stopOpen} variant="stop" icon="stop" title={`停止 @${target?.name}？`} description="发送停止请求，实际状态以执行实例上报为准。" closeLabel="关闭停止确认" onDismiss={closeStop}
      footer={<div className="dialog-buttons"><button data-dialog-focus onClick={closeStop}>取消</button><button className="danger solid-danger icon-button" disabled={!canStop} onClick={() => void act('stop')}><Icon name="stop" />确认停止</button></div>}>
      <div className="stop-decision">
        {agent && <div className="stop-target"><span className="muted">{agent.title || agent.tool}</span><State value={agent.status} /></div>}
        <dl><dt>执行实例</dt><dd><code>{target?.incarnation ?? '—'}</code></dd>{agent?.directory && <><dt>工作目录</dt><dd><code>{agent.directory}</code></dd></>}</dl>
        {!canStop && <p className="warning-note">此执行当前不可停止。请返回查看最新状态。</p>}
        {error && <p role="alert" className="field-error"><Icon name="warning" />{error}</p>}
      </div>
    </InteractionDialog>
  </header>;
}
