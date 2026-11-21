import { defineRpc } from '../../../host/contracts';
import { z } from 'zod';
import type { BotsSnapshot } from '../domain/bots-observation';
const id = z.string().min(1).max(200), text = z.string().max(160);
const status = z.enum(['available', 'partial', 'unavailable', 'forbidden', 'unsupported', 'unconfigured']);
const issue = z.enum(['read_failed', 'timeout', 'denied', 'unsupported', 'identity_mismatch', 'invalid_config']).nullable();
const observed = <T extends z.ZodType>(item: T) => z.object({ status, issue, items: z.array(item).max(500), observedAt: z.string().nullable(), truncated: z.boolean(), gap: z.boolean() });
const conversation = z.object({ id, botId: id, teamId: id.nullable(), title: text, status: z.enum(['pending', 'running', 'idle', 'succeeded', 'failed', 'cancelled', 'unknown']), updatedAt: z.string(), attention: z.boolean().nullable() });
export const botsSnapshotSchema = z.object({
  observedAt: z.string(), authorityId: id.nullable(), connection: status, issue,
  bots: observed(z.object({ id, name: text, title: text, provider: text, model: text.nullable(), archived: z.boolean() })),
  teams: observed(z.object({ id, name: text, leadId: id.nullable(), memberIds: z.array(id).max(1000) })),
  conversations: observed(conversation),
  runtimes: observed(z.object({ id, serverId: id, generation: id, title: text, mode: z.enum(['managed', 'attached']), health: z.enum(['ready', 'disconnected', 'unavailable', 'identity-mismatch']), partial: z.boolean(), observedAt: z.string() })),
  agents: observed(z.object({ id, runtimeId: id, serverId: id, generation: id, title: text, provider: text, model: text.nullable(), ownership: z.enum(['ordinary', 'foreign']), state: z.enum(['running', 'idle', 'ready', 'closed', 'failed', 'unknown']), attention: z.boolean(), updatedAt: z.string() })),
});
/** Bounded SDK catalog queries. No history leases, commands or background monitoring. */
export const getBotsSnapshot = defineRpc({ name: 'observatory.bots.snapshot', input: z.object({}).strict(), output: botsSnapshotSchema });
type Assert<T extends true> = T;
type _Snapshot = Assert<z.infer<typeof botsSnapshotSchema> extends BotsSnapshot ? BotsSnapshot extends z.infer<typeof botsSnapshotSchema> ? true : false : false>;
