/** @aiPurpose Define stable recovery language and values without SDK, UI, schema, filesystem or RPC dependencies. */
export type Category = "network" | "timeout" | "rate_limit" | "quota" | "authentication" | "provider" | "unknown";
export type Phase = "submitting" | "accepted" | "not_sent" | "unknown" | "observed_running" | "observed_completed" | "failed_again" | "canceled" | "skipped";
/** Delivery facts describe this reservation only; an acknowledgement never proves recovery. */
export type RecoveryDelivery =
  | { status: "acknowledged"; messageId: string; reasonCode: "ledger_sent" }
  | { status: "not_sent"; messageId: string; reasonCode: "ledger_busy" | "target_gone" | "guard_rejected" | "consent_required" }
  | { status: "unknown"; messageId: string; reasonCode: "ledger_unsettled" | "ledger_conflict" | "send_error" | "send_timeout" | "reloaded"; relatedMessageId?: string };
export type RecoveryGuardReason = "recovery_inactive" | "incident_changed" | "generation_changed" | "agent_gone" | "agent_busy" |
  "permission_pending" | "provider_unavailable" | "baseline_changed" | "stop_evidence_missing" | "automation_disabled" |
  "policy_changed" | "outside_scope" | "cycle_changed";
export interface Incident {
  id: string; revision: number; agentId: string; title: string; provider: string;
  workspaceId: string | null; projectId: string | null; workspaceName: string; projectName: string;
  turnId: string | null; generation: number; source: "hook" | "snapshot";
  message: string; code?: string; category: Category; observedAt: string;
  lastUserMessageAt: string | null; errorFingerprint: string | null; baselineCaptured: boolean;
  state: "detected" | "superseded" | "submitted"; blockedReason: string | null;
}
export interface Attempt {
  id: string; operationId: string; incidentId: string; revision: number; agentId: string;
  phase: Phase; reason: string; createdAt: string; updatedAt: string;
  baselineGeneration: number; observedGeneration: number | null; observedTurnId: string | null;
  trigger: "manual" | "automatic"; policyRevision?: number;
  /** Null until delivery evidence is recorded. Absence of evidence never proves that nothing was sent. */
  delivery: RecoveryDelivery | null; guardReason?: RecoveryGuardReason; previousAttemptId?: string;
}
export interface Target { incidentId: string; revision: number }
export interface Policy { enabled: boolean; scanIntervalSeconds: number; projectIds: string[]; workspaceIds: string[]; revision: number }
export interface RecoveryCycle {
  id: string; agentId: string; incidentId: string; title: string; projectId: string | null; workspaceId: string | null;
  projectName: string; workspaceName: string;
  status: "waiting" | "checking" | "recovered" | "blocked" | "paused" | "stopped";
  pauseSource: "user" | "scope" | null; retryIndex: number; attemptCount: number; currentAttemptId: string | null;
  nextAt: string | null; reason: string; createdAt: string; updatedAt: string;
}
export interface AuditEvent {
  id: string; at: string; agentId: string | null; cycleId: string | null;
  kind: "scheduled" | "submitted" | "retry" | "recovered" | "blocked" | "paused" | "resumed" | "stopped" | "policy" | "scan_error" | "scan_restored";
  message: string;
}
export interface Automation {
  policy: Policy; cycles: RecoveryCycle[]; events: AuditEvent[];
  monitor: { scannedAt: string | null; scannedAgents: number; error: string | null };
  catalog: { projects: Array<{ id: string; name: string }>; workspaces: Array<{ id: string; name: string }> };
}
export interface RecoveryState { incidents: Incident[]; attempts: Attempt[]; generations: Record<string, number>; automation: Automation }

/** @aiPurpose Preserve the Agent-wide barrier for unfinished recovery observations. @aiUsage Check every reservation before admitting another send or independent cycle. */
export function blocksAgentRecovery(attempt: Attempt): boolean {
  return ["submitting", "accepted", "unknown", "observed_running"].includes(attempt.phase);
}

/** @aiPurpose Distinguish an independent failed turn from an unresolved recovery observation. @aiUsage Current eligibility and policy must also pass; this never authorizes replay of the previous reservation. */
export function canStartIndependentCycle(item: Incident, cycle: RecoveryCycle, attempts: readonly Attempt[]): boolean {
  if (!["blocked", "checking"].includes(cycle.status) || item.agentId !== cycle.agentId || item.id === cycle.incidentId ||
    item.source !== "hook" || item.state !== "detected") return false;
  const previous = attempts.find(attempt => attempt.id === cycle.currentAttemptId);
  return !!previous && previous.agentId === item.agentId &&
    ["observed_completed", "failed_again", "canceled"].includes(previous.phase) && previous.observedGeneration !== null &&
    item.generation > previous.observedGeneration && !attempts.some(attempt => attempt.agentId === item.agentId && blocksAgentRecovery(attempt));
}

export const BACKOFF_MINUTES = [5, 20, 60] as const;
export const defaultPolicy: Policy = { enabled: false, scanIntervalSeconds: 60, projectIds: [], workspaceIds: [], revision: 0 };
export function emptyAutomation(): Automation {
  return { policy: { ...defaultPolicy, projectIds: [], workspaceIds: [] }, cycles: [], events: [], monitor: { scannedAt: null, scannedAgents: 0, error: null }, catalog: { projects: [], workspaces: [] } };
}

// i18n-ignore: Fixed recovery instruction is command data and must remain stable across display languages.
export const CONTINUE_PROMPT = "请继续本会话中尚未完成的工作。先核实当前进度及上一次失败操作是否已经产生结果，再从可确认的位置继续；如仍有阻塞，请明确说明。";
export function classifyError(message: string, code = ""): Category {
  const value = `${code} ${message}`;
  if (/insufficient[_\s-]*(quota|credit|balance)|out of credits|billing|payment required|余额|配额耗尽|402\b/i.test(value)) return "quota";
  if (/unauthori[sz]ed|forbidden|invalid.{0,12}(key|token)|authentication|token.{0,12}expired|\b40[13]\b|认证|登录失效/i.test(value)) return "authentication";
  if (/rate.?limit|too many requests|\b429\b|限流/i.test(value)) return "rate_limit";
  if (/timed?\s*out|timeout|etimedout|超时/i.test(value)) return "timeout";
  if (/econn|enotfound|eai_again|network|socket|fetch failed|connection.{0,12}(lost|closed|reset|refused)|断网|网络/i.test(value)) return "network";
  if (/\b50[0234]\b|overloaded|unavailable|high demand|internal.server|upstream|上游|服务不可用/i.test(value)) return "provider";
  return "unknown";
}

