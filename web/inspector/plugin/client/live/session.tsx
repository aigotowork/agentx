import { stringifyJson } from '@agentx/client';
import { useEffect, useRef, useState } from 'react';
import { useInfiniteQuery, useQueryClient } from '@tanstack/react-query';
import type { PluginTheme } from '../../../host/contracts';
import type { AgentView, Descriptor, TranscriptPage } from '../../../host/live/client';
import { messages } from '../../../host/live/client';
import { datasetKey, referenceKey, selectionKey } from '../../../host/live/model';
import { blocksTarget, loadCommands, reconcile, runCommand, saveCommands, type CommandEntry } from '../../../host/live/operations';
import { Button } from '../../../uiux/systems/inspector/inspector.native';

const browserStorage = { getItem: (key: string) => localStorage.getItem(key), setItem: (key: string, value: string) => localStorage.setItem(key, value) };

/** @aiPurpose Inspect one exact native current/stopped reference, bounded messages/transcript and durable operation receipts.
 * @aiUsage Key by selectionKey; dataset resets must unmount this block before another target becomes actionable.
 * @aiBehavior Persists request identity before POST; unknown commands can only reconcile their original receipt. */
export function NativeSession({ theme, dataset, row }: { theme: PluginTheme; dataset: Descriptor; row: AgentView }) {
  const key = selectionKey(dataset, row.agent);
  const draftKey = `agentx-inspector:draft:${key}`;
  const [text, setText] = useState(() => { try { return sessionStorage.getItem(draftKey) ?? ''; } catch { return ''; } });
  const [entries, setEntries] = useState(() => loadCommands(dataset, browserStorage));
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  const alive = useRef(true);
  const cache = useQueryClient();
  useEffect(() => () => { alive.current = false; }, []);
  const relevant = entries.filter(entry => referenceKey(entry.agent) === referenceKey(row.agent));
  const blocked = working || relevant.some(entry => blocksTarget(entry, row.agent));
  const thread = useInfiniteQuery({
    queryKey: ['native-messages', key], initialPageParam: undefined as string | undefined,
    queryFn: ({ pageParam, signal }) => messages(dataset.generation, row.agent, pageParam, signal),
    getNextPageParam: page => page.has_more ? page.next_after : undefined,
  });
  const update = (entry: CommandEntry) => {
    const current = loadCommands(dataset, browserStorage);
    const next = [entry, ...current.filter(item => item.requestId !== entry.requestId)];
    saveCommands(dataset, next, browserStorage);
    if (alive.current) setEntries(next);
  };
  const refresh = async (entry: CommandEntry) => {
    setError(null);
    try { update(await reconcile(dataset, entry)); } catch (failure) { if (alive.current) setError(String(failure)); }
  };
  useEffect(() => {
    const pending = relevant.filter(entry => ['Accepted', 'Running'].includes(entry.operation?.status ?? ''));
    if (!pending.length) return;
    const timeout = setTimeout(() => { void Promise.all(pending.map(refresh)); }, 1500);
    return () => clearTimeout(timeout);
  }, [entries]);
  const act = async (action: 'messages' | 'stop' | 'resume' | 'transcript') => {
    if (blocked) return;
    setWorking(true); setError(null);
    try {
      const result = await runCommand(dataset, row.agent, action, action === 'messages' ? text : undefined, update);
      if (alive.current && result.operation?.status === 'Succeeded' && action === 'messages') { setText(''); try { sessionStorage.removeItem(draftKey); } catch {} }
      void cache.invalidateQueries({ queryKey: ['native-directory', datasetKey(dataset)] });
      void thread.refetch();
    } catch (failure) { if (alive.current) setError(String(failure)); }
    finally { if (alive.current) setWorking(false); }
  };
  const transcript = relevant.map(entry => entry.operation?.result).find(result => result && 'Transcript' in result) as { Transcript: TranscriptPage } | undefined;
  const transcriptValue = transcript?.Transcript && referenceKey(transcript.Transcript.agent) === referenceKey(row.agent) ? transcript.Transcript : undefined;
  const changeText = (value: string) => { setText(value); try { sessionStorage.setItem(draftKey, value); } catch {} };
  const messageRows = thread.data?.pages.flatMap(page => page.messages) ?? [];
  return <>
    <h2>{row.agent.name} <span className="native-muted">{row.status}</span></h2>
    <dl className="native-facts"><dt>工具</dt><dd>{row.tool}</dd><dt>目录</dt><dd>{row.directory || '未知'}</dd><dt>工作区 / Role</dt><dd>{row.collaboration.workspace ?? '全局 / 未知'} · {row.collaboration.role?.name ?? '—'} · {row.collaboration.status}</dd><dt>实例</dt><dd>{row.agent.incarnation ?? '—'}</dd><dt>停止记录</dt><dd>{row.agent.stopped_event ?? '—'}</dd><dt>Provider 会话</dt><dd>{row.session_id ?? '未知'}</dd><dt>状态证据</dt><dd>{row.detail || '未提供'}</dd></dl>
    <div className="native-actions"><Button theme={theme} size="sm" label="读取最近模型对话" disabled={!row.has_transcript || blocked} onPress={() => void act('transcript')} /><Button theme={theme} size="sm" label="停止" variant="danger" disabled={!row.can_stop || blocked} onPress={() => void act('stop')} /><Button theme={theme} size="sm" label="恢复会话" disabled={!row.can_resume || blocked} onPress={() => void act('resume')} /></div>
    {row.can_send && <section aria-label="即时消息"><textarea aria-label="即时消息内容" value={text} onChange={event => changeText(event.target.value)} maxLength={dataset.max_text} placeholder="发送即时消息…" /><div className="native-actions"><Button theme={theme} label="发送" size="sm" disabled={blocked || !text.trim()} onPress={() => void act('messages')} /></div></section>}
    {error && <p className="native-error" role="alert">{error}</p>}
    <ReceiptList theme={theme} entries={relevant} onRefresh={entry => void refresh(entry)} />
    {transcriptValue && <section aria-label="模型对话证据"><h3>最近模型对话 <span className="native-muted">有界样本</span></h3>{!transcriptValue.available && <p>{transcriptValue.reason ?? '内容不可用'}</p>}{transcriptValue.exchanges.map(exchange => <details key={exchange.position}><summary>#{exchange.position} · {exchange.at || '时间未知'} · {exchange.user.slice(0, 90)}</summary><pre>{exchange.user}</pre><pre>{exchange.assistant || exchange.action || '未提供模型文本'}</pre>{exchange.files.length > 0 && <p>{exchange.files.join(' · ')}</p>}</details>)}</section>}
    <section aria-label="AgentX 通信"><h3>AgentX 消息</h3>{thread.error && <p role="alert" className="native-error">{String(thread.error)}</p>}{thread.isPending && <p role="status">正在读取消息…</p>}{!thread.isPending && !messageRows.length && !thread.error && <p className="native-muted">当前范围没有消息</p>}{messageRows.map(message => <details key={message.id}><summary>{message.at} · {message.from} → {message.recipients.join(', ')}</summary><pre>{message.text}</pre><p className="native-muted">{message.intent ?? '无意图标记'} · #{message.id}</p></details>)}{thread.hasNextPage && <Button theme={theme} label="继续读取消息" size="sm" busy={thread.isFetchingNextPage} onPress={() => void thread.fetchNextPage()} />}</section>
  </>;
}

function ReceiptList({ theme, entries, onRefresh }: { theme: PluginTheme; entries: CommandEntry[]; onRefresh: (entry: CommandEntry) => void }) {
  if (!entries.length) return null;
  return <section aria-label="操作回执"><h3>操作回执</h3>{entries.map(entry => <details key={entry.requestId}><summary>{entry.action} · {entry.operation?.status ?? (entry.refused ? '请求被拒绝' : '结果未知')} · {entry.requestId}</summary>{entry.error && <p className="native-error">{entry.error}</p>}{entry.operation?.error && <p className="native-error">{entry.operation.error}</p>}<p className="native-muted">{entry.operation?.status === 'Succeeded' && entry.action === 'messages' ? '原生消息提交成功；模型执行结果另看对话。' : 'Accepted / Running 表示尚在执行；Unknown 只能核对原回执。'}</p><Button theme={theme} size="sm" label="查询原回执" onPress={() => onRefresh(entry)} />{entry.operation?.result && <pre>{stringifyJson(entry.operation.result)}</pre>}</details>)}</section>;
}
