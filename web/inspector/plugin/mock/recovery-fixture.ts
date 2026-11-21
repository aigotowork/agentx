import { memoryRecovery } from "./recovery-memory";
import { emptyAutomation } from "../shared/automation";
import type { RecoveryList } from "../shared/recovery";
const at = new Date().toISOString();
const state: RecoveryList = {
  scannedAt: at, scannedAgents: 40, automation: emptyAutomation(),
  incidents: [
    { id: "network", revision: 1, agentId: "retry-network", title: "视频转码 · 上游连接中断", provider: "codex", projectId: "video", workspaceId: "video-main", projectName: "视频平台", workspaceName: "main", turnId: null, generation: 0, source: "snapshot", message: "ECONNRESET: upstream connection closed", category: "network", observedAt: at, lastUserMessageAt: at, errorFingerprint: "ECONNRESET", baselineCaptured: true, state: "detected", blockedReason: null },
    { id: "quota", revision: 1, agentId: "retry-quota", title: "支付流程 · 等待审批后继续", provider: "claude", projectId: "shop", workspaceId: "shop-pay", projectName: "商城", workspaceName: "payment", turnId: null, generation: 0, source: "snapshot", message: "429 insufficient_quota", category: "quota", observedAt: at, lastUserMessageAt: at, errorFingerprint: "429", baselineCaptured: true, state: "detected", blockedReason: "等待权限处理，请打开原会话" },
  ], attempts: [],
};
state.automation.catalog = { projects: [{ id: "video", name: "视频平台" }, { id: "shop", name: "商城" }], workspaces: [{ id: "video-main", name: "视频平台 / main" }, { id: "shop-pay", name: "商城 / payment" }] };
state.automation.cycles = state.incidents.map((i, n) => ({ id: `cycle-${i.id}`, agentId: i.agentId, incidentId: i.id, title: i.title, projectId: i.projectId, workspaceId: i.workspaceId, projectName: i.projectName, workspaceName: i.workspaceName, status: n ? "blocked" : "waiting", pauseSource: null, retryIndex: 0, attemptCount: n ? 1 : 0, currentAttemptId: null, nextAt: n ? null : new Date(Date.now() + 300_000).toISOString(), reason: n ? "等待权限处理" : "5 分钟后恢复", createdAt: at, updatedAt: at }));
state.automation.events = [{ id: "event-policy", at, agentId: null, cycleId: null, kind: "policy", message: "自动恢复已关闭" }, { id: "event-waiting", at, agentId: "retry-network", cycleId: "cycle-network", kind: "scheduled", message: "首次异常，5 分钟后恢复" }];
const scenario = new URLSearchParams(globalThis.location?.search ?? "").get("scenario");
if (scenario === "recovery-not-sent" || scenario === "recovery-unknown") {
  const unknown = scenario === "recovery-unknown", incident = state.incidents[0], cycle = state.automation.cycles[0];
  incident.revision = unknown ? 2 : 3; incident.state = unknown ? "submitted" : "detected";
  const attempt = { id: "fixture-attempt-network", operationId: "fixture-operation-network", incidentId: incident.id, revision: 1, agentId: incident.agentId,
    phase: unknown ? "unknown" as const : "not_sent" as const, reason: unknown ? "等待原会话核实，不会自动补发" : "有待处理权限，本次未发送", createdAt: at, updatedAt: at,
    baselineGeneration: 0, observedGeneration: null, observedTurnId: null, trigger: "manual" as const,
    delivery: unknown ? { status: "unknown" as const, messageId: "fixture-attempt-network", reasonCode: "ledger_unsettled" as const }
      : { status: "not_sent" as const, messageId: "fixture-attempt-network", reasonCode: "guard_rejected" as const },
    ...(!unknown ? { guardReason: "permission_pending" as const } : {}) };
  state.attempts.push(attempt);
  Object.assign(cycle,{status:"blocked",currentAttemptId:attempt.id,nextAt:null,attemptCount:1,reason:attempt.reason});
}
/** Original recovery rules operate over isolated memory and synthetic observation ports. */
const invokeRecovery = memoryRecovery(state);
export async function recoveryFixture(name:string,input:unknown) {return invokeRecovery(name,input);}
