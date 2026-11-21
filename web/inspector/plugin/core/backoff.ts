import { includesScope } from "../shared/domain/observation";
import { t } from "../shared/text";
/** @aiPurpose Own recovery-chain transitions, retry deadlines, scope guards and durable audit events. */
import { randomUUID } from "../../host/identity";
import { BACKOFF_MINUTES, canStartIndependentCycle, type AuditEvent, type Policy, type RecoveryCycle } from "../shared/domain/recovery";
import type { Incident } from "../shared/domain/recovery";
import type { RecoveryState } from "../shared/domain/recovery";

export const iso = (time: number) => new Date(time).toISOString();
export const delayMs = (index: number) => BACKOFF_MINUTES[Math.min(index, 2)] * 60000;
export function inScope(item: Incident, policy: Policy) {
  return includesScope(policy, item);
}
export function currentCycle(state: RecoveryState, agentId: string) {
  return state.automation.cycles.findLast((cycle) => cycle.agentId === agentId);
}
export function audit(state: RecoveryState, time: number, kind: AuditEvent["kind"], message: string, cycle?: RecoveryCycle) {
  state.automation.events.push({ id: randomUUID(), at: iso(time), kind, message,
    agentId: cycle?.agentId ?? null, cycleId: cycle?.id ?? null });
}
export function transition(state: RecoveryState, cycle: RecoveryCycle, status: RecoveryCycle["status"],
  time: number, kind: AuditEvent["kind"], reason: string) {
  cycle.status = status; cycle.reason = reason; cycle.updatedAt = iso(time);
  if (status !== "waiting" && status !== "paused") cycle.nextAt = null;
  audit(state, time, kind, reason, cycle);
}
export function deferCycle(state: RecoveryState, cycle: RecoveryCycle, time: number, reason: string) {
  cycle.retryIndex = Math.min(cycle.retryIndex + 1, 2);
  cycle.nextAt = iso(time + delayMs(cycle.retryIndex));
  transition(state, cycle, "waiting", time, "retry", t("personalRecovery.m091", { p0: reason, p1: BACKOFF_MINUTES[cycle.retryIndex] }));
}
/** @aiPurpose Schedule a failure without confusing old custody with a new failed turn. @aiUsage Supply current eligibility from a complete scan; retirement and successor creation share the caller's transaction. */
export function enroll(state: RecoveryState, item: Incident, time: number, currentlyEligible: boolean) {
  const existing = currentCycle(state, item.agentId);
  if (existing && existing.incidentId === item.id) return existing;
  if (existing && ["checking", "blocked", "paused", "waiting"].includes(existing.status)) {
    if (!currentlyEligible || !canStartIndependentCycle(item, existing, state.attempts)) return existing;
    // End the old custody without claiming recovery; its attempts and blocked audit remain historical facts.
    transition(state, existing, "stopped", time, "stopped", t("personalRecovery.m144"));
  }
  const cycle: RecoveryCycle = { id: randomUUID(), agentId: item.agentId, incidentId: item.id, status: "waiting",
    title: item.title, projectId: item.projectId, workspaceId: item.workspaceId, projectName: item.projectName, workspaceName: item.workspaceName,
    pauseSource: null, retryIndex: 0, attemptCount: 0, currentAttemptId: null,
    nextAt: iso(time + delayMs(0)), reason: t("personalRecovery.m092"), createdAt: iso(time), updatedAt: iso(time) };
  state.automation.cycles.push(cycle);
  audit(state, time, "scheduled", cycle.reason, cycle);
  return cycle;
}
