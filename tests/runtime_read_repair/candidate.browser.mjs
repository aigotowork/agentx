/** Read-only independent browser composition against an explicitly disposable native candidate. */
import assert from 'node:assert/strict';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
assert(config.disposable === true, 'configuration must explicitly identify a disposable candidate');
await mkdir(config.evidence, {recursive: false});
const {chromium} = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--no-proxy-server', '--disable-dev-shm-usage']});
const context = await browser.newContext({viewport: {width: 1440, height: 950}});
const traffic = [], errors = [], checks = [];
const byRequest = new WeakMap();
let failure;
// Enforce read-only browser acceptance even if a regression attempted a mutation.
await context.route('**/api/**', route => route.request().method() === 'GET' ? route.continue() : route.abort());
const inspector = await context.newPage();
const listings = () => traffic.filter(row => row.app === 'inspector' && row.path === '/api/v1/agents');
function observe(page, app) {
  page.on('pageerror', error => errors.push({app, error: error.message}));
  page.on('request', request => {
    const url = new URL(request.url());
    if (!url.pathname.startsWith('/api/')) return;
    const row = {app, method: request.method(), path: url.pathname, query: url.search, at: Date.now()};
    byRequest.set(request, row);
    traffic.push(row);
  });
  page.on('response', response => {
    const row = byRequest.get(response.request());
    if (row) { row.status = response.status(); row.responseAt = Date.now(); }
  });
}
observe(inspector, 'inspector');
try {
  const descriptor = await (await context.request.get(config.url + '/api/v1/management')).json();
  assert.equal(descriptor.generation, config.generation);
  assert.equal(descriptor.scope, config.scope);
  const firstResponse = inspector.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/agents');
  await inspector.goto(config.url + '/inspector/');
  await inspector.getByRole('button', {name: '全部（含停止历史）', exact: true}).waitFor();
  const firstPage = await (await firstResponse).json();
  assert(firstPage.agents.length > 0, 'retained native fixture must have current rows');
  const renderedCurrent = inspector.getByText(`已加载 ${firstPage.agents.length} · ${firstPage.next ? '部分覆盖' : '当前成员分页结束'}`, {exact: true});
  await renderedCurrent.waitFor();
  assert.equal(listings().length, 1, 'initial render must not walk all current/history pages');
  assert.equal(new URLSearchParams(listings()[0].query).get('current_only'), 'true');
  checks.push({case: 'native first-page current render', text: await renderedCurrent.innerText(), requestCount: listings().length});
  const historyResponse = inspector.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/agents' && new URL(response.url()).searchParams.get('current_only') === 'false');
  await inspector.getByRole('button', {name: '全部（含停止历史）', exact: true}).click();
  const historicalPage = await (await historyResponse).json();
  await inspector.getByRole('button', {name: '查看当前成员', exact: true}).waitFor();
  await inspector.getByText(/^已加载 \d+ · /).waitFor();
  const firstHistory = listings().find(row => new URLSearchParams(row.query).get('current_only') === 'false');
  assert(firstHistory);
  const more = inspector.getByRole('button', {name: '加载更多记录', exact: true});
  if (await more.count()) {
    const nextResponse = inspector.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/agents' && new URL(response.url()).searchParams.has('after'));
    await more.click();
    await nextResponse;
  }
  const before = listings().filter(row => new URLSearchParams(row.query).get('current_only') === 'false').length;
  await inspector.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/management', {timeout: 8000});
  assert.equal(listings().filter(row => new URLSearchParams(row.query).get('current_only') === 'false').length, before);
  checks.push({case: 'native explicit history and no interval rescan', historyRequests: before, text: await inspector.getByText(/^已加载 \d+ · /).innerText()});
  await inspector.screenshot({path: config.evidence + '/inspector.png', fullPage: true});
  const stopped = historicalPage.agents.find(row => row.agent.stopped_event)?.agent;
  assert(stopped, 'retained native fixture must include a stopped reference outside the current page');
  const direct = await context.newPage();
  observe(direct, 'inspector-direct');
  const exactResponse = direct.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/agents/' + stopped.name);
  await direct.goto(config.url + '/inspector/?ref=' + encodeURIComponent(JSON.stringify([stopped.name, stopped.incarnation, stopped.stopped_event])));
  await exactResponse;
  await direct.getByRole('heading', {name: new RegExp('^' + stopped.name + ' stopped')}).waitFor();
  const directReads = traffic.filter(row => row.app === 'inspector-direct');
  assert.equal(directReads.filter(row => row.path === '/api/v1/agents').length, 1);
  assert(!directReads.some(row => row.path === '/api/v1/agents' && new URLSearchParams(row.query).get('current_only') === 'false'));
  const exact = directReads.find(row => row.path === '/api/v1/agents/' + stopped.name);
  assert.equal(new URLSearchParams(exact.query).get('stopped_event'), stopped.stopped_event);
  checks.push({case: 'native direct stopped URL outside current page', agent: stopped, directoryRequests: 1, historySweep: false});
  await direct.screenshot({path: config.evidence + '/inspector-direct.png', fullPage: true});
  const space = await context.newPage();
  observe(space, 'space');
  const rolesResponse = space.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/roles');
  await space.goto(config.url + '/space/');
  await space.getByRole('button', {name: '＋ Agent', exact: true}).waitFor();
  await rolesResponse;
  assert(traffic.some(row => row.app === 'space' && row.path === '/api/v1/agents' && row.status === 200));
  await space.screenshot({path: config.evidence + '/space.png', fullPage: true});
  checks.push({case: 'native Space normal reads', statuses: traffic.filter(row => row.app === 'space').map(row => ({path: row.path, status: row.status}))});
  assert.deepEqual(errors, []);
  assert(traffic.every(row => row.method === 'GET'));
  assert(!traffic.some(row => row.status >= 400), 'ordinary candidate reads must succeed');
} catch (error) { failure = {message: error.message, stack: error.stack}; }
finally {
  await writeFile(config.evidence + '/result.json', JSON.stringify({status: failure ? 'failed' : 'passed', boundary: 'Actual browser and disposable native candidate; GET-only admission, no Provider launch or live-service writes.', browser: browser.version(), config, traffic, checks, errors, failure}, null, 2) + '\n');
  await browser.close();
}
if (failure) throw new Error(failure.message);
