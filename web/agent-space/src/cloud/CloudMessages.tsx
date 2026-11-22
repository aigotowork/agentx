import { useCallback, useState } from 'react';
import type { CloudScopeTypes } from '@agentx/client';
import { useRead } from '../../../management/deployment/model';
import { Markdown } from '../conversation/Markdown';
import { cloudProblem, cloudTime, type Session } from './model';
import { exactLabel, sameAgent, type CloudAgent } from './route';

type Message = CloudScopeTypes['MessageView'];
type MessagePage = CloudScopeTypes['MessageHistoryPage'];
const intentLabel: Record<Message['intent'], string> = { request: '请求', inform: '通知', ack: '确认' };

/**
 * @aiPurpose Read actual cloud mailbox rows for the exact selected Agent; accepted rows do not prove delivery.
 * @aiUsage Key by exact selection and scope session. Render only with a connected scope bearer.
 */
export function CloudMessages({ session, agent }: { session: Session & { access: 'scope' }; agent: CloudAgent }) {
  const [before, setBefore] = useState<string>();
  const [older, setOlder] = useState<Message[]>([]);
  const load = useCallback(async (signal: AbortSignal): Promise<MessagePage> => {
    return session.client.listMessages({
      generation: session.generation, host: agent.host, name: agent.name, incarnation: agent.incarnation, before, limit: 20,
    }, { signal });
  }, [session, agent.host, agent.name, agent.incarnation, before]);
  const { read, retry } = useRead(load);
  const page = read.kind === 'ready' ? read.data : null;
  const messages = page ? [...older, ...page.items] : older;
  const items = messages.map(message => <CloudMessage key={message.id} message={message} sent={sameAgent(message.from, agent)} />);
  function refresh() {
    setBefore(undefined);
    setOlder([]);
    retry();
  }
  function showOlder() {
    if (!page?.next || page.next === before) return;
    setOlder(messages);
    setBefore(page.next);
  }
  return <>
    <div className="cloud-read-bar">
      <h3>云端消息</h3><span title="记录的是云端接受的消息；无法以此判断是否送达。">只读 · 最新优先</span>
      <button type="button" onClick={refresh} disabled={read.kind === 'loading'}>刷新消息</button>
    </div>
    {messages.length > 0 && <ol className="cloud-messages" aria-label="云端消息">{items}</ol>}
    {read.kind === 'loading' && <p className="cloud-note" role="status">正在读取云端消息…</p>}
    {read.kind === 'error' && <div className="cloud-empty" role="alert">{cloudProblem(read.error, '无法读取云端消息。')} <button type="button" onClick={retry}>重试</button></div>}
    {page && !messages.length && <p className="cloud-empty">此实例没有云端消息。</p>}
    {page?.next && page.next !== before && <button type="button" className="cloud-older" onClick={showOlder}>加载较早消息</button>}
  </>;
}

function CloudMessage({ message, sent }: { message: Message; sent: boolean }) {
  const counterpart = sent ? message.to : message.from;
  const at = cloudTime(message.created_at);
  return <li className="cloud-message">
    <header><span className="cloud-direction">{sent ? '发往' : '来自'}</span><strong title={exactLabel(counterpart)}>@{counterpart.name}</strong><span>{intentLabel[message.intent]}</span>{at && <time dateTime={at.toISOString()}>{at.toLocaleString()}</time>}</header>
    <Markdown text={message.body} />
    <details><summary>消息详情</summary><dl className="cloud-facts">
      <div><dt>发送方</dt><dd><code>{exactLabel(message.from)}</code></dd></div>
      <div><dt>接收方</dt><dd><code>{exactLabel(message.to)}</code></dd></div>
      <div><dt>消息 ID</dt><dd><code>{message.id}</code></dd></div>
      {message.thread && <div><dt>线程</dt><dd><code>{message.thread}</code></dd></div>}
      {message.reply_to && <div><dt>回复</dt><dd><code>{message.reply_to}</code></dd></div>}
    </dl></details>
  </li>;
}
