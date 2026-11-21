/** Independent actual-browser paging acceptance; all API facts are explicit finite fixtures. */
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile, mkdir, writeFile} from 'node:fs/promises';
import {resolve, extname} from 'node:path';
import {createRequire} from 'node:module';

const [assetRoot, evidence] = process.argv.slice(2);
if (!assetRoot || !evidence) throw new Error('Usage: node inspector.browser.mjs ASSET_ROOT NEW_EVIDENCE_DIRECTORY');
await mkdir(evidence, {recursive: false});
const root = resolve(assetRoot);
const server = createServer(async (req, res) => {
  const pathname = new URL(req.url, 'http://fixture').pathname;
  const relative = pathname.replace(/^\/inspector\/?/, '') || 'index.html';
  const file = resolve(root, relative);
  if (!file.startsWith(root + '/')) { res.writeHead(403).end(); return; }
  try {
    const bytes = await readFile(file);
    res.writeHead(200, {'content-type': {'.js': 'text/javascript', '.css': 'text/css', '.html': 'text/html'}[extname(file)] ?? 'application/octet-stream'}).end(bytes);
  } catch { res.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const {chromium} = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--no-proxy-server', '--disable-dev-shm-usage']});
const page = await browser.newPage({viewport: {width: 1440, height: 950}});
const traffic = [], errors = [], checks = [];
let generation = 'fixture-one', exactMode = 'match', failure;
const descriptor = () => ({generation, scope: '/disposable/independent', project: '/fixture', tools: ['codex'], profiles: [], max_page: 100, max_text: 100000, supports_labels: true});
const agent = (name, stopped = false) => ({agent: {name, incarnation: stopped ? null : '4621819117588971520', stopped_event: stopped ? '42' : null}, tool: 'codex', status: stopped ? 'stopped' : 'active', stored_status: 'active', status_context: '', detail: 'fixture', directory: '/fixture', session_id: null, can_send: !stopped, can_stop: false, can_resume: false, can_fork: false, can_stop_participation: false, has_transcript: false, collaboration: {status: 'adopted', workspace: '/fixture', role: null, reason: null}, pending_messages: 0, title: null, request_error: null, labels: {}, labels_revision: '0', file_bindings: []});
const current = Array.from({length: 101}, (_, index) => agent(`member${String(index).padStart(3, '0')}`));
page.on('pageerror', error => errors.push(error.message));
await page.route('**/api/**', async route => {
  const request = route.request(), url = new URL(request.url());
  traffic.push({at: Date.now(), method: request.method(), path: url.pathname, query: url.search});
  if (request.method() !== 'GET') { await route.abort(); return; }
  let body;
  if (url.pathname === '/api/v1/management') body = descriptor();
  else if (url.pathname === '/api/v1/agents') {
    assert.equal(url.searchParams.get('generation'), generation);
    assert.equal(url.searchParams.get('limit'), '100');
    const isCurrent = url.searchParams.get('current_only') === 'true';
    const after = url.searchParams.get('after');
    if (generation === 'fixture-two') body = {generation, agents: [agent('replacement')], next: null, current_complete: true};
    else if (isCurrent) body = {generation, agents: after ? current.slice(100) : current.slice(0, 100), next: after ? null : 'current-opaque', current_complete: !!after};
    else body = {generation, agents: after ? [agent('member000', true)] : [current[0]], next: after ? null : 'history-opaque', current_complete: true};
  } else if (url.pathname.endsWith('/messages')) body = {generation, messages: [], next_after: '0', has_more: false};
  else if (url.pathname.startsWith('/api/v1/agents/')) {
    if (url.pathname.endsWith('/archived')) {
      assert.equal(url.searchParams.get('stopped_event'), '42');
      body = agent('archived', exactMode === 'match');
    } else {
      await route.fulfill({status: 409, contentType: 'application/json', body: JSON.stringify({error: {code: 'StaleReference', message: 'fixture generation changed'}})});
      return;
    }
  }
  else { await route.fulfill({status: 500, contentType: 'application/json', body: JSON.stringify({error: {code: 'unexpected-fixture-read', message: url.pathname}})}); return; }
  await route.fulfill({status: 200, contentType: 'application/json', body: JSON.stringify(body)});
});
const listings = () => traffic.filter(row => row.path === '/api/v1/agents');
const descriptors = () => traffic.filter(row => row.path === '/api/v1/management');
try {
  await page.goto(`http://127.0.0.1:${server.address().port}/inspector/`);
  await page.getByText('已加载 100 · 部分覆盖', {exact: true}).waitFor();
  assert.equal(listings().length, 1);
  assert(!traffic.some(row => row.path.includes('/inspection')));
  checks.push('First current page renders after one request, before history demand; session tab does not fetch inspection.');
  await page.getByRole('button', {name: '加载更多当前成员', exact: true}).click();
  await page.getByText('已加载 101 · 当前成员分页结束', {exact: true}).waitFor();
  assert.equal(new URLSearchParams(listings().at(-1).query).get('after'), 'current-opaque');
  await page.getByRole('button', {name: '刷新', exact: true}).click();
  await page.waitForFunction(() => !document.querySelector('[aria-busy="true"]'));
  await page.getByRole('button', {name: '全部（含停止历史）', exact: true}).click();
  await page.getByText('已加载 1 · 当前已覆盖 · 历史部分覆盖', {exact: true}).waitFor();
  assert.equal(new URLSearchParams(listings().at(-1).query).get('current_only'), 'false');
  await page.getByRole('button', {name: '加载更多记录', exact: true}).click();
  await page.getByText('已加载 2 · 目录分页结束', {exact: true}).waitFor();
  const sameName = page.locator('button.native-resource').filter({hasText: 'member000'});
  assert.equal(await sameName.count(), 2);
  await sameName.filter({hasText: '停止 #42'}).click();
  await page.getByRole('heading', {name: /member000 stopped/}).waitFor();
  assert.equal(new URLSearchParams(traffic.filter(row => row.path.endsWith('/messages')).at(-1).query).get('stopped_event'), '42');
  checks.push('Explicit current/history paging uses independent opaque cursors; same-name current/stopped selection retains exact history ID.');
  const historyReads = listings().filter(row => new URLSearchParams(row.query).get('current_only') === 'false').length;
  const before = descriptors().length;
  await page.waitForResponse(response => response.url().includes('/api/v1/management'), {timeout: 8000});
  assert(descriptors().length > before);
  assert.equal(listings().filter(row => new URLSearchParams(row.query).get('current_only') === 'false').length, historyReads);
  checks.push('Descriptor polling occurs without repeating loaded historical pages.');
  await page.screenshot({path: `${evidence}/history.png`, fullPage: true});
  generation = 'fixture-two';
  await page.waitForResponse(response => response.url().includes('/api/v1/management'), {timeout: 8000});
  await page.locator('button.native-resource').filter({hasText: 'replacement'}).waitFor();
  assert.equal(await page.locator('button.native-resource').filter({hasText: 'member000'}).count(), 0);
  assert(!await page.getByRole('heading', {name: /member000 stopped/}).count());
  checks.push('Generation replacement clears historical data and selected exact conversation before exposing replacement data.');
  const directStart = traffic.length;
  const reference = encodeURIComponent(JSON.stringify(['archived', null, '42']));
  await page.goto(`http://127.0.0.1:${server.address().port}/inspector/?ref=${reference}`);
  await page.getByRole('heading', {name: /archived stopped/}).waitFor();
  const directTraffic = traffic.slice(directStart);
  assert.equal(directTraffic.filter(row => row.path === '/api/v1/agents').length, 1);
  assert.equal(directTraffic.filter(row => row.path === '/api/v1/agents/archived').length, 1);
  assert(!directTraffic.some(row => row.path === '/api/v1/agents' && new URLSearchParams(row.query).get('current_only') === 'false'));
  assert.equal(new URLSearchParams(directTraffic.find(row => row.path.endsWith('/messages')).query).get('stopped_event'), '42');
  checks.push('Direct stopped URL resolves one exact row outside the current first page without fetching history pages.');
  exactMode = 'mismatch';
  const mismatchStart = traffic.length;
  await page.reload();
  await page.getByRole('alert').filter({hasText: '会话引用已变化'}).waitFor();
  assert.equal(await page.getByRole('heading', {name: /archived/}).count(), 0);
  assert(!traffic.slice(mismatchStart).some(row => row.path.endsWith('/messages')));
  checks.push('A same-name current reply cannot replace the requested stopped identity or expose its conversation/actions.');
  assert.deepEqual(errors, []);
  assert(traffic.every(row => row.method === 'GET'));
} catch (error) { failure = {message: error.message, stack: error.stack}; }
finally {
  await writeFile(`${evidence}/result.json`, JSON.stringify({status: failure ? 'failed' : 'passed', boundary: 'Actual freshly built UI and browser, finite intercepted API fixtures; no native/runtime integration claim.', browser: browser.version(), checks, traffic, errors, failure}, null, 2) + '\n');
  await browser.close();
  await new Promise(resolve => server.close(resolve));
}
if (failure) throw new Error(failure.message);
