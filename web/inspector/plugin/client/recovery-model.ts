import { includesScope } from "../shared/domain/observation";
import { t } from "../shared/text";
/** @aiPurpose Project recovery facts into grouped rows and compact status text without owning retry decisions. */
import { cycleLabels, type RecoveryCycle } from "../shared/automation";
import { phaseLabels, type Attempt, type Incident, type RecoveryList } from "../shared/recovery";
import type { Tone } from "./recovery-controls";

export const categoryIcons = { network: "WifiOff", timeout: "Timer", rate_limit: "Gauge", quota: "Wallet", authentication: "KeyRound", provider: "CloudOff", unknown: "CircleHelp" };
export const statusIcons = { waiting: "Clock3", checking: "Activity", recovered: "CircleCheck", blocked: "CircleAlert", paused: "Pause", stopped: "CircleStop" };
export const statusTones: Record<RecoveryCycle["status"], Tone> = {
  waiting: "warning", checking: "accent", recovered: "success", blocked: "danger", paused: "muted", stopped: "muted",
};
export function remaining(nextAt: string | null, now: number) {
  if (!nextAt) return "";
  const seconds = Math.max(0, Math.ceil((Date.parse(nextAt) - now) / 1000));
  if (!seconds) return t("personalRecovery.m021");
  if (seconds < 60) return `${seconds}s`;
  return seconds < 3600 ? `${Math.ceil(seconds / 60)}m` : `${Math.ceil(seconds / 3600)}h`;
}
export function rowStatus(item: Incident, cycle: RecoveryCycle | undefined, attempts: Attempt[], now: number, enabled: boolean) {
  const attempt = attempts.findLast((a) => a.incidentId === item.id && a.phase !== "skipped");
  if (attempt?.phase === "not_sent") return { icon: "CircleAlert", tone: "warning" as const, label: attempt.reason, text: phaseLabels.not_sent };
  if (cycle?.incidentId === item.id) {
    const frozen = cycle.status === "waiting" && !enabled;
    return { icon: frozen ? "Pause" : statusIcons[cycle.status], tone: frozen ? "muted" as const : statusTones[cycle.status],
      label: frozen ? t("personalRecovery.m022") : `${cycleLabels[cycle.status]}${cycle.nextAt ? t("personalRecovery.m023", { p0: cycle.nextAt }) : ""}`,
      text: frozen ? t("personalRecovery.m024") : cycle.status === "waiting" ? remaining(cycle.nextAt, now) : cycleLabels[cycle.status] };
  }
  if (attempt) return { icon: attempt.phase === "unknown" ? "CircleAlert" : attempt.phase === "observed_completed" ? "CircleCheck" : "Activity",
    tone: attempt.phase === "unknown" ? "danger" as const : attempt.phase === "observed_completed" ? "success" as const : "accent" as const,
    label: phaseLabels[attempt.phase], text: attempt.phase === "unknown" ? t("personalRecovery.m025") : attempt.phase === "observed_completed" ? t("personalRecovery.m026") : attempt.phase === "observed_running" ? t("personalRecovery.m027") : phaseLabels[attempt.phase] };
  return { icon: item.blockedReason ? "CircleAlert" : item.state === "detected" ? "CircleDot" : "CircleCheck",
    tone: item.blockedReason ? "warning" as const : "muted" as const,
    label: item.blockedReason || (item.state === "detected" ? t("personalRecovery.m028") : t("personalRecovery.m029")), text: item.blockedReason ? t("personalRecovery.m030") : item.state === "detected" ? t("personalRecovery.m031") : t("personalRecovery.m029") };
}
/** Counts delivery refusals separately even when an unrelated early turn was observed. */
export function recoveryOutcomeCounts(attempts: Attempt[]) {
  const counts = { submitted: 0, notSent: 0, unknown: 0, skipped: 0 };
  for (const attempt of attempts) {
    if (attempt.phase === "not_sent" || attempt.delivery?.status === "not_sent") counts.notSent++;
    else if (attempt.phase === "unknown" || attempt.phase === "submitting") counts.unknown++;
    else if (attempt.phase === "skipped") counts.skipped++;
    else counts.submitted++;
  }
  return counts;
}
export function deliveryText(attempt: Attempt) {
  const delivery = attempt.delivery;
  if (!delivery) return t("recoveryDelivery.evidenceUnavailable");
  const reason = delivery.reasonCode === "guard_rejected" && attempt.guardReason ? t(`recoveryDelivery.guard_${attempt.guardReason}`) : t(`recoveryDelivery.${delivery.reasonCode}`);
  return `${t(`recoveryDelivery.${delivery.status}`)} · ${reason}`;
}
export interface Filters { projects: string[]; workspaces: string[]; categories: string[]; search: string }
export function matches(item: Pick<Incident, "projectId" | "workspaceId" | "category" | "title" | "message" | "projectName" | "workspaceName" | "provider">, filters: Filters) {
  return includesScope({ projectIds: filters.projects, workspaceIds: filters.workspaces }, item) &&
    (!filters.categories.length || filters.categories.includes(item.category)) &&
    (!filters.search.trim() || `${item.title} ${item.message} ${item.projectName} ${item.workspaceName} ${item.provider}`.toLocaleLowerCase().includes(filters.search.trim().toLocaleLowerCase()));
}
export type GroupRow<T> = { kind: "project" | "workspace"; key: string; name: string; count: number; items: T[] } | { kind: "item"; key: string; item: T };
/** @aiPurpose Flatten collapsible project/Workspace hierarchy for one virtual list. @aiUsage Keep keys scoped by both project and Workspace to avoid name collisions. */
export function groupRows<T extends { id: string; projectId: string | null; workspaceId: string | null; projectName: string; workspaceName: string }>(items: T[], collapsed: string[]): GroupRow<T>[] {
  const projects = new Map<string, T[]>(), rows: GroupRow<T>[] = [];
  for (const item of items) {
    const id = item.projectId ?? "ungrouped";
    projects.set(id, [...(projects.get(id) ?? []), item]);
  }
  for (const [projectId, entries] of projects) {
    const key = `project:${projectId}`;
    rows.push({ kind: "project", key, name: entries[0].projectName, count: entries.length, items: entries });
    if (collapsed.includes(key)) continue;
    const workspaces = new Map<string, T[]>();
    for (const item of entries) {
      const workspaceId = item.workspaceId ?? "ungrouped";
      workspaces.set(workspaceId, [...(workspaces.get(workspaceId) ?? []), item]);
    }
    for (const [workspaceId, members] of workspaces) {
      const workspaceKey = `${key}:workspace:${workspaceId}`;
      rows.push({ kind: "workspace", key: workspaceKey, name: members[0].workspaceName, count: members.length, items: members });
      if (!collapsed.includes(workspaceKey)) rows.push(...members.map((item) => ({ kind: "item" as const, key: item.id, item })));
    }
  }
  return rows;
}

export interface RecordRow extends Pick<Incident, "projectId" | "workspaceId" | "projectName" | "workspaceName" | "category" | "provider"> {
  id: string; agentId: string | null; title: string; at: string; message: string; icon: string; tone: Tone;
}
export function recoveryRecords(data: RecoveryList): RecordRow[] {
  const incidents = new Map(data.incidents.map((i) => [i.agentId, i])), cycles = new Map(data.automation.cycles.map((c) => [c.id, c]));
  const records: RecordRow[] = data.automation.events.map((event) => {
    const cycle = event.cycleId ? cycles.get(event.cycleId) : undefined, item = event.agentId ? incidents.get(event.agentId) : undefined;
    return { id: event.id, agentId: event.agentId, at: event.at, title: cycle?.title || item?.title || t("personalRecovery.m032"),
      projectId: cycle?.projectId ?? item?.projectId ?? null, workspaceId: cycle?.workspaceId ?? item?.workspaceId ?? null,
      projectName: cycle?.projectName || item?.projectName || t("personalRecovery.m032"), workspaceName: cycle?.workspaceName || item?.workspaceName || t("personalRecovery.m033"),
      category: item?.category ?? "unknown", provider: item?.provider ?? "", message: event.message,
      icon: event.kind === "recovered" ? "CircleCheck" : ["blocked", "scan_error"].includes(event.kind) ? "CircleAlert" : event.kind === "policy" ? "Settings2" : "History",
      tone: event.kind === "recovered" ? "success" : ["blocked", "scan_error"].includes(event.kind) ? "danger" : "muted" };
  });
  for (const attempt of data.attempts) {
    const item = incidents.get(attempt.agentId);
    records.push({ id: attempt.id, agentId: attempt.agentId, title: item?.title ?? attempt.agentId, at: attempt.updatedAt,
      projectId: item?.projectId ?? null, workspaceId: item?.workspaceId ?? null,
      projectName: item?.projectName ?? t("personalRecovery.m034"), workspaceName: item?.workspaceName ?? t("personalRecovery.m035"), category: item?.category ?? "unknown", provider: item?.provider ?? "",
      message: `${attempt.trigger === "automatic" ? t("personalRecovery.m036") : t("personalRecovery.m037")} · ${phaseLabels[attempt.phase]} · ${attempt.reason}${attempt.delivery ? ` · ${deliveryText(attempt)}` : ""}`,
      icon: attempt.trigger === "automatic" ? "Zap" : "Hand", tone: attempt.phase === "unknown" ? "danger" : attempt.phase === "not_sent" ? "warning" : attempt.phase === "observed_completed" ? "success" : "muted" });
  }
  return records.sort((a, b) => b.at.localeCompare(a.at));
}
