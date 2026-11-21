/** @aiPurpose Project bounded host-wide statistics while preserving metric scope and directory provenance. */
import { intervalMetrics } from "./stats-facts";
import { t } from "../shared/text";
import type { DirectoryHealth } from "../shared/domain/source-health";
import {
  emptyStatsMetrics, statsRangeDays, TOOL_KINDS, vendorNames, vendorOf,
  type StatsBasis, type StatsBasisEntry, type StatsBasisStatus, type StatsBreakdownRow, type StatsDay, type StatsMetrics, type StatsRange, type StatsSnapshot, type StatsTool,
} from "../shared/stats";
import {
  statsDayText, statsLocalDay, statsLocalHour, statsZone, STATS_HISTORY_DAYS, STATS_DAY_MS,
  type StatsDirectoryAgent, type StatsFactsState, type StatsIdentity, type StatsInterval, type StatsCoverageInterval,
} from "./stats-types";

export interface StatsSnapshotInput {
  now: number; range: StatsRange; utcOffsetMinutes: number; timeZone?: string | null;
  facts: StatsFactsState; coverage: readonly StatsCoverageInterval[];
  admission: "available" | "limited" | "unavailable";
  directory: readonly StatsDirectoryAgent[]; directoryHealth: DirectoryHealth;
  directoryObservedAt: number; ended: ReadonlyMap<string, number>;
  running: ReadonlyMap<string, number>; awaiting: ReadonlySet<string>; background: ReadonlySet<string>;
}

const metricKeys = Object.keys(emptyStatsMetrics()) as (keyof StatsMetrics)[];
const dayString = statsDayText;
const weekday = (day: number) => ((day + 3) % 7 + 7) % 7;
const active = (metrics: StatsMetrics) => metrics.sessions > 0 || metrics.turns > 0;
const text = (value: string | null | undefined, fallback: string, maximum = 240) => (value?.trim() || fallback).slice(0, maximum);
const titleOf = (agent: StatsDirectoryAgent | undefined) => text(agent?.title, t("serverStatsSnapshot.m001"));

function add(target: StatsMetrics, delta: StatsMetrics, includeSessions = true): void {
  // Iterate the metric allowlist: breakdown rows also contain numeric metadata that must not be added here.
  for (const key of metricKeys) {
    if (!includeSessions && (key === "sessions" || key === "childSessions")) continue;
    target[key] = key === "peakParallel" ? Math.max(target[key], delta[key]) : target[key] + delta[key];
  }
}

interface Group { row: StatsBreakdownRow; days: Set<number> }
type Dimension = "models" | "providers" | "vendors" | "projects";
const dimensions: readonly Dimension[] = ["models", "providers", "vendors", "projects"];

function identity(dimension: Dimension, agent: StatsIdentity | undefined) {
  if (!agent) return { key: "unknown", label: t("serverStatsSnapshot.m002"), vendor: dimension === "projects" ? null : "other" };
  const provider = text(agent.provider, "unknown", 1024), model = text(agent.model, "unknown", 1024);
  const vendor = vendorOf(agent.model, agent.provider);
  switch (dimension) {
    case "models": return { key: model, label: model.slice(0, 240), vendor };
    case "providers": return { key: provider, label: provider.slice(0, 240), vendor: vendorOf(null, agent.provider) };
    case "vendors": return { key: vendor, label: vendorNames[vendor] ?? vendor, vendor };
    case "projects": return { key: text(agent.projectKey, "unknown", 1024), label: text(agent.projectName, t("serverStatsSnapshot.m003")), vendor: null };
  }
}

function groupFor(groups: Map<string, Group>, id: { key: string; label: string; vendor: string | null }): Group {
  let group = groups.get(id.key);
  if (!group) {
    group = { row: { ...emptyStatsMetrics(), ...id, activeDays: 0, lastActiveAt: null, running: 0, open: 0, contextUsed: null, contextMax: null }, days: new Set() };
    groups.set(id.key, group);
  }
  return group;
}

function latest(group: Group, stamp: string): void {
  const at = Date.parse(stamp);
  if (Number.isFinite(at) && (group.row.lastActiveAt === null || at > Date.parse(group.row.lastActiveAt))) group.row.lastActiveAt = new Date(at).toISOString();
}

function finishGroups(groups: Map<string, Group>, limit: number, dimension: Dimension, intervals: ReadonlyMap<Group, StatsInterval[]>): StatsBreakdownRow[] {
  const sorted = [...groups.values()].sort((a, b) => b.row.turns - a.row.turns || b.row.sessions - a.row.sessions || a.row.label.localeCompare(b.row.label) || a.row.key.localeCompare(b.row.key));
  const visible = sorted.slice(0, sorted.length > limit ? limit - 1 : limit);
  if (sorted.length > limit) {
    const overflow = groupFor(new Map(), { key: "__other__", label: t("serverStatsSnapshot.m004"), vendor: dimension === "projects" ? null : "other" });
    for (const group of sorted.slice(limit - 1)) {
      add(overflow.row, group.row);
      overflow.row.running += group.row.running; overflow.row.open += group.row.open;
      for (const field of ["contextUsed", "contextMax"] as const) if (group.row[field] !== null) overflow.row[field] = (overflow.row[field] ?? 0) + group.row[field];
      if (group.row.lastActiveAt) latest(overflow, group.row.lastActiveAt);
      for (const day of group.days) overflow.days.add(day);
    }
    const combined = sorted.slice(limit - 1).flatMap(group => intervals.get(group) ?? []);
    const measured = intervalMetrics(combined);
    overflow.row.busyMs = measured.busyMs; overflow.row.peakParallel = measured.peakParallel;
    overflow.row.observedActiveMs = measured.observedActiveMs; overflow.row.waitMs = measured.waitMs;
    visible.push(overflow);
  }
  return visible.map(({ row, days }) => ({ ...row, activeDays: days.size }));
}

/** @aiPurpose Project admitted timestamps and half-open intervals in the actual requested clock. @aiUsage Directory values are current live context only; historical grouping uses frozen fact identity. */
export function buildStatsSnapshot(input: StatsSnapshotInput): StatsSnapshot {
  const { now, facts, range, utcOffsetMinutes } = input;
  const zone = statsZone(utcOffsetMinutes, input.timeZone, now), dayOf = (at: number) => statsLocalDay(at, zone);
  const today = dayOf(now), count = statsRangeDays[range], first = today - count + 1;
  const boundaries = new Map<number, number>();
  const dayStart = (day: number): number => {
    const cached = boundaries.get(day); if (cached !== undefined) return cached;
    let low = day * STATS_DAY_MS - 16 * 3_600_000, high = day * STATS_DAY_MS + 16 * 3_600_000;
    while (low < high) { const middle = Math.floor((low + high) / 2); if (dayOf(middle) < day) low = middle + 1; else high = middle; }
    boundaries.set(day, low); return low;
  };
  const fromAt = dayStart(first), previousAt = dayStart(first - count);
  const days = new Map<number, StatsMetrics>(), spans = new Map<number, StatsInterval[]>();
  const getDay = (day: number) => { let item = days.get(day); if (!item) { item = emptyStatsMetrics(); days.set(day, item); } return item; };
  const inRange = (day: number) => day >= first && day <= today;
  const groups: Record<Dimension, Map<string, Group>> = { models: new Map(), providers: new Map(), vendors: new Map(), projects: new Map() };
  const agentTurns = new Map<string, number>(), tools = new Map<string, StatsTool>();
  const punchcard = { sessions: Array<number>(168).fill(0), turns: Array<number>(168).fill(0) };
  let attributionKnown = true;
  for (const point of facts.points) {
    if (point.at > now) continue;
    const day = dayOf(point.at), metrics = { ...emptyStatsMetrics(), ...point.metrics };
    add(getDay(day), metrics);
    if (!inRange(day)) continue;
    if (!point.identity.model || !point.identity.projectKey) attributionKnown = false;
    punchcard.sessions[weekday(day) * 24 + statsLocalHour(point.at, zone)] += metrics.sessions;
    punchcard.turns[weekday(day) * 24 + statsLocalHour(point.at, zone)] += metrics.turns;
    agentTurns.set(point.agentId, (agentTurns.get(point.agentId) ?? 0) + metrics.turns);
    for (const dimension of dimensions) {
      const group = groupFor(groups[dimension], identity(dimension, point.identity)); add(group.row, metrics);
      if (active(metrics)) group.days.add(day); latest(group, new Date(point.at).toISOString());
    }
    for (const tool of point.tools) {
      const key = JSON.stringify([tool.name, tool.kind]), current = tools.get(key);
      if (current) { current.calls += tool.calls; current.failures += tool.failures; } else tools.set(key, { ...tool });
    }
  }
  const groupSpans = new Map<Group, StatsInterval[]>();
  for (const span of facts.intervals) {
    const end = Math.min(now, span.end);
    for (let cursor = span.start; cursor < end;) {
      const day = dayOf(cursor), stop = Math.min(end, dayStart(day + 1));
      if (stop <= cursor) break;
      const list = spans.get(day) ?? []; list.push({ ...span, start: cursor, end: stop }); spans.set(day, list); cursor = stop;
    }
    if (end <= fromAt || span.start > now) continue;
    for (const dimension of dimensions) {
      const group = groupFor(groups[dimension], identity(dimension, span.identity)), list = groupSpans.get(group) ?? [];
      list.push({ ...span, start: Math.max(fromAt, span.start), end }); groupSpans.set(group, list);
    }
  }
  for (const [day, intervals] of spans) add(getDay(day), intervalMetrics(intervals));
  // Group busy time is the group's own union, while active and wait time remain additive Agent observations.
  for (const [group, intervals] of groupSpans) add(group.row, intervalMetrics(intervals));
  const directory = new Map(input.directory.map(agent => [agent.id, agent]));
  const live = { open: 0, running: 0, approvals: 0, unread: 0, idle: 0 };
  const fanouts = new Map<string, number>(); let children = 0, maxDepth = 0;
  for (const agent of directory.values()) {
    const ended = input.ended.get(agent.id), nativeRunning = ended === undefined || input.directoryObservedAt > ended;
    const running = (nativeRunning && (agent.activeTurn || agent.status === "running")) || input.running.has(agent.id) || input.background.has(agent.id);
    if (!agent.archivedAt) {
      live.open++; if (running) live.running++; if (agent.pendingPermissions > 0) live.approvals++;
      if (!running && input.awaiting.has(agent.id)) live.idle++;
      else if (!running && agent.requiresAttention && agent.attentionReason === "finished") live.unread++;
    }
    if (agent.parentAgentId && inRange(dayOf(Date.parse(agent.createdAt)))) {
      children++; fanouts.set(agent.parentAgentId, (fanouts.get(agent.parentAgentId) ?? 0) + 1);
      let parent: string | null = agent.parentAgentId, depth = 0; const seen = new Set([agent.id]);
      while (parent && !seen.has(parent) && depth < 32) { seen.add(parent); depth++; parent = directory.get(parent)?.parentAgentId ?? null; }
      maxDepth = Math.max(maxDepth, depth);
    }
    const current: StatsIdentity = { provider: agent.provider, model: agent.model, projectKey: agent.projectKey, projectName: agent.projectName, workspaceId: agent.workspaceId ?? null, metadataAt: null };
    for (const dimension of dimensions) {
      const group = groupFor(groups[dimension], identity(dimension, current));
      if (!agent.archivedAt) { group.row.open++; if (running) group.row.running++; }
      for (const field of ["contextUsed", "contextMax"] as const) if (agent[field] !== null) group.row[field] = (group.row[field] ?? 0) + agent[field];
    }
  }
  const totals = emptyStatsMetrics(), previous = emptyStatsMetrics(), retainedTotals = emptyStatsMetrics();
  const activeDays: number[] = []; let busiestDay: StatsSnapshot["records"]["busiestDay"] = null;
  for (const [day, metrics] of [...days].sort(([a], [b]) => a - b)) {
    add(retainedTotals, metrics);
    if (inRange(day)) add(totals, metrics); else if (day >= first - count && day < first) add(previous, metrics);
    if (!active(metrics) || day > today) continue;
    activeDays.push(day);
    if (!busiestDay || metrics.sessions + metrics.turns > busiestDay.sessions + busiestDay.turns) busiestDay = { day: dayString(day), sessions: metrics.sessions, turns: metrics.turns };
  }
  const coverage = [...input.coverage].sort((a, b) => a.start.localeCompare(b.start));
  const covered = (from: number, to: number) => {
    if (input.admission !== "available" || facts.limited || from < Date.parse(facts.retainedFrom)) return false;
    let cursor = from;
    for (const span of coverage) { if (Date.parse(span.end) < cursor) continue; if (Date.parse(span.start) > cursor) return false; cursor = Math.max(cursor, Date.parse(span.end)); if (cursor >= to) return true; }
    return false;
  };
  const known = covered(fromAt, now), directoryKnown = input.directoryHealth.status === "healthy" && input.directoryHealth.complete;
  const status = (value: boolean): StatsBasisStatus => value ? "complete" : input.admission === "unavailable" ? "unknown" : "partial";
  const entry = (source: StatsBasisEntry["source"], grain: StatsBasisEntry["grain"], complete: boolean): StatsBasisEntry => ({
    status: status(complete), source, grain, observedSince: facts.since, retainedFrom: facts.retainedFrom, retentionDays: STATS_HISTORY_DAYS,
    gap: !known, capped: facts.limited || input.admission === "limited",
  });
  const contentKnown = totals.contentSamples === totals.contentComplete;
  const basis: StatsBasis = {
    storage: "atomic-checkpoint", admission: input.admission, since: facts.since, retainedFrom: facts.retainedFrom, historyDays: STATS_HISTORY_DAYS,
    sessions: entry("directory", "created-at", directoryKnown && known), lifecycle: entry("lifecycle", "timestamp", known),
    content: entry("timeline", "timestamp", known && contentKnown), concurrency: entry("lifecycle", "interval", known),
    attribution: entry("mixed", "timestamp", known && attributionKnown), toolNames: entry("timeline", "timestamp", known && contentKnown),
    query: { previous: status(covered(previousAt, now) && directoryKnown && contentKnown && previous.contentSamples === previous.contentComplete), calendar: "partial", cumulative: status(!facts.coverageGap && !facts.limited && input.admission === "available") },
  };
  const calendarFirst = today - weekday(today) - 52 * 7;
  const calendar: StatsSnapshot["calendar"] = { start: dayString(calendarFirst), sessions: [], turns: [], sessionsKnown: [], turnsKnown: [] };
  for (let day = calendarFirst; day <= today; day++) {
    const metrics = days.get(day), full = covered(dayStart(day), Math.min(now, dayStart(day + 1)));
    calendar.sessions.push(metrics?.sessions ?? 0); calendar.turns.push(metrics?.turns ?? 0);
    calendar.sessionsKnown.push(full && directoryKnown); calendar.turnsKnown.push(full);
  }
  basis.query.calendar = calendar.sessionsKnown.every(Boolean) && calendar.turnsKnown.every(Boolean) ? "complete" : "partial";
  let length = 0, longest = 0, longestEnd: string | null = null, lastDay: number | null = null;
  for (const day of activeDays) { length = lastDay === day - 1 ? length + 1 : 1; if (length > longest) { longest = length; longestEnd = dayString(day); } lastDay = day; }
  let current = 0, cursor = days.get(today) && active(days.get(today)!) ? today : today - 1;
  while (days.has(cursor) && active(days.get(cursor)!)) { current++; cursor--; }
  const busiest = [...agentTurns].filter(([, turns]) => turns > 0).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))[0];
  const named = [...tools.values()].sort((a, b) => b.calls - a.calls || a.name.localeCompare(b.name));
  const kinds = TOOL_KINDS.map(kind => ({ name: kind, kind, calls: named.filter(tool => tool.kind === kind).reduce((sum, tool) => sum + tool.calls, 0), failures: named.filter(tool => tool.kind === kind).reduce((sum, tool) => sum + tool.failures, 0) })).filter(kind => kind.calls > 0);
  const edges = facts.intervals.filter(span => span.kind === "active" && span.start < Math.min(span.end, now)).flatMap(span => [[span.start, span.agentId, 1], [Math.min(span.end, now), span.agentId, -1]] as Array<[number, string, number]>).sort((a, b) => a[0] - b[0] || a[2] - b[2]);
  const participants = new Map<string, number>(); let peakParallel: StatsSnapshot["records"]["peakParallel"] = null;
  for (const [at, agentId, delta] of edges) { const value = (participants.get(agentId) ?? 0) + delta; if (value > 0) participants.set(agentId, value); else participants.delete(agentId); if (!peakParallel || participants.size > peakParallel.count) peakParallel = { count: participants.size, at: new Date(at).toISOString() }; }
  return {
    generatedAt: new Date(now).toISOString(), range, utcOffsetMinutes, timeZone: zone.name, from: dayString(first), to: dayString(today),
    observedSince: facts.since, contentSince: facts.since, directoryTruncated: !input.directoryHealth.complete, directoryHealth: input.directoryHealth, basis,
    totals, previous, retainedTotals, cumulative: { since: facts.since, metrics: facts.cumulative },
    daily: Array.from({ length: count }, (_, index) => ({ ...getDay(first + index), day: dayString(first + index) })), calendar, punchcard,
    streak: { current, longest, longestEnd, activeDays: activeDays.filter(inRange).length, retainedActiveDays: activeDays.length },
    records: { busiestDay, longestTurn: facts.longestTurn, peakParallel: peakParallel?.count ? peakParallel : null,
      busiestSession: busiest ? { agentId: busiest[0], title: titleOf(directory.get(busiest[0])), turns: busiest[1] } : null },
    live, subagents: { parents: fanouts.size, children, maxFanout: Math.max(0, ...fanouts.values()), maxDepth },
    models: finishGroups(groups.models, 24, "models", groupSpans), providers: finishGroups(groups.providers, 24, "providers", groupSpans), vendors: finishGroups(groups.vendors, 12, "vendors", groupSpans), projects: finishGroups(groups.projects, 24, "projects", groupSpans),
    tools: named.slice(0, 24), toolKinds: kinds,
  };
}
