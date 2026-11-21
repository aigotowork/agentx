/** Actual browser cases for shared trace projections, pacing continuity and reuse. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
const [origin, exampleFile, evidence, selectedPhase] = process.argv.slice(2);
assert(origin && exampleFile && evidence, 'Usage: node tests/runtime_atlas_multiview_browser.mjs ORIGIN EXAMPLE_HTML EVIDENCE');
const require = createRequire(import.meta.url);
const { chromium } = require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const model = JSON.parse(await readFile(new URL('../.docs/architecture/runtime-atlas/model.json', import.meta.url), 'utf8'));
await mkdir(evidence, { recursive: true });
const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const errors = [], requests = [], checks = [];
let page;
async function capture(name) { await page.screenshot({ path: join(evidence, name + '.png') }); }
async function phase(name) { await page.locator(`[data-testid="event-phase"][data-phase="${name}"]`).waitFor({ timeout: 25000 }); }
async function open(scenarioId, step = -1, view = 'relations', speed = 0.5) {
  const scenario = model.scenarios.find(s => s.id === scenarioId);
  await page.goto(`${origin}#mode=${scenario.modeId}&scenario=${scenarioId}&step=${step}&view=${view}&speed=${speed}`);
  await page.getByTestId('event-phase').waitFor();
}
async function frameSample(duration = 240) {
  return page.evaluate(duration => new Promise(resolve => {
    const samples = [], started = performance.now();
    function tick() {
      const marker = document.querySelector('[data-testid="flow-marker"]');
      const matrix = marker?.transform.baseVal.consolidate()?.matrix;
      samples.push({ ms: performance.now() - started, progress: Number(marker?.getAttribute('data-progress')), x: matrix?.e, y: matrix?.f });
      if (performance.now() - started >= duration) resolve(samples); else requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  }), duration);
}
async function geometry(width, height) {
  await page.setViewportSize({ width, height });
  const result = await page.evaluate(() => {
    const rect = id => document.querySelector(`[data-testid="${id}"]`).getBoundingClientRect().toJSON();
    return { width: innerWidth, document: document.documentElement.scrollWidth, map: rect('atlas-map'), facts: rect('facts'), dock: rect('step-detail'), views: document.querySelector('#view-controls').getBoundingClientRect().toJSON(), speed: rect('speed-select') };
  });
  assert(result.document <= width + 1, 'Only the diagram may scroll horizontally');
  assert(result.facts.bottom <= height + 1, 'Facts remain available in the workspace');
  assert(result.map.height >= height * (width <= 600 ? 0.38 : 0.5), 'Diagrams remain primary');
  assert(result.views.right <= width && result.speed.right <= width);
  return result;
}
try {
  const context = await browser.newContext({ viewport: { width: 1366, height: 768 }, reducedMotion: 'no-preference' });
  page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => requests.push(request.url()));
  if (selectedPhase === 'layout') {
    await open('appserver-mail', -1, 'flow');
    await geometry(1366, 768);
    await capture('flow-desktop-final');
    await page.getByTestId('view-sequence').click();
    await capture('sequence-desktop-final');
    await page.getByTestId('view-relations').click();
    await capture('relations-desktop-final');
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByTestId('view-flow').click();
    const cards = await page.locator('.trace-step__body').evaluateAll(items => items.map(item => item.getBoundingClientRect().toJSON()));
    assert(cards.every(item => item.left >= 8 && item.right <= 382), 'Narrow flow cards fit horizontally');
    assert(cards[1].top > cards[0].bottom, 'Narrow flow becomes a vertical chain');
    await geometry(390, 844);
    await capture('flow-narrow-final');
    await open('headless-close', -1, 'sequence', 2);
    await page.getByTestId('step-next').click();
    await phase('applied');
    await capture('sequence-narrow-final');
    const header = await page.getByTestId('sequence-node-pty').boundingBox();
    assert(header.x >= 8 && header.x + header.width <= 382, 'Follow keeps active participant header fully visible');
    await page.goto(pathToFileURL(exampleFile).href);
    await page.getByTestId('event-phase').waitFor();
    assert.match(await page.locator('.map-legend').textContent(), /参与方.*服务.*存储/);
    await page.setViewportSize({ width: 1366, height: 768 });
    await capture('example-relations-final');
    await page.getByTestId('view-flow').click();
    await capture('example-flow-final');
    await page.getByTestId('view-sequence').click();
    await capture('example-sequence-final');
    assert.deepEqual(errors, []);
    await writeFile(join(evidence, 'result.json'), JSON.stringify({ result: 'passed', phase: selectedPhase, browser: browser.version(), checks: ['Responsive vertical narrow flow and desktop row chain', 'Narrow sequence active participant fully readable', 'Model-derived generic kind legend'], errors }, null, 2));
    await browser.close();
    await writeFile(join(evidence, 'browser-closed.json'), JSON.stringify({ closed: true }));
    process.exit(0);
  }
  await open('appserver-mail');
  assert.equal(await page.getByTestId('speed-select').inputValue(), '0.5');
  const speedSamples = {};
  for (const rate of [0.5, 2]) {
    await open('appserver-mail', -1, 'relations', rate);
    await page.getByTestId('step-next').click();
    await page.getByTestId('flow-marker').waitFor();
    const samples = await frameSample();
    speedSamples[rate] = samples;
    assert(Math.hypot(samples.at(-1).x - samples[0].x, samples.at(-1).y - samples[0].y) > 3, 'Real SVG marker moves');
  }
  const velocity = samples => (samples.at(-1).progress - samples[0].progress) / (samples.at(-1).ms - samples[0].ms);
  const ratio = velocity(speedSamples[2]) / velocity(speedSamples[0.5]);
  assert(ratio > 3.4 && ratio < 4.6, '0.5× versus 2× changes actual elapsed motion by four');
  await writeFile(join(evidence, 'speed-frames.json'), JSON.stringify({ ratio, speedSamples }, null, 2));
  checks.push('Default slower rate; actual SVG speed ratio and frame movement');

  await open('appserver-mail', -1, 'relations', 0.25);
  const initialFacts = await page.getByTestId('facts').innerText();
  await page.getByTestId('step-next').click();
  await page.getByTestId('flow-marker').waitFor();
  await page.getByTestId('step-play').click();
  const position = Number(await page.getByTestId('flow-marker').getAttribute('data-progress'));
  const markerBefore = await page.getByTestId('flow-marker').getAttribute('transform');
  await page.getByTestId('speed-select').selectOption('1');
  assert.equal(await page.getByTestId('flow-marker').getAttribute('transform'), markerBefore, 'Rate update cannot move a paused packet');
  await page.getByTestId('view-flow').click();
  assert.equal(await page.locator('[data-step-index]').count(), model.scenarios.find(s => s.id === 'appserver-mail').steps.length);
  assert.equal(await page.getByTestId('facts').innerText(), initialFacts);
  await capture('01-flow-paused');
  await page.getByTestId('view-sequence').click();
  assert(Math.abs(Number(await page.getByTestId('flow-marker').getAttribute('data-progress')) - position) < 1e-8);
  assert.equal(await page.getByTestId('step-play').getAttribute('aria-pressed'), 'false');
  assert.equal(await page.getByTestId('facts').innerText(), initialFacts);
  await capture('02-sequence-paused');
  await page.getByTestId('view-relations').click();
  assert.equal(await page.getByTestId('flow-marker').getAttribute('transform'), markerBefore);
  await page.getByTestId('speed-select').selectOption('2');
  await page.getByTestId('step-play').click();
  await page.getByTestId('view-sequence').click();
  await phase('applied');
  await page.getByTestId('step-play').click();
  assert.match(await page.getByTestId('facts').innerText(), /1 条待收/);
  checks.push('Rate/view switching keeps paused progress, prior facts and running transition; eventual patch shared');

  await open('appserver-mail', -1, 'flow', 2);
  const target = model.scenarios.find(s => s.id === 'appserver-mail').steps[4];
  await page.getByTestId('flow-step-' + target.id).focus();
  await page.keyboard.press('Enter');
  await phase('flow');
  assert.match(await page.getByTestId('step-counter').innerText(), /5 \/ 6/);
  await page.getByTestId('view-sequence').click();
  const packet = page.getByTestId('flow-marker');
  await packet.waitFor();
  assert.equal(await packet.getAttribute('data-direction'), 'reverse');
  const direction = await page.evaluate(() => {
    const row = document.querySelector('.sequence-row.is-current');
    const path = row.querySelector('.map-edge__line');
    const p = row.querySelector('[data-testid="flow-marker"]').transform.baseVal.consolidate().matrix;
    const start = path.getPointAtLength(0), end = path.getPointAtLength(path.getTotalLength());
    return { from: Math.hypot(p.e-start.x,p.f-start.y), to: Math.hypot(p.e-end.x,p.f-end.y) };
  });
  assert(direction.to < direction.from, 'Reverse packet starts at the destination endpoint');
  await page.getByTestId('step-reset').click();
  await phase('ready');
  assert.equal(await page.getByTestId('flow-marker').count(), 0);
  await open('appserver-disconnect', -1, 'sequence', 2);
  await page.getByTestId('step-next').click();
  await page.getByTestId('flow-break').waitFor();
  assert.equal(await page.getByTestId('flow-marker').count(), 0, 'Disconnect is not a successful transfer');
  await capture('03-sequence-disconnect');
  checks.push('Flow keyboard navigation stages selected event; sequence preserves reverse/disconnect; reset cancels');

  // Every existing trace projects its real beat count, not a copied diagram.
  for (const scenario of model.scenarios) {
    await open(scenario.id, -1, 'sequence', 2);
    assert.equal(await page.locator('.sequence-row').count(), scenario.steps.reduce((n, step) => n + step.beats.length, 0));
  }
  await open('appserver-mail', -1, 'flow');
  const geometries = [await geometry(1366, 768)];
  await capture('04-flow-desktop');
  await page.getByTestId('view-relations').click();
  await capture('05-relationships-desktop');
  await page.getByTestId('view-sequence').click();
  await capture('06-sequence-desktop');
  const narrow = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, reducedMotion: 'reduce' });
  const desktop = page;
  page = await narrow.newPage();
  page.on('pageerror', error => errors.push(error.message));
  await open('headless-close', -1, 'flow', 2);
  geometries.push(await geometry(390, 844));
  await page.getByTestId('view-sequence').tap();
  await page.getByTestId('step-next').tap();
  await phase('applied');
  assert.match(await page.getByTestId('facts').innerText(), /无后台依赖/);
  await capture('07-narrow-sequence');
  await page.getByTestId('view-flow').tap();
  await capture('08-narrow-flow');
  await narrow.close();
  page = desktop;
  await writeFile(join(evidence, 'geometry.json'), JSON.stringify(geometries, null, 2));
  checks.push('14 trace projections; desktop/narrow compact geometry, touch and reduced-motion application');

  await page.goto(pathToFileURL(exampleFile).href);
  await page.getByTestId('event-phase').waitFor();
  assert.notEqual(await page.getByTestId('mode-select').inputValue(), 'appserver');
  const custom = await page.evaluate(() => JSON.parse(document.getElementById('atlas-model').textContent));
  assert.equal(await page.locator('.fact-card').count(), custom.factDefinitions.length);
  assert.equal(await page.locator('[data-node-id]').count() > 0, true);
  assert(!custom.factDefinitions.some(f => ['connection','host','provider','turn','inbox'].includes(f.id)));
  for (const view of ['relations','flow','sequence']) { await page.getByTestId('view-' + view).click(); await capture('example-' + view); }
  await page.getByTestId('speed-select').selectOption('2');
  await page.getByTestId('step-next').click();
  await phase('applied');
  assert.equal(await page.getByTestId('state-diff').getAttribute('data-applied'), 'true');
  await page.getByTestId('detail-toggle').click();
  await page.keyboard.press('Escape');
  assert.equal(await page.getByTestId('inspector').isVisible(), false);
  checks.push('Unrelated model uses same offline viewer, custom facts, generic topology and all three diagrams');
  await page.goto(`${origin}#mode=invalid&view=invalid&speed=Infinity&step=900`);
  await phase('ready');
  assert.equal(await page.getByTestId('view-relations').getAttribute('aria-pressed'), 'true');
  assert.equal(await page.getByTestId('speed-select').inputValue(), '0.5');
  assert.deepEqual(errors, []);
  assert(requests.filter(url => /^https?:/.test(url)).every(url => new URL(url).origin === new URL(origin).origin));
  await writeFile(join(evidence, 'result.json'), JSON.stringify({ result: 'passed', browser: browser.version(), checks, errors }, null, 2));
} catch (error) {
  if (page && !page.isClosed()) await capture('failure');
  await writeFile(join(evidence, 'failure.json'), JSON.stringify({ message: error.message, checks, errors }, null, 2));
  throw error;
} finally {
  await browser.close();
  await writeFile(join(evidence, 'browser-closed.json'), JSON.stringify({ closed: true }));
}
