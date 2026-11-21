/** Exact, synthetic StatsSnapshot rows for comparison acceptance; no extra production DTO. */
import { addStatsMetrics, emptyStatsMetrics } from "../shared/stats";
import { fixtureStatsBasis } from "./stats-fixture";

export function metricComparisonFixture(snapshot) {
  const variant = new URLSearchParams(location.search).get("compareCase") ?? "recorded";
  if (variant === "error") throw new Error("模拟统计读取失败");
  const row = (key, label, vendor, metrics) => ({ ...emptyStatsMetrics(), key, label, vendor, activeDays: 0,
    lastActiveAt: "2026-11-12T00:00:00Z", running: 0, open: 0, contextUsed: null, contextMax: null, ...metrics });
  const a = row("model-a", "Model A", "openai", { sessions: 12, turns: 100, completed: 60, failed: 20, canceled: 10,
    runSamples: 40, runMs: 1200000, waitMs: 12000, approvals: 6, activeDays: 3, linesAdded: 12345, linesRemoved: 45,
    filesChanged: 7, toolCalls: 25, toolFailures: 5, userMessages: 4, assistantMessages: 8, errors: 3 });
  const b = row("model-b", "Model B", "anthropic", { sessions: 8, turns: 50, completed: 20, failed: 10, canceled: 15,
    runSamples: 10, runMs: 600000, activeDays: 2, toolCalls: 10, userMessages: 3, assistantMessages: 6 });
  const c = { ...b, key: "model-c", label: "Model C", turns: 25 };
  for (const entry of [a, b, c]) { entry.contentSamples = entry.completed + entry.failed + entry.canceled; entry.contentComplete = entry.contentSamples; }
  const contentFields = ["linesAdded", "linesRemoved", "filesChanged", "toolCalls", "toolFailures", "userMessages", "assistantMessages", "errors"];
  if (variant === "unknown") for (const entry of [a, b, c]) {
    for (const key of ["completed", "failed", "canceled", "runSamples", "runMs", "contentSamples", "contentComplete", ...contentFields]) entry[key] = 0;
  }
  if (variant === "zero") for (const entry of [a, b, c]) Object.assign(entry, emptyStatsMetrics(), { turns: 1, completed: 1, runSamples: 1, runMs: 0, userMessages: 1, contentSamples: 1, contentComplete: 1 });
  if (variant === "partial") { for (const key of contentFields) b[key] = 0; b.contentComplete = 0; b.contentPartial = b.contentSamples; }
  if (variant === "identities") a.label = c.label = "Same model with a deliberately long deployment name for narrow screens";
  const models = variant === "empty" ? [] : variant === "single" ? [a] : [a, b, c];
  // These synthetic rows have an explicit evidence window. Missing real-server
  // basis must remain unknown; only this fixture can declare its recorded zeros.
  const evidence = (status, source, grain) => ({ status, source, grain,
    observedSince: "2026-10-01T00:00:00Z", retainedFrom: "2026-10-01T00:00:00Z",
    retentionDays: 400, gap: status !== "complete", capped: false });
  const basis = {
    ...fixtureStatsBasis("2026-10-01T00:00:00Z"),
    sessions: evidence("complete", "directory", "created-at"),
    lifecycle: evidence(variant === "unknown" ? "unknown" : "complete", "lifecycle", "timestamp"),
    content: evidence(variant === "unknown" ? "unknown" : variant === "partial" ? "partial" : "complete", "timeline", "timestamp"),
    concurrency: evidence("unknown", "lifecycle", "timestamp"),
    attribution: evidence("complete", "mixed", "timestamp"),
    toolNames: evidence("partial", "timeline", "timestamp"),
    query: { previous: "unknown", calendar: "unknown", cumulative: "partial" },
  };
  const output = { ...snapshot, from: "2026-11-06", to: "2026-11-12", observedSince: "2026-10-01T00:00:00Z",
    basis,
    contentSince: variant === "partial" ? "2026-11-10T00:00:00Z" : "2026-10-01T00:00:00Z",
    models, providers: [{ ...a, key: "provider-a", label: "Harness A" }, { ...b, key: "provider-b", label: "Harness B" }],
    vendors: [{ ...a, key: "vendor-a", label: "Vendor A" }, { ...b, key: "vendor-b", label: "Vendor B" }],
    totals: models.reduce((total, entry) => addStatsMetrics(total, entry), emptyStatsMetrics()) };
  if (variant === "unknown") output.directoryHealth = { ...snapshot.directoryHealth, status: "unknown", complete: false, lastSuccessAt: null };
  if (variant === "partial") {
    output.directoryTruncated = true;
    output.directoryHealth = { ...snapshot.directoryHealth, status: "stale", complete: false, errorCode: "incomplete" };
  }
  return output;
}
