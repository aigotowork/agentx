/** @aiPurpose Transport for the manager's passive retained window; independent from the runner-discovering legacy list. */
import { defineRpc } from "../../host/contracts";
import { z } from "zod";
import { insightJobSummarySchema, type InsightJobSummary } from "./insights";
import { INSIGHT_JOBS_LIMIT, type InsightJobsSnapshot as DomainSnapshot, type InsightJobsSource } from "./domain/insight-jobs";

export { INSIGHT_JOBS_LIMIT } from "./domain/insight-jobs";
export type { InsightJobsSource } from "./domain/insight-jobs";
export type InsightJobsSnapshot = DomainSnapshot<InsightJobSummary>;
export const insightJobsSnapshotSchema = z.object({
  revision: z.string().min(1).max(80), generatedAt: z.iso.datetime(),
  jobs: z.array(insightJobSummarySchema).max(INSIGHT_JOBS_LIMIT),
  source: z.object({ status: z.enum(["available", "partial"]), complete: z.boolean(), truncated: z.boolean(),
    total: z.number().int().nonnegative(), errorCode: z.literal("storage_error").nullable() }),
  retention: z.object({ scope: z.literal("retained"), reportLimit: z.number().int().nonnegative(), statusLimit: z.number().int().nonnegative(), reportBytesLimit: z.number().int().nonnegative() }),
});
/** No attach, SDK, runner discovery, scheduling, settings writes or ledger creation on this read. */
export const getInsightJobsSnapshot = defineRpc({ name: "observatory.insights.jobs.snapshot", input: z.object({}).strict(), output: insightJobsSnapshotSchema });

type Assert<T extends true> = T;
type _ToDomain = Assert<z.infer<typeof insightJobsSnapshotSchema> extends InsightJobsSnapshot ? true : false>;
type _FromDomain = Assert<InsightJobsSnapshot extends z.infer<typeof insightJobsSnapshotSchema> ? true : false>;
type _Source = Assert<z.infer<typeof insightJobsSnapshotSchema>["source"] extends InsightJobsSource ? true : false>;
