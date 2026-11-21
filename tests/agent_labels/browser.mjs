/** Independent labels paths on the actual candidate; only compatibility omission is simulated. */
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { checkpoint } from '../collaboration_workbench/browser_checkpoint.mjs';

const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
const origin = new URL(config.url);
assert.equal(origin.hostname, '127.0.0.1');
assert(Number(origin.port) > 1024 && config.agent.incarnation && config.generation);
await mkdir(config.evidence, { recursive: false });
const require = createRequire(import.meta.url);
const driverPath = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
const { chromium } = require(driverPath);
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',
  headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
const page = await context.newPage();
page.setDefaultTimeout(20000);
const steps = [], traffic = [], errors = [];
let stage = 'open', failure;
page.on('pageerror', error => errors.push(error.message));
page.on('request', request => {
  assert.equal(new URL(request.url()).origin, origin.origin, `unexpected external request: ${request.url()}`);
  if (new URL(request.url()).pathname.startsWith('/api/')) traffic.push({ kind: 'request', method: request.method(), url: request.url(), body: request.postData() });
});
page.on('response', response => {
  if (new URL(response.url()).pathname.startsWith('/api/')) traffic.push({ kind: 'response', status: response.status(), url: response.url() });
});
const button = (name, root = page) => root.getByRole('button', { name, exact: true });
const labelsPath = `/api/v1/agents/${config.agent.name}/labels`;
const pattern = `**${labelsPath}`;
const journalKey = 'agentx.management.operations.v1';
async function until(description, observe, timeout = 25000) {
  const end = Date.now() + timeout;
  let last;
  while (Date.now() < end) {
    last = await observe();
    if (last) return last;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`${description}: timed out; last=${JSON.stringify(last)}`);
}
async function api(path, query = {}) {
  const url = new URL(path, origin);
  for (const [key, value] of Object.entries({ generation: config.generation, ...query })) if (value != null) url.searchParams.set(key, value);
  const response = await context.request.get(url.href);
  const body = await response.json();
  assert.equal(response.status(), 200, JSON.stringify(body));
  return body;
}
const snapshot = () => api(labelsPath, { incarnation: config.agent.incarnation });
async function receipt(id) {
  return until(`operation ${id}`, async () => {
    const value = await api(`/api/v1/operations/${id}`);
    if (['Failed', 'Unknown'].includes(value.status)) throw new Error(JSON.stringify(value));
    return value.status === 'Succeeded' ? value : null;
  });
}
async function external(set) {
  const payload = { request_id: randomUUID(), generation: config.generation, agent: config.agent, set, remove: [], if_revision: null };
  const response = await context.request.post(new URL(labelsPath, origin).href, { data: payload });
  assert.equal(response.status(), 202, await response.text());
  return receipt(payload.request_id);
}
async function visible(text) {
  await until(`visible ${text}`, async () => (await page.locator('body').innerText()).includes(text));
}
async function closeOperations() {
  const close = button('返回工作页');
  if (await close.isVisible()) await close.click();
}
async function capture(id, meaning, nativeState = {}) {
  const journal = await page.evaluate(key => JSON.parse(localStorage.getItem(key) ?? '[]'), journalKey);
  const drafts = await page.getByRole('textbox', { name: /^标签 (key|value) \d+$/ }).evaluateAll(elements => elements.map(element => ({ name: element.getAttribute('aria-label'), value: element.value })));
  steps.push(await checkpoint(page, config.evidence, id, meaning, { ...nativeState, journal, drafts }));
  await writeFile(`${config.evidence}/steps.json`, JSON.stringify(steps, null, 2) + '\n');
}
async function selectAgent() {
  const row = page.locator('.agent-row').filter({ has: page.locator('strong', { hasText: new RegExp(`^${config.agent.name}$`) }) }).first();
  if (!(await row.isVisible()) && await page.locator('.resource-rail .mobile-picker').isVisible()) await page.locator('.resource-rail .mobile-picker').click();
  await row.click();
  const summary = page.locator('.labels-disclosure > summary');
  await summary.waitFor();
  if (await page.locator('.labels-disclosure').getAttribute('open') === null) await summary.click();
}
async function editor() {
  await closeOperations();
  if (await button('编辑标签').isVisible()) await button('编辑标签').click();
  await button('添加标签').waitFor();
}
async function addRow(key, value) {
  await button('添加标签').click();
  const keys = page.getByRole('textbox', { name: /^标签 key \d+$/ });
  const values = page.getByRole('textbox', { name: /^标签 value \d+$/ });
  await keys.last().fill(key); await values.last().fill(value);
}
async function keyValue(key) {
  const keys = await page.getByRole('textbox', { name: /^标签 key \d+$/ }).all();
  for (let i = 0; i < keys.length; i++) if (await keys[i].inputValue() === key) return page.getByRole('textbox', { name: `标签 value ${i + 1}`, exact: true });
  throw new Error(`draft key missing: ${key}`);
}
async function saveSuccess() {
  const responsePromise = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === labelsPath);
  await button('保存标签').click();
  const response = await responsePromise;
  const body = await response.json();
  assert.equal(response.status(), 202, JSON.stringify(body));
  const operation = await receipt(body.request_id);
  await until('saved draft closes', async () => !(await button('添加标签').isVisible()));
  await closeOperations();
  const current = await snapshot();
  await until('detail refresh displays confirmed labels', async () => {
    const facts = await page.locator('.labels-disclosure .label-fact').allTextContents();
    return Object.entries(current.labels).every(([key, value]) => facts.includes(`${key}=${value}`));
  });
  return operation;
}

try {
  await page.goto(config.url, { waitUntil: 'domcontentloaded' });
  await button('启动 Agent').waitFor();
  await selectAgent();
  const initial = await snapshot();
  await visible('unknown/key=<img src=x onerror=alert(1)>');
  assert.equal(await page.locator('.labels-disclosure img').count(), 0, 'opaque label text became markup');
  stage = 'B01-edit';
  await editor(); await addRow('web/state', '编辑完成');
  const edit = await saveSuccess();
  const saved = await snapshot();
  assert.equal(saved.labels['web/state'], '编辑完成');
  for (const [key, value] of Object.entries(initial.labels)) assert.equal(saved.labels[key], value, `unrelated ${key} lost`);
  await capture('B01-edit', 'Selected exact Agent changes one label through real HTTP and retains every unrelated key; unknown text is inert.', { initial, saved, edit });

  stage = 'B02-server-selection-group';
  await page.locator('.agent-filter-options > summary').click();
  const selection = page.locator('.label-selection');
  await selection.locator('summary').click();
  await page.getByRole('textbox', { name: '标签筛选条件', exact: true }).fill('feature=search');
  await button('应用标签筛选').click();
  await until('actual server selector request', async () => traffic.find(item => item.kind === 'request' && (() => {
    const url = new URL(item.url); return url.pathname === '/api/v1/agents' && url.searchParams.getAll('label').includes('feature=search') && url.searchParams.get('current_only') === 'true';
  })()));
  await page.getByRole('textbox', { name: '标签分组 key', exact: true }).fill('project');
  await visible('project · atlas'); await visible('project · comet'); await visible('仅已加载');
  assert((await page.locator('.label-group h3').allTextContents()).every(text => text.includes('已加载')));
  await button('清除标签筛选').click();
  await visible('project · 缺失');
  await capture('B02-filter-group', 'Native predicates precede paging; one scalar grouping key partitions loaded rows and explicitly labels the count scope.', { groups: await page.locator('.label-group h3').allTextContents() });

  stage = 'B03-revision-conflict';
  await selectAgent(); await editor(); await addRow('web/conflict', '保留草稿');
  const baseline = await snapshot();
  let conflict;
  await page.route(pattern, async route => {
    if (route.request().method() !== 'POST') { await route.continue(); return; }
    const original = route.request().postDataJSON();
    const externalOperation = await external({ 'remote/extra': 'preserve-on-rebase' });
    const response = await route.fetch();
    conflict = { original, externalOperation, status: response.status(), body: await response.json() };
    await route.fulfill({ response });
  }, { times: 1 });
  await button('保存标签').click();
  await until('real server revision conflict', async () => conflict);
  assert.equal(conflict.status, 409, JSON.stringify(conflict));
  await closeOperations();
  assert.equal(await (await keyValue('web/conflict')).inputValue(), '保留草稿');
  assert.equal((await snapshot()).labels['web/conflict'], undefined);
  await capture('B03-conflict-retained', 'Actual external write advances revision; stale save refuses and retains the user draft.', { baseline, conflict, current: await snapshot() });
  await button('刷新并保留草稿').click();
  await visible('已刷新并保留草稿');
  assert.equal(await (await keyValue('web/conflict')).inputValue(), '保留草稿');
  const rebased = await saveSuccess();
  assert.equal((await snapshot()).labels['remote/extra'], 'preserve-on-rebase');
  assert.equal((await snapshot()).labels['web/conflict'], '保留草稿');
  await capture('B03-rebased-save', 'Explicit refresh preserves intended changes plus the remote key; user saves the new revision.', { rebased, snapshot: await snapshot() });

  stage = 'B04-admitted-reply-lost';
  await editor(); await addRow('web/lost', 'one-effect');
  let dropped;
  await page.route(pattern, async route => {
    if (route.request().method() !== 'POST') { await route.continue(); return; }
    const body = route.request().postDataJSON();
    const upstream = await route.fetch();
    assert.equal(upstream.status(), 202);
    dropped = { body, admitted: await upstream.json() };
    await route.abort('failed');
  }, { times: 1 });
  await button('保存标签').click();
  await until('upstream commit before reply loss', async () => dropped);
  await page.reload({ waitUntil: 'domcontentloaded' });
  const recovered = await receipt(dropped.body.request_id);
  await until('browser retrieves original receipt', async () => {
    const journal = await page.evaluate(key => JSON.parse(localStorage.getItem(key) ?? '[]'), journalKey);
    return journal.find(item => item.id === dropped.body.request_id && item.operation?.status === 'Succeeded');
  });
  const repeated = traffic.filter(item => item.kind === 'request' && item.method === 'POST' && new URL(item.url).pathname === labelsPath
    && JSON.parse(item.body).request_id === dropped.body.request_id);
  assert.equal(repeated.length, 1, 'reload repeated admitted labels write');
  assert.equal((await snapshot()).labels['web/lost'], 'one-effect');
  await selectAgent(); await closeOperations();
  await capture('B04-lost-reply-reload', 'Reload recovers the original admitted request and exact target; one write, one committed effect.', { dropped, recovered, snapshot: await snapshot() });

  stage = 'B05-mobile-keyboard';
  await page.setViewportSize({ width: 390, height: 844 });
  await selectAgent(); await editor();
  await addRow('web/mobile', 'keyboard-draft');
  const mobileValue = await keyValue('web/mobile');
  await mobileValue.focus(); await page.keyboard.press('Tab');
  await button('保存标签').scrollIntoViewIfNeeded();
  const geometry = await page.evaluate(() => ({ width: innerWidth, scroll: document.documentElement.scrollWidth,
    focused: document.activeElement?.getAttribute('aria-label'), selected: document.querySelector('.agent-toolbar h1')?.textContent,
    toolbar: (() => { const box = document.querySelector('.agent-toolbar')?.getBoundingClientRect(); return box ? { top: box.top, bottom: box.bottom } : null; })(),
    save: (() => { const box = [...document.querySelectorAll('button')].find(item => item.textContent === '保存标签')?.getBoundingClientRect(); return box ? { top: box.top, bottom: box.bottom } : null; })() }));
  assert(geometry.scroll <= geometry.width + 1, JSON.stringify(geometry));
  assert(geometry.toolbar && geometry.save && geometry.save.top >= geometry.toolbar.top && geometry.save.bottom <= geometry.toolbar.bottom + 1, JSON.stringify(geometry));
  assert.equal(await mobileValue.inputValue(), 'keyboard-draft');
  await capture('B05-mobile', 'Narrow viewport keeps selected labels and draft in the local workspace; keyboard moves among named controls.', geometry);
  await button('放弃草稿').click();

  stage = 'B05-old-service-compatibility';
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.route('**/api/v1/management', async route => {
    const upstream = await route.fetch();
    const body = await upstream.json(); delete body.supports_labels;
    await route.fulfill({ response: upstream, json: body });
  });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await selectAgent();
  await visible('当前服务不支持标签');
  assert.equal(await button('编辑标签').isEnabled(), false);
  await capture('B05-old-service', 'Explicit compatibility simulation omits only supports_labels from an actual descriptor; current UI reports unsupported and disables editing.', { simulation: 'Descriptor.supports_labels omitted in browser response; all other responses actual candidate.' });
  assert.deepEqual(errors, []);
} catch (error) {
  failure = { stage, message: error.message, stack: error.stack };
  try { await capture('failure', 'Retained failing state; no acceptance claim.', failure); } catch (error) { failure.captureError = error.message; }
} finally {
  await writeFile(`${config.evidence}/traffic.json`, JSON.stringify(traffic, null, 2) + '\n');
  await writeFile(`${config.evidence}/report.json`, JSON.stringify({ status: failure ? 'fail' : 'pass', failure, errors, steps,
    driver: { playwright: require(`${driverPath}/package.json`).version, chromium: browser.version() },
    limits: 'Main paths use real embedded UI and native HTTP. Only old-service capability-field omission is simulated. No participants, paid model, physical mobile or screen-reader acceptance.' }, null, 2) + '\n');
  await context.tracing.stop({ path: `${config.evidence}/trace.zip` });
  await context.close(); await browser.close();
}
if (failure) { console.error(JSON.stringify(failure)); process.exitCode = 1; }
