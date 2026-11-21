/** @aiPurpose Retain timestamp contributions and observed intervals inside the lifecycle owner's atomic checkpoint. */
import { z } from "zod";
import { addStatsMetrics, emptyStatsMetrics, statsMetricsSchema, statsToolSchema, type StatsMetrics } from "../shared/stats";
import { STATS_DAY_MS, STATS_HISTORY_DAYS, type StatsDirectoryAgent, type StatsFactsState, type StatsIdentity, type StatsInterval, type StatsPoint } from "./stats-types";

export const statsIdentitySchema = z.object({ provider: z.string().max(128), model: z.string().max(240).nullable(),
  projectKey: z.string().max(1024).nullable(), projectName: z.string().max(240).nullable(), workspaceId: z.string().max(512).nullable(), metadataAt: z.iso.datetime().nullable() });
const stamp = z.number().finite().nonnegative();
export const statsPointSchema = z.object({ at: stamp, agentId: z.string().max(256), identity: statsIdentitySchema, metrics: statsMetricsSchema.partial(), tools: z.array(statsToolSchema).max(600) });
export const statsIntervalSchema = z.object({ agentId: z.string().max(256), identity: statsIdentitySchema, kind: z.enum(["active", "wait"]), start: stamp, end: stamp });
export const statsFactsSchema = z.object({ since: z.iso.datetime(), retainedFrom: z.iso.datetime(), points: z.array(statsPointSchema).max(50_000),
  intervals: z.array(statsIntervalSchema).max(50_000), cumulative: statsMetricsSchema,
  longestTurn: z.object({ ms: stamp, agentId: z.string().max(256), title: z.string().max(240), model: z.string().max(240).nullable(), at: z.iso.datetime() }).nullable(),
  sessions: z.array(z.string().max(256)).max(20_000), limited: z.boolean(),
  coverage: z.array(z.object({ start: z.iso.datetime(), end: z.iso.datetime() })).max(4096), coverageGap: z.boolean() });
const POINT_BUDGET = 12 * 1024 * 1024, INTERVAL_BUDGET = 6 * 1024 * 1024;
const iso = (at: number) => new Date(at).toISOString();
const bytes = (value: unknown) => new TextEncoder().encode(JSON.stringify(value)).byteLength;
const sparse = (metrics: Partial<StatsMetrics>): Partial<StatsMetrics> => Object.fromEntries(Object.entries(metrics).filter(([, value]) => value !== 0));

/** @aiPurpose Measure Agent activity and host busy time over exactly the same half-open support. @aiUsage Inputs may overlap; each Agent is unioned before host concurrency is calculated. */
export function intervalMetrics(intervals: readonly StatsInterval[], from = -Infinity, to = Infinity): StatsMetrics {
  const metrics = emptyStatsMetrics(), groups = new Map<string, Array<[number, number]>>();
  for (const interval of intervals) {
    const start = Math.max(from, interval.start), end = Math.min(to, interval.end);
    if (end <= start) continue;
    const key = `${interval.kind}:${interval.agentId}`, spans = groups.get(key) ?? [];
    spans.push([start, end]); groups.set(key, spans);
  }
  const edges: Array<[number, number]> = [];
  for (const [key, spans] of groups) {
    spans.sort((a, b) => a[0] - b[0]);
    let start = spans[0]![0], end = spans[0]![1];
    const add = () => {
      if (key.startsWith("wait:")) metrics.waitMs += end - start;
      else { metrics.observedActiveMs += end - start; edges.push([start, 1], [end, -1]); }
    };
    for (const span of spans.slice(1)) {
      if (span[0] <= end) end = Math.max(end, span[1]);
      else { add(); [start, end] = span; }
    }
    add();
  }
  edges.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  let parallel = 0, previous = 0;
  for (const [at, delta] of edges) {
    if (parallel > 0) metrics.busyMs += at - previous;
    parallel += delta; metrics.peakParallel = Math.max(metrics.peakParallel, parallel); previous = at;
  }
  return metrics;
}

/** @aiPurpose Own bounded detailed observations and generation totals, with no raw-hook acceptance or independent persistence. @aiUsage Telemetry mutates and checkpoints this object on its single queue. */
export class StatisticsFacts {
  private state: StatsFactsState;
  private pointBytes = 0;
  private intervalBytes = 0;
  private readonly sessions = new Set<string>();
  constructor(at: number) { this.state = { since: iso(at), retainedFrom: iso(Math.max(0, at - STATS_HISTORY_DAYS * STATS_DAY_MS)), points: [], intervals: [], cumulative: emptyStatsMetrics(), longestTurn: null, sessions: [], limited: false, coverage: [{ start: iso(at), end: iso(at) }], coverageGap: false }; }
  restore(value: unknown): void {
    this.state = statsFactsSchema.parse(value);
    this.sessions.clear();
    for (const id of this.state.sessions) this.sessions.add(id);
    this.pointBytes = this.state.points.reduce((sum, point) => sum + bytes(point), 0);
    this.intervalBytes = this.state.intervals.reduce((sum, span) => sum + bytes(span), 0);
  }
  get since(): string { return this.state.since; }
  get retainedFrom(): string { return this.state.retainedFrom; }
  get coverage() { return this.state.coverage; }
  /** Return the last known continuous edge when collection resumed after suspension. */
  heartbeat(at: number, resumed = false): number | null {
    const current = this.state.coverage.at(-1), last = current ? Date.parse(current.end) : at;
    if (at <= last) return null;
    if (resumed || at - last > 90_000) {
      this.state.coverageGap = true;
      this.state.coverage.push({ start: iso(at), end: iso(at) });
      return last;
    }
    if (current) current.end = iso(at);
    else this.state.coverage.push({ start: iso(at), end: iso(at) });
    return null;
  }
  point(point: StatsPoint): void {
    const item = { ...point, identity: { ...point.identity }, metrics: sparse(point.metrics), tools: point.tools.map(tool => ({ ...tool })) };
    if (item.at >= Date.parse(this.state.since)) addStatsMetrics(this.state.cumulative, item.metrics);
    if (item.at >= Date.parse(this.state.retainedFrom)) { this.state.points.push(item); this.pointBytes += bytes(item); }
  }
  interval(span: StatsInterval): void {
    const start = Math.max(span.start, Date.parse(this.state.retainedFrom));
    if (span.end <= start) return;
    const item = { ...span, identity: { ...span.identity }, start };
    this.state.intervals.push(item); this.intervalBytes += bytes(item);
  }
  directory(agents: readonly StatsDirectoryAgent[]): boolean {
    let changed = false;
    for (const agent of agents) {
      if (this.sessions.has(agent.id)) continue;
      if (this.sessions.size >= 20_000) { this.state.limited = true; continue; }
      this.sessions.add(agent.id); changed = true;
      const at = Date.parse(agent.createdAt);
      if (!Number.isFinite(at)) continue;
      const identity: StatsIdentity = { provider: "unknown", model: null, projectKey: null, projectName: null,
        workspaceId: null, metadataAt: null };
      // Creation time is native evidence; its configuration was only observed on discovery, not historically proven.
      this.point({ at, agentId: agent.id, identity, metrics: { sessions: 1, childSessions: agent.parentAgentId ? 1 : 0 }, tools: [] });
    }
    return changed;
  }
  longest(agentId: string, title: string | null, identity: StatsIdentity, ms: number, at: number): void {
    if (!this.state.longestTurn || ms > this.state.longestTurn.ms) this.state.longestTurn = { agentId, title: (title ?? "").slice(0, 240), model: identity.model, ms, at: iso(at) };
  }
  needsPrune(at: number): boolean {
    return Math.floor(at / STATS_DAY_MS) * STATS_DAY_MS - STATS_HISTORY_DAYS * STATS_DAY_MS > Date.parse(this.state.retainedFrom)
      || this.state.points.length > 50_000 || this.pointBytes > POINT_BUDGET
      || this.state.intervals.length > 50_000 || this.intervalBytes > INTERVAL_BUDGET || this.state.coverage.length > 4096;
  }
  /** Finalize a prefix only after the owner has split open intervals at that boundary. */
  prune(at: number, open: readonly StatsInterval[]): number {
    let edge = Math.max(Date.parse(this.state.retainedFrom), Math.floor(at / STATS_DAY_MS) * STATS_DAY_MS - STATS_HISTORY_DAYS * STATS_DAY_MS);
    const droppedPoints = new Set<StatsPoint>();
    const capacityEdge = <T>(rows: T[], size: number, budget: number, time: (row: T) => number, dropped?: Set<T>): number => {
      if (rows.length <= 50_000 && size <= budget) return 0;
      const sorted = [...rows].sort((a, b) => time(a) - time(b));
      let index = 0;
      // Leave headroom so a busy host does not sort on every admission near the cap.
      while (index < sorted.length && (sorted.length - index > 45_000 || size > budget * 0.9)) {
        const row = sorted[index++]!; size -= bytes(row); dropped?.add(row);
      }
      this.state.limited = true;
      return time(sorted[index - 1]!);
    };
    edge = Math.max(edge, capacityEdge(this.state.points, this.pointBytes, POINT_BUDGET, point => point.at, droppedPoints),
      capacityEdge(this.state.intervals, this.intervalBytes, INTERVAL_BUDGET, span => span.end));
    if (this.state.coverage.length > 4096) {
      edge = Math.max(edge, Date.parse(this.state.coverage[this.state.coverage.length - 4096]!.start));
      this.state.limited = true;
    }
    const previous = Date.parse(this.state.retainedFrom);
    if (edge <= previous && !droppedPoints.size) return previous;
    addStatsMetrics(this.state.cumulative, intervalMetrics([...this.state.intervals, ...open], previous, edge));
    this.state.points = this.state.points.filter(point => point.at >= edge && !droppedPoints.has(point));
    this.state.intervals = this.state.intervals.filter(span => span.end > edge).map(span => ({ ...span, start: Math.max(edge, span.start) }));
    this.state.retainedFrom = iso(edge);
    this.state.coverage = this.state.coverage.filter(span => Date.parse(span.end) >= edge).map(span => ({ start: iso(Math.max(edge, Date.parse(span.start))), end: span.end }));
    this.pointBytes = this.state.points.reduce((sum, point) => sum + bytes(point), 0);
    this.intervalBytes = this.state.intervals.reduce((sum, span) => sum + bytes(span), 0);
    return edge;
  }
  snapshot(open: readonly StatsInterval[]): StatsFactsState {
    const intervals = [...this.state.intervals, ...open], cumulative = { ...this.state.cumulative };
    addStatsMetrics(cumulative, intervalMetrics(intervals));
    return { ...this.state, points: [...this.state.points], intervals, cumulative, sessions: [...this.sessions] };
  }
  serialize(): StatsFactsState { return { ...this.state, sessions: [...this.sessions] }; }
}
