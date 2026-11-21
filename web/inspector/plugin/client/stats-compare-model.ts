/** @aiPurpose Project native historical metrics into descriptive comparisons without inventing content samples or quality scores. */
import type { StatsBasis, StatsBreakdownRow, StatsMetrics, StatsSnapshot } from "../shared/stats";
import { averageTurnMs, hasContent, successRate } from "../shared/stats-view";

export type CompareDimension = "models" | "providers" | "vendors";
export type ComparisonPair = readonly [string, string];
export type MetricId = typeof metricGroups[number]["metrics"][number];
export type MetricUnit = "count" | "milliseconds" | "rate";
export interface MetricReading { value: number | null; unit: MetricUnit; numerator: number | null; denominator: number | null; complete?: boolean }

/** @aiPurpose Define descriptive groups in reading order. @aiUsage Consume IDs with the page-owned literal translation map. */
export const metricGroups = [
  { id: "activity", metrics: ["sessions", "turns", "activeDays"] },
  { id: "outcomes", metrics: ["completed", "failed", "canceled", "completionRate"] },
  { id: "time", metrics: ["runMs", "averageTurnMs", "approvals", "waitMs"] },
  { id: "content", metrics: ["linesAdded", "linesRemoved", "filesChanged", "toolCalls", "toolFailureRate", "userMessages", "assistantMessages", "errors"] },
] as const;

const contentFields = ["userMessages", "assistantMessages", "toolCalls", "toolFailures", "edits", "linesAdded", "linesRemoved", "filesChanged", "shellCalls", "webCalls", "subagentCalls", "errors", "compactions"] as const satisfies readonly (keyof StatsMetrics)[];

/** @aiPurpose Detect recorded content, including complete zero-tool samples and error-only windows. @aiUsage This is evidence of some content, never the number or proportion of covered turns. */
export function hasRecordedContent(row: StatsMetrics): boolean {
  return hasContent(row) || contentFields.some(field => row[field] > 0);
}

/** @aiPurpose Retain native identities and exclude unknown/overflow aggregates from object selection. @aiUsage Resolve each dimension independently after a snapshot change. */
export function comparisonRows(stats: StatsSnapshot, dimension: CompareDimension): StatsBreakdownRow[] {
  return stats[dimension].filter(row => row.key !== "unknown" && row.key !== "__other__" && (row.turns > 0 || row.sessions > 0))
    .sort((a, b) => b.turns - a.turns || b.sessions - a.sessions || a.key.localeCompare(b.key));
}

/** @aiPurpose Keep available selections across range refreshes without ever comparing an object with itself. @aiUsage Persist keys per dimension; null means fewer than two selectable objects. */
export function resolveComparisonPair(rows: readonly StatsBreakdownRow[], preferred?: ComparisonPair): ComparisonPair | null {
  if (rows.length < 2) return null;
  const exists = (key: string | undefined): key is string => key !== undefined && rows.some(row => row.key === key);
  const a = exists(preferred?.[0]) ? preferred[0] : rows.find(row => row.key !== preferred?.[1])!.key;
  const b = exists(preferred?.[1]) && preferred[1] !== a ? preferred[1] : rows.find(row => row.key !== a)!.key;
  return [a, b];
}

/** @aiPurpose Use each metric's actual observation denominator and preserve missing values. @aiUsage Content totals are recorded counters; their sample denominator is recorded independently of turn starts. */
export function readComparisonMetric(row: StatsBreakdownRow, id: MetricId): MetricReading {
  const plain = (value: number | null, unit: MetricUnit = "count"): MetricReading => ({ value, unit, numerator: value, denominator: null });
  switch (id) {
    case "completionRate": return { value: successRate(row), numerator: row.completed, denominator: row.completed + row.failed, unit: "rate" };
    case "averageTurnMs": return { value: averageTurnMs(row), numerator: row.runMs, denominator: row.runSamples, unit: "milliseconds" };
    case "toolFailureRate": return { value: row.toolCalls > 0 ? row.toolFailures / row.toolCalls : null, numerator: hasRecordedContent(row) ? row.toolFailures : null, denominator: row.toolCalls, unit: "rate" };
    case "runMs": return plain(row.runSamples > 0 ? row.runMs : null, "milliseconds");
    case "waitMs": return plain(row.waitMs, "milliseconds");
    case "linesAdded": case "linesRemoved": case "filesChanged": case "toolCalls": case "userMessages": case "assistantMessages": case "errors":
      return plain(hasRecordedContent(row) ? row[id] : null);
    default: return plain(row[id]);
  }
}

/** @aiPurpose Keep recorded positive facts while withholding unsupported zeros and differences from partial breakdowns. @aiUsage The snapshot basis belongs to the host query, not to the current directory alone. */
export function readComparisonWithBasis(row: StatsBreakdownRow, id: MetricId, basis: StatsBasis): MetricReading {
  const reading = readComparisonMetric(row, id);
  const content = contentFields.includes(id as typeof contentFields[number]) || id === "toolFailureRate";
  const family = id === "sessions" ? basis.sessions : content ? basis.content : basis.lifecycle;
  const complete = family.status === "complete" && basis.attribution.status === "complete" && (!content || row.contentSamples === row.contentComplete);
  const value = !complete && reading.value === 0 ? null : reading.value;
  return { ...reading, value, complete };
}

/** @aiPurpose Compute signed A minus B in the metric's native unit. @aiUsage Rate differences are multiplied by 100 for percentage-point presentation, never ranked. */
export function comparisonDifference(a: MetricReading, b: MetricReading): number | null {
  return a.value === null || b.value === null || a.complete === false || b.complete === false ? null : a.value - b.value;
}

/** @aiPurpose Format unabridged values with at most two fractional digits. @aiUsage Pass rate values already multiplied by 100; unit and rounding explanations belong to the view. */
export function formatComparisonNumber(value: number, locale: string, signed = false): string {
  const text = value.toLocaleString(locale, { maximumFractionDigits: 2 });
  const rounded = Math.round(value * 100) / 100;
  if (Object.is(rounded, -0) || rounded === 0) return "0";
  return signed && rounded > 0 ? `+${text}` : text;
}
