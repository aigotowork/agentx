/** Fixture HTTP is confined to this test; production never falls back to fixtures. */
import assert from 'node:assert/strict';
import { directory, messages, query, type Descriptor, type Operation } from '../host/live/client';
import { datasetKey, referenceKey, selectionKey, unknown } from '../host/live/model';
import { blocksTarget, runCommand, reconcile, saveCommands, loadCommands, requestId, type CommandEntry } from '../host/live/operations';
import { latestObservations, inspection, analyze, continuation, configure, cancelAnalysis } from '../host/live/inspection';
import type { Snapshot, Usage } from '../host/native';
const dataset: Descriptor = { scope: '/disposable/test', generation: 'g1', project: '/test', tools: [], profiles: [], max_page: 100, max_text: 65536 };
const current = { name: 'namesake', incarnation: '18446744073709551615', stopped_event: null };
const stopped = { name: 'namesake', incarnation: null, stopped_event: '9007199254740993' };
const later = { ...current, incarnation: '18446744073709551614' };
assert.notEqual(referenceKey(current), referenceKey(stopped));
assert.notEqual(referenceKey(current), referenceKey(later));
assert.notEqual(selectionKey(dataset, current), selectionKey({ ...dataset, generation: 'g2' }, current));
assert.notEqual(datasetKey(dataset), datasetKey({ ...dataset, scope: '/other' }));
assert.equal(new URLSearchParams(query('g1', stopped)).get('stopped_event'), '9007199254740993');
assert.equal(new URLSearchParams(query('g1', current)).get('incarnation'), current.incarnation);
// Native management parses UUIDs before admitting any command (store.rs Request::Submit).
const uuidV4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
assert.match(requestId(), uuidV4);
const uuidDescriptor = Object.getOwnPropertyDescriptor(crypto, 'randomUUID');
const randomDescriptor = Object.getOwnPropertyDescriptor(crypto, 'getRandomValues');
try {
  Object.defineProperty(crypto, 'randomUUID', { configurable: true, value: undefined });
  Object.defineProperty(crypto, 'getRandomValues', { configurable: true, value: (bytes: Uint8Array) => bytes.fill(255) });
  assert.equal(requestId(), 'ffffffff-ffff-4fff-bfff-ffffffffffff', 'LAN fallback sets UUID version and variant bits');
} finally {
  if (uuidDescriptor) Object.defineProperty(crypto, 'randomUUID', uuidDescriptor); else Reflect.deleteProperty(crypto, 'randomUUID');
  if (randomDescriptor) Object.defineProperty(crypto, 'getRandomValues', randomDescriptor); else Reflect.deleteProperty(crypto, 'getRandomValues');
}
const originalFetch = globalThis.fetch;
const calls: { path: string; options?: RequestInit }[] = [];
const reply = (body: unknown) => new Response(JSON.stringify(body), { status: 200, headers: { 'content-type': 'application/json' } });
const mockFetch = (handler: (path: string, options?: RequestInit) => Response | Promise<Response>) => {
  calls.length = 0;
  globalThis.fetch = (async (path: string | URL | Request, options?: RequestInit) => { calls.push({ path: String(path), options }); return handler(String(path), options); }) as typeof fetch;
};
const memory = new Map<string, string>();
const storage = { getItem: (key: string) => memory.get(key) ?? null, setItem: (key: string, value: string) => { memory.set(key, value); } };
try {
  for (const action of ['messages', 'stop', 'resume', 'transcript'] as const) {
    let persisted: CommandEntry | undefined;
    mockFetch((path, options) => {
      if (path === '/api/v1/management') return reply(dataset);
      assert.equal(path, `/api/v1/agents/namesake/${action}`);
      const body = JSON.parse(String(options?.body));
      assert.match(body.request_id, uuidV4, 'submitted management request obeys native UUID admission');
      assert.equal(body.request_id, persisted?.requestId, 'UUID is persisted unchanged before POST');
      return reply({ request_id: body.request_id, generation: 'g1', status: 'Accepted', result: null, error: null });
    });
    const submitted = await runCommand(dataset, current, action, action === 'messages' ? 'test-only' : undefined, entry => { persisted = entry; });
    assert.equal(submitted.operation?.status, 'Accepted');
  }
  mockFetch(path => reply({ generation: 'g1', agents: [], next: new URL(path, 'http://test').searchParams.has('after') ? null : 'cursor' }));
  assert.deepEqual(await directory('g1'), { agents: [], complete: true, pages: 2 });
  mockFetch(() => reply({ generation: 'g2', agents: [], next: null }));
  await assert.rejects(directory('g1'), /代次/);
  mockFetch(() => reply({ generation: 'g1', agents: [], next: 'loop' }));
  await assert.rejects(directory('g1'), /游标重复/);
  mockFetch(() => reply({ generation: 'g1', agents: [], next: `cursor-${calls.length}` }));
  assert.equal((await directory('g1')).complete, false);
  assert.equal(calls.length, 20);
  mockFetch(() => reply({ generation: 'g2', messages: [], next_after: '0', has_more: false }));
  await assert.rejects(messages('g1', current), /代次/);
  const journal: CommandEntry[] = [];
  mockFetch((path, options) => {
    if (path === '/api/v1/management') return reply(dataset);
    assert.equal(options?.method, 'POST');
    assert.equal(journal.length, 1, 'journal precedes external effect');
    throw new Error('reply lost after possible acceptance');
  });
  const uncertain = await runCommand(dataset, current, 'messages', 'test-only', entry => { journal.push(entry); saveCommands(dataset, [entry], storage); });
  assert.equal(uncertain.operation, null);
  assert.match(uncertain.error ?? '', /结果未知/);
  assert.equal(blocksTarget(uncertain, current), true);
  assert.equal(blocksTarget(uncertain, later), false);
  assert.equal(loadCommands(dataset, storage)[0].requestId, uncertain.requestId);
  assert.deepEqual(loadCommands({ ...dataset, generation: 'g2' }, storage), []);
  assert.equal(calls.filter(call => call.options?.method === 'POST').length, 1);
  const operation: Operation = { request_id: uncertain.requestId, generation: 'g1', status: 'Unknown', result: null, error: null };
  mockFetch(() => reply(operation));
  assert.equal(blocksTarget(await reconcile(dataset, uncertain), current), true);
  assert.equal(calls[0].options?.method, undefined, 'reconciliation is GET only');
  mockFetch(() => reply({ ...operation, generation: 'g2' }));
  await assert.rejects(reconcile(dataset, uncertain), /身份不匹配/);
  mockFetch(path => path === '/api/v1/management' ? reply(dataset) : new Response(JSON.stringify({ error: { code: 'conflict', message: 'explicit rejection' } }), { status: 409 }));
  const refused = await runCommand(dataset, current, 'stop', undefined, () => {});
  assert.equal(refused.refused, true);
  assert.equal(blocksTarget(refused, current), false, 'explicit native rejection is distinct from lost/unknown result');
  mockFetch(() => reply(dataset));
  await assert.rejects(runCommand(dataset, current, 'stop', undefined, () => { throw new Error('storage denied'); }), /storage denied/);
  assert.equal(calls.length, 1, 'failed journal never POSTs');
  mockFetch(() => reply({ ...dataset, generation: 'g2' }));
  await assert.rejects(runCommand(dataset, current, 'stop', undefined, () => {}), /数据集已改变/);
  assert.equal(calls.length, 1);
  const usage: Usage = { input: '18446744073709551615', output: '1', cache_read: '0', cache_write: '0', reasoning: null, context_tokens: null, context_window: null, context_usage_ratio: null, model: null, unit: 'tokens', basis: 'parser_normalized_session_cumulative' };
  const coverage = { observed_at: 101, retained_from: 100, discovery_complete: false, dropped_observations: 0, gaps: [], basis: 'sampled' };
  const sample = { agent: current, session_id: 's', source_key: 'opaque', usage, availability: 'available' };
  const old = { id: 'o1', at: 100, agent: current, session_id: 's', status: 'active', model_request: null, sample };
  const missing = { ...old, id: 'o2', at: 101, sample: { ...sample, usage: null, availability: 'missing' } };
  const snapshot: Snapshot = { generation: 'g1', settings: { enabled: true, collected_from: 100, revision: 1, recovery: { enabled: false, error_duration_seconds: 600 } }, coverage, observations: [old, missing], jobs: [], attempts: [], dropped_attempts: 0, observation_revision: 2 };
  assert.equal(latestObservations(snapshot).length, 1);
  assert.equal(latestObservations(snapshot)[0].sample?.usage, null, 'latest missing must not reuse previous known sample');
  assert.equal(unknown(null), '未知');
  assert.equal(unknown(usage.input), '18446744073709551615');
  mockFetch(() => reply(snapshot));
  await inspection('g1');
  assert.equal(calls[0].path, '/api/v1/inspection?generation=g1');
  mockFetch(() => reply({ ...snapshot, generation: 'g2' }));
  await assert.rejects(inspection('g1'), /代次/);
  mockFetch(() => reply({}));
  await configure({ Configure: { generation: 'g1', expected_revision: 1, enabled: false, recovery: { enabled: false, error_duration_seconds: 600 } } });
  await analyze({ Analyze: { generation: 'g1', request_id: 'analysis-test', observation_ids: ['o1'] } });
  await cancelAnalysis({ CancelAnalysis: { generation: 'g1', id: 'analysis-test' } });
  await continuation({ RequestContinuation: { generation: 'g1', episode: { agent: current, session_id: 's', source: 'model_request', failing_turn: 't', error_since: 100 } } });
  assert.deepEqual(calls.map(call => call.path), ['/api/v1/inspection/configure', '/api/v1/inspection/analyze', '/api/v1/inspection/cancel-analysis', '/api/v1/inspection/continuation']);
  for (const call of calls) assert.equal(call.options?.method, 'POST');
  mockFetch(() => { throw new Error('offline'); });
  await assert.rejects(inspection('g1'), /offline/);
  assert.equal(calls.length, 1, 'network error never invokes fixtures or another origin');
  console.log('live adapter scenarios passed');
} finally { globalThis.fetch = originalFetch; }
