/** @aiPurpose Project existing recovery facts into client filters, labels and conservative eligibility. */
import type {
  Attempt,
  Incident,
  RecoveryCycle,
  RecoveryList,
  Target,
} from "../../shared/contracts";
import { blocksAgentRecovery } from "../../../plugin/shared/domain/recovery";
import type { IconName, Tone } from "../../../uiux/systems/recovery/recovery.web";
export type Selection = { kind: "incident" | "attempt" | "cycle"; id: string };
export interface Filters {
  search: string;
  project: string;
  workspace: string;
  category: string;
  status: string;
}
export const emptyFilters: Filters = {
  search: "",
  project: "",
  workspace: "",
  category: "",
  status: "attention",
};
export const categoryLabels: Record<Incident["category"], string> = {
  network: "网络",
  timeout: "超时",
  rate_limit: "限流",
  quota: "额度",
  authentication: "认证",
  provider: "上游服务",
  unknown: "未分类",
};
export const phaseLabels: Record<Attempt["phase"], string> = {
  submitting: "提交中",
  accepted: "已接受",
  not_sent: "明确未发送",
  unknown: "投递未知",
  observed_running: "已观察运行",
  observed_completed: "已观察完成",
  failed_again: "再次失败",
  canceled: "已取消",
  skipped: "已跳过",
};
export const cycleLabels: Record<RecoveryCycle["status"], string> = {
  waiting: "等待退避",
  checking: "观察中",
  recovered: "已恢复",
  blocked: "受阻",
  paused: "已暂停",
  stopped: "已停止",
};
export interface Status {
  label: string;
  tone: Tone;
  icon: IconName;
  key: string;
}
/**
 * @aiPurpose Project a domain attempt phase into display semantics.
 * @aiUsage Display the server attempt phase without inferring overall task completion.
 */
export function attemptStatus(attempt: Attempt): Status {
  const p = attempt.phase;
  return {
    label: phaseLabels[p],
    tone:
      p === "unknown"
        ? "unknown"
        : p === "not_sent" || p === "failed_again"
          ? "danger"
          : p === "observed_completed"
            ? "accent"
            : p === "accepted" || p === "observed_running"
              ? "info"
              : "neutral",
    icon:
      p === "unknown"
        ? "unknown"
        : p === "not_sent" || p === "failed_again"
          ? "warning"
          : p === "observed_completed"
            ? "check"
            : p === "observed_running"
              ? "run"
              : p === "accepted"
                ? "document"
                : "clock",
    key: p,
  };
}
/**
 * @aiPurpose Find the newest authoritative attempt for an incident.
 * @aiUsage Resolve the latest attempt for this incident; never join attempts by title.
 */
export function latestAttempt(
  item: Incident,
  data: RecoveryList,
): Attempt | undefined {
  return data.attempts
    .filter((a) => a.incidentId === item.id)
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0];
}
/**
 * @aiPurpose Compute conservative client eligibility feedback.
 * @aiUsage Explain a conservative client block; final command eligibility belongs to the server.
 */
export function reasonFor(item: Incident, data: RecoveryList): string | null {
  if (item.state === "superseded") return "异常已被后续状态替代";
  const blocking = data.attempts.find(
    (a) => a.agentId === item.agentId && blocksAgentRecovery(a),
  );
  if (blocking) return `${phaseLabels[blocking.phase]} · 保留原操作`;
  if (item.blockedReason) return item.blockedReason;
  if (item.state !== "detected") return "已提交，等待后续观察";
  return null;
}
/**
 * @aiPurpose Combine incident display semantics with a local request barrier.
 * @aiUsage Include the local request barrier when displaying an incident, without changing its source facts.
 */
export function incidentStatus(
  item: Incident,
  data: RecoveryList,
  localPending = false,
): Status {
  if (localPending)
    return {
      label: "请求待核实",
      tone: "unknown",
      icon: "unknown",
      key: "unknown",
    };
  const block = data.attempts
    .filter((a) => a.agentId === item.agentId && blocksAgentRecovery(a))
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0];
  if (block) return attemptStatus(block);
  if (item.state === "superseded")
    return {
      label: "已替代",
      tone: "neutral",
      icon: "archive",
      key: "history",
    };
  if (item.blockedReason)
    return { label: "受阻", tone: "warning", icon: "pause", key: "blocked" };
  if (item.state === "detected")
    return { label: "可继续", tone: "accent", icon: "run", key: "ready" };
  const last = latestAttempt(item, data);
  return last
    ? attemptStatus(last)
    : { label: "已提交", tone: "info", icon: "document", key: "submitted" };
}
/**
 * @aiPurpose Match one domain object against the current view scope and query.
 * @aiUsage Apply browser view filters only; these values never configure automatic recovery scope.
 */
export function matches(
  item: {
    title: string;
    projectId: string | null;
    workspaceId: string | null;
    projectName: string;
    workspaceName: string;
    category?: string;
    message?: string;
    agentId: string;
  },
  filters: Filters,
) {
  return (
    (!filters.project || filters.project === item.projectId) &&
    (!filters.workspace || filters.workspace === item.workspaceId) &&
    (!filters.category || item.category === filters.category) &&
    `${item.title} ${item.projectName} ${item.workspaceName} ${item.message || ""} ${item.agentId}`
      .toLocaleLowerCase()
      .includes(filters.search.trim().toLocaleLowerCase())
  );
}
/**
 * @aiPurpose Project the scoped incident collection and local pending requests.
 * @aiUsage Filter source incidents and local barriers together so pending requests cannot appear ready.
 */
export function visibleIncidents(
  data: RecoveryList,
  filters: Filters,
  pendingAgent: (agentId: string) => boolean = () => false,
) {
  return data.incidents
    .filter((item) => {
      if (!matches(item, filters)) return false;
      const pending = pendingAgent(item.agentId),
        s = incidentStatus(item, data, pending);
      switch (filters.status) {
        case "attention":
          return (
            pending ||
            item.state === "detected" ||
            data.attempts.some(
              (a) => a.agentId === item.agentId && blocksAgentRecovery(a),
            ) ||
            data.automation.cycles.some(
              (c) =>
                c.incidentId === item.id &&
                ["waiting", "checking"].includes(c.status),
            )
          );
        case "ready":
          return !pending && !reasonFor(item, data);
        case "pending":
          return [
            "unknown",
            "accepted",
            "submitting",
            "observed_running",
          ].includes(s.key);
        case "blocked":
          return s.key === "blocked";
        default:
          return true;
      }
    })
    .sort(
      (a, b) =>
        Number(!pendingAgent(b.agentId) && !reasonFor(b, data)) -
          Number(!pendingAgent(a.agentId) && !reasonFor(a, data)) ||
        b.observedAt.localeCompare(a.observedAt),
    );
}
export const toTarget = (item: Incident): Target => ({
  incidentId: item.id,
  revision: item.revision,
});
/**
 * @aiPurpose Resolve the incident related to a selected domain object.
 * @aiUsage Resolve an incident through the selected object's stable domain identity.
 */
export function selectedIncident(
  selection: Selection | null,
  data: RecoveryList,
): Incident | undefined {
  if (!selection) return;
  return selection.kind === "incident"
    ? data.incidents.find((i) => i.id === selection.id)
    : selection.kind === "attempt"
      ? data.incidents.find(
          (i) =>
            i.id ===
            data.attempts.find((a) => a.id === selection.id)?.incidentId,
        )
      : data.incidents.find(
          (i) =>
            i.id ===
            data.automation.cycles.find((c) => c.id === selection.id)
              ?.incidentId,
        );
}
export const formatTime = (value: string | null, full = false) => {
  if (!value) return "未知";
  const d = new Date(value);
  return Number.isNaN(d.valueOf())
    ? "未知"
    : new Intl.DateTimeFormat(
        "zh-CN",
        full
          ? {
              month: "2-digit",
              day: "2-digit",
              hour: "2-digit",
              minute: "2-digit",
              second: "2-digit",
              hour12: false,
            }
          : { hour: "2-digit", minute: "2-digit", hour12: false },
      ).format(d);
};
