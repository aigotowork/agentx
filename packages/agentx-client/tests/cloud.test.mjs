import assert from 'node:assert/strict';
import { once } from 'node:events';
import { spawn } from 'node:child_process';
import { chmod, mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { CloudAdminError, CloudAdminOutcomeUnknown, CloudScopeOutcomeUnknown, createCloudAdminClient, createCloudScopeClient } from '../dist/index.js';

const adminToken = 'a'.repeat(40);
const proposedToken = 'b'.repeat(43);
const receipt = { id: 'office', scope_url: 'https://cloud.example/scopes/office', credential_version: 2, generation: 'scope-generation' };
const overview = { deployment_id: 'deployment-1', protocol_version: 1, observed_at: 1700000000, scopes: 1, hosts: 1, online_hosts: 1, agents: 0, unavailable: 0 };

test('admin SDK binds bearer to one root and decodes actual long integers and cursors', async () => {
  let calls = 0;
  const client = createCloudAdminClient({
    baseUrl: 'https://cloud.example/',
    token: adminToken,
    fetch: async request => {
      calls++;
      assert.equal(request.headers.get('authorization'), `Bearer ${adminToken}`);
      assert.equal(request.redirect, 'error');
      assert.equal(request.credentials, 'omit');
      if (new URL(request.url).pathname.endsWith('/overview')) {
        return new Response(JSON.stringify(overview).replace('1700000000', '9007199254740993'));
      }
      if (new URL(request.url).pathname.endsWith('/agents')) {
        const url = new URL(request.url);
        assert.deepEqual(Object.fromEntries(url.searchParams), { host: 'host-b', after: 'agent-a', limit: '2' });
        return Response.json({ items: [{ name: 'agent-b', host: 'host-b', incarnation: '901' }], next: null });
      }
      assert.equal(request.url, 'https://cloud.example/api/v1/admin/scopes/office/hosts?after=host-a&limit=2');
      return new Response('{"items":[{"name":"host-b","last_seen":1700000000,"online":true}],"next":"host-b"}');
    },
  });
  assert.equal((await client.overview()).observed_at, 9007199254740993n);
  assert.deepEqual(await client.listHosts('office', { after: 'host-a', limit: 2 }), { items: [{ name: 'host-b', last_seen: 1700000000n, online: true }], next: 'host-b' });
  assert.deepEqual(await client.listAgents('office', { host: 'host-b', after: 'agent-a', limit: 2 }),
    { items: [{ name: 'agent-b', host: 'host-b', incarnation: '901' }], next: null });
  assert.throws(() => createCloudAdminClient({ baseUrl: 'http://cloud.example', token: adminToken }), /HTTPS/);
  assert.throws(() => createCloudAdminClient({ baseUrl: 'https://cloud.example/scopes/office', token: adminToken }), /deployment root/);
  assert.throws(() => client.getScope('../office'), /Scope ID/);
  assert.equal(calls, 3);
});

test('admin SDK keeps credentials out of failures and does not retry an uncertain mutation', async () => {
  let response;
  let calls = 0;
  const client = createCloudAdminClient({
    baseUrl: 'https://cloud.example', token: adminToken,
    fetch: async request => {
      calls++;
      assert.equal(request.method, 'POST');
      assert.equal(request.url, 'https://cloud.example/api/v1/admin/scopes/office/token-rotations');
      assert.deepEqual(await request.json(), { request_id: 'rotation-1', token: proposedToken, expected_version: 1 });
      return response;
    },
  });
  const rotate = () => client.rotateScopeToken('office', { request_id: 'rotation-1', token: proposedToken, expected_version: 1 });
  response = Response.json({ code: 'conflict', message: `Conflict involving ${proposedToken} and ${adminToken}` }, { status: 409 });
  await assert.rejects(rotate, error => error instanceof CloudAdminError && error.code === 'conflict' && error.status === 409
    && !error.message.includes(proposedToken) && !error.message.includes(adminToken));
  response = Response.json({ ...receipt, id: 'other', token: proposedToken });
  await assert.rejects(rotate, error => error instanceof CloudAdminOutcomeUnknown && error.requestId === 'rotation-1'
    && !JSON.stringify(error).includes(proposedToken) && error.cause === undefined);
  assert.equal(calls, 2);
});

function execute(args, env) {
  return new Promise((resolve, reject) => {
    const binary = process.env.AGENTX_HTTP_TEST_BIN ?? fileURLToPath(new URL('../dist/cli.js', import.meta.url));
    const child = spawn(process.execPath, [binary, ...args], { env });
    let stdout = '', stderr = '';
    child.stdout.setEncoding('utf8').on('data', value => { stdout += value; });
    child.stderr.setEncoding('utf8').on('data', value => { stderr += value; });
    child.once('error', reject);
    child.once('close', code => resolve({ code, stdout, stderr }));
  });
}

const control = {
  generation: 'generation-1', host: 'host-a', installation_id: 'installation-1', request_id: 'request:42',
  command: { kind: 'stop', name: 'agent_a', incarnation: '18446744073709551615' },
};
const queued = { ...control, status: 'queued', claim_id: null, outcome: null };

test('control SDK binds catalogue and original receipt to exact identities', async () => {
  let result = queued;
  const calls = [];
  const api = createCloudAdminClient({ baseUrl: 'https://cloud.example', token: adminToken, fetch: async request => {
    const path = decodeURIComponent(new URL(request.url).pathname);
    calls.push([request.method, path]);
    assert.equal(request.headers.get('authorization'), `Bearer ${adminToken}`);
    if (path.endsWith('/capabilities')) {
      return new Response('{"generation":"generation-1","host":"host-a","installation_id":"installation-1","online":true,"observed_at":9007199254740993,"workspaces":[]}');
    }
    if (request.method === 'POST') assert.deepEqual(await request.json(), control);
    return Response.json(result);
  } });
  assert.equal((await api.getHostCapabilities('office', control.host)).observed_at, 9007199254740993n);
  assert.deepEqual(await api.submitControlOperation('office', control), queued);
  assert.throws(() => api.getControlOperation('office', '..'), /request ID/);
  result = { ...queued, status: 'succeeded', claim_id: 'claimed', outcome: { status: 'succeeded', agent: {
    name: 'agent_a', host: 'host-a', incarnation: '18446744073709551614',
  } } };
  await assert.rejects(api.recoverControlOperation('office', control),
    error => error instanceof CloudAdminError && error.code === 'InvalidReceipt');
  await assert.rejects(api.submitControlOperation('office', control),
    error => error instanceof CloudAdminOutcomeUnknown && error.requestId === control.request_id);
  assert.deepEqual(calls, [
    ['GET', '/api/v1/admin/scopes/office/hosts/host-a/capabilities'],
    ['POST', '/api/v1/admin/scopes/office/operations'],
    ['GET', '/api/v1/admin/scopes/office/operations/request:42'],
    ['POST', '/api/v1/admin/scopes/office/operations'],
  ]);
});

test('control CLI retains admission intent and only replays after an exact lookup misses', async t => {
  const directory = await mkdtemp(join(tmpdir(), 'agentx-control-cli-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const methods = [], inputs = [];
  let stored = null;
  let drop = 'after';
  const prefix = '/api/v1/admin/scopes/office/operations';
  const server = createServer(async (request, response) => {
    const send = (data, status = 200) => response.writeHead(status, { 'Content-Type': 'application/json' }).end(JSON.stringify(data));
    if (request.headers.authorization !== `Bearer ${adminToken}`) return send({ code: 'unauthorized', message: 'unauthorized' }, 401);
    if (request.url === '/api/v1/admin/overview') return send(overview);
    methods.push(request.method);
    if (request.method === 'GET' && decodeURIComponent(request.url).startsWith(`${prefix}/`)) {
      return stored ? send(stored) : send({ code: 'not_found', message: 'not found' }, 404);
    }
    if (request.method !== 'POST' || request.url !== prefix) return send({ code: 'not_found', message: 'not found' }, 404);
    let raw = '';
    for await (const chunk of request) raw += chunk;
    const input = JSON.parse(raw);
    inputs.push(input);
    if (drop !== 'before') stored = { ...input, status: 'queued', claim_id: null, outcome: null };
    if (drop) {
      drop = '';
      request.socket.destroy();
    } else send(stored);
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  t.after(() => new Promise(resolve => server.close(resolve)));
  const baseUrl = `http://127.0.0.1:${server.address().port}`;
  const env = { ...process.env, AGENTX_ADMIN_URL: baseUrl, AGENTX_ADMIN_TOKEN: adminToken, AGENTX_API_CONFIG: undefined, XDG_CONFIG_HOME: directory };
  const pending = join(directory, 'stop.json');
  const target = ['--generation', control.generation, '--host', control.host, '--installation-id', control.installation_id];
  const stop = await execute(['admin', 'operations', 'stop', 'office', ...target,
    '--name', control.command.name, '--incarnation', control.command.incarnation, '--request-id', control.request_id, '--pending', pending], env);
  assert.equal(stop.code, 3);
  assert.deepEqual(JSON.parse(await readFile(pending, 'utf8')), { base_url: baseUrl, deployment_id: overview.deployment_id, scope_id: 'office', operation: 'control', body: control });
  if (process.getuid) assert.equal((await stat(pending)).mode & 0o777, 0o600);
  assert.match(stop.stderr, /OutcomeUnknown/);
  assert.ok(!stop.stderr.includes(adminToken));
  const after = await execute(['admin', 'pending', 'replay', pending], env);
  assert.equal(after.code, 0, after.stderr);
  assert.deepEqual(JSON.parse(after.stdout), queued);
  assert.deepEqual(methods, ['POST', 'GET']);
  assert.deepEqual(inputs, [control]);

  drop = 'before';
  stored = null;
  const second = join(directory, 'launch.json');
  const launch = await execute(['admin', 'operations', 'launch', 'office', ...target,
    '--workspace', 'studio', '--name', 'new_agent', '--title', 'Summary', '--task', 'Read the lab notes.', '--pending', second], env);
  assert.equal(launch.code, 3, launch.stderr);
  const resumed = await execute(['admin', 'pending', 'replay', second], env);
  assert.equal(resumed.code, 0, resumed.stderr);
  assert.deepEqual(methods, ['POST', 'GET', 'POST', 'GET', 'POST']);
  assert.deepEqual(inputs[1], inputs[2]);
  assert.deepEqual(inputs[1].command, { kind: 'launch', workspace: 'studio', name: 'new_agent', title: 'Summary', initial_prompt: 'Read the lab notes.' });
  const check = await execute(['admin', 'operations', 'show', 'office', '--request-id', inputs[1].request_id], env);
  assert.equal(check.code, 0, check.stderr);
  assert.deepEqual(JSON.parse(check.stdout), stored);
});

test('admin CLI retains exact mutation across lost response, binds replay to the deployment, and reveals only on request', async t => {
  const directory = await mkdtemp(join(tmpdir(), 'agentx-admin-cli-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const requests = [];
  let deploymentId = overview.deployment_id;
  const server = createServer(async (request, response) => {
    if (request.method === 'GET' && request.url === '/api/v1/admin/overview') {
      response.writeHead(200, { 'content-type': 'application/json' });
      response.end(JSON.stringify({ ...overview, deployment_id: deploymentId }));
      return;
    }
    let body = '';
    for await (const chunk of request) body += chunk;
    requests.push({ path: request.url, method: request.method, auth: request.headers.authorization, body });
    if (requests.length === 1) request.socket.destroy();
    else {
      response.writeHead(200, { 'content-type': 'application/json' });
      response.end(JSON.stringify(receipt));
    }
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  t.after(() => new Promise(resolve => server.close(resolve)));
  const baseUrl = `http://127.0.0.1:${server.address().port}`;
  const env = { ...process.env, AGENTX_ADMIN_URL: baseUrl, AGENTX_ADMIN_TOKEN: adminToken, AGENTX_API_CONFIG: undefined, XDG_CONFIG_HOME: directory };
  const pending = join(directory, 'rotation.json');
  const first = await execute(['admin', 'scopes', 'rotate-token', 'office', '--expected-version', '1', '--request-id', 'request-1', '--pending', pending], env);
  assert.equal(first.code, 3);
  assert.equal(requests.length, 1);
  const body = JSON.parse(requests[0].body);
  const saved = JSON.parse(await readFile(pending, 'utf8'));
  assert.deepEqual(saved, { base_url: baseUrl, deployment_id: overview.deployment_id, scope_id: 'office', operation: 'rotate-token', body });
  assert.equal(body.request_id, 'request-1');
  assert.equal(body.expected_version, 1);
  assert.match(body.token, /^[A-Za-z0-9_-]{43}$/);
  if (process.getuid) assert.equal((await stat(pending)).mode & 0o777, 0o600);
  assert.equal(first.stdout, '');
  assert.ok(first.stderr.includes(pending) && first.stderr.includes('OutcomeUnknown'));
  assert.ok(!first.stderr.includes(body.token) && !first.stderr.includes(adminToken));
  assert.equal(requests[0].path, '/api/v1/admin/scopes/office/token-rotations');
  assert.equal(requests[0].auth, `Bearer ${adminToken}`);

  const mismatch = await execute(['admin', 'pending', 'replay', pending], { ...env, AGENTX_ADMIN_URL: 'https://different.example' });
  assert.equal(mismatch.code, 2);
  assert.equal(requests.length, 1);
  deploymentId = 'replacement-deployment';
  const sameUrlMismatch = await execute(['admin', 'pending', 'replay', pending], env);
  assert.equal(sameUrlMismatch.code, 2);
  assert.match(sameUrlMismatch.stderr, /deployment ID/);
  assert.equal(requests.length, 1);
  deploymentId = overview.deployment_id;
  const replay = await execute(['admin', 'pending', 'replay', pending], env);
  assert.equal(replay.code, 0);
  assert.deepEqual(requests, [requests[0], requests[0]]);
  assert.deepEqual(JSON.parse(replay.stdout), receipt);
  assert.ok(!replay.stderr.includes(body.token) && !replay.stderr.includes(adminToken));

  const reveal = await execute(['admin', 'pending', 'reveal', pending], { ...env, AGENTX_ADMIN_URL: undefined, AGENTX_ADMIN_TOKEN: undefined });
  assert.equal(reveal.code, 0);
  assert.deepEqual(JSON.parse(reveal.stdout), { token: body.token });
  if (process.getuid) {
    await chmod(pending, 0o644);
    const refused = await execute(['admin', 'pending', 'reveal', pending], env);
    assert.equal(refused.code, 2);
    assert.equal(refused.stdout, '');
    assert.ok(!refused.stderr.includes(body.token));
  }
});

test('scope SDK uses separate bearer and preserves original claim, generation and decimal message identity', async () => {
  const agent = { name: 'receiver', host: 'host-b', incarnation: '9007199254740997' };
  const from = { name: 'sender', host: 'host-a', incarnation: '9007199254740999' };
  const id = '9007199254740993';
  const claim = { generation: 'scope-generation', installation_id: 'installation-b', agent, claim_id: 'claim-1' };
  const prefix = 'https://cloud.example/scopes/office/api/v1';
  const methods = [];
  const api = createCloudScopeClient({
    baseUrl: receipt.scope_url,
    token: proposedToken,
    fetch: async request => {
      assert.equal(request.headers.get('authorization'), `Bearer ${proposedToken}`);
      assert.ok(request.url.startsWith(prefix));
      const path = request.url.slice(prefix.length);
      methods.push(path);
      if (path === '/descriptor') return Response.json({ scope_id: 'office', generation: claim.generation, protocol_version: 1 });
      const supplied = await request.json();
      if (path === '/inbox/claim') {
        assert.deepEqual(supplied, { ...claim, wait_ms: 100 });
        throw new TypeError('simulated interrupted response');
      }
      if (path === '/inbox/confirm') {
        assert.deepEqual(supplied, { ...claim, message_ids: [id] });
        return Response.json({ confirmed: [id] });
      }
      assert.equal(path, '/inbox/recover');
      assert.deepEqual(supplied, claim);
      const result = { claim_id: claim.claim_id, next: null, items: [{ id, from, to: agent, body: 'hello', intent: 'request', reply_to: null, thread: null }] };
      return new Response(JSON.stringify(result).replace('"body":"hello"', '"body":"hello","created_at":9007199254740995'));
    },
  });
  assert.equal((await api.describe()).generation, claim.generation);
  await assert.rejects(api.claimInbox({ ...claim, wait_ms: 100 }), CloudScopeOutcomeUnknown);
  assert.deepEqual(methods, ['/descriptor', '/inbox/claim']);
  const recovered = await api.recoverInbox(claim);
  assert.equal(recovered.items[0].created_at, 9007199254740995n);
  assert.equal(recovered.items[0].id, id);
  assert.deepEqual(await api.confirmInbox({ ...claim, message_ids: [id] }), { confirmed: [id] });
  assert.deepEqual(methods, ['/descriptor', '/inbox/claim', '/inbox/recover', '/inbox/confirm']);
});
