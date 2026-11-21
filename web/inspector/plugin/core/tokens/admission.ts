/** @aiPurpose Own the single observation-to-contribution rule for every provider adapter and log consumer. */
import type { Candidate, LedgerCall } from "./types";

/** Request duration supported by the local-log speed projection (100 ms through 15 min). */
export const validStart = (start: number | null, at: number): number | null => start !== null && at - start >= 100 && at - start <= 900_000 ? start : null;

export type ContributionDecision = { kind: "new" | "replace"; value: Candidate } | { kind: "duplicate" | "conflict"; value: Candidate };
/** @aiPurpose Select one counted contribution for one adapter key. @aiUsage Inferred keys share this rule but cannot prove provider-level uniqueness. */
export function admitContribution(previous: Candidate | undefined, next: Candidate): ContributionDecision {
  if (!previous) return { kind: "new", value: structuredClone(next) };
  let selected: Candidate;
  if (previous.forked && !next.forked) selected = structuredClone(next);
  else if (!previous.forked && next.forked) return { kind: "duplicate", value: previous };
  else {
    const before = previous.call, after = next.call;
    if (["input", "cacheWrite", "cacheRead", "source", "subagent", "calls"].some(key => before[key as keyof typeof before] !== after[key as keyof typeof after])
      || (before.model !== null && after.model !== null && before.model !== after.model)) return { kind: "conflict", value: previous };
    const at = Math.max(before.at, after.at);
    selected = { forked: previous.forked, call: { ...before, at, model: before.model ?? after.model, cwd: before.cwd || after.cwd,
      output: Math.max(before.output, after.output), reasoning: Math.max(before.reasoning, after.reasoning), start: validStart(before.start ?? after.start, at) } };
  }
  return previous.forked === selected.forked && JSON.stringify(previous.call) === JSON.stringify(selected.call) ? { kind: "duplicate", value: previous } : { kind: "replace", value: selected };
}

export function validLedgerCall(call: LedgerCall): boolean {
  return Number.isSafeInteger(call.at) && call.at >= 0 && call.at <= 8.64e15 && [call.input, call.cacheWrite, call.cacheRead, call.output, call.reasoning, call.calls ?? 1].every(value => Number.isSafeInteger(value) && value >= 0)
    && (call.calls === undefined || call.calls >= 1) && call.reasoning <= call.output && (call.start === null || Number.isSafeInteger(call.start) && call.start >= 0 && call.start <= call.at);
}

/** One derived scalar protects exact integer addition in every subset projection; it is never persisted. */
export function tokenCallMass(call: LedgerCall): bigint {
  return [call.input, call.cacheWrite, call.cacheRead, call.output, call.reasoning, call.calls ?? 1, call.start === null ? 0 : call.at - call.start, call.start === null ? 0 : call.output]
    .reduce((sum, value) => sum + BigInt(value), 0n);
}
