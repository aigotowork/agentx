/** Focused implementation cases; combined native/browser acceptance lives in tests/web_context. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import './source-imports.mjs';
const { SpaceController } = await import('../src/offline/controller.ts');
const { installContext, exactTarget } = await import('../src/live/context.ts');
const agent = { id: 'exact-a', name: 'toku', membership: { kind: 'global' }, ref: { scope: 's', generation: 'g', incarnation: '123', stopped_event: null } };
const supply = () => ({ scope: { id: 's', generation: 'g' }, agents: [agent], posts: [], workspaces: [] });
const target = { scope: 's', generation: 'g', agent: { name: 'toku', incarnation: '123', stopped_event: null } };
const attachment = { id: 'file-a', name: 'picture.png', media_type: 'image/png', size: 1, url: '/api/v1/attachments/file-a?generation=g' };
test('failed batch keeps previous draft and refs; late success is exact-bound', async () => {
  const controller = new SpaceController(supply()); controller.writeDraft(agent.id, 'original');
  await assert.rejects(controller.addContext(agent.id, 'context', [new Blob(['a'])], async () => { throw new Error('disk full'); }, 65536), /disk full/);
  assert.equal(controller.getSnapshot().drafts[agent.id], 'original');
  assert.deepEqual(controller.getSnapshot().attachments[agent.id] ?? [], []);
  let release;
  const pending = controller.addContext(agent.id, 'context', [new Blob(['a'])], () => new Promise(resolve => { release = resolve; }), 65536);
  controller.replaceSupply({ ...supply(), agents: [{ ...agent, id: 'successor', ref: { ...agent.ref, incarnation: '456' } }] });
  release([attachment]); await assert.rejects(pending, /失效/);
  assert.equal(controller.getSnapshot().drafts.successor, undefined);
  assert.equal(controller.getSnapshot().uploading[agent.id], false);
});
test('parent readiness/results deduplicate pending requests independently of routes', async () => {
  const controller = new SpaceController(supply());
  const sent = [], listeners = new Map();
  const parent = { postMessage: value => sent.push(value) };
  globalThis.window = { parent, addEventListener: (type, fn) => listeners.set(type, fn), removeEventListener: type => listeners.delete(type) };
  globalThis.location = { search: '?scope=s&generation=g&agent=toku&incarnation=123&layout=sidebar' };
  let uploads = 0, release;
  const live = { capabilities: () => ({ can_send: true }), blocked: () => false, descriptor: { max_text: 65536 }, upload: () => { uploads++; return new Promise(resolve => { release = resolve; }); } };
  controller.openExact(agent.id);
  const cleanup = installContext(controller, live, () => {});
  assert.equal(sent[0].type, 'ready'); assert.equal(uploads, 0); assert.equal(controller.getSnapshot().selectedId, agent.id);
  const receive = listeners.get('message');
  const payload = { protocol: 'agentx.context.v1', type: 'addContext', requestId: 'one', target, context: { text: 'page', files: [new Blob(['a'])] } };
  receive({ source: {}, origin: 'http://parent', data: payload }); assert.equal(uploads, 0);
  receive({ source: parent, origin: 'http://parent', data: { protocol: payload.protocol, type: 'hello', requestId: 'hello' } });
  assert.equal(sent.at(-1).requestId, 'hello'); assert.equal(sent.at(-1).type, 'ready');
  receive({ source: parent, origin: 'http://parent', data: payload });
  receive({ source: parent, origin: 'http://parent', data: payload }); assert.equal(uploads, 1);
  release([attachment]); await new Promise(resolve => setImmediate(resolve));
  assert.equal(controller.getSnapshot().drafts[agent.id], 'page');
  assert.equal(controller.getSnapshot().attachments[agent.id].length, 1);
  assert.equal(sent.filter(value => value.requestId === 'one' && value.status === 'added').length, 2);
  receive({ source: parent, origin: 'http://parent', data: payload }); await new Promise(resolve => setImmediate(resolve)); assert.equal(uploads, 1);
  assert.throws(() => exactTarget(controller, { ...target, agent: { ...target.agent, incarnation: '999' } }), /精确目标/);
  cleanup(); assert.equal(listeners.size, 0);

});

test('selected capture snapshots fence departure, away/back, capability loss and upload commit', async () => {
  const other = { ...agent, id: 'exact-b', name: 'navi', ref: { ...agent.ref, incarnation: '456' } };
  const controller = new SpaceController({ ...supply(), agents: [agent, other] });
  const sent = [], listeners = new Map(), parent = { postMessage: value => sent.push(value) };
  globalThis.window = { parent, addEventListener: (type, fn) => listeners.set(type, fn), removeEventListener: type => listeners.delete(type) };
  globalThis.location = { search: '' };
  let release, uploads = 0, available = true;
  const live = { capabilities: () => ({ can_send: available }), blocked: () => false, descriptor: { max_text: 65536 }, upload: () => { uploads++; return new Promise(resolve => { release = resolve; }); } };
  const cleanup = installContext(controller, live, () => {});
  const receive = listeners.get('message');
  const request = (type, id, body = {}) => receive({ source: parent, origin: 'http://parent', data: { protocol: 'agentx.context.v1', type, requestId: id, ...body } });
  request('getTarget', 'empty'); assert.equal(sent.at(-1).type, 'target'); assert.equal(sent.at(-1).status, 'unavailable'); assert.equal(uploads, 0);
  controller.select(agent.id);
  request('getTarget', 'selected'); const capture = sent.at(-1);
  assert.deepEqual(capture.target, target); assert.equal(capture.status, 'selected');
  controller.presentation('chat'); request('getTarget', 'mode'); assert.equal(sent.at(-1).selectionRevision, capture.selectionRevision);
  controller.close(); controller.select(agent.id);
  request('addContext', 'stale', { target, selectionRevision: capture.selectionRevision, context: { text: 'stale' } });
  await new Promise(resolve => setImmediate(resolve)); assert.equal(sent.at(-1).status, 'error'); assert.equal(uploads, 0);
  request('getTarget', 'fresh'); const current = sent.at(-1);
  request('addContext', 'race', { target, selectionRevision: current.selectionRevision, context: { text: 'must not commit', files: [new Blob(['a'])] } });
  assert.equal(uploads, 1); controller.select(other.id); controller.select(agent.id);
  release([attachment]); await new Promise(resolve => setImmediate(resolve));
  assert.equal(sent.at(-1).status, 'error'); assert.equal(controller.getSnapshot().drafts[agent.id], undefined); assert.equal(controller.getSnapshot().attachments[agent.id], undefined);
  request('getTarget', 'before-loss'); const beforeLoss = sent.at(-1);
  request('addContext', 'loss', { target, selectionRevision: beforeLoss.selectionRevision, context: { text: 'blocked', files: [new Blob(['b'])] } });
  available = false; release([attachment]); await new Promise(resolve => setImmediate(resolve));
  assert.equal(sent.at(-1).status, 'error'); assert.equal(controller.getSnapshot().drafts[agent.id], undefined);
  available = true; request('getTarget', 'success-target'); const success = sent.at(-1);
  request('addContext', 'success', { target, selectionRevision: success.selectionRevision, context: { text: 'accepted' } });
  release([]); await new Promise(resolve => setImmediate(resolve));
  assert.equal(sent.at(-1).status, 'added'); assert.deepEqual(sent.at(-1).target, target); assert.equal(sent.at(-1).selectionRevision, success.selectionRevision);
  assert.equal(controller.getSnapshot().drafts[agent.id], 'accepted');
  cleanup();
});

test('successful-send cleanup clears only the exact sent revision and attachment IDs', async () => {
  const second = { ...agent, id: 'exact-b', name: 'navi', ref: { ...agent.ref, incarnation: '456' } };
  const controller = new SpaceController({ ...supply(), agents: [agent, second] });
  controller.writeDraft(agent.id, 'sent draft');
  await controller.addContext(agent.id, '', [new Blob(['a'])], async () => [attachment], 65536);
  const revision = controller.draftRevision(agent.id);
  controller.writeDraft(second.id, 'other draft'); controller.select(second.id);
  controller.clearSentDraft(agent.id, 'sent draft', [attachment.id], revision);
  assert.equal(controller.getSnapshot().drafts[agent.id], '');
  assert.deepEqual(controller.getSnapshot().attachments[agent.id], []);
  assert.equal(controller.getSnapshot().drafts[second.id], 'other draft');
  assert.equal(controller.getSnapshot().selectedId, second.id);
});

test('receipt completion preserves new edits, including edit-away/back and newly attached files', async () => {
  const controller = new SpaceController(supply()); controller.writeDraft(agent.id, 'original');
  await controller.addContext(agent.id, '', [new Blob(['a'])], async () => [attachment], 65536);
  const revision = controller.draftRevision(agent.id);
  controller.writeDraft(agent.id, 'edited'); controller.writeDraft(agent.id, 'original');
  const newer = { ...attachment, id: 'new-file' };
  await controller.addContext(agent.id, '', [new Blob(['b'])], async () => [newer], 65536);
  controller.clearSentDraft(agent.id, 'original', [attachment.id], revision);
  assert.equal(controller.getSnapshot().drafts[agent.id], 'original');
  assert.deepEqual(controller.getSnapshot().attachments[agent.id].map(item => item.id), ['new-file']);
});

test('attachment-only success and a stale exact-instance completion cannot clear a successor', async () => {
  const controller = new SpaceController(supply());
  await controller.addContext(agent.id, '', [new Blob(['a'])], async () => [attachment], 65536);
  controller.clearSentDraft(agent.id, '', [attachment.id], controller.draftRevision(agent.id));
  assert.deepEqual(controller.getSnapshot().attachments[agent.id], []);
  controller.writeDraft(agent.id, 'old'); const revision = controller.draftRevision(agent.id);
  const successor = { ...agent, id: 'successor', ref: { ...agent.ref, incarnation: '456' } };
  controller.replaceSupply({ ...supply(), agents: [successor] }); controller.writeDraft(successor.id, 'new');
  controller.clearSentDraft(agent.id, 'old', [], revision);
  assert.equal(controller.getSnapshot().drafts.successor, 'new');
});
