import { defineRpc } from "../../host/contracts";
import { z } from "zod";
import { ProviderTimelinePayloadSchema } from "./agentx-timeline";

export const ProviderSubagentSchema = z.object({
  id: z.string(), parentAgentId: z.string(), parentSubagentId: z.string().nullable().optional(),
  provider: z.string(), title: z.string().nullable(), description: z.string().nullable(),
  subtitle: z.string().nullable().optional(), cwd: z.string().nullable().optional(),
  status: z.enum(["running", "completed", "failed", "canceled"]),
  createdAt: z.string(), updatedAt: z.string(), toolCallId: z.string().nullable(),
});
export type ProviderSubagent = z.infer<typeof ProviderSubagentSchema>;

export { ProviderTimelinePayloadSchema, type ProviderTimelinePayload } from "./agentx-timeline";

export const providerSubagentsRpc = defineRpc({
  name: "observatory.provider-subagents.list",
  input: z.object({ parentAgentId: z.string().min(1).max(512) }),
  output: z.object({ subagents: z.array(ProviderSubagentSchema), error: z.string().nullable(), truncated: z.boolean() }),
});
export const providerTimelineRpc = defineRpc({
  name: "observatory.provider-subagents.timeline",
  input: z.object({ parentAgentId: z.string().min(1).max(512), subagentId: z.string().min(1).max(512),
    direction: z.enum(["tail", "before", "after"]).default("tail"),
    cursor: z.object({ epoch: z.string(), seq: z.number().int().nonnegative() }).optional(),
    limit: z.number().int().min(1).max(150).default(100) }),
  output: z.object({ page: ProviderTimelinePayloadSchema.nullable(), error: z.string().nullable() }),
});
