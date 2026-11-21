/** Actual embedded Space direct navigation; no synthetic domain/API responses. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { checkpoint } from '../collaboration_workbench/browser_checkpoint.mjs';

const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
const origin = new URL(config.url);
assert.equal(origin.hostname, '127.0.0.1');
assert(Number(origin.port) > 1024 && ![18967, 18971, 38023].includes(Number(origin.port)));
const require = createRequire(import.meta.url);
const driver = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
const { chromium } = require(driver);
await mkdir(config.evidence, { recursive: false });
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',
  headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
const page = await context.newPage(), traffic = [], errors = [], steps = [];
let frame, failure, stage = 'open';
page.setDefaultTimeout(20000);
page.on('pageerror', error => errors.push(error.message));
page.on('request', request => {
  const url = new URL(request.url());
  if (!['http:', 'https:'].includes(url.protocol)) return;
  assert.equal(url.origin, origin.origin, `unexpected external request ${url}`);
  if (url.pathname.startsWith('/api/')) traffic.push({ method: request.method(), url: request.url(), body: request.postData(), at: Date.now() });
});
async function until(label, observe, timeout = 35000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const value = await observe();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 120));
  }
  throw new Error(label + ' timed out');
}
const relation = () => frame.getByRole('region', { name: 'Agent 启动关系', exact: true });
const composer = () => frame.getByRole('textbox', { name: 'Agent 消息草稿', exact: true });
const action = name => relation().getByRole('button', { name: `打开 ${name} 的精确会话`, exact: true });
async function target() {
  const id = randomUUID();
  return page.evaluate(id => new Promise((resolve, reject) => {
    const timer = setTimeout(() => { window.removeEventListener('message', receive); reject(new Error('getTarget timeout')); }, 5000);
    function receive(event) {
      if (event.source !== document.querySelector('iframe').contentWindow || event.data?.requestId !== id) return;
      clearTimeout(timer); window.removeEventListener('message', receive); resolve(event.data);
    }
    window.addEventListener('message', receive);
    document.querySelector('iframe').contentWindow.postMessage({ protocol: 'agentx.context.v1', type: 'getTarget', requestId: id }, location.origin);
  }), id);
}
async function selected(reference) {
  return until('selected exact ' + reference.name, async () => {
    const value = await target();
    if (value.status !== 'selected' || ['name', 'incarnation', 'stopped_event'].some(key => value.target.agent[key] !== reference[key])) return false;
    assert.equal(value.target.generation, config.generation);
    assert.equal(value.target.scope, config.scope);
    return value;
  });
}
async function capture(id, meaning, state = {}) {
  const child = { target: await target(), draft: await composer().inputValue({ timeout: 1000 }).catch(() => null), semantic: await frame.locator('body').ariaSnapshot(),
    geometry: await frame.evaluate(() => ({ width: innerWidth, height: innerHeight, scrollWidth: document.documentElement.scrollWidth })) };
  await writeFile(`${config.evidence}/${id}-child.json`, JSON.stringify(child, null, 2) + '\n');
  await frame.locator('body').screenshot({ path: `${config.evidence}/${id}-child.png` });
  steps.push(await checkpoint(page, config.evidence, id, meaning, { ...state, child }));
  await writeFile(`${config.evidence}/steps.json`, JSON.stringify(steps, null, 2) + '\n');
}
async function go(label) {
  const reference = config.refs[label];
  await action(reference.name).click();
  if (config.phase === 'stopped') {
    await frame.locator('.conversation-heading-copy').getByText(new RegExp(`@${reference.name} ·`)).waitFor();
  } else await selected(reference);
  await relation().waitFor();
  return reference;
}
async function office(path) {
  const select = frame.getByRole('combobox', { name: '工作区范围', exact: true });
  assert.equal(await select.inputValue(), path);
}
try {
  const start = config.phase === 'stopped' ? config.refs.D : config.phase === 'reset' ? config.refs['B-resumed'] : config.refs.A;
  const query = new URLSearchParams({ scope: config.scope, generation: config.generation, agent: start.name, view: 'chat', layout: 'embedded' });
  for (const [key, value] of Object.entries(start)) if (key !== 'name' && value != null) query.set(key, value);
  // Only the harness host document is provided locally. All iframe resources
  // and API responses come from the immutable candidate's real listener.
  await page.route('**/acceptance-parent', route => route.fulfill({ contentType: 'text/html', body:
    '<!doctype html><html><body style="margin:0"><iframe title="AgentX Space" style="border:0;width:100vw;height:100vh"></iframe></body></html>' }));
  await page.goto(`${origin.origin}/acceptance-parent`);
  await page.locator('iframe').evaluate((element, src) => { element.src = src; }, `${origin.origin}/space/?${query}`);
  frame = await until('candidate child frame', async () => page.frames().find(value => value.url().startsWith(`${origin.origin}/space/`)));
  await composer().waitFor();
  if (config.phase === 'stopped') await frame.locator('.conversation-heading-copy').getByText(new RegExp(`@${start.name} ·`)).waitFor();
  else await selected(start);
  if (config.phase === 'tree') {
    stage = 'repeated-tree-navigation';
    await relation().getByText('外部启动', { exact: true }).waitFor();
    await composer().fill('A exact draft');
    await go('B');
    await office(config.offices.project);
    await composer().fill('B exact draft');
    await go('D');
    await office(config.offices.child);
    await composer().fill('D exact draft');
    await capture('R04-downstream', 'A→B→D opens actual child office and exact D without mutations.');
    await go('B');
    assert.equal(await composer().inputValue(), 'B exact draft');
    await go('A');
    assert.equal(await composer().inputValue(), 'A exact draft');
    await capture('R04-upstream', 'D→B→A restores exact drafts and parent office.');
    await go('B'); await go('D'); await go('B'); await go('A');
    assert.equal(await composer().inputValue(), 'A exact draft');
    stage = 'narrow-embedded';
    await page.setViewportSize({ width: 390, height: 844 });
    await go('B'); await go('D');
    assert.equal(await composer().inputValue(), 'D exact draft');
    await composer().scrollIntoViewIfNeeded();
    assert(await composer().isVisible());
    const size = await frame.evaluate(() => ({ width: innerWidth, scrollWidth: document.documentElement.scrollWidth }));
    assert(size.scrollWidth <= size.width + 1, JSON.stringify(size));
    await action(config.refs.B.name).scrollIntoViewIfNeeded();
    assert(await action(config.refs.B.name).isVisible());
    await capture('R11-narrow', 'The same complete iframe supports narrow repeated exact navigation and retained D draft.', size);
    stage = 'relation-read-failure';
    await page.route('**/api/v1/agents/*/launch-relations?*', route => route.abort('failed'));
    await relation().getByRole('alert').waitFor();
    assert.equal(await relation().getByRole('button', { name: /^打开 / }).count(), 0, 'failed read retained stale actionable targets');
    await capture('R11-error', 'Failed actual relation reads expose local retry and remove stale actions; draft remains.');
    await page.unroute('**/api/v1/agents/*/launch-relations?*');
    await relation().getByRole('button', { name: '重试', exact: true }).click();
    await go('B');
    assert.equal(await composer().inputValue(), 'B exact draft');
    await capture('R11-recovered', 'Retry recovers actual relation navigation and preserved B draft.');
  } else if (config.phase === 'stopped') {
    stage = 'stopped-upstream';
    const old = config.refs['B-stopped'];
    await go('B-stopped');
    assert(old.stopped_event && !old.incarnation);
    await frame.locator('.conversation-heading-copy').getByText(/停止历史/).waitFor();
    await go('A'); await go('B-stopped'); await go('D'); await go('B-stopped');
    await capture('R05-stopped-exact', 'Stopped B remains the exact upstream/downstream historical conversation on repeated navigation.');
  } else {
    stage = 'delayed-old-generation-read';
    await action(config.refs.A.name).waitFor();
    await composer().fill('old generation draft');
    let held, release;
    const waiting = new Promise(resolve => { release = resolve; });
    await page.route('**/api/v1/agents/*/launch-relations?*', async route => {
      if (held) { await route.continue(); return; }
      const response = await route.fetch();
      held = { url: route.request().url(), status: response.status(), body: await response.json() };
      await waiting;
      await route.fulfill({ response });
    });
    await until('real old-generation relation response held', async () => held);
    assert.equal(held.status, 200);
    assert.equal(held.body.generation, config.generation);
    await capture('R10-before-reset', 'Real successful old-generation relation response is held before native disposable reset.', { held });
    assert(config.environment.AGENTX_DIR === config.scope && config.scope.includes('agentx-management-'));
    const reset = await promisify(execFile)(config.candidate, ['reset'], { env: config.environment, cwd: config.cwd, timeout: 30000 });
    const next = await until('new management generation', async () => {
      const response = await context.request.get(`${origin.origin}/api/v1/management`);
      const value = await response.json();
      return value.generation !== config.generation ? value : null;
    });
    await until('reset clears old exact browser target', async () => (await target()).status === 'unavailable');
    release();
    await page.unroute('**/api/v1/agents/*/launch-relations?*');
    await until('retired composer removed', async () => await composer().count() === 0);
    assert.equal(await relation().count(), 0);
    assert.equal((await target()).status, 'unavailable');
    await capture('R10-after-reset', 'Late real old-generation relation response cannot restore retired exact conversation or actions.', { next, reset, held });
  }
  assert.equal(traffic.filter(value => value.method !== 'GET').length, 0, 'selection/navigation caused native mutation');
  assert.deepEqual(errors, []);
} catch (error) {
  failure = { stage, message: error.message, stack: error.stack };
  try { if (frame) await capture('failure', 'Failed checkpoint, not accepted evidence.', failure); }
  catch (error) { failure.captureError = error.message; }
} finally {
  await writeFile(`${config.evidence}/traffic.json`, JSON.stringify(traffic, null, 2) + '\n');
  await writeFile(`${config.evidence}/report.json`, JSON.stringify({ status: failure ? 'fail' : 'pass', failure, errors, steps,
    browser: browser.version(), playwright: require(`${driver}/package.json`).version }, null, 2) + '\n');
  await context.tracing.stop({ path: `${config.evidence}/trace.zip` });
  await context.close(); await browser.close();
}
if (failure) { console.error(JSON.stringify(failure)); process.exitCode = 1; }
