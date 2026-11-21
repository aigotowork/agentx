/** Real candidate HTTP/mailbox and actual Canvas; only fixture scope mutates. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { chromium } from '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright/index.mjs';
const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
assert.equal(new URL(config.url).hostname, '127.0.0.1');
assert.notEqual(new URL(config.url).port, '38023');
await mkdir(config.out, { recursive: true });
const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--no-proxy-server'] });
const errors = [], mutations = [], evidence = {};
const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, locale: 'zh-CN' });
async function mailbox(pending = true) {
  const response = await context.request.get(`${config.url}/api/v1/human-mailbox?generation=${config.generation}&pending=${pending}&limit=100`);
  assert.equal(response.status(), 200); return response.json();
}
let failure;
try {
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(String(error)));
  page.on('request', request => { if (request.method() !== 'GET') mutations.push({ method: request.method(), url: request.url(), body: request.postDataJSON() }); });
  await page.goto(`${config.url}/space/`);
  const human = page.getByRole('button', { name: '@bigboss 人类通信端', exact: true });
  await human.waitFor();
  await page.waitForFunction(() => document.querySelector('.office-human')?.textContent?.includes('23'));
  await human.click();
  const reader = page.getByRole('region', { name: '发给 @bigboss', exact: true });
  await reader.getByText('@bigboss 信箱 · 23 待处理', { exact: true }).waitFor();
  assert.equal(mutations.length, 0, 'reading never marks');
  const initial = await mailbox(); assert.equal(initial.unhandled_count, 23); assert.equal(initial.messages.length, 23);
  const id = await reader.locator('article').first().getAttribute('data-human-message');
  await reader.getByRole('button', { name: /^处理来自/ }).first().click();
  await reader.getByText('@bigboss 信箱 · 22 待处理', { exact: true }).waitFor();
  assert.equal((await mailbox()).messages.some(message => message.id === id), false);
  await reader.getByRole('button', { name: '全部历史', exact: true }).click();
  await reader.locator(`article[data-human-message="${id}"]`).getByText('已处理', { exact: true }).waitFor();
  const handled = (await mailbox(false)).messages.find(message => message.id === id).handled_at;
  const duplicate = await context.request.post(`${config.url}/api/v1/human-mailbox/handle`, { data: { generation: config.generation, ids: [id] } });
  assert.equal(duplicate.status(), 200);
  assert.equal((await mailbox(false)).messages.find(message => message.id === id).handled_at, handled);
  await reader.getByRole('button', { name: '更早信件', exact: true }).click();
  await reader.getByRole('button', { name: '回到最新', exact: true }).waitFor({ state: 'visible' });
  await page.waitForFunction(() => document.querySelectorAll('.scene-human-reader article').length === 3);
  evidence.olderIds = await reader.locator('article').evaluateAll(nodes => nodes.map(node => node.dataset.humanMessage));
  await reader.getByRole('button', { name: '回到最新', exact: true }).click();
  await reader.getByRole('button', { name: '待处理', exact: true }).click();
  await page.waitForFunction(() => document.querySelectorAll('.scene-human-reader article').length === 20);
  // Pin exactly the displayed page while a new native message arrives. The first post-action
  // read is released normally; this is a controlled response-race fixture, not native timing.
  const displayed = await reader.locator('article').evaluateAll(nodes => nodes.map(node => node.dataset.humanMessage));
  const saved = await mailbox();
  const firstTwenty = { ...saved, messages: saved.messages.slice(0, 20), next_before: saved.messages[19].id };
  let pin = true;
  await page.route('**/api/v1/human-mailbox?*', async route => {
    if (pin) await route.fulfill({ json: firstTwenty }); else await route.continue();
  });
  await promisify(execFile)(config.candidate, ['send', '--from', 'mailbox-fixture', '@bigboss', '--intent', 'inform', '--', 'Later unseen mailbox fixture'], { env: config.env, cwd: config.project });
  const newId = (await mailbox()).messages[0].id;
  assert.ok(!displayed.includes(newId));
  // Release on POST so subsequent GET reconciles the real native state.
  await page.route('**/api/v1/human-mailbox/handle', async route => { pin = false; await route.continue(); });
  await reader.getByRole('button', { name: '处理本页 20 条', exact: true }).click();
  await reader.getByText('@bigboss 信箱 · 3 待处理', { exact: true }).waitFor();
  const remaining = await mailbox(); assert.equal(remaining.unhandled_count, 3); assert.ok(remaining.messages.some(message => message.id === newId));
  assert.equal((await mailbox(false)).messages.length, 24, 'all correspondence survives');
  await page.reload(); await human.click();
  await reader.getByText('@bigboss 信箱 · 3 待处理', { exact: true }).waitFor();
  evidence.native = { firstHandled: id, handledAt: handled, displayedBatch: displayed, laterUnseen: newId, remaining: remaining.messages.map(message => message.id), totalHistory: 24 };
  await page.screenshot({ path: `${config.out}/mailbox-desktop.png` });
  await page.setViewportSize({ width: 390, height: 720 });
  await reader.getByRole('button', { name: '全部历史', exact: true }).click();
  await reader.getByText('已处理', { exact: true }).first().waitFor();
  const bounds = await reader.boundingBox(); assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= 390);
  await page.screenshot({ path: `${config.out}/mailbox-narrow.png` });
  const stale = await context.request.post(`${config.url}/api/v1/human-mailbox/handle`, { data: { generation: 'stale-generation', ids: remaining.messages.map(message => message.id) } });
  assert.notEqual(stale.status(), 200); assert.equal((await mailbox()).unhandled_count, 3);
  evidence.staleStatus = stale.status();

  // Explicit demo supply + seeded random + virtual browser clock exercise decorative time only.
  const motion = await context.newPage(); motion.on('pageerror', error => errors.push(String(error)));
  await motion.addInitScript(() => {
    Math.random = () => .1;
    window.__catDraws = [];
    const clear = CanvasRenderingContext2D.prototype.clearRect, draw = CanvasRenderingContext2D.prototype.drawImage;
    CanvasRenderingContext2D.prototype.clearRect = function(...args) { if (this.canvas.classList.contains('office-canvas')) window.__catDraws = []; return clear.apply(this, args); };
    CanvasRenderingContext2D.prototype.drawImage = function(image, x, y, width, height, ...rest) { if (this.canvas.classList.contains('office-canvas')) window.__catDraws.push({ x, y, width, height }); return draw.call(this, image, x, y, width, height, ...rest); };
  });
  await motion.clock.install();
  await motion.goto(`${config.url}/space/?supply=demo`);
  await motion.waitForFunction(() => window.agentxSceneDebug?.list({ kind: 'canvases' }).value?.[0]?.lastFrame);
  await motion.clock.pauseAt(new Date(Date.now() + 1000));
  const activities = new Map();
  async function sample() {
    return motion.evaluate(() => {
      const api = window.agentxSceneDebug, canvasId = api.list({ kind: 'canvases' }).value[0].id;
      const snapshot = api.snapshot({ canvasId }).value;
      const cat = snapshot.objects.find(object => object.details.activity && object.details.decorative);
      const home = snapshot.objects.find(object => object.kind === 'human');
      const bounds = cat.geometry.bounds.world;
      return { cat, home, frame: snapshot.frame, drawMatched: window.__catDraws.some(draw => Object.keys(draw).every(key => draw[key] === bounds[key])) };
    });
  }
  for (let step = 0; step < 220 && activities.size < 4; step++) {
    await motion.clock.runFor(1000);
    const value = await sample();
    assert.equal(value.drawMatched, true, 'actual Canvas cat draw matches debug presented sample');
    assert.deepEqual(value.home.geometry.anchor.world, { x: 80, y: 84 });
    if (!activities.has(value.cat.details.activity)) {
      activities.set(value.cat.details.activity, value);
      await motion.screenshot({ path: `${config.out}/cat-${value.cat.details.activity}.png` });
    }
  }
  assert.deepEqual([...activities.keys()].sort(), ['jump', 'patrol', 'rest', 'yarn']);
  await motion.getByRole('button', { name: '@bigboss 人类通信端', exact: true }).click();
  await motion.clock.runFor(100);
  assert.equal((await sample()).cat.details.activity, 'rest', 'mailbox selection settles cat');
  await motion.getByRole('button', { name: '关闭人类消息', exact: true }).click();
  await motion.emulateMedia({ reducedMotion: 'reduce' }); await motion.clock.runFor(100);
  const reduced = await sample(); assert.equal(reduced.cat.details.activity, 'rest'); assert.equal(reduced.frame.still, true);
  await motion.emulateMedia({ reducedMotion: 'no-preference' });
  await motion.getByRole('button', { name: '暂停动作', exact: true }).click(); await motion.clock.runFor(100);
  assert.equal((await sample()).cat.details.activity, 'rest');
  evidence.motion = { supply: 'explicit demo', random: .1, clock: 'Playwright virtual RAF/timers', activities: Object.fromEntries(activities), reduced };
  assert.equal(errors.length, 0, JSON.stringify(errors));
  assert.equal(mutations.filter(item => !item.url.endsWith('/human-mailbox/handle')).length, 0);
} catch (error) { failure = error; }
finally {
  await writeFile(`${config.out}/report.json`, JSON.stringify({ pass: !failure, error: failure?.stack, errors, mutations, evidence }, null, 2));
  await browser.close();
}
if (failure) throw failure;
