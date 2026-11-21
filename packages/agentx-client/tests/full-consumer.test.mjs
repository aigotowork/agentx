import test from 'node:test';
import assert from 'node:assert/strict';
import {
  createAgentXClient, createWorkbenchClient, createObservationClient, createManagementClient,
  parseWorkbenchJson, stringifyWorkbenchJson, parseObservationJson, WorkOutcomeUnknown,
  participantFromAgent, MutationOutcomeUnknown,
} from '../dist/index.js';

const response = (value, status = 200) => new Response(typeof value === 'string' ? value : JSON.stringify(value), { status, headers: { 'Content-Type': 'application/json' } });
const marker = value => ({ $integer: String(value) });
const work = { scope: '/scope', generation: 'G', project: '/project', id: 'W' };
const apply = { generation: 'G', command_id: 'Q', work, expected_revision: 18446744073709551615n, command: { SendNote: { roles: ['Implementer'], text: '' } } };

test('all-integer wire policy derives bigint with bounds, strict markers, preserved opaque identities and duplicate refusal', () => {
  const native = parseWorkbenchJson('ParticipantRef', '{"canonical_name":"9007199254740992","incarnation_bits":{"$integer":"18446744073709551615"}}');
  assert.deepEqual(native, { canonical_name: '9007199254740992', incarnation_bits: 18446744073709551615n });
  assert.deepEqual(JSON.parse(stringifyWorkbenchJson('ParticipantRef', native)), { canonical_name: '9007199254740992', incarnation_bits: marker('18446744073709551615') });
  for (const bad of ['-0', '01', '+1', '18446744073709551616', '-1']) {
    assert.throws(() => parseWorkbenchJson('ParticipantRef', JSON.stringify({ canonical_name: 'naze', incarnation_bits: marker(bad) })));
  }
  assert.throws(() => parseWorkbenchJson('ParticipantRef', '{"canonical_name":"a","canonical_name":"b","incarnation_bits":{"$integer":"1"}}'));
  assert.throws(() => parseWorkbenchJson('Page', '{"limit":32,"after":null}'));
  assert.throws(() => parseWorkbenchJson('Page', '{"limit":{"$integer":"32","extra":true},"after":null}'));
  assert.throws(() => stringifyWorkbenchJson('Page', { limit: 32, after: null }));
  assert.deepEqual(participantFromAgent({ name: 'naze', incarnation: '18446744073709551615', stopped_event: null }), { canonical_name: 'naze', incarnation_bits: 18446744073709551615n });
  assert.throws(() => participantFromAgent({ name: 'naze', incarnation: null, stopped_event: 'old' }));
});

test('Workbench generated requests carry markers without fabricating Origin; native Rejected remains a value, response loss preserves complete original intent', async () => {
  let posted;
  const api = createWorkbenchClient({ baseUrl: 'http://127.0.0.1:43210', fetch: async request => {
    assert.equal(request.headers.get('origin'), null);
    posted = JSON.parse(await request.text());
    return response({ Rejected: { RevisionConflict: { current_revision: marker('9007199254740992') } } });
  } });
  const rejected = await api.works.note({ generation: apply.generation, command_id: apply.command_id, work, expected_revision: apply.expected_revision }, apply.command.SendNote);
  assert.equal(rejected.Rejected.RevisionConflict.current_revision, 9007199254740992n);
  assert.equal(posted.expected_revision.$integer, '18446744073709551615');
  assert.deepEqual(posted.command, apply.command);
  let count = 0;
  const lost = createWorkbenchClient({ baseUrl: 'http://127.0.0.1:43210', fetch: async () => { ++count; throw new Error('reply lost'); } });
  await assert.rejects(lost.apply(apply), error => error instanceof WorkOutcomeUnknown && error.input.command_id === 'Q' && error.input.expected_revision === apply.expected_revision && Object.isFrozen(error.input.command));
  assert.equal(count, 1);
});

test('three endpoints remain independent and passive scope/failure reads preserve native evidence', async () => {
  const paths = [];
  const failure = { LocalFailure: { stage: 'Connect', kind: 'EndpointMissing', os_code: null, at: null } };
  const clients = createAgentXClient({ observationUrl: 'http://127.0.0.1:43211', fetch: async request => {
    const url = new URL(request.url); paths.push(url.pathname);
    if (url.pathname.endsWith('/scope')) return response({ canonical_scope: '/missing' });
    assert.deepEqual(JSON.parse(await request.text()), { expected_daemon: null, read: 'Snapshot' });
    return response(failure);
  } });
  assert.equal(clients.management, undefined); assert.equal(clients.workbench, undefined);
  assert.deepEqual(await clients.observation.scope(), { canonical_scope: '/missing' });
  assert.deepEqual(await clients.observation.runtime.snapshot(), failure);
  assert.deepEqual(paths, ['/api/v1/scope', '/api/v1/read']);
  clients.observation.close();
  await assert.rejects(clients.observation.runtime.snapshot());
  assert.equal(parseObservationJson('LocalFailure', JSON.stringify({ LocalFailure: failure.LocalFailure }.LocalFailure)).kind, 'EndpointMissing');
});

test('Inspector uses actual single external tag and ordinary lossless integers; no management Operation is fabricated', async () => {
  let posted;
  const snapshot = '{"generation":"G","settings":{"enabled":false,"collected_from":null,"revision":18446744073709551615,"recovery":{"enabled":false,"error_duration_seconds":600}},"coverage":{"observed_at":null,"retained_from":null,"discovery_complete":false,"dropped_observations":0,"gaps":[],"basis":"native"},"observations":[],"jobs":[],"attempts":[],"dropped_attempts":0,"observation_revision":0}';
  const api = createManagementClient({ baseUrl: 'http://127.0.0.1:43212', fetch: async request => { posted = await request.text(); return response(snapshot); } });
  const result = await api.inspection.configure({ generation: 'G', expected_revision: 18446744073709551615n, enabled: false, recovery: { enabled: false, error_duration_seconds: 600 } });
  assert.match(posted, /^\{"Configure":/); assert.match(posted, /"expected_revision":18446744073709551615/); assert.doesNotMatch(posted, /\$integer/);
  assert.equal(result.settings.revision, 18446744073709551615n); assert.equal(result.settings.enabled, false); assert.equal(result.settings.collected_from, null);
  const lost = createManagementClient({ baseUrl: 'http://127.0.0.1:43212', fetch: async () => { throw new Error('lost'); } });
  await assert.rejects(lost.inspection.configure({ generation: 'G', expected_revision: 1n, enabled: false, recovery: { enabled: false, error_duration_seconds: 600 } }), error => error instanceof MutationOutcomeUnknown && error.operation === 'inspection.configure' && error.input.expected_revision === 1n);
});

test('attachment upload/download use exact raw bytes, not JSON or multipart', async () => {
  const bytes = new Uint8Array([0, 255, 1, 2]);
  const api = createManagementClient({ baseUrl: 'http://127.0.0.1:43213', fetch: async request => {
    if (request.method === 'POST') {
      assert.deepEqual(new Uint8Array(await request.arrayBuffer()), bytes);
      return response({ id: 'asset', name: 'x.bin', media_type: 'application/octet-stream', size: 4, url: '/api/v1/attachments/asset' }, 201);
    }
    return new Response(bytes, { headers: { 'Content-Type': 'application/octet-stream' } });
  } });
  const uploaded = await api.attachments.upload({ generation: 'G', name: 'x.bin', bytes: new Blob([bytes]) });
  assert.equal(uploaded.size, 4n);
  assert.deepEqual(new Uint8Array(await (await api.attachments.download({ generation: 'G', id: 'asset' })).arrayBuffer()), bytes);
});

test('management ordinary JSON preserves native finite doubles separately from bounded integers', async () => {
  const { parseManagementJson, stringifyManagementJson } = await import('../dist/index.js');
  const usage = { input:'1', output:'2', cache_read:'3', cache_write:'4', reasoning:null, context_tokens:null, context_window:null, context_usage_ratio:0.5, model:null, unit:'tokens', basis:'finite' };
  assert.equal(parseManagementJson('Usage', JSON.stringify(usage)).context_usage_ratio, 0.5);
  assert.equal(JSON.parse(stringifyManagementJson('Usage', usage)).context_usage_ratio, 0.5);
  const binding = { path:'x', checked_at:1.25, last_success_at:0.75 };
  assert.equal(parseManagementJson('FileBindingView', JSON.stringify(binding)).checked_at, 1.25);
  assert.equal(parseManagementJson('FileBindingView', JSON.stringify(binding)).last_success_at, 0.75);
  assert.throws(() => parseManagementJson('Attachment', '{"id":"a","name":"b","media_type":"x","size":0.5,"url":"/a"}'));
});

test('caller storage round-trips nested management longs without rounding and restores emitted types', async () => {
  const { parseJson, stringifyJson, parseManagementJson } = await import('../dist/index.js');
  const intent = { generation: 'G', expected_revision: 18446744073709551615n, enabled: true, recovery: { enabled: false, error_duration_seconds: 600 } };
  const text = stringifyJson({ input: intent, at: 123, opaque: '9007199254740993' });
  const envelope = parseJson(text);
  assert.equal(envelope.input.expected_revision, 18446744073709551615n);
  assert.equal(envelope.at, 123);
  assert.equal(envelope.opaque, '9007199254740993');
  assert.deepEqual(parseManagementJson('ConfigureInput', stringifyJson(envelope.input)), intent);
  // Even safe long integers recover bigint through the owning schema.
  const small = parseManagementJson('ConfigureInput', stringifyJson({ ...intent, expected_revision: 1 }));
  assert.equal(small.expected_revision, 1n);
  assert.throws(() => parseJson('{"id":1,"id":2}'));
  assert.throws(() => parseJson('{"value":1e999}'));
});
