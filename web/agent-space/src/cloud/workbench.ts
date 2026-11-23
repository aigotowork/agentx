import { CloudAdminError, type CloudControlInput, type CloudHostCapabilities } from '@agentx/client';
import type { MessageInput } from '../../../management/api/native';
import { hostGroup, type AgentSpaceSupply, type Office, type RecordChannel } from '../offline/model';
import type { RouteTarget } from '../navigation/route';
import { CloudHistory, messagePage } from './history';
import { agentsPage, cloudPageSize, cloudProblem, hostsPage, projectCloudAgent, resolveAgent, ScopeChanged, ScopeUnavailable, type CloudHost, type Session } from './model';
import { CloudOperations, controlPending, type ControlEntry } from './operations';
import { cloudNavigation, sameAgent, type CloudAgent } from './route';

type Roster = { rows: Map<string, CloudAgent>; next: string | null; pages: number; phase: 'loading' | 'ready' | 'error'; error?: string };
type PendingSend = { input: { generation: string; request_id: string; to: CloudAgent; body: string }; done?: () => void; busy: boolean; uncertain: boolean };
const roster = (): Roster => ({ rows: new Map(), next: null, pages: 0, phase: 'loading' });
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
  connection: 'connecting' | 'pinned' | 'failed' | 'reset' = 'pinned';
  error = '';
  catalogError = '';
  actionError = '';
  private office = '';
  private currentRoster = roster();
  private rosterRead?: AbortController;
  private hostRead?: AbortController;
  private lifetime?: AbortController;
  private timer?: ReturnType<typeof setTimeout>;
  private desired: readonly string[] = [];
  private readers = new Map<string, CloudHistory>();
  private known = new Map<string, { agent: CloudAgent; current: boolean }>();
  private retired = new Set<string>();
  private sends = new Map<string, PendingSend>();
  private launchFollowers = new Map<string, (project: () => string) => void>();
  private accepted = new Map<string, string>();
  private stoppedByReceipt = new Set<string>();
  constructor(private readonly session: Session, private readonly publish: (supply: AgentSpaceSupply) => void) {
    this.navigation = cloudNavigation(session.scope, session.generation);
    this.access = session.access;
    this.descriptor = { scope: session.scope, generation: session.generation, max_text: 64 * 1024 };
    if (session.access === 'admin') this.operations = new CloudOperations(session, this.emit, this.failure);
  }
  get more() { return !!this.currentRoster.next; }
  start() {
    const controller = this.lifetime = new AbortController();
    const tick = () => {
      clearTimeout(this.timer);
      if (controller.signal.aborted || document.hidden) return;
      this.refreshData();
      this.timer = setTimeout(tick, 5000);
    };
    document.addEventListener('visibilitychange', tick, { signal: controller.signal });
    tick();
    return () => { controller.abort(); this.rosterRead?.abort(); this.hostRead?.abort(); this.rosterRead = this.hostRead = undefined; clearTimeout(this.timer); this.desired = []; };
  }
  private get signal() { return this.lifetime?.signal; }
  private id(agent: CloudAgent) { return projectCloudAgent(agent, this.session).id; }
  private failure = (error: unknown) => {
    if (error instanceof ScopeChanged || error instanceof ScopeUnavailable
      || error instanceof CloudAdminError && ([401, 403].includes(error.status ?? 0) || error.code === 'stale_generation')) {
      this.connection = 'reset';
      this.error = cloudProblem(error, '云端连接已失效，请重新登录。');
    }
  };
  private active() {
    const signal = this.signal;
    if (!signal || signal.aborted || this.connection === 'reset') throw new Error('云端会话已失效，请重新登录。');
    return signal;
  }
  private emit = () => {
    if (this.signal?.aborted) return;
    const group = new Set([...this.hosts.map(host => host.name), ...[...this.currentRoster.rows.values()].map(agent => agent.host), ...[...this.known.values()].map(row => row.agent.host)]);
    const workspaces: Office[] = [...group].sort().map(host => ({ id: hostGroup(host), title: host, origin: 'cloud-host' }));
    const agents = [...this.currentRoster.rows.values()].filter(agent => !this.stoppedByReceipt.has(this.id(agent))).map(agent => projectCloudAgent(agent, this.session));
    const subjects = [...this.known.values()].map(({ agent }) => projectCloudAgent(agent, this.session));
    const { phase, error, next } = this.currentRoster;
    this.publish({
      mode: 'live', scope: { cloud: true, id: this.session.scope, generation: this.session.generation, coverage: '云端已注册实例 · 进程状态与本地工作区未提供' },
      workspaces, agents, subjects, posts: [],
      population: { revision: this.currentRoster.pages, complete: phase === 'ready' && !next },
      roster: { kind: 'cloud-registration', office: this.office, phase, error, more: !!next, exhausted: phase === 'ready' && !next, emptyPage: !agents.length },
      records: (id: string, channel: RecordChannel) => channel === 'model' ? { status: 'unavailable', reason: '云端未提供 Provider Transcript' }
        : this.readers.get(id)?.records ?? { status: 'loading' },
    });
  };
  chooseOffice(office: string) {
    if (this.office === office) return;
    this.rosterRead?.abort();
    this.rosterRead = undefined;
    this.office = office;
    this.currentRoster = roster();
    if (this.signal && !this.signal.aborted) void this.loadRoster(false);
  }
  private async loadRoster(append: boolean, pages?: number) {
    if (this.rosterRead || !this.signal || this.signal.aborted || this.connection === 'reset') return;
    const request = this.rosterRead = new AbortController();
    const signal = AbortSignal.any([request.signal, this.signal]);
    const current = this.currentRoster;
    const host = this.office.startsWith('host:') ? this.office.slice(5) : undefined;
    if (!current.pages || append) { current.phase = 'loading'; this.emit(); }
    try {
      const rows = append ? new Map(current.rows) : new Map<string, CloudAgent>();
      let after = append ? current.next ?? undefined : undefined;
      let read = append ? current.pages : 0;
      const count = Math.max(1, pages ?? 1);
      for (let i = 0; i < count; i++) {
        const page = await agentsPage(this.session, { host, after, limit: cloudPageSize }, signal);
        signal.throwIfAborted();
        if (page.next && page.next === after) throw new Error('成员清单游标未前进');
        for (const agent of page.items) {
          if (!host || host === agent.host) rows.set(this.id(agent), agent);
        }
        read++; after = page.next ?? undefined;
        if (!after) break;
      }
      // A partial or host-filtered snapshot cannot evict an exact reader from a different page.
      for (const [id, entry] of this.known) {
        if (rows.has(id)) { entry.current = true; this.retired.delete(id); }
        else if ((!host || host === entry.agent.host) && (rowsHasReplacement(rows, entry.agent) || (!after && current.rows.has(id)))) {
          entry.current = false; this.retired.add(id);
        }
      }
      this.currentRoster = { rows, next: after ?? null, pages: read, phase: 'ready' };
    } catch (error) {
      if (!signal.aborted) {
        this.failure(error);
        current.phase = 'error'; current.error = cloudProblem(error, '无法读取云端成员。');
      }
    } finally {
      if (this.rosterRead === request) this.rosterRead = undefined;
      if (!signal.aborted) this.emit();
    }
  }
  refreshMembers = () => { void this.loadRoster(false, this.currentRoster.pages); };
  moreMembers = () => {
    if (this.currentRoster.phase === 'error') this.refreshMembers();
    else if (this.currentRoster.next) void this.loadRoster(true);
  };
  loadMore = this.moreMembers;
  retryCatalog = () => { void this.loadHosts(); };
  private async loadHosts() {
    if (this.hostRead || !this.signal || this.signal.aborted || this.connection === 'reset') return;
    const request = this.hostRead = new AbortController();
    const signal = AbortSignal.any([request.signal, this.signal]);
    try {
      const hosts = new Map<string, CloudHost>();
      let after: string | undefined;
      do {
        const page = await hostsPage(this.session, { after, limit: 100 }, signal);
        signal.throwIfAborted();
        for (const host of page.items) hosts.set(host.name, host);
        if (page.next && page.next === after) throw new Error('主机目录游标未前进');
        after = page.next ?? undefined;
      } while (after);
      this.hosts = [...hosts.values()];
      this.catalogError = '';
    } catch (error) {
      if (!signal.aborted) { this.failure(error); this.catalogError = cloudProblem(error, '无法读取主机目录。'); }
    } finally {
      if (this.hostRead === request) this.hostRead = undefined;
      if (!signal.aborted) this.emit();
    }
  }
  refresh = async () => { this.refreshData(); };
  refreshData = () => {
    if (!this.signal || this.signal.aborted || this.connection === 'reset') return;
    this.refreshMembers();
    this.retryCatalog();
    for (const id of this.desired) this.retryMessagesLive(id);
    for (const entry of this.operations?.entries ?? []) if (controlPending(entry)) void this.lookupControl(entry);
  };
  openReference = async (target: RouteTarget, request: AbortSignal) => {
    if (target.scope !== this.session.scope || target.generation !== this.session.generation
      || !target.host || !target.incarnation || target.record !== 'current') throw new Error('云端实例引用与当前连接不匹配');
    const signal = AbortSignal.any([this.active(), request]);
    const agent = { host: target.host, name: target.name, incarnation: target.incarnation };
    try {
      const [initial, current] = await Promise.all([
        messagePage(this.session, agent, signal),
        this.session.access === 'admin' ? resolveAgent(this.session, agent, signal) : null,
      ]);
      signal.throwIfAborted();
      const id = this.remember(agent, !!current);
      if (!this.readers.has(id)) {
        const reader = this.createReader(agent);
        reader.seed(initial); this.readers.set(id, reader);
      }
      this.emit();
    } catch (error) {
      if (!signal.aborted) { this.failure(error); this.emit(); }
      throw new Error(cloudProblem(error, '无法读取此精确的云端实例'));
    }
  };
  private remember(agent: CloudAgent, current: boolean) {
    const id = this.id(agent);
    this.known.set(id, { agent, current });
    return id;
  }
  private createReader(agent: CloudAgent) { return new CloudHistory(agent, this.session, this.emit, this.failure); }
  select = (id: string | null) => this.selectReaders(id ? [id] : []);
  selectReaders = (ids: readonly string[], _selected?: string | null) => {
    if (ids.length === this.desired.length && ids.every((id, index) => id === this.desired[index])) return;
    const previous = this.desired;
    this.desired = ids;
    for (const id of ids) {
      const row = this.currentRoster.rows.get(id);
      if (row) this.remember(row, !this.retired.has(id));
      const agent = row ?? this.known.get(id)?.agent;
      if (agent && !this.readers.has(id)) this.readers.set(id, this.createReader(agent));
      if (!previous.includes(id)) this.retryMessagesLive(id);
    }
    // Keep parked conversations and their scroll tokens bounded beyond the visible shelf.
    for (const id of this.readers.keys()) {
      if (this.readers.size <= 20) break;
      if (!ids.includes(id) && !this.sends.has(id)) this.readers.delete(id);
    }
    for (const id of this.known.keys()) if (!this.readers.has(id)) this.known.delete(id);
  };
  messagePage(id: string) { return this.readers.get(id)?.page; }
  retryMessagesLive = (id: string) => { if (this.signal && this.connection !== 'reset') void this.readers.get(id)?.update(this.signal, 'latest'); };
  moreMessagesFor = (id: string) => { if (this.signal && this.connection !== 'reset') void this.readers.get(id)?.update(this.signal, 'older'); };

  private currentAgent(id: string) {
    if (this.retired.has(id) || this.stoppedByReceipt.has(id)) return;
    const known = this.known.get(id);
    return this.currentRoster.rows.get(id) ?? (known?.current ? known.agent : undefined);
  }
  blocked(key: string) {
    return this.session.access !== 'admin' || this.connection !== 'pinned' || this.sends.has(key)
      || !!this.operations?.entries.some(entry => controlPending(entry) && (entry.identity.kind === 'launch' ? key === 'launch:new'
        : !!entry.identity.target && this.id({ ...entry.identity.target, host: entry.identity.host }) === key));
  }
  canSendTo(id: string) { return this.session.access === 'admin' && this.connection === 'pinned' && !!this.currentAgent(id); }
  capabilities(id: string) {
    return { can_stop: this.canSendTo(id), can_stop_participation: false, can_fork: false, can_resume: false, has_transcript: false };
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
    const pending = { input: { generation: this.session.generation, to: agent, request_id: crypto.randomUUID(), body: message.text }, done, busy: false, uncertain: false };
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
      const result = await this.session.client.sendHumanMessage(this.session.scope, pending.input, { signal });
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
        if (error instanceof CloudAdminError && [404, 409].includes(error.status ?? 0)) this.retired.add(id);
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
    if (this.session.access !== 'admin') throw new Error('Scope 访问为只读。');
    try {
      const value = await this.session.client.getHostCapabilities(this.session.scope, host, { signal });
      signal.throwIfAborted();
      if (value.generation !== this.session.generation) throw new ScopeChanged();
      return value;
    } catch (error) {
      if (!signal.aborted) { this.failure(error); this.emit(); }
      throw new Error(cloudProblem(error, '无法读取主机操作目录。'));
    }
  }
  async control(captured: CloudHostCapabilities, command: CloudControlInput['command'], id?: string, followLaunch?: (project: () => string) => void) {
    const signal = this.active();
    const key = id ?? 'launch:new';
    if (!this.operations || this.blocked(key)) throw new Error('操作不可用或原回执尚未确定。');
    const value = await this.catalogue(captured.host, signal);
    if (!value.online || !value.installation_id || captured.generation !== value.generation
      || captured.installation_id !== value.installation_id) throw new Error('主机安装已变化或当前离线，请重新选择。');
    if (command.kind === 'launch') {
      const workspace = value.workspaces.find(item => item.alias === command.workspace);
      if (!workspace || !(command.role ? workspace.roles.some(role => role.name === command.role && role.available) : workspace.default_available)) throw new Error('所选工作区或岗位已不可用。');
    } else {
      const agent = id && this.currentAgent(id);
      if (!agent || !sameAgent(agent, { host: captured.host, name: command.name, incarnation: command.incarnation })
        || !await resolveAgent(this.session, agent, signal)) throw new Error('原实例已不在当前注册清单；不会停止同名实例。');
    }
    if (this.blocked(key)) throw new Error('操作不可用或原回执尚未确定。');
    const request_id = crypto.randomUUID();
    if (command.kind === 'launch' && followLaunch) this.launchFollowers.set(request_id, followLaunch);
    try {
      const entry = await this.operations.submit({ generation: this.session.generation, host: value.host, installation_id: value.installation_id,
        request_id, command }, signal);
      this.recordOutcome(entry);
      this.refreshData();
    } finally {
      if (!this.operations.entries.some(entry => entry.identity.request_id === request_id)) this.launchFollowers.delete(request_id);
    }
  }
  lookupControl = async (entry: ControlEntry) => {
    if (!this.operations || !this.signal || this.signal.aborted || this.connection === 'reset') return;
    await this.operations.lookup(entry, this.signal);
    this.recordOutcome(entry);
  };
  private recordOutcome(entry: ControlEntry) {
    const outcome = entry.receipt?.outcome;
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
      this.remember(outcome.agent, this.currentRoster.rows.has(id));
      if (!this.readers.has(id)) this.readers.set(id, this.createReader(outcome.agent));
      this.emit();
      return id;
    });
  }
}

function rowsHasReplacement(rows: ReadonlyMap<string, CloudAgent>, agent: CloudAgent) {
  return [...rows.values()].some(row => row.name === agent.name && !sameAgent(row, agent));
}
