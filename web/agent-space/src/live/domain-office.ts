import { DomainCapabilityError, ManagementError, type Agent, type AgentAddress, type AgentSnapshot, type AgentXScope, type Message, type MessageCursor, type MessageHistoryCursor, type NativeMemberCursor, type Workspace, type WorkspaceCatalog } from '@agentx/client';
import { GLOBAL_OFFICE, LEGACY_OFFICE, UNKNOWN_OFFICE } from '../offline/model';
import { domainAgentKey } from './projection';
import type { HistoryReadPage, ReadState } from '../conversation/read-state';

export interface OfficeExploration extends ReadState {
  office: string; workspace: Workspace; rows: Map<string, AgentSnapshot>;
  request?: AbortController;
  next?: NativeMemberCursor | null; currentComplete: boolean; revision: number;
}
type Lane = ReadState & { pending?: boolean };
interface ExactReader extends ReadState {
  id: string; office: string; agent: Agent; subject: AgentSnapshot; snapshot?: AgentSnapshot;
  lease: AbortController; pending: boolean;
  stale?: boolean;
  older: Lane; live: Lane;
  history?: ReadHistory; restoring?: ReadHistory;
}
type ReadHistory = {
  snapshot: AgentSnapshot; messages: Map<string, Message>;
  before: MessageHistoryCursor | null; checkpoint: MessageCursor;
  token: object; arrivals: number; emptyOlder: boolean;
};
const historyPageLimit = 20;

// Match the window shelf bound without depending on the presentation layer.
const retainedReaderLimit = 8;
const sameMembership = (a: AgentSnapshot['membership'], b: AgentSnapshot['membership']) =>
  a.kind === b.kind && (a.kind !== 'adopted' || (b.kind === 'adopted' && a.workspace === b.workspace));

const issueOf = (error: unknown) => error instanceof Error ? error.message : String(error);
const staleAgent = (error: unknown) => error instanceof ManagementError && error.code === 'StaleReference';
const unsupportedHistory = (error: unknown) => error instanceof DomainCapabilityError && error.capability === 'message-history';

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
  constructor(readonly scope: AgentXScope, private limit: number, private changed: () => void, private invalidated: (error: unknown) => boolean = () => false) {}
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
      if (!request.signal.aborted && !this.invalidated(error)) this.catalogError = issueOf(error);
    } finally {
      if (this.discovery === request) { this.discovery = undefined; this.changed(); }
    }
  }

  choose(office: string) {
    if (this.closed) return;
    if (this.office !== office) {
      this.office = office;
      for (const id of this.readerSet.keys()) this.parkReader(id);
    }
    if (this.exploration?.office === office) return;
    const workspace = office === GLOBAL_OFFICE ? this.scope.workspace('global') : this.catalog?.workspaces.find(entry => entry.workspace.address.key === office)?.workspace;
    this.exploration?.request?.abort();
    this.exploration = workspace && this.newExploration(office, workspace);
    if (this.exploration) void this.readMembers(this.exploration);
  }

  more = () => {
    const office = this.exploration;
    if (office && (office.phase === 'error' || office.next)) void this.readMembers(office);
  };

  private newExploration(office: string, workspace: Workspace): OfficeExploration {
    return { office, workspace, rows: new Map(), phase: 'loading', currentComplete: false, revision: ++this.revision };
  }

  refresh = () => {
    const old = this.exploration;
    if (!old || this.closed || old.phase === 'unavailable') return;
    old.request?.abort();
    const next = this.exploration = this.newExploration(old.office, old.workspace);
    void this.readMembers(next);
  };

  private async readMembers(office: OfficeExploration) {
    if (office.request || this.closed || office.next === null) return;
    const request = office.request = new AbortController();
    office.phase = 'loading'; office.error = undefined; this.changed();
    try {
      const page = await office.workspace.memberPage({ limit: this.limit, after: office.next ?? undefined }, { signal: request.signal });
      if (request.signal.aborted || this.exploration !== office) return;
      for (const { snapshot } of page.members) office.rows.set(domainAgentKey(snapshot.address), snapshot);
      office.next = page.coverage.next;
      office.currentComplete = page.coverage.currentComplete;
      office.phase = 'ready'; office.revision = ++this.revision;
    } catch (error) {
      if (request.signal.aborted || this.exploration !== office) return;
      if (this.invalidated(error)) return;
      if (error instanceof DomainCapabilityError && error.capability === 'workspace-members') {
        office.phase = 'unavailable'; office.error = '此服务不支持成员列表';
        office.next = null; office.currentComplete = false;
      } else { office.phase = 'error'; office.error = issueOf(error); }
    } finally {
      if (!request.signal.aborted && this.exploration === office) { office.request = undefined; this.changed(); }
    }
  }

  /** Resolve an exact route/related subject without admitting it to the workspace roster. */
  async resolve(address: AgentAddress, signal?: AbortSignal, office?: string, current: () => boolean = () => true) {
    const agent = this.scope.agent(address);
    const snapshot = await this.readSnapshot(agent, signal);
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
      void this.openHistory(reader);
    }
    while (this.readerSet.size + this.parked.size > retainedReaderLimit) this.parked.delete(this.parked.keys().next().value!);
    return changed;
  }

  private parkReader(id: string) {
    const reader = this.readerSet.get(id)!;
    this.readerSet.delete(id);
    reader.lease.abort();
    const history = reader.stale ? undefined : reader.history ?? reader.restoring;
    if (history) this.parked.set(id, history);
  }

  private clearReaders() {
    for (const reader of this.readerSet.values()) reader.lease.abort();
    this.readerSet.clear(); this.parked.clear();
  }

  messagePage(id: string): HistoryReadPage | undefined {
    const reader = this.readerSet.get(id);
    if (!reader) return;
    const history = reader.history;
    return { phase: reader.phase, error: reader.error, more: !reader.stale && !!history?.before,
      older: { phase: reader.older.phase, error: reader.older.error },
      live: { phase: reader.live.phase, error: reader.live.error },
      token: history?.token, arrivals: history?.arrivals, emptyOlder: history?.emptyOlder };
  }

  continueMessages = (id: string) => {
    const reader = this.readerSet.get(id);
    if (!reader || reader.stale) return;
    if (reader.history) void this.readOlder(reader, reader.history);
    else if (reader.phase === 'error') void this.openHistory(reader);
  };

  retryLive(id: string) {
    const reader = this.readerSet.get(id);
    if (reader?.agent.kind === 'current' && !reader.stale && reader.history) void this.readLive(reader, reader.history);
  }

  /** One bounded update per visible current reader; older pages never gate its live cursor. */
  catchUp() {
    for (const id of this.readerSet.keys()) this.retryLive(id);
  }

  private async openHistory(reader: ExactReader) {
    const signal = reader.lease.signal;
    if (reader.pending || signal.aborted) return;
    reader.pending = true;
    if (reader.phase === 'error') { reader.phase = 'loading'; reader.error = undefined; this.changed(); }
    try {
      if (!reader.snapshot) {
        const snapshot = await this.readSnapshot(reader.agent, signal);
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
      const page = await reader.agent.messageHistory({ limit: this.historyLimit }, { signal });
      if (signal.aborted) return;
      if (!page.liveAfter) throw new Error('缺少当前消息游标');
      reader.history = { snapshot: reader.snapshot, token: {}, arrivals: 0, emptyOlder: false,
        before: page.nextBefore, checkpoint: page.liveAfter, messages: new Map(page.messages.map(message => [message.id, message])) };
      reader.phase = 'ready';
    } catch (error) {
      if (signal.aborted || this.invalidated(error)) return;
      if (unsupportedHistory(error)) {
        reader.phase = 'unavailable'; reader.error = '此服务不支持消息历史';
      } else {
        this.noteStale(reader, error);
        reader.phase = reader.stale ? 'unavailable' : 'error';
        reader.error = reader.stale ? '此 Agent 实例已失效' : issueOf(error);
      }
    } finally {
      if (!signal.aborted) { reader.pending = false; this.changed(); }
    }
  }

  private get historyLimit() { return Math.min(historyPageLimit, this.scope.descriptor.max_page); }

  private async readOlder(reader: ExactReader, history: ReadHistory) {
    const before = history.before, signal = reader.lease.signal, lane = reader.older;
    if (!before || lane.pending || signal.aborted) return;
    lane.pending = true; lane.phase = 'loading'; lane.error = undefined; this.changed();
    try {
      const page = await reader.agent.messageHistory({ limit: this.historyLimit, before }, { signal });
      if (signal.aborted || reader.stale) return;
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
      if (signal.aborted || this.invalidated(error)) return;
      if (this.noteStale(reader, error)) {
        history.before = null; lane.phase = 'ready';
      } else if (unsupportedHistory(error)) {
        history.before = null; lane.phase = 'unavailable'; lane.error = '此服务不支持读取更早消息';
      } else { lane.phase = 'error'; lane.error = issueOf(error); }
    } finally {
      if (!signal.aborted) { lane.pending = false; this.changed(); }
    }
  }

  private async readLive(reader: ExactReader, history: ReadHistory) {
    const signal = reader.lease.signal, lane = reader.live;
    if (lane.pending || signal.aborted) return;
    lane.pending = true;
    try {
      const page = await reader.agent.messages({ limit: this.historyLimit, after: history.checkpoint }, { signal });
      if (signal.aborted || reader.stale) return;
      if (page.hasMore && page.checkpoint.position === history.checkpoint.position) throw new Error('新消息游标没有前进');
      history.arrivals += appendMessages(history.messages, page.messages);
      history.checkpoint = page.checkpoint;
      lane.phase = 'ready'; lane.error = undefined;
    } catch (error) {
      if (signal.aborted || this.invalidated(error)) return;
      this.noteStale(reader, error);
      lane.phase = reader.stale ? 'unavailable' : 'error';
      lane.error = reader.stale ? '此 Agent 实例已失效' : issueOf(error);
    } finally {
      if (!signal.aborted) { lane.pending = false; this.changed(); }
    }
  }

  /** Validate the submitted ID against its original lease, independent of pane focus. */
  async verifySend(id: string) {
    const reader = this.readerSet.get(id);
    if (!reader?.snapshot || reader.stale || reader.agent.kind !== 'current') throw new Error('请先验证当前精确会话');
    const signal = reader.lease.signal;
    try {
      const snapshot = await this.readSnapshot(reader.agent, signal);
      if (signal.aborted || reader.stale || !this.belongs(snapshot, reader.office) || !snapshot.facts.can_send) throw new Error('精确目标或发送能力已失效');
      return { snapshot, agent: reader.agent, signal };
    } catch (error) {
      if (!signal.aborted && this.noteStale(reader, error)) this.changed();
      throw error;
    }
  }

  private async readSnapshot(agent: Agent, signal?: AbortSignal) {
    try { return await agent.snapshot({ signal }); }
    catch (error) {
      if (!signal?.aborted) this.invalidated(error);
      throw error;
    }
  }

  private noteStale(reader: ExactReader, error: unknown) {
    if (!staleAgent(error)) return false;
    reader.stale = true;
    reader.live.phase = 'unavailable'; reader.live.error = '此 Agent 实例已失效';
    return true;
  }

  dispose() {
    this.closed = true;
    this.discovery?.abort(); this.discovery = undefined;
    this.exploration?.request?.abort();
    if (this.exploration) this.exploration.next = null;
    this.clearReaders();
  }
}

function appendMessages(records: Map<string, Message>, messages: readonly Message[]) {
  let added = 0;
  for (const message of messages) if (!records.has(message.id)) { records.set(message.id, message); added++; }
  return added;
}
