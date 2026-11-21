/** @aiPurpose Bind the complete management command tree to public SDK methods with durable exact inputs. */
import { readFile, open } from 'node:fs/promises';
import { basename } from 'node:path';
import { createManagementClient } from './management.js';
import { parseManagementJson, stringifyManagementJson } from './codec.js';
import type { SchemaTypes as M } from './generated/management/schema-types.gen.js';
import { UsageError, required, integer, inputText, restrict, retain, keyValues, newId, pageLimit } from './cli-runtime.js';
import type { CliContext, CliResult } from './cli-runtime.js';

const exactFlags = ['generation', 'incarnation', 'stopped-event'];
const pageFlags = ['after', 'limit'];
const messageFlags = ['text', 'file', 'intent', 'reply-to', 'thread', 'attachment'];
const writeFlags = ['input', 'generation', 'request-id', 'pending', 'wait', 'poll-seconds', 'wait-timeout-seconds'];
/** @aiPurpose Execute a selected management command without native CLI delegation. @aiUsage Receives one invocation; binary download writes only the explicitly selected exclusive output. */
export async function runManagement(ctx: CliContext): Promise<CliResult> {
  const v = ctx.values, p = ctx.positionals;
  const [group, action, target, name] = p;
  const api = createManagementClient({ baseUrl: ctx.url('management'), fetch: ctx.fetch });
  const opts = { signal: ctx.signal };
  const generation = async () => v.generation ?? (await api.describe(opts)).generation;
  const output = (value: unknown, schema = 'json'): CliResult => ({ value, schema, domain: 'management' });
  function exact(agentName: string): M['AgentRef'] {
    if (Boolean(v.incarnation) === Boolean(v['stopped-event'])) throw new UsageError('Supply exactly one --incarnation or --stopped-event');
    return { name: agentName, incarnation: v.incarnation, stopped_event: v['stopped-event'] };
  }
  async function message(): Promise<M['MessageInput']> {
    if ((v.text === undefined) === (v.file === undefined)) throw new UsageError('Supply exactly one --text or --file');
    if (v.intent !== undefined && !['request', 'inform', 'ack'].includes(v.intent)) throw new UsageError('--intent must be request, inform or ack');
    return { text: v.text ?? await inputText(required(v.file, '--file')), attachments: v.attachment ?? [], intent: v.intent, reply_to: v['reply-to'], thread: v.thread };
  }
  async function saved<K extends keyof M>(schema: K, input: M[K], operation: string): Promise<M[K]> {
    const body = stringifyManagementJson(schema, input);
    const value = parseManagementJson(schema, body);
    const metadata: Record<string, string> = {};
    if (value && typeof value === 'object') {
      for (const key of ['generation', 'request_id'] as const) { const item = Reflect.get(value, key); if (typeof item === 'string') metadata[key] = item; }
    }
    await retain(ctx, metadata.request_id ?? newId(), body, operation, metadata);
    return value;
  }
  async function supplied<K extends keyof M>(schema: K, make: () => Promise<M[K]>): Promise<M[K]> {
    if (!v.input) return make();
    const permitted = ['input', 'pending', 'wait', 'poll-seconds', 'wait-timeout-seconds', 'management-url', 'workbench-url', 'observation-url', 'timeout-seconds', 'config', 'format', 'help', 'version'];
    if (Object.keys(v).some(key => !permitted.includes(key))) throw new UsageError('--input cannot combine with body/identity flags');
    if (p.length > 3 && !(group === 'agents' && action === 'messages' && target === 'send' && p.length === 3)) throw new UsageError('--input replaces target/body flags');
    return parseManagementJson(schema, await inputText(v.input));
  }
  async function receipt(operation: M['Operation'], sent = false): Promise<CliResult> {
    if (!v.wait) return output(operation);
    return sent ? output({ generation: operation.generation, request_id: operation.request_id, sent: await api.waitForSent(operation, ctx.waitOptions) }) : output(await api.waitForOperation(operation, ctx.waitOptions));
  }
  if (group === 'management' && action === 'show') { restrict(ctx, [], 2); return output(await api.describe(opts)); }
  if (group === 'workspaces' && action === 'list') { restrict(ctx, ['generation'], 2); return output(await api.workspaces.list(await generation(), opts)); }
  if (group === 'roles' && action === 'list') { restrict(ctx, ['generation', 'workspace'], 2); return output(await api.roles.list({ generation: await generation(), workspace: required(v.workspace, '--workspace') }, opts)); }
  if (group === 'directory' && action === 'inspect') { restrict(ctx, ['generation'], 3); return output(await api.directory.inspect({ generation: await generation(), directory: required(target, 'PATH') }, opts)); }
  if (group === 'directory' && action === 'send') {
    restrict(ctx, [...writeFlags, ...messageFlags], v.input ? 2 : 3);
    const input = await supplied('SubmitDirectoryMessage', async () => ({ generation: await generation(), request_id: v['request-id'] ?? newId(), directory: required(target, 'PATH'), message: await message() }));
    return receipt(await api.directory.send(await saved('SubmitDirectoryMessage', input, 'directory.send'), opts));
  }
  if (group === 'agents' && action === 'list') {
    restrict(ctx, ['generation', ...pageFlags, 'current-only', 'label'], 2);
    return output(await api.listAgents({ generation: await generation(), after: v.after, limit: pageLimit(v.limit), current_only: v['current-only'], label: v.label }, opts));
  }
  if (group === 'agents' && action === 'show') { restrict(ctx, exactFlags, 3); return output(await api.getAgent({ generation: await generation(), agent: exact(required(target, 'NAME')) }, opts)); }
  if (group === 'agents' && action === 'launch') {
    restrict(ctx, [...writeFlags, 'label', 'tool', 'profile', 'role', 'workspace', 'directory', 'title', 'prompt'], 2);
    if ((v.role?.length ?? 0) > 1) throw new UsageError('Launch takes one --role');
    const input = await supplied('SubmitLaunch', async () => ({ generation: await generation(), request_id: v['request-id'] ?? newId(), launch: {
      labels: keyValues(v.label), tool: v.tool, profile: v.profile, role: v.role?.[0], workspace: v.workspace, directory: v.directory, title: v.title, prompt: v.prompt,
    } }));
    return receipt(await api.agents.launch(await saved('SubmitLaunch', input, 'agents.launch'), opts));
  }
  if (group === 'agents' && action && ['kill', 'stop-participation', 'resume', 'fork', 'transcript'].includes(action)) {
    restrict(ctx, [...exactFlags, ...writeFlags], v.input ? 2 : 3);
    const input = await supplied('SubmitTarget', async () => ({ generation: await generation(), request_id: v['request-id'] ?? newId(), agent: exact(required(target, 'NAME')) }));
    const retained = await saved('SubmitTarget', input, `agents.${action}`);
    const operation = action === 'kill' ? await api.agents.kill(retained, opts) : action === 'stop-participation' ? await api.agents.stopParticipation(retained, opts) : action === 'resume' ? await api.agents.resume(retained, opts) : action === 'fork' ? await api.agents.fork(retained, opts) : await api.agents.transcript(retained, opts);
    return receipt(operation);
  }
  if (group === 'agents' && action === 'relations') {
    restrict(ctx, [...exactFlags, ...pageFlags], 3);
    return output(await api.agents.relations({ generation: await generation(), agent: exact(required(target, 'NAME')), after: v.after, limit: pageLimit(v.limit) }, opts));
  }
  if (group === 'agents' && action === 'messages' && target === 'list') {
    restrict(ctx, [...exactFlags, ...pageFlags], 4);
    return output(await api.agents.messages.list({ generation: await generation(), agent: exact(required(name, 'NAME')), after: v.after, limit: pageLimit(v.limit) }, opts));
  }
  if (group === 'agents' && action === 'messages' && target === 'send') {
    restrict(ctx, [...exactFlags, ...writeFlags, ...messageFlags], v.input ? 3 : 4);
    if (v.input && [...messageFlags, ...exactFlags, 'request-id'].some(key => key in v)) throw new UsageError('--input cannot combine with message/identity flags');
    const input = await supplied('SubmitMessage', async () => ({ generation: await generation(), request_id: v['request-id'] ?? newId(), agent: exact(required(name, 'NAME')), message: await message() }));
    return receipt(await api.send(await saved('SubmitMessage', input, 'agents.messages.send'), opts), true);
  }
  if (group === 'agents' && action === 'labels' && target === 'get') {
    restrict(ctx, exactFlags, 4); return output(await api.agents.labels.get({ generation: await generation(), agent: exact(required(name, 'NAME')) }, opts));
  }
  if (group === 'agents' && action === 'labels' && target === 'patch') {
    restrict(ctx, [...exactFlags, ...writeFlags, 'set', 'remove', 'if-revision'], v.input ? 3 : 4);
    const input = await supplied('SubmitLabels', async () => ({ generation: await generation(), request_id: v['request-id'] ?? newId(), agent: exact(required(name, 'NAME')), set: keyValues(v.set), remove: v.remove ?? [], if_revision: v['if-revision'] }));
    return receipt(await api.agents.labels.patch(await saved('SubmitLabels', input, 'agents.labels.patch'), opts));
  }
  if (group === 'operations' && (action === 'show' || action === 'wait')) {
    restrict(ctx, action === 'show' ? ['generation'] : ['generation', 'sent', 'poll-seconds', 'wait-timeout-seconds'], 3);
    const identity = { generation: required(v.generation, '--generation (original receipt)'), request_id: required(target, 'REQUEST_ID') };
    return output(action === 'show' ? await api.getOperation(identity, opts) : v.sent ? { ...identity, sent: await api.waitForSent(identity, ctx.waitOptions) } : await api.waitForOperation(identity, ctx.waitOptions));
  }
  if (group === 'attachments' && action === 'upload') {
    restrict(ctx, ['generation', 'name', 'media-type'], 3);
    const path = required(target, 'FILE');
    if (path === '-' && !v.name) throw new UsageError('stdin upload requires --name');
    const bytes: Buffer[] = [];
    if (path === '-') for await (const chunk of process.stdin) bytes.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
    else bytes.push(await readFile(path));
    const blob = new Blob([new Uint8Array(Buffer.concat(bytes))]);
    return output(await api.attachments.upload({ generation: await generation(), name: v.name ?? basename(path), media_type: v['media-type'], bytes: blob }, opts));
  }
  if (group === 'attachments' && action === 'download') {
    restrict(ctx, ['generation', 'output'], 3);
    const path = required(v.output, '--output');
    const file = path === '-' ? undefined : await open(path, 'wx', 0o600);
    try {
      const blob = await api.attachments.download({ generation: await generation(), id: required(target, 'ID') }, opts);
      const bytes = new Uint8Array(await blob.arrayBuffer());
      if (file) { await file.writeFile(bytes); await file.sync(); } else process.stdout.write(bytes);
    } finally { await file?.close(); }
    return undefined;
  }
  if (group === 'mailbox' && action === 'list') {
    restrict(ctx, ['generation', 'before', 'limit', 'pending-only'], 2);
    return output(await api.humanMailbox.list({ generation: await generation(), before: v.before, pending: v['pending-only'], limit: pageLimit(v.limit) }, opts));
  }
  if (group === 'mailbox' && action === 'handle') {
    restrict(ctx, ['generation', 'pending']); if (p.length < 3) throw new UsageError('Supply displayed message IDs');
    const input = await saved('HandleHumanMessages', { generation: await generation(), ids: p.slice(2) }, 'humanMailbox.handle');
    return output(await api.humanMailbox.handle(input, opts));
  }
  if (group === 'office' && action === 'activity') {
    restrict(ctx, ['generation', ...pageFlags], 2); return output(await api.officeActivity.list({ generation: await generation(), after: v.after, limit: pageLimit(v.limit) }, opts));
  }
  if (group === 'inspection' && action === 'show') { restrict(ctx, ['generation'], 2); return output(await api.inspection.get(await generation(), opts)); }
  if (group === 'inspection' && action === 'configure') {
    restrict(ctx, ['generation', 'pending', 'if-revision', 'enabled', 'disabled', 'recovery-enabled', 'recovery-disabled', 'error-duration-seconds'], 2);
    if (Boolean(v.enabled) === Boolean(v.disabled) || Boolean(v['recovery-enabled']) === Boolean(v['recovery-disabled'])) throw new UsageError('Choose enabled/disabled and recovery-enabled/recovery-disabled explicitly');
    const duration = Number(integer(v['error-duration-seconds'], '--error-duration-seconds'));
    if (!Number.isSafeInteger(duration) || duration < 0 || duration > 4294967295) throw new UsageError('Recovery duration outside u32');
    const input = await saved('ConfigureInput', { generation: await generation(), expected_revision: integer(v['if-revision'], '--if-revision'), enabled: Boolean(v.enabled), recovery: { enabled: Boolean(v['recovery-enabled']), error_duration_seconds: duration } }, 'inspection.configure');
    return output(await api.inspection.configure(input, opts));
  }
  if (group === 'inspection' && action === 'analyses' && target === 'create') {
    restrict(ctx, ['generation', 'pending', 'request-id', 'observation'], 3);
    if (!v.observation?.length) throw new UsageError('Supply --observation ID');
    const input = await saved('AnalyzeInput', { generation: await generation(), request_id: v['request-id'] ?? newId(), observation_ids: v.observation }, 'inspection.analyze');
    return output(await api.inspection.analyze(input, opts));
  }
  if (group === 'inspection' && action === 'analyses' && target === 'cancel') {
    restrict(ctx, ['generation', 'pending'], 4);
    const input = await saved('CancelAnalysisInput', { generation: await generation(), id: required(name, 'JOB_ID') }, 'inspection.cancelAnalysis');
    return output(await api.inspection.cancelAnalysis(input, opts));
  }
  if (group === 'inspection' && action === 'continue') {
    restrict(ctx, ['generation', 'pending', 'episode'], 2);
    const episode = parseManagementJson('Episode', await inputText(required(v.episode, '--episode FILE|-')));
    const input = await saved('RequestContinuationInput', { generation: await generation(), episode }, 'inspection.requestContinuation');
    return output(await api.inspection.requestContinuation(input, opts));
  }
  throw new UsageError('Unknown management command; use --help');
}
