/** Passive diagnostic fixture data; reading it never requests an Agent operation. */
import { UI_CHUNK } from "../shared/generated/ui-chunk-manifest";
export function diagnosticsFixture(variant = "healthy", now = Date.now()) {
  const at = delta => new Date(now + delta).toISOString();
  const unknown = variant === "unknown", stale = variant === "stale", closed = variant === "closed", unavailable = variant === "unavailable";
  return { capturedAt: at(0), target: { sdk: "0.15.3", clientChunk: UI_CHUNK },
    directory: { source: "agentx-directory", status: unknown ? "unknown" : unavailable ? "unavailable" : stale ? "stale" : "healthy",
      observedAt: unknown || unavailable ? null : at(stale ? -300000 : -1000), lastAttemptAt: unknown ? null : at(-1000),
      lastSuccessAt: unknown || unavailable ? null : at(stale ? -300000 : -1000), complete: !unknown && !unavailable && !stale,
      revision: unknown ? 0 : 2, errorCode: stale || unavailable ? "timeout" : null },
    bridge: { startedAt: at(-600000), session: { state: unknown ? "unknown" : closed ? "closed" : "open", epoch: unknown ? 0 : 2,
      observedAt: unknown ? null : at(-1000), lastFrameAt: unknown ? null : at(-2000), lastClosedAt: unknown ? null : at(-30000) },
      pending: closed ? 1 : 0, queued: closed ? 1 : 0,
      operations: unknown ? [] : [{ operation: "provider_subagents", attempts: 3, successes: 2, failures: 1, lastAttemptAt: at(-5000),
        lastOutcome: "failure", lastSuccessAt: at(-40000), lastSuccessEpoch: 1, lastFailureAt: at(-5000),
        lastFailureEpoch: 2, lastFailureCode: "timeout" }] } };
}
