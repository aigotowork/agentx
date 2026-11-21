import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createManagementClient, parseManagementJson, stringifyManagementJson, OutcomeUnknown, ReceiptError, ObservationStopped, ManagementError, ProtocolError } from '../dist/index.js';

const identity = { generation: 'G', request_id: 'Q' };
const agent = { name: 'naze', incarnation: '18446744073709551615', stopped_event: null };
const input = { ...identity, agent, message: { text: '', attachments: ['asset-1'], intent: 'ack', reply_to: '18446744073709551615', thread: '' } };
const op = (status = 'Accepted', result = null, error = null) => ({ ...identity, status, result, error });
const respond = (body, status = 200) => new Response(body, { status, headers: { 'content-type': 'application/json' } });
const apiFor = (body) => createManagementClient({ baseUrl: 'http://finite.invalid', fetch: async () => respond(body) });

// This is the public generated transport + handwritten codec boundary, not real AgentX/native evidence.
test('generated transport preserves numeric tokens and field-specific string/null semantics', async () => {
  const raw = `{"agent":${JSON.stringify(agent)},"tool":"qoder","status":"active","stored_status":"active","status_context":"","detail":"","directory":"/tmp","session_id":null,"collaboration":{"status":"unavailable","role":null},"can_send":true,"can_stop":false,"can_resume":false,"has_transcript":false,"pending_messages":18446744073709551615,"request_error":{"consecutive_failures":9007199254740992,"error_since":-9223372036854775808,"last_error_at":9223372036854775807,"reason":"","source":"hook","turn_id":"9007199254740992"}}`;
  const actual = await apiFor(raw).getAgent({ generation: 'G', agent });
  assert.equal(actual.pending_messages, 18446744073709551615n);
  assert.equal(actual.request_error.consecutive_failures, 9007199254740992n);
  assert.equal(actual.request_error.error_since, -9223372036854775808n);
  assert.equal(actual.request_error.last_error_at, 9223372036854775807n);
  assert.equal(actual.request_error.turn_id, '9007199254740992');
  assert.equal(actual.agent.incarnation, agent.incarnation);
  assert.equal(actual.session_id, null);
  assert.equal(actual.can_stop, false);
  assert.match(stringifyManagementJson('AgentView', actual), /"pending_messages":18446744073709551615/);
  for (const value of ['0', '9007199254740991', '9007199254740992', '18446744073709551615']) {
    assert.equal(parseManagementJson('AgentView', raw.replace('"pending_messages":18446744073709551615', `"pending_messages":${value}`)).pending_messages, BigInt(value));
  }
  assert.throws(() => parseManagementJson('AgentView', raw.replace('"pending_messages":18446744073709551615', '"pending_messages":18446744073709551616')));
  assert.throws(() => stringifyManagementJson('AgentView', { ...actual, pending_messages: 9007199254740992 }));
  assert.throws(() => stringifyManagementJson('AgentView', { ...actual, pending_messages: '9007199254740992' }));
});

test('generated send preserves full payload; response loss retains a detached original input without retry', async () => {
  let count = 0;
  let posted;
  const mutable = structuredClone(input);
  const api = createManagementClient({ baseUrl: 'http://finite.invalid', fetch: async (request) => {
    ++count; posted = await request.text(); mutable.message.text = 'changed after dispatch'; throw new TypeError('response lost');
  } });
  await assert.rejects(api.send(mutable), error => {
    assert.ok(error instanceof OutcomeUnknown);
    assert.deepEqual(error.input, input);
    assert.ok(Object.isFrozen(error.input.message)); return true;
  });
  assert.deepEqual(JSON.parse(posted), input);
  assert.equal(count, 1);
});

test('typed HTTP rejection differs from transport unknown; receipt identity and external tag checked', async () => {
  const api = createManagementClient({ baseUrl: 'http://finite.invalid', fetch: async () => respond('{"error":{"code":"StaleReference","message":"old target"}}', 409) });
  await assert.rejects(api.send(input), error => error instanceof ManagementError && error.code === 'StaleReference');
  await assert.rejects(apiFor(JSON.stringify({ ...op(), request_id: 'wrong' })).getOperation(identity), error => error instanceof ReceiptError && error.operation.request_id === 'wrong');
  await assert.rejects(apiFor(JSON.stringify(op('Succeeded', { Stopped: { name: 'naze', outcome: 'stopped' } }))).waitForSent(identity), error => error instanceof ReceiptError && error.code === 'ProtocolError' && 'Stopped' in error.operation.result);
  await assert.rejects(apiFor(JSON.stringify(op('Unknown'))).waitForSent(identity), error => error instanceof ReceiptError && error.code === 'OutcomeUnknown');
  await assert.rejects(apiFor(JSON.stringify(op('Failed', null, 'failed'))).waitForSent(identity), error => error instanceof ReceiptError && error.code === 'Failed');
  await assert.rejects(apiFor(JSON.stringify(op('Succeeded', { Sent: { message_id: '1', recipients: [] }, Stopped: { name: 'naze', outcome: 'stopped' } }))).waitForSent(identity), error => error instanceof ProtocolError && error.rawText.includes('Stopped'));
  const result = { message_id: '18446744073709551615', recipients: ['naze'] };
  assert.deepEqual(await apiFor(JSON.stringify(op('Succeeded', { Sent: result }))).waitForSent(identity), result);
});

test('cancel and deadline stop observation, retain identity and last receipt, never issue a mutation', async () => {
  let count = 0;
  const controller = new AbortController();
  const api = createManagementClient({ baseUrl: 'http://finite.invalid', fetch: async (request) => {
    assert.equal(request.method, 'GET'); ++count;
    queueMicrotask(() => controller.abort()); return respond(JSON.stringify(op()));
  } });
  await assert.rejects(api.waitForSent(identity, { signal: controller.signal }), error => error instanceof ObservationStopped && error.code === 'Canceled' && error.identity.request_id === 'Q');
  assert.equal(count, 1);
  await assert.rejects(apiFor(JSON.stringify(op())).waitForOperation(identity, { timeoutMs: 5, pollIntervalMs: 10 }), error => error instanceof ObservationStopped && error.code === 'DeadlineExceeded' && error.lastOperation.status === 'Accepted');
});

test('generated query keeps false and AND label encoding; CLI help offline and unknown flags rejected', async () => {
  let url;
  const api = createManagementClient({ baseUrl: 'http://finite.invalid', fetch: async request => {
    url = new URL(request.url); return respond('{"agents":[],"generation":"G","current_complete":false,"next":null}');
  } });
  await api.listAgents({ generation: 'G', current_only: false, label: ['a', 'b'] });
  assert.equal(url.searchParams.get('current_only'), 'false');
  assert.deepEqual(url.searchParams.getAll('label'), ['a', 'b']);
  const bin = new URL('../dist/cli.js', import.meta.url);
  const result = spawnSync(process.execPath, [bin.pathname, '--help'], { encoding: 'utf8', env: {} });
  assert.equal(result.status, 0); assert.match(result.stdout, /agentx-http/); assert.equal(result.stderr, '');
  const invalid = spawnSync(process.execPath, [bin.pathname, '--bogus'], { encoding: 'utf8', env: {} });
  assert.equal(invalid.status, 2); assert.equal(invalid.stdout, ''); assert.equal(JSON.parse(invalid.stderr).code, 'InvalidInput');
});
