import type { Policy, RecoveryCycle, AuditEvent, Automation } from "./domain/recovery";
export type { Policy, RecoveryCycle, AuditEvent, Automation } from "./domain/recovery";
export { BACKOFF_MINUTES, defaultPolicy, emptyAutomation } from "./domain/recovery";
import { onTextChange } from "./i18n";
import { t } from "../shared/text";
/** @aiPurpose Define durable backoff policies, recovery chains and management RPCs shared by both runtimes. */
import { defineRpc } from "../../host/contracts";
import { z } from "zod";

export const policySchema = z.object({
  enabled: z.boolean(), scanIntervalSeconds: z.number().int().min(30).max(300),
  projectIds: z.array(z.string().min(1)).max(500), workspaceIds: z.array(z.string().min(1)).max(500),
  revision: z.number().int().nonnegative(),
}) satisfies z.ZodType<Policy>;
export const cycleSchema = z.object({
  id: z.string(), agentId: z.string(), incidentId: z.string(),
  title: z.string(), projectId: z.string().nullable(), workspaceId: z.string().nullable(),
  projectName: z.string(), workspaceName: z.string(),
  status: z.enum(["waiting", "checking", "recovered", "blocked", "paused", "stopped"]),
  pauseSource: z.enum(["user", "scope"]).nullable(), retryIndex: z.number().int().nonnegative().max(2),
  attemptCount: z.number().int().nonnegative(), currentAttemptId: z.string().nullable(),
  nextAt: z.string().nullable(), reason: z.string(), createdAt: z.string(), updatedAt: z.string(),
}) satisfies z.ZodType<RecoveryCycle>;
export const cycleLabels: Record<RecoveryCycle["status"], string> = {
  waiting: t("personalRecovery.m167"), checking: t("personalRecovery.m168"), recovered: t("personalRecovery.m169"), blocked: t("personalRecovery.m025"), paused: t("personalRecovery.m170"), stopped: t("personalRecovery.m171"),
};
export const auditSchema = z.object({
  id: z.string(), at: z.string(), agentId: z.string().nullable(), cycleId: z.string().nullable(),
  kind: z.enum(["scheduled", "submitted", "retry", "recovered", "blocked", "paused", "resumed", "stopped", "policy", "scan_error", "scan_restored"]),
  message: z.string(),
}) satisfies z.ZodType<AuditEvent>;
export const monitorSchema = z.object({
  scannedAt: z.string().nullable(), scannedAgents: z.number(), error: z.string().nullable(),
});
const scopeOptionSchema = z.object({ id: z.string(), name: z.string() });
export const automationSchema = z.object({ policy: policySchema, cycles: z.array(cycleSchema), events: z.array(auditSchema), monitor: monitorSchema,
  catalog: z.object({ projects: z.array(scopeOptionSchema), workspaces: z.array(scopeOptionSchema) }),
}) satisfies z.ZodType<Automation>;
export const configureAutomation = defineRpc({
  name: "recovery.configure", input: policySchema,
  output: z.object({ policy: policySchema }),
});
export const manageAutomation = defineRpc({
  name: "recovery.manage", input: z.object({ cycleId: z.string(), action: z.enum(["pause", "resume", "stop"]) }),
  output: z.object({ cycle: cycleSchema }),
});

// Display labels follow the observatory language; persisted identities and command text remain stable.
onTextChange(() => {
  Object.assign(cycleLabels, {
  waiting: t("personalRecovery.m167"), checking: t("personalRecovery.m168"), recovered: t("personalRecovery.m169"), blocked: t("personalRecovery.m025"), paused: t("personalRecovery.m170"), stopped: t("personalRecovery.m171"),
});
});
