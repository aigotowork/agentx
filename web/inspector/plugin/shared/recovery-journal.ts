/** @aiPurpose Own the single current recovery journal representation, separate from offline historical conversion. */
import { z } from "zod";
import { attemptSchema, incidentSchema } from "./recovery";
import { automationSchema } from "./automation";

/** @aiPurpose Validate current recovery evidence without defaults or historical format branches. @aiUsage Storage and offline migration validate their final result through this schema. */
export const recoveryJournalSchema = z.object({
  version: z.literal(3), incidents: z.array(incidentSchema), attempts: z.array(attemptSchema),
  generations: z.record(z.string(), z.number()), automation: automationSchema,
});
export type RecoveryJournal = z.output<typeof recoveryJournalSchema>;
