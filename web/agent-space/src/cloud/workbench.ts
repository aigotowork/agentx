/** @aiPurpose Compose Namespace catalogue, exact conversations and Admin controls for the shared App. */
import { newCloudId, CloudAdminError, type CloudAdminTypes, type CloudControlInput, type CloudHostCapabilities } from '@agentx/client';
import type { MessageInput } from '../../../management/api/native';
import { type AgentSpaceSupply, type CloudOffice, type RecordChannel } from '../offline/model';
import type { RouteTarget } from '../navigation/route';
import { watchChanges } from './changes';
import { CloudHistory, messagePage } from './history';
import { CloudTranscriptReader, readTranscriptPage } from './transcript';
import { agentsPage, cloudPageSize, cloudProblem, hostsPage, projectCloudAgent, projectCloudOffice, resolveAgent, unsupportedCloudOperation, workspacesPage, NamespaceChanged, NamespaceUnavailable, type CloudHost, type CloudRosterAgent, type CloudSubject, type PageQuery, type Session } from './model';
import { CloudOperations, controlPending, type ControlEntry } from './operations';
import { cloudNavigation, cloudOfficeQuery, sameAgent, type CloudAgent } from './route';

type Roster = { rows: Map<string, CloudRosterAgent>; next: string | null; pages: number; phase: 'loading' | 'ready' | 'error' | 'unavailable'; error?: string };
type PendingSend = { input: CloudAdminTypes['SendHumanMessage']; done?: () => void; busy: boolean; uncertain: boolean };
const roster = (): Roster => ({ rows: new Map(), next: null, pages: 0, phase: 'loading' });
const RECEIPT_REFRESH_MS = 5_000;
const isDefiniteRejection = (error: unknown) => error instanceof CloudAdminError && error.status !== undefined
  && error.status >= 400 && error.status < 500 && ![408, 425, 429].includes(error.status);

/** Admitted cloud supply for the shared App. Exact reader subjects are separate from the paged current roster. */
export class CloudWorkbench {
  readonly kind = 'cloud';
  readonly navigation;
  readonly descriptor;
  readonly access;
  readonly operations?: CloudOperations;
  hosts: readonly CloudHost[] = [];
  private projects: readonly CloudOffice[] = [];
  private catalogPhase: 'loading' | 'ready' | 'error' = 'loading';
  connection: 'connecting' | 'pinned' | 'failed' | 'reset' = 'pinned';
  error = '';
  catalogError = '';
  actionError = '';
  private office = '';
  private currentRoster = roster();
  private rosterRead?: AbortController;
  private catalogRead?: AbortController;
  private refreshCatalogAfterRead = false;
  private lifetime?: AbortController;
  private watcher?: AbortController;
  private receiptTimer?: ReturnType<typeof setTimeout>;
  private checkingReceipts = false;
  private desired: readonly string[] = [];
  private selected: string | null = null;
  private follow?: { id: string; controller: AbortController };
  private readers = new Map<string, CloudHistory>();
  private transcripts = new Map<string, CloudTranscriptReader>();
  private known = new Map<string, { agent: CloudSubject; current: boolean }>();
  private retired = new Set<string>();
  private sends = new Map<string, PendingSend>();
  private launchFollowers = new Map<string, (project: () => string) => void>();
  private accepted = new Map<string, string>();
  private stoppedByReceipt = new Set<string>();
  private completedControls = new Set<string>();
  private refreshRosterAfterRead = false;
  constructor(private readonly session: Session, private readonly publish: (supply: AgentSpaceSupply) => void) {
    this.navigation = cloudNavigation(session.namespace, session.generation);
    this.access = session.access;
    this.descriptor = { scope: session.namespace, generation: session.generation, max_text: 64 * 1024 };
    if (session.access === 'admin') this.operations = new CloudOperations(session, this.emit, this.failure);
  }
  get more() { return !!this.currentRoster.next; }
  start() {
    const controller = this.lifetime = new AbortController();
    const visibility = () => {
      this.watcher?.abort();
      if (!document.hidden && this.connection !== 'reset') {
        this.watcher = new AbortController();
        const signal = AbortSignal.any([controller.signal, this.watcher.signal]);
        void watchChanges(this.session, signal, async (next, previous) => {
          if (this.error && this.connection === 'pinned') { this.error = ''; this.emit(); }
          const reset = !previous || next.epoch !== previous.epoch;
          const rosterChanged = reset || next.roster !== previous?.roster;
          if (rosterChanged || this.currentRoster.phase === 'error') {
            if (this.rosterRead) this.refreshRosterAfterRead = true;
            else this.refreshMembers();
            if (this.operations?.entries.some(controlPending)) void this.checkReceipts(signal);
          }
          if (rosterChanged || this.catalogError) this.invalidateCatalogue();
          const id = this.selected;
          const reader = id && this.readers.get(id);
          if (reader && (reset || next.messages !== previous?.messages || reader.page.phase === 'error' || reader.liveFailed)) {
            await reader.update(signal, 'latest', true);
          }
          if (id && !signal.aborted && (reset || next.transcript !== previous?.transcript)) await this.refreshTranscript(id, signal);
        }, error => {
          this.failure(error);
          if (unsupportedCloudOperation(error)) {
            if (!this.currentRoster.pages) this.refreshMembers();
            if (this.catalogPhase === 'loading') this.retryCatalog();
            this.error = '此服务不支持监视云端更新，请手动刷新。';
            this.emit();
            return true;
          }
          if (this.connection === 'pinned') this.error = '无法监视云端更新，正在重试。';
          this.emit();
          return this.connection === 'reset';
        });
      }
      this.followSelected();
      this.scheduleReceipts();
    };
    document.addEventListener('visibilitychange', visibility, { signal: controller.signal });
    visibility();
    return () => {
      controller.abort(); this.rosterRead?.abort(); this.catalogRead?.abort(); this.rosterRead = this.catalogRead = undefined;
      this.refreshCatalogAfterRead = false;
      clearTimeout(this.receiptTimer); this.receiptTimer = undefined;
      this.desired = []; this.selected = null; this.follow = undefined;
    };
  }
  private get signal() { return this.lifetime?.signal; }
  private id(agent: CloudAgent) { return projectCloudAgent(agent, this.session).id; }
  private failure = (error: unknown) => {
    if (error instanceof NamespaceChanged || error instanceof NamespaceUnavailable
      || error instanceof CloudAdminError && ([401, 403].includes(error.status ?? 0) || error.code === 'stale_generation')) {
      this.connection = 'reset';
      this.watcher?.abort();
      this.rosterRead?.abort();
      this.catalogRead?.abort();
      this.error = cloudProblem(error, '云端连接已失效，请重新登录。');
      this.currentRoster = { ...roster(), phase: 'error', error: this.error };
      this.catalogPhase = 'error';
      this.catalogError = this.error;
      this.hosts = [];
      this.projects = [];
      // Keep exact display identities for open drafts while invalidating their data and actions.
      for (const entry of this.known.values()) {
        const { host, name, incarnation, workspace } = entry.agent;
        entry.agent = { host, name, incarnation, workspace };
        entry.current = false;
      }
      this.readers.clear();
      this.transcripts.clear();
      this.followSelected();
      this.scheduleReceipts();
      this.emit();
    }
  };
  private active() {
    const signal = this.signal;
    if (!signal || signal.aborted || this.connection === 'reset') throw new Error('云端会话已失效，请重新登录。');
    return signal;
  }
  private emit = () => {
    if (this.signal?.aborted) return;
    const agents = [...this.currentRoster.rows.values()].filter(agent => !this.stoppedByReceipt.has(this.id(agent))).map(agent => projectCloudAgent(agent, this.session));
    const subjects = [...this.known].map(([id, { agent }]) => projectCloudAgent(this.currentRoster.rows.get(id) ?? agent, this.session,
      this.transcripts.get(id)?.facts, this.retired.has(id) || this.stoppedByReceipt.has(id)));
    const { phase, error, next } = this.currentRoster;
    this.publish({
      mode: 'live', scope: { cloud: true, id: this.session.namespace, generation: this.session.generation, coverage: '云端已注册实例 · 显示主机已上报的状态与工作目录' },
      workspaces: this.projects, cloudDirectory: { phase: this.catalogPhase, hosts: this.hosts, projects: this.projects }, agents, subjects, posts: [],
      population: { revision: this.currentRoster.pages, complete: phase === 'ready' && !next },
      roster: { office: this.office, phase, error, more: !!next },
      records: (id: string, channel: RecordChannel) => this.connection === 'reset' ? { status: 'unavailable', reason: this.error }
        : channel === 'model' ? this.transcripts.get(id)?.records ?? { status: 'loading' }
        : this.readers.get(id)?.records ?? { status: 'loading' },
    });
  };
  chooseOffice(office: string) {
    if (this.office === office) return;
    this.rosterRead?.abort();
    this.rosterRead = undefined;
    this.refreshRosterAfterRead = false;
    this.office = office;
    this.currentRoster = roster();
    if (this.signal && !this.signal.aborted) void this.loadRoster(false);
  }
  private async loadRoster(append: boolean, pages?: number) {
    if (this.rosterRead || !this.signal || this.signal.aborted || this.connection === 'reset') return;
    const request = this.rosterRead = new AbortController();
    const signal = AbortSignal.any([request.signal, this.signal]);
    const current = this.currentRoster;
    const query = cloudOfficeQuery(this.office);
    if (!current.pages || append) { current.phase = 'loading'; this.emit(); }
    try {
      const rows = append ? new Map(current.rows) : new Map<string, CloudRosterAgent>();
      let after = append ? current.next ?? undefined : undefined;
      let read = append ? current.pages : 0;
      const count = Math.max(1, pages ?? 1);
      for (let i = 0; i < count; i++) {
        const page = await agentsPage(this.session, { ...query, after, limit: cloudPageSize }, signal);
        signal.throwIfAborted();
        if (page.next && page.next === after) throw new Error('成员清单游标未前进');
        for (const agent of page.items) {
          if (inSelection(agent, query)) rows.set(this.id(agent), agent);
        }
        read++; after = page.next ?? undefined;
        if (!after) break;
      }
      // A partial or project-filtered snapshot cannot evict an exact reader from another office.
      for (const [id, entry] of this.known) {
        const fresh = rows.get(id);
        if (fresh) { entry.agent = fresh; entry.current = true; this.retired.delete(id); }
        else if (inSelection(entry.agent, query) && (rowsHasReplacement(rows, entry.agent) || (!after && current.rows.has(id)))) {
          entry.current = false; this.retired.add(id);
        }
      }
      this.currentRoster = { rows, next: after ?? null, pages: read, phase: 'ready' };
    } catch (error) {
      if (!signal.aborted) {
        this.failure(error);
        current.phase = unsupportedCloudOperation(error) ? 'unavailable' : 'error';
        current.error = cloudProblem(error, '无法读取云端成员。');
      }
    } finally {
      if (this.rosterRead === request) {
        this.rosterRead = undefined;
        if (this.refreshRosterAfterRead) {
          this.refreshRosterAfterRead = false;
          this.refreshMembers();
        }
      }
      if (!signal.aborted) this.emit();
    }
  }
  refreshMembers = () => { void this.loadRoster(false, this.currentRoster.pages); };
  moreMembers = () => {
    if (this.currentRoster.phase === 'error') this.refreshMembers();
    else if (this.currentRoster.next) void this.loadRoster(true);
  };
  loadMore = this.moreMembers;
  retryCatalog = () => { void this.loadCatalogue(); };
  private invalidateCatalogue() {
    if (this.catalogRead) this.refreshCatalogAfterRead = true;
    else this.retryCatalog();
  }
  private async loadCatalogue() {
    if (this.catalogRead || !this.signal || this.signal.aborted || this.connection === 'reset') return;
    const request = this.catalogRead = new AbortController();
    const signal = AbortSignal.any([request.signal, this.signal]);
    try {
      const reportFailure = (error: unknown) => {
        if (!signal.aborted) this.failure(error);
        throw error;
      };
      const [hosts, projects] = await Promise.allSettled([
        readCatalogue(query => hostsPage(this.session, query, signal), signal).catch(reportFailure),
        readCatalogue(query => workspacesPage(this.session, query, signal), signal).catch(reportFailure),
      ]);
      signal.throwIfAborted();
      if (hosts.status === 'fulfilled') this.hosts = hosts.value;
      if (projects.status === 'fulfilled') this.projects = projects.value.map(projectCloudOffice);
      if (hosts.status === 'rejected') throw hosts.reason;
      if (projects.status === 'rejected') throw projects.reason;
      this.catalogPhase = 'ready';
      this.catalogError = '';
    } catch (error) {
      if (!signal.aborted) { this.failure(error); this.catalogPhase = 'error'; this.catalogError = cloudProblem(error, '无法读取项目目录。'); }
    } finally {
      if (this.catalogRead === request) {
        this.catalogRead = undefined;
        if (this.refreshCatalogAfterRead) {
          this.refreshCatalogAfterRead = false;
          this.retryCatalog();
        }
      }
      if (!signal.aborted) this.emit();
    }
  }
  refresh = async () => { this.refreshData(); };
  refreshData = () => {
    if (!this.signal || this.signal.aborted || this.connection === 'reset') return;
    this.refreshMembers();
    this.retryCatalog();
    for (const id of this.desired) this.retryMessagesLive(id);
    if (this.selected) void this.refreshTranscript(this.selected);
    void this.checkReceipts(this.signal);
  };
  private scheduleReceipts() {
    clearTimeout(this.receiptTimer);
    this.receiptTimer = undefined;
    const signal = this.signal;
    if (signal && !signal.aborted && !document.hidden && this.connection === 'pinned' && !this.checkingReceipts
      && this.operations?.entries.some(controlPending)) {
      this.receiptTimer = setTimeout(() => void this.checkReceipts(signal), RECEIPT_REFRESH_MS);
    }
  }
  private async checkReceipts(signal: AbortSignal) {
    if (signal.aborted || this.checkingReceipts || this.connection !== 'pinned') return;
    this.checkingReceipts = true;
    try {
      const pending = this.operations?.entries.filter(controlPending) ?? [];
      for (let i = 0; i < pending.length && !signal.aborted && !document.hidden && this.connection === 'pinned'; i += 4) {
        await Promise.all(pending.slice(i, i + 4).map(entry => this.lookupControl(entry)));
      }
    } finally {
      this.checkingReceipts = false;
      this.scheduleReceipts();
    }
  }
  openReference = async (target: RouteTarget, request: AbortSignal, office: string) => {
    if (target.scope !== this.session.namespace || target.generation !== this.session.generation
      || !target.host || !target.incarnation || target.record !== 'current') throw new Error('云端实例引用与当前连接不匹配');
    const signal = AbortSignal.any([this.active(), request]);
    const agent = { host: target.host, name: target.name, incarnation: target.incarnation };
    try {
      const missing = (error: unknown) => {
        if (!unsupportedCloudOperation(error)) throw error;
        return undefined;
      };
      const [initial, current] = await Promise.all([
        messagePage(this.session, agent, signal).catch(missing),
        resolveAgent(this.session, agent, signal).catch(missing),
      ]);
      signal.throwIfAborted();
      let history = initial?.items.flatMap(item => [item.from, item.to]).find(item => sameAgent(item, agent));
      if (!current && !history && (!initial || cloudOfficeQuery(office).workspace_id)) {
        // An empty AgentX mailbox has no membership metadata. The exact Transcript envelope carries it.
        history = (await readTranscriptPage(this.session, agent, signal)).agent;
      }
      this.active();
      signal.throwIfAborted();
      const id = this.remember(current ?? history ?? agent, !!current);
      if (!this.readers.has(id)) {
        const reader = this.createReader(agent);
        if (initial) reader.seed(initial);
        else reader.unavailable();
        this.readers.set(id, reader);
      }
      this.emit();
    } catch (error) {
      if (!signal.aborted) { this.failure(error); this.emit(); }
      throw new Error(cloudProblem(error, '无法读取此精确的云端实例'));
    }
  };
  private remember(agent: CloudSubject, current: boolean) {
    const id = this.id(agent);
    this.known.set(id, { agent, current });
    return id;
  }
  private createReader(agent: CloudAgent) { return new CloudHistory(agent, this.session, this.emit, this.failure); }
  select = (id: string | null) => this.selectReaders(id ? [id] : [], id);
  selectReaders = (ids: readonly string[], selected: string | null = ids[0] ?? null) => {
    if (this.connection === 'reset') return;
    if (selected === this.selected && ids.length === this.desired.length && ids.every((id, index) => id === this.desired[index])) return;
    const previous = this.desired, previousSelected = this.selected;
    this.desired = ids;
    this.selected = selected;
    for (const id of ids) {
      const row = this.currentRoster.rows.get(id);
      if (row) this.remember(row, !this.retired.has(id));
      const agent = row ?? this.known.get(id)?.agent;
      if (agent && !this.readers.has(id)) this.readers.set(id, this.createReader(agent));
      if (id !== selected && !previous.includes(id)) this.retryMessagesLive(id);
    }
    // Keep parked conversations and their scroll tokens bounded beyond the visible shelf.
    for (const id of this.readers.keys()) {
      if (this.readers.size <= 20) break;
      if (!ids.includes(id) && !this.sends.has(id)) this.readers.delete(id);
    }
    for (const id of this.known.keys()) if (!this.readers.has(id)) { this.known.delete(id); this.transcripts.delete(id); }
    this.followSelected();
    if (selected && selected !== previousSelected) void this.refreshTranscript(selected);
  };
  private followSelected() {
    const life = this.signal;
    const id = life && !life.aborted && this.connection === 'pinned' && !document.hidden && this.selected && this.desired.includes(this.selected)
      ? this.selected : undefined;
    if (this.follow?.id === id) return;
    this.follow?.controller.abort();
    this.follow = undefined;
    const reader = id && this.readers.get(id);
    if (!life || !id || !reader) return;
    const controller = new AbortController();
    this.follow = { id, controller };
    const signal = AbortSignal.any([life, controller.signal]);
    void reader.update(signal, 'latest');
  }
  refreshTranscript = (id: string, request?: AbortSignal) => {
    const life = this.signal;
    const agent = this.currentRoster.rows.get(id) ?? this.known.get(id)?.agent;
    if (!life || life.aborted || document.hidden || this.connection !== 'pinned' || !agent) return Promise.resolve();
    let reader = this.transcripts.get(id);
    if (!reader) {
      reader = new CloudTranscriptReader(agent, this.session, this.emit, this.failure);
      this.transcripts.set(id, reader);
    }
    const follow = this.follow?.id === id ? this.follow.controller.signal : undefined;
    const signal = AbortSignal.any([life, ...(follow ? [follow] : []), ...(request ? [request] : [])]);
    return reader.refresh(signal);
  };
  transcriptPage(id: string) { return this.transcripts.get(id)?.page; }
  moreTranscriptFor = (id: string) => {
    if (!this.signal || this.connection === 'reset') return;
    const follow = this.follow?.id === id ? this.follow.controller.signal : this.signal;
    void this.transcripts.get(id)?.readOlder(follow);
  };

  messagePage(id: string) { return this.readers.get(id)?.page; }
  retryMessagesLive = (id: string) => { if (this.signal && this.connection !== 'reset') void this.readers.get(id)?.update(this.signal, 'latest'); };
  moreMessagesFor = (id: string) => { if (this.signal && this.connection !== 'reset') void this.readers.get(id)?.update(this.signal, 'older'); };

  private currentAgent(id: string) {
    if (this.retired.has(id) || this.stoppedByReceipt.has(id)) return;
    const known = this.known.get(id);
    const agent = this.currentRoster.rows.get(id) ?? (known?.current ? known.agent : undefined);
    return agent?.workspace ? { host: agent.host, name: agent.name, incarnation: agent.incarnation, workspace: agent.workspace } : undefined;
  }
  blocked(key: string) {
    return this.session.access !== 'admin' || this.connection !== 'pinned' || this.sends.has(key)
      || !!this.operations?.entries.some(entry => controlPending(entry) && (entry.identity.kind === 'add_workspace' ? key === 'workspace:' + entry.identity.host
        : entry.identity.kind === 'launch' ? key === 'launch:new'
        : !!entry.identity.target && this.id({ ...entry.identity.target, host: entry.identity.host }) === key));
  }
  canSendTo(id: string) { return this.session.access === 'admin' && this.connection === 'pinned' && !!this.currentAgent(id); }
  capabilities(id: string) {
    return { can_stop: this.canSendTo(id), can_stop_participation: false, can_fork: false, can_resume: false, has_transcript: this.transcripts.get(id)?.records.status === 'available' };
  }
  async act(work: () => Promise<unknown>) {
    this.actionError = '';
    try { await work(); }
    catch (error) { this.actionError = cloudProblem(error, error instanceof Error ? error.message : '操作未完成。'); }
    this.emit();
  }
  messageFeedback(id: string) {
    const pending = this.sends.get(id);
    return { accepted: this.accepted.get(id), pending: !!pending, uncertain: !!pending?.uncertain, busy: !!pending?.busy };
  }
  async send(id: string, message: MessageInput, done: () => void) {
    const agent = this.currentAgent(id);
    if (!agent || this.blocked(id) || !message.text.trim() || new TextEncoder().encode(message.text).length > this.descriptor.max_text) throw new Error('消息为空、过长或精确实例当前不可发送。');
    const pending = { input: { generation: this.session.generation, to: agent, request_id: newCloudId(), body: message.text }, done, busy: false, uncertain: false };
    this.sends.set(id, pending);
    await this.finishSend(id, pending);
  }
  retrySend = async (id: string) => {
    const pending = this.sends.get(id);
    if (pending?.uncertain && !pending.busy) await this.act(() => this.finishSend(id, pending));
  };
  private async finishSend(id: string, pending: PendingSend) {
    if (this.session.access !== 'admin') return;
    const signal = this.active();
    pending.busy = true;
    this.emit();
    try {
      const result = await this.session.client.sendHumanMessage(this.session.namespace, pending.input, { signal });
      signal.throwIfAborted();
      if (!sameAgent(result.to, pending.input.to) || result.body !== pending.input.body || !/^[1-9][0-9]{0,19}$/.test(result.id)) throw new Error('云端消息回执不匹配');
      this.sends.delete(id);
      this.accepted.set(id, result.id);
      this.readers.get(id)?.accept(result);
      pending.done?.();
      this.retryMessagesLive(id);
    } catch (error) {
      if (signal.aborted) return;
      this.failure(error);
      if (isDefiniteRejection(error) && !pending.uncertain) {
        this.sends.delete(id);
        if (error instanceof CloudAdminError && ['not_found', 'stale_agent'].includes(error.code)) this.retired.add(id);
        throw new Error(cloudProblem(error, '云端拒绝了这条消息。'));
      }
      pending.uncertain = true;
    } finally {
      pending.busy = false;
      if (!signal.aborted) this.emit();
    }
  }
  async catalogue(host: string, request?: AbortSignal) {
    const life = this.active();
    const signal = request ? AbortSignal.any([life, request]) : life;
    if (this.session.access !== 'admin') throw new Error('Namespace 访问为只读。');
    try {
      const value = await this.session.client.getHostCapabilities(this.session.namespace, host, { signal });
      signal.throwIfAborted();
      if (value.generation !== this.session.generation) throw new NamespaceChanged();
      return value;
    } catch (error) {
      if (!signal.aborted) { this.failure(error); this.emit(); }
      throw new Error(cloudProblem(error, '无法读取主机操作目录。'));
    }
  }
  async control(captured: CloudHostCapabilities, command: CloudControlInput['command'], id?: string, followLaunch?: (project: () => string) => void) {
    const signal = this.active();
    const key = command.kind === 'add_workspace' ? 'workspace:' + captured.host : id ?? 'launch:new';
    if (!this.operations || this.blocked(key)) throw new Error('操作不可用或原回执尚未确定。');
    const value = await this.catalogue(captured.host, signal);
    if (!value.online || !value.installation_id || captured.host !== value.host || captured.generation !== value.generation
      || captured.installation_id !== value.installation_id) throw new Error('主机安装已变化或当前离线，请重新选择。');
    if (command.kind === 'launch') {
      const workspace = value.workspaces.find(item => item.workspace_id === command.workspace_id);
      if (!workspace || !(command.role ? workspace.roles.some(role => role.name === command.role && role.available) : workspace.default_available)) throw new Error('所选工作区或岗位已不可用。');
      if ((command.working_directory != null || command.create_directory != null) && value.directory_launch !== true) throw new Error('此主机仅支持从工作区根目录启动。');
    } else if (command.kind === 'add_workspace') {
      if (!value.workspace_management || !value.project_roots.includes(command.parent)) throw new Error('此主机无法在所选位置添加工作区。');
    } else {
      const agent = id && this.currentAgent(id);
      if (!agent || !sameAgent(agent, { host: captured.host, name: command.name, incarnation: command.incarnation })
        || !await resolveAgent(this.session, agent, signal)) throw new Error('原实例已不在当前注册清单；不会停止同名实例。');
    }
    if (this.blocked(key)) throw new Error('操作不可用或原回执尚未确定。');
    const request_id = newCloudId();
    if (command.kind === 'launch' && followLaunch) this.launchFollowers.set(request_id, followLaunch);
    try {
      const entry = await this.operations.submit({ generation: this.session.generation, host: value.host, installation_id: value.installation_id,
        request_id, command }, signal);
      this.recordOutcome(entry);
      return entry;
    } finally {
      if (!this.operations.entries.some(entry => entry.identity.request_id === request_id)) this.launchFollowers.delete(request_id);
      this.scheduleReceipts();
    }
  }
  lookupControl = async (entry: ControlEntry) => {
    if (!this.operations || !this.signal || this.signal.aborted || this.connection === 'reset') return;
    await this.operations.lookup(entry, this.signal);
    this.recordOutcome(entry);
  };
  private recordOutcome(entry: ControlEntry) {
    const outcome = entry.receipt?.outcome;
    const requestId = entry.identity.request_id;
    if (outcome?.status === 'workspace_added' && !this.completedControls.has(requestId)) {
      this.completedControls.add(requestId);
      this.invalidateCatalogue();
    }
    if (outcome?.status === 'succeeded' && !this.completedControls.has(requestId)) {
      this.completedControls.add(requestId);
      if (this.rosterRead) this.refreshRosterAfterRead = true;
      else this.refreshMembers();
    }
    if (outcome?.status === 'succeeded' && entry.identity.kind === 'stop') {
      const id = this.id(outcome.agent);
      if (!this.stoppedByReceipt.has(id)) { this.stoppedByReceipt.add(id); this.emit(); }
    }
    const follow = this.launchFollowers.get(entry.identity.request_id);
    if (!follow || !outcome) return;
    this.launchFollowers.delete(entry.identity.request_id);
    if (outcome.status !== 'succeeded' || this.connection !== 'pinned' || this.signal?.aborted) return;
    follow(() => {
      const id = this.id(outcome.agent);
      this.remember(outcome.agent, true);
      if (!this.readers.has(id)) this.readers.set(id, this.createReader(outcome.agent));
      this.emit();
      return id;
    });
  }
}

function rowsHasReplacement(rows: ReadonlyMap<string, CloudAgent>, agent: CloudAgent) {
  return [...rows.values()].some(row => row.host === agent.host && row.name === agent.name && !sameAgent(row, agent));
}

function inSelection(agent: CloudSubject, query: ReturnType<typeof cloudOfficeQuery>) {
  return (!query.host || agent.host === query.host) && (!query.workspace_id
    || agent.workspace?.kind === 'project' && agent.workspace.workspace_id === query.workspace_id);
}

async function readCatalogue<T>(read: (query: PageQuery) => Promise<{ items: T[]; next: string | null }>, signal: AbortSignal) {
  const items: T[] = [], seen = new Set<string>();
  let after: string | undefined;
  do {
    const page = await read({ after, limit: 100 });
    signal.throwIfAborted();
    items.push(...page.items);
    if (page.next && seen.has(page.next)) throw new Error('目录游标未前进');
    if (page.next) seen.add(page.next);
    after = page.next ?? undefined;
  } while (after);
  return items;
}
