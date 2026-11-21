/** On-demand registry/adapter contracts. PNG encoding is a supplied port here; real pixels are browser-owned. */
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
const { SceneDebugRegistry } = await import('../src/scene/debug/registry.ts');
const { SpaceDebugAdapter } = await import('../src/scene/debug/space-adapter.ts');
const { createSpaceInspectionProvider } = await import('../src/scene/debug/dancingai-provider.ts');
const { sampleCat } = await import('../src/scene/cat.ts');
const { projectHall } = await import('../src/scene/model.ts');
const { worldToScreen } = await import('../src/scene/geometry.ts');
const read = result => { assert.equal(result.ok, true, JSON.stringify(result)); return result.value; };
function fixture() {
  const resource = {
    id: 'scope:gen:person:inc1', name: 'person', title: '@person', state: 'running', nativeStatus: 'active',
    ref: { scope: 'scope', generation: 'gen', name: 'person', incarnation: 'inc1', record: 'current' },
    membership: { kind: 'directory', path: '/test' }, role: { kind: 'none' }, createdAt: '', pendingMessages: 2,
  };
  const scene = projectHall([resource], new Map([['/test', 'Test']]), 1200);
  const input = { scene, camera: { x: 5, y: 10, zoom: 1 }, viewport: { width: 1000, height: 700 }, visibleKeys: new Set(scene.actors.map(actor => actor.key)), paused: false, props: [], humanMessages: 1 };
  return input;
}
function frame(input, elapsed = 0) {
  return { input, elapsed, cat: sampleCat(input.scene, elapsed, .3, input.paused), still: input.paused, flights: [], actors: input.scene.actors.map(actor => {
    const frame = { ...actor.home, pose: 'sit', facing: 'up' };
    return { actor, frame, screen: worldToScreen(frame, input.camera), dim: !input.visibleKeys.has(actor.key), hidden: !input.visibleKeys.has(actor.key) };
  }) };
}
function harness({ controls = true, present = true } = {}) {
  const registry = new SceneDebugRegistry(), calls = [];
  const surface = { width: 2000, height: 1400,
    getBoundingClientRect: () => ({ x: 100, y: 50, width: 1000, height: 700 }),
    toDataURL: () => 'data:image/png;base64,AA==',
  };
  let adapter;
  const mount = registry.mount(id => adapter = new SpaceDebugAdapter(id, surface, () => undefined));
  const input = fixture();
  const callbacks = {
    actor: (actor, point) => calls.push({ action: 'actor', key: actor.key, point }),
    human: () => calls.push({ action: 'human' }), prop: prop => calls.push({ action: 'prop', prop }),
    pan: (dx, dy) => calls.push({ action: 'pan', dx, dy }),
    controls: controls ? { zoom: factor => calls.push({ action: 'zoom', factor }), panorama: () => calls.push({ action: 'panorama' }), pause: paused => calls.push({ action: 'pause', paused }) } : undefined,
  };
  adapter.update(input, callbacks);
  if (present) adapter.present(frame(input));
  return { api: registry.facade, adapter, mount, input, callbacks, calls, surface, canvasId: mount.id };
}

test('DancingAI projects original frames/relations/capture and exposes only the real public facade', () => {
  const h = harness({present: false});
  const provider = createSpaceInspectionProvider(h.api, read(h.api.list({kind: 'canvases'}))[0]);
  const root = provider.list(1).items[0];
  assert.equal(provider.inspect(root.id).data.observation.state, 'awaiting-frame');
  assert.deepEqual(provider.inspect(root.id).capabilities, {});
  h.adapter.present(frame(h.input));
  assert.equal(provider.list(1).truncated, true);
  const items = provider.list(2000).items, actor = items.find(item => item.type === 'AgentSpace/actor');
  const room = items.find(item => item.type === 'AgentSpace/room');
  assert.ok(provider.inspect(room.id).relations.some(edge => edge.type === 'contains' && edge.target.id === actor.id));
  assert.equal(provider.getObject(root.id), h.api); assert.equal(provider.getObject(actor.id), undefined);
  const detail = provider.inspect(actor.id);
  assert.deepEqual(detail.capabilities.bounds(), detail.data.observation.geometry.bounds.client);
  const box = detail.capabilities.bounds();
  assert.equal(provider.hitTest({x:box.x+box.width/2,y:box.y+box.height/2}), actor.id);
  assert.equal(provider.hitTest({x:-100,y:-100}), undefined);
  const capture = detail.capabilities.captureScene();
  assert.equal(capture.scope, 'scene'); assert.equal(capture.dataUrl, h.surface.toDataURL());
  assert.ok(detail.data.actions.some(action => action.id === 'object.activate'));
  const description = h.api[Symbol.for('@dancingai/objects/describe/v1')]();
  assert.equal(description.methods.find(method => method.name === 'invoke').effect, 'write');
  h.input.scene.actors.length = 0;
  h.adapter.present({...frame(h.input), actors: []});
  assert.equal(provider.inspect(actor.id), undefined);
  h.mount.dispose(); assert.equal(provider.isCurrent(), false);
});

test('fresh discovery needs no IDs; mount, pre-frame, retired and empty registry states are distinct', async () => {
  const registry = new SceneDebugRegistry(), api = registry.facade;
  assert.equal(Object.isFrozen(api), true);
  const description = read(api.describe());
  assert.equal(description.version, 1); assert.ok(description.model.objectKinds.includes('furniture'));
  assert.deepEqual(read(api.list({ kind: 'canvases' })), []);
  assert.equal(api.snapshot({ canvasId: 'absent' }).error.code, 'no_scene');
  const h = harness({ present: false });
  const discovered = read(h.api.list({ kind: 'canvases' }))[0];
  assert.equal(discovered.state, 'awaiting-frame');
  assert.equal(h.api.snapshot({ canvasId: discovered.id }).error.code, 'no_frame');
  const waiting = h.api.waitForFrame({ canvasId: discovered.id, afterFrame: 0 });
  h.adapter.present(frame(h.input));
  assert.equal(read(await waiting).frameId, 1);
  h.mount.dispose(); h.mount.dispose();
  assert.deepEqual(read(h.api.list({ kind: 'canvases' })), []);
  assert.equal(h.api.snapshot({ canvasId: discovered.id }).error.code, 'unknown_canvas');
});

test('semantic discovery includes exact actors and room-bitmap furniture with truthful coordinate spaces', () => {
  const h = harness(), objects = read(h.api.list({ kind: 'objects', canvasId: h.canvasId }));
  const actor = objects.find(item => item.kind === 'actor'), room = objects.find(item => item.kind === 'room');
  assert.equal(actor.parentId, room.id);
  assert.equal(actor.details.reference.incarnation, 'inc1');
  assert.deepEqual(actor.geometry.anchor.canvas, worldToScreen(actor.details.sampled, h.input.camera));
  assert.equal(actor.geometry.anchor.client.x, actor.geometry.anchor.canvas.x + 100);
  assert.equal(actor.geometry.anchor.backing.x, actor.geometry.anchor.canvas.x * 2);
  assert.ok(actor.geometry.bounds.world.width > 0);
  assert.ok(objects.some(item => item.kind === 'furniture' && item.rendering === 'room-bitmap' && item.parentId === room.id));
  assert.ok(objects.some(item => item.kind === 'effect' && item.rendering === 'aggregate' && item.geometry.bounds === null));
  const furniture = objects.find(item => item.kind === 'furniture');
  const unavailable = read(h.api.list({ kind: 'actions', canvasId: h.canvasId, objectId: furniture.id })).find(action => action.scope === 'object');
  assert.equal(unavailable.availability.available, false);
  assert.equal(h.api.invoke({ canvasId: h.canvasId, actionId: unavailable.id, objectId: furniture.id }).error.code, 'unavailable_action');
  h.mount.dispose();
});

test('all snapshot/inspection/discovery data is detached from the presented frame and action definitions', () => {
  const h = harness(), before = read(h.api.snapshot({ canvasId: h.canvasId }));
  const actor = before.objects.find(item => item.kind === 'actor');
  before.frame.camera.x = 999;
  actor.geometry.anchor.world.x = 999;
  actor.details.reference.incarnation = 'wrong';
  const actions = read(h.api.list({ kind: 'actions', canvasId: h.canvasId }));
  actions.find(item => item.id === 'camera.zoom').parameters.factor.minimum = -999;
  const after = read(h.api.inspect({ canvasId: h.canvasId, objectId: actor.id }));
  assert.notEqual(after.geometry.anchor.world.x, 999); assert.equal(after.details.reference.incarnation, 'inc1');
  assert.equal(h.api.invoke({ canvasId: h.canvasId, actionId: 'camera.zoom', args: { factor: -1 } }).error.code, 'invalid_arguments');
  assert.equal(h.input.camera.x, 5);
  h.mount.dispose();
});

test('discovered activation uses exact callback; receipt remains unconfirmed until an observed later input/frame', async () => {
  const h = harness(), canvas = read(h.api.list({ kind: 'canvases' }))[0];
  const objects = read(h.api.list({ kind: 'objects', canvasId: canvas.id }));
  const actor = objects.find(item => item.kind === 'actor');
  const action = read(h.api.list({ kind: 'actions', canvasId: canvas.id, objectId: actor.id })).find(item => item.scope === 'object' && item.availability.available);
  const receipt = read(h.api.invoke({ canvasId: canvas.id, actionId: action.id, objectId: actor.id, args: {} }));
  assert.equal(receipt.presentation, 'not-confirmed');
  assert.deepEqual(h.calls[0], { action: 'actor', key: actor.details.key, point: actor.geometry.anchor.world });
  let resolved = false;
  const wait = h.api.waitForFrame({ canvasId: canvas.id, afterFrame: receipt.afterFrame, afterInputRevision: receipt.afterInputRevision }).then(result => { resolved = true; return result; });
  h.adapter.present(frame(h.input, 40));
  await Promise.resolve(); assert.equal(resolved, false, 'unrelated animation frame does not satisfy newer-input threshold');
  const selected = { ...h.input, selected: { key: actor.details.key, point: actor.geometry.anchor.world } };
  h.adapter.update(selected, h.callbacks); h.adapter.present(frame(selected, 80));
  assert.equal(read(await wait).selectedKey, actor.details.key);
  assert.equal(read(h.api.inspect({ canvasId: canvas.id, objectId: actor.id })).details.selected, true);
  h.mount.dispose();
});

test('invalid/unsupported/unavailable actions never invoke callbacks, and pending exact-identity input blocks stale activation', () => {
  const h = harness({ controls: false }), request = { canvasId: h.canvasId };
  const actor = read(h.api.list({ kind: 'objects', ...request })).find(item => item.kind === 'actor');
  assert.equal(h.api.invoke({ ...request, actionId: 'native.send' }).error.code, 'unsupported_action');
  for (const args of [{ dx: NaN, dy: 0 }, { dx: 10001, dy: 0 }, { dx: 0, dy: 0, extra: true }, null, []]) {
    assert.equal(h.api.invoke({ ...request, actionId: 'camera.pan', args }).error.code, 'invalid_arguments');
  }
  assert.equal(h.api.invoke({ ...request, actionId: 'camera.zoom', args: { factor: 2 } }).error.code, 'unavailable_action');
  const replaced = fixture();
  replaced.scene.actors[0].key = 'scope:gen:person:inc2';
  h.adapter.update(replaced, h.callbacks);
  assert.equal(h.api.invoke({ ...request, actionId: 'object.activate', objectId: actor.id }).error.code, 'unavailable_action');
  h.adapter.present(frame(replaced));
  assert.equal(h.api.invoke({ ...request, actionId: 'object.activate', objectId: actor.id }).error.code, 'unknown_object');
  assert.deepEqual(h.calls, []);
  h.mount.dispose();
});

test('pan/zoom/panorama/pause use explicit existing owners and do not mutate the frame', () => {
  const h = harness(), request = { canvasId: h.canvasId };
  for (const [actionId, args] of [['camera.pan', { dx: 30, dy: -10 }], ['camera.zoom', { factor: 1.25 }], ['camera.panorama', {}], ['animation.pause', { paused: true }]]) {
    assert.equal(read(h.api.invoke({ ...request, actionId, args })).accepted, true);
  }
  assert.deepEqual(h.calls, [{ action: 'pan', dx: 30, dy: -10 }, { action: 'zoom', factor: 1.25 }, { action: 'panorama' }, { action: 'pause', paused: true }]);
  assert.equal(h.input.paused, false); assert.deepEqual(h.input.camera, { x: 5, y: 10, zoom: 1 });
  h.mount.dispose();
});

test('capture correlates the actual supplied pixel port with last presentation; encoding failure is explicit', () => {
  const h = harness(), request = { canvasId: h.canvasId };
  const snapshot = read(h.api.snapshot(request)), capture = read(h.api.capture(request));
  assert.deepEqual(capture.frame, snapshot.frame); assert.deepEqual(capture.canvas, snapshot.canvas);
  assert.equal(capture.includes, 'canvas-only'); assert.equal(capture.dataUrl, 'data:image/png;base64,AA==');
  assert.deepEqual(capture.canvas.backing, { width: 2000, height: 1400 });
  h.surface.toDataURL = () => { throw new Error('tainted'); };
  assert.equal(h.api.capture(request).error.code, 'capture_failed');
  h.surface.toDataURL = () => 'data:,';
  assert.equal(h.api.capture(request).error.code, 'capture_failed');
  h.mount.dispose();
});

test('bounded wait reports timeout/disposal, validates thresholds and leaves remount identity distinct', async () => {
  const h = harness(), request = { canvasId: h.canvasId };
  assert.equal((await h.api.waitForFrame({ ...request, afterFrame: .5 })).error.code, 'invalid_arguments');
  assert.equal((await h.api.waitForFrame({ ...request, afterFrame: 1, timeoutMs: 1 })).error.code, 'frame_timeout');
  const waiting = h.api.waitForFrame({ ...request, afterFrame: 1, timeoutMs: 10000 });
  h.mount.dispose();
  assert.equal((await waiting).error.code, 'disposed');
  const another = harness(); assert.notEqual(another.canvasId, h.canvasId); another.mount.dispose();
});

test('live semantic data exposes exact rendered actor frames, decorative cat activity and maintained source locations',()=>{
 const h=harness(),provider=createSpaceInspectionProvider(h.api,read(h.api.list({kind:'canvases'}))[0]);
 const items=provider.list(2000).items,actor=items.find(item=>item.type==='AgentSpace/actor'),human=items.find(item=>item.type==='AgentSpace/human');
 const moving=time=>{const sample=frame(h.input,time);return {...sample,actors:sample.actors.map(actor=>({...actor,frame:{...actor.frame,pose:'walk'}}))};};
 h.adapter.present(moving(0));const before=provider.inspect(actor.id);
 h.adapter.present(moving(160));const after=provider.inspect(actor.id);
 assert.equal(after.data.animation.pose,'walk');assert.notEqual(after.data.animation.frame,before.data.animation.frame);
 assert.ok(after.sources.some(source=>source.file.endsWith('/runtime/sprite.ts')));
 let active;
 for(let time=8000;time<18000;time+=160){h.adapter.present(frame(h.input,time));const detail=provider.inspect(human.id);if(detail.data.animation.activity!=='rest'){active=detail;break;}}
 assert.ok(active,'Cat motion is observed using the presented sample');assert.equal(active.data.activity,'human-mailbox');
 assert.ok(active.data.animation.decorative);assert.ok(active.sources.some(source=>source.file.endsWith('/cat.ts')));
 h.input.paused=true;h.adapter.present(frame(h.input,12000));assert.equal(provider.inspect(human.id).data.animation.paused,true);
 h.mount.dispose();
});
