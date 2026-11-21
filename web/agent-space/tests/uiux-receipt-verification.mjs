/** Actual LiveSpace receipt adapter checks over the public SDK with a disposable transport. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseManagementJson } from '@agentx/client';
import './source-imports.mjs';
const { LiveSpace } = await import('../src/live/controller.ts');
const { SpaceController } = await import('../src/offline/controller.ts');
const { emptySupply } = await import('../src/live/projection.ts');

const listener = 'http://disposable.invalid';
const descriptor = { scope: 'isolated', generation: 'g', project: '/office', tools: [], profiles: [], max_page: 100, max_text: 65536 };
const { scope, generation } = descriptor;
const ref = { name: 'same', incarnation: '1', stopped_event: null };
const archivedRef = { name: ref.name, incarnation: null, stopped_event: '20' };
const row = { agent: ref, collaboration: { status: 'adopted', workspace: null, role: null }, directory: '/office',
  tool: 'test', status: 'listening', stored_status: 'listening', status_context: '', detail: '',
  can_send: true, can_stop: false, can_resume: false, has_transcript: false };
const archived = { ...row, agent: archivedRef, status: 'stopped', stored_status: 'stopped', can_send: false };

async function fixture(t, status) {
  const storage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const document = Object.getOwnPropertyDescriptor(globalThis, 'document');
  let journal, reportedScope = scope, mismatchedReceipt = false;
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: { getItem: () => null, setItem: (key, value) => { journal = value; } } });
  Object.defineProperty(globalThis, 'document', { configurable: true, value: Object.assign(new EventTarget(), { hidden: true }) });
  t.after(() => {
    if (storage) Object.defineProperty(globalThis, 'localStorage', storage);
    else Reflect.deleteProperty(globalThis, 'localStorage');
    if (document) Object.defineProperty(globalThis, 'document', document);
    else Reflect.deleteProperty(globalThis, 'document');
  });
  const state = new SpaceController(emptySupply());
  const posts = [], lookups = [];
  const receipt = request_id => Response.json({
    request_id, generation, status, error: status === 'Failed' ? 'simulated failure' : null,
    result: status === 'Succeeded' ? { Sent: { message_id: '7', recipients: [ref.name] } } : null,
  });
  t.mock.method(globalThis, 'fetch', async request => {
    const url = new URL(request.url);
    assert.equal(url.origin, listener);
    if (request.method === 'POST') {
      assert.equal(url.pathname, '/api/v1/agents/same/messages');
      const body = parseManagementJson('SubmitMessage', await request.text());
      assert.equal(body.generation, generation);
      assert.equal(body.agent.name, ref.name);
      assert.equal(body.agent.incarnation, ref.incarnation);
      assert.equal(body.agent.stopped_event ?? null, null);
      assert(journal.includes(body.request_id));
      posts.push(body.request_id);
      return receipt(body.request_id);
    }
    assert.equal(request.method, 'GET');
    switch (url.pathname) {
      case '/api/v1/management': return Response.json({ ...descriptor, scope: reportedScope });
      case '/api/v1/workspaces': return Response.json({ generation, workspaces: [], coverage: { errors: [], agent_limit_reached: false, scan_limit_reached: false, agent_pages: 1, directories_examined: 0 } });
      case '/api/v1/agents': return Response.json({ generation, agents: [row, archived], next: null, current_complete: true });
      case '/api/v1/agents/same':
        assert.equal(url.searchParams.get('incarnation'), ref.incarnation);
        assert.equal(url.searchParams.get('stopped_event'), null);
        return Response.json(row);
      case '/api/v1/agents/same/messages':
        assert.equal(url.searchParams.get('incarnation'), ref.incarnation);
        return Response.json({ generation, messages: [], next_after: 'checkpoint', has_more: false });
      case '/api/v1/roles': return Response.json({ generation, workspace: null, roles: [] });
      case '/api/v1/office-activity': return Response.json({ generation, next_after: '0', has_more: false, messages: [], created: [], human_messages: [], human_has_more: false, human_unhandled_count: 0 });
      default: {
        assert.match(url.pathname, /^\/api\/v1\/operations\//);
        const id = url.pathname.split('/').at(-1);
        assert.equal(id, posts[0], 'recovery queries the original request');
        assert.equal(url.searchParams.get('generation'), generation);
        lookups.push(id);
        return receipt(mismatchedReceipt ? 'other' : id);
      }
    }
  });
  const live = new LiveSpace(state.replaceSupply, { baseUrl: listener, expectedScope: scope });
  t.after(live.start());
  await live.refresh();
  assert.equal(live.connection, 'pinned');
  const agent = live.supply.agents.find(item => item.ref.record === 'current'), namesake = live.supply.agents.find(item => item.ref.record === 'stopped');
  assert.equal(agent.name, namesake.name);
  assert.notEqual(agent.id, namesake.id);
  state.writeDraft(agent.id, 'sent');
  live.select(agent.id);
  const deadline = Date.now() + 1000;
  while (live.messagePage(agent.id)?.phase === 'loading' && Date.now() < deadline) await new Promise(resolve => setImmediate(resolve));
  assert.equal(live.messagePage(agent.id)?.phase, 'ready', live.messagePage(agent.id)?.error);
  return { live, state, agent, namesake, posts, lookups, setStatus: value => { status = value; },
    mismatchReceipt: value => { mismatchedReceipt = value; }, replaceScope: () => { reportedScope = 'other'; } };
}

for (const status of ['Succeeded', 'Accepted', 'Running', 'Unknown', 'Failed']) {
  test(`actual adapter ${status}: Succeeded-only cleanup and no resubmit`, async t => {
    const { live, state, agent: { id }, posts, lookups, setStatus } = await fixture(t, status);
    const revision = state.draftRevision(id);
    const accepted = await live.target('send', id, { text: 'sent', attachments: [] }, () => state.clearSentDraft(id, 'sent', [], revision));
    assert.equal(accepted, status === 'Succeeded');
    assert.equal(live.receipts[0].operation?.status, status);
    assert.equal(state.getSnapshot().drafts[id], status === 'Succeeded' ? '' : 'sent');
    assert.equal(posts.length, 1);
    if (['Accepted', 'Running', 'Unknown'].includes(status)) {
      await assert.rejects(live.target('send', id, { text: 'sent' }), /不可用|已失效/);
      assert.equal(posts.length, 1);
      live.select(null);
      setStatus('Succeeded');
      await live.lookup(live.receipts[0]);
      assert.equal(state.getSnapshot().drafts[id], '');
      assert.equal(posts.length, 1);
      assert.deepEqual(lookups, posts);
    }
  });
}

test('delayed lookup after reader departure preserves edit-away/back, newer files and other exact namesake', async t => {
  const { live, state, agent: { id }, namesake, posts, setStatus } = await fixture(t, 'Accepted');
  await state.addContext(id, '', [new Blob(['a'])], async () => [{ id: 'sent-file' }], 65536);
  await state.addContext(namesake.id, 'history draft', [new Blob(['c'])], async () => [{ id: 'namesake-file' }], 65536);
  const revision = state.draftRevision(id);
  await live.target('send', id, { text: 'sent', attachments: ['sent-file'] }, () => state.clearSentDraft(id, 'sent', ['sent-file'], revision));
  live.select(null);
  state.writeDraft(id, 'new');
  state.writeDraft(id, 'sent');
  await state.addContext(id, '', [new Blob(['b'])], async () => [{ id: 'new-file' }], 65536);
  setStatus('Succeeded');
  await live.lookup(live.receipts[0]);
  assert.equal(live.receipts[0].operation?.status, 'Succeeded');
  assert.equal(state.getSnapshot().drafts[id], 'sent');
  assert.deepEqual(state.getSnapshot().attachments[id].map(item => item.id), ['new-file']);
  assert.equal(state.getSnapshot().drafts[namesake.id], 'history draft');
  assert.deepEqual(state.getSnapshot().attachments[namesake.id].map(item => item.id), ['namesake-file']);
  assert.equal(posts.length, 1);
});

test('scope change and mismatching receipt identity never complete original draft', async t => {
  const { live, state, agent: { id }, posts, lookups, setStatus, mismatchReceipt, replaceScope } = await fixture(t, 'Accepted');
  let completed = 0;
  await live.target('send', id, { text: 'sent' }, () => completed++);
  setStatus('Succeeded');
  mismatchReceipt(true);
  await live.lookup(live.receipts[0]);
  assert.match(live.receipts[0].issue, /does not match the retained request/);
  assert.equal(completed, 0);
  mismatchReceipt(false);
  replaceScope();
  await live.refresh();
  assert.equal(live.connection, 'reset');
  await live.lookup(live.receipts[0]);
  assert.equal(completed, 0);
  assert.equal(state.getSnapshot().drafts[id], 'sent');
  assert.equal(posts.length, 1);
  assert.deepEqual(lookups, posts);
});
