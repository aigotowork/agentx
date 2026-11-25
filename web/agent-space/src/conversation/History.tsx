/** Message, reasoning and tool DOM adapted from web/conversation/HistoryRecord.tsx.
 * Copyright (c) 2025-present Mohamed Boudra. Apache-2.0; see ../../public/notices/Conversation-Apache-2.0.txt.
 * AgentX changes: explicit local records, no provider, extensions, remote media or runtime actions.
 */
import { useEffect, useRef } from "react";
import type { HistoryRecord } from "../offline/model";
import { Attachments } from "../../uiux/systems/space/space.web";
import { Markdown } from "./Markdown";
import { CopyButton } from "./CopyButton";

/** @aiPurpose Preserve the source's readable chronological messages and local process disclosures.
 * @aiUsage Pass native or explicitly demo records; use followEnd only for live-tail readers, not deliberate forward paging.
 */
export function History({ items, agentName, followEnd = true, compact, match }: { items: readonly HistoryRecord[]; agentName: string; followEnd?: boolean; compact?: boolean; match?: string }) {
  const root = useRef<HTMLDivElement>(null), following = useRef(true);
  const lastId = items.at(-1)?.id;
  useEffect(() => {
    if (!followEnd) return;
    const region = root.current?.closest(".conversation-history"); if (!region) return;
    const scroll = () => { following.current = region.scrollHeight - region.scrollTop - region.clientHeight < 120; };
    region.addEventListener("scroll", scroll, { passive: true });
    return () => region.removeEventListener("scroll", scroll);
  }, [followEnd]);
  useEffect(() => { if (followEnd && following.current) root.current?.lastElementChild?.scrollIntoView({ block: "end" }); }, [followEnd, lastId]);
  return <div ref={root} className="history-feed history-projected-feed" role={followEnd ? "log" : undefined} aria-label="会话记录">{items.map((item, index) => {
    const at = compact ? dayLabel(item) : undefined;
    return <div className="history-entry" key={item.id}>
      {at && at !== dayLabel(items[index - 1]) && <div className="history-date"><span>{at}</span></div>}
      <div data-history-record={item.id} data-history-match={match === item.id || undefined}><Record item={item} agentName={agentName} compact={compact} /></div>
    </div>;
  })}</div>;
}
const calendarDay = new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium" });
const clockTime = new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit" });
function dayLabel(item?: HistoryRecord) {
  return item?.kind === "message" && Number.isFinite(Date.parse(item.at)) ? calendarDay.format(new Date(item.at)) : undefined;
}
function Record({ item, agentName, compact }: { item: HistoryRecord; agentName: string; compact?: boolean }) {
  if (item.kind === "message") {
    const author = item.author;
    const dated = Number.isFinite(Date.parse(item.at));
    const kind = (compact && item.recipients ? author === agentName || author === `@${agentName}` : item.role === "assistant") ? "message-assistant" : "message-user";
    const recipients = item.recipients && (!compact || item.recipients.length !== 1 || item.recipients[0] !== agentName) ? item.recipients.join(", ") : '';
    return <article className={`message history-standard-record ${kind}`}><header className="message-header"><div className="history-message-author"><strong title={author}>{author}</strong>{recipients && <small>→ {recipients}</small>}{dated && <time dateTime={item.at} title={item.at}>{clockTime.format(new Date(item.at))}</time>}{(!compact || !item.native) && <small>{item.native ? "原生记录" : "示例"}</small>}{item.intent && <small>{item.intent}</small>}{item.thread && <small>线程 {item.thread}</small>}{item.replyTo && <details><summary>回复引用</summary><code>{item.replyTo}</code></details>}</div><div className="message-actions"><CopyButton text={item.text} /></div></header><Markdown text={item.text} />{!!item.attachments?.length && <Attachments items={item.attachments} />}</article>;
  }
  if (item.kind === "reasoning") return <details className="history-reasoning history-standard-record" aria-label="示例思考过程"><summary className="history-record-label">思考 · 示例</summary><Markdown text={item.text} /></details>;
  const status = item.status === "failed" ? "失败" : "完成";
  return <div className="history-tool-record"><details className="tool-record history-standard-record history-tool" data-state={item.status}><summary><span className="history-tool-face"><svg className="history-tool-chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 5 7 7-7 7" /></svg><strong>{item.name}</strong><span className="history-tool-summary">{item.summary}</span><span className="history-tool-state">{status} · 示例</span></span></summary><div className="tool-details"><section className="history-observation-content"><header><strong>详情</strong><CopyButton text={item.text} /></header><pre tabIndex={0}>{item.text}</pre></section></div></details></div>;
}
