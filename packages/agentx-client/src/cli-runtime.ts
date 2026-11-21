/** @aiPurpose Own Node CLI parsing, configuration, structured inputs and durable request retention. */
import { parseArgs } from 'node:util';
import { readFile, mkdir, open } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { homedir } from 'node:os';
import { randomUUID } from 'node:crypto';
import { stringifyJson } from './codec.js';
import type { WaitOptions } from './client.js';

const string = { type: 'string' } as const;
const repeat = { type: 'string', multiple: true } as const;
const flag = { type: 'boolean' } as const;
/** @aiPurpose Declare the complete HTTP CLI option grammar. @aiUsage Used only by the Node CLI; each handler additionally rejects options outside its command. */
export function parseCli() {
  return parseArgs({ allowPositionals: true, strict: true, options: {
    help: flag, version: flag, config: string, format: string,
    'management-url': string, 'workbench-url': string, 'observation-url': string,
    'admin-url': string, 'new-token-file': string, 'expected-version': string, host: string, 'installation-id': string, task: string,
    'timeout-seconds': string, generation: string, incarnation: string, 'stopped-event': string,
    after: string, before: string, limit: string, 'current-only': flag, label: repeat,
    text: string, file: string, input: string, intent: string, 'reply-to': string, thread: string, attachment: repeat,
    'request-id': string, 'command-id': string, pending: string, wait: flag, sent: flag,
    'poll-seconds': string, 'wait-timeout-seconds': string,
    workspace: string, directory: string, tool: string, profile: string, role: repeat, title: string, prompt: string,
    set: repeat, remove: repeat, 'if-revision': string, name: string, 'media-type': string, output: string,
    'pending-only': flag, enabled: flag, disabled: flag, 'recovery-enabled': flag, 'recovery-disabled': flag,
    'error-duration-seconds': string, observation: repeat, episode: string,
    brief: string, 'implementer-ref': string, 'checker-ref': string, 'participant-ref': string, 'work-ref': string,
    question: string, amendment: string, result: string, check: string, 'brief-revision': string, reason: string,
    selection: string, 'expected-daemon': string, 'expected-epoch': string, field: repeat, window: string,
  } });
}
/** @aiPurpose Represent invalid local input before dispatch. @aiUsage Convert to structured stderr and exit2. */
export class UsageError extends Error {}
/** @aiPurpose Supply one invocation's parsed inputs and owned cancellation lifetime. @aiUsage Domain handlers own exact command/body construction, never mutate shared runtime configuration. */
export interface CliContext {
  values: ReturnType<typeof parseCli>['values'];
  positionals: string[];
  config: Record<string, unknown>;
  signal: AbortSignal;
  fetch: typeof fetch;
  waitOptions: WaitOptions;
  url(domain: 'management' | 'workbench' | 'observation'): string;
}
/** @aiPurpose Address JSON output by its actual listener schema. @aiUsage Marker listeners print marker JSON so saved output operands can round-trip through structured files. */
export type CliResult = { value: unknown; schema: string; domain: 'management' | 'workbench' | 'observation' | 'cloud-admin' } | undefined;
/** @aiPurpose Require a named local operand. @aiUsage Empty identity/path operands are invalid while explicit message text may be empty. */
export function required(value: string | undefined, label: string): string {
  if (!value) throw new UsageError(`${label} is required`); return value;
}
/** @aiPurpose Convert bounded CLI timing input to milliseconds. @aiUsage Omission selects the declared fallback in seconds. */
export function seconds(value: unknown, fallback: number): number {
  const number = value === undefined ? fallback : Number(value);
  if (!Number.isFinite(number) || number <= 0) throw new UsageError('Seconds must be positive and finite');
  return number * 1000;
}
/** @aiPurpose Parse canonical integer flags without Number loss. @aiUsage Native marker operands and management long integers use bigint; width checks remain in emitted schemas. */
export function integer(value: string | undefined, label: string): bigint {
  const text = required(value, label);
  if (!/^(0|-[1-9][0-9]*|[1-9][0-9]*)$/.test(text)) throw new UsageError(`${label} must be a canonical decimal integer`);
  return BigInt(text);
}
/** @aiPurpose Parse a safe bounded numeric CLI flag. @aiUsage Use only for management small integers; marker-surface flags use integer(). */
export function pageLimit(value: string | undefined, fallback?: number, max = 100): number | undefined {
  if (value === undefined) return fallback;
  const number = Number(value);
  if (!Number.isInteger(number) || number < 1 || number > max) throw new UsageError(`--limit must be 1..${max}`);
  return number;
}
/** @aiPurpose Read at most one stdin operand per invocation. @aiUsage FILE reads UTF-8; '-' is reserved for this invocation's single structured or text input. */
export async function inputText(path: string): Promise<string> {
  if (path !== '-') return readFile(path, 'utf8');
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  return Buffer.concat(chunks).toString('utf8');
}
/** @aiPurpose Enforce command-specific flags and single stdin ownership. @aiUsage Call before any service request or local mutation. */
export function restrict(ctx: CliContext, allowed: string[], length?: number): void {
  const global = ['help', 'version', 'config', 'format', 'management-url', 'workbench-url', 'observation-url', 'timeout-seconds'];
  for (const key of Object.keys(ctx.values)) if (!global.includes(key) && !allowed.includes(key)) throw new UsageError(`--${key} is not valid for this command`);
  if (length !== undefined && ctx.positionals.length !== length) throw new UsageError('Unexpected or missing command arguments');
  let stdin = 0;
  // Only file-valued options consume stdin. Literal message text '-' does not.
  for (const key of ['file', 'input', 'brief', 'implementer-ref', 'checker-ref', 'participant-ref', 'work-ref', 'amendment', 'episode', 'selection', 'expected-daemon', 'expected-epoch', 'window'] as const) if (ctx.values[key] === '-') ++stdin;
  if (ctx.positionals[0] === 'attachments' && ctx.positionals[1] === 'upload' && ctx.positionals[2] === '-') ++stdin;
  if (ctx.positionals[0] === 'observe') {
    const [, group, action, ...operands] = ctx.positionals;
    if (group === 'config' && action === 'compare') stdin += operands.filter(value => value === '-').length;
    else if ((group === 'config' || group === 'reports' || group === 'requests') && action === 'show' && operands[0] === '-') ++stdin;
  }
  if (stdin > 1) throw new UsageError('Only one input may consume stdin');
}
/** @aiPurpose Preserve a complete mutation input before HTTP dispatch. @aiUsage Supply already schema-validated wire JSON and a stable request/command identity; existing different content is never overwritten. */
export async function retain(ctx: CliContext, id: string, body: string, operation: string, identity: Record<string, string> = {}): Promise<string> {
  const path = ctx.values.pending ?? join('.agentx-http', 'requests', `${encodeURIComponent(id)}.json`);
  await mkdir(dirname(path), { recursive: true });
  let file;
  try { file = await open(path, 'wx', 0o600); }
  catch (error) {
    if (!(error && typeof error === 'object' && 'code' in error && error.code === 'EEXIST')) throw error;
    if ((await readFile(path, 'utf8')).trim() !== body) throw new UsageError(`Pending file already contains different input: ${path}`);
  }
  if (file) { try { await file.writeFile(`${body}\n`); await file.sync(); } finally { await file.close(); } }
  console.error(stringifyJson({ pending: path, operation, id, ...identity }));
  return path;
}
/** @aiPurpose Parse distinct KEY=VALUE fields without erasing duplicate intent or empty values. @aiUsage Used for launch labels and explicit label patches. */
export function keyValues(items: string[] = []): Record<string, string> {
  const result: Record<string, string> = {};
  for (const item of items) {
    const index = item.indexOf('='); if (index < 1) throw new UsageError('Expected KEY=VALUE');
    const key = item.slice(0, index); if (Object.hasOwn(result, key)) throw new UsageError(`Duplicate key ${key}`);
    Object.defineProperty(result, key, { value: item.slice(index + 1), enumerable: true, configurable: true, writable: true });
  }
  return result;
}
/** @aiPurpose Allocate one explicit local intent identity. @aiUsage Preserve the selected value for pending input and original receipt lookup. */
export function newId(): string { return randomUUID(); }
/** @aiPurpose Load declared CLI config once without contacting any listener. @aiUsage Explicit missing config fails; missing default config is allowed. */
export async function configuration(values: ReturnType<typeof parseCli>['values']): Promise<Record<string, unknown>> {
  const explicit = values.config ?? process.env.AGENTX_API_CONFIG;
  const path = explicit ?? join(process.env.XDG_CONFIG_HOME ?? join(homedir(), '.config'), 'agentx-http', 'config.json');
  try {
    const content: unknown = JSON.parse(await readFile(path, 'utf8'));
    if (!content || typeof content !== 'object' || Array.isArray(content)) throw new UsageError('Config must be a JSON object');
    return Object.fromEntries(Object.entries(content));
  } catch (error) {
    if (!explicit && error && typeof error === 'object' && 'code' in error && error.code === 'ENOENT') return {};
    throw error;
  }
}
