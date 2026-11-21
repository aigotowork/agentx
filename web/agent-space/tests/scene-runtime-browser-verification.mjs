/** Finite independent scene/debug integration; requires an already owned static demo host. */
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { chromium } from '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright/index.mjs';

const out = process.env.AGENT_SPACE_EVIDENCE;
const url = process.env.AGENT_SPACE_URL;
assert(out && url, 'Explicit owned evidence and host required');
await mkdir(out, { recursive: true });
const records = [], errors = [];
const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox'] });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  page.on('pageerror', error => errors.push(String(error)));
  await page.addInitScript(() => {
    const activity = window.__sceneVerification = { pending: new Set(), visibility: new Set(), media: new Set(), draws: [], count: 0 };
    const request = window.requestAnimationFrame, cancel = window.cancelAnimationFrame;
    window.requestAnimationFrame = callback => {
      const id = request.call(window, time => { activity.pending.delete(id); callback(time); });
      activity.pending.add(id); return id;
    };
    window.cancelAnimationFrame = id => { activity.pending.delete(id); return cancel.call(window, id); };
    const add = EventTarget.prototype.addEventListener, remove = EventTarget.prototype.removeEventListener;
    function tracked(target, type) {
      return target === document && type === 'visibilitychange' ? activity.visibility
        : target instanceof MediaQueryList && type === 'change' && target.media.includes('prefers-reduced-motion') ? activity.media : null;
    }
    EventTarget.prototype.addEventListener = function(type, callback, ...rest) { tracked(this, type)?.add(callback); return add.call(this, type, callback, ...rest); };
    EventTarget.prototype.removeEventListener = function(type, callback, ...rest) { tracked(this, type)?.delete(callback); return remove.call(this, type, callback, ...rest); };
    const clear = CanvasRenderingContext2D.prototype.clearRect, draw = CanvasRenderingContext2D.prototype.drawImage;
    CanvasRenderingContext2D.prototype.clearRect = function(...args) {
      if (this.canvas.classList.contains('office-canvas')) { activity.draws = []; activity.count++; }
      return clear.apply(this, args);
    };
    CanvasRenderingContext2D.prototype.drawImage = function(image, x, y, width, height, ...rest) {
      if (this.canvas.classList.contains('office-canvas')) {
        const matrix = this.getTransform();
        activity.draws.push({ x, y, width, height, matrix: { a: matrix.a, d: matrix.d, e: matrix.e, f: matrix.f } });
      }
      return draw.call(this, image, x, y, width, height, ...rest);
    };
  });
  await page.goto(url);
  await page.waitForFunction(() => window.agentxSceneDebug?.list({ kind: 'canvases' }).value?.[0]?.lastFrame);
  const read = result => { assert.equal(result.ok, true, JSON.stringify(result)); return result.value; };
  async function checkpoint(name, capture = false) {
    // Observe after a presentation opportunity; pending React commits can otherwise be
    // newer than the explicitly last-presented Canvas, as the public contract states.
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const result = await page.evaluate(capture => {
      const api = window.agentxSceneDebug;
      const read = r => { if (!r.ok) throw Error(JSON.stringify(r)); return r.value; };
      const canvasId = read(api.list({ kind: 'canvases' }))[0]?.id;
      const snapshot = canvasId ? read(api.snapshot({ canvasId })) : null;
      const activity = window.__sceneVerification;
      const actors = snapshot?.objects.filter(o => o.kind === 'actor' && o.visibility.drawn).map(object => {
        const dom = [...document.querySelectorAll('.office-actor')].find(node => node.getAttribute('data-hall-object') === object.details.key);
        const x = parseFloat(dom?.style.getPropertyValue('--object-x')), y = parseFloat(dom?.style.getPropertyValue('--object-y'));
        const bounds = object.geometry.bounds.world;
        return { id: object.id, anchor: object.geometry.anchor, domAnchor: { x, y }, bounds,
          drawMatched: activity.draws.some(d => d.x === bounds.x && d.y === bounds.y && d.width === bounds.width && d.height === bounds.height && Math.abs((d.x * d.matrix.a + d.matrix.e) / snapshot.canvas.cssToBacking.x - object.geometry.bounds.canvas.x) < .01) };
      }) ?? [];
      const textarea = document.querySelector('textarea');
      return { snapshot, actors, pending: activity.pending.size, visibility: activity.visibility.size, media: activity.media.size,
        drawCount: activity.count, draft: textarea?.value, composer: textarea?.getBoundingClientRect().toJSON(),
        viewport: { width: innerWidth, height: innerHeight }, scrollWidth: document.documentElement.scrollWidth,
        capture: capture ? read(api.capture({ canvasId })) : null };
    }, capture);
    records.push({ name, ...result });
    if (result.snapshot) {
      for (const actor of result.actors) {
        assert(Math.abs(actor.domAnchor.x - actor.anchor.canvas.x) < .01, `${name}: DOM x`);
        assert(Math.abs(actor.domAnchor.y - actor.anchor.canvas.y) < .01, `${name}: DOM y`);
        assert(actor.drawMatched, `${name}: actual Canvas draw bounds match sampled actor ${actor.id}`);
      }
      assert(result.actors.length, `${name}: actual drawn actor`);
    }
    if (result.capture) {
      const bytes = Buffer.from(result.capture.dataUrl.split(',')[1], 'base64');
      assert.equal(bytes.toString('hex', 0, 8), '89504e470d0a1a0a');
      assert.equal(bytes.readUInt32BE(16), result.capture.canvas.backing.width);
      assert.equal(bytes.readUInt32BE(20), result.capture.canvas.backing.height);
      assert.equal(result.capture.frame.frameId, result.snapshot.frame.frameId);
      assert.equal(result.capture.frame.inputRevision, result.snapshot.frame.inputRevision);
      assert.equal(result.capture.includes, 'canvas-only');
      await writeFile(`${out}/${name}-canvas.png`, bytes);
      delete result.capture.dataUrl;
    }
    return result;
  }
  async function invoke(actionId, args = {}, objectId) {
    return read(await page.evaluate(async request => {
      const api = window.agentxSceneDebug;
      const canvasId = api.list({ kind: 'canvases' }).value[0].id;
      const receipt = api.invoke({ canvasId, ...request });
      if (!receipt.ok) return receipt;
      const r = receipt.value;
      const wait = await api.waitForFrame({ canvasId, afterFrame: r.afterFrame, afterInputRevision: r.afterInputRevision });
      return wait.ok ? { ok: true, value: { receipt: r, frame: wait.value } } : wait;
    }, { actionId, args, ...(objectId ? { objectId } : {}) }));
  }

  const discovery = read(await page.evaluate(() => window.agentxSceneDebug.describe()));
  assert(discovery.model.objectKinds.includes('actor'));
  const overview = await checkpoint('desktop-overview');
  assert(overview.pending <= 1); assert.equal(overview.visibility, 1); assert.equal(overview.media, 1);
  const objects = overview.snapshot.objects;
  assert(objects.some(o => o.kind === 'furniture' && o.rendering === 'room-bitmap'));
  const chosen = read(await page.evaluate(() => {
    const api = window.agentxSceneDebug, read = r => { if (!r.ok) throw Error(JSON.stringify(r)); return r.value; };
    const canvasId = read(api.list({ kind: 'canvases' }))[0].id;
    for (const object of read(api.list({ kind: 'objects', canvasId }))) {
      const actions = read(api.list({ kind: 'actions', canvasId, objectId: object.id }));
      const action = actions.find(a => a.scope === 'object' && a.availability.available && !Object.keys(a.parameters).length);
      if (object.kind === 'actor' && !object.details.selected && action) return { ok: true, value: { object, action, actions } };
    }
    return { ok: false, error: 'No eligible discovered actor' };
  }));
  assert(chosen.action.result.fields.afterInputRevision);
  const continuation = ['pause', 'narrow'].includes(process.env.AGENT_SPACE_CONTINUE);
  let panned;
  if (!continuation) {
  const activation = await invoke(chosen.action.id, {}, chosen.object.id);
  assert.equal(activation.receipt.presentation, 'not-confirmed');
  const selected = await checkpoint('discovered-selection', true);
  assert.equal(selected.snapshot.objects.find(o => o.id === chosen.object.id).details.selected, true);
  assert.equal(selected.snapshot.frame.selectedKey, chosen.object.details.key);
  await page.getByRole('textbox', { name: 'Agent 消息草稿' }).fill('independent scene continuity');
  await page.getByRole('button', { name: '关闭并返回办公室', exact: true }).click();
  await page.waitForTimeout(100);
  const closed = await checkpoint('selection-close');
  assert.deepEqual(closed.snapshot.frame.camera, overview.snapshot.frame.camera);

  }
  const canvasActions = read(await page.evaluate(() => {
    const api = window.agentxSceneDebug, canvasId = api.list({ kind: 'canvases' }).value[0].id;
    return api.list({ kind: 'actions', canvasId });
  }));
  const pan = canvasActions.find(a => a.parameters.dx && a.parameters.dy);
  assert(pan.availability.available);
  await invoke(pan.id, { dx: 23, dy: -11 });
  panned = await checkpoint(continuation ? 'continuation-setup-pan' : 'pan');
  assert.equal(panned.snapshot.frame.camera.x, overview.snapshot.frame.camera.x + 23);
  assert.equal(panned.snapshot.frame.camera.y, overview.snapshot.frame.camera.y - 11);
  assert.equal(panned.snapshot.frame.camera.zoom, overview.snapshot.frame.camera.zoom);
  if (!continuation) {
  await invoke(chosen.action.id, {}, chosen.object.id);
  const reselected = await checkpoint('reselected');
  assert.equal(reselected.draft, 'independent scene continuity');
  assert.equal(reselected.snapshot.frame.camera.zoom, panned.snapshot.frame.camera.zoom);
  await page.getByRole('button', { name: '关闭并返回办公室', exact: true }).click();
  await page.waitForTimeout(100);
  assert.deepEqual((await checkpoint('panned-close')).snapshot.frame.camera, panned.snapshot.frame.camera);
  }

  // Reader close deliberately restores actor focus, whose tooltip owns a separate
  // positioning RAF. Dismiss it before measuring scene decorative quiescence.
  await page.evaluate(() => document.activeElement?.blur());
  await page.mouse.move(0, 0);

  if (process.env.AGENT_SPACE_CONTINUE !== 'narrow') {
  const pause = canvasActions.find(a => a.parameters.paused);
  await invoke(pause.id, { paused: true });
  const paused = await checkpoint('paused');
  await page.waitForTimeout(160);
  const pausedLater = await checkpoint('paused-stable');
  assert.equal(pausedLater.snapshot.frame.elapsed, paused.snapshot.frame.elapsed);
  assert.equal(pausedLater.drawCount, paused.drawCount); assert.equal(pausedLater.pending, 0);
  const errorsAndDetached = await page.evaluate(async ({ chosen, pan }) => {
    const api = window.agentxSceneDebug, canvasId = api.list({ kind: 'canvases' }).value[0].id;
    const before = api.snapshot({ canvasId }).value;
    const object = before.objects.find(o => o.id === chosen.object.id);
    object.details.key = 'caller-mutation'; before.frame.camera.x = 1e9;
    const after = api.snapshot({ canvasId }).value;
    const furniture = after.objects.find(o => o.kind === 'furniture');
    return { detached: after.frame.camera.x !== 1e9 && after.objects.find(o => o.id === object.id).details.key !== 'caller-mutation',
      unsupported: api.invoke({ canvasId, actionId: 'not-registered' }),
      invalid: api.invoke({ canvasId, actionId: pan.id, args: { dx: 'wrong', dy: 0 } }),
      missing: api.inspect({ canvasId, objectId: 'unknown-object' }),
      unavailable: api.invoke({ canvasId, actionId: chosen.action.id, objectId: furniture.id }),
      timeout: await api.waitForFrame({ canvasId, afterFrame: after.frame.frameId, timeoutMs: 20 }) };
  }, { chosen, pan });
  assert(errorsAndDetached.detached);
  for (const [field, code] of Object.entries({ unsupported: 'unsupported_action', invalid: 'invalid_arguments', missing: 'unknown_object', unavailable: 'unavailable_action', timeout: 'frame_timeout' })) assert.equal(errorsAndDetached[field].error.code, code);
  records.push({ name: 'errors-and-detached', ...errorsAndDetached });
  await invoke(pause.id, { paused: false });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForTimeout(100);
  const reduced = await checkpoint('reduced-motion');
  await page.waitForTimeout(160);
  const reducedLater = await checkpoint('reduced-motion-stable');
  assert.equal(reducedLater.snapshot.frame.elapsed, reduced.snapshot.frame.elapsed);
  assert.equal(reducedLater.drawCount, reduced.drawCount); assert.equal(reducedLater.pending, 0);
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.waitForTimeout(100);

  for (let cycle = 0; cycle < 3; cycle++) {
    const oldId = read(await page.evaluate(() => window.agentxSceneDebug.list({ kind: 'canvases' })))[0].id;
    await page.evaluate(() => {
      const api = window.agentxSceneDebug, item = api.list({ kind: 'canvases' }).value[0];
      window.__pendingSceneWait = api.waitForFrame({ canvasId: item.id, afterFrame: item.lastFrame.frameId + 10000, timeoutMs: 10000 });
    });
    await page.getByRole('button', { name: '聊天', exact: true }).click();
    const unmounted = await page.evaluate(async oldId => ({ list: window.agentxSceneDebug.list({ kind: 'canvases' }),
      stale: window.agentxSceneDebug.snapshot({ canvasId: oldId }), waiter: await window.__pendingSceneWait,
      pending: window.__sceneVerification.pending.size, visibility: window.__sceneVerification.visibility.size, media: window.__sceneVerification.media.size }), oldId);
    assert.deepEqual(read(unmounted.list), []); assert.equal(unmounted.stale.error.code, 'unknown_canvas');
    assert.equal(unmounted.waiter.error.code, 'disposed');
    assert.equal(unmounted.pending, 0); assert.equal(unmounted.visibility, 0); assert.equal(unmounted.media, 0);
    records.push({ name: `unmount-${cycle}`, ...unmounted });
    await page.getByRole('button', { name: '场景', exact: true }).click();
    await page.waitForFunction(() => window.agentxSceneDebug.list({ kind: 'canvases' }).value[0]?.lastFrame);
    const remount = await checkpoint(`remount-${cycle}`);
    assert.notEqual(remount.snapshot.frame.canvasId, oldId); assert(remount.pending <= 1);
    assert.equal(remount.visibility, 1); assert.equal(remount.media, 1);
    assert.deepEqual(remount.snapshot.frame.camera, panned.snapshot.frame.camera);
  }
  }
  for (const width of [390, 320]) {
    await page.setViewportSize({ width, height: width === 390 ? 844 : 640 });
    await page.waitForTimeout(120);
    // A retained world camera can point between rooms after a large resize.
    // Use the discovered panorama operation before seeking an on-screen actor.
    const panorama = canvasActions.find(a => a.scope === 'canvas' && !Object.keys(a.parameters).length);
    await invoke(panorama.id);
    // The model and IDs are rediscovered after each remount/resize.
    const choice = await page.evaluate(() => {
      const api = window.agentxSceneDebug, canvasId = api.list({ kind: 'canvases' }).value[0].id;
      for (const object of api.list({ kind: 'objects', canvasId }).value) {
        if (object.kind !== 'actor') continue;
        const action = api.list({ kind: 'actions', canvasId, objectId: object.id }).value.find(a => a.scope === 'object' && a.availability.available);
        if (action) return { object, action };
      }
    });
    assert(choice, `${width}: discover eligible actor`);
    await invoke(choice.action.id, {}, choice.object.id);
    const narrow = await checkpoint(`narrow-${width}`, true);
    assert(narrow.scrollWidth <= width + 1);
    assert(narrow.composer.width > 0 && narrow.composer.bottom <= narrow.viewport.height + 1);
    await page.screenshot({ path: `${out}/narrow-${width}-page.png` });
    await page.setViewportSize({ width, height: 480 });
    await page.waitForTimeout(120);
    const short = await checkpoint(`short-${width}`);
    assert(short.composer.width > 0 && short.composer.bottom <= 481);
    await page.getByRole('button', { name: '关闭并返回办公室', exact: true }).click();
    await page.waitForTimeout(80);
  }
  assert.deepEqual(errors, [], 'No page runtime errors');
  records.push({ name: 'completed', runtimeErrors: errors, contract: discovery });
  await page.close();
} catch (error) {
  records.push({ name: 'FAILED', error: String(error), stack: error.stack, runtimeErrors: errors });
  throw error;
} finally {
  await browser.close();
  await writeFile(`${out}/result.json`, JSON.stringify(records, null, 2));
  console.log(JSON.stringify(records.map(({ name, snapshot, error, runtimeErrors }) => ({ name, frame: snapshot?.frame, error, runtimeErrors })), null, 2));
}
