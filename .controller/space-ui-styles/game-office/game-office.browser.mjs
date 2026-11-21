/** Independent Chromium evidence for the illustrated office and its Design Canvas stories. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { setTimeout as delay } from 'node:timers/promises';
import { extname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const [productURL, designURL, output, productBuild, designBuild] = process.argv.slice(2);
assert(productURL && designURL && output && Boolean(productBuild) === Boolean(designBuild),
  'Usage: node game-office.browser.mjs PRODUCT_URL DESIGN_URL NEW_EVIDENCE_DIR [PRODUCT_BUILD DESIGN_BUILD]');
const product = new URL(productURL), design = new URL('#candidates', designURL);
const builds = productBuild && new Map([[product.origin, resolve(productBuild)], [design.origin, resolve(designBuild)]]);
const fixture = fileURLToPath(new URL('../../space-scene-art/verification/preview.py', import.meta.url));
const { chromium } = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? 'playwright');
const worlds = { inkpost: '烛影邮局', windward: '风屿驿站', jadecourt: '流光庭院' };
const desktop = { width: 1440, height: 900 }, phone = { width: 390, height: 844 };
const office = '/studio/catalog', mora = /^@mora(?:\s|$)/, history = /版片编号是 R-14/;
const identity = { office, scope: '/fixture/agentx-space-studio/.agentx', generation: 'studio-review-1',
  agent: 'mora', incarnation: '101', record: 'current', stopped_event: null };
const exactPath = /^\/api\/v1\/agents\/mora(?:\/messages|\/launch-relations)?$/;
const draft = 'R-14 · 浏览器验收草稿 · 未发送';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const report = { status: 'fail', urls: { product: product.href, design: design.href }, probes: [], cases: {}, requests: {}, issues: [] };
await mkdir(output); // A fresh location preserves any earlier runs.

const button = (root, name) => root.getByRole('button', { name, exact: typeof name === 'string' });
const stage = root => root.locator('.office-scene[data-scene-art="studio"]');
const actor = root => button(root, mora).and(root.locator('button.office-actor'));
const contact = root => button(root, mora).and(root.locator('button[data-agent-key]'));
const composer = root => root.getByRole('textbox', { name: 'Agent 消息草稿' });
const styleSelect = page => page.getByRole('combobox', { name: '界面风格' }).and(page.locator('select:visible')).first();
const view = (page, name) => button(page.getByRole('navigation', { name: '呈现方式' }), name);
function productLink(style) {
  const url = new URL(product);
  url.search = new URLSearchParams({ art: 'studio', office, style });
  return url.href;
}
const exactRoute = page => Object.fromEntries(Object.keys(identity).map(key => [key, new URL(page.url()).searchParams.get(key)]));
async function reader(root) {
  await root.getByRole('heading', { name: /@mora/ }).waitFor();
  await root.getByText(history).waitFor();
}
async function ready(root, style) {
  // The renderer publishes these attributes only once it has presented the selected art.
  const surface = stage(root).and(root.locator(`[data-art-status="ready"][data-scene-world="${style}"]`));
  await surface.waitFor();
  return surface;
}
async function open(page, url) {
  const response = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 10000 });
  assert(response?.ok(), `Preview HTML did not load: ${url}`);
}
async function shot(page, name, target = page) {
  await page.evaluate(() => document.fonts.ready.then(() => {}));
  await target.screenshot({ path: join(output, name + '.png'), timeout: 6000 });
}
async function portrait(root) {
  return root.locator('.reader-identity img.studio-avatar').evaluate(async img => {
    await img.decode();
    return { path: new URL(img.currentSrc).pathname, width: img.naturalWidth, height: img.naturalHeight };
  });
}

async function scene(surface, { png, lamp, actorBox, palette } = {}) {
  const result = await surface.evaluate((node, options) => {
    const canvas = node.querySelector('canvas'), rect = canvas.getBoundingClientRect();
    const api = window.agentxSceneDebug;
    const need = value => {
      if (!value?.ok) throw new Error(value?.error?.message ?? 'Canvas diagnostics are unavailable');
      return value.value;
    };
    // Locate the registered scene by its real client rect; the Design Canvas can mount multiple cards.
    const matches = need(api?.list({ kind: 'canvases' })).filter(entry => entry.lastFrame).map(entry =>
      need(api.snapshot({ canvasId: entry.id }))
    ).filter(({ canvas: { client } }) => ['x', 'y', 'width', 'height'].every(key => Math.abs(client[key] - rect[key]) < 0.5));
    if (matches.length !== 1) throw new Error('Expected one diagnostic match for this Canvas; found ' + matches.length);
    const snapshot = matches[0];
    const context = canvas.getContext('2d');
    const scaleX = canvas.width / rect.width, scaleY = canvas.height / rect.height;
    const sample = ({ x, y, width, height }) => {
      const left = Math.max(0, Math.floor(x)), top = Math.max(0, Math.floor(y));
      return context.getImageData(left, top, Math.min(canvas.width - left, Math.ceil(width)),
        Math.min(canvas.height - top, Math.ceil(height)));
    };
    const actors = snapshot.objects.filter(object => object.kind === 'actor').map(object => {
      const { anchor, dom, bounds } = object.geometry;
      return { name: object.label, key: object.details.key, goal: object.details.goal, anchor: anchor?.world, body: bounds?.backing,
        alignment: dom && anchor && { x: dom.client.x + dom.client.width / 2 - anchor.client.x,
          y: dom.client.y + dom.client.height - anchor.client.y },
        drawn: object.visibility.drawn, intersection: object.visibility.canvasIntersection };
    });
    const room = snapshot.objects.find(object => object.kind === 'room' && object.details.key === '/studio/catalog');
    const result = { world: node.dataset.sceneWorld, frame: snapshot.frame, actors, room: room?.geometry.bounds?.backing };
    if (options.png) {
      const capture = need(api.capture({ canvasId: snapshot.frame.canvasId }));
      if (capture.frame.frameId !== snapshot.frame.frameId) throw new Error('PNG and snapshot refer to different presentations');
      result.png = capture.dataUrl;
    }
    function pixels(box) {
      const { data, width, height } = sample(box);
      return { width, height, pixels: [...data] };
    }
    if (options.lamp) {
      const lantern = node.querySelector('button[aria-label="场景灯"]'), box = lantern.getBoundingClientRect();
      result.lamp = pixels({
        x: (box.x - rect.x - 15) * scaleX, y: (box.y - rect.y - 20) * scaleY,
        width: (box.width + 30) * scaleX, height: (box.height + 34) * scaleY,
      });
    }
    if (options.actorBox) result.actorPixels = pixels(options.actorBox);
    if (options.palette) {
      function colors(box) {
        const { data, width, height } = sample(box);
        const counts = new Map();
        for (let y = 0; y < height; y += 5) for (let x = 0; x < width; x += 5) {
          const at = (y * width + x) * 4;
          const rgb = Array.from(data.subarray(at, at + 3), value => (Math.floor(value / 16) * 16 + 8).toString(16).padStart(2, '0')).join('');
          counts.set(rgb, (counts.get(rgb) ?? 0) + 1);
        }
        return [...counts].sort((a, b) => b[1] - a[1]).slice(0, 7).map(([rgb]) => '#' + rgb);
      }
      result.palette = {
        vista: colors({ x: 0, y: 0, width: canvas.width, height: Math.max(1, result.room.y) }),
        room: colors(result.room),
      };
    }
    return result;
  }, { png: Boolean(png), lamp, actorBox, palette });
  if (png) {
    const data = Buffer.from(result.png.split(',')[1], 'base64');
    assert(data.length > 4000, 'Expected painted, nonblank Canvas');
    result.sha256 = hash(data);
    await writeFile(join(output, png + '.png'), data);
    delete result.png;
  }
  return result;
}
function comparePixels(a, b) {
  assert.deepEqual([a.width, a.height], [b.width, b.height], 'Sampled regions must be the same size');
  let changed = 0, rgbDelta = 0, beforeLight = 0, afterLight = 0;
  for (let i = 0; i < a.pixels.length; i += 4) {
    const delta = Math.abs(a.pixels[i] - b.pixels[i]) + Math.abs(a.pixels[i+1] - b.pixels[i+1]) + Math.abs(a.pixels[i+2] - b.pixels[i+2]);
    if (delta > 6) changed++;
    rgbDelta += delta;
    beforeLight += .2126 * a.pixels[i] + .7152 * a.pixels[i+1] + .0722 * a.pixels[i+2];
    afterLight += .2126 * b.pixels[i] + .7152 * b.pixels[i+1] + .0722 * b.pixels[i+2];
  }
  const total = a.width * a.height;
  return { changed, total, meanRgbDelta: rgbDelta / (3 * total), meanLuminanceDelta: (afterLight - beforeLight) / total };
}
async function nextFrame(page, before, { still, paused, input = false } = {}) {
  const { canvasId, frameId, inputRevision } = before;
  const match = await page.waitForFunction(({ canvasId, frameId, inputRevision, still, paused, input }) => {
    const info = window.agentxSceneDebug?.list({ kind: 'canvases' });
    const frame = info?.value?.find(entry => entry.id === canvasId)?.lastFrame;
    return frame && frame.frameId > frameId && (!input || frame.inputRevision > inputRevision) &&
      (still === undefined || still === frame.still) && (paused === undefined || paused === frame.paused) && frame;
  }, { canvasId, frameId, inputRevision, still, paused, input }, { timeout: 5000 });
  return match.jsonValue();
}
async function noFurtherFrames(page, frame) {
  // Require a quiet interval. A newer input (for example a read refresh) may present once,
  // but may neither move the clock nor schedule another frame of that input.
  const current = () => page.evaluate(canvasId => window.agentxSceneDebug.list({ kind: 'canvases' }).value
    .find(entry => entry.id === canvasId)?.lastFrame, frame.canvasId);
  await delay(180);
  let from = await current();
  assert(from?.still && from.paused === frame.paused && from.elapsed === frame.elapsed);
  const inputRedraws = [];
  for (let attempt = 0; attempt < 4; attempt++) {
    const start = performance.now();
    const result = await page.evaluate(({ canvasId, afterFrame }) =>
      window.agentxSceneDebug.waitForFrame({ canvasId, afterFrame, timeoutMs: 650 }),
    { canvasId: frame.canvasId, afterFrame: from.frameId });
    const to = result.value ?? await current();
    assert(to?.still && to.paused === from.paused && to.elapsed === from.elapsed, 'Still input advanced the animation clock');
    if (!result.ok && result.error.code === 'frame_timeout' && to.frameId === from.frameId) {
      return { elapsedMs: Math.round(performance.now() - start), from, to, inputRedraws };
    }
    assert(to.inputRevision > from.inputRevision, 'Still scene presented a new frame without new input');
    inputRedraws.push({ from: from.frameId, to: to.frameId, inputRevision: to.inputRevision });
    from = to;
  }
  assert.fail('Scene never settled into a 650ms window without frames');
}
function verifyAnchors(evidence) {
  assert.equal(evidence.actors.length, 6);
  assert.equal(new Set(evidence.actors.map(a => a.key)).size, 6);
  for (const a of evidence.actors) {
    assert(a.drawn && a.intersection !== 'outside' && a.alignment, a.name + ' is not anchored to the visible scene');
    assert(Math.abs(a.alignment.x) < 2 && Math.abs(a.alignment.y) < 2, a.name + ' DOM target drifted from its rendered anchor');
  }
}
async function lampAndMotion(page, surface, style, detail, requests) {
  const nav = page.getByRole('navigation', { name: '办公室镜头' });
  const pause = button(nav, '暂停动作'), lamp = button(surface, '场景灯');
  const beforePause = (await scene(surface)).frame;
  assert.equal(await lamp.getAttribute('aria-pressed'), 'true');
  await pause.click();
  const paused = await nextFrame(page, beforePause, { still: true, paused: true });
  const canvas = await surface.locator('canvas').elementHandle();
  const before = await scene(surface, { lamp: true, png: style + '-lamp-on', palette: true });
  detail.ambient = { baseline: { ...before, lamp: undefined } };
  await shot(page, style + '-1440-room');
  const beforeRequests = requests.filter(row => exactPath.test(row.path)).length;
  await lamp.focus();
  await page.keyboard.press('Space');
  assert.equal(await lamp.getAttribute('aria-pressed'), 'false');
  const blank = await nextFrame(page, before.frame, { still: true, paused: true, input: true });
  const off = await scene(surface, { lamp: true, png: style + '-lamp-off' });
  const delta = comparePixels(before.lamp, off.lamp);
  detail.ambient.off = { frame: off.frame, sha256: off.sha256, pressed: false, delta };
  assert(delta.changed > 50 && delta.meanLuminanceDelta < -1, 'Switch must visibly dim the region around the same lantern');
  assert.equal(await canvas.evaluate(node => node.isConnected), true);
  assert.equal(off.frame.canvasId, paused.canvasId);
  assert.deepEqual(off.frame.camera, before.frame.camera);
  detail.ambient.paused = await noFurtherFrames(page, blank);
  await page.keyboard.press('Enter');
  assert.equal(await lamp.getAttribute('aria-pressed'), 'true');
  await nextFrame(page, off.frame, { still: true, paused: true, input: true });
  const restored = await scene(surface, { lamp: true });
  assert.equal(comparePixels(before.lamp, restored.lamp).changed, 0, 'Pausing must let the original static lamp pixels be restored');
  assert.equal(requests.filter(row => exactPath.test(row.path)).length, beforeRequests);
  await button(nav, '播放动作').click();
  await nextFrame(page, restored.frame, { still: false, paused: false });
  // Fixed pixel boxes plus frame advance distinguish actual visible motion from an empty draw loop.
  const actorBox = restored.actors.find(actor => /^@luna(?:\s|$)/.test(actor.name))?.body;
  assert(actorBox, 'Expected fixture actor @luna in the room');
  const motionA = await scene(surface, { lamp: true, actorBox, png: style + '-motion-a' });
  await delay(560);
  const motionB = await scene(surface, { lamp: true, actorBox, png: style + '-motion-b' });
  const lampDelta = comparePixels(motionA.lamp, motionB.lamp);
  const actorDelta = comparePixels(motionA.actorPixels, motionB.actorPixels);
  detail.ambient.motion = { a: motionA.frame, b: motionB.frame, lampDelta, actorDelta };
  assert(motionB.frame.frameId > motionA.frame.frameId && motionB.frame.elapsed > motionA.frame.elapsed &&
    lampDelta.changed > 0 && actorDelta.changed > 0, 'With lamp on, both lamp and actor pixels must change across actual rendered frames');
  assert.equal(await lamp.getAttribute('aria-pressed'), 'true');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const reduced = await nextFrame(page, motionB.frame, { still: true, paused: false });
  assert(await page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches));
  detail.ambient.reduced = await noFurtherFrames(page, reduced);
}

const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml',
  '.json': 'application/json', '.png': 'image/png', '.woff2': 'font/woff2', '.txt': 'text/plain' };
async function replay(route) {
  const request = route.request(), url = new URL(request.url()), directory = builds.get(url.origin);
  if (!directory) return route.abort('blockedbyclient');
  if (url.pathname.startsWith('/api/')) {
    if (url.origin !== product.origin || request.method() !== 'GET') return route.fulfill({ status: 405 });
    const raw = execFileSync('python3', ['-c', `
import json, runpy, sys
from urllib.parse import parse_qsl
print(json.dumps(runpy.run_path(sys.argv[1])['payload'](sys.argv[2], dict(parse_qsl(sys.argv[3])))))
`, fixture, url.pathname, url.search.slice(1)], { encoding: 'utf8' });
    const [status, json] = JSON.parse(raw);
    return route.fulfill({ status, json });
  }
  const path = resolve(directory, url.pathname === '/' ? 'index.html' : decodeURIComponent(url.pathname).replace(/^\//, ''));
  assert(path.startsWith(directory + sep), 'Replay is restricted to the supplied builds');
  try {
    return await route.fulfill({ contentType: mime[extname(path)] ?? (url.pathname.startsWith('/__uiux/') ? 'application/json' : 'application/octet-stream'),
      body: await readFile(path) });
  } catch (error) {
    if (error.code !== 'ENOENT' && error.code !== 'EISDIR') throw error;
    return route.fulfill({ status: 404, body: 'Not found in this build' });
  }
}
function exactReads(requests) {
  const reads = requests.filter(row => exactPath.test(row.path));
  assert(reads.some(row => row.path.endsWith('/mora')) && reads.some(row => row.path.endsWith('/messages')), 'Expected exact SDK subject and message GETs');
  assert(reads.every(row => row.incarnation === '101' && row.stopped_event === null), 'Reader requested a different incarnation');
  return reads.length;
}
async function probe(browser, url) {
  const page = await browser.newPage();
  try {
    const response = await page.goto(url, { waitUntil: 'commit', timeout: 5000 });
    return { url, status: response?.status(), available: Boolean(response?.ok()) };
  } catch (error) {
    return { url, available: false, error: String(error.message).split('\n')[0] };
  } finally { await page.close(); }
}
async function runCase(browser, label, viewport, action, options = {}) {
  const detail = report.cases[label] = { status: 'fail' };
  const requests = report.requests[label] = [];
  const context = await browser.newContext({ viewport, serviceWorkers: 'block', reducedMotion: 'reduce', ...options });
  await context.route('**/*', route => {
    const request = route.request(), origin = new URL(request.url()).origin;
    if (origin !== product.origin && origin !== design.origin) return route.abort('blockedbyclient');
    if (!['GET', 'HEAD', 'OPTIONS'].includes(request.method())) return route.abort('blockedbyclient');
    return report.mode === 'exact build replay' ? replay(route) : route.continue();
  });
  context.on('request', request => {
    const url = new URL(request.url()), method = request.method();
    if (!['http:', 'https:'].includes(url.protocol)) return;
    if (url.pathname.startsWith('/api/') || !['GET', 'HEAD', 'OPTIONS'].includes(method)) {
      requests.push({ origin: url.origin, path: url.pathname, method, incarnation: url.searchParams.get('incarnation'),
        stopped_event: url.searchParams.get('stopped_event') });
    }
    if (url.origin !== product.origin && url.origin !== design.origin) report.issues.push({ label, unexpectedOrigin: url.origin });
  });
  context.on('response', response => {
    const { pathname: path } = new URL(response.url());
    if (response.status() >= 400 && (path.startsWith('/api/') || /\.(svg|js|css)$/.test(path))) report.issues.push({ label, path, status: response.status() });
  });
  const page = await context.newPage();
  page.setDefaultTimeout(9000);
  page.on('pageerror', error => report.issues.push({ label, error: error.message }));
  try {
    await action(page, detail, requests);
    if (label !== 'design-canvas') {
      assert(requests.length && requests.every(row => row.origin === product.origin && row.method === 'GET'), 'Product must use only same-origin read-only SDK requests');
    }
    detail.status = 'pass';
  } catch (error) {
    detail.failure = String(error.stack ?? error);
    await shot(page, label + '-failure').catch(() => {});
    detail.snapshot = (await page.locator('body').ariaSnapshot({ timeout: 1600 }).catch(() => '')).slice(0, 4800);
    console.error(label + ': ' + error.message);
  } finally { await context.close(); }
}

let browser;
try {
  browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE, args: ['--disable-dev-shm-usage'] });
  report.browser = browser.version();
  report.probes = await Promise.all([probe(browser, productLink('inkpost')), probe(browser, design.href)]);
  if (report.probes.every(row => row.available)) {
    report.mode = 'actual preview HTTP';
  } else {
    assert(builds, 'Direct Chromium could not reach both previews; provide the Controller’s exact product and Design Canvas build paths to replay');
    report.mode = 'exact build replay';
    report.builds = await Promise.all([...builds].map(async ([origin, directory]) =>
      ({ origin, directory, indexSha256: hash(await readFile(join(directory, 'index.html'))) })));
    report.fixtureSha256 = hash(await readFile(fixture));
  }

  for (const style of Object.keys(worlds)) await runCase(browser, 'desktop-' + style, desktop, async (page, detail, requests) => {
    await open(page, productLink(style));
    const surface = await ready(page, style);
    detail.url = page.url(); detail.title = await page.title();
    await lampAndMotion(page, surface, style, detail, requests);
    const room = await scene(surface, { palette: true });
    verifyAnchors(room);
    detail.room = room;
    await actor(page).click();
    await reader(page);
    assert.deepEqual(exactRoute(page), identity);
    detail.reader = { route: exactRoute(page), portrait: await portrait(page), exactReads: exactReads(requests) };
    await composer(page).fill(draft);
    await shot(page, style + '-1440-reader');
  }, { reducedMotion: 'no-preference' });

  await runCase(browser, 'desktop-switch', desktop, async (page, detail, requests) => {
    await open(page, productLink('inkpost'));
    await ready(page, 'inkpost');
    await actor(page).click();
    await reader(page);
    await composer(page).fill(draft);
    const before = await scene(stage(page));
    const reads = exactReads(requests), initialPortrait = await portrait(page);
    const handle = await stage(page).locator('canvas').elementHandle();
    assert(before.frame.selectedKey, 'Reader must select the rendered Agent');
    detail.switches = [];
    let previous = before.frame;
    for (const style of ['windward', 'jadecourt', 'inkpost']) {
      await styleSelect(page).selectOption(style);
      const surface = await ready(page, style);
      const state = await scene(surface), image = await portrait(page);
      assert.equal(await handle.evaluate(node => node === document.querySelector('.office-scene canvas')), true);
      assert.equal(state.frame.canvasId, before.frame.canvasId);
      assert.deepEqual(state.frame.camera, before.frame.camera);
      assert.equal(state.frame.selectedKey, before.frame.selectedKey);
      assert(state.frame.frameId > previous.frameId);
      assert.deepEqual(exactRoute(page), identity);
      assert.equal(await composer(page).inputValue(), draft);
      assert.equal(exactReads(requests), reads);
      if (style === 'inkpost') assert.deepEqual(image, initialPortrait);
      else assert.notEqual(image.path, initialPortrait.path);
      detail.switches.push({ style, frame: state.frame, portrait: image });
      previous = state.frame;
    }
    await view(page, '聊天').click();
    await reader(page);
    assert.deepEqual(exactRoute(page), identity);
    assert.equal(await composer(page).inputValue(), draft);
    await view(page, '场景').click();
    await ready(page, 'inkpost');
    assert.equal(await composer(page).inputValue(), draft);
    assert.equal(exactReads(requests), reads);
  });

  await runCase(browser, 'mobile', phone, async (page, detail, requests) => {
    await open(page, productLink('windward'));
    await ready(page, 'windward');
    await shot(page, 'windward-390-room');
    await button(page, '名册').tap();
    const roster = page.getByRole('region', { name: 'Agent 名册' });
    await contact(roster).tap();
    await reader(page);
    await composer(page).fill(draft);
    const reads = exactReads(requests);
    assert.deepEqual(exactRoute(page), identity);
    const box = await composer(page).boundingBox();
    detail.composer = box;
    assert(box && box.x >= 0 && box.y >= 0 && box.x + box.width <= phone.width && box.y + box.height <= phone.height);
    await styleSelect(page).selectOption('jadecourt');
    await reader(page);
    assert.deepEqual(exactRoute(page), identity);
    assert.equal(await composer(page).inputValue(), draft);
    assert.equal(exactReads(requests), reads);
    await shot(page, 'jadecourt-390-reader');
    await button(page, '返回办公室').tap();
    await composer(page).waitFor({ state: 'hidden' });
    await ready(page, 'jadecourt');
    detail.returnFocus = await page.evaluate(() => document.activeElement?.getAttribute('aria-label') ?? null);
    await view(page, '聊天').tap();
    await contact(roster).focus();
    await page.keyboard.press('Enter');
    await reader(page);
    assert.deepEqual(exactRoute(page), identity);
    assert.equal(await composer(page).inputValue(), draft);
    detail.exactReadsAfterReopen = exactReads(requests);
  }, { isMobile: true, hasTouch: true });

  await runCase(browser, 'design-canvas', { width: 1600, height: 1000 }, async (page, detail, requests) => {
    await open(page, design.href);
    detail.url = page.url(); detail.title = await page.title(); detail.cards = {};
    for (const [style, label] of Object.entries(worlds)) {
      const card = page.getByRole('article', { name: label, exact: true });
      await card.scrollIntoViewIfNeeded(); // Stories are deliberately lazy before their outer card enters view.
      const candidates = card.locator(`[data-space-world="${style}"]`);
      const root = await candidates.count() ? candidates : page.locator(`[data-space-world="${style}"]`);
      await root.waitFor();
      await root.scrollIntoViewIfNeeded();
      const surface = await ready(root, style);
      const evidence = await scene(surface, { png: 'design-' + style + '-canvas', palette: true });
      verifyAnchors(evidence);
      detail.cards[style] = evidence;
      await shot(page, 'design-' + style + '-room', root);
      if (style === 'inkpost') {
        await actor(root).click();
        await reader(root);
        await composer(root).fill(draft);
        assert.equal(await composer(root).inputValue(), draft);
        await shot(page, 'design-' + style + '-reader', root);
        detail.operated = style + ' @mora';
      }
    }
    assert.deepEqual(requests, [], 'Design Canvas stories must stay in local demo state');
  });

  const desktopScenes = Object.keys(worlds).map(style => report.cases['desktop-' + style]?.ambient?.baseline);
  const designScenes = Object.values(report.cases['design-canvas']?.cards ?? {});
  for (const [label, scenes] of [['product', desktopScenes], ['Design Canvas', designScenes]]) {
    if (scenes.length !== 3 || !scenes.every(Boolean)) continue;
    assert.equal(new Set(scenes.map(row => row.sha256)).size, 3, label + ' skins painted identical Canvas pixels');
    assert.equal(new Set(scenes.map(row => JSON.stringify(row.palette))).size, 3, label + ' skins painted the same palette');
    const goals = row => row.actors.map(({ name, goal }) => [name, goal]).sort(([a], [b]) => a.localeCompare(b));
    for (const row of scenes.slice(1)) assert.deepEqual(goals(row), goals(scenes[0]), label + ' changed the cast or its layout anchors');
  }
  assert.deepEqual(report.issues, []);
  if (Object.values(report.cases).every(result => result.status === 'pass')) report.status = 'pass';
} catch (error) {
  report.failure = String(error.stack ?? error);
} finally {
  await browser?.close();
  await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({ status: report.status, mode: report.mode, probes: report.probes, failure: report.failure,
    cases: Object.fromEntries(Object.entries(report.cases).map(([name, result]) => [name, result.status])), issues: report.issues }, null, 2));
  if (report.status !== 'pass') process.exitCode = 1;
}
