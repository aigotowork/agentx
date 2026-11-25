/** Focused consumer race/continuity cases; native HTTP/browser composition is independently owned. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { selectAgentXScope } from '@agentx/client';
import './source-imports.mjs';
const { LiveSpace } = await import('../src/live/controller.ts');
const { SpaceController } = await import('../src/offline/controller.ts');
const { DomainOfficeReads } = await import('../src/live/domain-office.ts');
const { exactAgentKey } = await import('../src/offline/model.ts');
const listener = 'http://disposable.invalid';
const descriptor = { scope: 'scope-a', generation: 'generation-a', project: '/work', tools: [], profiles: [], supports_workspace_members: true, supports_message_history: true, max_page: 100, max_text: 65536 };
const row = { agent: { name: 'mavu', incarnation: '2', stopped_event: null }, title: null, file_bindings: [], pending_messages: 0, tool: 'qoder', status: 'active', stored_status: 'active', status_context: '', request_error: null, detail: '', directory: '/office-b', session_id: 'fixture', collaboration: { status: 'adopted', workspace: '/office-b', role: null, reason: null }, can_send: true, can_stop: true, can_resume: false, has_transcript: false };
const { agent: { name, incarnation }, collaboration, ...facts } = row;
const target = { creation: { name, incarnation }, availability: 'available', launchFailure: null, agent: {
  address: { scope: { baseUrl: listener, scope: descriptor.scope, generation: descriptor.generation }, reference: { kind: 'current', name, incarnation } },
  membership: { kind: 'adopted', workspace: collaboration.workspace, role: null }, facts,
} };
const bind = async live => {
  const scope = await selectAgentXScope({ baseUrl: listener, expectedScope: descriptor.scope,
    fetch: request => new URL(request.url).pathname === '/api/v1/management' ? Response.json(descriptor) : globalThis.fetch(request) });
  live.descriptor = scope.descriptor;
  live.domain = new DomainOfficeReads(scope, 20, () => {});
  live.connection = 'pinned';
  return scope;
};

test('stale rendered relation cannot borrow a newer scope/generation even with identical target tuple', async t => {
  let requests = 0;
  t.mock.method(globalThis, 'fetch', async () => { requests++; throw new Error('unexpected dispatch'); });
  const live = new LiveSpace(() => {});
  live.descriptor = { ...descriptor, generation: 'generation-b' };
  await assert.rejects(live.openRelated(target, { scope: descriptor.scope, generation: descriptor.generation }), /scope \/ generation/);
  live.descriptor = { ...descriptor, scope: 'scope-b' };
  await assert.rejects(live.openRelated(target, { scope: descriptor.scope, generation: descriptor.generation }), /scope \/ generation/);
  assert.equal(requests, 0);
});

test('generation replacement and deliberate selection both refuse a late relation response', async t => {
  let publish = 0, release, reachedFetch;
  const started = () => new Promise(resolve => { reachedFetch = resolve; });
  t.mock.method(globalThis, 'fetch', () => new Promise(resolve => { release = () => resolve(Response.json(row)); reachedFetch(); }));
  const live = new LiveSpace(() => { publish++; }); await bind(live);
  let reached = started();
  const first = live.openRelated(target, { scope: descriptor.scope, generation: descriptor.generation });
  await reached; live.descriptor = { ...descriptor, generation: 'replacement' }; release(); await assert.rejects(first, /导航已失效/);
  live.descriptor = descriptor;
  reached = started();
  const second = live.openRelated(target, { scope: descriptor.scope, generation: descriptor.generation });
  const pendingId = exactAgentKey({ scope: descriptor.scope, generation: descriptor.generation, name: 'another', incarnation: '3', record: 'current' });
  await reached; live.select(pendingId); release(); await assert.rejects(second, /导航已失效/);
  assert.equal(publish, 0);
  assert.equal(live.domain.subjects.size, 0);
});

test('exact cross-office navigation preserves both drafts and chooses stopped history deliberately', () => {
  const a = { id: 'exact-a', membership: { kind: 'directory', path: '/office-a' }, ref: { record: 'current' } };
  const b = { id: 'exact-b', membership: { kind: 'directory', path: '/office-b' }, ref: { record: 'stopped' } };
  const page = new SpaceController({ scope: { id: 's', generation: 'g' }, agents: [a,b], posts: [], workspaces: [] });
  page.writeDraft(a.id, 'draft A'); page.writeDraft(b.id, 'draft B');
  page.openExact(b.id);
  assert.equal(page.getSnapshot().workspaceId, '/office-b'); assert.equal(page.getSnapshot().officeView, 'history');
  page.openExact(a.id);
  assert.equal(page.getSnapshot().workspaceId, '/office-a'); assert.equal(page.getSnapshot().selectedId, a.id);
  assert.deepEqual(page.getSnapshot().drafts, { 'exact-a': 'draft A', 'exact-b': 'draft B' });
  page.openExact('missing'); assert.equal(page.getSnapshot().selectedId, a.id);
});

test('supported fork dispatches an exact prepared target and unsupported fork never submits', async t => {
  const live = new LiveSpace(() => {}); const scope = await bind(live);
  const exact = { address: { scope: scope.address, reference: { kind: 'current', name: row.agent.name, incarnation: row.agent.incarnation } },
    membership: { kind: 'adopted', workspace: '/office-b', role: null }, facts: { ...row, can_fork: true } };
  const id = exactAgentKey({ scope: descriptor.scope, generation: descriptor.generation, ...row.agent, record: 'current' });
  live.overview.set(id, exact);
  const originalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  let persisted;
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: { setItem: (_key, value) => { persisted = JSON.parse(value); } } });
  t.after(() => { if (originalStorage) Object.defineProperty(globalThis, 'localStorage', originalStorage); else Reflect.deleteProperty(globalThis, 'localStorage'); });
  const calls = [];
  t.mock.method(globalThis, 'fetch', async (request) => {
    const body = await request.json(), path = new URL(request.url).pathname;
    assert.equal(persisted[0].id, body.request_id);
    calls.push({ path, body });
    return Response.json({ request_id: body.request_id, generation: descriptor.generation, status: 'Succeeded', error: null,
      result: { Launched: { agents: ['forked'], failed: 0, errors: [] } } });
  });
  assert.equal(await live.target('fork', id), true);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].path, '/api/v1/agents/mavu/fork');
  assert.equal(calls[0].body.agent.name, name);
  assert.equal(calls[0].body.agent.incarnation, incarnation);
  assert.equal(calls[0].body.generation, descriptor.generation);
  assert.match(calls[0].body.request_id, /^[0-9a-f-]{36}$/i);
  live.overview.set(id, { ...exact, facts: { ...exact.facts, can_fork: false } });
  await assert.rejects(live.target('fork', id), /原生能力不可用/);
  assert.equal(calls.length, 1);
});
