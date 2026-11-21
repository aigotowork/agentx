import type { Agent, AgentAddress, AgentSnapshot, AgentXScope, MemberCursor, Message, MessageCursor, MessageHistoryCursor, NativeMemberCursor, Workspace, WorkspaceCatalog } from '@agentx/client';
import { GLOBAL_OFFICE, LEGACY_OFFICE, UNKNOWN_OFFICE } from '../offline/model';
import { domainAgentKey } from './projection';

type Phase = 'loading' | 'ready' | 'error';
interface ReadState { phase: Phase; error?: string }
type MemberPaging =
  | { kind: 'native-workspace-members'; next?: NativeMemberCursor | null; currentComplete: boolean }
  | { kind: 'global-agent-page'; next?: MemberCursor | null; scanned: number; legacy: number; unavailable: number };
export interface OfficeExploration extends ReadState {
  office: string; workspace: Workspace; rows: Map<string, AgentSnapshot>;
  request?: AbortController;
  paging: MemberPaging; emptyPage: boolean; revision: number;
}
type Lane = ReadState & { pending?: boolean };
interface ExactReader extends ReadState {
  id: string; office: string; agent: Agent; subject: AgentSnapshot; snapshot?: AgentSnapshot;
  lease: AbortController; pending: boolean;
  older: Lane; live: Lane;
  history?: ReadHistory; restoring?: ReadHistory;
}
type StoredMessages = { snapshot: AgentSnapshot; messages: Map<string, Message> };
type ForwardHistory = StoredMessages & { mode: 'forward'; checkpoint: MessageCursor; hasMore: boolean };
type LatestHistory = StoredMessages & {
  mode: 'latest'; before: MessageHistoryCursor | null; checkpoint: MessageCursor | null;
  token: object; arrivals: number; emptyOlder: boolean;
};
type ReadHistory = ForwardHistory | LatestHistory;
type PageState = ReadState & { more: boolean };
export type MessageReadPage = (PageState & { mode: 'forward' })
  | (PageState & { mode: 'latest'; older?: Lane; live?: Lane; token?: object; arrivals?: number; emptyOlder?: boolean });
const historyPageLimit = 20;

// Match the window shelf bound without depending on the presentation layer.
const retainedReaderLimit = 8;
const sameMembership = (a: AgentSnapshot['membership'], b: AgentSnapshot['membership']) =>
  a.kind === b.kind && (a.kind !== 'adopted' || (b.kind === 'adopted' && a.workspace === b.workspace));

const issueOf = (error: unknown) => error instanceof Error ? error.message : String(error);

export class DomainOfficeReads {
  catalog?: WorkspaceCatalog;
  catalogError = '';
  exploration?: OfficeExploration;
  private readerSet = new Map<string, ExactReader>();
  private parked = new Map<string, ReadHistory>();
  private office = '';
  private revision = 0;
  private discovery?: AbortController;
  private closed = false;
  readonly subjects = new Map<string, AgentSnapshot>();
  private fresh = new Set<string>();
  constructor(readonly scope: AgentXScope, private limit: number, private changed: () => void) {}
  get supportsHistory() { return this.scope.descriptor.supports_message_history === true; }
  get readers(): ReadonlyMap<string, ExactReader> { return this.readerSet; }
  *parkedSubjects() { for (const { snapshot } of this.parked.values()) yield snapshot; }

  async discover() {
    if (this.discovery || this.closed) return;
    const request = this.discovery = new AbortController();
    this.catalogError = '';
    this.changed();
    try {
      this.catalog = await this.scope.workspaces({ signal: request.signal });
    } catch (error) {
      if (!request.signal.aborted) this.catalogError = issueOf(error);
    } finally {
      if (this.discovery === request) { this.discovery = undefined; this.changed(); }
    }
  }

  choose(office: string) {
    if (this.closed) return;
    if (this.office !== office) {
      this.office = office;
      if (this.supportsHistory) for (const id of this.readerSet.keys()) this.parkReader(id);
      else this.clearReaders();
    }
    if (this.exploration?.office === office) return;
    const workspace = office === GLOBAL_OFFICE ? this.scope.workspace('global') : this.catalog?.workspaces.find(entry => entry.workspace.address.key === office)?.workspace;
    this.exploration?.request?.abort();
    this.exploration = workspace && this.newExploration(office, workspace);
    if (this.exploration) void this.readMembers(this.exploration);
  }

  more = () => {
    const office = this.exploration;
    if (office && (office.phase === 'error' || office.paging.next)) void this.readMembers(office);
  };

  private newExploration(office: string, workspace: Workspace): OfficeExploration {
    const paging: MemberPaging = this.scope.supportsWorkspaceMembers ? { kind: 'native-workspace-members', currentComplete: false }
      : { kind: 'global-agent-page', scanned: 0, legacy: 0, unavailable: 0 };
    return { office, workspace, rows: new Map(), phase: 'loading', paging, emptyPage: false, revision: ++this.revision };
  }

  refresh = () => {
    const old = this.exploration;
    if (!old || this.closed) return;
    old.request?.abort();
    const next = this.exploration = this.newExploration(old.office, old.workspace);
    void this.readMembers(next);
  };

  private async readMembers(office: OfficeExploration) {
    const paging = office.paging;
    if (office.request || this.closed || paging.next === null) return;
    const request = office.request = new AbortController();
    office.phase = 'loading'; office.error = undefined; this.changed();
    try {
      const page = paging.kind === 'native-workspace-members'
        ? await office.workspace.memberPage({ limit: this.limit, after: paging.next }, { signal: request.signal })
        : await office.workspace.members({ limit: this.limit, after: paging.next }, { signal: request.signal });
      if (request.signal.aborted || this.exploration !== office) return;
      for (const { snapshot } of page.members) office.rows.set(domainAgentKey(snapshot.address), snapshot);
      const coverage = page.coverage;
      if (paging.kind === 'native-workspace-members' && coverage.kind === 'native-workspace-members') {
        paging.next = coverage.next; paging.currentComplete = coverage.currentComplete;
      } else if (paging.kind === 'global-agent-page' && coverage.kind === 'global-agent-page') {
        paging.next = coverage.next; paging.scanned += coverage.scanned;
        paging.legacy += coverage.legacy; paging.unavailable += coverage.unavailable;
      }
      office.emptyPage = page.members.length === 0; office.phase = 'ready'; office.revision = ++this.revision;
    } catch (error) {
      if (request.signal.aborted || this.exploration !== office) return;
      office.phase = 'error'; office.error = issueOf(error);
    } finally {
      if (!request.signal.aborted && this.exploration === office) { office.request = undefined; this.changed(); }
    }
  }

  /** Resolve an exact route/related subject without admitting it to the workspace roster. */
  async resolve(address: AgentAddress, signal?: AbortSignal, office?: string, current: () => boolean = () => true) {
    const agent = this.scope.agent(address);
    const snapshot = await agent.snapshot({ signal });
    if (signal?.aborted || this.closed || !current()) throw new Error('精确导航已失效');
    if (office && !this.belongs(snapshot, office)) throw new Error('办公室与精确 Agent 归属不匹配');
    const id = domainAgentKey(snapshot.address);
    this.subjects.delete(id); this.subjects.set(id, snapshot); this.fresh.add(id);
    if (this.subjects.size > 20) {
      const oldest = this.subjects.keys().next().value!; this.subjects.delete(oldest); this.fresh.delete(oldest);
    }
    return snapshot;
  }

  private belongs(snapshot: AgentSnapshot, office: string) {
    if (!office) return true;
    if (office === LEGACY_OFFICE) return snapshot.membership.kind === 'legacy';
    if (office === UNKNOWN_OFFICE) return snapshot.membership.kind === 'unavailable';
    return snapshot.membership.kind === 'adopted' && snapshot.membership.workspace === (office === GLOBAL_OFFICE ? 'global' : office);
  }

  /** Reconcile visible leases before publication; pending subjects can be retried on the next supply update. */
  selectReaders(ids: readonly string[], observed: (id: string) => AgentSnapshot | undefined) {
    if (this.closed) return false;
    let changed = false;
    for (const id of this.readerSet.keys()) {
      if (!ids.includes(id)) { this.parkReader(id); changed = true; }
    }
    for (const id of ids) {
      if (this.readerSet.has(id)) continue;
      const cached = this.parked.get(id);
      if (this.parked.delete(id)) changed = true;
      const subject = observed(id) ?? cached?.snapshot;
      if (!subject || !this.belongs(subject, this.office)) continue;
      const agent = this.scope.agent(subject.address);
      const verified = this.fresh.delete(id) ? this.subjects.get(id) : undefined;
      const snapshot = !cached && verified && this.belongs(verified, this.office) ? verified : undefined;
      const reader: ExactReader = { id, office: this.office, agent, subject, snapshot, restoring: cached, lease: new AbortController(), pending: false, phase: 'loading', older: { phase: 'ready' }, live: { phase: 'ready' } };
      this.readerSet.set(id, reader);
      changed = true;
      void this.readMessages(reader, 'initial');
    }
    while (this.readerSet.size + this.parked.size > retainedReaderLimit) this.parked.delete(this.parked.keys().next().value!);
    return changed;
  }

  private parkReader(id: string) {
    const reader = this.readerSet.get(id)!;
    this.readerSet.delete(id);
    reader.lease.abort();
    const history = reader.history ?? reader.restoring;
    if (history) this.parked.set(id, history);
  }

  private clearReaders() {
    for (const reader of this.readerSet.values()) reader.lease.abort();
    this.readerSet.clear(); this.parked.clear();
  }

  messagePage(id: string): MessageReadPage | undefined {
    const reader = this.readerSet.get(id);
    if (!reader) return;
    const history = reader.history;
    if (history?.mode === 'latest') return {
      mode: 'latest' as const, phase: reader.phase, error: reader.error, more: history.before !== null,
      older: { ...reader.older }, live: { ...reader.live }, token: history.token, arrivals: history.arrivals,
      emptyOlder: history.emptyOlder,
    };
    return { mode: this.supportsHistory ? 'latest' : 'forward',
      phase: reader.phase, error: reader.error, more: history?.hasMore ?? false };
  }

  continueMessages = (id: string) => {
    const reader = this.readerSet.get(id);
    if (!reader) return;
    if (reader.history?.mode === 'latest') void this.readOlder(reader, reader.history);
    else if (reader.phase === 'error' || reader.history?.hasMore) void this.readMessages(reader, 'explicit');
  };

  retryLive(id: string) {
    const reader = this.readerSet.get(id);
    if (reader?.agent.kind === 'current' && reader.history?.mode === 'latest') void this.readLive(reader, reader.history);
  }

  /** One bounded update per visible current reader; older pages never gate its live cursor. */
  catchUp() {
    for (const reader of this.readerSet.values()) {
      if (reader.agent.kind !== 'current') continue;
      if (reader.history?.mode === 'latest') void this.readLive(reader, reader.history);
      else if (reader.phase === 'ready' && !reader.history?.hasMore) void this.readMessages(reader, 'passive');
    }
  }

  private async readMessages(reader: ExactReader, mode: 'initial' | 'explicit' | 'passive') {
    const signal = reader.lease.signal;
    if (reader.pending || signal.aborted) return;
    reader.pending = true;
    if (mode === 'explicit') { reader.phase = 'loading'; reader.error = undefined; this.changed(); }
    try {
      if (!reader.snapshot) {
        const snapshot = await reader.agent.snapshot({ signal });
        if (signal.aborted) return;
        const restored = reader.restoring;
        reader.restoring = undefined;
        if (!this.belongs(snapshot, reader.office)) throw new Error('原生记录已不再确认此办公室归属');
        reader.snapshot = snapshot;
        if (restored && sameMembership(restored.snapshot.membership, snapshot.membership)) {
          reader.history = restored;
          reader.phase = 'ready';
          return;
        }
      }
      const snapshot = reader.snapshot;
      if (this.supportsHistory) {
        const page = await reader.agent.messageHistory({ limit: this.historyLimit }, { signal });
        if (signal.aborted) return;
        reader.history = { mode: 'latest', snapshot, token: {}, arrivals: 0, emptyOlder: false,
          before: page.nextBefore, checkpoint: page.liveAfter, messages: new Map(page.messages.map(message => [message.id, message])) };
      } else {
        const history = reader.history?.mode === 'forward' ? reader.history : undefined;
        const page = await reader.agent.messages({ limit: this.limit, after: history?.checkpoint }, { signal });
        if (signal.aborted) return;
        if (page.hasMore && page.checkpoint.position === (history?.checkpoint.position ?? '0')) throw new Error('消息游标没有前进');
        const messages = history?.messages ?? new Map<string, Message>();
        appendMessages(messages, page.messages);
        reader.history = { mode: 'forward', snapshot, checkpoint: page.checkpoint, hasMore: page.hasMore, messages };
      }
      reader.phase = 'ready';
    } catch (error) {
      if (signal.aborted) return;
      reader.phase = 'error'; reader.error = issueOf(error);
    } finally {
      if (!signal.aborted) { reader.pending = false; this.changed(); }
    }
  }

  private get historyLimit() { return Math.min(historyPageLimit, this.scope.descriptor.max_page); }

  private async readOlder(reader: ExactReader, history: LatestHistory) {
    const before = history.before, signal = reader.lease.signal, lane = reader.older;
    if (!before || lane.pending || signal.aborted) return;
    lane.pending = true; lane.phase = 'loading'; lane.error = undefined; this.changed();
    try {
      const page = await reader.agent.messageHistory({ limit: this.historyLimit, before }, { signal });
      if (signal.aborted) return;
      if (page.nextBefore?.position === before.position) throw new Error('更早消息游标没有前进');
      const older = new Map<string, Message>();
      appendMessages(older, page.messages.filter(message => !history.messages.has(message.id)));
      history.emptyOlder = older.size === 0;
      // Re-read the shared map here: a live response may have appended while this page was pending.
      if (older.size) {
        for (const [id, message] of history.messages) older.set(id, message);
        history.messages = older;
      }
      history.before = page.nextBefore;
      lane.phase = 'ready';
    } catch (error) {
      if (!signal.aborted) { lane.phase = 'error'; lane.error = issueOf(error); }
    } finally {
      if (!signal.aborted) { lane.pending = false; this.changed(); }
    }
  }

  private async readLive(reader: ExactReader, history: LatestHistory) {
    const signal = reader.lease.signal, lane = reader.live;
    if (lane.pending || signal.aborted) return;
    lane.pending = true;
    try {
      const page = await reader.agent.messages({ limit: this.historyLimit, after: history.checkpoint ?? undefined }, { signal });
      if (signal.aborted) return;
      if (page.hasMore && page.checkpoint.position === (history.checkpoint?.position ?? '0')) throw new Error('新消息游标没有前进');
      history.arrivals += appendMessages(history.messages, page.messages);
      history.checkpoint = page.checkpoint;
      lane.phase = 'ready'; lane.error = undefined;
    } catch (error) {
      if (!signal.aborted) { lane.phase = 'error'; lane.error = issueOf(error); }
    } finally {
      if (!signal.aborted) { lane.pending = false; this.changed(); }
    }
  }

  /** Validate the submitted ID against its original lease, independent of pane focus. */
  async verifySend(id: string) {
    const reader = this.readerSet.get(id);
    if (!reader?.snapshot || reader.agent.kind !== 'current') throw new Error('请先验证当前精确会话');
    const signal = reader.lease.signal;
    const snapshot = await reader.agent.snapshot({ signal });
    if (signal.aborted || !this.belongs(snapshot, reader.office) || !snapshot.facts.can_send) throw new Error('精确目标或发送能力已失效');
    return { snapshot, agent: reader.agent, signal };
  }

  dispose() {
    this.closed = true;
    this.discovery?.abort(); this.discovery = undefined;
    this.exploration?.request?.abort();
    this.clearReaders();
  }
}

function appendMessages(records: Map<string, Message>, messages: readonly Message[]) {
  let added = 0;
  for (const message of messages) if (!records.has(message.id)) { records.set(message.id, message); added++; }
  return added;
}
