/** @aiPurpose Validate the directory health shared by observation, statistics and token attribution queries. */
import { z } from "zod";
import type { DirectoryHealth } from "./domain/source-health";

export type { DirectoryHealth } from "./domain/source-health";
/** @aiPurpose Validate explicit source health without inventing healthy defaults. @aiUsage Current read contracts supply the whole object; unavailable observations remain explicit. */
export const directoryHealthSchema = z.object({
  source: z.literal("agentx-directory"),
  status: z.enum(["healthy", "stale", "unavailable", "unknown"]),
  observedAt: z.iso.datetime().nullable(), lastAttemptAt: z.iso.datetime().nullable(), lastSuccessAt: z.iso.datetime().nullable(),
  complete: z.boolean(), revision: z.number().int().nonnegative(),
  errorCode: z.enum(["read_failed", "timeout", "invalidated", "incomplete"]).nullable(),
});

type Assert<T extends true> = T;
type _SchemaToDomain = Assert<z.infer<typeof directoryHealthSchema> extends DirectoryHealth ? true : false>;
type _DomainToSchema = Assert<DirectoryHealth extends z.infer<typeof directoryHealthSchema> ? true : false>;
