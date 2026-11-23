/** @aiPurpose One page-lifetime owner for exact selection, navigation and drafts. */
import type { LatestBookmark } from "../../uiux/systems/space/layouts/useConversationScroll";
import type { Attachment } from "../../../management/api/native";
import { findConversation, membershipKey, type AgentSpaceSupply, type AgentState, type OfficeView, type RecordChannel } from "./model";
import { emptyWindows, openWindow, removeWindow, toggleWindowFocus, type ConversationWindows } from "../workspace/windows";
export type ConversationOrigin = "actor" | "directory" | "list" | "resume";
export type Presentation = "scene" | "chat";
export type DraftEditorSize = "auto" | "compact" | "expanded" | number;
export interface DraftDetails { intent: string; thread: string; replyTo: string; height: DraftEditorSize; issue: string }
const defaultDraftDetails: DraftDetails = { intent: 'request', thread: '', replyTo: '', height: 'auto', issue: '' };
export interface SpaceSnapshot {
  presentation: Presentation; selectedId: string | null; reading: boolean;
  workspaceId: string; officeView: OfficeView; postId: string | null; query: string; state: AgentState | "";
  channels: Readonly<Record<string, RecordChannel>>;
  panel: "" | "directory" | "mailbox"; detail: boolean; mail: "pending" | "all"; before: string;
  attachments: Readonly<Record<string, readonly Attachment[]>>; uploading: Readonly<Record<string, boolean>>; contextErrors: Readonly<Record<string, string>>;
  drafts: Readonly<Record<string, string>>;
  draftDetails?: Readonly<Record<string, DraftDetails>>;
  windows: ConversationWindows;
}
/** Own one page; supplied exact IDs include scope/generation/incarnation and record kind. */
export class SpaceController {
  private snapshot: SpaceSnapshot = { presentation: "scene", selectedId: null, reading: false, workspaceId: "", officeView: "members", postId: null, query: "", state: "", panel: "", detail: false, mail: "pending", before: "", channels: {}, attachments: {}, uploading: {}, contextErrors: {}, drafts: {}, windows: emptyWindows };
  navigationRevision = 0;
  private resourceReturn: "members" | "history" = "members";
  private listeners = new Set<() => void>();
  private draftRevisions = new Map<string, number>();
  readonly readerLatest = new Map<string, LatestBookmark>();
  readonly readerAnchors = new Map<string, { id: string; offset: number }>();
  readonly editorPositions = new Map<string, { top: number; start: number; end: number }>();
  draftRevision = (id: string) => this.draftRevisions.get(id) ?? 0;
  draftDetails = (id: string): DraftDetails => this.snapshot.draftDetails?.[id] ?? defaultDraftDetails;
  editDraftDetails = (id: string, patch: Partial<DraftDetails>) => {
    if (!this.has(id)) return;
    if ("intent" in patch || "thread" in patch || "replyTo" in patch) this.reviseDraft(id);
    this.update({ draftDetails: { ...this.snapshot.draftDetails, [id]: { ...this.draftDetails(id), ...patch } } });
  };
  private reviseDraft(id: string) { this.draftRevisions.set(id, this.draftRevision(id) + 1); }
  constructor(public supply: AgentSpaceSupply, readonly windowed = false) {}
  replaceSupply = (supply: AgentSpaceSupply) => {
    const reset = this.supply.scope.id !== supply.scope.id || this.supply.scope.generation !== supply.scope.generation || this.supply.scope.listener !== supply.scope.listener;
    this.supply = supply;
    this.update(reset ? { selectedId: null, reading: false, windows: emptyWindows } : {}, false);
  };
  getSnapshot = () => this.snapshot;
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  noteBrowserNavigation() { return ++this.navigationRevision; }
  /** Hydrate navigation atomically without changing exact draft/upload ownership. */
  restoreNavigation = (patch: Partial<SpaceSnapshot>) => {
    if (!this.windowed) return this.update(patch, false);
    const windows = patch.workspaceId === this.snapshot.workspaceId ? this.snapshot.windows : emptyWindows;
    this.update({ ...patch, windows: patch.reading && patch.selectedId ? openWindow(windows, patch.selectedId)
      : { ...windows, activeId: null, focused: false } }, false);
  };
  private update(patch: Partial<SpaceSnapshot>, navigation = true) {
    if (navigation && Object.keys(patch).some(key => ['presentation', 'selectedId', 'reading', 'workspaceId', 'officeView', 'postId', 'query', 'state', 'channels', 'panel', 'detail', 'mail', 'before'].includes(key))) this.navigationRevision++;
    this.snapshot = { ...this.snapshot, ...patch }; this.listeners.forEach(listener => listener()); }
  private has(id: string) { return !!findConversation(this.supply, id); }
  private windowSelection(windows: ConversationWindows) {
    const selectedId = windows.activeId;
    return { windows, selectedId, reading: !!selectedId, detail: selectedId === this.snapshot.selectedId && this.snapshot.detail };
  }
  activateWindow = (id: string) => {
    if (!this.windowed || !this.snapshot.windows.panes.some(pane => pane.id === id) || !this.has(id)) return;
    const windows = openWindow(this.snapshot.windows, id);
    if (windows !== this.snapshot.windows) this.update({ ...this.windowSelection(windows), panel: "" });
  };
  focusWindow = (id: string) => {
    if (!this.windowed || !this.snapshot.windows.panes.some(pane => pane.id === id) || !this.has(id)) return;
    this.update({ ...this.windowSelection(toggleWindowFocus(this.snapshot.windows, id)), panel: "" });
  };
  minimizeWindow = (id: string) => this.changeWindow(id, true);
  closeWindow = (id: string) => this.changeWindow(id, false);
  private changeWindow(id: string, minimize: boolean) {
    if (!this.windowed) return;
    const { windows } = this.snapshot;
    const next = removeWindow(windows, id, minimize);
    if (next !== windows) this.update(this.windowSelection(next));
  }
  /** Accept an addressed roster intent once; only the separate explicit route can open an unpaged subject. */
  openConversation = (id: string, origin: ConversationOrigin) => {
    const { workspaceId, selectedId, reading, officeView } = this.snapshot;
    const agent = origin === "resume" && id === selectedId ? findConversation(this.supply, id) : this.supply.agents.find(row => row.id === id);
    if (!agent || workspaceId && membershipKey(agent) !== workspaceId || origin === "actor" && agent.ref.record !== "current") return false;
    if (reading && selectedId === id) return true;
    this.update({ selectedId: id, reading: true, panel: "", detail: false, officeView: officeView === "posts" || officeView === "skills" ? "members" : officeView,
      ...(this.windowed ? { windows: openWindow(this.snapshot.windows, id) } : {}) });
    return true;
  };
  select = (id: string) => this.openConversation(id, "list");
  /** Deliberate exact relation navigation; preserve every per-Agent draft and office camera. */
  openExact = (id: string, sourceId?: string) => {
    const agent = findConversation(this.supply, id);
    if (!agent) return;
    const current = this.snapshot;
    const targetOffice = membershipKey(agent);
    const sourceInShelf = this.windowed && !!sourceId && current.windows.panes.some(pane => pane.id === sourceId && !pane.minimized);
    // A deliberate jump from a pane can span offices without removing the source from the shelf.
    const workspaceId = this.windowed && (!current.workspaceId || sourceInShelf && current.workspaceId !== targetOffice) ? "" : targetOffice;
    const windows = sourceInShelf || current.workspaceId === workspaceId ? current.windows : emptyWindows;
    const companion = sourceInShelf && sourceId !== id ? openWindow(windows, sourceId) : windows;
    this.update({ workspaceId, selectedId: id, reading: true, panel: "", detail: false, postId: null, query: '', state: '', officeView: agent.ref.record === 'stopped' ? 'history' : 'members',
      ...(this.windowed ? { windows: openWindow(companion, id) } : {}) });
  };
  historyDirectory = () => this.update(this.windowed ? { officeView: "history", panel: "directory", state: "" }
    : { officeView: "history", panel: "directory", reading: false, detail: false, state: "" });
  close = () => {
    if (this.windowed && this.snapshot.windows.activeId) this.closeWindow(this.snapshot.windows.activeId);
    else this.update({ reading: false, detail: false, panel: "" });
  };
  panel = (panel: SpaceSnapshot["panel"]) => { if (panel !== this.snapshot.panel) this.update(this.windowed ? { panel } : { panel, reading: panel ? false : this.snapshot.reading, detail: panel ? false : this.snapshot.detail }); };
  detail = (detail: boolean) => this.update({ detail });
  mailbox = (mail: SpaceSnapshot["mail"], before: string) => this.update({ mail, before });
  presentation = (presentation: Presentation) => this.update({ presentation, panel: "" });
  office = (workspaceId: string) => this.update({ workspaceId, selectedId: null, reading: false, panel: "", detail: false, postId: null, query: "", state: "", officeView: "members", windows: emptyWindows });
  view = (officeView: OfficeView) => {
    const resourceView = officeView === "posts" || officeView === "skills";
    const previous = this.snapshot.officeView;
    if (resourceView && (previous === "members" || previous === "history")) this.resourceReturn = previous;
    this.update({ officeView, reading: this.windowed && !resourceView && !!this.snapshot.windows.activeId, panel: "", detail: false,
      postId: officeView === "posts" ? this.supply.posts.find(post => !this.snapshot.workspaceId || post.office === this.snapshot.workspaceId)?.id ?? null : null, state: "" });
  };
  closeResources = () => this.view(this.resourceReturn);
  post = (postId: string) => { if (this.supply.posts.some(post => post.id === postId)) this.update({ postId }); };
  channel = (id: string, channel: RecordChannel) => { if (this.has(id)) this.update({ channels: { ...this.snapshot.channels, [id]: channel } }); };
  filter = (patch: Partial<Pick<SpaceSnapshot, "query" | "state">>) => this.update(this.windowed ? patch : { ...patch, reading: false });
  clearFilters = () => this.filter({ query: "", state: "" });
  writeDraft = (id: string, text: string) => { if (this.has(id)) { this.reviseDraft(id); this.update({ drafts: { ...this.snapshot.drafts, [id]: text } }); } };
  /** Clear only the submitted revision; preserve edits made while a receipt was pending, even when text is identical. */
  clearSentDraft = (id: string, expected: string, sentAttachments: readonly string[], revision: number) => {
    if (!this.has(id)) return;
    const sent = new Set(sentAttachments);
    const matches = this.draftRevision(id) === revision && (this.snapshot.drafts[id] ?? '') === expected;
    if (matches) this.reviseDraft(id);
    this.update({
      drafts: matches ? { ...this.snapshot.drafts, [id]: "" } : this.snapshot.drafts,
      draftDetails: matches ? { ...this.snapshot.draftDetails, [id]: { ...defaultDraftDetails, height: this.draftDetails(id).height } } : this.snapshot.draftDetails,
      attachments: { ...this.snapshot.attachments, [id]: (this.snapshot.attachments[id] ?? []).filter(item => !sent.has(item.id)) },
    });
  };
  removeAttachment = (id: string, attachment: string) => this.update({ attachments: { ...this.snapshot.attachments, [id]: (this.snapshot.attachments[id] ?? []).filter(item => item.id !== attachment) } });
  contextError = (id: string, error: string) => this.update({ contextErrors: { ...this.snapshot.contextErrors, [id]: error } });
  /** Commit context only after all uploads succeed; never retarget late completion. */
  async addContext(id: string, text: string, files: readonly Blob[], upload: (files: readonly Blob[]) => Promise<Attachment[]>, maxText: number, beforeCommit?: () => void) {
    if (!this.has(id) || this.snapshot.uploading[id]) throw new Error('目标不可用或附件正在上传');
    if ((this.snapshot.attachments[id]?.length ?? 0) + files.length > 8) throw new Error('每条消息最多 8 个附件');
    this.update({ uploading: { ...this.snapshot.uploading, [id]: true }, contextErrors: { ...this.snapshot.contextErrors, [id]: '' } });
    const scope = this.supply.scope;
    try {
      if (files.some(file => file.size > 10 * 1024 * 1024)) throw new Error('单个附件不能超过 10 MiB');
      const items = await upload(files);
      if (!this.has(id) || scope.id !== this.supply.scope.id || scope.generation !== this.supply.scope.generation) throw new Error('上传期间目标已失效');
      beforeCommit?.();
      const previous = this.snapshot.drafts[id] ?? '';
      const draft = [previous, text].filter(Boolean).join('\n\n');
      if (new TextEncoder().encode(draft).length > maxText) throw new Error('上下文超过消息字节上限');
      this.reviseDraft(id);
      this.update({ drafts: { ...this.snapshot.drafts, [id]: draft }, attachments: { ...this.snapshot.attachments, [id]: [...(this.snapshot.attachments[id] ?? []), ...items] } });
    } catch (error) { this.contextError(id, error instanceof Error ? error.message : String(error)); throw error; }
    finally { this.update({ uploading: { ...this.snapshot.uploading, [id]: false } }); }
  }
}
