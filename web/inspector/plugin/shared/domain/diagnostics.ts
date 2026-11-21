/** @aiPurpose Define bounded, passive diagnostic evidence without SDK, transport, UI or storage dependencies. */
import type { DirectoryHealth } from "./source-health";

/**
 * @aiPurpose Bound diagnostic cardinality with product-owned operation labels.
 * @aiUsage Adapters map known native request types to these buckets; arbitrary strings belong to other.
 */
export const BRIDGE_OPERATIONS = ["provider_subagents", "provider_timeline", "workspace_inspect", "workspace_restore", "agent_refresh", "attention_clear",
  "workspace_labels", "workspace_label_assign", "workspace_pin",
  "workspace_label_delete_inspect", "workspace_label_delete", "other"] as const;
/** @aiPurpose Name one fixed diagnostic bucket. @aiUsage These labels report requests; they do not declare host capability support. */
export type BridgeOperation = typeof BRIDGE_OPERATIONS[number];
/** @aiPurpose Describe a sanitized failure without carrying source error text. @aiUsage send_failed labels a local transport exception without changing the original request rejection. */
export type BridgeDiagnosticFailure = "timeout" | "closed" | "rejected" | "busy" | "disposed" | "send_failed";
/**
 * @aiPurpose Separate the currently observed IPC session from historical request outcomes.
 * @aiUsage Initial state is unknown. A frame proves open; close proves closed. Epoch increments only after a closed session reopens.
 */
export interface BridgeSessionDiagnostics {
  state: "unknown" | "open" | "closed" | "disposed";
  epoch: number;
  observedAt: string | null;
  lastFrameAt: string | null;
  lastClosedAt: string | null;
}
/**
 * @aiPurpose Retain aggregate request evidence for the current plugin process.
 * @aiUsage Outcomes across epochs are historical; null epoch means no frame send was attempted. An epoch does not prove delivery; success means a response, not a completed business action.
 */
export interface BridgeRequestDiagnostics {
  operation: BridgeOperation;
  attempts: number;
  successes: number;
  failures: number;
  lastAttemptAt: string | null;
  lastOutcome: "success" | "failure" | null;
  lastSuccessAt: string | null;
  lastSuccessEpoch: number | null;
  lastFailureAt: string | null;
  lastFailureEpoch: number | null;
  lastFailureCode: BridgeDiagnosticFailure | null;
}
/** @aiPurpose Snapshot bounded IPC evidence. @aiUsage pending includes queued requests; no payloads, request IDs or free-form errors are permitted. */
export interface BridgeDiagnostics {
  startedAt: string;
  session: BridgeSessionDiagnostics;
  pending: number;
  queued: number;
  operations: BridgeRequestDiagnostics[];
}
/**
 * @aiPurpose Carry a safe, read-only view of known diagnostic sources and artifact identity.
 * @aiUsage null source metadata means unknown; the SDK target and chunk manifest do not establish actual host capabilities.
 */
export interface DiagnosticsSnapshot {
  capturedAt: string;
  target: { sdk: "0.15.3"; clientChunk: { name: "ui"; hash: string; api: number; bytes: number } };
  directory: DirectoryHealth | null;
  bridge: BridgeDiagnostics | null;
}
