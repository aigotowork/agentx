/** Synthetic report facts for basis/cost acceptance; never starts an analysis runner. */
export function analysisEvidenceFixture(job, variant, now = Date.now()) {
  const at = delta => new Date(now + delta).toISOString();
  const full = { ...job,
    source: { epoch: "fixture-payment-root", endSeq: 28, capturedAt: at(-300000), userTurns: 4,
      lastTurnId: "fixture-payment-turn", observedTurnCounter: 4, observedGeneration: 4 },
    freshness: { state: "current", newTurns: 0, latestTurnAt: at(-360000) },
    reviewContext: { mode: "full", baseJobId: null, newRows: 28, unresolvedFindings: 0 },
    runner: { kind: "grok", label: "Grok 4.7 · xhigh", provider: null, model: "grok-4.7", thinkingOptionId: "xhigh", modeId: null, sessionAgentId: null },
    usage: { inputTokens: 12345, outputTokens: 678} };
  if (variant === "zero") return { ...full, usage: { inputTokens: 0, outputTokens: 0},
    coverage: { ...full.coverage, rows: 0, characters: 0 } };
  if (variant === "legacy") {
    const legacy = { ...job, usage: null, coverage: null };
    delete legacy.source; delete legacy.freshness; delete legacy.reviewContext; delete legacy.runner;
    return legacy;
  }
  if (variant === "incremental") return { ...full, usage: { inputTokens: 12345, outputTokens: null},
    coverage: { ...full.coverage, truncated: true },
    reviewContext: { mode: "incremental", baseJobId: "fixture-base-report", newRows: 7, unresolvedFindings: 2 },
    freshness: { state: "new_activity", newTurns: 2, latestTurnAt: at(-1000) } };
  if (variant === "missing") return { ...full, report: { ...full.report, findings: full.report.findings.map((finding,index) =>
    index ? finding : { ...finding, evidence: [...finding.evidence, "payment-root:fixture-payment-root:999-999"] }) } };
  return full;
}
