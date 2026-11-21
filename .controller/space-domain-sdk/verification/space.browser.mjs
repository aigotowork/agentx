/** Independent browser acceptance: real compiled Space and public SDK with disposable in-memory API fixtures. */
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';
import { currentId, fixture, generation, listener, messageText, office, otherOffice, scope, stoppedId } from './fixtures.mjs';

const [output, evidence] = process.argv.slice(2);
assert(output && evidence, 'usage: node space.browser.mjs BUILD_DIR NEW_EVIDENCE_DIR');
await mkdir(evidence);
const dir = resolve(output);
const media = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.svg': 'image/svg+xml' };
const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://127.0.0.1');
  const file = resolve(dir, '.' + (url.pathname === '/' ? '/index.html' : url.pathname));
  if (!file.startsWith(dir + sep)) { response.writeHead(403).end(); return; }
  try {
    const body = await readFile(file);
    response.writeHead(200, { 'Content-Type': media[extname(file)] ?? 'application/octet-stream' }).end(body);
  } catch { response.writeHead(404).end(); }
});
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? 'playwright');
const records = {};
const report = { build: dir, API: 'synthetic native-shaped responses through actual public SDK', cases: [], status: 'fail' };
let browser;
const pathCalls = (test, path) => test.calls.filter(call => call.path === path);
const reads = test => test.calls.filter(call => /^\/api\/v1\/agents\/luna(?:\/messages)?$/.test(call.path));
const deliberateReads = test => reads(test).filter(call => call.query.after !== 'checkpoint-live-2');
const button = (root, name) => root.getByRole('button', { name, exact: typeof name === 'string' });
const byKey = (page, attribute, key) => page.locator(`button[${attribute}=${JSON.stringify(key)}]`);
const settle = page => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
const visible = async (locator) => locator.waitFor({ state: 'visible' });
const equalInput = async (locator, value) => { await visible(locator); assert.equal(await locator.inputValue(), value); };
async function capture(page, file) { await settle(page); await page.screenshot({ path: resolve(evidence, file + '.png') }); }
function checked(test) { assert.deepEqual(test.violations, []); assert(test.calls.every(call => call.method === 'GET')); }
const caseDone = label => report.cases.push(label);

try {
  await new Promise((ok, fail) => server.once('error', fail).listen(0, '127.0.0.1', ok));
  const base = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  async function pageFor(label, test, width = 1440, beforeResponse) {
    const page = await browser.newPage({ viewport: { width, height: width < 760 ? 844 : 920 } });
    page.setDefaultTimeout(9000);
    const errors = [];
    records[label] = { calls: test.calls, violations: test.violations, errors };
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/api/v1/**', async route => {
      const result = test.respond(route.request());
      const deferred = beforeResponse?.(test.calls.at(-1), result);
      if (deferred) {
        await deferred;
        // Chromium may have already canceled it when the app leaves this Workspace.
        await route.fulfill({ ...result, headers: { 'Access-Control-Allow-Origin': '*' } }).catch(() => {});
      } else await route.fulfill({ ...result, headers: { 'Access-Control-Allow-Origin': '*' } });
    });
    await page.addInitScript(() => {
      const scale = CanvasRenderingContext2D.prototype.scale;
      CanvasRenderingContext2D.prototype.scale = function (x, y) {
        if (this.canvas.closest('.space-scene-stage')) window.__spaceVerificationScale = x;
        return scale.call(this, x, y);
      };
    });
    return { page, errors };
  }
  const test = fixture();
  const { page, errors } = await pageFor('desktop', test);
  const view = page.getByRole('navigation', { name: '呈现方式' });
  const officePicker = page.getByLabel('工作区范围');
  const actor = byKey(page, 'data-hall-object', currentId);
  const roster = page.getByRole('region', { name: 'Agent 名册' });
  const currentRow = byKey(roster, 'data-agent-key', currentId);
  const stoppedRow = byKey(roster, 'data-agent-key', stoppedId);
  const moreMembers = page.getByRole('button', { name: '加载更多成员', exact: true });
  const draft = page.getByRole('textbox', { name: 'Agent 消息草稿' });
  const close = page.getByRole('button', { name: '关闭并返回办公室', exact: true });
  const camera = async () => {
    await settle(page);
    return page.evaluate(() => ({
      scale: window.__spaceVerificationScale,
      rooms: [...document.querySelectorAll('.hall-room-name')].map(e => [e.getBoundingClientRect().x, e.getBoundingClientRect().y]),
    }));
  };
  await page.goto(base + '/?' + new URLSearchParams({ office, supply: 'demo', managementUrl: 'http://127.0.0.1:19742' }));
  await visible(page.getByText(/本页无匹配成员，可继续/));
  await visible(moreMembers);
  assert.equal(await officePicker.inputValue(), office);
  assert.equal(await actor.count(), 0);
  assert.equal(pathCalls(test, '/api/v1/agents').length, 1);
  assert.equal(pathCalls(test, '/api/v1/workspaces').length, 1);
  assert.equal(test.calls[0].path, '/api/v1/management');
  await capture(page, 'desktop-sparse-office');
  await moreMembers.click();
  await visible(actor);
  assert.equal(await actor.getAttribute('data-room-key'), office);
  assert.equal(pathCalls(test, '/api/v1/agents').length, 2);
  await moreMembers.click();
  await visible(page.getByText(/本次扫描结束/));
  assert.equal(await moreMembers.count(), 0);
  assert.equal(pathCalls(test, '/api/v1/agents').length, 3);
  assert.equal(await byKey(page, 'data-hall-object', stoppedId).count(), 0);
  caseDone('Trusted host selection beats URL mode/endpoint; sparse bounded workspace pages do not invent members; stopped namesake is not an actor.');

  await button(page, '暂停动作').click();
  await capture(page, 'desktop-office');
  await button(page, '放大大厅').click();
  const overview = await camera();
  assert(Number.isFinite(overview.scale) && overview.rooms.length, 'office camera scale and landmarks must be sampled');
  await actor.click();
  await visible(page.getByText(messageText[0], { exact: true }));
  assert.deepEqual(reads(test).map(call => call.path), ['/api/v1/agents/luna', '/api/v1/agents/luna/messages']);
  assert.equal((await camera()).scale, overview.scale);
  const firstRoute = new URL(page.url());
  assert.equal(firstRoute.searchParams.get('scope'), scope);
  assert.equal(firstRoute.searchParams.get('generation'), generation);
  assert.equal(firstRoute.searchParams.get('incarnation'), '9007199254740999');
  const draftText = '这条草稿只属于本班次。';
  await draft.fill(draftText);
  await button(page, '更多消息').click();
  await visible(page.getByText(messageText[1], { exact: true }));
  await visible(page.getByText(messageText[0], { exact: true }));
  assert.equal(deliberateReads(test).at(-1).query.after, 'checkpoint-live-1');
  assert.equal(deliberateReads(test).length, 3);
  await capture(page, 'desktop-actor-conversation');
  await button(view, '聊天').click();
  await visible(currentRow);
  await equalInput(draft, draftText);
  await visible(page.getByText(messageText[1], { exact: true }));
  assert.equal(deliberateReads(test).length, 3);
  assert(!new URL(page.url()).search.includes(encodeURIComponent(draftText)));
  await capture(page, 'desktop-conversation-list');
  await button(view, '场景').click();
  await equalInput(draft, draftText);
  assert.equal(deliberateReads(test).length, 3);
  await close.click();
  assert.deepEqual(await camera(), overview);
  await button(view, '聊天').click();
  await currentRow.click();
  await visible(page.getByText(messageText[0], { exact: true }));
  await equalInput(draft, draftText);
  await button(view, '场景').click();
  await equalInput(draft, draftText);
  assert.equal((await camera()).scale, overview.scale);
  caseDone('Actor and list open one exact conversation; paging follows its checkpoint; mode switch keeps draft without reads; closing restores camera and the list reopens the same draft.');

  await page.setViewportSize({ width: 390, height: 844 });
  await equalInput(draft, draftText);
  const box = await draft.boundingBox();
  assert(box && box.y >= 0 && box.y + box.height <= 844);
  await capture(page, 'narrow-actor-conversation');
  await close.click();
  await capture(page, 'narrow-office');
  const returnedPosition = await actor.evaluate((element) => {
    const actor = element.getBoundingClientRect();
    const stage = element.closest('.space-scene-stage').getBoundingClientRect();
    return { x: actor.x + actor.width / 2, y: actor.y + actor.height / 2, stage: stage.toJSON() };
  });
  const { x, y, stage } = returnedPosition;
  assert(x > stage.left && x < stage.right && y > stage.top && y < stage.bottom, JSON.stringify(returnedPosition));
  await actor.press('Enter');
  await equalInput(draft, draftText);
  await close.click();
  await page.setViewportSize({ width: 1440, height: 920 });
  await button(view, '聊天').click();
  await button(page, /^历史(?:\s|$)/).click();
  await stoppedRow.click();
  await visible(page.getByText(messageText[2], { exact: true }));
  assert.equal(await page.getByText(messageText[0], { exact: true }).count(), 0);
  if (await draft.count()) assert(await draft.evaluate(e => e.readOnly));
  assert.equal(reads(test).at(-1).query.stopped_event, '170');
  assert(!reads(test).at(-1).query.incarnation);
  const stoppedRoute = new URL(page.url());
  assert.equal(stoppedRoute.searchParams.get('stopped_event'), '170');
  await capture(page, 'desktop-stopped-conversation');
  checked(test);
  assert.deepEqual(errors, []);
  await page.close();
  caseDone('390px composer remains usable; same-name stopped history has a separate exact route, record and read-only composer.');

  const directTest = fixture();
  const { page: direct, errors: directErrors } = await pageFor('exact-route', directTest);
  await direct.goto(firstRoute.href);
  await visible(direct.getByText(messageText[0], { exact: true }));
  await equalInput(direct.getByRole('textbox', { name: 'Agent 消息草稿' }), '');
  assert.equal(await byKey(direct, 'data-hall-object', currentId).count(), 0);
  assert.equal(pathCalls(directTest, '/api/v1/agents').length, 1);
  assert.deepEqual(reads(directTest).map(call => call.path), ['/api/v1/agents/luna', '/api/v1/agents/luna/messages']);
  checked(directTest);
  assert.deepEqual(directErrors, []);
  await direct.close();
  caseDone('Direct address restores a reader for an unpaged exact subject without adding a fake roster actor or restoring another page’s draft.');

  const longTest = fixture();
  const { page: long, errors: longErrors } = await pageFor('history-position', longTest, 390, (call, result) => {
    if (call.path !== '/api/v1/agents/luna/messages' || !result.json?.messages?.length) return;
    result.json.messages[0].text = Array.from({ length: 36 }, (_, i) => `长记录 ${i} · ${call.query.after ?? '起始'}`).join('\n\n');
  });
  await long.goto(base + '/?' + new URLSearchParams({ office, view: 'chat' }));
  await button(long, '加载更多成员').click();
  await byKey(long, 'data-agent-key', currentId).click();
  const longHistory = long.locator('.conversation-history');
  const top = () => longHistory.evaluate(e => e.scrollTop);
  const nextMessagePage = button(long, '更多消息');
  await visible(nextMessagePage);
  await settle(long);
  assert.equal(await top(), 0, 'forward history begins at the first available record');
  await nextMessagePage.focus();
  const originalTop = await top();
  assert(originalTop > 0);
  await long.keyboard.press('Enter');
  await long.getByText('长记录 35 · checkpoint-live-1', { exact: true }).waitFor();
  await settle(long);
  assert.equal(await top(), originalTop);
  await button(long.getByRole('navigation', { name: '呈现方式' }), '场景').click();
  await settle(long);
  assert.equal(await top(), originalTop);
  assert.equal(reads(longTest).filter(call => call.path.endsWith('/messages')).length, 2);
  checked(longTest);
  assert.deepEqual(longErrors, []);
  await long.close();
  caseDone('A long forward history starts at its beginning and keeps reading position through page append and presentation switch.');

  const mismatch = fixture({ describedScope: '/fixture/other-listener' });
  const { page: wrong, errors: wrongErrors } = await pageFor('wrong-scope', mismatch, 390);
  await wrong.goto(base + '/?' + new URLSearchParams({ office, supply: 'demo', managementUrl: 'http://127.0.0.1:19742' }));
  await visible(button(wrong, '重试连接'));
  assert.deepEqual(mismatch.calls.map(call => call.path), ['/api/v1/management']);
  assert.equal(await byKey(wrong, 'data-hall-object', currentId).count(), 0);
  checked(mismatch);
  assert.deepEqual(wrongErrors, []);
  await capture(wrong, 'scope-mismatch');
  await wrong.close();
  caseDone('Wrong observed scope stops after descriptor and exposes scoped retry rather than falling back.');

  const delayed = fixture(), release = Promise.withResolvers();
  const { page: quick, errors: quickErrors } = await pageFor('superseded-reader', delayed, 1440, call => {
    if (call.path === '/api/v1/agents/luna/messages') return release.promise;
  });
  try {
    await quick.goto(base + '/?' + new URLSearchParams({ office }));
    await button(quick, '加载更多成员').click();
    await Promise.all([
      quick.waitForRequest(request => new URL(request.url()).pathname === '/api/v1/agents/luna/messages', { timeout: 9000 }),
      byKey(quick, 'data-hall-object', currentId).press('Enter'),
    ]);
    await quick.getByLabel('工作区范围').selectOption(otherOffice);
    await quick.getByRole('button', { name: /@mora\b/ }).waitFor();
  } finally { release.resolve(); }
  await settle(quick);
  assert.equal(await quick.getByText(messageText[0], { exact: true }).count(), 0);
  assert.equal(new URL(quick.url()).searchParams.get('agent'), null);
  checked(delayed);
  assert.deepEqual(quickErrors, []);
  await quick.close();
  caseDone('A delayed Agent message completion cannot reopen the closed reader after selection moves to another Workspace.');

  const staleTest = fixture();
  const { page: stale, errors: staleErrors } = await pageFor('stale-generation', staleTest);
  await stale.goto(base + '/?' + new URLSearchParams({ office }));
  await button(stale, '加载更多成员').click();
  await byKey(stale, 'data-hall-object', currentId).click();
  await visible(stale.getByText(messageText[0], { exact: true }));
  const staleDraft = stale.getByRole('textbox', { name: 'Agent 消息草稿' });
  await staleDraft.fill('旧代次中的草稿');
  staleTest.changeGeneration();
  await button(stale, '加载更多成员').click();
  await visible(button(stale, '重新连接'));
  assert.equal(pathCalls(staleTest, '/api/v1/agents').length, 3);
  assert(pathCalls(staleTest, '/api/v1/management').length >= 2, 'confirm replacement through an independently read descriptor');
  assert(await staleDraft.evaluate(e => e.readOnly));
  await stale.evaluate(() => {
    window.__spaceReconnectLocations = [];
    window.addEventListener('agentx:location-changed', () => window.__spaceReconnectLocations.push(location.search));
  });
  staleTest.allowReconnect();
  await Promise.all([
    stale.waitForResponse(response => new URL(response.url()).searchParams.get('generation') === 'successor-generation', { timeout: 9000 }),
    button(stale, '重新连接').click(),
  ]);
  await staleDraft.waitFor({ state: 'detached' });
  assert.equal(new URL(stale.url()).searchParams.get('agent'), null);
  assert.deepEqual(await stale.evaluate(() => window.__spaceReconnectLocations), ['']);
  checked(staleTest);
  assert.deepEqual(staleErrors, []);
  await stale.close();
  caseDone('Changed generation makes the open draft read-only. Explicit reconnect starts a new session, clears the old target and announces the cleaned URL.');
  report.status = 'pass';
} catch (error) {
  report.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  await new Promise(resolve => server.close(resolve));
  await writeFile(resolve(evidence, 'report.json'), JSON.stringify({ ...report, records, listener }, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
}
