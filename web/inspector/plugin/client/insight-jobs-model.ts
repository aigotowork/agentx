/** @aiPurpose Preserve native analysis job identity and classify cancellation receipts without optimistic success. */
import type { InsightJob, InsightJobSummary } from "../shared/insights";
import type { InsightJobsSource } from "../shared/insight-jobs";

export interface InsightJobsFilters { search: string; status: InsightJob["status"] | null; trigger: InsightJob["trigger"] | null; agentId: string | null }
export interface InsightJobsView { filters: InsightJobsFilters; expanded: string[]; offset: number }
export type InsightCancellation = "pending" | "canceled" | "completed" | "failed" | "skipped" | "unknown";
/** @aiPurpose Initialize an unfiltered host window. @aiUsage Workspace names never establish scope. */
export function initialInsightJobsView(): InsightJobsView {
  return { filters: { search: "", status: null, trigger: null, agentId: null }, expanded: [], offset: 0 };
}
/** @aiPurpose Intersect search and native identity/state filters over the loaded window. @aiUsage projectName is a searchable display field only. */
export function filterInsightJobs(jobs: readonly InsightJobSummary[], filters: InsightJobsFilters): InsightJobSummary[] {
  const search = filters.search.trim().toLocaleLowerCase();
  return jobs.filter(job => (!filters.status || job.status === filters.status) && (!filters.trigger || job.trigger === filters.trigger)
    && (filters.agentId === null || job.agentId === filters.agentId)
    && (!search || [job.id, job.agentId, job.agentTitle, job.projectName, job.stage, job.runner?.label, job.runner?.provider, job.model, job.error].join(" ").toLocaleLowerCase().includes(search)));
}
/** @aiPurpose Keep same-name Agents separate. @aiUsage Full IDs disambiguate filter choices; historical labels remain display facts. */
export function insightJobAgents(jobs: readonly InsightJobSummary[]): Array<{ id: string; title: string }> {
  const agents = new Map<string, string>();
  for (const job of jobs) if (!agents.has(job.agentId)) agents.set(job.agentId, job.agentTitle || job.agentId);
  return [...agents].map(([id, title]) => ({ id, title }));
}
/** @aiPurpose Expose active analysis cancellation only. @aiUsage The manager rechecks the actual job at command time. */
export function canCancelInsightJob(job: Pick<InsightJob, "status">): boolean {
  return job.status === "queued" || job.status === "collecting" || job.status === "analyzing";
}
/** @aiPurpose Accept only a matching job's explicit canceled result. @aiUsage Throws or malformed/nonterminal/mismatched replies remain unknown and must never be resent automatically. */
export function insightCancellation(jobId: string, job: Pick<InsightJob, "id" | "status"> | null | undefined): InsightCancellation {
  if (job?.id !== jobId) return "unknown";
  return job.status === "canceled" || job.status === "completed" || job.status === "failed" || job.status === "skipped" ? job.status : "unknown";
}
/** @aiPurpose Separate a complete empty retained window from missing source evidence. @aiUsage Complete does not mean full historical retention. */
export function completeInsightJobsSource(source: InsightJobsSource): boolean {
  return source.status === "available" && source.complete && !source.truncated && source.errorCode === null;
}
