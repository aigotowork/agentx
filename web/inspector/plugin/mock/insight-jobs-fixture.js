/** Synthetic analysis jobs only: this module never attaches an SDK or starts a runner. */
const base = Date.now();
const stamp = delta => new Date(base + delta).toISOString();
export const jobsCase = () => new URLSearchParams(location.search).get("jobsCase") ?? "mixed";
let sequence = 1;
let jobs;
export function seedInsightJobs(makeJob) {
  if (jobs) return jobs;
  const agentIds = ["jobs-agent-a", "jobs-agent-b", "jobs-orphan", "jobs-completed", "jobs-agent-a", "jobs-agent-a", "jobs-agent-a"];
  jobs = ["queued", "collecting", "analyzing", "completed", "failed", "canceled", "skipped"].map((status, index) => ({
    ...makeJob(agentIds[index]), id: `jobs-${status}`, agentId: agentIds[index],
    agentTitle: index === 2 ? "目录缺失的历史 Agent" : "同名 Agent",
    status, trigger: index % 2 ? "auto" : "manual", createdAt: stamp(-index * 60_000 - 300_000), updatedAt: stamp(-index * 30_000),
    report: status === "completed" ? makeJob(agentIds[index]).report : null,
    evidence: status === "completed" ? makeJob(agentIds[index]).evidence : [],
    stage: status, error: status === "failed" ? "模拟上游响应失败" : null,
    runner: { kind: "provider", label: "Recorded runner · a deliberately long historical deployment label · model-a", provider: "codex", model: "model-a", thinkingOptionId: "max", modeId: "read-only", sessionAgentId: null },
    usage: status === "completed" ? { inputTokens: 1200, outputTokens: 300} : null,
  }));
  return jobs;
}
export function getFixtureInsightJobs() { return jobs ?? []; }
export function fixtureInsightJobsSnapshot() {
  const variant = jobsCase();
  if (variant === "error") throw new Error("fixture private storage failure");
  const entries = variant === "empty" ? [] : jobs ?? [];
  const partial = variant === "partial", truncated = variant === "truncated";
  return {
    revision: `fixture-jobs-${sequence}-${variant}`, generatedAt: new Date().toISOString(),
    jobs: entries.map(({ report, evidence, ...job }) => ({ ...job, reportAvailable: !!report, summary: report?.summary ?? null,
      health: report?.health ?? null, findings: report?.findings.length ?? 0, resolvedFindings: 0 })),
    source: { status: partial || truncated ? "partial" : "available", complete: !partial && !truncated, truncated, total: truncated ? 145 : entries.length, errorCode: partial ? "storage_error" : null },
    retention: { scope: "retained", reportLimit: 60, statusLimit: 60, reportBytesLimit: 33554432 },
  };
}
export function cancelFixtureInsightJob(input) {
  const entry = jobs?.find(job => job.id === input.jobId);
  if (!entry) throw new Error("fixture job missing");
  const variant = jobsCase();
  if (variant === "cancel-unknown") throw new Error("fixture cancellation reply lost");
  if (variant === "cancel-mismatch") return { ...structuredClone(entry), id: "another-job", status: "canceled" };
  if (variant === "cancel-completed") Object.assign(entry, { status: "completed", stage: "completed" });
  else if (variant === "cancel-failed") Object.assign(entry, { status: "failed", error: "模拟取消前分析失败" });
  else if (["queued", "collecting", "analyzing"].includes(entry.status)) Object.assign(entry, { status: "canceled", stage: "canceled" });
  entry.updatedAt = stamp(++sequence);
  return structuredClone(entry);
}
export function fixtureInsightJobsRevision() { return sequence; }
