/** Host-scoped facts consumed by domain rules. Provider payloads never cross this boundary. */
export interface AgentObservation {
  id: string; title: string | null; provider: string; workspaceId?: string | null;
  status: string; lastError: string | null; lastUserMessageAt: string | null;
  hasActiveTurn: boolean; pendingPermissionCount: number; archivedAt?: string | null;
  providerUnavailable?: boolean; internal?: boolean; attentionReason?: string | null;
}
export interface WorkspaceObservation {
  id: string; projectId: string; projectDisplayName: string; name: string;
  title?: string | null; archivingAt: string | null;
}
export interface AgentIdentity { id: string; title: string | null; provider: string; workspaceId?: string | null }
export interface TurnStartedFact { agent: AgentIdentity; turnId: string | null }
export interface TurnEndedFact extends TurnStartedFact {
  outcome: { kind: "completed" } | { kind: "canceled"; reason?: string } | { kind: "failed"; error: { message: string; code?: string } };
}
export interface ObservationScope { projectIds: string[]; workspaceIds: string[] }
/** Names are presentation; IDs own identity. Empty selections mean all; dimensions intersect. */
export function includesScope(scope: ObservationScope, subject: { projectId?: string | null; workspaceId?: string | null }): boolean {
  return (!scope.projectIds.length || scope.projectIds.includes(subject.projectId ?? "ungrouped")) &&
    (!scope.workspaceIds.length || scope.workspaceIds.includes(subject.workspaceId ?? "ungrouped"));
}
