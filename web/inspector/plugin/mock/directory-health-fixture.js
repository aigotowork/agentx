/** Synthetic directory health; local token log coverage remains independent. */
export function fixtureDirectoryHealth(now = Date.now()) {
  const scenario = new URLSearchParams(globalThis.location?.search ?? "").get("scenario");
  const status = globalThis.__inspectorMockDirectoryHealthStatus ?? (scenario?.startsWith("directory-") ? scenario.slice("directory-".length) : "healthy");
  const success = status === "healthy" ? new Date(now).toISOString() : status === "stale" ? new Date(now - 12 * 60_000).toISOString() : null;
  return { source: "agentx-directory", status, complete: status === "healthy", revision: status === "healthy" ? 3 : 2,
    observedAt: new Date(now).toISOString(), lastAttemptAt: new Date(now).toISOString(), lastSuccessAt: success,
    errorCode: status === "healthy" ? null : "read_failed" };
}
