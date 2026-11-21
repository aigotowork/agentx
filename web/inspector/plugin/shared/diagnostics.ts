/** @aiPurpose Validate the read-only diagnostics RPC and serialize only explicitly allowed diagnostic facts. */
import { defineRpc } from "../../host/contracts";
import { z } from "zod";
import { BRIDGE_OPERATIONS, type BridgeDiagnostics, type DiagnosticsSnapshot } from "./domain/diagnostics";
import { directoryHealthSchema } from "./source-health";

export type { BridgeDiagnostics, DiagnosticsSnapshot } from "./domain/diagnostics";
const at = z.iso.datetime().nullable();
const count = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
const epoch = count.nullable();
/** @aiPurpose Enforce the fixed bridge evidence whitelist. @aiUsage Parsing strips unknown fields, including nested source metadata. */
export const bridgeDiagnosticsSchema = z.object({
  startedAt: z.iso.datetime(),
  session: z.object({ state: z.enum(["unknown", "open", "closed", "disposed"]), epoch: count, observedAt: at, lastFrameAt: at, lastClosedAt: at }),
  pending: count.max(24), queued: count.max(16),
  operations: z.array(z.object({
    operation: z.enum(BRIDGE_OPERATIONS), attempts: count, successes: count, failures: count, lastAttemptAt: at,
    lastOutcome: z.enum(["success", "failure"]).nullable(), lastSuccessAt: at, lastSuccessEpoch: epoch,
    lastFailureAt: at, lastFailureEpoch: epoch, lastFailureCode: z.enum(["timeout", "closed", "rejected", "busy", "disposed", "send_failed"]).nullable(),
  })).max(BRIDGE_OPERATIONS.length),
});
/** @aiPurpose Validate the entire safe snapshot without personal or business identifiers. @aiUsage directory and bridge may be null when their metadata is unavailable. */
export const diagnosticsSnapshotSchema = z.object({
  capturedAt: z.iso.datetime(),
  target: z.object({ sdk: z.literal("0.15.3"), clientChunk: z.object({ name: z.literal("ui"), hash: z.string().regex(/^[0-9a-f]{16}$/), api: count.positive(), bytes: count }) }),
  directory: directoryHealthSchema.nullable(), bridge: bridgeDiagnosticsSchema.nullable(),
});
/** @aiPurpose Read cached diagnostics without probes or refresh side effects. @aiUsage Input is empty and strict; querying never loads an Agent or changes a preference. */
export const getDiagnostics = defineRpc({ name: "observatory.diagnostics.snapshot", input: z.object({}).strict(), output: diagnosticsSnapshotSchema });

/**
 * @aiPurpose Produce a shareable, language-neutral summary using the exact RPC whitelist.
 * @aiUsage Copy only this output; parsing removes extra fields and rejects invalid known fields rather than preserving raw error text.
 */
export function formatDiagnosticsSummary(snapshot: DiagnosticsSnapshot): string {
  return JSON.stringify(diagnosticsSnapshotSchema.parse(snapshot), null, 2);
}

type Assert<T extends true> = T;
type _BridgeToDomain = Assert<z.infer<typeof bridgeDiagnosticsSchema> extends BridgeDiagnostics ? true : false>;
type _DomainToBridge = Assert<BridgeDiagnostics extends z.infer<typeof bridgeDiagnosticsSchema> ? true : false>;
type _SnapshotToDomain = Assert<z.infer<typeof diagnosticsSnapshotSchema> extends DiagnosticsSnapshot ? true : false>;
type _DomainToSnapshot = Assert<DiagnosticsSnapshot extends z.infer<typeof diagnosticsSnapshotSchema> ? true : false>;
