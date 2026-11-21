/** Actual Space browser, intercepted disposable API: uncertainty remains visible and never replays. */
import assert from 'node:assert/strict';
import {mkdir, writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
const [url, evidence] = process.argv.slice(2);
if (!url || !evidence) throw new Error('Usage: node space.browser.mjs STATIC_SPACE_ORIGIN NEW_EVIDENCE_DIRECTORY');
await mkdir(evidence, {recursive: false});
const {chromium} = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--no-proxy-server', '--disable-dev-shm-usage']});
const page = await browser.newPage({viewport: {width: 1440, height: 950}});
const traffic = [], errors = [], checks = [];
let operation, failure;
const generation = 'independent-space', scope = '/disposable/space';
page.on('pageerror', error => errors.push(error.message));
await page.route('**/api/**', async route => {
  const request = route.request(), path = new URL(request.url()).pathname;
  traffic.push({method: request.method(), path});
  let body;
  if (request.method() === 'POST') {
    assert.equal(path, '/api/v1/agents', 'no other fixture mutation is permitted');
    const input = request.postDataJSON();
    assert.equal(input.generation, generation);
    assert(await page.evaluate(id => (localStorage.getItem('agentx.space.operations.v1') ?? '').includes(id), input.request_id), 'persist original receipt before submission');
    operation = {generation, request_id: input.request_id, status: 'Unknown', error: 'worker spawn: executable=/fixture/host cwd=/fixture/missing', result: null};
    body = operation;
  } else if (path === '/api/v1/management') body = {generation, scope, project: '/fixture', tools: ['codex'], profiles: [{name: 'fixture', tool: 'codex', description: 'finite intercepted fixture'}], max_page: 100, max_text: 100000};
  else if (path === '/api/v1/agents') body = {generation, agents: [], next: null, current_complete: true};
  else if (path === '/api/v1/workspaces') body = {generation, workspaces: [], coverage: {agent_pages: 1, agent_limit_reached: false, directories_examined: 1, scan_limit_reached: false, errors: []}};
  else if (path === '/api/v1/roles') body = {generation, workspace: null, roles: []};
  else if (path === '/api/v1/office-activity') body = {generation, next_after: '0', has_more: false, messages: [], created: [], human_messages: [], human_has_more: false, human_unhandled_count: 0};
  else if (path.startsWith('/api/v1/operations/')) {
    assert.equal(path.split('/').at(-1), operation.request_id, 'query only original operation ID');
    body = operation;
  } else { await route.abort(); return; }
  await route.fulfill({status: request.method() === 'POST' ? 202 : 200, contentType: 'application/json', body: JSON.stringify(body)});
});
try {
  await page.goto(url + '/space/');
  await page.getByRole('button', {name: '＋ Agent', exact: true}).click();
  const dialog = page.getByRole('dialog', {name: '新建 Agent', exact: true});
  await dialog.getByRole('textbox', {name: '新 Agent 任务', exact: true}).fill('finite browser fixture, never a Provider launch');
  await dialog.getByRole('button', {name: '启动', exact: true}).click();
  await dialog.getByRole('alert').filter({hasText: '结果未确定'}).waitFor();
  assert(await dialog.isVisible(), 'uncertain operation must not close dialog as if successful');
  assert(await dialog.getByRole('button', {name: '启动', exact: true}).isDisabled());
  assert.equal(traffic.filter(row => row.method === 'POST').length, 1);
  checks.push('Unknown launch leaves dialog open with actionable native error and disables repeat submission.');
  await dialog.getByRole('button', {name: '关闭操作对话框', exact: true}).click();
  const visibleError = page.locator('.live-status > .live-error').filter({hasText: 'worker spawn'});
  assert(await visibleError.isVisible(), 'original error visible outside collapsed receipt details');
  assert.equal(await page.locator('.live-receipts').getAttribute('open'), null);
  await page.screenshot({path: `${evidence}/unknown.png`, fullPage: true});
  await page.reload();
  await page.locator('.live-status > .live-error').filter({hasText: 'worker spawn'}).waitFor();
  assert.equal(traffic.filter(row => row.method === 'POST').length, 1);
  checks.push('Reload restores original Unknown/error visibly without another POST.');
  operation = {...operation, status: 'Failed'};
  const before = traffic.filter(row => row.path.startsWith('/api/v1/operations/')).length;
  await page.locator('.live-status > .live-error').filter({hasText: 'worker spawn'}).getByRole('button', {name: '查询原回执', exact: true}).click();
  await page.locator('.live-status > .live-error').filter({hasText: '操作失败'}).waitFor();
  assert(traffic.filter(row => row.path.startsWith('/api/v1/operations/')).length > before);
  assert.equal(traffic.filter(row => row.method === 'POST').length, 1);
  await page.screenshot({path: `${evidence}/failed.png`, fullPage: true});
  checks.push('Explicit original-ID GET reconciliation updates Failed feedback without replay.');
  assert.deepEqual(errors, []);
} catch (error) { failure = {message: error.message, stack: error.stack}; }
finally {
  await writeFile(`${evidence}/result.json`, JSON.stringify({status: failure ? 'failed' : 'passed', boundary: 'Actual freshly built Space/browser; API fixtures intercepted before network; no Provider or native mutation.', browser: browser.version(), checks, traffic, operation, errors, failure}, null, 2) + '\n');
  await browser.close();
}
if (failure) throw new Error(failure.message);
