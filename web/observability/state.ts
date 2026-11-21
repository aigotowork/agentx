/** @aiPurpose Own page-memory investigation identity, retained evidence and finite API effects. */
import { AdapterDisconnected, AdapterError, type ObservationApi } from "./api/client.js";
import type * as N from "./api/native.js";
import { configKey, sameDaemon, sameRequest, variant } from "./presentation.js";

export type Subject = "requests" | "connections" | "peers" | "hosts" | "reports";
export type DetailTab = "overview" | "configuration" | "events";
export type Availability = "starting" | "connected" | "adapter-lost" | "daemon-lost" | "closed";
export interface EventEvidence {
  head: N.ObservationHead;
  window: N.ObservationWindow | null;
  records: N.ObservationRecord[];
  gaps: Array<{ first: bigint; last: bigint }>;
  boundary: string | null;
  request: N.RequestObservationRef | null;
}
export interface Workspace {
  scope: N.ScopeRef;
  availability: Availability;
  message: string;
  busy: boolean;
  subject: Subject;
  tab: DetailTab;
  pane: "list" | "detail";
  search: string;
  activeOnly: boolean;
  daemon: N.DaemonRef | null;
  offeredDaemon: N.DaemonRef | null;
  generation: string | null;
  fenced: boolean;
  historical: boolean;
  ready: N.InspectionReady | null;
  receivedAt: N.ObservationTime | null;
  snapshot: N.RuntimeSnapshot | null;
  requests: N.RuntimePage<N.RequestObservation> | null;
  connections: N.RuntimePage<N.ConnectionObservation> | null;
  peers: N.RuntimePage<N.PeerObservation> | null;
  hosts: N.RuntimePage<N.HostObservation> | null;
  reports: N.RuntimePage<N.RetainedClientReport> | null;
  selected: N.RequestObservation | null;
  report: N.RetainedClientReport | null;
  configurations: Map<string, N.SnapshotLookup>;
  configRefs: Map<string, N.ConfigSnapshotRef>;
  left: N.ConfigSnapshotRef | null;
  right: N.ConfigSnapshotRef | null;
  comparison: N.ConfigurationRead<N.CompareConfigResult> | null;
  configurationHead: N.ConfigurationHead | null;
  events: EventEvidence | null;
  archivedEvents: EventEvidence[];
  issue: string | null;
}

/**
 * @aiPurpose Own the one-scope investigation and independent evidence lifetimes.
 * @aiUsage Construct with the real connected ObservationApi; subscribe to project semantic views.
 * @aiBehavior Reads are finite and serialized; loss retains evidence and era changes require explicit rebinding.
 */
export class Investigation {
  readonly state: Workspace;
  private readonly listeners = new Set<() => void>();
  private readonly pending = new Set<AbortController>();
  private version = 0;

  constructor(private readonly api: ObservationApi) {
    this.state = {
      scope: api.scope, availability: "starting", message: "正在读取已存在的 daemon", busy: false,
      subject: "requests", tab: "overview", pane: "list", search: "", activeOnly: false,
      daemon: null, offeredDaemon: null, generation: null, fenced: false, historical: false,
      ready: null, receivedAt: null, snapshot: null, requests: null, connections: null,
      peers: null, hosts: null, reports: null, selected: null, report: null,
      configurations: new Map(), configRefs: new Map(), left: null, right: null,
      comparison: null, configurationHead: null, events: null, archivedEvents: [], issue: null,
    };
  }

  subscribe(listener: () => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  changed(): void { for (const listener of this.listeners) listener(); }
  navigate(subject: Subject): void { this.state.subject = subject; this.state.pane = "list"; this.changed(); }
  detail(tab: DetailTab): void { this.state.tab = tab; this.state.pane = "detail"; this.changed(); }

  /** UI uses one finite operation at a time; every response is era/version checked. */
  private async perform(action: () => Promise<void>, allowFenced = false): Promise<void> {
    if (this.state.busy || this.state.availability === "closed" || (this.state.fenced && !allowFenced)) return;
    this.state.busy = true;
    this.state.issue = null;
    this.changed();
    try { await action(); }
    catch (error) {
      this.state.historical = true;
      if (error instanceof AdapterDisconnected) {
        this.state.availability = "adapter-lost";
        this.state.message = "本地适配器连接中断；保留的资料仅代表此前观测";
      } else if (error instanceof AdapterError) {
        this.state.availability = "connected";
        this.state.message = `适配器拒绝读取 · ${error.code}`;
      } else if (!(error instanceof DOMException && error.name === "AbortError")) {
        this.state.issue = "读取未完成；保留已加载资料，请显式重试";
      }
    } finally { this.state.busy = false; this.changed(); }
  }

  private async read(read: N.BrowserRead, bind = false): Promise<N.InspectionResponse | null> {
    const version = this.version;
    const controller = new AbortController();
    this.pending.add(controller);
    let result: N.ReadResult;
    try { result = await this.api.read({ expected_daemon: bind ? null : this.state.daemon, read }, controller.signal); }
    finally { this.pending.delete(controller); }
    if (version !== this.version || this.state.availability === "closed") return null;
    if ("LocalFailure" in result) {
      this.state.availability = "daemon-lost";
      this.state.historical = true;
      this.state.message = `适配器可达 · daemon 读取失败：${result.LocalFailure.stage} / ${result.LocalFailure.kind}`;
      return null;
    }
    const { ready, received_at, response } = result.Inspection;
    if ("DaemonChanged" in response || (!bind && this.state.daemon && !sameDaemon(this.state.daemon, ready.daemon))) {
      this.state.offeredDaemon = "DaemonChanged" in response ? response.DaemonChanged.actual : ready.daemon;
      this.state.fenced = true;
      this.state.historical = true;
      this.state.message = "daemon 已更换；旧请求与证据保持历史身份，请显式进入新上下文";
      return null;
    }
    const generation = typeof ready.generation.value === "object" && "Present" in ready.generation.value ? ready.generation.value.Present : null;
    if (!bind && this.state.generation !== null && generation !== null && generation !== this.state.generation) {
      this.state.fenced = true;
      this.state.historical = true;
      this.state.message = "数据代际已改变；配置与事件关联已停止，请显式进入新上下文";
      return null;
    }
    this.state.daemon = ready.daemon;
    this.state.generation = generation;
    this.state.ready = ready;
    this.state.receivedAt = received_at;
    this.state.availability = "connected";
    this.state.historical = false;
    this.state.message = "本地适配器已连接 · 有限被动读取";
    if ("Runtime" in response && "Err" in response.Runtime) this.state.issue = `运行证据不可用 · ${variant(response.Runtime.Err)}${typeof response.Runtime.Err === "object" && "DetailUnavailable" in response.Runtime.Err ? ` / ${response.Runtime.Err.DetailUnavailable}` : ""}`;
    if ("Configuration" in response && "Err" in response.Configuration) this.state.issue = `配置证据不可用 · ${variant(response.Configuration.Err)}${typeof response.Configuration.Err === "object" && "Query" in response.Configuration.Err ? ` / ${variant(response.Configuration.Err.Query)}` : ""}`;
    if ("Events" in response && "Err" in response.Events) this.state.issue = `事件证据不可用 · ${variant(response.Events.Err)}`;
    return response;
  }

  async refresh(): Promise<void> {
    await this.perform(async () => {
      const result = await this.read("Snapshot", this.state.daemon === null);
      if (!result || !("Runtime" in result) || !("Ok" in result.Runtime) || !("Snapshot" in result.Runtime.Ok)) return;
      this.state.snapshot = result.Runtime.Ok.Snapshot;
      await this.loadListBody(false);
    });
  }

  async enterCurrentContext(): Promise<void> {
    await this.perform(async () => {
      this.version++;
      this.state.fenced = false;
      this.state.offeredDaemon = null;
      this.state.daemon = null;
      this.state.generation = null;
      this.state.snapshot = null;
      this.state.requests = null;
      this.state.connections = null;
      this.state.peers = null;
      this.state.hosts = null;
      this.state.reports = null;
      this.state.configurationHead = null;
      this.archiveEvents();
      const response = await this.read("Snapshot", true);
      if (response && "Runtime" in response && "Ok" in response.Runtime && "Snapshot" in response.Runtime.Ok) {
        this.state.snapshot = response.Runtime.Ok.Snapshot;
        await this.loadListBody(false);
      }
    }, true);
  }

  private async loadListBody(more: boolean): Promise<void> {
    const subject = this.state.subject;
    const existing = this.state[subject];
    const before = more && existing ? existing.next_before ?? null : null;
    if (more && before === null) return;
    const page: N.RuntimeList = { before, limit: 48n };
    const read: N.BrowserRead = subject === "requests" ? { Requests: { select: this.state.activeOnly ? "Active" : "AllRetained", page } }
      : subject === "connections" ? { Connections: page }
      : subject === "peers" ? { Peers: page }
      : subject === "hosts" ? { Hosts: page } : { ClientReports: page };
    const response = await this.read(read);
    if (!response || !("Runtime" in response) || !("Ok" in response.Runtime)) return;
    const result = response.Runtime.Ok;
    if ("Requests" in result) this.state.requests = mergePage(this.state.requests, result.Requests, more);
    if ("Connections" in result) this.state.connections = mergePage(this.state.connections, result.Connections, more);
    if ("Peers" in result) this.state.peers = mergePage(this.state.peers, result.Peers, more);
    if ("Hosts" in result) this.state.hosts = mergePage(this.state.hosts, result.Hosts, more);
    if ("ClientReports" in result) this.state.reports = mergePage(this.state.reports, result.ClientReports, more);
  }

  async loadList(more = false): Promise<void> { await this.perform(() => this.loadListBody(more)); }

  async selectRequest(reference: N.RequestObservationRef): Promise<void> {
    if (this.state.busy) return;
    const retained = this.state.requests?.rows.find((row) => sameRequest(row.reference, reference));
    if (retained) this.state.selected = retained;
    this.state.report = null;
    this.state.tab = "overview";
    this.state.pane = "detail";
    if (!this.currentRequest(reference)) { this.state.issue = "此请求属于历史 daemon；只能查看已加载证据"; this.changed(); return; }
    await this.perform(async () => {
      const response = await this.read({ RequestDetail: reference });
      if (response && "Runtime" in response && "Ok" in response.Runtime && "RequestDetail" in response.Runtime.Ok) this.state.selected = response.Runtime.Ok.RequestDetail;
    });
  }

  async selectReport(reference: N.ClientReportRef): Promise<void> {
    if (this.state.busy) return;
    this.state.pane = "detail";
    await this.perform(async () => {
      const response = await this.read({ ClientReportDetail: reference });
      if (response && "Runtime" in response && "Ok" in response.Runtime && "ClientReportDetail" in response.Runtime.Ok) this.state.report = response.Runtime.Ok.ClientReportDetail;
    });
  }

  currentRequest(reference: N.RequestObservationRef): boolean {
    return !this.state.fenced && this.state.daemon !== null && sameDaemon(reference.daemon, this.state.daemon);
  }

  currentConfig(reference: N.ConfigSnapshotRef): boolean {
    return this.currentRequest({ daemon: reference.epoch.daemon, id: 1n }) && reference.epoch.database_generation === this.state.generation;
  }

  async loadConfiguration(reference: N.ConfigSnapshotRef, side: "left" | "right"): Promise<void> {
    if (this.state.busy) return;
    this.state[side] = reference;
    this.state.configRefs.set(configKey(reference), reference);
    this.state.comparison = null;
    this.state.tab = "configuration";
    const cached = this.state.configurations.has(configKey(reference));
    if (cached || !this.currentConfig(reference)) { this.changed(); return; }
    await this.perform(async () => {
      const result = await this.read({ ConfigurationLookup: reference });
      if (result && "Configuration" in result && "Ok" in result.Configuration && "Snapshot" in result.Configuration.Ok) {
        this.state.configurations.set(configKey(reference), result.Configuration.Ok.Snapshot.result);
      }
    });
  }

  async loadConfigurationHead(): Promise<void> {
    await this.perform(async () => {
      const result = await this.read({ ConfigurationHead: { expected: this.state.daemon && this.state.generation ? { daemon: this.state.daemon, database_generation: this.state.generation } : null } });
      if (result && "Configuration" in result && "Ok" in result.Configuration && "Head" in result.Configuration.Ok) this.state.configurationHead = result.Configuration.Ok.Head;
    });
  }

  async compare(): Promise<void> {
    const { left, right } = this.state;
    if (!left || !right) return;
    // Native comparison owns incomparable/withheld semantics, including same-ref operands.
    await this.perform(async () => {
      const response = await this.read({ ConfigurationCompare: { left, right, fields: "All" } });
      if (response && "Configuration" in response && "Ok" in response.Configuration && "Comparison" in response.Configuration.Ok) this.state.comparison = response.Configuration.Ok.Comparison;
    });
  }

  private archiveEvents(): void {
    if (this.state.events) this.state.archivedEvents.push(this.state.events);
    this.state.events = null;
  }

  async openEvents(mode: "request" | "attempt" | "all" = "request"): Promise<void> {
    const selected = this.state.selected;
    if (selected && !this.currentRequest(selected.reference)) return;
    await this.perform(async () => {
      const response = await this.read("EventsDescribe");
      if (!response || !("Events" in response) || !("Ok" in response.Events) || !("Head" in response.Events.Ok)) return;
      const head = response.Events.Ok.Head;
      this.archiveEvents();
      this.state.events = {
        head, request: selected?.reference ?? null, records: [], gaps: [], boundary: null,
        window: eventWindow(head, mode === "request" ? selected?.reference.id ?? null : null, mode === "attempt" ? selected?.origin?.attempt ?? null : null),
      };
      this.state.tab = "events";
      await this.readEventsBody();
    });
  }

  private async readEventsBody(): Promise<void> {
    const events = this.state.events;
    if (!events?.window || events.boundary) return;
    const response = await this.read({ EventsRead: { window: events.window, limit: 128n } });
    if (!response || !("Events" in response) || !("Ok" in response.Events) || !("Read" in response.Events.Ok)) return;
    const result = response.Events.Ok.Read;
    if ("Page" in result) {
      events.records.push(...result.Page.records);
      events.window = result.Page.next;
      events.head = result.Page.head;
    } else if ("Gap" in result) {
      events.gaps.push({ first: result.Gap.first_missing, last: result.Gap.last_missing });
      events.head = result.Gap.head;
      events.boundary = "Gap";
    } else {
      events.boundary = variant(result);
      events.head = "EpochChanged" in result ? result.EpochChanged.head : result.StreamChanged.head;
    }
  }

  async nextEvents(): Promise<void> { await this.perform(() => this.readEventsBody()); }

  async continueRetainedEvents(): Promise<void> {
    const events = this.state.events;
    if (!events || events.boundary !== "Gap" || !events.window) return;
    const after = events.head.oldest > 0n ? events.head.oldest - 1n : 0n;
    // An evicted fixed window may be entirely gone. Do not extend its through frontier.
    if (after > events.window.through) {
      events.boundary = "WindowExpired";
      this.changed();
      return;
    }
    events.window = { ...events.window, after: { stream: events.head.stream, sequence: after } };
    events.boundary = null;
    await this.nextEvents();
  }

  async close(): Promise<void> {
    this.version++;
    for (const controller of this.pending) controller.abort();
    this.pending.clear();
    this.state.availability = "closed";
    this.state.historical = true;
    this.state.message = "调查已关闭 · 已加载证据仍可阅读";
    this.changed();
    try { await this.api.close(); } catch { /* A closed adapter has no further browser authority. */ }
  }
}

function mergePage<T extends { reference: { id: bigint } }>(previous: N.RuntimePage<T> | null, page: N.RuntimePage<T>, more: boolean): N.RuntimePage<T> {
  if (!more || !previous) return page;
  const incoming = new Set(page.rows.map((row) => row.reference.id));
  return { ...page, rows: [...previous.rows.filter((row) => !incoming.has(row.reference.id)), ...page.rows] };
}

/**
 * @aiPurpose Form one immutable native window with exact bigint frontiers.
 * @aiUsage Supply a native head and an explicit request or attempt selector; null selects the retained scope.
 */
export function eventWindow(head: N.ObservationHead, request: bigint | null, attempt: string | null = null): N.ObservationWindow {
  return {
    after: { stream: head.stream, sequence: head.oldest > 0n ? head.oldest - 1n : 0n }, through: head.newest,
    filter: { family: null, minimum_severity: null, origin: null, request, client_attempt: attempt, request_id: null, participant: null, configuration: null, domain_event: null },
  };
}
