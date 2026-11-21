/** @aiPurpose Standardize host-scoped observation identity, provenance, completeness and data-source discovery. */
import { defineRpc } from "../../host/contracts";
import { z } from "zod";
import { directoryHealthSchema } from "./source-health";

export const observationScopeSchema = z.object({ projectIds: z.array(z.string()).default([]), workspaceIds: z.array(z.string()).default([]) });
export const sourceHealthSchema = z.object({ source: z.literal("agentx-directory"), observedAt: z.string().nullable(), complete: z.boolean(), error: z.string().nullable(), revision: z.number().int().nonnegative(),
  status: directoryHealthSchema.shape.status, lastAttemptAt: directoryHealthSchema.shape.lastAttemptAt,
  lastSuccessAt: directoryHealthSchema.shape.lastSuccessAt, errorCode: directoryHealthSchema.shape.errorCode });
export const observedAgentSchema = z.object({
  id: z.string(), title: z.string().nullable(), provider: z.string(), workspaceId: z.string().nullable(), projectId: z.string().nullable(),
  parentAgentId: z.string().nullable(), status: z.string(), activeTurn: z.boolean(), pendingPermissionCount: z.number().int(),
  attentionReason: z.string().nullable(), lastError: z.string().nullable(), lastUserMessageAt: z.string().nullable(), archivedAt: z.string().nullable(),
});
export const observedWorkspaceSchema = z.object({ id: z.string(), projectId: z.string(), projectName: z.string(), name: z.string(), archivingAt: z.string().nullable() });
export const dataSources = [
  { id: "agentx-directory", owner: "AgentX", persistence: "cache", contract: "observatory.data.snapshot" },
  { id: "telemetry", owner: "TelemetryStore", persistence: "atomic-checkpoint", contract: "observatory.telemetry.get" },
  { id: "recovery", owner: "RecoveryService", persistence: "journal", contract: "recovery.list" },
  { id: "deliveries", owner: "DeferredSend", persistence: "journal", contract: "observatory.messages.list" },
  { id: "tokens", owner: "TokenStore", persistence: "contribution-checkpoint", contract: "observatory.tokens.get" },
] as const;
export const observationSnapshotSchema = z.object({
  health: sourceHealthSchema, agents: z.array(observedAgentSchema), workspaces: z.array(observedWorkspaceSchema),
  sources: z.array(z.object({ id: z.string(), owner: z.string(), persistence: z.string(), contract: z.string() })),
});
export type ObservationSnapshot = z.infer<typeof observationSnapshotSchema>;
/** @aiPurpose Read a complete native directory projection with explicit freshness and source semantics. @aiUsage The selected plugin host scopes identity; incomplete or failed snapshots must not authorize commands. */
export const getObservationSnapshot = defineRpc({ name: "observatory.data.snapshot", input: observationScopeSchema, output: observationSnapshotSchema });
