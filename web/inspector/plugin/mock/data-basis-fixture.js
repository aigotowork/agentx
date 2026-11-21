import { fixtureStatsBasis } from "./stats-fixture";

// Current model states only: source uncertainty and capacity are explicit facts.
export function statsBasisFixture(snapshot) {
  const kind = new URLSearchParams(location.search).get("basisCase") ?? "partial";
  const since = new Date(Date.now() - 12 * 86_400_000).toISOString();
  const basis = fixtureStatsBasis(since), unavailable = kind === "unavailable", capped = kind === "capped";
  basis.admission = unavailable ? "unavailable" : capped ? "limited" : "available";
  for (const family of ["sessions", "lifecycle", "content", "concurrency", "attribution", "toolNames"]) {
    basis[family] = { ...basis[family], status: unavailable || kind === "unknown" ? "unknown" : "partial", gap: true, capped };
  }
  basis.query = { previous: unavailable ? "unknown" : "partial", calendar: "partial", cumulative: unavailable ? "unknown" : "partial" };
  const turnsKnown = snapshot.calendar.turns.map((_, index) => !unavailable && kind !== "unknown" && index >= snapshot.calendar.turns.length - 12);
  return { ...snapshot, basis, observedSince: since, contentSince: since, cumulative: { ...snapshot.cumulative, since }, revision: `${snapshot.revision}-basis-${kind}`,
    calendar: { ...snapshot.calendar, sessionsKnown: [...turnsKnown], turnsKnown } };
}

export function tokenLedgerFixture(snapshot) {
  const kind = new URLSearchParams(location.search).get("ledgerCase") ?? "known";
  const reasons = kind === "pressure" ? ["capacity", "index_capacity"] : kind === "inferred" ? ["inferred_identity"]
    : kind === "conflict" ? ["conflicting_observation"] : kind === "reading" ? ["read_budget"] : [];
  return { ...snapshot, revision: `${snapshot.revision}-ledger-${kind}`, coverage: { ...snapshot.coverage,
    truncated: { ...snapshot.coverage.truncated, read: kind === "reading" },
    ledger: { ...snapshot.coverage.ledger, reasons, paused: kind === "pressure",
      persistence: kind === "pressure" ? "capacity" : "ready" } } };
}
