/** Deterministic scene lifecycle/frame/effect scenarios; no browser or native runtime mutation. */
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { existsSync } from 'node:fs';
import { registerHooks } from 'node:module';
registerHooks({ resolve(specifier, context, next) {
  if (specifier.startsWith('.') && context.parentURL) {
    const exact = new URL(specifier, context.parentURL);
    if (!existsSync(exact) && existsSync(new URL(`${exact.href}.ts`))) return next(`${exact.href}.ts`, context);
  }
  return next(specifier, context);
} });
const { SceneRuntime, browserSceneEnvironment } = await import('../src/scene/runtime/runtime.ts');
const { ActorAnimation } = await import('../src/scene/runtime/actors.ts');
const { MessageEffects } = await import('../src/scene/runtime/messages.ts');
const { worldToScreen } = await import('../src/scene/geometry.ts');
const { HUMAN_ENDPOINT } = await import('../src/offline/model.ts');

function actor(key = 'scope:gen:name:inc1', status = 'active', arrival) {
  return { key, home: { x: 160, y: 160 }, rest: { x: 320, y: 160 }, resource: {
    id: key, name: 'name', nativeStatus: status, state: status === 'active' ? 'running' : 'idle', arrival,
  } };
}
function input(actors = [actor()], changes = {}) {
  return {
    scene: { width: 1200, height: 500, actors, rooms: [{ origin: { x: 0, y: 128 }, actors, office: { door: { x: 80, y: 240 } } }], human: { x: 20, y: 20 } },
    camera: { x: 13, y: 21, zoom: .75 }, viewport: { width: 1000, height: 700 },
    visibleKeys: new Set(actors.map(item => item.key)), paused: false, props: [], humanMessages: 0,
    ...changes,
  };
}
function environment() {
  let serial = 0;
  const pending = new Map(), listeners = new Set();
  return {
    pending, listeners, isHidden: false, reduced: false, now: 10000,
    requestFrame(callback) { const handle = ++serial; pending.set(handle, callback); return handle; },
    cancelFrame(handle) { pending.delete(handle); },
    wallNow() { return this.now; }, hidden() { return this.isHidden; }, reducedMotion() { return this.reduced; },
    subscribe(callback) { listeners.add(callback); return () => listeners.delete(callback); },
    signal(reason) { for (const callback of [...listeners]) callback(reason); },
    step(time) {
      assert.equal(pending.size, 1, 'one scene has exactly one scheduled frame');
      const [handle, callback] = pending.entries().next().value;
      pending.delete(handle); callback(time);
    },
  };
}
function flight(id, receivedAt = 10000, from = actor().key, to = HUMAN_ENDPOINT) { return { id, receivedAt, from, to }; }

test('one scheduler/subscription survives repeated wake; disposal is permanent and remount starts cleanly', () => {
  const env = environment(), frames = [], supplied = input();
  const runtime = new SceneRuntime(supplied, env, frame => frames.push(frame));
  runtime.start(); runtime.start(); runtime.update(supplied); runtime.update(supplied);
  assert.equal(env.pending.size, 1); assert.equal(env.listeners.size, 1);
  env.step(40); env.step(80);
  assert.deepEqual(frames.map(frame => frame.elapsed), [0, 40]);
  const staleTick = env.pending.values().next().value, staleSignal = [...env.listeners][0];
  runtime.dispose(); runtime.dispose(); runtime.start(); runtime.update(supplied);
  staleSignal('visibility'); staleTick(10000);
  assert.equal(frames.length, 2); assert.equal(env.pending.size, 0); assert.equal(env.listeners.size, 0);
  assert.equal(runtime.point(supplied.scene.actors[0].key), undefined);
  const remounted = new SceneRuntime(supplied, env, frame => frames.push(frame));
  remounted.start(); env.step(10040);
  assert.equal(frames.at(-1).elapsed, 0);
  remounted.dispose(); assert.equal(env.pending.size, 0); assert.equal(env.listeners.size, 0);
});

test('pause and reduced motion draw settled frames without continuation; resumed clock does not jump', () => {
  const env = environment(), frames = [], supplied = input();
  const runtime = new SceneRuntime(supplied, env, frame => frames.push(frame));
  runtime.start(); env.step(40); env.step(80);
  runtime.update({ ...supplied, paused: true, messages: [flight('paused')] }); env.step(120);
  assert.equal(frames.at(-1).still, true); assert.equal(frames.at(-1).elapsed, 40);
  assert.equal(frames.at(-1).actors[0].frame.pose, 'sit'); assert.equal(frames.at(-1).flights.length, 0);
  assert.equal(env.pending.size, 0);
  runtime.update(supplied); env.step(1000); env.step(1040);
  assert.equal(frames.at(-1).elapsed, 80);
  env.reduced = true; env.signal('motion'); env.step(1080);
  assert.equal(frames.at(-1).still, true); assert.equal(env.pending.size, 0);
  env.reduced = false; env.signal('motion'); env.step(2000); env.step(3000);
  assert.equal(frames.at(-1).elapsed, 180, 'long visible ticks retain the 100ms increment cap');
  runtime.dispose();
});

test('visibility restoration consumes hidden messages, clears flights and settles interrupted paths', () => {
  const env = environment(), frames = [], initial = input();
  const runtime = new SceneRuntime(initial, env, frame => frames.push(frame));
  runtime.start(); env.step(40);
  const moving = input([actor(undefined, 'listening')], { messages: [flight('visible')] });
  runtime.update(moving); env.step(80); env.step(120);
  assert.equal(frames.at(-1).actors[0].frame.pose, 'walk'); assert.equal(frames.at(-1).flights.length, 1);
  env.isHidden = true; env.signal('visibility'); env.step(160);
  runtime.update({ ...moving, messages: [flight('visible'), flight('hidden')] }); env.step(200);
  const before = frames.at(-1).elapsed;
  env.isHidden = false; env.signal('visibility'); env.step(10000);
  assert.equal(frames.at(-1).elapsed, before); assert.equal(frames.at(-1).flights.length, 0);
  assert.deepEqual([frames.at(-1).actors[0].frame.x, frames.at(-1).actors[0].frame.y], [320, 160]);
  runtime.dispose();
});

test('a presented moving actor shares one world/screen sample and exact-key point with selection/filtering', () => {
  const env = environment(), frames = [], initial = input();
  const runtime = new SceneRuntime(initial, env, frame => frames.push(frame));
  runtime.start(); env.step(40);
  runtime.update(input([actor(undefined, 'listening')], { visibleKeys: new Set() })); env.step(80); env.step(120);
  const frame = frames.at(-1), sample = frame.actors[0];
  assert.equal(sample.frame.pose, 'walk'); assert.equal(sample.dim, true); assert.equal(sample.hidden, true);
  assert.deepEqual(sample.screen, worldToScreen(sample.frame, frame.input.camera));
  assert.equal(runtime.point(sample.actor.key), sample.frame);
  const replacement = actor('scope:gen:name:inc2', 'listening');
  runtime.update(input([replacement])); env.step(160);
  assert.equal(runtime.point(sample.actor.key), undefined);
  assert.deepEqual([frames.at(-1).actors[0].frame.x, frames.at(-1).actors[0].frame.y], [320, 160]);
  runtime.dispose();
});

test('new visible arrival uses door once with a 1600ms cap; reflow and reappearance do not invent arrivals', () => {
  const animation = new ActorAnimation(input([])), arriving = actor(undefined, 'active', 'arrival-1');
  let supplied = input([arriving]);
  assert.deepEqual(animation.sample(supplied, 0, false)[0].frame, { x: 80, y: 240, pose: 'walk', facing: 'up' });
  assert.equal(animation.sample(supplied, 1599, false)[0].frame.pose, 'walk');
  assert.equal(animation.sample(supplied, 1600, false)[0].frame.pose, 'sit');
  const reflowed = { ...arriving, home: { x: 500, y: 400 } };
  supplied = input([reflowed]);
  assert.deepEqual([animation.sample(supplied, 1700, false)[0].frame.x, animation.point(arriving.key).y], [500, 400]);
  animation.sample(input([]), 1800, false);
  assert.equal(animation.sample(supplied, 1900, false)[0].frame.pose, 'sit');
  const alreadyPresent = new ActorAnimation(supplied);
  assert.equal(alreadyPresent.sample(supplied, 0, false)[0].frame.pose, 'sit', 'initial supplied arrivals are historical');
  const reduced = new ActorAnimation(input([]));
  assert.equal(reduced.sample(input([arriving]), 0, true)[0].frame.pose, 'sit');
  animation.dispose(); alreadyPresent.dispose(); reduced.dispose();
});

test('selected actor holds supplied point and releases toward the native behavior goal from that frame', () => {
  const person = actor(undefined, 'listening'), animation = new ActorAnimation(input([person]));
  const selected = { key: person.key, point: { x: 230, y: 200 } };
  const held = animation.sample(input([person], { selected }), 100, false)[0].frame;
  assert.deepEqual(held, { x: 230, y: 200, pose: 'idle', facing: 'down' });
  animation.sample(input([person], { selected }), 200, false);
  const released = animation.sample(input([person]), 200, false)[0].frame;
  assert.deepEqual([released.x, released.y], [230, 200]);
  const moving = animation.sample(input([person]), 300, false)[0].frame;
  assert.ok(moving.x > 230 && moving.x < 320); assert.equal(moving.pose, 'walk');
  animation.dispose();
});

test('messages are deduplicated, recent, exact-addressed and bounded to four six-letter batches', () => {
  const initial = input(), effects = new MessageEffects(initial);
  const messages = Array.from({ length: 30 }, (_, i) => flight(String(i)));
  const supplied = { ...initial, messages };
  assert.deepEqual(effects.sample(supplied, 0, 10000, false).map(item => item.event.id), ['0', '1', '2', '3', '4', '5']);
  assert.deepEqual(effects.sample(supplied, 700, 10700, false).map(item => item.event.id), ['6', '7', '8', '9', '10', '11']);
  assert.deepEqual(effects.sample(supplied, 1400, 11400, false).map(item => item.event.id), ['12', '13', '14', '15', '16', '17']);
  assert.deepEqual(effects.sample(supplied, 2100, 12100, false).map(item => item.event.id), ['18', '19', '20', '21', '22', '23']);
  assert.deepEqual(effects.sample(supplied, 2800, 12800, false), []);
  const rejected = { ...initial, messages: [flight('stale', 6000), flight('wrong-incarnation', 10000, 'scope:gen:name:inc2')] };
  assert.deepEqual(effects.sample(rejected, 2800, 10000, false), []);
  effects.dispose();
});

test('message geometry, pause and visibility invalidate effects without replaying seen history', () => {
  const initial = input(), historical = flight('initial');
  const effects = new MessageEffects({ ...initial, messages: [historical] });
  assert.deepEqual(effects.sample({ ...initial, messages: [historical] }, 0, 10000, false), []);
  const newMessage = flight('new');
  assert.equal(effects.sample({ ...initial, messages: [newMessage] }, 0, 10000, false).length, 1);
  const moved = input([{ ...actor(), home: { x: 161, y: 160 } }], { messages: [newMessage] });
  assert.deepEqual(effects.sample(moved, 10, 10000, false), []);
  effects.restore([flight('hidden')]);
  assert.deepEqual(effects.sample({ ...moved, messages: [flight('hidden')] }, 20, 10000, false), []);
  effects.sample({ ...moved, messages: [flight('paused')] }, 30, 10000, true);
  assert.deepEqual(effects.sample({ ...moved, messages: [flight('paused')] }, 40, 10000, false), []);
  assert.equal(effects.sample({ ...moved, messages: [flight('removed')] }, 50, 10000, false).length, 1);
  assert.deepEqual(effects.sample(input([]), 60, 10000, false), []);
  effects.dispose();
});

test('seen-message rollover keeps the current supplied history consumed', () => {
  const initial = input(), effects = new MessageEffects(initial);
  const messages = Array.from({ length: 300 }, (_, i) => flight(`history-${i}`, 0));
  effects.sample({ ...initial, messages }, 0, 10000, false);
  assert.deepEqual(effects.sample({ ...initial, messages: messages.map(item => ({ ...item, receivedAt: 10000 })) }, 10, 10000, false), []);
  effects.dispose();
});

test('browser environment removes the exact visibility/preference listeners it acquired', () => {
  const documentDescriptor = Object.getOwnPropertyDescriptor(globalThis, 'document');
  const mediaDescriptor = Object.getOwnPropertyDescriptor(globalThis, 'matchMedia');
  const mediaListeners = new Set(), documentListeners = new Set();
  const media = { matches: false, addEventListener: (_, fn) => mediaListeners.add(fn), removeEventListener: (_, fn) => mediaListeners.delete(fn) };
  Object.defineProperty(globalThis, 'matchMedia', { configurable: true, value: () => media });
  Object.defineProperty(globalThis, 'document', { configurable: true, value: {
    hidden: false, addEventListener: (_, fn) => documentListeners.add(fn), removeEventListener: (_, fn) => documentListeners.delete(fn),
  } });
  try {
    const env = browserSceneEnvironment(), reasons = [];
    const unsubscribe = env.subscribe(reason => reasons.push(reason));
    assert.equal(mediaListeners.size, 1); assert.equal(documentListeners.size, 1);
    for (const listener of mediaListeners) listener();
    for (const listener of documentListeners) listener();
    assert.deepEqual(reasons, ['motion', 'visibility']);
    unsubscribe(); assert.equal(mediaListeners.size, 0); assert.equal(documentListeners.size, 0);
  } finally {
    if (documentDescriptor) Object.defineProperty(globalThis, 'document', documentDescriptor); else delete globalThis.document;
    if (mediaDescriptor) Object.defineProperty(globalThis, 'matchMedia', mediaDescriptor); else delete globalThis.matchMedia;
  }
});
