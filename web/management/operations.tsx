import { stringifyJson } from '@agentx/client';
/** @aiPurpose Display and recover native operation receipts including directory routing outcomes. */
import { useState } from 'react';
import type { Management } from './use-management';
import type { Receipt } from './model';
import { messageOf, needsAttention } from './model';
import type { CommandResult } from './api/native';
import type { AgentRef } from './api/native';
import { EmptyState, Icon, State, Timestamp, type Glyph } from './presentation';

const kindLabels = { launch: '启动 Agent', 'directory-send': '目录消息', send: '发送消息', stop: '停止 Agent', resume: '恢复 Agent', transcript: '读取模型记录', labels: '更新标签' };
const statusOf = (receipt: Receipt) => receipt.rejected ? 'rejected' : receipt.operation?.status ?? 'unconfirmed';
function outcomeText(result: CommandResult): string {
  if ('Launched' in result) return `${result.Launched.agents.join(', ') || '无 Agent'}${result.Launched.failed ? ` · ${result.Launched.failed} 失败` : ''}${result.Launched.errors.length ? ` · ${result.Launched.errors.join('; ')}` : ''}`;
  if ('Sent' in result) return `消息 ${result.Sent.message_id} · ${result.Sent.recipients.join(', ')}`;
  if ('DirectorySent' in result) {
    const { entry, recipient, sent, created } = result.DirectorySent;
    return `${created ? '新建' : '复用'} @${recipient.name} · ${entry.resolved_workspace ?? '全局'} / ${entry.role.name} · 消息 ${sent.message_id} → ${sent.recipients.join(', ')}`;
  }
  if ('Stopped' in result) return `${result.Stopped.name} · ${result.Stopped.outcome === 'sent' ? '停止请求已发送，等待状态更新' : result.Stopped.outcome}`;
  if ('LabelsPatched' in result) return result.LabelsPatched.changed ? '标签已更新' : '标签没有变化';
  return result.Transcript.available ? `${result.Transcript.exchanges.length} 条模型交互` : result.Transcript.reason || '模型记录不可用';
}
function OperationDetail({ receipt, model, onAgent }: { receipt: Receipt; model: Management; onAgent: (agent: AgentRef) => void }) {
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const status = statusOf(receipt);
  const sameGeneration = receipt.body.generation === model.descriptor?.generation;
  const replayable = !receipt.rejected && receipt.missing && !receipt.operation && sameGeneration;
  async function act(replay = false) { setBusy(true); setError(''); try { if (replay) await model.replay(receipt); else await model.lookup(receipt); } catch (error) { setError(messageOf(error)); } finally { setBusy(false); } }
  const result = receipt.operation?.result;
  const resultText = result ? outcomeText(result) : '';
  const agent = result && 'DirectorySent' in result ? result.DirectorySent.recipient : 'agent' in receipt.body ? receipt.body.agent : undefined;
  return <article className="operation-row" data-native-status={status}><header className="operation-detail-heading"><div><span className="context-label">操作回执</span><h1>{kindLabels[receipt.kind]}</h1><p className="muted">{receipt.label} · <Timestamp value={receipt.at} /></p></div><State value={status} /></header>
    {resultText && <p className="operation-result">{resultText}</p>}
    {(receipt.issue || receipt.operation?.error || error) && <p className="error" role="status">{error || receipt.operation?.error || receipt.issue}</p>}
    {status === 'Unknown' && <p className="warning-note">服务已记录「结果未知」。可查询原操作，不能重发。</p>}
    {replayable && <p className="warning-note">服务尚未找到这条回执。重发将使用原请求与原 ID。</p>}
    {!sameGeneration && model.descriptor && <p className="warning-note">原请求属于另一数据代次，不能在这里查询或重发。</p>}
    <div className="operation-actions">
      <button className="icon-button" disabled={busy || receipt.rejected || !sameGeneration} onClick={() => void act()}><Icon name="refresh" />查询原操作</button>
      {replayable && <button disabled={busy} onClick={() => void act(true)}>以原 ID 重发</button>}
      {agent && sameGeneration && <button onClick={() => onAgent(agent)}>查看 @{agent.name}</button>}
      {!needsAttention(receipt) && <button onClick={() => model.dismiss(receipt.id)}>收起回执</button>}
    </div>
    <details className="receipt-raw"><summary>原始请求与结果</summary><dl><dt>请求 ID</dt><dd><code>{receipt.id}</code></dd><dt>数据代次</dt><dd><code>{receipt.body.generation}</code></dd></dl><pre>{stringifyJson(receipt.body)}</pre>{result && <pre>{stringifyJson(result)}</pre>}</details>
  </article>;
}

/**
 * @aiPurpose Browse, inspect and recover durable receipts independently of Agent selection.
 * @aiUsage An explicit selected ID opens the matching receipt; otherwise show the first in the active filter.
 */
export function OperationsWorkspace({ model, selectedId, onSelect, onBack, onAgent }: { model: Management; selectedId?: string; onSelect: (id?: string) => void; onBack: () => void; onAgent: (agent: AgentRef) => void }) {
  const [attentionOnly, setAttentionOnly] = useState(false);
  const attention = model.receipts.filter(needsAttention);
  const requested = model.receipts.find(receipt => receipt.id === selectedId);
  const visible = attentionOnly && (!requested || needsAttention(requested)) ? attention : model.receipts;
  const selected = requested ?? visible[0];
  const setFilter = (value: boolean) => { setAttentionOnly(value); onSelect(undefined); };
  return <section className="workspace operations-panel" aria-label="操作记录">
    <aside className="collection-rail">
      <header className="collection-header"><button className="quiet icon-button" aria-label="返回工作页" onClick={onBack}><Icon name="back" />返回工作页</button><h2>操作记录</h2><p className="muted">此浏览器提交的请求</p></header>
      <div className="collection-tools segmented"><button aria-pressed={visible === model.receipts} onClick={() => setFilter(false)}>全部 {model.receipts.length}</button><button aria-pressed={visible === attention} onClick={() => setFilter(true)}>待处理 {attention.length}</button></div>
      <div className="collection-list">{visible.map(receipt => <button className="collection-row" key={receipt.id} aria-pressed={receipt.id === selected?.id} onClick={() => onSelect(receipt.id)}><span className="collection-row-title"><strong>{kindLabels[receipt.kind]}</strong><Timestamp value={receipt.at} /></span><span className="collection-row-title"><span className="muted ellipsis">{receipt.label}</span><State value={statusOf(receipt)} /></span></button>)}
      {!visible.length && <EmptyState icon="operations" title={attentionOnly ? '没有待处理操作' : '还没有操作记录'} />}</div>
    </aside>
    <div className="operations-scroll">{model.operationError && <p className="error" role="alert">{model.operationError}</p>}{selected ? <OperationDetail key={selected.id} receipt={selected} model={model} onAgent={onAgent} /> : <EmptyState icon="operations" title="暂无回执">启动或发送消息后，回执会保留在这里。</EmptyState>}</div>
  </section>;
}

/** @aiPurpose Point to the exact journaled operation in its originating workspace. @aiUsage Pass the saved receipt, never infer success from the click. */
export function ReceiptNotice({ receipt, onOpen, onClose }: { receipt: Receipt; onOpen: (id: string) => void; onClose: () => void }) {
  const status = statusOf(receipt);
  const tone = status === 'Succeeded' ? 'ready' : status === 'Failed' || status === 'rejected' ? 'bad' : status === 'Unknown' || receipt.issue ? 'warn' : 'active';
  const icon: Glyph = tone === 'ready' ? 'check' : tone === 'bad' || tone === 'warn' ? 'warning' : 'clock';
  return <div className="context-receipt" data-tone={tone}><div className="receipt-summary" role="status"><span key={icon} className="receipt-mark"><Icon name={icon} /></span><strong>{kindLabels[receipt.kind]}</strong><State value={status} /></div><button className="quiet icon-button receipt-link" onClick={() => onOpen(receipt.id)}>查看回执<Icon name="chevron" /></button><button className="quiet icon-control" onClick={onClose} aria-label="关闭提示" title="关闭提示"><Icon name="close" /></button></div>;
}
