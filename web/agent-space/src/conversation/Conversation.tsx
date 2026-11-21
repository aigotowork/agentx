/** @aiPurpose Read AgentX messages and Provider transcripts for one exact Agent. */
import { useId, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { adoptedRoleLabel, type AgentResource, type RecordChannel, type SuppliedRecords } from "../offline/model";
import { HALL_STATE_LABEL } from "../scene/model";
import { Button, ConversationBlock, ConversationComposer } from "../../uiux/systems/space/space.web";
import type { SpaceController } from "../offline/controller";
import { History } from "./History";
import { HistorySearch, matchingRecords } from "./HistorySearch";
import { CopyButton } from "./CopyButton";
import { AgentActions } from "../live/Actions";
import { operationIssue, type LiveSpace } from "../live/controller";
import { AgentAvatar } from "../Directory";
import { studioSignalLabel } from "../scene/studio-art";
import { RelationDisclosure } from "./LaunchRelations";
import { ForkControl, ForkFeedback } from "./Fork";
import { BoundWork } from "./BoundWork";
import "./compact-surfaces.css";
interface Props {
  agent: AgentResource; records: SuppliedRecords; channel: RecordChannel; onChannel(channel: RecordChannel): void; draft: string;
  controller: SpaceController; onDraft(text: string): void; onBack?(): void; live?: LiveSpace; scrollPositions?: Map<string, number>; studio?: boolean; backLabel?: string;
  presentationControl?: ReactNode;
  workspace?: boolean;
  workspaceControls?: ReactNode;
  onOpenRelated?(id: string): void;
}
/** @aiPurpose Keep exact identity, both record channels and the native message composer together.
 * @aiUsage Parent owns exact drafts and channels; artwork never controls native capabilities.
 * @aiLayout In workspace mode the surrounding toolbar supplies close and sizing; disclosures belong to this pane.
 */
export function Conversation({ agent, records, channel, onChannel, draft, onDraft, onBack, live, controller, scrollPositions, studio, backLabel, presentationControl, workspace, workspaceControls, onOpenRelated }: Props) {
  const snapshot = controller.getSnapshot(), attachments = snapshot.attachments[agent.id] ?? [], uploading = !!snapshot.uploading[agent.id];
  const contextError = snapshot.contextErrors[agent.id];
  const { intent, thread, replyTo, height, issue: sendIssue } = controller.draftDetails(agent.id);
  const [searchOpen, setSearchOpen] = useState(false), [query, setQuery] = useState('');
  const [seek, setSeek] = useState<{ id: string }>();
  const searchButton = useRef<HTMLButtonElement>(null);
  const searchId = useId();
  const disclosureGroup = useId();
  const stopped = agent.ref.record === 'stopped';
  const reset = live?.connection === 'reset';
  const page = channel === 'messages' ? live?.messagePage(agent.id) : undefined;
  const scrollKey = JSON.stringify([agent.id, channel]);
  const preserveScroll = !!live || workspace || !!controller.supply.roster;
  const forwardHistory = page?.mode === 'forward';
  const latest = page?.mode === 'latest' ? page : undefined;
  const readOnly = stopped || reset;
  const canCompose = !readOnly && !!live?.canSendTo(agent.id);
  const sendBlocked = !!live?.blocked(agent.id);
  const canSend = canCompose && !sendBlocked && (!!draft.trim() || attachments.length > 0);
  const maxText = live?.descriptor?.max_text ?? 200_000;
  const validSize = new TextEncoder().encode(draft).length <= maxText;
  async function send(): Promise<void> {
    if (!live || channel !== 'messages' || !canSend || !validSize || uploading) return;
    const sentDraft = draft, sentAttachments = attachments.map(item => item.id), revision = controller.draftRevision(agent.id);
    controller.editDraftDetails(agent.id, { issue: '' });
    await live.act(async () => {
      try {
        await live.target('send', agent.id, { text: sentDraft, intent: intent || null, thread: thread || null, reply_to: replyTo || null, attachments: sentAttachments }, () => controller.clearSentDraft(agent.id, sentDraft, sentAttachments, revision));
      } catch (error) {
        controller.editDraftDetails(agent.id, { issue: error instanceof Error ? error.message : String(error) });
        throw error;
      }
    });
  }
  const receipt = live?.receipts.find(item => item.key === agent.id && item.kind === 'send' && live.ownsReceipt(item));
  const status = receipt?.operation?.status;
  const pendingReceipt = !!receipt && status !== 'Succeeded' && status !== 'Failed';
  const receiptFeedback = pendingReceipt ? (receipt.issue || status === 'Unknown' ? '发送结果未确定 · 草稿已保留' : '等待发送回执…')
    : status === 'Failed' ? `发送失败 · ${receipt?.operation?.error || '草稿已保留'}` : status === 'Succeeded' ? (draft ? '上一条已发送' : '已发送') : '';
  const composerStatus = reset ? '连接已失效 · 草稿已保留' : stopped ? '停止历史 · 只读' : uploading ? '附件上传中…' : sendIssue || receiptFeedback || (!live ? '演示 · 不发送' : !canCompose ? '当前不可发送' : sendBlocked ? '发送不可用 · 查看连接与回执' : '');
  const receiptAction = live && receipt && pendingReceipt && <button type="button" className="composer-receipt" disabled={!live.canLookup(receipt)} onClick={() => void live.lookup(receipt)} title={receipt.issue || receipt.operation?.error || '查询同一次发送'}>查询原回执</button>;
  async function addFiles(files: readonly Blob[]) {
    if (!live || channel !== 'messages' || !canCompose || sendBlocked || !files.length) return;
    try { await controller.addContext(agent.id, '', files, items => live.upload(agent.id, items), maxText); }
    catch (error) { controller.contextError(agent.id, error instanceof Error ? error.message : String(error)); }
  }
  const recordKind = agent.ref.record === "stopped" ? "停止历史" : "当前实例";
  const identity = { title: agent.title, role: studio && agent.launchTitle ? agent.launchTitle : adoptedRoleLabel(agent.role), kind: stopped && workspace ? `历史 #${agent.ref.stopped_event ?? ''}` : recordKind, state: (studio && studioSignalLabel(agent)) || HALL_STATE_LABEL[agent.state], source: agent.source.title, avatar: <AgentAvatar agent={agent} studio={studio} /> };
  const exactLabel = `${agent.title} · ${recordKind} ${agent.ref.stopped_event ?? agent.ref.incarnation ?? ''}`;
  const options = canCompose && <MessageOptions intent={intent} thread={thread} replyTo={replyTo} onEdit={patch => controller.editDraftDetails(agent.id, patch)} />;
  const editorMemory = { key: agent.id, positions: controller.editorPositions };
  const composer = channel === 'messages' && (!readOnly || draft || attachments.length > 0 || reset) && <ConversationComposer draft={draft} onDraft={onDraft} attachments={attachments} draftOnly={!canCompose} uploading={uploading} canSend={canSend} canAttach={canCompose && !sendBlocked} readOnly={readOnly} maxText={maxText} status={composerStatus} contextError={contextError} onSubmit={send} onAddFiles={addFiles} onRemoveAttachment={id => controller.removeAttachment(agent.id, id)} options={options} editorSize={height} onEditorSize={height => controller.editDraftDetails(agent.id, { height })} editorMemory={editorMemory} receiptAction={receiptAction} />;
  const items = records.status === 'available' ? records.items : undefined;
  const matches = useMemo(() => matchingRecords(items ?? [], query), [items, query]);
  const hit = matches.findIndex(item => item.id === seek?.id);
  const searchTrigger = <button ref={searchButton} className="workspace-reader-trigger history-search-trigger" type="button" title="查找已读入记录" aria-label="查找已读入记录" aria-expanded={searchOpen} aria-controls={searchOpen ? searchId : undefined} onClick={() => setSearchOpen(!searchOpen)}><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><circle cx="10.5" cy="10.5" r="5.5" /><path d="m15 15 5 5" /></svg></button>;
  function moveMatch(direction: -1 | 1) {
    if (!matches.length) return;
    const next = hit < 0 ? (direction > 0 ? 0 : matches.length - 1) : (hit + direction + matches.length) % matches.length;
    setSeek({ id: matches[next].id });
  }
  const historySearch = searchOpen && <HistorySearch id={searchId} query={query} index={hit} count={matches.length} onQuery={text => { setQuery(text); setSeek(undefined); }} onMove={moveMatch} onClose={() => { setSearchOpen(false); searchButton.current?.focus(); }} />;
  function continueHistory() {
    if (document.hidden || reset || page?.phase !== 'ready' || !page.more) return;
    if (latest) {
      if (latest.older?.phase !== 'ready') return;
      const bookmark = controller.readerLatest.get(scrollKey);
      if (bookmark) bookmark.following = false;
    }
    live?.moreMessagesFor(agent.id);
  }
  const retryHistory = () => live?.moreMessagesFor(agent.id);
  const olderBoundary = latest && <OlderBoundary page={latest} reset={reset} onRead={continueHistory} onRetry={retryHistory} />;
  const olderStatus = latest?.older?.phase === 'error' ? <>
    <small title={latest.older.error}>更早消息读取失败</small>
    <Button disabled={reset} onClick={event => {
      event.currentTarget.closest('.conversation-layout')?.querySelector<HTMLElement>('.conversation-history')?.focus({ preventScroll: true });
      retryHistory();
    }}>重试更早消息</Button>
  </> : latest?.older?.phase === 'loading' && <small>正在读取更早消息…</small>;
  const forwardBoundary = forwardHistory && <MessageBoundary phase={page.phase} more={page.more} visibleCount={items?.length ?? 0} error={items ? page.error : undefined} reset={reset} stopped={stopped} onRetry={retryHistory} />;
  const liveIssue = latest?.live?.phase === 'error' && <Button title={latest.live.error} disabled={reset} onClick={() => live?.retryMessagesLive(agent.id)}>重试新消息</Button>;
  const transcript = channel === 'model' && <TranscriptControls agent={agent} live={live} />;
  const showRecords = !(channel === 'model' && live && records.status === 'unavailable');
  const legacyNotice = forwardHistory && <p className="history-order">当前服务无法直达最新 · 从最早记录开始</p>;
  const history = <>{olderBoundary}<BoundWork agent={agent} />{transcript}{legacyNotice}{showRecords && <RecordReader key={scrollKey} agent={agent} records={records} channel={channel} followEnd={!preserveScroll} sourceInFacts={workspace} match={searchOpen ? seek?.id : undefined} forward={forwardHistory} partial={page?.more} />}{forwardBoundary}</>;
  const relations = live && <RelationDisclosure key={agent.id} agent={agent} live={live} controller={controller} onOpenRelated={onOpenRelated} />;
  const actions = live && <AgentActions live={live} agent={agent} />;
  const facts = <AgentFacts key={agent.id} agent={agent} controller={controller} group={workspace ? disclosureGroup : undefined} syncDetail={!workspace || snapshot.selectedId === agent.id} source={workspace ? identity.source : undefined} provenance={workspace && records.status === 'available' ? records.provenance : undefined} actions={actions} />;
  const fork = live && <ForkControl agent={agent} live={live} />;
  const forkResult = live && <ForkFeedback key={agent.id} agent={agent} live={live} onOpen={onOpenRelated ?? controller.openExact} />;
  const latestScroll = latest?.token && { token: latest.token, arrivals: latest.arrivals ?? 0, bookmarks: controller.readerLatest,
    beforeLayout: `${latest.more}:${latest.older?.phase}:${latest.emptyOlder}`, firstId: items?.[0]?.id, onOlder: continueHistory };
  const historyScroll = scrollPositions ? { key: scrollKey, positions: scrollPositions, anchors: controller.readerAnchors, ready: records.status === "available", forward: forwardHistory, onForward: forwardHistory ? continueHistory : undefined, seek, latest: latestScroll } : undefined;
  function chooseChannel(value: RecordChannel) { if (value !== channel) { setQuery(''); setSeek(undefined); onChannel(value); } }
  return <ConversationBlock workspace={workspace} workspaceControls={workspaceControls} identity={identity} exactLabel={exactLabel} channel={channel} onChannel={chooseChannel} onBack={onBack} backLabel={backLabel} presentationControl={presentationControl} facts={facts} primaryAction={fork} result={forkResult} relations={relations} history={history} olderStatus={olderStatus} liveIssue={liveIssue} composer={composer} historyScroll={historyScroll} historySearch={historySearch} searchTrigger={searchTrigger} />;
}

function OlderBoundary({ page, reset, onRead, onRetry }: { page: Extract<NonNullable<ReturnType<LiveSpace['messagePage']>>, { mode: 'latest' }>; reset: boolean; onRead(): void; onRetry(): void }) {
  const root = useRef<HTMLDivElement>(null);
  const [short, setShort] = useState(false);
  useLayoutEffect(() => {
    const node = root.current, region = node?.closest<HTMLElement>('.conversation-history');
    if (!node || !region) return;
    const measure = () => {
      const last = region.lastElementChild;
      if (last) {
        const depth = last.getBoundingClientRect().bottom - region.getBoundingClientRect().top + region.scrollTop;
        setShort(region.clientHeight > 0 && depth - node.getBoundingClientRect().height <= region.clientHeight);
      }
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(region);
    for (const child of region.children) if (child !== node) observer.observe(child);
    return () => observer.disconnect();
  }, [page.token, page.phase, page.older?.phase, page.more]);

  if (page.phase === 'error') {
    return <div className="history-boundary" data-state="error"><Button disabled={reset} onClick={onRetry}>重试消息</Button></div>;
  }
  if (!page.token) return null;
  const pending = page.older?.phase !== 'ready';
  const showMore = page.more && (short || page.emptyOlder);
  function demand(action: () => void) {
    root.current?.closest<HTMLElement>('.conversation-history')?.focus({ preventScroll: true });
    action();
  }
  return <div ref={root} className="history-older">
    {showMore && <Button disabled={reset || pending} style={pending ? { visibility: 'hidden' } : undefined} onClick={() => demand(onRead)}>查看更早消息</Button>}
  </div>;
}

function MessageBoundary({ phase, more, error, visibleCount, stopped, reset, onRetry }: { phase: 'loading' | 'ready' | 'error'; more: boolean; visibleCount: number; error?: string; stopped: boolean; reset: boolean; onRetry(): void }) {
  const boundary = useRef<HTMLDivElement>(null);
  const hint = useRef<HTMLSpanElement>(null);
  const [noScroll, setNoScroll] = useState(false);
  useLayoutEffect(() => {
    const region = boundary.current?.closest<HTMLElement>('.conversation-history');
    if (!region || !more || phase !== 'ready') return;
    const measure = () => {
      const end = hint.current?.getBoundingClientRect().bottom;
      const availableEnd = region.getBoundingClientRect().top + region.clientHeight;
      setNoScroll(!!region.clientHeight && end !== undefined && end + region.scrollTop <= availableEnd);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(region);
    const feed = region.querySelector('.history-feed');
    if (feed) observer.observe(feed);
    return () => observer.disconnect();
  }, [more, phase, visibleCount]);
  if (phase === 'error') return <div className="history-boundary" data-state="error">{error && <span role="status">后续消息读取失败 · {error}</span>}<button type="button" disabled={reset} onClick={onRetry}>重试消息</button></div>;
  if (reset || phase === 'loading' && !visibleCount) return null;
  const text = phase === 'loading' ? '正在读取后续消息…' : more ? `已读入 ${visibleCount} 条` : visibleCount ? stopped ? '已到历史末尾' : '已到当前记录末尾' : '';
  return text && <div ref={boundary} className="history-boundary" data-state={phase}><span ref={hint} role="status">{text}{more && phase === 'ready' && !noScroll && ' · 向下继续'}</span>{more && phase === 'ready' && noScroll && <button type="button" onClick={onRetry}>继续读取</button>}</div>;
}

function MessageOptions({ intent, thread, replyTo, onEdit }: { intent: string; thread: string; replyTo: string; onEdit(patch: { intent?: string; thread?: string; replyTo?: string }): void }) {
  const customized = intent !== 'request' || !!thread || !!replyTo;
  const hint = customized ? [intent, thread && `线程 ${thread}`, replyTo && `回复 ${replyTo}`].filter(Boolean).join(' · ') : '消息选项';
  return <details className="message-options" data-customized={customized || undefined}>
    <summary aria-label="消息选项" title={hint}>⋯</summary>
    <div className="message-options-fields">
      <label>意图<select aria-label="消息意图" value={intent} onChange={event => onEdit({ intent: event.target.value })}><option value="request">request</option><option value="inform">inform</option><option value="ack">ack</option></select></label>
      <label>线程<input aria-label="消息线程" value={thread} onChange={event => onEdit({ thread: event.target.value })} /></label>
      <label>回复引用<input aria-label="消息回复引用" value={replyTo} onChange={event => onEdit({ replyTo: event.target.value })} /></label>
    </div>
  </details>;
}
function TranscriptControls({ agent, live }: { agent: AgentResource; live?: LiveSpace }) {
  const supported = live?.capabilities(agent.id)?.has_transcript;
  const receipt = live?.receipts.find(item => item.key === agent.id && item.kind === 'transcript' && live.ownsReceipt(item));
  const issue = receipt && operationIssue(receipt);
  const pending = receipt?.operation?.status === 'Accepted' || receipt?.operation?.status === 'Running';
  return <div className="space-read-status"><small>只读</small>{live && (supported
    ? <button type="button" disabled={live.blocked(agent.id, 'transcript')} onClick={() => void live.act(() => live.target('transcript', agent.id))}>读取 Transcript</button>
    : <small role="status">Transcript 不可用</small>)}{pending && <small role="status">读取中…</small>}{receipt && issue && receipt.operation?.status !== 'Failed' && <span role="status">{issue} <button type="button" disabled={!live?.canLookup(receipt)} onClick={() => void live?.lookup(receipt)}>查询原回执</button></span>}</div>;
}
function RecordReader({ agent, records, channel, followEnd, sourceInFacts, match, forward, partial }: { agent: AgentResource; records: SuppliedRecords; channel: RecordChannel; followEnd: boolean; sourceInFacts?: boolean; match?: string; forward?: boolean; partial?: boolean }) {
  if (records.status === "loading") return <div className="record-availability" role="status">{channel === "messages" ? "正在读取消息" : "正在读取 Transcript"}</div>;
  if (records.status !== "available") return <div className="record-availability" role="status"><strong>{records.status === "error" ? "读取失败" : "记录未提供"}</strong><span>{records.reason}</span></div>;
  const empty = channel === "messages" ? partial ? "这一段没有消息" : "暂无 AgentX 消息" : "Transcript 为空";
  return <>{!sourceInFacts && <details className="record-provenance"><summary>{channel === "model" ? "Provider 来源" : "AgentX 消息来源"} ⓘ</summary><span>{records.provenance}</span></details>}{records.items.length ? <History items={records.items} agentName={agent.name} followEnd={followEnd} compact={sourceInFacts} match={match} forward={forward} /> : <div className="record-availability">{empty}</div>}</>;
}
function AgentFacts({ agent, controller, group, syncDetail, source, provenance, actions }: { agent: AgentResource; controller: SpaceController; group?: string; syncDetail: boolean; source?: string; provenance?: string; actions?: ReactNode }) {
  const membership = agent.membership.kind === "directory" ? agent.membership.path : agent.membership.kind === "global" ? "全局成员" : agent.membership.kind === "legacy" ? "旧记录 · 未记录归属" : "归属未知";
  function syncRouteDetail(event: React.ToggleEvent<HTMLDetailsElement>) {
    const snapshot = controller.getSnapshot();
    if (group && snapshot.selectedId !== agent.id) return;
    if (event.currentTarget.open !== snapshot.detail) controller.detail(event.currentTarget.open);
  }
  return <details className="hall-conversation-options" name={group} open={syncDetail ? controller.getSnapshot().detail : undefined} onToggle={syncDetail ? syncRouteDetail : undefined}>
    <summary className="hall-options-trigger workspace-reader-trigger" aria-label="会话详情" title="会话详情"><span aria-hidden="true">⋯</span></summary>
    <div className={group ? "workspace-reader-panel reader-facts" : "hall-options-menu reader-facts"}>
      <strong>{agent.title} · {agent.ref.record === "stopped" ? "停止历史" : "当前实例"}</strong>
      {actions}
      <p>{agent.description}</p>{agent.launchTitle && <p>职责 · {agent.launchTitle}</p>}{agent.role.kind === "adopted" && <p>岗位说明 · {agent.role.description || "岗位说明未提供"}</p>}
      <dl>{source && <><dt>实例来源</dt><dd>{source}</dd></>}{provenance && <><dt>记录来源</dt><dd>{provenance}</dd></>}<dt>成员归属</dt><dd>{membership}</dd><dt>独立工作目录</dt><dd>{agent.cwd ?? "未提供"}</dd><dt>采用岗位</dt><dd>{adoptedRoleLabel(agent.role)}</dd>{agent.role.kind === "adopted" && <><dt>采用来源 / 快照</dt><dd>{agent.role.source}<br />{agent.role.revision}</dd><dt>Role 启动配置引用</dt><dd>{agent.role.profile}</dd></>}<dt>采用的启动配置</dt><dd>{agent.profile ?? "未提供"}</dd><dt>工具 / 观测模型</dt><dd>{agent.tool} / {agent.model ?? "模型未提供"}</dd><dt>精确引用</dt><dd><code>{agent.id}</code><CopyButton text={agent.id} /></dd></dl>
      {!group && <span>状态不代表任务验收。</span>}
    </div>
  </details>;
}
