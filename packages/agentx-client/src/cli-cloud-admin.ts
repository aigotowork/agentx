/** @aiPurpose Bind Node cloud administration to the public SDK and preserve exact mutation intent without printing secrets. */
import { constants } from 'node:fs';
import { mkdir, open } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, join } from 'node:path';
import { homedir } from 'node:os';
import { CloudAdminError, createCloudAdminClient, snapshotControlInput } from './cloud-admin.js';
import type { CloudControlInput, CloudNamespacePut, CloudNamespaceRotation } from './cloud-admin.js';
import { assertCloudRequestId, assertCloudToken, assertNamespaceId, cloudBaseUrl } from './cloud-transport.js';
import { newCloudId } from './cloud-identifiers.js';
import { inputText, pageLimit, required, restrict, UsageError } from './cli-runtime.js';
import type { CliContext, CliResult } from './cli-runtime.js';
import { parseJson, stringifyJson } from './codec.js';

type Change = { operation: 'put'; body: CloudNamespacePut } | { operation: 'rotate-token'; body: CloudNamespaceRotation } | { operation: 'control'; body: CloudControlInput };
type Pending = { base_url: string; deployment_id: string; namespace_id: string } & Change;

function validVersion(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value > 0 && value <= 0xffff_ffff;
}

async function loadPending(path: string): Promise<Pending> {
  try {
    const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    let input: unknown;
    try {
      const stat = await file.stat();
      if (!stat.isFile() || (process.getuid && (stat.uid !== process.getuid() || (stat.mode & 0o077) !== 0))) {
        throw new UsageError('Pending file must be a private regular file owned by this user (mode 0600 or stricter)');
      }
      input = parseJson(await file.readFile('utf8'));
    } finally { await file.close(); }
    if (!input || typeof input !== 'object' || !('base_url' in input) || typeof input.base_url !== 'string'
        || !('deployment_id' in input) || typeof input.deployment_id !== 'string' || !input.deployment_id
        || !('namespace_id' in input) || typeof input.namespace_id !== 'string' || !('operation' in input)
        || !('body' in input) || !input.body || typeof input.body !== 'object') throw new UsageError('Invalid cloud pending file');
    const identity = { base_url: cloudBaseUrl(input.base_url, 'admin'), deployment_id: input.deployment_id, namespace_id: input.namespace_id };
    assertNamespaceId(identity.namespace_id);
    if (input.operation === 'control') return { ...identity, operation: 'control', body: snapshotControlInput(input.body) };
    if (!('request_id' in input.body) || typeof input.body.request_id !== 'string'
        || !('token' in input.body) || typeof input.body.token !== 'string') throw new UsageError('Invalid cloud pending file');
    const { request_id, token } = input.body;
    assertCloudRequestId(request_id);
    assertCloudToken(token);
    if (input.operation === 'put') return { ...identity, operation: 'put', body: { request_id, token } };
    if (input.operation === 'rotate-token' && 'expected_version' in input.body && validVersion(input.body.expected_version)) {
      return { ...identity, operation: 'rotate-token', body: { request_id, token, expected_version: input.body.expected_version } };
    }
    throw new UsageError('Invalid cloud pending file');
  } catch (error) {
    if (error instanceof UsageError) throw error;
    throw new UsageError('Cannot load cloud pending file; use an existing private file created by this CLI');
  }
}

async function savePending(input: Pending, path?: string): Promise<string> {
  const identity = JSON.stringify([input.base_url, input.deployment_id, input.namespace_id, input.operation, input.body.request_id]);
  const state = process.env.XDG_STATE_HOME ?? join(homedir(), '.local', 'state');
  const pathToFile = path ?? join(state, 'agentx-http', 'cloud-admin', `${createHash('sha256').update(identity).digest('hex')}.json`);
  const parent = dirname(pathToFile);
  await mkdir(parent, { recursive: true, mode: 0o700 });
  let file;
  try { file = await open(pathToFile, 'wx', 0o600); }
  catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code === 'EEXIST') {
      throw new UsageError('Pending file already exists; use admin pending replay FILE to resend the original request');
    }
    throw error;
  }
  try { await file.writeFile(`${stringifyJson(input)}\n`); await file.sync(); }
  finally { await file.close(); }
  if (process.platform !== 'win32') {
    const directory = await open(parent, 'r');
    try { await directory.sync(); } finally { await directory.close(); }
  }
  return pathToFile;
}

function showPending(path: string, bearer: string, input: Pending): void {
  // The environment or caller may choose a path containing the secret.
  const redacted = (value: string) => (input.operation === 'control' ? [bearer] : [bearer, input.body.token])
    .reduce((text, secret) => text.replaceAll(secret, '[redacted]'), value);
  console.error(stringifyJson({ pending: redacted(path), ...(input.operation === 'control'
    ? { namespace_id: input.namespace_id, request_id: redacted(input.body.request_id) } : {}) }));
}

/** @aiPurpose Execute an admin command with the selected deployment identity and caller-owned credential. */
export async function runCloudAdmin(ctx: CliContext): Promise<CliResult> {
  const [, subject, action, operand] = ctx.positionals, v = ctx.values;
  const allow = (flags: string[], length: number) => restrict(ctx, ['admin-url', ...flags], length);
  const output = (value: unknown): CliResult => ({ value, domain: 'cloud-admin', schema: 'json' });
  const opts = { signal: ctx.signal };
  if (subject === 'pending' && action === 'reveal') {
    restrict(ctx, [], 4);
    const input = await loadPending(required(operand, 'FILE'));
    if (input.operation === 'control') throw new UsageError('Control requests do not contain a communication token');
    return output({ token: input.body.token });
  }

  const selected = v['admin-url'] ?? process.env.AGENTX_ADMIN_URL ?? ctx.config.admin_url;
  if (typeof selected !== 'string' || !selected) throw new UsageError('--admin-url or AGENTX_ADMIN_URL is required');
  const baseUrl = cloudBaseUrl(selected, 'admin');
  const bearer = required(process.env.AGENTX_ADMIN_TOKEN, 'AGENTX_ADMIN_TOKEN');
  const api = createCloudAdminClient({ baseUrl, token: bearer, fetch: ctx.fetch });
  const identifyDeployment = async () => {
    const { deployment_id } = await api.overview(opts);
    if (!deployment_id) throw new UsageError('Missing cloud deployment identity');
    return deployment_id;
  };
  const send = (input: Pending) => input.operation === 'control'
    ? api.submitControlOperation(input.namespace_id, input.body, opts)
    : input.operation === 'put' ? api.putNamespace(input.namespace_id, input.body, opts) : api.rotateNamespaceToken(input.namespace_id, input.body, opts);

  if (subject === 'overview') {
    allow([], 2);
    return output(await api.overview(opts));
  }
  if (subject === 'pending' && action === 'replay') {
    allow([], 4);
    const path = required(operand, 'FILE');
    const input = await loadPending(path);
    if (input.base_url !== baseUrl) throw new UsageError('Selected admin deployment differs from the original pending request');
    if (input.deployment_id !== await identifyDeployment()) throw new UsageError('The deployment ID at this URL differs from the original pending request');
    showPending(path, bearer, input);
    if (input.operation === 'control') {
      try { return output(await api.recoverControlOperation(input.namespace_id, input.body, opts)); }
      catch (error) {
        // A missing receipt alone permits explicit same-input replay; other errors leave the original pending.
        if (!(error instanceof CloudAdminError && error.status === 404 && error.code !== 'InvalidErrorResponse')) throw error;
      }
    }
    return output(await send(input));
  }
  if (subject === 'hosts' && action === 'capabilities') {
    allow(['host'], 4);
    return output(await api.getHostCapabilities(required(operand, 'SCOPE'), required(v.host, '--host'), opts));
  }
  if (subject === 'operations') {
    const namespace_id = required(operand, 'SCOPE');
    assertNamespaceId(namespace_id);
    if (action === 'show') {
      allow(['request-id'], 4);
      return output(await api.getControlOperation(namespace_id, required(v['request-id'], '--request-id'), opts));
    }
    if (action === 'launch' || action === 'stop' || action === 'add-workspace') {
      allow(['generation', 'host', 'installation-id', 'request-id', 'pending',
        ...(action === 'launch' ? ['workspace-id', 'role', 'title', 'name', 'task', 'directory', 'create-directory'] : action === 'add-workspace' ? ['parent', 'directory', 'name', 'create-directory'] : ['name', 'incarnation'])], 4);
      if (v.role && v.role.length !== 1) throw new UsageError('Select at most one --role for a cloud launch');
      const command: CloudControlInput['command'] = action === 'launch'
        ? { kind: 'launch', workspace_id: required(v['workspace-id'], '--workspace-id'), role: v.role?.[0], name: v.name, title: v.title, initial_prompt: v.task, working_directory: v.directory, create_directory: v['create-directory'] }
        : action === 'add-workspace'
        ? { kind: 'add_workspace', name: required(v.name, '--name'), parent: required(v.parent, '--parent'), directory: required(v.directory, '--directory'), create_directory: !!v['create-directory'] }
        : { kind: 'stop', name: required(v.name, '--name'), incarnation: required(v.incarnation, '--incarnation') };
      const body = snapshotControlInput({
        request_id: v['request-id'] ?? newCloudId(), generation: required(v.generation, '--generation'),
        host: required(v.host, '--host'), installation_id: required(v['installation-id'], '--installation-id'), command,
      });
      const input: Pending = { base_url: baseUrl, deployment_id: await identifyDeployment(), namespace_id, operation: 'control', body };
      const path = await savePending(input, v.pending);
      showPending(path, bearer, input);
      return output(await send(input));
    }
  }
  if (subject === 'namespaces') {
    const pageFlags = ['after', 'limit'];
    if (action === 'list' || action === 'hosts' || action === 'agents' || action === 'workspaces') {
      const inventoryFlags = [...pageFlags, 'generation'];
      allow(action === 'agents' ? [...inventoryFlags, 'host', 'workspace-id'] : action === 'workspaces' ? [...inventoryFlags, 'host'] : action === 'hosts' ? inventoryFlags : pageFlags, action === 'list' ? 3 : 4);
      const paging = { after: v.after, limit: pageLimit(v.limit) };
      if (action === 'list') return output(await api.listNamespaces(paging, opts));
      const page = { ...paging, generation: v.after ? required(v.generation, '--generation from the original inventory page') : v.generation };
      const id = required(operand, 'ID');
      if (action === 'hosts') return output(await api.listHosts(id, page, opts));
      if (action === 'workspaces') return output(await api.listWorkspaces(id, { ...page, host: v.host }, opts));
      return output(await api.listAgents(id, { ...page, host: v.host, workspace_id: v['workspace-id'] }, opts));
    }
    if (action === 'show' || action === 'get') {
      allow([], 4);
      return output(await api.getNamespace(required(operand, 'ID'), opts));
    }
    if (action === 'create' || action === 'put' || action === 'rotate-token') {
      const writeFlags = ['request-id', 'pending', 'new-token-file'];
      allow(action === 'rotate-token' ? [...writeFlags, 'expected-version'] : writeFlags, 4);
      const namespace_id = required(operand, 'ID'), request_id = v['request-id'] ?? newCloudId();
      assertNamespaceId(namespace_id);
      assertCloudRequestId(request_id);
      const token = v['new-token-file'] === undefined
        ? newCloudId() : (await inputText(v['new-token-file'])).trim();
      assertCloudToken(token);
      let change: Change;
      if (action !== 'rotate-token') change = { operation: 'put', body: { request_id, token } };
      else {
        const version = v['expected-version'];
        const expected_version = version && /^[1-9]\d*$/.test(version) ? Number(version) : undefined;
        if (!validVersion(expected_version)) throw new UsageError('--expected-version must be a positive 32-bit integer');
        change = { operation: action, body: { request_id, token, expected_version } };
      }
      const input: Pending = { base_url: baseUrl, deployment_id: await identifyDeployment(), namespace_id, ...change };
      const path = await savePending(input, v.pending);
      showPending(path, bearer, input);
      return output(await send(input));
    }
  }
  throw new UsageError('Unknown admin command; see agentx-http --help');
}
