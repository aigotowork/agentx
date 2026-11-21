/** @aiPurpose Project recorded analysis facts and export them without estimating missing usage or asserting evidence reachability. */
import type { InsightEvidence, InsightJob } from "../shared/insights";

export type EvidenceAvailability = "retained" | "empty" | "missing";

const recordedNumber = (value: number | null | undefined): number | null =>
  typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : null;

/** @aiPurpose Describe the saved material for a reference. @aiUsage A saved excerpt never proves that the original timeline is still reachable. */
export function evidenceAvailability(source?: InsightEvidence): EvidenceAvailability {
  return !source ? "missing" : source.text.trim() ? "retained" : "empty";
}

/** @aiPurpose Preserve independent freshness, review and usage facts. @aiUsage Missing values remain null/unknown; zero and per-field partial Token usage remain known. */
export function analysisFacts(job: InsightJob) {
  const references = [...new Set(job.report?.findings.flatMap(finding => finding.evidence) ?? [])];
  const sources = new Map((job.evidence ?? []).map(source => [source.ref, source]));
  const evidence = references.map(ref => ({ ref, availability: evidenceAvailability(sources.get(ref)) }));
  return {
    freshness: job.freshness?.state ?? "unknown",
    mode: job.reviewContext?.mode ?? "unknown",
    inputTokens: recordedNumber(job.usage?.inputTokens),
    outputTokens: recordedNumber(job.usage?.outputTokens),
    evidence,
    retainedReferences: evidence.filter(item => item.availability === "retained").length,
    totalReferences: references.length,
  };
}

/** @aiPurpose Copy the same recorded basis used on screen, including unknowns. @aiUsage Keep report and evidence text untouched; availability is local material availability, never original-source verification. */
export function analysisReportExport(job: InsightJob) {
  const facts = analysisFacts(job);
  return {
    jobId: job.id,
    agentId: job.agentId,
    agentTitle: job.agentTitle,
    projectName: job.projectName,
    analyzedAt: job.updatedAt,
    model: job.model,
    effort: job.effort,
    runner: job.runner ?? null,
    usage: job.usage ? { inputTokens: facts.inputTokens, outputTokens: facts.outputTokens } : null,
    coverage: job.coverage,
    source: job.source ?? null,
    freshness: job.freshness ?? null,
    reviewContext: job.reviewContext ?? null,
    findingFeedback: job.findingFeedback ?? {},
    report: job.report,
    evidence: job.evidence ?? [],
    recordedFacts: facts,
  };
}
