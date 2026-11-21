/** Verify visible causal playback, staged facts and compact local disclosure. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const [origin, evidence] = process.argv.slice(2);
assert(origin && evidence, 'Usage: node tests/runtime_atlas_motion_browser.mjs ORIGIN EVIDENCE_DIR');
const root = fileURLToPath(new URL('../', import.meta.url));
const artifact = join(root, '.docs/architecture/runtime-atlas/index.html');
const model = JSON.parse(await readFile(join(root, '.docs/architecture/runtime-atlas/model.json'), 'utf8'));
const require = createRequire(import.meta.url);
const { chromium } = require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
await mkdir(evidence, { recursive: true });
const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true,
  args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const errors = [];
const requests = [];
const checks = [];
let page;

async function capture(name) {
  await page.screenshot({ path: join(evidence, `${name}.png`), fullPage: false });
  const state = await page.evaluate(() => ({ url: location.href, viewport: { width: innerWidth, height: innerHeight },
    phase: document.querySelector('[data-testid="event-phase"]')?.getAttribute('data-phase'),
    facts: document.querySelector('[data-testid="facts"]')?.innerText,
    diff: document.querySelector('[data-testid="state-diff"]')?.innerText,
    documentWidth: document.documentElement.scrollWidth }));
  await writeFile(join(evidence, `${name}.json`), JSON.stringify(state, null, 2));
}

async function phase(value) {
  await page.locator(`[data-testid="event-phase"][data-phase="${value}"]`).waitFor({ timeout: 18000 });
}

async function framePositions(duration = 150) {
  return page.evaluate(duration => new Promise(resolve => {
    const started = performance.now();
    const samples = [];
    function frame() {
      const marker = document.querySelector('[data-testid="flow-marker"]');
      const box = marker?.getBoundingClientRect();
      samples.push({ time: performance.now() - started, x: box?.x, y: box?.y });
      if (performance.now() - started >= duration) resolve(samples);
      else requestAnimationFrame(frame);
    }
    requestAnimationFrame(frame);
  }), duration);
}

function displacement(samples) {
  const positions = samples.filter(item => Number.isFinite(item.x));
  assert(positions.length > 1, 'An actual SVG marker must be present across browser frames');
  const start = positions[0];
  return Math.max(...positions.map(item => Math.hypot(item.x - start.x, item.y - start.y)));
}

async function gotoScenario(id, step = -1) {
  const scenario = model.scenarios.find(item => item.id === id);
  await page.goto(`${origin}#mode=${scenario.modeId}&scenario=${id}&step=${step}`);
  await page.getByTestId('event-phase').waitFor();
}

async function compactGeometry(width, height) {
  await page.setViewportSize({ width, height });
  const geometry = await page.evaluate(() => {
    const box = selector => document.querySelector(selector).getBoundingClientRect().toJSON();
    return { viewport: { width: innerWidth, height: innerHeight },
      map: box('[data-testid="atlas-map"]'), dock: box('[data-testid="step-detail"]'),
      facts: box('[data-testid="facts"]'), width: document.documentElement.scrollWidth,
      svg: box('[data-testid="atlas-map"] svg') };
  });
  assert(geometry.width <= width + 1, 'Only the map region may scroll horizontally');
  assert(geometry.map.height >= height * 0.5, 'The relationship graph must own most of the workspace');
  assert(geometry.dock.height <= 160, 'The playback dock must remain compact');
  assert(geometry.facts.bottom <= height, 'State controls must remain in the desktop viewport');
  await writeFile(join(evidence, `geometry-${width}.json`), JSON.stringify(geometry, null, 2));
}

try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, reducedMotion: 'no-preference' });
  page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => requests.push(request.url()));

  await gotoScenario('appserver-mail');
  const originalFacts = await page.getByTestId('facts').innerText();
  await page.getByTestId('step-play').click();
  await phase('flow');
  await page.getByTestId('flow-marker').first().waitFor();
  const moving = await framePositions();
  assert(displacement(moving) > 4, 'Normal-motion playback must visibly move data along the edge');
  assert.equal(await page.getByTestId('facts').innerText(), originalFacts, 'Facts remain before-state during transport');
  await page.getByTestId('step-play').click();
  assert.equal(await page.getByTestId('step-play').getAttribute('aria-pressed'), 'false');
  const paused = await framePositions();
  assert(displacement(paused) < 1, 'Pause must freeze the current packet');
  await capture('01-paused-flow');
  await writeFile(join(evidence, 'packet-frames.json'), JSON.stringify({ moving, paused }, null, 2));
  await page.getByTestId('step-play').click();
  await phase('applied');
  await page.getByTestId('step-play').click();
  assert.match(await page.getByTestId('facts').innerText(), /1 条待收/);
  assert.match(await page.getByTestId('state-diff').innerText(), /1 条待收/);
  await capture('02-applied-state');
  checks.push('Normal-motion packet moves; pause freezes/resume completes; facts apply after execution with visible change');

  await compactGeometry(1440, 900);
  await compactGeometry(1366, 768);
  await capture('03-compact-desktop');
  await page.getByTestId('detail-toggle').click();
  await page.getByTestId('inspector').waitFor({ state: 'visible' });
  await page.keyboard.press('Escape');
  await page.getByTestId('inspector').waitFor({ state: 'hidden' });
  checks.push('1440x900 and1366x768 map-first geometry; long detail opens locally and Escape dismisses');

  await gotoScenario('appserver-disconnect', 1);
  await page.getByTestId('event-replay').click();
  await page.getByTestId('step-reset').click();
  await phase('ready');
  assert.equal(await page.getByTestId('flow-marker').count(), 0);
  await page.getByTestId('step-next').click();
  await page.locator('[data-testid="event-phase"][data-beat-kind="disconnect"]').waitFor();
  await capture('04-disconnect-in-progress');
  await phase('applied');
  assert.match(await page.getByTestId('facts').innerText(), /客户端连接已断/);
  await capture('04-disconnect');
  checks.push('Disconnect is visible; reset cancels obsolete replay and the next event applies its own state');

  await page.getByTestId('step-play').click();
  await page.getByTestId('mode-select').selectOption('headless');
  await phase('ready');
  assert.equal(await page.getByTestId('flow-marker').count(), 0);
  checks.push('Mode switch cancels old playback rather than applying a late patch');

  await gotoScenario('appserver-mail', 4);
  await page.getByTestId('event-replay').click();
  await page.getByTestId('flow-marker').first().waitFor();
  const reverse = await page.evaluate(() => {
    const path = document.querySelector('[data-testid="map-edge-bridge-native"] .map-edge__line');
    const marker = document.querySelector('[data-testid="flow-marker"]').getBoundingClientRect();
    const matrix = path.getScreenCTM();
    const first = path.getPointAtLength(0).matrixTransform(matrix);
    const last = path.getPointAtLength(path.getTotalLength()).matrixTransform(matrix);
    const position = { x: marker.x + marker.width / 2, y: marker.y + marker.height / 2 };
    return { first: { x: first.x, y: first.y }, last: { x: last.x, y: last.y }, position,
      toStart: Math.hypot(position.x - first.x, position.y - first.y),
      toEnd: Math.hypot(position.x - last.x, position.y - last.y) };
  });
  assert(reverse.toEnd < reverse.toStart, 'Reverse native receipt must begin at the native backend end of the connection');
  await writeFile(join(evidence, 'reverse-origin.json'), JSON.stringify(reverse, null, 2));
  await capture('05-reverse-receipt');
  await page.getByTestId('step-reset').click();
  await phase('ready');
  checks.push('An authored reverse receipt can replay on the native connection without advancing another scenario');

  const narrow = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, reducedMotion: 'reduce' });
  const desktop = page;
  page = await narrow.newPage();
  page.on('pageerror', error => errors.push(error.message));
  await gotoScenario('headless-close');
  await page.getByTestId('step-next').tap();
  await phase('applied');
  assert.match(await page.getByTestId('facts').innerText(), /无后台依赖/);
  assert.match(await page.getByTestId('map-node-pty').textContent(), /后台|宿主/);
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
  await capture('05-narrow-reduced');
  await page.getByTestId('detail-toggle').tap();
  await page.getByTestId('inspector').waitFor({ state: 'visible' });
  await capture('06-narrow-detail');
  await narrow.close();
  checks.push('390px touch/reduced motion retains staged local action and background state with bounded disclosure');

  page = desktop;
  await page.goto(pathToFileURL(artifact).href);
  await page.getByTestId('event-phase').waitFor();
  assert.equal(await page.getByTestId('mode-select').inputValue(), 'appserver');
  assert.deepEqual(errors, []);
  assert(requests.filter(url => /^https?:/.test(url)).every(url => new URL(url).origin === new URL(origin).origin));
  await writeFile(join(evidence, 'result.json'), JSON.stringify({ result: 'passed', checks, errors,
    browser: browser.version(), artifactSha256: createHash('sha256').update(await readFile(artifact)).digest('hex'),
    limits: 'Explanatory animation under real browser frames; no live AgentX/Provider call or measured runtime timing.' }, null, 2));
} catch (error) {
  if (page && !page.isClosed()) await capture('failure');
  await writeFile(join(evidence, 'failure.json'), JSON.stringify({ message: error.message, checks, errors }, null, 2));
  throw error;
} finally {
  await browser.close();
  await writeFile(join(evidence, 'browser-closed.json'), JSON.stringify({ closed: true }));
}
