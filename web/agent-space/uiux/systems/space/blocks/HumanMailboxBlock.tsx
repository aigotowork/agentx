/** @aiPurpose Controlled exact-sender mailbox grouping and deliberate displayed-message handling. */
import { useRef, useState, type ReactNode } from "react";
import { useOverlay } from "../components/SpaceInteractions";
import type { AgentRef } from "../../../../../management/api/native";
interface HumanMessage { id: string; at: string; from: string; text: string; handled_at?: string | null; sender?: AgentRef | null }
export interface MailboxFeed {
  messages: readonly HumanMessage[]; hasMore: boolean; error?: string; actionError?: string;
  unhandledCount?: number | bigint; pending?: boolean; older?: boolean; busy?: boolean;
  openSender?(reference: AgentRef): Promise<{ id: string }>;
  handle?(ids: readonly string[]): void; history?(pending: boolean): void; page?(older: boolean): void;
}

/** @aiPurpose Read the scope-human mailbox and deliberately mark displayed messages handled.
 * @aiUsage Supply native confirmed state/actions and an exact number or bigint count; opening, paging and closing never handle messages.
 * @aiBehavior Pending count covers all history; handling retains history and excludes unseen IDs.
 */
export function HumanMailboxBlock({ feed, onClose, onNavigate, renderText }: { feed?: MailboxFeed; onClose(): void; onNavigate?(id: string): void; renderText(text: string): ReactNode }) {
  const pendingIds = feed?.messages.filter(message => !message.handled_at).map(message => message.id) ?? [];
  const disabled = !!feed?.busy || !!feed?.error;
  const count = feed?.unhandledCount;
  const root = useRef<HTMLElement>(null), close = useOverlay(true, onClose, 40, root);
  const [navigationError, setNavigationError] = useState(""), [opening, setOpening] = useState(false);
  const groups = new Map<string, HumanMessage[]>();
  for (const message of feed?.messages ?? []) {
    const key = message.sender ? JSON.stringify(message.sender) : `legacy:${message.from}`;
    groups.set(key, [...(groups.get(key) ?? []), message]);
  }
  async function navigate(message: HumanMessage) {
    if (!message.sender || !feed?.openSender || !onNavigate || opening) return;
    setOpening(true); setNavigationError("");
    try { const resource = await feed.openSender(message.sender); onNavigate(resource.id); onClose(); }
    catch (error) { setNavigationError(error instanceof Error ? error.message : String(error)); }
    finally { setOpening(false); }
  }
  return <section ref={root} className="scene-human-reader" aria-label="发给 @bigboss">
    <header><strong>@bigboss 信箱{count !== undefined ? ` · ${count} 待处理` : ""}</strong><button type="button" aria-label="关闭人类消息" onClick={close}>×</button></header>
    {feed?.history && <nav className="human-mailbox-actions" aria-label="人类消息筛选">
      <button type="button" aria-pressed={feed.pending} disabled={disabled} onClick={() => feed.history?.(true)}>待处理</button>
      <button type="button" aria-pressed={!feed.pending} disabled={disabled} onClick={() => feed.history?.(false)}>全部历史</button>
      {feed.handle && <button type="button" disabled={disabled || !pendingIds.length} onClick={() => feed.handle?.(pendingIds)} title="仅标记当前页显示的待处理信件；保留历史">处理本页 {pendingIds.length} 条</button>}
    </nav>}
    {!feed ? <p role="status">信箱读取中</p> : <>
      {feed.error && <p role="alert">读取失败：{feed.error}</p>}
      {feed.actionError && <p role="alert">{feed.actionError}</p>}
      {navigationError && <p role="alert">{navigationError}</p>}
      {[...groups].map(([key, messages]) => {
        const first = messages[0], ids = messages.filter(message => !message.handled_at).map(message => message.id);
        return <section className="human-sender-group" key={key} aria-label={`来自 @${first.from}`}>
          <header><button type="button" disabled={opening || !first.sender || !feed.openSender || !onNavigate} title={first.sender ? "打开精确发送者会话" : "历史信件未记录精确发送者"} onClick={() => void navigate(first)}>@{first.from}</button><span>{messages.length} 封</span>
          {feed.handle && <button type="button" disabled={disabled || !ids.length} onClick={() => feed.handle?.(ids)}>处理此组 {ids.length} 条</button>}</header>
          {messages.map(message => <HumanLetter key={message.id} message={message} disabled={disabled} handle={feed.handle} renderText={renderText} />)}
        </section>;
      })}
      {!feed.error && !feed.messages.length && <p>{feed.pending ? "本页没有待处理信件" : "暂无信件"}</p>}
      {feed.page && <nav className="human-mailbox-actions" aria-label="信箱分页">
        <button type="button" disabled={disabled || !feed.older} onClick={() => feed.page?.(false)}>回到最新</button>
        <button type="button" disabled={disabled || !feed.hasMore} onClick={() => feed.page?.(true)}>更早信件</button>
      </nav>}
    </>}
  </section>;
}

function HumanLetter({ message, handle, disabled, renderText }: { message: HumanMessage; handle?: MailboxFeed["handle"]; disabled: boolean; renderText(text: string): ReactNode }) {
  const status = message.handled_at ? "已处理" : handle ? "待处理" : "历史记录";
  return <article data-human-message={message.id}>
    <header><strong>@{message.from}</strong><span title={message.handled_at ?? undefined}>{status}</span>
      {handle && !message.handled_at && <button type="button" disabled={disabled} onClick={() => handle([message.id])} aria-label={`处理来自 @${message.from} 的信件 ${message.id}`}>标为已处理</button>}
    </header><time>{message.at}</time>{renderText(message.text)}
  </article>;
}
