/** @aiPurpose Declare recovery-owned storage, fresh observations and ledger-backed delivery capabilities. */
import type { AgentObservation, WorkspaceObservation } from "./observation";
import type { Attempt, RecoveryState, RecoveryDelivery } from "./recovery";
/** Consumer-owned storage port: changes must serialize and persist before returning. */
export interface RecoveryStore {
  read(): Promise<RecoveryState>;
  change<T>(update: (state: RecoveryState) => T): Promise<T>;
  close(): Promise<void>;
}
/** Consumer-owned fresh observation capabilities, independent of message delivery. */
export interface AgentObservationPort {
  scan(): Promise<{ agents: AgentObservation[]; workspaces: WorkspaceObservation[] }>;
  refresh(id: string): Promise<AgentObservation | null>;
  refreshWorkspace(id: string): Promise<WorkspaceObservation | null>;
}
/** Consumer-owned Agent port. A send acknowledgement is not proof of successful recovery. */
export interface AgentPort extends AgentObservationPort {
  send(id: string, text: string, attempt: Attempt): Promise<RecoveryDelivery>;
}
export type DeliveryObservation = Pick<AgentObservation, "status" | "hasActiveTurn" | "pendingPermissionCount" | "lastUserMessageAt" | "archivedAt" | "attentionReason" | "providerUnavailable">;
