/** Actual public browser distribution + real Space Canvas; only explicit demo supply is mutated. */
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require = createRequire(import.meta.url);
const {chromium, selectors} = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? 'playwright');
const target = process.env.AGENT_SPACE_URL ?? 'http://127.0.0.1:18975/?supply=demo';
assert.equal(new URL(target).searchParams.get('supply'), 'demo', 'Use explicit demo supply; never invoke actions on live Agents');
await selectors.register('spaceTools', () => ({query(root, selector) {return this.queryAll(root, selector)[0] ?? null;}, queryAll(root, selector) {
  return (window.__inspectionRoots ?? []).filter(root => root.host.isConnected).flatMap(root => [...root.querySelectorAll(selector)]).filter(node => node.getClientRects().length && getComputedStyle(node).visibility !== 'hidden');
}}));
const browser = await chromium.launch({headless: true, executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', args: ['--no-proxy-server']});
try {
  const page = await browser.newPage({viewport: {width: 1440, height: 1000}}), errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {window.__inspectionRoots = []; const attach = Element.prototype.attachShadow;
    Element.prototype.attachShadow = function(options) {const root = attach.call(this, options); window.__inspectionRoots.push(root); return root;};});
  await page.goto(target, {waitUntil: 'networkidle'});
  await page.waitForFunction(() => window.dancingai?.inspection?.sources().some(source => source.id.startsWith('agent-space:')));
  await page.waitForFunction(() => {
    const api = window.agentxSceneDebug, canvases = api.list({kind: 'canvases'});
    if (!canvases.ok || !canvases.value[0]?.lastFrame?.viewport.width) return false;
    const canvasId = canvases.value[0].id, objects = api.list({kind: 'objects', canvasId});
    return objects.ok && objects.value.some(object => object.kind === 'actor' && api.list({kind: 'actions', canvasId, objectId: object.id}).value?.some(action => action.id === 'object.activate' && action.availability.available));
  });
  const result = await page.evaluate(async () => {
    const inspection = window.dancingai.inspection;
    const source = inspection.sources().find(source => source.id.startsWith('agent-space:'));
    const listing = inspection.list(source.id, 2000);
    if (listing.truncated) throw Error('Demo inventory was truncated');
    const root = listing.items.find(item => item.type === 'AgentSpaceCanvas'), native = inspection.objects.get(root.handle);
    const detail = inspection.inspect(root.handle), actor = listing.items.find(item => {
      if (item.type !== 'AgentSpace/actor') return false;
      const detail = inspection.inspect(item.handle);
      return !detail.data.observation.details.selected && detail.data.actions.some(action => action.id === 'object.activate' && action.availability.available);
    });
    if (!actor) throw Error('No available demo actor');
    const before = inspection.inspect(actor.handle);
    const bounds = inspection.bounds(actor.handle);
    let unavailable; try {inspection.objects.get(actor.handle);} catch (error) {unavailable = error.code;}
    const description = inspection.objects.describe(native);
    const canvasId = detail.data.canvasId;
    const call = native.invoke({canvasId, actionId: 'object.activate', objectId: actor.id, args: {}});
    if (!call.ok) throw Error(JSON.stringify(call));
    const frame = await native.waitForFrame({canvasId, afterFrame: call.value.afterFrame, afterInputRevision: call.value.afterInputRevision});
    if (!frame.ok) throw Error(JSON.stringify(frame));
    const after = native.inspect({canvasId, objectId: actor.id});
    return {oldHandle: root.handle, rootNative: native === window.agentxSceneDebug, unavailable,
      bounds, beforeKey: before.data.observation.details.key, selected: after.value.details.selected,
      selectedKey: frame.value.selectedKey, invokeParameters: description.methods.find(method => method.name === 'invoke').parameters};
  });
  assert.equal(result.rootNative, true); assert.equal(result.unavailable, 'UNAVAILABLE'); assert.ok(result.bounds.width > 0);
  assert.equal(result.selected, true); assert.equal(result.selectedKey, result.beforeKey); assert.ok(result.invokeParameters[0].fields.actionId);
  await page.waitForFunction(() => window.dancingai.inspection.sources().some(source => source.id.startsWith('agent-space:')));
  const observed = await page.evaluate(async oldHandle => {
    const inspection = window.dancingai.inspection;
    let stale; try {inspection.objects.get(oldHandle);} catch (error) {stale = error.code;}
    const source = inspection.sources().find(source => source.id.startsWith('agent-space:'));
    const listing = inspection.list(source.id, 2000), root = listing.items.find(item => item.type === 'AgentSpaceCanvas');
    const image = await inspection.capture(root.handle, 'captureScene');
    return {stale, sourceId: source.id, imageScope: image.scope, png: image.dataUrl.startsWith('data:image/png;base64,'), count: listing.items.length};
  }, result.oldHandle);
  assert.equal(observed.stale, 'STALE'); assert.equal(observed.imageScope, 'scene'); assert.equal(observed.png, true); assert.ok(observed.count > 10);
  const tools = selector => page.locator('spaceTools=' + selector);
  await tools('button[aria-label="展开 Inspection"]').click();
  await tools('button[aria-label="对象与数据"]').click();
  await tools('select[aria-label="对象来源"]').selectOption(observed.sourceId);
  await tools('.dv-studio-tree button').filter({hasText: 'AgentX pixel office'}).click();
  await tools('button[aria-label="开始实时观察"]').click();
  const reader = tools('#object-reader'), firstSample = await reader.innerText();
  await page.waitForTimeout(650);
  assert.notEqual(await reader.innerText(), firstSample, 'Live observation reads newer presented frames');
  await tools('button[aria-label="暂停实时观察"]').click();
  const pausedSample = await reader.innerText();
  await page.waitForTimeout(450);
  assert.equal(await reader.innerText(), pausedSample, 'Pause retains the displayed sample');
  await tools('[role="tab"]').filter({hasText: '能力'}).click();
  await tools('button[aria-label="查看 invoke 调用代码"]').waitFor();
  await page.screenshot({path: process.env.AGENT_SPACE_SCREENSHOT ?? '/tmp/agent-space-dancingai.png'});
  await tools('button[aria-label="关闭对象与数据"]').click(); await tools('button[aria-label="收起 Inspection"]').waitFor();
  // The expanded floating toolbar occupies the app's top-right navigation area.
  // Exercise its native drag control to keep both tool and application controls usable.
  const grip = await tools('button[aria-label="移动工具栏"]').boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2); await page.mouse.down();
  await page.mouse.move(350, 75, {steps: 8}); await page.mouse.up();
  await page.getByRole('button', {name: '聊天', exact: true}).click();
  await page.waitForFunction(() => !window.dancingai.inspection.sources().some(source => source.id.startsWith('agent-space:')));
  await page.getByRole('button', {name: '场景', exact: true}).click();
  await page.waitForFunction(oldSource => window.dancingai.inspection.sources().some(source => source.id.startsWith('agent-space:') && source.id !== oldSource), observed.sourceId);
  await page.goBack(); await page.waitForFunction(() => !window.dancingai.inspection.sources().some(source => source.id.startsWith('agent-space:')));
  await page.goForward(); await page.waitForFunction(() => window.dancingai.inspection.sources().some(source => source.id.startsWith('agent-space:')));
  await page.setViewportSize({width: 390, height: 844});
  await tools('button[aria-label="对象与数据"]').click(); await tools('button[aria-label="关闭对象与数据"]').click();
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({passed: true, target, objects: observed.count, cases: ['default Page mount', 'real semantic source', 'native facade and parameter metadata', 'no fake actor native instance', 'invoke + later presented frame', 'route invalidates handles and renews source', 'Canvas-only PNG', 'actual object panel', 'live frames and pause retention', 'independent toolbar state', 'chat/remount', 'back/forward', '390px tool interaction']}));
} finally {await browser.close();}
