import test from 'node:test';
import assert from 'node:assert/strict';

const listeners = new Map();
globalThis.window = { addEventListener: (type, handler) => listeners.set(type, handler), removeEventListener: type => listeners.delete(type) };
const { ContextBridge } = await import('./public/bridge.js');
const origin = 'http://fixture.invalid:18764';
const target = { scope: '/fixture', generation: 'g1', agent: { name: 'same', incarnation: '18446744073709551615', stopped_event: null } };
function fixture() {
  const sent = [];
  const events = new Map();
  const frame = { addEventListener: (type, handler) => events.set(type, handler), removeEventListener: type => events.delete(type), contentWindow: { postMessage: (value, toOrigin) => { sent.push({ value, toOrigin }); } } };
  const states = [];
  const bridge = new ContextBridge(frame, origin, (phase, text) => states.push({ phase, text }));
  const reply = data => bridge.receive({ source: frame.contentWindow, origin, data: { protocol: 'agentx.context.v1', ...data } });
  bridge.connect();
  assert.equal(frame.src, `${origin}/space/`);
  events.get('load')();
  reply({ type: 'ready', requestId: sent.at(-1).value.requestId });
  return { bridge, frame, sent, reply, states };
}

test('selected-context exchange preserves exact string target and opaque revision, with no query fallback', async () => {
  const f = fixture();
  try {
    const capture = f.bridge.captureTarget();
    const read = f.sent.at(-1).value;
    assert.equal(read.type, 'getTarget');
    f.bridge.receive({ source: {}, origin, data: { protocol: read.protocol, type: 'target', requestId: read.requestId, status: 'selected', target, selectionRevision: 'wrong' } });
    assert.equal(f.bridge.pending.size, 1, 'wrong source cannot resolve');
    f.reply({ type: 'target', requestId: read.requestId, status: 'selected', target, selectionRevision: 'opaque-r1' });
    const binding = await capture;
    assert.equal(binding.target.agent.incarnation, '18446744073709551615');
    const context = { title: '演示订单', text: '补充信息', url: 'http://parent.invalid/' };
    const add = f.bridge.add(context, binding);
    const request = f.sent.at(-1).value;
    assert.deepEqual(request.target, target);
    assert.equal(request.selectionRevision, 'opaque-r1');
    assert.deepEqual(request.context, context);
    f.reply({ type: 'result', requestId: request.requestId, status: 'added', target, selectionRevision: 'opaque-r1' });
    assert.equal((await add).target.agent.name, 'same');
    assert(f.sent.every(item => item.toOrigin === origin));
    assert.equal(f.frame.src, `${origin}/space/`);
  } finally { f.bridge.dispose(); }
});

test('unavailable and selection-change errors never retarget/retry or accept a mismatched result', async () => {
  const f = fixture();
  try {
    const capture = f.bridge.captureTarget();
    f.reply({ type: 'target', requestId: f.sent.at(-1).value.requestId, status: 'unavailable', error: '未选择成员' });
    await assert.rejects(capture, /未选择成员/);
    const binding = { target, selectionRevision: 'r1' };
    const add = f.bridge.add({ text: '保留捕获' }, binding);
    f.reply({ type: 'result', requestId: f.sent.at(-1).value.requestId, status: 'error', error: 'selection changed' });
    await assert.rejects(add, /selection changed/);
    assert.equal(f.sent.filter(item => item.value.type === 'addContext').length, 1, 'no automatic retry');
    const mismatched = f.bridge.add({ text: '保留捕获' }, binding);
    f.reply({ type: 'result', requestId: f.sent.at(-1).value.requestId, status: 'added', target: { ...target, agent: { ...target.agent, incarnation: '2' } }, selectionRevision: 'r1' });
    await assert.rejects(mismatched, /身份未确认/);
  } finally { f.bridge.dispose(); }
});
