import type { Category, Incident, Attempt, Target, RecoveryDelivery, RecoveryGuardReason } from "./domain/recovery";
export type { Category, Incident, Attempt, Target, RecoveryDelivery, RecoveryGuardReason } from "./domain/recovery";
export { CONTINUE_PROMPT, classifyError } from "./domain/recovery";
import { onTextChange } from "./i18n";
import { t } from "../shared/text";
/** @aiPurpose Define incident/attempt contracts and safe, fixed follow-up text for manual and automated recovery. */
import { defineRpc } from "../../host/contracts";
import { z } from "zod";
import { automationSchema } from "./automation";

export const categorySchema = z.enum(["network", "timeout", "rate_limit", "quota", "authentication", "provider", "unknown"]) satisfies z.ZodType<Category>;
export const categoryLabels: Record<Category, string> = {
  network: t("personalRecovery.m172"), timeout: t("personalRecovery.m173"), rate_limit: t("personalRecovery.m174"), quota: t("personalRecovery.m175"),
  authentication: t("personalRecovery.m176"), provider: t("personalRecovery.m177"), unknown: t("personalRecovery.m178"),
};

export const phaseSchema = z.enum(["submitting", "accepted", "not_sent", "unknown", "observed_running", "observed_completed", "failed_again", "canceled", "skipped"]);
export const phaseLabels: Record<z.infer<typeof phaseSchema>, string> = {
  submitting: t("personalRecovery.m179"), accepted: t("personalRecovery.m180"), not_sent: t("recoveryDelivery.notSent"), unknown: t("personalRecovery.m181"),
  observed_running: t("personalRecovery.m182"), observed_completed: t("personalRecovery.m183"),
  failed_again: t("personalRecovery.m147"), canceled: t("personalRecovery.m184"), skipped: t("personalRecovery.m185"),
};
export const incidentSchema = z.object({
  id: z.string(), revision: z.number().int().positive(), agentId: z.string(), title: z.string(), provider: z.string(),
  workspaceId: z.string().nullable(), projectId: z.string().nullable(), workspaceName: z.string(), projectName: z.string(),
  turnId: z.string().nullable(), generation: z.number().int(), source: z.enum(["hook", "snapshot"]),
  message: z.string(), code: z.string().optional(), category: categorySchema, observedAt: z.string(),
  lastUserMessageAt: z.string().nullable(), errorFingerprint: z.string().nullable(),
  baselineCaptured: z.boolean(),
  state: z.enum(["detected", "superseded", "submitted"]), blockedReason: z.string().nullable(),
}) satisfies z.ZodType<Incident>;
export const recoveryDeliverySchema = z.discriminatedUnion("status", [
  z.object({ status: z.literal("acknowledged"), messageId: z.string(), reasonCode: z.literal("ledger_sent") }),
  z.object({ status: z.literal("not_sent"), messageId: z.string(), reasonCode: z.enum(["ledger_busy", "target_gone", "guard_rejected", "consent_required"]) }),
  z.object({ status: z.literal("unknown"), messageId: z.string(), reasonCode: z.enum(["ledger_unsettled", "ledger_conflict", "send_error", "send_timeout", "reloaded"]), relatedMessageId: z.string().optional() }),
]) satisfies z.ZodType<RecoveryDelivery>;
export const recoveryGuardReasonSchema = z.enum(["recovery_inactive", "incident_changed", "generation_changed", "agent_gone", "agent_busy", "permission_pending",
  "provider_unavailable", "baseline_changed", "stop_evidence_missing", "automation_disabled", "policy_changed", "outside_scope", "cycle_changed"]) satisfies z.ZodType<RecoveryGuardReason>;
export const attemptSchema = z.object({
  id: z.string(), operationId: z.string(), incidentId: z.string(), revision: z.number(), agentId: z.string(),
  phase: phaseSchema, reason: z.string(), createdAt: z.string(), updatedAt: z.string(),
  baselineGeneration: z.number(), observedGeneration: z.number().nullable(), observedTurnId: z.string().nullable(),
  trigger: z.enum(["manual", "automatic"]),
  policyRevision: z.number().int().nonnegative().optional(),
  delivery: recoveryDeliverySchema.nullable(), guardReason: recoveryGuardReasonSchema.optional(), previousAttemptId: z.string().optional(),
}) satisfies z.ZodType<Attempt>;
export const targetSchema = z.object({ incidentId: z.string().min(1), revision: z.number().int().positive() }) satisfies z.ZodType<Target>;
export const listRecovery = defineRpc({
  name: "recovery.list", input: z.object({}),
  output: z.object({ incidents: z.array(incidentSchema), attempts: z.array(attemptSchema), scannedAt: z.string(), scannedAgents: z.number(), automation: automationSchema }),
});
export const continueRecovery = defineRpc({
  name: "recovery.continue",
  input: z.object({ operationId: z.string().min(8).max(128), targets: z.array(targetSchema).min(1).max(500) }),
  output: z.object({ attempts: z.array(attemptSchema) }),
});
export type RecoveryList = z.infer<typeof listRecovery.output>;

// Display labels follow the observatory language; persisted identities and command text remain stable.
onTextChange(() => {
  Object.assign(categoryLabels, {
  network: t("personalRecovery.m172"), timeout: t("personalRecovery.m173"), rate_limit: t("personalRecovery.m174"), quota: t("personalRecovery.m175"),
  authentication: t("personalRecovery.m176"), provider: t("personalRecovery.m177"), unknown: t("personalRecovery.m178"),
});
  Object.assign(phaseLabels, {
  submitting: t("personalRecovery.m179"), accepted: t("personalRecovery.m180"), not_sent: t("recoveryDelivery.notSent"), unknown: t("personalRecovery.m181"),
  observed_running: t("personalRecovery.m182"), observed_completed: t("personalRecovery.m183"),
  failed_again: t("personalRecovery.m147"), canceled: t("personalRecovery.m184"), skipped: t("personalRecovery.m185"),
});
});
