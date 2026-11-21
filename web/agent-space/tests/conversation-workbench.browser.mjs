/** Focused regression of the latest reader's overlap/error recovery and exact Fork journey.
 * Uses only the isolated synthetic workbench fixture. Optional baseline URL is read-only.
 */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { serveWorkspace } from './workspace.fixture.mjs';

const [build, output, baseline] = process.argv.slice(2);
assert(build && output, 'Usage: node conversation-workbench.browser.mjs BUILD EVIDENCE [BASELINE_URL]');
await mkdir(output, { recursive: true });
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const fixture = await serveWorkspace(build, 0, '127.0.0.1', { workbench: true });
const evidence = { source: fixture.base, browser: browser.version(), baseline, cases: {}, density: [], errors: [] };
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: 'reduce' });
page.setDefaultTimeout(25_000);
page.on('pageerror', error => evidence.errors.push(error.message));
const button = (root, name) => root.getByRole('button', { name, exact: typeof name === 'string' });
const pane = (root, name) => root.getByRole('region', { name: '@' + name, exact: true });
const history = root => root.locator('.conversation-history');
const draft = root => root.getByRole('textbox', { name: 'Agent 消息草稿' });
const anchor = root => history(root).evaluate(node => {
  const top = node.getBoundingClientRect().top;
  const record = [...node.querySelectorAll('[data-history-record]')].find(row => row.getBoundingClientRect().bottom > top);
  return record && [record.dataset.historyRecord, Math.round((record.getBoundingClientRect().top - top) * 10) / 10];
});
const calls = (name, service, method = 'GET') => fixture.calls.filter(call => call.path === `/api/v1/agents/${name}/${service}` && call.method === method);
const sameAnchor = (left, right) => {
  assert.equal(left[0], right[0], JSON.stringify([left, right]));
  assert(Math.abs(left[1] - right[1]) <= 2, JSON.stringify([left, right]));
};
async function open(root, name) {
  await root.locator('.office-camera-controls button[aria-expanded]').click();
  await button(root.getByRole('region', { name: '场景名册' }), new RegExp('^@' + name)).click();
  const result = pane(root, name);
  await result.waitFor();
  return result;
}
async function atBottom(reader) {
  return history(reader).evaluate(node => node.scrollHeight - node.clientHeight - node.scrollTop <= 2);
}
async function waitForBottom(reader) {
  await history(reader).evaluate(node => new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(Error('Reader did not reach latest')), 20_000);
    function check() {
      if (node.scrollHeight - node.clientHeight - node.scrollTop > 2) requestAnimationFrame(check);
      else { clearTimeout(timer); resolve(); }
    }
    check();
  }));
}
async function arrival(root, query, name, text) {
  const original = query.query;
  const response = await root.request.post(new URL(`/api/v1/agents/${name}/messages`, fixture.base).href, { data: {
    request_id: 'focused-' + name + '-' + Date.now(), generation: original.generation,
    agent: { name, incarnation: original.incarnation, stopped_event: null }, message: { text },
  } });
  assert(response.ok());
}
async function density(root, readers, label) {
  const size = root.viewportSize();
  const data = await Promise.all(readers.map(reader => history(reader).evaluate(node => {
    const clip = node.getBoundingClientRect();
    const visible = rect => rect.top >= clip.top - 1 && rect.bottom <= clip.bottom + 1 && rect.right > clip.left && rect.left < clip.right;
    const lines = new Set();
    let characters = 0, complete = 0;
    const messages = [...node.querySelectorAll('.history-standard-record.message')];
    for (const message of messages) {
      const box = message.getBoundingClientRect();
      if (box.bottom < clip.top || box.top > clip.bottom) continue;
      if (visible(box)) complete++;
      const body = message.querySelector('.markdown');
      if (!body) continue;
      const walker = document.createTreeWalker(body, NodeFilter.SHOW_TEXT);
      for (let text = walker.nextNode(); text; text = walker.nextNode()) {
        const range = document.createRange();
        for (let i = 0; i < text.length; i++) {
          const character = text.textContent[i];
          if (!character.trim()) continue;
          range.setStart(text, i); range.setEnd(text, i + 1);
          const box = range.getBoundingClientRect();
          if (visible(box)) { characters++; lines.add(Math.round(box.top)); }
        }
      }
    }
    return { historyPx: Math.round(clip.height), bodyCharacters: characters, bodyLines: lines.size, complete, loaded: messages.length };
  })));
  evidence.density.push({ label, ...size, readers: data });
  await root.screenshot({ path: join(output, label + '.png') });
  return data;
}
async function reachable(locator) {
  assert(await locator.evaluate(node => {
    const b = node.getBoundingClientRect();
    return b.width > 0 && b.height > 0 && b.top >= 0 && b.bottom <= innerHeight && node.contains(document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2));
  }), 'control must be visible and receive a tap');
}

try {
  if (baseline) {
    await page.goto(baseline);
    const a = await open(page, 'luna'), b = await open(page, 'mora');
    await a.getByText(/^\[luna \/ 1\]/).waitFor();
    await b.getByText(/^\[mora \/ 1\]/).waitFor();
    await density(page, [a, b], 'baseline-1440-two');
    await page.setViewportSize({ width: 1280, height: 800 });
    await button(a, /聚焦会话/).click();
    await density(page, [a], 'baseline-1280-one');
    await page.setViewportSize({ width: 1440, height: 440 });
    await draft(a).fill('继续核对记录、草稿和原回执。\n'.repeat(32));
    await density(page, [a], 'baseline-1440-short');
    await page.setViewportSize({ width: 375, height: 812 });
    await density(page, [a], 'baseline-375');
    await page.setViewportSize({ width: 1440, height: 900 });
  }
  await page.goto(fixture.base);
  const a = await open(page, 'luna'), b = await open(page, 'mora');
  await a.getByText(/^\[luna \/ 120\]/).waitFor(); await b.getByText(/^\[mora \/ 84\]/).waitFor();
  await waitForBottom(a); await waitForBottom(b);
  assert.equal(await a.locator('[data-history-record]').count(), 20);
  assert.equal(calls('luna', 'message-history')[0].query.before, undefined);
  assert.equal(calls('luna', 'message-history')[0].query.limit, '20');
  await density(page, [a, b], 'current-1440-two');

  let failOlder = true, releaseOlder, olderStarted;
  await page.route('**/api/v1/agents/luna/message-history?*', async route => {
    if (!new URL(route.request().url()).searchParams.has('before')) return route.continue();
    if (failOlder) {
      failOlder = false;
      await route.fulfill({ status: 503, json: { error: { code: 'Unavailable', message: 'focused failure' } } });
      return;
    }
    if (olderStarted) {
      olderStarted();
      await new Promise(resolve => { releaseOlder = resolve; });
      releaseOlder = undefined;
    }
    await route.continue();
  });
  // Place the first visible row near the older edge; a real upward gesture owns demand.
  await history(a).evaluate(node => { node.scrollTop = 145; });
  await history(a).hover();
  await page.mouse.wheel(0, -105);
  const retry = button(a, '重试更早消息');
  await retry.waitFor();
  const recorded = await anchor(a);
  await reachable(retry);
  const failedTop = await history(a).evaluate(node => node.scrollTop);
  assert(failedTop > 0, 'failed read should happen near the edge but not at scrollTop=0');
  assert.equal(calls('mora', 'message-history').length, 1);
  const reading = new Promise(resolve => { olderStarted = resolve; });
  await retry.click(); await reading;
  sameAnchor(recorded, await anchor(a));
  assert.equal(await history(a).evaluate(node => node.scrollTop), failedTop, 'retry must not scroll to expose its hit target');
  // Concurrent arrivals must use the original live watermark while the older page is in flight.
  await arrival(page, calls('luna', 'message-history')[0], 'luna', '独立的新消息 · Luna');
  await arrival(page, calls('mora', 'message-history')[0], 'mora', '独立的新消息 · Mora');
  const jump = button(a, '1 条新消息 · 回到最新');
  await jump.waitFor();
  assert(calls('luna', 'messages').some(call => call.query.after === '120'));
  await b.getByText('独立的新消息 · Mora').waitFor();
  assert(await atBottom(b));
  assert.equal(await button(b, /条新消息/).count(), 0);
  sameAnchor(recorded, await anchor(a));
  olderStarted = undefined; releaseOlder();
  await a.getByText(/^\[luna \/ 81\]/).waitFor();
  sameAnchor(recorded, await anchor(a));
  assert.equal(await a.locator('[data-history-record]').count(), 41);
  assert.equal(calls('luna', 'message-history')[1].query.before, '101');
  evidence.cases.parallel = { anchor: recorded, latestFollow: true, olderRetryAndArrivalIndependent: true };
  await jump.click();
  assert(await atBottom(a));
  assert.equal(await jump.count(), 0);
  evidence.cases.parallel.jumpToLatest = true;

  const longDraft = Array.from({ length: 32 }, (_, i) => `第 ${i + 1} 步：核对来源、已见回执和最后一条消息。 file-${i}.md`).join('\n');
  await draft(a).fill(longDraft);
  await draft(a).press('End'); await draft(a).press('Enter');
  await draft(a).evaluate(input => {
    input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true, isComposing: true }));
    input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
  });
  assert.equal(calls('luna', 'messages', 'POST').length, 1, 'the only message POST so far is the test arrival');
  const saved = await draft(a).inputValue();
  await history(a).evaluate(node => { node.scrollTop = node.scrollHeight / 2; });
  const beforeFork = await anchor(a);
  const rel = button(a, '启动关系');
  const seen = calls('luna', 'launch-relations').length;
  await rel.hover();
  const openMora = button(a, '打开 mora 的精确会话');
  await openMora.hover();
  assert.equal(await openMora.isVisible(), true);
  assert.equal(await a.locator('.launch-relations > div').nth(1).locator('li').count(), 5);
  assert.equal(calls('luna', 'launch-relations')[seen].query.limit, '5');
  assert(await button(a, '打开 nevo 的精确会话').isDisabled());
  await button(a, '更多启动关系').click();
  await button(a, '上一页').click();
  await openMora.waitFor();
  await page.keyboard.press('Escape'); await openMora.waitFor({ state: 'hidden' });
  const search = button(a, '查找已读入记录');
  await search.focus(); await page.keyboard.press('Tab');
  assert(await rel.evaluate(node => document.activeElement === node));
  await openMora.waitFor();
  await page.keyboard.press('Escape'); await openMora.waitFor({ state: 'hidden' });
  const afterClose = calls('luna', 'launch-relations').length;
  await button(a, 'Fork 新 Agent').click();
  const confirmation = page.getByRole('dialog', { name: 'Fork 新 Agent' });
  await confirmation.getByText('来源 · @luna').waitFor();
  assert.equal(calls('luna', 'fork', 'POST').length, 0);
  await button(confirmation, '确认Fork 新 Agent').click();
  await confirmation.waitFor({ state: 'hidden' });
  const action = button(a, '打开分叉会话');
  await action.waitFor();
  assert.equal(calls('luna', 'fork', 'POST').length, 1);
  assert.equal(await pane(page, 'fork2').count(), 0);
  assert.equal(await draft(a).inputValue(), saved);
  sameAnchor(beforeFork, await anchor(a));
  await action.click();
  const fork = pane(page, 'fork2');
  await fork.waitFor(); await a.waitFor();
  assert.equal(await draft(a).inputValue(), saved);
  await button(fork, '启动关系').click();
  await fork.getByText('外部启动', { exact: true }).waitFor();
  await fork.getByText('Fork 来源', { exact: true }).waitFor();
  await fork.getByText('@luna · 原实例 101', { exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => document.querySelector('[aria-label="@fork2"] [data-history-record]') === null);
  assert.equal(calls('luna', 'launch-relations').length, afterClose, 'closed source relations stop refreshing across the next poll');
  await button(fork, '关闭会话：@fork2').click();
  assert.equal(await draft(a).inputValue(), saved);
  const parkedAt = await anchor(a);
  await button(a, '最小化会话：@luna').click();
  await a.waitFor({ state: 'hidden' });
  const readsWhileParked = calls('luna', 'messages').length;
  await page.waitForResponse(response => response.url().includes('/api/v1/agents/mora/messages'));
  assert.equal(calls('luna', 'messages').length, readsWhileParked);
  await button(page.getByRole('navigation', { name: '已打开的会话' }), /@luna.*已最小化/).click();
  await a.getByText(/^\[luna \/ 81\]/).waitFor();
  sameAnchor(parkedAt, await anchor(a));
  assert.equal(await draft(a).inputValue(), saved);
  evidence.cases.fork = { source: calls('luna', 'fork', 'POST')[0].body.agent, latestPreserved: true, separateProvenance: true };

  await page.setViewportSize({ width: 1280, height: 800 });
  await button(a, /聚焦会话/).click();
  await density(page, [a], 'current-1280-one');
  await page.setViewportSize({ width: 1440, height: 440 });
  const send = button(a, '发送 AgentX 消息');
  await reachable(send);
  const [short] = await density(page, [a], 'current-1440-short');
  assert(short.historyPx > 0 && short.bodyLines > 0, 'short viewport must retain readable history');
  assert(await draft(a).evaluate(node => node.scrollHeight > node.clientHeight));
  const shortTop = await history(a).evaluate(node => node.scrollTop);
  await history(a).hover(); await page.mouse.wheel(0, 100);
  await history(a).evaluate((node, initial) => new Promise((resolve, reject) => {
    if (node.scrollTop !== initial) return resolve();
    const timer = setTimeout(() => reject(Error('short reader did not scroll')), 2000);
    node.addEventListener('scroll', () => { clearTimeout(timer); resolve(); }, { once: true });
  }), shortTop);
  evidence.cases.short = { height: short.historyPx, bodyLines: short.bodyLines, scrollable: true, sendReachable: true };

  // The server executes the one original request; the intercepted response leaves its result uncertain.
  // The explicit lookup must recover that exact request without another Fork POST.
  await page.route('**/api/v1/agents/mora/fork', async route => {
    const response = await route.fetch();
    const result = await response.json();
    assert.equal(result.status, 'Succeeded');
    await route.fulfill({ response, json: { generation: result.generation, request_id: result.request_id,
      status: 'Unknown', result: null, error: 'focused uncertainty' } });
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  await button(a, /恢复并排/).click();
  await button(b, 'Fork 新 Agent').click();
  const uncertain = page.getByRole('dialog', { name: 'Fork 新 Agent' });
  await button(uncertain, '确认Fork 新 Agent').click();
  await uncertain.getByText(/结果未确定/).waitFor();
  assert.equal(await button(b, '打开分叉会话').count(), 0);
  const original = calls('mora', 'fork', 'POST')[0].body.request_id;
  await button(uncertain, '查询原回执').click();
  await uncertain.waitFor({ state: 'hidden' });
  await button(b, '打开分叉会话').waitFor();
  assert.equal(calls('mora', 'fork', 'POST').length, 1);
  assert(fixture.calls.some(call => call.method === 'GET' && call.path === '/api/v1/operations/' + original && call.query.generation));
  evidence.cases.uncertainFork = { requestId: original, noRepeat: true };

  const phone = await browser.newPage({ viewport: { width: 375, height: 812 }, hasTouch: true, isMobile: true, reducedMotion: 'reduce' });
  phone.on('pageerror', error => evidence.errors.push(error.message));
  // Emulate a byte-bounded latest page whose body is too short to scroll.
  await phone.route('**/api/v1/agents/luna/message-history?*', async route => {
    const url = new URL(route.request().url());
    if (url.searchParams.has('before')) return route.continue();
    url.searchParams.set('limit', '1');
    await route.continue({ url: url.href });
  });
  await phone.goto(fixture.base);
  const shortPane = await open(phone, 'tobi');
  await shortPane.getByText(/^\[tobi \/ 5\]/).waitFor();
  assert.equal(await button(shortPane, 'Fork 新 Agent').count(), 0);
  const unavailable = button(shortPane, '启动关系');
  await unavailable.tap();
  await shortPane.getByText('外部启动', { exact: true }).waitFor();
  await draft(shortPane).tap();
  await shortPane.getByText('外部启动', { exact: true }).waitFor({ state: 'hidden' });
  const phonePane = await open(phone, 'luna');
  await phonePane.getByText('独立的新消息 · Luna').waitFor();
  const earlier = button(phonePane, '查看更早消息');
  await earlier.waitFor();
  const offsetOf = id => history(phonePane).evaluate((node, recordId) => {
    const row = node.querySelector(`[data-history-record="${recordId}"]`);
    return row.getBoundingClientRect().top - node.getBoundingClientRect().top;
  }, id);
  const shortOffset = await offsetOf('121');
  await earlier.tap();
  await phonePane.getByText(/^\[luna \/ 120\]/).waitFor();
  await phone.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const restoredOffset = await offsetOf('121');
  evidence.cases.mobileShortPage = { before: shortOffset, after: restoredOffset };
  assert(Math.abs(shortOffset - restoredOffset) <= 2, `short first-page anchor shifted from ${shortOffset} to ${restoredOffset}`);
  await density(phone, [phonePane], 'current-375-phone');
  await draft(phonePane).fill(longDraft);
  await reachable(button(phonePane, '发送 AgentX 消息'));
  const sizing = phonePane.getByLabel('调整草稿高度', { exact: true });
  await sizing.tap();
  await button(phonePane, '收紧').tap();
  assert(await sizing.evaluate(node => node === document.activeElement));
  evidence.cases.phone = { touchDisclosure: true, touchSizing: true, supportedOnly: true, shortPageAnchor: shortOffset };
  assert.deepEqual(evidence.errors, []);
} finally {
  evidence.calls = fixture.calls.filter(call => /message-history|messages|launch-relations|fork|operations/.test(call.path));
  await writeFile(join(output, 'focused.json'), JSON.stringify(evidence, null, 2));
  await browser.close();
  await fixture.close();
}
