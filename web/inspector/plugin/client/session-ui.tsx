import { t } from "../shared/text";
import { Fragment } from "react";
import { Text, View } from "react-native";
import { Icon } from "../../host/ui";
import type { PluginTheme } from "../../host/contracts";
import { completionReadState, stopShown, stopUnseen, waitingKind, type AgentRecord, type AgentStatus } from "../shared/model";
import { stopLabels } from "../shared/stop-reason";
import { ProjectMark, fontSize, hue, space } from "../../uiux/systems/inspector/inspector.native";

/** Session status, result-read and identity semantics; no generic controls live here. */
/** @aiPurpose Provide statusNames presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const statusNames: Record<AgentStatus, string> = {
  get running() { return t("secondaryUi.running"); }, get waiting() { return t("secondaryUi.waitingForApproval"); }, get idle() { return t("secondaryUi.waitingForReply"); }, get completed() { return t("secondaryUi.completed"); }, get quiet() { return t("secondaryUi.idle"); },
  get error() { return t("secondaryUi.error"); }, get initializing() { return t("secondaryUi.starting"); }, get closed() { return t("secondaryUi.closed"); }, get canceled() { return t("secondaryUi.canceled"); },
};
/** The state a row is announced with: a question or a plan is not an ordinary approval, a usage limit not an ordinary stop. */
/** @aiPurpose Provide stateName presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function stateName(agent: AgentRecord): string {
  const waiting = waitingKind(agent);
  if (waiting === "question") return t("secondaryUi.question");
  if (waiting === "plan") return t("secondaryUi.planNeedsApproval");
  // The row shows why the turn stopped (额度用尽 / 空轮 / 可能未收尾) wherever the stop applies; it is announced the same way.
  if (stopShown(agent)) return stopLabels[agent.stop!.kind];
  return statusNames[agent.status];
}
/** Screen-reader state of a session; completed ones say whether their result was read. */
/** @aiPurpose Provide readLabel presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function readLabel(agent: AgentRecord): string {
  const state = completionReadState(agent), waiting = waitingKind(agent);
  if (waiting === "question") return t("secondaryUi.questionWaitingForYourAnswer");
  if (waiting === "plan") return t("secondaryUi.planNeedsApprovalWaitingForYou");
  if (stopShown(agent)) return agent.stop?.kind === "quota" ? t("secondaryStatus.quotaExhausted")
    : t("secondaryUi.message", { value1: stateName(agent), value2: stopUnseen(agent) ? t("secondaryUi.notReadYet") : t("secondaryUi.read") });
  return state === "unread" ? t("secondaryUi.completedResultUnread") : state === "read" ? t("secondaryUi.completedRead") : agent.background && agent.status === "running" ? t("secondaryUi.runningBackgroundSubtasksActive") : statusNames[agent.status];
}
/** @aiPurpose Provide checkoutName presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function checkoutName(agent: AgentRecord): string {
  return agent.checkoutKind === "worktree" ? agent.worktreeName || t("secondaryUi.unnamedWorktree") : agent.checkoutKind === "main" ? t("secondaryUi.mainWorkspace") : agent.checkoutKind === "directory" ? t("secondaryUi.localFolder") : t("secondaryUi.unknownWorkspace");
}
/** @aiPurpose Provide providerLine presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const providerLine = (agent: Pick<AgentRecord, "provider" | "model">) => agent.model ? `${agent.provider} · ${agent.model}` : agent.provider;

/** project · worktree · branch on one line: the project's colour mark, git parts in violet; each part keeps its own accessible name. */
/** @aiPurpose Compose IdentityLine presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function IdentityLine({ theme, agent, project = true, compact = false }: { theme: PluginTheme; agent: AgentRecord; project?: boolean; compact?: boolean }) {
  const muted = theme.colors.foregroundMuted, git = hue(theme, "violet");
  const parts: Array<{ key: string; label: string; text: string; icon?: string }> = [];
  if (project) parts.push({ key: "project", label: t("secondaryUi.project", { value1: agent.projectName }), text: agent.projectName });
  // Phones keep two parts: a worktree is usually named after its branch.
  if (agent.checkoutKind === "worktree" && !(compact && agent.branch)) parts.push({ key: "worktree", label: t("secondaryUi.worktree", { name: checkoutName(agent) }), text: checkoutName(agent), icon: "GitFork" });
  else if (!agent.branch) parts.push({ key: "checkout", label: t("secondaryUi.workspaceType", { value1: checkoutName(agent) }), text: checkoutName(agent) });
  if (agent.branch) parts.push({ key: "branch", label: t("secondaryUi.branch", { value1: agent.branch }), text: agent.branch, icon: "GitBranch" });
  return <View style={{ flexDirection: "row", alignItems: "center", flexShrink: 1, minWidth: 0, gap: space.xs + 2 }}>
    {parts.map((part, index) => <Fragment key={part.key}>
      {index > 0 ? <Text style={{ color: muted, fontSize: fontSize.meta }}>·</Text> : null}
      <View accessibilityLabel={part.label} style={{ flexDirection: "row", alignItems: "center", gap: part.key === "project" ? 5 : 3, flexShrink: 1, minWidth: 0 }}>
        {part.key === "project" ? <ProjectMark theme={theme} name={part.text} size={14} /> : null}
        {part.icon ? <Icon name={part.icon} size={12} color={git} /> : null}
        <Text numberOfLines={1} style={{ flexShrink: 1, color: muted, fontSize: fontSize.meta, lineHeight: 17 }}>{part.text}</Text>
      </View>
    </Fragment>)}
  </View>;
}

