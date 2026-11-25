/** Native read/action lifecycle; scene state and drafts remain with SpaceController. */
import type { Attachment, AgentRef, AgentView, Descriptor, MessageInput } from '../../../management/api/native';
import { DomainGenerationMismatch, ManagementError, selectAgentXScope, type AgentAddress, type AgentCursor, type AgentSnapshot, type DirectoryPreview, type InboxCursor, type InboxPage, type LaunchDraft, type LaunchRelationCursor, type OfficeActivityCursor, type OperationReceipt, type ReadonlyValue, type RelatedConversation, type Transcript, type TypedOperationReceipt } from '@agentx/client';
import { hostSelection, type SpaceHostBinding } from './host';
import { DomainOfficeReads } from './domain-office';
import { humanPageSize, mailboxHead } from './mailbox';
import { loadJournal, makeReceipt, persistJournal, restoreCommand, type Kind, type Receipt, type SpaceCommand } from './operation-journal';
import { requestId } from '../../../management/model';
import { GLOBAL_OFFICE, LEGACY_OFFICE, UNKNOWN_OFFICE, HUMAN_ENDPOINT } from '../offline/model';
import type { AgentSpaceSupply, Post, SuppliedRecords, SceneMessage } from '../offline/model';
import { domainCoverage, domainAgentKey, domainReference, nativeSnapshot, emptySupply, messageRecords, projectAgent, projectPosts, transcriptRecords } from './projection';

export type { Receipt } from './operation-journal';
type CommandOutcome = { receipt: OperationReceipt; transcript?: Transcript };
const transcriptOutcome = (response: TypedOperationReceipt<'transcript'>): CommandOutcome => ({
  receipt: response.receipt, transcript: response.result ?? undefined,
});
const issueOf = (error: unknown) => error instanceof Error ? error.message : String(error);
const unresolved = (receipt: Receipt) => !receipt.operation || ['Accepted', 'Running', 'Unknown'].includes(receipt.operation.status);
/** Describe original receipt outcomes without turning uncertainty into definite non-execution. */
export function operationIssue(receipt: Receipt): string | undefined {
  const operation = receipt.operation;
  if (operation?.status === 'Unknown') return `结果未确定：${operation.error ?? '请查询原回执，勿重复提交。'}`;
  if (operation?.status === 'Failed') return `操作失败：${operation.error ?? '原生操作失败'}`;
  return receipt.issue;
}

/** Own one mounted real supply; no operation is automatically submitted or repeated. */
export class LiveSpace {
  readonly kind = 'native';
  descriptor?: ReadonlyValue<Descriptor>;
  supply: AgentSpaceSupply = emptySupply();
  receipts = loadJournal();
  error = '';
  actionError = '';
  roleError = '';
  more = false;
  entry?: DirectoryPreview;
  entryError = '';
  entryInput = '';
  connection: 'connecting' | 'pinned' | 'failed' | 'reset' = 'connecting';
  private domain?: DomainOfficeReads;
  private officeRevision = 0;
  private overviewRead?: AbortController;
  private overview = new Map<string, AgentSnapshot>();
  private navigationEpoch = 0;
  private records = new Map<string, SuppliedRecords>();
  private posts: Post[] = [];
  private office = '';
  private desiredReaders: readonly string[] = [];
  private selected: string | null = null;
  private population = { revision: 0, complete: false };
  private activityCursor?: OfficeActivityCursor;
  private activity: SceneMessage[] = [];
  private arrivals = new Map<string, { event: string; at: number }>();
  activityError = '';
  private human?: AgentSpaceSupply['human'];
  private humanBefore?: InboxCursor;
  private humanNext?: InboxCursor;
  private humanPending = true;
  private humanRevision = 0;
  private humanBusy = false;
  private humanHead?: string;
  private pages = 1;
  private entryEpoch = 0;
  private timer?: ReturnType<typeof setTimeout>;
  private abort?: AbortController;
  private refreshing = false;
  private busyLookups = new Set<string>();
  private completions = new Map<string, () => void>();
  constructor(private publish: (supply: AgentSpaceSupply) => void, readonly host?: SpaceHostBinding) {}
  start() {
    const lifetime = this.abort = new AbortController();
    const onVisibility = () => {
      clearTimeout(this.timer);
      if (!document.hidden) void this.refresh();
    };
    document.addEventListener('visibilitychange', onVisibility, { signal: lifetime.signal });
    onVisibility();
    return () => {
      lifetime.abort(); this.overviewRead?.abort(); this.domain?.dispose(); this.domain = undefined;
      this.descriptor = undefined; this.connection = 'connecting';
      this.overview.clear(); this.desiredReaders = []; this.selected = null;
      this.refreshing = false; clearTimeout(this.timer); this.navigationEpoch++; this.entryEpoch++;
    };
  }
  private get focused() { return this.domain?.exploration; }
  private get listener() { return this.domain?.scope.address.baseUrl; }
  private get rosterSnapshots() { return this.focused?.rows ?? (this.usesOverview ? this.overview : undefined); }
  get catalogError() { return this.domain?.catalogError ?? ''; }
  retryCatalog = () => { void this.domain?.discover(); };
  moreMembers = () => { this.domain?.more(); };
  refreshMembers = () => { this.domain?.refresh(); };
  moreMessagesFor = (id: string) => { this.domain?.continueMessages(id); };
  messagePage(id: string) { return this.domain?.messagePage(id); }
  retryMessagesLive = (id: string) => { this.domain?.retryLive(id); };
  ownsReceipt(receipt: Receipt) {
    const listener = this.listener;
    const sameListener = !!listener && (receipt.listener === listener || receipt.listener === undefined && this.host?.embedded);
    return !!sameListener && receipt.scope === this.descriptor?.scope && receipt.body.generation === this.descriptor?.generation;
  }
  canLookup(receipt: Receipt) { return this.connection !== 'reset' && this.ownsReceipt(receipt); }
  private emit(selectionOnly = false) {
    const descriptor = this.descriptor;
    const domain = this.domain, focused = this.focused;
    const reset = this.connection === 'reset';
    const readersChanged = this.connection === 'pinned' && domain?.selectReaders(this.desiredReaders, id => this.observed(id));
    if (selectionOnly && !readersChanged) return;
    const records = new Map(this.records);
    const rows = [...this.rosterSnapshots?.values() ?? []].map(nativeSnapshot);
    const subjects = new Map<string, ReadonlyValue<AgentView>>();
    if (domain) {
      for (const snapshot of domain.subjects.values()) subjects.set(domainAgentKey(snapshot.address), nativeSnapshot(snapshot));
      for (const snapshot of domain.parkedSubjects()) {
        const id = domainAgentKey(snapshot.address);
        if (!subjects.has(id) && !this.rosterSnapshots?.has(id)) subjects.set(id, nativeSnapshot(snapshot));
      }
      for (const reader of domain.readers.values()) {
        const snapshot = reader.snapshot ?? reader.subject;
        subjects.set(reader.id, nativeSnapshot(snapshot));
        records.set(`${reader.id}:messages`, reader.history
          ? messageRecords([...reader.history.messages.values()])
          : reader.phase === 'unavailable' ? { status: 'unavailable', reason: reader.error ?? '消息历史不可用' }
          : reader.phase === 'error' ? { status: 'error', reason: reader.error ?? '读取失败' } : { status: 'loading' });
      }
    }
    const project = (agent: ReadonlyValue<AgentView>) => {
      const resource = projectAgent(agent, descriptor!);
      const arrival = this.arrivals.get(resource.id);
      return arrival && Date.now() - arrival.at < 6000 ? { ...resource, arrival: arrival.event } : resource;
    };
    const workspaces = domain?.catalog?.workspaces.map(({ workspace, sources }) => ({ path: workspace.address.key, sources }));
    this.supply = descriptor ? {
      mode: 'live', scope: { id: descriptor.scope, generation: descriptor.generation, listener: this.listener, coverage: domainCoverage(domain?.catalog, domain?.catalogError ?? '') },
      workspaces: workspaces?.map(office => ({ id: office.path, title: office.path.split('/').filter(Boolean).at(-1) ?? office.path, origin: office.sources.includes('role-marker') ? 'role-marker' : 'explicit-membership', sources: office.sources })) ?? [],
      activity: this.activity, human: this.human, population: focused ? { revision: focused.revision, complete: focused.phase === 'ready' && focused.currentComplete } : this.population,
      // A reset ends native leases, but the last published identities still
      // anchor the visible drafts (including readers absent during roster refresh).
      agents: reset ? this.supply.agents : rows.map(project),
      subjects: reset ? this.supply.subjects : [...subjects.values()].map(project),
      posts: this.posts,
      roster: !reset && focused ? { office: focused.office, phase: focused.phase, error: focused.error, more: focused.next != null } : undefined,
      records: (id, channel) => reset ? { status: 'unavailable', reason: '连接已失效' }
        : records.get(`${id}:${channel}`) ?? { status: 'unavailable', reason: channel === 'model' ? '请主动读取 Provider Transcript' : '消息读取中' },
    } : emptySupply();
    this.publish(this.supply);
  }
  private get options() { return { signal: this.abort?.signal }; }
  private current(scope: string, generation: string) { return !this.abort?.signal.aborted && this.connection !== 'reset' && this.descriptor?.scope === scope && this.descriptor.generation === generation; }
  /** Stop the old read session when a resource, rather than a preflight, reports a reset. */
  private invalidated = (error: unknown) => {
    if (!(error instanceof DomainGenerationMismatch) && !(error instanceof ManagementError && error.code === 'StaleGeneration')) return false;
    if (this.connection === 'pinned') {
      this.connection = 'reset'; this.error = '作用域或数据代次已变化；请重新连接';
      this.abort?.abort(); this.overviewRead?.abort(); this.domain?.dispose();
      this.refreshing = false; clearTimeout(this.timer);
      this.more = false; this.records.clear(); this.activityCursor = undefined; this.activity = [];
      this.arrivals.clear(); this.human = undefined; this.humanBefore = undefined; this.humanNext = undefined;
      this.entry = undefined; this.completions.clear();
      this.emit();
    }
    return true;
  };
  refreshData = () => {
    if (this.connection === 'pinned') {
      this.retryCatalog(); this.refreshMembers(); void this.readRoles();
    }
    void this.refresh();
  };
  async refresh() {
    if (this.refreshing || this.abort?.signal.aborted || this.connection === 'reset') return;
    this.refreshing = true; clearTimeout(this.timer);
    const lifetime = this.abort;
    const alive = () => this.abort === lifetime && !lifetime?.signal.aborted;
    try {
      if (!this.domain) {
        this.connection = 'connecting'; this.error = ''; this.emit();
        const { selection, pageLimit } = hostSelection(this.host);
        const scope = await selectAgentXScope(selection, { signal: lifetime?.signal });
        if (!alive()) return;
        const domain = this.domain = new DomainOfficeReads(scope, Math.min(pageLimit, scope.descriptor.max_page), () => {
          if (!alive() || this.connection !== 'pinned' || this.domain !== domain) return;
          domain.choose(this.office); this.emit();
        }, this.invalidated);
        this.descriptor = scope.descriptor;
        this.connection = 'pinned'; this.error = '';
        domain.choose(this.office); this.emit();
        await domain.discover();
      }
      if (!alive() || this.connection !== 'pinned' || !this.descriptor) return;
      const descriptor = this.descriptor, officeRevision = this.officeRevision;
      // A known office uses only its bounded Workspace provider. Unknown route offices wait for discovery.
      const overviewRead = this.usesOverview ? this.overviewRead = new AbortController() : undefined;
      const signal = lifetime && overviewRead ? AbortSignal.any([lifetime.signal, overviewRead.signal]) : overviewRead?.signal;
      const feed = overviewRead ? this.readOverview(signal).then(result => {
        if (!this.current(descriptor.scope, descriptor.generation) || !alive() || overviewRead.signal.aborted || officeRevision !== this.officeRevision) return;
        this.overview = new Map(result.rows.map(row => [domainAgentKey(row.address), row]));
        this.more = result.more;
        this.population = { revision: this.population.revision + 1, complete: result.complete };
        this.emit();
      }).catch(error => {
        if (alive() && !overviewRead.signal.aborted && !this.invalidated(error)) throw error;
      }) : undefined;
      const results = await Promise.allSettled([feed, this.readActivity(descriptor)]);
      if (this.overviewRead === overviewRead) this.overviewRead = undefined;
      if (!alive() || this.connection !== 'pinned') return;
      this.error = results[0].status === 'rejected' && officeRevision === this.officeRevision ? issueOf(results[0].reason) : '';
      this.emit();
      if (!this.posts.length) await this.readRoles();
      if (!document.hidden) this.domain?.catchUp();
      for (const receipt of this.receipts.filter(item => this.ownsReceipt(item) && unresolved(item) && item.operation?.status !== 'Unknown')) await this.lookup(receipt);
    } catch (error) {
      if (!alive()) return;
      if (!this.domain) this.connection = 'failed';
      this.error = issueOf(error); this.emit();
    } finally {
      if (alive()) {
        this.refreshing = false;
        if (this.connection === 'pinned' && !document.hidden) this.timer = setTimeout(() => { if (!document.hidden) void this.refresh(); }, 3000);
      }
    }
  }
  private async readActivity(descriptor: ReadonlyValue<Descriptor>) {
    const scope = this.domain!.scope, humanRevision = this.humanRevision;
    try {
      const baseline = this.activityCursor === undefined;
      const page = await scope.officeActivity({ limit: 100, after: this.activityCursor }, this.options);
      if (!this.current(descriptor.scope, descriptor.generation)) return;
      this.activityCursor = page.checkpoint; this.activityError = '';
      const roster = [...this.rosterSnapshots?.values() ?? []].map(nativeSnapshot);
      const refs = new Map(roster.filter(row => row.agent.incarnation !== null).map(row => [JSON.stringify([row.agent.name, row.agent.incarnation, row.agent.stopped_event]), projectAgent(row, descriptor).id]));
      const key = (ref: AgentRef) => refs.get(JSON.stringify([ref.name, ref.incarnation, ref.stopped_event]));
      const now = Date.now();
      if (!baseline && !page.has_more && !document.hidden) {
        for (const ref of page.created) { const id = key(ref); if (id) this.arrivals.set(id, { event: `${page.checkpoint.position}:${id}`, at: now }); }
        const activity: SceneMessage[] = [];
        for (const event of page.messages) {
          const from = event.human_sender ? HUMAN_ENDPOINT : event.sender && key(event.sender);
          if (!from || now - Date.parse(event.at) > 10000) continue;
          if (event.human_recipient && from !== HUMAN_ENDPOINT) activity.push({ id: `${event.id}:human`, from, to: HUMAN_ENDPOINT, receivedAt: now });
          for (const recipient of event.recipients) { const to = key(recipient); if (to && to !== from) activity.push({ id: `${event.id}:${to}`, from, to, receivedAt: now }); }
        }
        this.activity = [...this.activity.filter(item => now - item.receivedAt < 4000), ...activity].slice(-24);
      } else { this.activity = []; this.arrivals.clear(); }
      for (const [id, arrival] of this.arrivals) if (now - arrival.at >= 6000 || !roster.some(row => projectAgent(row, descriptor).id === id)) this.arrivals.delete(id);
      if (this.humanBusy || humanRevision !== this.humanRevision) return;
      if (this.humanBefore) {
        if (this.human) this.human = { ...this.human, unhandledCount: page.human_unhandled_count };
        return;
      }
      const head = mailboxHead(page, this.humanPending);
      if (head) {
        this.humanHead = undefined;
        this.acceptHuman(descriptor, head);
      } else {
        // A full pending page may extend past the unfiltered prefix. Appends
        // and monotone handling change the prefix or count before it gets stale.
        const hint = JSON.stringify([descriptor.generation, humanRevision, page.human_unhandled_count.toString(), page.human_messages.map(message => [message.id, message.handled_at])]);
        if (hint !== this.humanHead && await this.readHuman(descriptor)) this.humanHead = hint;
      }
    } catch (error) {
      if (!this.current(descriptor.scope, descriptor.generation) || this.invalidated(error)) return;
      this.activityCursor = undefined; this.activity = []; this.arrivals.clear(); this.activityError = issueOf(error);
      this.humanHead = undefined;
      if (!this.humanBefore && humanRevision === this.humanRevision) await this.readHuman(descriptor);
    }
  }
  private async readHuman(descriptor: ReadonlyValue<Descriptor>) {
    const revision = this.humanRevision, scope = this.domain?.scope;
    if (!scope || this.humanBusy) return false;
    try {
      const page = await scope.inbox.list({ limit: humanPageSize, pending: this.humanPending, before: this.humanBefore }, this.options);
      if (!this.current(descriptor.scope, descriptor.generation) || revision !== this.humanRevision) return false;
      this.acceptHuman(descriptor, page);
      return true;
    } catch (error) {
      if (this.invalidated(error)) return false;
      if (this.current(descriptor.scope, descriptor.generation) && revision === this.humanRevision) this.human = { ...this.human, messages: this.human?.messages ?? [], hasMore: false, error: issueOf(error), history: this.humanHistory, page: this.humanPage };
      return false;
    }
  }
  private acceptHuman(descriptor: ReadonlyValue<Descriptor>, page: InboxPage) {
    this.humanNext = page.next ?? undefined;
    this.human = { messages: page.messages, hasMore: page.next !== null, unhandledCount: page.unhandledCount,
      pending: this.humanPending, before: this.humanBefore?.position, nextBefore: this.humanNext?.position, older: !!this.humanBefore, handle: this.handleHuman, history: this.humanHistory, page: this.humanPage, openSender: reference => this.openReference(reference, descriptor) };
  }
  /** Restore mailbox read filters/cursor; idempotent and GET-only, never handles letters. */
  mailbox = (pending: boolean, before: string) => {
    const scope = this.domain?.scope.address;
    if (!scope || !this.current(scope.scope, scope.generation) || !this.descriptor || this.humanBusy || pending === this.humanPending && (this.humanBefore?.position ?? '') === before) return;
    this.humanPending = pending; this.humanBefore = before ? { scope, kind: 'human-inbox', pending, position: before } : undefined; this.humanRevision++;
    this.human = undefined; this.emit();
    void this.readHuman(this.descriptor).then(() => this.emit());
  };
  private humanHistory = (pending: boolean) => {
    if (this.humanBusy || !this.descriptor) return;
    this.humanPending = pending; this.humanBefore = undefined; this.humanRevision++;
    this.human = undefined; this.emit();
    void this.readHuman(this.descriptor).then(() => this.emit());
  };
  private humanPage = (older: boolean) => {
    if (this.humanBusy || !this.descriptor || (older && !this.humanNext)) return;
    this.humanBefore = older ? this.humanNext : undefined; this.humanRevision++;
    this.human = undefined; this.emit();
    void this.readHuman(this.descriptor).then(() => this.emit());
  };
  private handleHuman = async (ids: readonly string[]) => {
    const descriptor = this.descriptor, scope = this.domain?.scope;
    if (!scope || !descriptor || this.connection === 'reset' || this.humanBusy || this.human?.error || !ids.length) return;
    const eligible = new Set(this.human?.messages.filter(message => !message.handled_at).map(message => message.id));
    if (ids.some(id => !eligible.has(id))) return;
    this.humanRevision++; this.humanBusy = true;
    this.human = { ...this.human!, busy: true, actionError: undefined }; this.emit();
    try {
      await scope.inbox.handle(ids, this.options);
      if (!this.current(descriptor.scope, descriptor.generation)) return;
      this.humanBusy = false;
      await this.readHuman(descriptor);
    } catch (error) {
      if (this.invalidated(error)) return;
      if (this.current(descriptor.scope, descriptor.generation)) {
        this.humanBusy = false;
        await this.readHuman(descriptor);
        if (this.human) this.human = { ...this.human, actionError: `处理结果请以列表为准，可安全重试：${issueOf(error)}` };
      }
    } finally {
      if (this.current(descriptor.scope, descriptor.generation)) { this.humanBusy = false; if (this.human) this.human = { ...this.human, busy: false }; this.emit(); }
    }
  };
  private get usesOverview() { return ['', LEGACY_OFFICE, UNKNOWN_OFFICE].includes(this.office); }
  private async readOverview(signal?: AbortSignal) {
    const scope = this.domain!.scope;
    const rows: AgentSnapshot[] = []; let after: AgentCursor | null = null; let complete = false;
    for (let page = 0; page < this.pages; page++) {
      const result = await scope.agents({ limit: scope.descriptor.max_page, after: after ?? undefined }, { signal });
      rows.push(...result.agents); after = result.next; complete = result.currentComplete;
      if (!after) break;
    }
    return { rows, more: after !== null, complete };
  }
  loadMore = () => { if (this.focused) { this.moreMembers(); return; } this.pages++; void this.refresh(); };
  chooseOffice(office: string) {
    if (this.office === office) return;
    this.office = office; this.officeRevision++; this.posts = []; this.entry = undefined; this.entryError = ''; this.entryEpoch++;
    this.overviewRead?.abort();
    this.more = false; this.pages = 1; this.desiredReaders = []; this.selected = null; this.navigationEpoch++;
    this.domain?.choose(office);
    if (this.domain && this.usesOverview) void this.refresh();
    this.emit(); void this.readRoles();
  }
  private async readRoles() {
    const descriptor = this.descriptor, office = this.office, domain = this.domain, revision = this.officeRevision;
    if (!descriptor || !domain || this.connection === 'reset') return;
    if (office && office !== '$global' && !office.startsWith('/')) { this.posts = []; this.roleError = ''; this.emit(); return; }
    try {
      const catalog = await domain.scope.workspace(office.startsWith('/') ? office : 'global').roles(this.options);
      if (!this.current(descriptor.scope, descriptor.generation) || this.officeRevision !== revision) return;
      this.posts = projectPosts(catalog); this.roleError = '';
    } catch (error) { if (this.invalidated(error)) return; if (this.current(descriptor.scope, descriptor.generation) && this.officeRevision === revision) { this.posts = []; this.roleError = issueOf(error); } }
    this.emit();
  }
  async launchRelations(id: string, after?: LaunchRelationCursor, signal?: AbortSignal) {
    const domain = this.domain, descriptor = this.descriptor, snapshot = this.observed(id);
    if (!domain || !descriptor || !snapshot || !this.current(descriptor.scope, descriptor.generation)) throw new Error('精确 Agent 已不可用');
    const agent = domain.scope.agent(snapshot.address);
    const result = await agent.launchRelations({ limit: Math.min(5, domain.scope.descriptor.max_page), after }, { signal: this.readSignal(signal) })
      .catch(error => { this.invalidated(error); throw error; });
    if (this.domain !== domain || !this.current(descriptor.scope, descriptor.generation)) throw new Error('关系数据代次已变化');
    return result;
  }
  async openRelated(target: RelatedConversation, source: { scope: string; generation: string }, signal?: AbortSignal) {
    if (target.availability !== 'available') throw new Error('精确会话未保留');
    return this.openAddress(target.agent.address, source, signal);
  }
  async openReference(reference: AgentRef, source: { scope: string; generation: string }, signal?: AbortSignal, office?: string) {
    if (!this.domain) throw new Error('精确会话未保留');
    return this.openAddress({ scope: this.domain.scope.address, reference: domainReference(reference) }, source, signal, office);
  }
  private readSignal(signal?: AbortSignal) {
    const lifetime = this.options.signal;
    return lifetime && signal ? AbortSignal.any([lifetime, signal]) : signal ?? lifetime;
  }
  private async openAddress(address: AgentAddress, source: { scope: string; generation: string }, signal?: AbortSignal, office?: string) {
    const descriptor = this.descriptor, epoch = ++this.navigationEpoch;
    if (!descriptor) throw new Error('精确会话未保留');
    if (!this.current(source.scope, source.generation)) throw new Error('关系来源 scope / generation 已失效');
    const domain = this.domain;
    if (!domain) throw new Error('精确会话未保留');
    const snapshot = await domain.resolve(address, this.readSignal(signal), office, () => epoch === this.navigationEpoch && this.domain === domain && this.current(descriptor.scope, descriptor.generation));
    const resource = projectAgent(nativeSnapshot(snapshot), descriptor);
    this.emit();
    return resource;
  }
  select(id: string | null) {
    this.selectReaders(id ? [id] : [], id);
  }
  /** The first two distinct visible exact IDs get leases; focus is independent. */
  selectReaders(ids: readonly string[], activeId: string | null) {
    const desired = [...new Set(ids)].slice(0, 2);
    const active = activeId && desired.includes(activeId) ? activeId : null;
    if (this.selected !== active) { this.selected = active; this.navigationEpoch++; }
    if (desired.length === this.desiredReaders.length && desired.every((id, i) => id === this.desiredReaders[i])) return;
    this.desiredReaders = desired;
    this.emit(true);
  }
  private observed(id: string) {
    return this.focused?.rows.get(id) ?? this.domain?.subjects.get(id)
      ?? this.domain?.readers.get(id)?.subject
      ?? (this.usesOverview ? this.overview.get(id) : undefined);
  }
  async act(work: () => Promise<unknown>) {
    this.actionError = '';
    try { await work(); } catch (error) { if (!this.invalidated(error)) this.actionError = issueOf(error); }
    this.emit();
  }
  private actionSnapshot(id: string) {
    if (this.connection === 'reset') return;
    const reader = this.domain?.readers.get(id);
    return reader ? (reader.stale ? undefined : reader.snapshot) : this.desiredReaders.includes(id) ? undefined : this.observed(id);
  }
  capabilities(id: string) {
    const snapshot = this.actionSnapshot(id);
    return snapshot && nativeSnapshot(snapshot);
  }
  /** Paging status is independent of send authority; the operation still verifies the lease afresh. */
  canSendTo(id: string) {
    const reader = this.domain?.readers.get(id);
    return this.connection === 'pinned' && reader?.agent.kind === 'current' && !reader.stale && !!reader.snapshot?.facts.can_send;
  }
  blocked(key: string, kind?: Kind) {
    return !this.descriptor || this.connection === 'reset' || !!this.error || this.receipts.some(receipt => this.ownsReceipt(receipt) && receipt.key === key && unresolved(receipt) && (kind === 'transcript' ? receipt.kind === kind : receipt.kind !== 'transcript'));
  }
  private save(required = false) {
    try { persistJournal(this.receipts); }
    catch { if (required) throw new Error('无法保存恢复凭据，操作未发送'); this.error = '无法保存最新回执；请保留此页面'; }
  }
  private async submit(key: string, command: SpaceCommand, dispatch: () => Promise<CommandOutcome>, onSucceeded?: () => void): Promise<boolean> {
    const receipt = makeReceipt(key, command);
    if (!this.canLookup(receipt) || this.blocked(key, command.kind)) throw new Error('操作不可用或原回执尚未确定');
    this.receipts = [receipt, ...this.receipts];
    try { this.save(true); } catch (error) { this.receipts = this.receipts.filter(item => item !== receipt); throw error; }
    if (onSucceeded) this.completions.set(receipt.id, onSucceeded);
    this.emit();
    try {
      const outcome = await dispatch();
      this.accept(receipt, outcome);
      this.save(); this.emit();
      return outcome.receipt.status === 'Succeeded';
    } catch (error) {
      this.invalidated(error);
      if (unresolved(receipt)) receipt.issue = `回执未确认：${issueOf(error)}；仅查询原操作，勿重复提交。`;
    }
    this.save(); this.emit();
    return false;
  }
  private accept(receipt: Receipt, { receipt: operation, transcript }: CommandOutcome) {
    if (operation.request_id !== receipt.id || operation.generation !== receipt.body.generation) throw new Error('回执身份不匹配');
    // The original response can arrive after a lookup has already confirmed the same operation.
    if (!unresolved(receipt) && operation.status !== receipt.operation?.status) return;
    receipt.operation = operation; receipt.issue = undefined;
    const current = this.canLookup(receipt) && this.current(receipt.scope, receipt.body.generation);
    if (operation.status === 'Succeeded' || operation.status === 'Failed') {
      const complete = this.completions.get(receipt.id);
      this.completions.delete(receipt.id);
      if (operation.status === 'Succeeded' && current) complete?.();
    }
    if (!current || receipt.kind !== 'transcript') return;
    if (transcript) this.records.set(`${receipt.key}:model`, transcriptRecords(transcript));
    else if (operation.status === 'Failed') this.records.set(`${receipt.key}:model`, { status: 'error', reason: operation.error ?? 'Provider 读取失败' });
  }
  lookup = async (receipt: Receipt) => {
    if (this.busyLookups.has(receipt.id) || !this.canLookup(receipt) || !this.current(receipt.scope, receipt.body.generation)) return;
    const scope = this.domain!.scope;
    this.busyLookups.add(receipt.id);
    try {
      const command = restoreCommand(receipt, scope.address);
      const outcome = command.kind === 'transcript' ? transcriptOutcome(await scope.operation(command).observe(this.options))
        : await scope.operation(command).observe(this.options);
      this.accept(receipt, outcome);
    }
    catch (error) { this.invalidated(error); receipt.issue = `${issueOf(error)}；保留原凭据，未重发。`; }
    finally { this.busyLookups.delete(receipt.id); this.save(); this.emit(); }
  };
  async target(kind: 'send' | 'stop' | 'kill' | 'stop-participation' | 'fork' | 'resume' | 'transcript', id: string, message?: MessageInput, onSucceeded?: () => void) {
    const descriptor = this.descriptor, domain = this.domain;
    if (!descriptor || !domain || this.blocked(id, kind)) throw new Error('目标已失效或原回执尚未确定');
    if (kind === 'send') {
      const { snapshot, agent, signal } = await domain.verifySend(id);
      if (signal.aborted || this.domain !== domain || !this.current(descriptor.scope, descriptor.generation)) throw new Error('发送期间目标已失效');
      if (!snapshot.facts.can_send) throw new Error('原生能力不可用');
      if (!message || (!message.text.trim() && !message.attachments?.length) || new TextEncoder().encode(message.text).length > descriptor.max_text) throw new Error('消息为空或超过原生字节上限');
      const prepared = agent.prepareMessage(message, requestId());
      return this.submit(id, { ...prepared, kind }, async () => ({ receipt: await agent.send(prepared) }), onSucceeded);
    }
    const snapshot = this.actionSnapshot(id);
    if (!snapshot) throw new Error('目标已失效');
    const agent = domain.scope.agent(snapshot.address), facts = snapshot.facts, request = requestId();
    switch (kind) {
      case 'stop': case 'kill': {
        if (agent.kind !== 'current' || !facts.can_stop) break;
        const prepared = agent.prepareKill(request);
        return this.submit(id, prepared, () => agent.kill(prepared), onSucceeded);
      }
      case 'stop-participation': {
        if (agent.kind !== 'current' || !facts.can_stop_participation) break;
        const prepared = agent.prepareStopParticipation(request);
        return this.submit(id, prepared, () => agent.stopParticipation(prepared), onSucceeded);
      }
      case 'resume': {
        if (!facts.can_resume) break;
        const prepared = agent.prepareResume(request);
        return this.submit(id, prepared, () => agent.resume(prepared), onSucceeded);
      }
      case 'fork': {
        if (!facts.can_fork) break;
        const prepared = agent.prepareFork(request);
        return this.submit(id, prepared, () => agent.fork(prepared), onSucceeded);
      }
      case 'transcript': {
        if (!facts.has_transcript) break;
        const prepared = agent.prepareTranscript(request);
        return this.submit(id, prepared, async () => transcriptOutcome(await agent.requestTranscript(prepared, this.options)), onSucceeded);
      }
    }
    throw new Error('原生能力不可用');
  }
  async upload(id: string, files: readonly Blob[]): Promise<Attachment[]> {
    const descriptor = this.descriptor, scope = this.domain?.scope;
    if (!scope || !descriptor || !this.canSendTo(id) || this.blocked(id)) throw new Error('目标不可用或原操作未确定');
    const result: Attachment[] = [];
    for (const file of files) {
      const name = file instanceof File ? file.name : 'attachment';
      const { address, ...attachment } = await scope.attachments.upload({ name, mediaType: file.type || 'application/octet-stream', bytes: file })
        .catch(error => { this.invalidated(error); throw error; });
      result.push({ id: address.id, ...attachment });
      if (!this.current(descriptor.scope, descriptor.generation) || !this.canSendTo(id)) throw new Error('上传期间目标已失效');
    }
    return result;
  }
  latestLaunch(key: string) { return this.latestReceipt('launch', key); }
  latestReceipt(kind: Kind, key: string) {
    return this.receipts.find(receipt => receipt.kind === kind && receipt.key === key && this.ownsReceipt(receipt));
  }
  forkReference(receipt: Receipt) {
    const operation = receipt.operation;
    if (receipt.kind !== 'fork' || !this.canLookup(receipt) || operation?.status !== 'Succeeded'
      || operation.request_id !== receipt.id || operation.generation !== receipt.body.generation
      || !operation.result || !('Launched' in operation.result)) return;
    const { agent_refs: refs, agents } = operation.result.Launched;
    if (refs?.length !== 1 || agents.length !== 1 || refs[0].name !== agents[0]) return;
    const ref = refs[0];
    if (ref.incarnation != null && ref.stopped_event == null) return ref;
  }
  private checkLaunch(id: string) {
    const receipt = this.receipts.find(item => item.id === id);
    const issue = receipt && operationIssue(receipt);
    if (issue) throw new Error(issue);
  }
  async launchNew(launch: Omit<LaunchDraft, "labels">, source: { scope: string; generation: string }) {
    const scope = this.domain?.scope;
    if (!scope || !this.current(source.scope, source.generation)) throw new Error('启动来源范围已变化');
    const id = requestId();
    const prepared = scope.prepareLaunch({ ...launch, labels: {} }, id);
    await this.submit('launch:new', prepared, () => scope.launch(prepared));
    this.checkLaunch(id);
  }
  async launch(post: Post, prompt: string, title = '', directory = post.office === '$global' ? this.descriptor?.project ?? '' : post.office, receiptKey = post.id) {
    const descriptor = this.descriptor, scope = this.domain?.scope;
    if (!scope || !descriptor || this.connection === 'reset' || !this.posts.some(current => current.id === post.id)) throw new Error('岗位已失效');
    const resolved = directory.trim();
    if (!resolved.startsWith('/') || resolved.split(/[\\/]/).includes('..')) throw new Error('请输入已存在目录的绝对路径');
    const check = await scope.workspace(resolved).roles(this.options);
    if (check.workspace.address.key !== resolved) throw new Error('工作目录验证结果不匹配');
    if (!this.current(descriptor.scope, descriptor.generation)) throw new Error('启动期间连接范围已变化');
    const id = requestId();
    const workspace = scope.workspace(post.office === '$global' ? 'global' : post.office);
    const prepared = workspace.prepareLaunch({ labels: {}, role: post.name, directory: resolved, tool: null, profile: null, prompt: prompt || null, title: title.trim() || null }, id);
    await this.submit(receiptKey, prepared, () => workspace.launch(prepared));
    this.checkLaunch(id);
  }
  async preview(directory: string) {
    const descriptor = this.descriptor, scope = this.domain?.scope, epoch = ++this.entryEpoch;
    this.entry = undefined; this.entryError = ''; this.emit();
    if (!scope || !descriptor || this.connection === 'reset') return;
    try {
      const entry = await scope.directory.inspect(directory, this.options);
      if (epoch !== this.entryEpoch || !this.current(descriptor.scope, descriptor.generation)) return;
      this.entry = entry; this.entryInput = directory;
    } catch (error) { if (this.invalidated(error)) return; if (epoch === this.entryEpoch && this.current(descriptor.scope, descriptor.generation)) this.entryError = issueOf(error); }
    this.emit();
  }
  async directorySend(text: string) {
    const descriptor = this.descriptor, routes = this.domain?.scope.directory, entry = this.entry;
    if (!routes || !descriptor || !entry || entry.generation !== descriptor.generation || !['bound', 'adoptable', 'absent'].includes(entry.endpoint.status) || !text.trim() || new TextEncoder().encode(text).length > descriptor.max_text) throw new Error('入口或消息不可用');
    const prepared = routes.prepareSend(entry.directory, { attachments: [], text, intent: 'request', reply_to: null, thread: null }, requestId());
    await this.submit(`directory:${entry.directory}`, prepared, () => routes.send(prepared));
  }
}
