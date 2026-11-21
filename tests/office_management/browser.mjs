/** Actual embedded office consumer, no API interception or injected application state. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { checkpoint } from '../collaboration_workbench/browser_checkpoint.mjs';

const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
const origin = new URL(config.url);
assert.equal(origin.hostname, '127.0.0.1');
assert(Number(origin.port) > 1024 && config.generation && config.paths);
await mkdir(config.evidence, { recursive: false });
const require = createRequire(import.meta.url);
const driverPath = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
const { chromium } = require(driverPath);
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',
  headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
const page = await context.newPage();
page.setDefaultTimeout(25000);
const steps = [], traffic = [], errors = [];
let stage = 'known-offices', failure;
page.on('pageerror', error => errors.push(error.message));
page.on('request', request => {
  const url = new URL(request.url());
  if (url.origin !== origin.origin) errors.push(`External request ${url}`);
  if (url.pathname.startsWith('/api/')) traffic.push({ kind: 'request', method: request.method(), url: request.url(), body: request.postData() });
});
page.on('response', response => {
  if (new URL(response.url()).pathname.startsWith('/api/')) traffic.push({ kind: 'response', status: response.status(), url: response.url() });
});
const button = (name, root = page) => root.getByRole('button', { name, exact: true });
const destination = name => page.getByRole('navigation', { name: '管理台导航' }).getByRole('link', { name, exact: true });
async function until(description, observe) {
  const end = Date.now() + 30000;
  while (Date.now() < end) {
    const value = await observe();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 150));
  }
  throw new Error(`Timed out: ${description}`);
}
async function api(path, query = {}) {
  const url = new URL(path, origin);
  for (const [key, value] of Object.entries({ generation: config.generation, ...query })) if (value != null) url.searchParams.set(key, value);
  const response = await context.request.get(url.href);
  const body = await response.json();
  assert.equal(response.status(), 200, JSON.stringify(body));
  return body;
}
async function agents() { return (await api('/api/v1/agents')).agents; }
async function receipt(id) {
  return until(`operation ${id}`, async () => {
    const operation = await api(`/api/v1/operations/${id}`);
    if (['Failed', 'Unknown'].includes(operation.status)) throw new Error(JSON.stringify(operation));
    return operation.status === 'Succeeded' ? operation : null;
  });
}
async function submit(control, path) {
  const pending = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === path);
  await control.click();
  const response = await pending;
  const admitted = await response.json();
  assert.equal(response.status(), 202, JSON.stringify(admitted));
  return { body: response.request().postDataJSON(), operation: await receipt(admitted.request_id) };
}
async function capture(id, meaning, facts) {
  steps.push(await checkpoint(page, config.evidence, id, meaning, facts));
  await writeFile(`${config.evidence}/steps.json`, JSON.stringify(steps, null, 2) + '\n');
}
async function visibleText(text) {
  await until(`visible ${text}`, async () => (await page.locator('body').innerText()).includes(text));
}
async function selectAgent(name) {
  await destination('Agents').click();
  const filters = page.locator('.agent-filter-options');
  if (await filters.getAttribute('open') === null) await filters.locator('summary').click();
  await page.getByRole('combobox', { name: '办公室筛选', exact: true }).selectOption('all');
  await page.locator('.agent-row').filter({ has: page.locator('strong', { hasText: new RegExp(`^${name}$`) }) }).click();
  await button('消息').click();
}
async function closeOperations() {
  const control = button('返回工作页');
  if (await control.isVisible()) await control.click();
}
async function messages(reference) {
  return (await api(`/api/v1/agents/${reference.name}/messages`, { incarnation: reference.incarnation, stopped_event: reference.stopped_event })).messages;
}

try {
  await page.goto(config.url, { waitUntil: 'domcontentloaded' });
  await destination('工作区').click();
  const officeQuery = page.getByRole('combobox', { name: '岗位查询办公室', exact: true });
  await officeQuery.waitFor();
  await officeQuery.fill(config.paths.empty);
  await button('查看岗位').click();
  assert.equal(await officeQuery.inputValue(), config.paths.empty);
  await visibleText('controller');
  await page.getByText('发现不完整', { exact: false }).waitFor();
  const catalog = await api('/api/v1/workspaces');
  assert(catalog.workspaces.some(row => row.path === config.paths.empty));
  assert(!catalog.workspaces.some(row => row.path === config.paths.bare));
  assert.deepEqual(await agents(), []);
  assert(!traffic.some(row => row.method === 'POST'), 'Office navigation caused a mutation');
  await capture('OB01-empty-office', 'Known empty office exposes inherited posts, coverage and launch action before any Agent exists.', catalog);

  stage = 'catalog-error-recovery';
  const queryOffice = page.getByRole('combobox', { name: '岗位查询办公室', exact: true });
  await queryOffice.fill(`${config.paths.cwd}/does-not-exist`);
  const refusedCatalog = page.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/roles' && response.status() >= 400);
  await button('查看岗位').click();
  const refused = await refusedCatalog;
  const refusedBody = await refused.json();
  assert(refusedBody.error?.message);
  await visibleText(refusedBody.error.message);
  await capture('OB01-catalog-error', 'Invalid explicit office lookup has a local actionable error and no mutation; known-office navigation remains usable.', refusedBody);

  stage = 'role-workspace-cwd-launch';
  const childCatalog = page.waitForResponse(response => { const url = new URL(response.url()); return url.pathname === '/api/v1/roles' && url.searchParams.get('workspace') === config.paths.child && response.status() === 200; });
  await officeQuery.fill(config.paths.child);
  await button('查看岗位').click();
  await childCatalog;
  await button('启动岗位 reviewer').click();
  const dialog = page.getByRole('dialog');
  await dialog.waitFor();
  await dialog.getByRole('button', { name: /^位置与标题/ }).click();
  assert.equal(await dialog.getByRole('textbox', { name: '启动岗位', exact: true }).inputValue(), 'reviewer');
  assert.equal(await dialog.getByRole('textbox', { name: '启动办公室', exact: true }).inputValue(), config.paths.child);
  await dialog.getByRole('textbox', { name: '启动岗位', exact: true }).fill('global:reviewer');
  await dialog.getByRole('textbox', { name: '启动办公室', exact: true }).fill(config.paths.explicit);
  await dialog.getByRole('textbox', { name: '启动目录', exact: true }).fill(config.paths.cwd);
  await dialog.getByLabel('职责标题', { exact: true }).fill('office-browser-launch');
  await dialog.getByLabel('初始任务', { exact: true }).fill('office browser real hook task');
  const launch = await submit(button('启动', dialog), '/api/v1/agents');
  assert.equal(launch.body.launch.workspace, config.paths.explicit);
  assert.equal(launch.body.launch.directory, config.paths.cwd);
  assert.equal(launch.body.launch.role, 'global:reviewer');
  assert(!launch.body.launch.tool && !launch.body.launch.profile);
  const name = launch.operation.result.Launched.agents[0];
  const live = await until('ready Role-launched Agent', async () => (await agents()).find(row => row.agent.name === name && row.can_send && row.session_id));
  assert.equal(live.collaboration.workspace, config.paths.explicit);
  assert.equal(live.collaboration.role.path, `${config.scope}/roles/reviewer.toml`);
  assert.equal(live.collaboration.role.namespace, `global:${config.scope}/roles`);
  assert.equal(live.directory, config.paths.cwd);
  await closeOperations();
  await selectAgent(name);
  const composer = page.getByRole('textbox', { name: '消息内容', exact: true });
  await composer.fill('exact member draft survives office browsing');
  await destination('工作区').click();
  await capture('OB02-independent-launch', 'Deliberate post launch preserves independent membership, Role origin and cwd; native receipt identifies the execution.', { launch, live });

  stage = 'directory-preview-contact';
  await page.getByRole('textbox', { name: '路由目标目录', exact: true }).fill(config.paths.nested);
  const postsBefore = traffic.filter(row => row.method === 'POST').length;
  const previewResponse = page.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/directory-entry');
  await button('预览路由').click();
  const previewHttp = await previewResponse;
  assert.equal(previewHttp.status(), 200);
  const preview = await previewHttp.json();
  assert.equal(preview.requested_workspace, config.paths.child);
  assert.equal(preview.resolved_workspace, config.paths.project);
  assert.equal(preview.endpoint.status, 'absent');
  assert.equal(traffic.filter(row => row.method === 'POST').length, postsBefore);
  assert.equal((await agents()).length, 1);
  const text = 'office browser deliberate directory contact';
  await page.getByRole('textbox', { name: '目录消息内容', exact: true }).fill(text);
  const contact = await submit(button('向目录发送'), '/api/v1/directory-messages');
  assert.equal(contact.body.directory, config.paths.nested);
  assert.equal(contact.body.message.text, text);
  const sent = contact.operation.result.DirectorySent;
  assert.equal(sent.entry.resolved_workspace, config.paths.project);
  assert.equal(sent.created, true);
  assert.deepEqual(sent.sent.recipients, [sent.recipient.name]);
  await until('native directory reply', async () => (await messages(sent.recipient)).some(row => row.reply_to === sent.sent.message_id));
  await button('查看回执').click();
  await visibleText(sent.recipient.name);
  await visibleText(sent.sent.message_id);
  await capture('OB03-directory-receipt', 'Pure preview followed by deliberate directory contact shows actual route, exact recipient and native message receipt.', { preview, contact, messages: await messages(sent.recipient) });

  stage = 'draft-selection-continuity';
  await button(`查看 @${sent.recipient.name}`).click();
  await visibleText(`fixture reply #${sent.sent.message_id}`);
  await capture('OB03-recipient-reply', 'From the directory receipt, the actual recipient is selectable and its native reply is readable in the real conversation UI.', { recipient: sent.recipient, message_id: sent.sent.message_id });
  await selectAgent(name);
  await composer.waitFor();
  assert.equal(await composer.inputValue(), 'exact member draft survives office browsing');
  await page.reload({ waitUntil: 'domcontentloaded' });
  await composer.waitFor();
  assert.equal(await composer.inputValue(), 'exact member draft survives office browsing');
  await until('poll continuation', async () => traffic.filter(row => row.kind === 'request' && row.url.includes(`/agents/${name}?`)).length >= 3);
  assert.equal(await composer.inputValue(), 'exact member draft survives office browsing');
  await capture('OB04-draft-continuity', 'Exact member selection and unsent message survive office work, operation disclosure, reload and polling.', live.agent);

  stage = 'narrow-office-path';
  await page.setViewportSize({ width: 390, height: 844 });
  await destination('工作区').click();
  await page.getByRole('textbox', { name: '路由目标目录', exact: true }).waitFor();
  await button('预览路由').scrollIntoViewIfNeeded();
  assert(await button('预览路由').isVisible());
  await button('向目录发送').scrollIntoViewIfNeeded();
  assert(await button('向目录发送').isVisible());
  const geometry = await page.evaluate(() => ({ width: innerWidth, scrollWidth: document.documentElement.scrollWidth, height: innerHeight, scrollHeight: document.documentElement.scrollHeight }));
  assert(geometry.scrollWidth <= geometry.width + 1, JSON.stringify(geometry));
  await capture('OB05-mobile-office', 'Same office route and deliberate contact actions remain reachable at 390px without horizontal page overflow.', geometry);
  assert.deepEqual(errors, []);
} catch (error) {
  failure = { stage, message: error.message, stack: error.stack };
  try { await capture('failure', 'Failed checkpoint; no acceptance claimed.', failure); } catch (captureError) { failure.captureError = captureError.message; }
} finally {
  await writeFile(`${config.evidence}/traffic.json`, JSON.stringify(traffic, null, 2) + '\n');
  await writeFile(`${config.evidence}/report.json`, JSON.stringify({ status: failure ? 'fail' : 'pass', failure, errors, steps,
    driver: { playwright: require(`${driverPath}/package.json`).version, chromium: browser.version() },
    limits: 'Actual embedded candidate page and public native HTTP; local fixture Provider. Physical mobile and paid Provider unverified. Existing lifecycle browser remains separate regression.' }, null, 2) + '\n');
  await context.tracing.stop({ path: `${config.evidence}/trace.zip` });
  await context.close();
  await browser.close();
}
if (failure) { console.error(JSON.stringify(failure)); process.exitCode = 1; }
