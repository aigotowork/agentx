import { stringifyJson } from '@agentx/client';
/** Compact actions remain beside their office, post, or exact Agent subject. */
import { useAgentCommands } from '../interactions/AgentCommands';
import { useState } from 'react';
import type { AgentResource, Post } from '../offline/model';
import { operationIssue, type LiveSpace, type Receipt } from './controller';

/** @aiPurpose Display live connection, paging, and durable original-operation recovery.
 * @aiUsage Mount in the existing header; recovery only queries receipts and never repeats effects.
 */
export function LiveStatus({ live, onReconnect }: { live: LiveSpace; onReconnect?: () => void }) {
  const descriptor = live.descriptor;
  const scopedReceipts = live.receipts.filter(receipt => live.ownsReceipt(receipt));
  const pending = scopedReceipts.filter(receipt => !receipt.operation || ['Accepted', 'Running', 'Unknown'].includes(receipt.operation.status)).length;
  const unknown = scopedReceipts.filter(receipt => receipt.operation?.status === 'Unknown').length;
  const issues = [live.error && `数据连接：${live.error}`, live.roleError && `岗位目录：${live.roleError}`, live.activityError && `办公室动态：${live.activityError}`, live.actionError && `操作：${live.actionError}`, unknown > 0 && `${unknown} 项操作结果未确定`].filter((issue): issue is string => typeof issue === 'string' && issue.length > 0);
  const latestSubjects = [...new Map([...scopedReceipts].reverse().map(receipt => [`${receipt.kind}:${receipt.key}`, receipt])).values()];
  const problemReceipts = latestSubjects.filter(receipt => operationIssue(receipt));
  const operationFeedback = problemReceipts.map(receipt => <OperationFeedback key={receipt.id} live={live} receipt={receipt} />);
  return <div className="live-status">{(live.connection === 'failed' || live.connection === 'reset')
    ? <button type="button" onClick={live.connection === 'reset' ? onReconnect : () => void live.refresh()}>{live.connection === 'reset' ? '重新连接' : '重试连接'}</button>
    : <button type="button" className="live-toolbar-refresh" onClick={live.refreshData} aria-label="刷新真实数据" title="刷新真实数据">↻</button>}{live.more && <button type="button" onClick={live.loadMore}>更多成员</button>}{(issues.length > 0 || operationFeedback.length > 0) && <span className="live-error" role="status">{issues.join(' · ')}{operationFeedback}</span>}<details className="live-receipts"><summary aria-label={issues.length ? "连接异常 · 回执" : "连接 · 回执"} data-issue={!!issues.length || undefined}>{issues.length ? "连接异常" : <><span className="live-receipt-context">连接 · </span>回执</>} {pending || ''} ⓘ</summary><div>{live.connection === 'pinned' && <button type="button" className="live-receipt-refresh" onClick={live.refreshData}>刷新真实数据</button>}{issues.map(issue => <span key={issue} className="live-error" role="status">{issue}</span>)}<code>{descriptor?.scope || '连接中'}</code><small>{descriptor?.generation}</small>{live.receipts.map(receipt => <section key={receipt.id}><strong>{receipt.kind} · {receipt.operation?.status ?? '未确认'}</strong><code>{receipt.id}</code><small>{receipt.scope} · {receipt.body.generation}</small>{receipt.issue && <p role="status">{receipt.issue}</p>}{receipt.operation?.error && <p role="status">{receipt.operation.error}</p>}{receipt.operation?.result && <pre>{stringifyJson(receipt.operation.result)}</pre>}<button type="button" disabled={!live.canLookup(receipt)} onClick={() => void live.lookup(receipt)}>查询原回执</button></section>)}{!live.receipts.length && <span>暂无操作</span>}<a href="./NOTICE.md" target="_blank" rel="noreferrer">素材与许可</a></div></details></div>;
}

/** Keep native errors and original-receipt recovery beside their operation subject. */
function OperationFeedback({ live, receipt }: { live: LiveSpace; receipt: Receipt }) {
  const issue = operationIssue(receipt);
  if (!issue) return null;
  const canLookup = live.canLookup(receipt);
  return <span className="live-error" role="status"><span>{receipt.kind} · {issue}</span><button type="button" disabled={!canLookup} title={receipt.id} onClick={() => void live.lookup(receipt)}>查询原回执</button></span>;
}

/** @aiPurpose Offer native capabilities for one exact current or stopped Agent.
 * @aiUsage Mount inside the existing reader; target is captured by exact id, never display name.
 */
export function AgentActions({ live, agent }: { live: LiveSpace; agent: AgentResource }) {
  const native = live.capabilities(agent.id), blocked = live.blocked(agent.id);
  const commands = useAgentCommands();
  if (!native) return null;
  return <div className="live-agent-actions">{native?.can_stop_participation && <button type="button" disabled={blocked} onClick={() => commands?.request("stop-participation", agent)}>停止参与</button>}{!native.can_fork && <small>原生工具或会话不支持 Fork 新 Agent</small>}{native?.can_stop && <button type="button" disabled={blocked} title="终止进程 · Ctrl / ⌘ + Shift + K" onClick={() => commands?.request("kill", agent)}>终止进程</button>}{native?.can_resume && <button type="button" disabled={blocked} onClick={() => void live.act(() => live.target('resume', agent.id))}>恢复</button>}</div>;
}

/** @aiPurpose Launch the selected effective Role deliberately from its existing post detail.
 * @aiUsage Pass a current supplied post; the native service resolves the actual launch defaults.
 */
export function PostLaunch({ live, post }: { live: LiveSpace; post: Post }) {
  const [prompt, setPrompt] = useState(''), [title, setTitle] = useState(''), [directory, setDirectory] = useState(post.office === '$global' ? live.descriptor?.project ?? '' : post.office), [error, setError] = useState('');
  const receipt = live.latestLaunch(post.id);
  const submit = async (event: React.FormEvent) => { event.preventDefault(); const previous = live.latestLaunch(post.id); setError(''); try { await live.launch(post, prompt, title, directory); } catch (failure) { const latest = live.latestLaunch(post.id); if (!latest || latest.id === previous?.id || !operationIssue(latest)) setError(failure instanceof Error ? failure.message : String(failure)); } };
  return <form className="live-post-launch" onSubmit={event => void submit(event)}><label>工作目录<input aria-label="岗位工作目录" value={directory} onChange={event => setDirectory(event.target.value)} /></label><label>职责标题<input aria-label="岗位启动职责标题" value={title} maxLength={160} onChange={event => setTitle(event.target.value)} /></label><label>启动任务<textarea aria-label="岗位启动任务" value={prompt} onChange={event => setPrompt(event.target.value)} /></label>{error && <p className="live-error" role="alert">{error}</p>}{receipt && <OperationFeedback live={live} receipt={receipt} />}<button type="submit" disabled={live.blocked(post.id) || !directory.trim()}>按此岗位启动</button></form>;
}

/** @aiPurpose Preview inherited directory routing and send only after a deliberate resolved-entry action.
 * @aiUsage Keep this disclosure within office context; preview is read-only and never creates an Agent.
 */
export function DirectoryContact({ live, office }: { live: LiveSpace; office: string }) {
  const [directory, setDirectory] = useState(office.startsWith('/') ? office : '');
  const [text, setText] = useState('');
  const entry = live.entry;
  const matches = !!entry && (directory === entry.directory || directory === live.entryInput);
  const available = matches && ['bound', 'adoptable', 'absent'].includes(entry.endpoint.status);
  const canSend = available && !!text.trim() && !live.blocked(`directory:${entry.directory}`);
  return <details className="live-directory-contact"><summary>联系目录</summary><div><form onSubmit={event => { event.preventDefault(); void live.preview(directory); }}><label>目标目录<input aria-label="联系目录路径" value={directory} onChange={event => setDirectory(event.target.value)} /></label><button type="submit" disabled={!directory.trim()}>预览入口</button></form>{live.entryError && <p role="status">{live.entryError}</p>}{entry && <><dl><dt>目标</dt><dd><code>{entry.directory}</code></dd><dt>解析办公室</dt><dd>{entry.resolved_workspace ?? '全局'}</dd><dt>岗位</dt><dd>{entry.role.name} · {entry.role.path}</dd><dt>入口</dt><dd>{entry.endpoint.status} · {entry.endpoint.participant?.name ?? '未绑定'}{entry.endpoint.reason && ` · ${entry.endpoint.reason}`}</dd></dl><label>消息<textarea aria-label="目录联系消息" value={text} onChange={event => setText(event.target.value)} /></label><button type="button" disabled={!canSend} onClick={() => void live.act(() => live.directorySend(text))}>发送到此入口</button></>}</div></details>;
}
