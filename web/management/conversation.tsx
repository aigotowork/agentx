import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { MessageView } from './api/native';
import type { Management } from './use-management';
import { messageOf, readStored, refKey } from './model';
import { EmptyState, Icon, State, Timestamp } from './presentation';

const intentLabels: Record<string, string> = { request: '请求', inform: '告知', ack: '确认' };
const dayFormat = new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric', weekday: 'long', year: 'numeric' });
const dayKey = (value: string): string => {
  const date = new Date(value);
  return Number.isNaN(date.valueOf()) ? '日期未知' : dayFormat.format(date);
};
const prefersQuiet = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

function MessageRow({ message, related, agentName, onReply, onJump }: { message: MessageView; related?: MessageView; agentName?: string; onReply: (message: MessageView) => void; onJump: (id: string) => void }) {
  const name = message.from || '未知发送者';
  const intent = message.intent && (intentLabels[message.intent] ?? message.intent);
  return <article className="message-row" tabIndex={-1} id={`message-${message.id}`} data-speaker={message.from === agentName ? 'agent' : 'other'}>
    <span className="message-avatar" aria-hidden="true">{name.replace(/^@/, '').slice(0, 2)}</span>
    <div className="message-entry">
      <header className="message-heading"><strong>{name}</strong><span className="message-recipients">发给 {message.recipients.join('、')}</span><Timestamp value={message.at} timeOnly /><button className="quiet icon-button reply-action" onClick={() => onReply(message)} aria-label={`回复 ${name} 的消息`}><Icon name="reply" /><span>回复</span></button></header>
      {related && <button className="message-quote" onClick={() => onJump(related.id)} title="定位原消息"><Icon name="reply" /><strong>{related.from}</strong><span className="ellipsis">{related.text}</span></button>}
      {message.reply_to && !related && <span className="message-quote"><Icon name="reply" />回复 <code>{message.reply_to}</code></span>}
      <p className="message-text">{message.text}</p>
      {(intent || message.thread) && <footer>{intent && <span className="message-intent" title={message.intent ?? undefined}>{intent}</span>}{message.thread && <span className="message-thread">{message.thread}</span>}</footer>}
    </div>
  </article>;
}
interface MessageFields { text: string; intent: string; reply_to: string; thread: string }
interface Draft extends MessageFields { submission?: { id: string; fields: MessageFields } }
const emptyDraft: Draft = { text: '', intent: 'request', reply_to: '', thread: '' };
const sameMessage = (a: MessageFields, b: MessageFields) => a.text === b.text && a.intent === b.intent && a.reply_to === b.reply_to && a.thread === b.thread;

/**
 * @aiPurpose Read messages and compose work against one exact Agent reference.
 * @aiUsage Key this block by generation and exact reference to isolate drafts and reply targets.
 * @aiBehavior Drafts persist per reference; only a confirmed matching send clears its text.
 */
export function MessageWorkspace({ model, onOperation, onOpenOperation }: { model: Management; onOperation: (id: string) => void; onOpenOperation: (id: string) => void }) {
  const storageKey = `agentx.management.draft.${model.descriptor?.generation}.${model.selected ? refKey(model.selected) : ''}${model.storageSuffix}`;
  const [draft, setDraft] = useState<Draft>(() => readStored(storageKey, emptyDraft));
  const [error, setError] = useState('');
  const [storageIssue, setStorageIssue] = useState(false);
  const [options, setOptions] = useState(false);
  const [busy, setBusy] = useState(false);
  const [readingOlder, setReadingOlder] = useState(false);
  const [unseen, setUnseen] = useState(0);
  const input = useRef<HTMLTextAreaElement>(null);
  const log = useRef<HTMLDivElement>(null);
  const following = useRef(true);
  const previousCount = useRef(0);
  const byId = useMemo(() => new Map(model.messages.map(message => [message.id, message])), [model.messages]);
  const groups = useMemo(() => {
    const days: { label: string; rows: MessageView[] }[] = [];
    for (const message of model.messages) {
      const label = dayKey(message.at);
      const current = days.at(-1);
      if (current?.label === label) current.rows.push(message);
      else days.push({ label, rows: [message] });
    }
    return days;
  }, [model.messages]);
  const maxText = model.descriptor?.max_text ?? 65536;
  const byteLength = new TextEncoder().encode(draft.text).length;
  const overLimit = byteLength > maxText;
  const replyMessage = draft.reply_to ? byId.get(draft.reply_to) : undefined;
  const submitted = draft.submission;
  const sendReceipt = submitted ? model.receipts.find(item => item.id === submitted.id) : undefined;
  const unknownDuplicate = sendReceipt?.operation?.status === 'Unknown' && !!submitted && sameMessage(draft, submitted.fields);
  const canSend = !!model.detail?.can_send && !model.targetPending && !unknownDuplicate && !busy && !!draft.text.trim() && !overLimit;
  useEffect(() => {
    try { localStorage.setItem(storageKey, JSON.stringify(draft)); setStorageIssue(false); } catch { setStorageIssue(true); }
  }, [storageKey, draft]);
  useLayoutEffect(() => {
    const count = model.messages.length;
    const added = count - previousCount.current;
    if (added > 0) {
      if (following.current && log.current?.clientHeight) log.current.scrollTop = log.current.scrollHeight;
      else if (!following.current) setUnseen(old => old + added);
    }
    previousCount.current = count;
  }, [model.messages.length]);
  useEffect(() => {
    const element = log.current;
    if (!element) return;
    const observer = new ResizeObserver(() => {
      if (following.current && element.clientHeight) element.scrollTop = element.scrollHeight;
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (!submitted || sendReceipt?.operation?.status !== 'Succeeded') return;
    setDraft(previous => previous.submission?.id !== submitted.id ? previous : {
      ...previous, submission: undefined, ...(sameMessage(previous, submitted.fields) ? { text: '', reply_to: '' } : {}),
    });
  }, [submitted, sendReceipt]);
  async function send() {
    if (!canSend) return;
    setBusy(true); setError('');
    try {
      const fields = { text: draft.text, intent: draft.intent, reply_to: draft.reply_to, thread: draft.thread };
      await model.submit('send', { attachments: [], text: fields.text, intent: fields.intent || null, reply_to: fields.reply_to || null, thread: fields.thread || null }, id => {
        setDraft(previous => ({ ...previous, submission: { id, fields } }));
        onOperation(id);
      });
    } catch (error) { setError(messageOf(error)); }
    finally { setBusy(false); }
  }
  function reply(message: MessageView) { setDraft(previous => ({ ...previous, reply_to: message.id, thread: message.thread ?? '' })); input.current?.focus({ preventScroll: true }); }
  function removeReply() { setDraft(previous => ({ ...previous, reply_to: '' })); input.current?.focus({ preventScroll: true }); }
  function jumpToMessage(id: string) {
    const article = document.getElementById(`message-${id}`);
    if (!article) return;
    article.focus({ preventScroll: true });
    article.scrollIntoView({ block: 'nearest', behavior: prefersQuiet() ? 'instant' : 'smooth' });
  }
  function onLogScroll() {
    const element = log.current;
    if (!element?.clientHeight) return;
    const atBottom = element.scrollHeight - element.clientHeight - element.scrollTop <= 48;
    following.current = atBottom;
    setReadingOlder(!atBottom);
    if (atBottom) setUnseen(0);
  }
  function latest() {
    const element = log.current;
    if (!element) return;
    element.scrollTo({ top: element.scrollHeight, behavior: prefersQuiet() ? 'instant' : 'smooth' });
    element.focus({ preventScroll: true });
  }
  return <section className="message-workspace" aria-label="消息">
    <div className="conversation-viewport">
      <div ref={log} className="conversation-scroll" role="log" tabIndex={0} aria-label="Agent 消息" onScroll={onLogScroll}>
        {model.messageError && <p className="error" role="alert">{model.messageError}</p>}
        {!model.messages.length && !model.messageError && <EmptyState icon="message" title={!model.messagesReady ? '读取消息…' : '还没有消息'}>{model.messagesReady && model.detail?.can_send && '可以在下方发送第一条消息。'}</EmptyState>}
        {groups.map(group => <section className="message-day" key={group.rows[0].id} aria-label={group.label}><div className="day-marker"><span>{group.label}</span></div>{group.rows.map(message => <MessageRow key={message.id} message={message} related={message.reply_to ? byId.get(message.reply_to) : undefined} agentName={model.selected?.name} onReply={reply} onJump={jumpToMessage} />)}</section>)}
        {model.messageMore && <p className="muted">正在继续读取消息…</p>}
      </div>
      {readingOlder && <button type="button" className="latest-message icon-button" onClick={latest}>{unseen > 0 ? `${unseen} 条新消息` : '回到最新'}<Icon name="down" /></button>}
    </div>
    <form className="composer" onSubmit={event => { event.preventDefault(); void send(); }}>
      <div className="composer-dock">
        <div className="composer-editing">
          {draft.reply_to && <div className="reply-preview"><Icon name="reply" /><div><strong>{replyMessage ? `回复 @${replyMessage.from}` : '回复指定消息'}</strong><p className="ellipsis">{replyMessage?.text ?? draft.reply_to}</p></div><button type="button" className="quiet icon-control" onClick={removeReply} aria-label="移除引用" title="移除引用"><Icon name="close" /></button></div>}
          <textarea id="agent-message" ref={input} aria-label="消息内容" aria-invalid={overLimit || undefined} aria-describedby={overLimit ? 'message-size-error' : undefined} placeholder={model.detail?.can_send ? `发给 @${model.selected?.name}…` : '当前不可发送，仍可保留草稿'} value={draft.text} onChange={event => setDraft(previous => ({ ...previous, text: event.target.value }))} onKeyDown={event => { if (event.key === 'Enter' && !event.nativeEvent.isComposing && (event.ctrlKey || event.metaKey)) { event.preventDefault(); void send(); } }} rows={2} />
          {options && <div className="message-options"><label>回复消息 ID<input value={draft.reply_to} onChange={event => setDraft(previous => ({ ...previous, reply_to: event.target.value }))} /></label><label>话题<input value={draft.thread} onChange={event => setDraft(previous => ({ ...previous, thread: event.target.value }))} /></label></div>}
          {overLimit && <p id="message-size-error" className="field-error" role="alert"><Icon name="warning" />已用 {byteLength.toLocaleString('zh-CN')} / {maxText.toLocaleString('zh-CN')} 字节</p>}
          {error && <p className="field-error" role="alert"><Icon name="warning" />{error}</p>}
          {sendReceipt?.issue && <p className="receipt-note" role="status">{sendReceipt.issue}</p>}
          {unknownDuplicate && sendReceipt && <p className="receipt-note">这条消息的结果未知。<button type="button" className="quiet" onClick={() => onOpenOperation(sendReceipt.id)}>查看原请求</button></p>}
        </div>
        <div className="composer-toolbar"><select className="composer-intent" aria-label="消息意图" title="消息意图" value={draft.intent} onChange={event => setDraft(previous => ({ ...previous, intent: event.target.value }))}><option value="request">请求</option><option value="inform">告知</option><option value="ack">确认</option></select><button type="button" className="quiet icon-control composer-options" aria-label="消息选项" title="消息选项" aria-expanded={options} onClick={() => setOptions(value => !value)}><Icon name="sliders" /></button>
          {draft.text && <span className="draft-state">{storageIssue ? '草稿仅存于当前页' : '草稿已保留'}</span>}
          <span className="composer-shortcut" aria-hidden="true"><kbd>{/Mac/.test(navigator.platform) ? '⌘' : 'Ctrl'}</kbd><kbd>↵</kbd></span><button className="primary send-button icon-button" title="Ctrl / ⌘ + Enter" disabled={!canSend}>{busy ? '提交中…' : '发送'}<Icon name={busy ? 'clock' : 'send'} /></button></div>
      </div>
    </form>
  </section>;
}

/**
 * @aiPurpose Read a bounded transcript snapshot produced by an asynchronous native job.
 * @aiUsage The model owns operation polling and keeps snapshots across tab switches.
 */
export function TranscriptWorkspace({ model, onOperation, onOpenOperation }: { model: Management; onOperation: (id: string) => void; onOpenOperation: (id: string) => void }) {
  const [error, setError] = useState('');
  const receipt = model.transcriptReceipt;
  const page = model.transcript;
  async function request() { setError(''); try { await model.submit('transcript', undefined, onOperation); } catch (error) { setError(messageOf(error)); } }
  const status = receipt?.operation?.status;
  const pending = !!receipt && (!status || status === 'Accepted' || status === 'Running');
  return <section className="transcript-workspace" aria-label="模型记录"><div className="transcript-toolbar"><span>模型对话快照 {receipt && <State value={status ?? 'unconfirmed'} />}</span><button className="icon-button" disabled={!model.detail?.has_transcript || pending || status === 'Unknown'} onClick={() => void request()}><Icon name={page ? 'refresh' : 'transcript'} />{page ? '刷新快照' : '读取记录'}</button></div>
    {(error || receipt?.issue || receipt?.operation?.error || status === 'Unknown') && <div role="alert" className="error">{error || receipt?.issue || receipt?.operation?.error || '结果未知。可查询原请求。'}{receipt && <button className="quiet" onClick={() => onOpenOperation(receipt.id)}>查看操作</button>}</div>}
    {pending && <p className="empty" role="status">正在读取快照…</p>}
    {!receipt && <EmptyState icon="transcript" title={model.detail?.has_transcript ? '尚未读取模型记录' : '暂无可读取的模型记录'}>{model.detail?.has_transcript && '可读取此执行实例的模型对话快照。'}</EmptyState>}
    {page && !page.available && <p className="empty">{page.reason || '模型记录不可用'}</p>}
    {page?.available && !page.exchanges.length && <p className="empty">记录为空</p>}
    <div className="transcript-scroll">{page?.exchanges.map(exchange => <article className="exchange" key={exchange.position}><header><strong>#{exchange.position}</strong><time>{exchange.at}</time></header><h3>用户</h3><p className="message-text">{exchange.user || '—'}</p><h3>Agent</h3><p className="message-text">{exchange.assistant || '—'}</p>{(exchange.action || exchange.files.length > 0) && <details><summary>操作与文件</summary><p className="message-text">{exchange.action}</p><ul>{exchange.files.map(file => <li key={file}>{file}</li>)}</ul></details>}</article>)}</div>
  </section>;
}
