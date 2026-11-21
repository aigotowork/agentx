/** Exercise the explanatory artifact without calling AgentX or any Provider. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { join, resolve } from 'node:path';

const [origin, evidence, phase = 'all'] = process.argv.slice(2);
assert(origin && evidence, 'Usage: node tests/runtime_atlas_browser.mjs ORIGIN EVIDENCE_DIR');
assert(['all', 'remaining', 'presentation'].includes(phase), 'Unknown acceptance phase');
const root = fileURLToPath(new URL('../', import.meta.url));
const artifact = join(root, '.docs/architecture/runtime-atlas/index.html');
const artifactSha256 = createHash('sha256').update(await readFile(artifact)).digest('hex');
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
  const record = await page.evaluate(() => ({ url: location.href, viewport: { width: innerWidth, height: innerHeight },
    documentWidth: document.documentElement.scrollWidth, focus: document.activeElement?.getAttribute('data-testid'),
    facts: document.querySelector('[data-testid="facts"]')?.textContent,
    step: document.querySelector('[data-testid="step-detail"]')?.textContent,
    inspector: document.querySelector('[data-testid="inspector"]')?.textContent }));
  await writeFile(join(evidence, `${name}.json`), JSON.stringify(record, null, 2));
}

async function select(mode, scenario) {
  await page.getByTestId('mode-select').selectOption(mode);
  if (scenario) await page.getByTestId('scenario-select').selectOption(scenario);
}

function nodeHeader(id) {
  // The group's center may be a contained, independently selectable module.
  return page.getByTestId(`map-node-${id}`).locator('.map-node__title');
}

async function finish(scenario) {
  const count = model.scenarios.find(value => value.id === scenario).steps.length;
  for (let index = 0; index < count; index++) {
    await page.getByTestId('step-next').click();
    await applied();
  }
  assert(await page.getByTestId('step-next').isDisabled(), 'Scenario end must stop advancement');
}

async function applied() {
  await page.locator('[data-testid="event-phase"][data-phase="applied"]').waitFor({ timeout: 18000 });
}

try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, reducedMotion: 'reduce' });
  page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => requests.push(request.url()));
  await page.goto(origin);
  await page.getByTestId('mode-select').waitFor();
  assert.equal(await page.getByTestId('mode-select').inputValue(), 'appserver');
  assert.equal(await page.getByTestId('scenario-select').inputValue(), 'appserver-disconnect');
  if (phase !== 'remaining') {
  await nodeHeader('daemon').click();
  assert.match(await page.getByTestId('inspector').innerText(), /SQLite|数据库/);
  await page.getByTestId('module-daemon-inbox').focus();
  await page.keyboard.press('Enter');
  assert.match(await page.getByTestId('inspector').innerText(), /inbox_store|claim|批次/);
  await page.getByTestId('map-edge-bridge-native').locator('.map-edge__label').click();
  assert.match(await page.getByTestId('inspector').innerText(), /WebSocket/);
  checks.push('Process/module/transport selection, keyboard module activation and source provenance');
  await capture('01-overview');

  await finish('appserver-disconnect');
  assert.match(await page.getByTestId('map-node-bridge').textContent(), /退出/);
  assert.doesNotMatch(await page.getByTestId('map-node-codex').textContent(), /已退出|已终止/);
  assert.match(await page.getByTestId('facts').innerText(), /未知|待重新观察|推断/);
  assert.match(await page.getByTestId('facts').innerText(), /待收|等待有效宿主/);
  await page.getByTestId('step-prev').click();
  assert.match(await page.getByTestId('step-detail').innerText(), /关闭本条原生连接|WebSocket/);
  await page.getByTestId('step-next').click();
  await applied();
  const savedURL = page.url();
  await page.reload();
  assert.equal(page.url(), savedURL);
  assert(await page.getByTestId('step-next').isDisabled());
  checks.push('Disconnect retains backend, preserves unknown turn and pending inbox; previous and deep-link reload reconstruct state');
  await capture('02-disconnect');

  if (phase === 'presentation') {
    const geometry = await page.evaluate(() => ({
      height: innerHeight,
      actions: document.querySelector('[data-testid="step-detail"]').getBoundingClientRect().toJSON(),
      facts: [...document.querySelectorAll('.fact-value')].map(item => item.getBoundingClientRect().toJSON())
    }));
    assert(geometry.actions.bottom <= geometry.height, 'Current event dock must fit the desktop workspace');
    assert(geometry.facts.every(item => item.bottom <= geometry.height), 'All five facts must fit the desktop workspace');
    checks.push('Desktop event actions and all five facts visible together at 1440x900');
  }
  }

  if (phase === 'all') {
  for (const mode of model.modes) {
    await select(mode.id);
    for (const id of mode.nodeIds) assert.equal(await page.getByTestId(`map-node-${id}`).count(), 1);
    for (const scenario of model.scenarios.filter(value => value.modeId === mode.id)) {
      await page.getByTestId('scenario-select').selectOption(scenario.id);
      await finish(scenario.id);
      await page.getByTestId('step-reset').click();
      assert(await page.getByTestId('step-prev').isDisabled());
    }
  }
  checks.push('All five modes and fourteen source-backed scenarios are reachable, finite and resettable');

  await select('headless', 'headless-close');
  await page.getByTestId('step-next').click();
  await applied();
  await page.getByTestId('step-next').click();
  await applied();
  assert.match(await page.getByTestId('map-node-terminal').textContent(), /关闭/);
  assert.match(await page.getByTestId('map-node-pty').textContent(), /保留/);
  checks.push('Headless initiating terminal closes while the detached PTY remains');
  }

  const narrow = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, reducedMotion: 'reduce' });
  const desktop = page;
  page = await narrow.newPage();
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(origin);
  await nodeHeader('bridge').tap();
  assert.match(await page.getByTestId('inspector').innerText(), /协议桥/);
  await page.getByTestId('step-next').tap();
  await applied();
  const geometry = await page.evaluate(() => ({ viewport: innerWidth, width: document.documentElement.scrollWidth }));
  assert(geometry.width <= geometry.viewport + 1, `Document overflows horizontally: ${JSON.stringify(geometry)}`);
  // Advancing dismisses secondary disclosure; reopen the now-selected actor.
  await page.getByTestId('detail-toggle').tap();
  assert.match(await page.getByTestId('inspector').innerText(), /App Server 客户端/);
  await capture('03-narrow-touch');
  await page.getByTestId('inspector').scrollIntoViewIfNeeded();
  const inspectorBox = await page.getByTestId('inspector').boundingBox();
  assert(inspectorBox && inspectorBox.x >= 0 && inspectorBox.x + inspectorBox.width <= 391, 'Narrow local inspector is reachable and contained');
  await capture('04-narrow-inspector');
  checks.push('390px touch process selection and scenario advance; document width remains contained');
  await narrow.close();

  page = desktop;
  await page.goto(`${origin}#mode=invalid&scenario=invalid&step=999&node=invalid`);
  assert.equal(await page.getByTestId('mode-select').inputValue(), 'appserver');
  assert.equal(await page.getByTestId('scenario-select').inputValue(), 'appserver-disconnect');
  assert(await page.getByTestId('step-prev').isDisabled());
  await nodeHeader('bridge').click();
  await page.getByRole('button', { name: /^复制 / }).first().click();
  await page.getByText('已复制', { exact: true }).waitFor();
  checks.push('Invalid hash falls back coherently; source copy succeeds on insecure LAN context');
  await page.goto(pathToFileURL(artifact).href);
  await page.getByTestId('mode-select').waitFor();
  await select('tmux', 'tmux-detach');
  await finish('tmux-detach');
  assert.match(await page.getByTestId('facts').innerText(), /同一个 CLI 进程/);
  checks.push('Standalone file:// artifact works offline without module imports or fetch');
  await page.getByTestId('step-reset').click();
  await page.getByTestId('step-play').click();
  await page.getByTestId('step-counter').filter({ hasText: '2 / 3' }).waitFor({ timeout: 12000 });
  await page.getByTestId('step-play').click();
  assert.equal(await page.getByTestId('step-play').getAttribute('aria-pressed'), 'false');
  await page.getByTestId('module-daemon-inbox').focus();
  await page.keyboard.press('Enter');
  assert.match(await page.getByTestId('inspector').innerText(), /inbox_store|claim|批次/);
  checks.push('Continuous playback advances to the next event offline; keyboard module selection opens local detail');
  assert.deepEqual(errors, []);
  assert(requests.filter(url => /^https?:/.test(url)).every(url => new URL(url).origin === new URL(origin).origin), 'No external network assets');
  await writeFile(join(evidence, 'result.json'), JSON.stringify({ result: 'passed', phase, checks, errors, artifactSha256,
    browser: browser.version(), origin,
    artifact: resolve(artifact), limits: 'Actual browser actions on explanatory state only; no AgentX runtime, provider, physical touch or assistive-technology verification.' }, null, 2));
} catch (error) {
  if (page && !page.isClosed()) await capture('failure');
  await writeFile(join(evidence, 'failure.json'), JSON.stringify({ message: error.message, checks, errors }, null, 2));
  throw error;
} finally {
  await browser.close();
  await writeFile(join(evidence, 'browser-closed.json'), JSON.stringify({ closed: true }));
}
