/** Verify three Space presentations of one exact reader, plus the local Design Canvas stories. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { extname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const [productBase, designBase, output, productBuild, designBuild] = process.argv.slice(2);
assert(productBase && designBase && output && !!productBuild === !!designBuild,
  'Usage: node worlds.browser.mjs PRODUCT_URL DESIGN_URL NEW_EVIDENCE_DIR [PRODUCT_BUILD DESIGN_BUILD]');
const product = new URL(productBase), design = new URL('#candidates', designBase);
const artifacts = productBuild && new Map([[product.origin, resolve(productBuild)], [design.origin, resolve(designBuild)]]);
const fixture = fileURLToPath(new URL('../../space-scene-art/verification/preview.py', import.meta.url));
const { chromium } = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? 'playwright');
const worlds = ['lunar', 'terrarium', 'newsprint'];
const selectedCases = process.env.AGENTX_WORLDS_CASES?.split(',');
const worldLabel = { lunar: '月面站', terrarium: '苔庭', newsprint: '漫画社' };
const styles = ['classic', 'folio', 'signal', ...worlds];
const draftValue = '保留 R-14 的新署名｜浏览器验收草稿';
const name = /^@mora(?:\s|$)/;
const history = /版片编号是 R-14/;
const exact = /^\/api\/v1\/agents\/mora(?:\/messages|\/launch-relations)?$/;
const report = {
  status: 'pending', mode: artifacts ? 'built-artifact and fixture replay; no listener traffic' : 'actual preview HTTP',
  urls: { product: product.href, design: design.href }, artifacts: artifacts && Object.fromEntries(artifacts),
  browser: null, cases: {}, issues: [], requests: {}, extraOrigins: [],
};
const failures = [], openContexts = new Set();
await mkdir(output);
const browser = await chromium.launch({
  executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',
  args: ['--no-sandbox', '--disable-dev-shm-usage'],
});
report.browser = browser.version();
const button = (root, label) => root.getByRole('button', { name: label, exact: typeof label === 'string' });
const stage = root => root.locator('[data-scene-art="studio"]');
const selector = root => root.getByRole('combobox', { name: '界面风格' }).and(root.locator('select:visible'));
const input = root => root.getByRole('textbox', { name: 'Agent 消息草稿' });
const actor = root => button(root, name).and(root.locator('button.office-actor'));
const contact = root => button(root, name).and(root.locator('button[data-agent-key]'));
const portrait = root => root.locator('.reader-identity img.studio-avatar');
const office = '/studio/catalog';

async function readActor(root) {
  await root.getByText(history).waitFor();
  await root.getByRole('heading', { name: /@mora/ }).waitFor();
}
async function src(image) {
  await image.waitFor();
  return image.evaluate(async node => {
    await node.decode();
    return { src: node.currentSrc, width: node.naturalWidth, height: node.naturalHeight };
  });
}
function settle(root) {
  return root.evaluate(async () => {
    await document.fonts.ready;
    await new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}
async function shot(page, filename, locator) {
  await settle(page);
  if (locator) await locator.screenshot({ path: join(output, filename + '.png'), timeout: 6000 });
  else await page.screenshot({ path: join(output, filename + '.png'), timeout: 6000 });
}
async function state(root, world) {
  const ready = stage(root).and(root.locator(world
    ? `[data-art-status="ready"][data-scene-world="${world}"]`
    : '[data-art-status="ready"]:not([data-scene-world])'));
  await ready.waitFor();
  await settle(root);
  return ready;
}
async function scene(root, locator = stage(root)) {
  const result = await locator.evaluate(node => {
    const canvas = node.querySelector('canvas');
    const api = window.agentxSceneDebug;
    const ids = api?.list({ kind: 'canvases' });
    const row = ids?.ok && ids.value.length === 1 ? ids.value[0] : null;
    const result = row && api.snapshot({ canvasId: row.id });
    return {
      world: node.dataset.sceneWorld ?? null, anchors: node.querySelectorAll('button.office-actor').length,
      frame: result?.ok ? result.value.frame : null,
      pixels: canvas.toDataURL(),
    };
  });
  assert(result.pixels.length > 4000, 'scene must paint a nonblank-sized PNG');
  return { ...result, pixels: createHash('sha256').update(result.pixels).digest('hex') };
}
function sameRoute(page) {
  const p = new URL(page.url()).searchParams;
  return Object.fromEntries(['office', 'scope', 'generation', 'agent', 'incarnation', 'stopped_event', 'record'].map(key => [key, p.get(key)]));
}
function verifyRoute(route) {
  assert.equal(route.office, office);
  assert.equal(route.scope, '/fixture/agentx-space-studio/.agentx');
  assert.equal(route.generation, 'studio-review-1');
  assert.equal(route.agent, 'mora');
  assert.equal(route.incarnation, '101');
  assert.equal(route.record, 'current');
  assert.equal(route.stopped_event, null);
}
function uri(style) {
  const url = new URL(product);
  url.search = new URLSearchParams({ art: 'studio', style, office });
  return url.href;
}
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json',
  '.svg': 'image/svg+xml', '.png': 'image/png', '.woff2': 'font/woff2', '.txt': 'text/plain' };
async function replay(route) {
  const request = route.request(), url = new URL(request.url());
  const dir = artifacts.get(url.origin);
  if (!dir) return route.abort('blockedbyclient');
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
  const rel = url.pathname === '/' ? 'index.html' : decodeURIComponent(url.pathname).replace(/^\//, '');
  const file = resolve(dir, rel);
  assert(file.startsWith(dir + sep), 'replay only reads selected built artifacts');
  const contentType = mime[extname(file)] ?? (url.pathname.startsWith('/__uiux/') ? 'application/json' : 'application/octet-stream');
  try {
    return await route.fulfill({ contentType, body: await readFile(file) });
  } catch (error) {
    if (error.code !== 'ENOENT' && error.code !== 'EISDIR') throw error;
    return route.fulfill({ status: 404, body: 'Not found in selected build' });
  }
}

async function tab(label, viewport, phone = false) {
  const context = await browser.newContext({ viewport, reducedMotion: 'reduce', isMobile: phone, hasTouch: phone, serviceWorkers: 'block' });
  openContexts.add(context);
  if (artifacts) await context.route('**/*', replay);
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const api = report.requests[label] = [];
  context.on('request', request => {
    const url = new URL(request.url());
    if (!['http:', 'https:'].includes(url.protocol)) return;
    if (url.pathname.startsWith('/api/') || !['GET', 'HEAD', 'OPTIONS'].includes(request.method())) {
      api.push({ origin: url.origin, method: request.method(), path: url.pathname, query: Object.fromEntries(url.searchParams) });
    }
    if (url.origin !== product.origin && url.origin !== design.origin && !report.extraOrigins.includes(url.origin)) report.extraOrigins.push(url.origin);
  });
  context.on('response', response => {
    const url = new URL(response.url());
    if (response.status() >= 400 && (/^\/api\//.test(url.pathname) || /\.(svg|js|css)$/.test(url.pathname)))
      report.issues.push({ label, code: response.status(), path: url.pathname });
  });
  page.on('pageerror', error => report.issues.push({ label, error: error.message }));
  return { page, api, async close() { await context.close(); openContexts.delete(context); } };
}
function verifyNetwork(api, baseline) {
  const calls = api.filter(row => exact.test(row.path));
  assert(api.every(row => row.method === 'GET' && row.origin === product.origin));
  assert(calls.some(row => row.path.endsWith('/mora') && row.query.incarnation === '101'));
  assert(calls.some(row => row.path.endsWith('/messages') && row.query.incarnation === '101'));
  assert(calls.every(row => row.query.incarnation === '101' && !row.query.stopped_event));
  if (baseline !== undefined) assert.equal(calls.length, baseline, 'style switches must not repeat exact reads');
  return calls.length;
}

async function productCase(label, viewport, action, phone = false) {
  if (selectedCases && !selectedCases.includes(label)) return;
  const session = await tab(label, viewport, phone);
  const detail = report.cases[label] = { status: 'pending' };
  try {
    await action(session.page, session.api, detail);
    detail.status = 'pass';
  } catch (error) {
    detail.status = 'fail';
    detail.failure = String(error.stack ?? error);
    failures.push(label);
    await shot(session.page, label + '-failure').catch(() => {});
    detail.snapshot = (await session.page.locator('body').ariaSnapshot({ timeout: 3000 }).catch(() => '')).slice(0, 5000);
    console.error(label + ': ' + error.message);
  } finally { await session.close(); }
}

async function enter(page, style) {
  const response = await page.goto(uri(style), { waitUntil: 'domcontentloaded', timeout: 12000 });
  assert(response?.ok(), 'preview HTML did not load');
  const roomActors = stage(page).locator(`button.office-actor[data-room-key="${office}"]`);
  await roomActors.first().waitFor({ state: 'attached' });
  await state(page, style);
}

for (const style of worlds) {
  await productCase('desktop-' + style, { width: 1440, height: 900 }, async (page, api, detail) => {
    await enter(page, style);
    detail.title = await page.title();
    detail.office = await scene(page);
    assert.equal(detail.office.anchors, 6);
    assert(detail.office.frame, 'expected one diagnostic canvas');
    assert.deepEqual(await selector(page).locator('option').evaluateAll(options => options.map(option => option.value)), styles);
    await shot(page, style + '-1440-office');
    await actor(page).click();
    await readActor(page);
    await input(page).fill(draftValue);
    detail.identity = sameRoute(page);
    verifyRoute(detail.identity);
    detail.portrait = await src(portrait(page));
    await shot(page, style + '-1440-reader');
    detail.exactReads = verifyNetwork(api);
  });
}

await productCase('desktop-switch', { width: 1440, height: 900 }, async (page, api, detail) => {
  await enter(page, 'lunar');
  await actor(page).click();
  await readActor(page);
  await input(page).fill(draftValue);
  const identity = sameRoute(page);
  verifyRoute(identity);
  // Pan the real canvas via the public diagnostic action. Compare a customized camera.
  const prePan = (await scene(page)).frame;
  const pan = await page.evaluate(({ canvasId }) => window.agentxSceneDebug.invoke({ canvasId, actionId: 'camera.pan', args: { dx: 37, dy: -19 } }), prePan);
  // Method shape can be read from the checked-in facade; success still requires a subsequent frame.
  if (!pan.ok) throw new Error('camera.pan failed: ' + JSON.stringify(pan));
  await settle(page);
  const anchor = (await scene(page)).frame;
  assert.equal(anchor.camera.x, prePan.camera.x + 37);
  assert.equal(anchor.camera.y, prePan.camera.y - 19);
  const canvas = await stage(page).locator('canvas').elementHandle();
  const exactCount = verifyNetwork(api);
  const portraits = { lunar: await src(portrait(page)) };
  detail.anchor = anchor;
  detail.switches = [];
  let previous = anchor;
  for (const style of ['terrarium', 'newsprint', 'classic', 'lunar']) {
    const visible = selector(page).filter({ visible: true });
    const compact = visible.and(page.locator('.reader-presentation-control select'));
    await (await compact.count() ? compact : visible.first()).selectOption(style);
    await state(page, worlds.includes(style) ? style : null);
    await readActor(page);
    const presented = await scene(page);
    const image = await src(portrait(page));
    assert.equal(await canvas.evaluate(node => node === document.querySelector('.office-scene canvas')), true, 'Canvas node replaced');
    assert.equal(presented.frame.canvasId, anchor.canvasId);
    assert.deepEqual(presented.frame.camera, anchor.camera);
    assert.equal(presented.frame.selectedKey, anchor.selectedKey);
    assert(presented.frame.frameId > previous.frameId, 'new art must present in a later frame of the same runtime');
    assert.deepEqual(sameRoute(page), identity);
    assert.equal(new URL(page.url()).searchParams.get('style'), style);
    assert.equal(await input(page).inputValue(), draftValue);
    if (style === 'lunar') assert.deepEqual(image, portraits.lunar);
    else {
      assert(!Object.values(portraits).some(existing => existing.src === image.src), 'portrait did not change with style');
      portraits[style] = image;
    }
    verifyNetwork(api, exactCount);
    detail.switches.push({ style, ...presented, portrait: image, exactReads: exactCount });
    previous = presented.frame;
  }
});

for (const style of worlds) {
  await productCase('mobile-' + style, { width: 390, height: 844 }, async (page, api, detail) => {
    await enter(page, style);
    await shot(page, style + '-390-office');
    await button(page, '名册').tap();
    const row = contact(page.getByRole('region', { name: 'Agent 名册' }));
    const rowPortrait = await src(row.locator('img'));
    await row.tap();
    await readActor(page);
    detail.identity = sameRoute(page);
    verifyRoute(detail.identity);
    assert.deepEqual(await src(portrait(page)), rowPortrait);
    await input(page).fill(draftValue);
    detail.composer = await input(page).boundingBox();
    assert(detail.composer && detail.composer.x >= 0 && detail.composer.x + detail.composer.width <= 390 &&
      detail.composer.y >= 0 && detail.composer.y + detail.composer.height <= 844, 'composer outside mobile viewport');
    await shot(page, style + '-390-reader');
    const baseline = verifyNetwork(api);
    const styleSelect = selector(page).and(page.locator('.reader-presentation-control select'));
    await styleSelect.selectOption('classic');
    assert.equal(await input(page).inputValue(), draftValue);
    assert.deepEqual(sameRoute(page), detail.identity);
    const next = worlds[(worlds.indexOf(style) + 1) % worlds.length];
    await styleSelect.selectOption(next);
    await readActor(page);
    assert.equal(await input(page).inputValue(), draftValue);
    assert.deepEqual(sameRoute(page), detail.identity);
    assert.notEqual((await src(portrait(page))).src, rowPortrait.src);
    verifyNetwork(api, baseline);
    detail.switched = ['classic', next];
    const back = button(page, '返回办公室');
    await back.tap();
    await input(page).waitFor({ state: 'hidden' });
    await state(page, next);
    detail.returnFocus = await page.evaluate(() => document.activeElement?.getAttribute('aria-label') ?? document.activeElement?.textContent?.trim().slice(0, 45));
    await button(page.getByRole('navigation', { name: '呈现方式' }), '聊天').tap();
    const again = contact(page.getByRole('region', { name: 'Agent 名册' }));
    await again.focus();
    await page.keyboard.press('Enter');
    await readActor(page);
    assert.deepEqual(sameRoute(page), detail.identity);
    assert.equal(await input(page).inputValue(), draftValue);
    detail.exactReadsAfterReopen = verifyNetwork(api);
  }, true);
}

await productCase('design-canvas', { width: 1600, height: 1000 }, async (page, api, detail) => {
  const response = await page.goto(design.href, { waitUntil: 'domcontentloaded', timeout: 12000 });
  assert(response?.ok());
  detail.title = await page.title();
  detail.candidates = {};
  const candidates = page.locator('[data-space-world]');
  for (const style of worlds) {
    const card = page.getByRole('article', { name: worldLabel[style], exact: true });
    await card.scrollIntoViewIfNeeded(); // The story is mounted lazily inside its comparison card.
    const root = candidates.and(page.locator(`[data-space-world="${style}"]`));
    await root.waitFor({ state: 'attached' });
    await root.scrollIntoViewIfNeeded();
    const node = await state(root, style);
    const evidence = detail.candidates[style] = await scene(root, node);
    assert.equal(evidence.anchors, 6);
    await shot(page, 'design-' + style + '-office', root);
    if (style === 'lunar') {
      await actor(root).click();
      await readActor(root);
      await input(root).fill('Design Canvas local draft');
      assert.equal(await input(root).inputValue(), 'Design Canvas local draft');
      await shot(page, 'design-' + style + '-reader', root);
    }
  }
  assert.deepEqual(api, [], 'Design Canvas must not access Management or make mutations');
});

const officePixels = worlds.map(style => report.cases['desktop-' + style]?.office?.pixels);
const designPixels = worlds.map(style => report.cases['design-canvas']?.candidates?.[style]?.pixels);
for (const [label, pixels] of [['desktop', officePixels], ['design', designPixels]]) {
  if (pixels.every(Boolean) && new Set(pixels).size !== worlds.length) failures.push(label + ': identical rendered pixels');
}
report.status = failures.length || report.issues.length ? 'fail' : selectedCases ? 'partial-pass' : 'pass';
if (report.status === 'fail') process.exitCode = 1;
await Promise.all([...openContexts].map(context => context.close()));
await browser.close();
await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status: report.status, cases: Object.fromEntries(Object.entries(report.cases).map(([key, detail]) => [key, detail.status])), failures, issues: report.issues }, null, 2));
