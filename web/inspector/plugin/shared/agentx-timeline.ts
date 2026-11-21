/** @aiPurpose Adapt the two current AgentX timeline APIs into one source-identified timeline model. */
import { z } from "zod";
import { normalizeTimelinePage, type TimelinePage } from "./timeline";
import { t } from "./text";

const sequence = z.number().int().nonnegative();
const range = z.object({ startSeq: sequence, endSeq: sequence });
const cursor = z.object({ epoch: z.string().min(1), seq: sequence });
const item = z.object({ type: z.string().min(1) }).catchall(z.unknown());
const page = z.object({
  epoch: z.string().min(1), direction: z.enum(["tail", "before", "after"]),
  reset: z.boolean(), staleCursor: z.boolean(), gap: z.boolean(),
  window: z.object({ minSeq: sequence, maxSeq: sequence, nextSeq: sequence }),
  hasOlder: z.boolean(), hasNewer: z.boolean(), error: z.string().nullable(),
  truncated: z.boolean().optional(),
});

/** `agents.ref(id).timeline.refetch`: entries and their projected source ranges are current required fields. */
export const AgentTimelinePayloadSchema = page.extend({
  projection: z.enum(["projected", "canonical"]),
  startCursor: cursor.nullable(), endCursor: cursor.nullable(),
  entries: z.array(z.object({
    provider: z.string(), timestamp: z.string(), item, turnId: z.string().optional(),
    seqStart: sequence, seqEnd: sequence, sourceSeqRanges: z.array(range),
    collapsed: z.array(z.enum(["assistant_merge", "reasoning_merge", "tool_lifecycle", "identity"])),
  })),
});

/** Provider child timeline: native seq is required; projected ranges and cursors are genuinely optional in 0.15.3. */
export const ProviderTimelinePayloadSchema = page.extend({
  requestId: z.string(), parentAgentId: z.string(), subagentId: z.string(), provider: z.string().nullable(),
  projection: z.literal("projected").optional(),
  startCursor: cursor.nullable().optional(), endCursor: cursor.nullable().optional(),
  rows: z.array(z.object({
    seq: sequence, timestamp: z.string(), item,
    seqStart: sequence.optional(), seqEnd: sequence.optional(), sourceSeqRanges: z.array(range).optional(),
    turnId: z.string().optional(), providerMessageId: z.string().optional(),
  })),
});
export type ProviderTimelinePayload = z.infer<typeof ProviderTimelinePayloadSchema>;

function parse<Output>(schema: z.ZodType<Output>, value: unknown): Output {
  const result = schema.safeParse(value);
  if (!result.success) throw Object.assign(new Error(t("secondaryTimelineStore.couldNotReadExecutionRecords")), { code: "invalid_timeline_page" });
  if (result.data && typeof result.data === "object" && "error" in result.data && result.data.error) throw new Error(String(result.data.error));
  return result.data;
}

export function normalizeAgentTimelinePage(value: unknown): TimelinePage {
  const source = parse(AgentTimelinePayloadSchema, value);
  return normalizeTimelinePage({ ...source, rows: source.entries });
}

export function normalizeProviderTimelinePage(value: unknown): TimelinePage {
  const source = parse(ProviderTimelinePayloadSchema, value);
  return normalizeTimelinePage({ ...source, rows: source.rows.map(row => ({ ...row,
    seqStart: row.seqStart ?? row.seq, seqEnd: row.seqEnd ?? row.seq,
    sourceSeqRanges: row.sourceSeqRanges ?? [{ startSeq: row.seqStart ?? row.seq, endSeq: row.seqEnd ?? row.seq }],
  })) });
}
