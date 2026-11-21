/** @aiPurpose Derive disposable exact event-time projections from the bounded canonical contribution set. */
const basename = (path: string) => path.replace(/\\/g,"/").split("/").filter(Boolean).at(-1) ?? "";
import { t } from "../../shared/text";
import { addTokenCounts, emptyTokenCounts, type TokenCounts, type TokenSource } from "../../shared/tokens";
import type { Candidate, LedgerCall } from "./types";

export interface HourBucket extends TokenCounts { at: number; source: TokenSource; model: string | null; firstAt: number; lastAt: number }
export interface SessionHour extends HourBucket { sessionId: string; cwd: string; subagentCalls: number }
export interface ProjectHour extends TokenCounts { at: number; cwd: string; lastAt: number }
export interface SessionMeta { sessionId: string; source: TokenSource; cwd: string; firstAt: number; lastAt: number }
export interface CallRecord { key: string; at: number; model: string | null; output: number; tps: number }
export interface RecordHour { at: number; speeds: Record<string, number>; fastest: CallRecord[]; biggest: CallRecord[] }
/** Names describe consumers; `at` is the actual call time, never a synthetic UTC bucket boundary. */
export interface TokenData {
  hours: Map<string, HourBucket>; sessionHours: Map<string, SessionHour>; projectHours: Map<string, ProjectHour>;
  sessions: Map<string, SessionMeta>; recordHours: Map<number, RecordHour>; recent: Map<string, LedgerCall>;
  earliestAt: number | null; latestAt: number | null; threads: Map<string, string>;
}
export const sessionKey = (source: TokenSource, id: string): string => `${source}:${id}`;
export const projectName = (cwd: string): string => (basename(cwd) || t("serverTokensData.m001")).slice(0, 240);
export const modelCalls = (call: Pick<LedgerCall, "calls">): number => call.calls ?? 1;
export const speedBin = (tps: number): number => Math.round(Math.log(Math.max(0.001, tps)) / Math.log(1.04));
export const binSpeed = (bin: number): number => Math.pow(1.04, bin);
export function callCounts(call: LedgerCall): TokenCounts {
  return { calls: modelCalls(call), input: call.input, cacheWrite: call.cacheWrite, cacheRead: call.cacheRead, output: call.output, reasoning: call.reasoning,
    genMs: call.start === null ? 0 : call.at - call.start, genOutput: call.start === null ? 0 : call.output };
}
export function rootSession(data: Pick<TokenData, "threads">, source: TokenSource, sessionId: string): string {
  if (source !== "codex") return sessionId;
  let id = sessionId; const visited = new Set<string>();
  while (data.threads.has(id) && !visited.has(id)) { visited.add(id); id = data.threads.get(id)!; }
  return visited.has(id) ? sessionId : id;
}
/** @aiPurpose Build query-only projections once per changed ledger revision. @aiUsage Persist contributions, never these maps. */
export function projectTokenCalls(rows: Iterable<Candidate>, threads: ReadonlyMap<string, string>, now: number): TokenData {
  const data: TokenData = { hours: new Map(), sessionHours: new Map(), projectHours: new Map(), sessions: new Map(), recordHours: new Map(), recent: new Map(),
    earliestAt: null, latestAt: null, threads: new Map(threads) };
  for (const { call } of rows) {
    const counts = callCounts(call), id = sessionKey(call.source, call.sessionId), key = call.key;
    data.hours.set(key, { ...counts, at: call.at, source: call.source, model: call.model, firstAt: call.at, lastAt: call.at });
    data.sessionHours.set(key, { ...counts, at: call.at, source: call.source, model: call.model, firstAt: call.at, lastAt: call.at, sessionId: call.sessionId, cwd: call.cwd,
      subagentCalls: call.subagent ? counts.calls : 0 });
    data.projectHours.set(key, { ...counts, at: call.at, cwd: call.cwd, lastAt: call.at });
    const meta = data.sessions.get(id);
    if (!meta) data.sessions.set(id, { sessionId: call.sessionId, source: call.source, cwd: call.cwd, firstAt: call.at, lastAt: call.at });
    else { meta.firstAt = Math.min(meta.firstAt, call.at); if (call.at >= meta.lastAt) { meta.cwd = call.cwd; meta.lastAt = call.at; } }
    const row = data.recordHours.get(call.at) ?? { at: call.at, speeds: {}, fastest: [], biggest: [] };
    const ms = counts.genMs, item: CallRecord = { key, at: call.at, model: call.model, output: call.output, tps: ms ? call.output / ms * 1000 : 0 };
    if (ms >= 1000 && call.output >= 100) { const bin = String(speedBin(item.tps)); row.speeds[bin] = (row.speeds[bin] ?? 0) + 1; row.fastest.push(item); }
    if (counts.calls === 1 && call.source !== "grok") row.biggest.push(item);
    data.recordHours.set(call.at, row);
    if (call.at >= now - 75 * 60_000) data.recent.set(key, call);
    data.earliestAt = Math.min(data.earliestAt ?? call.at, call.at); data.latestAt = Math.max(data.latestAt ?? call.at, call.at);
  }
  return data;
}
