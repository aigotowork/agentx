/** Browser review of the real built Space Canvas, controls and conversation loop with isolated studio data. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const [base, out, profile] = process.argv.slice(2);
assert(base && out, 'usage: node studio.browser.mjs PREVIEW_ORIGIN NEW_EVIDENCE_DIR');
const sdk = profile === 'sdk';
const url = (office = '/studio/catalog') => new URL('?' + new URLSearchParams({ art: 'studio', ...(!sdk && { supply: 'studio' }), office }), base).href;
await mkdir(out);
const { chromium } = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? 'playwright');
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const report = { source: base, status: 'fail', cases: [], errors: [], api: [], badResponses: [] };
const button = (root, name) => root.getByRole('button', { name, exact: typeof name === 'string' });
const firstFrame = page => page.locator('[data-scene-art="studio"][data-art-status="ready"]');
const byKey = (page, field, key) => page.locator(`button[${field}=${JSON.stringify(key)}]`);
const draft = page => page.getByRole('textbox', { name: 'Agent 消息草稿' });
const close = page => button(page, '关闭并返回办公室');
const views = page => page.getByRole('navigation', { name: '呈现方式' });
const ariaActor = (page, name) => page.getByRole('button', { name: new RegExp('^@' + name + '(?: |$)') }).and(page.locator('button.office-actor'));
const settle = page => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
async function screenshot(page, name) { await settle(page); await page.screenshot({ path: resolve(out, name + '.png') }); }

async function tab(width, options = {}) {
  const page = await browser.newPage({ viewport: { width, height: width < 760 ? 844 : 900 }, ...options });
  page.setDefaultTimeout(9000);
  page.on('pageerror', e => report.errors.push(e.message));
  page.on('request', request => {
    if (new URL(request.url()).pathname.startsWith('/api/v1/')) report.api.push({ method: request.method(), url: request.url() });
  });
  page.on('response', response => { if (response.status() >= 400 && /\/assets\/|\.svg(?:\?|$)/.test(response.url())) report.badResponses.push([response.status(), response.url()]); });
  return page;
}
async function enter(page, office) {
  await page.goto(url(office));
  await page.locator('[data-space-art="studio"]').first().waitFor();
  if (!sdk) await page.locator('[data-space-supply="studio"]').first().waitFor();
  await firstFrame(page).first().waitFor();
  await settle(page);
}
async function capture(page, name) {
  const state = await page.evaluate(() => {
    const read = result => {
      if (!result.ok) throw new Error(result.error.code + ': ' + result.error.message);
      return result.value;
    };
    const api = window.agentxSceneDebug;
    const [{ id: canvasId }] = read(api.list({ kind: 'canvases' }));
    const snapshot = read(api.snapshot({ canvasId }));
    const pixels = read(api.capture({ canvasId }));
    return {
      frame: snapshot.frame, captured: pixels.frame,
      objects: read(api.list({ kind: 'objects', canvasId })),
      png: pixels.dataUrl,
    };
  });
  assert.equal(state.frame.frameId, state.captured.frameId);
  assert(state.png && state.png.length > 4000, 'PNG should contain a presented scene');
  await writeFile(resolve(out, name + '.png'), Buffer.from(state.png.split(',')[1], 'base64'));
  const { png, ...metadata } = state;
  return metadata;
}

try {
  const page = await tab(1440);
  await enter(page);
  await ariaActor(page, 'mora').waitFor();
  assert.equal(await page.locator('button.office-actor').count(), 6);
  const mora = ariaActor(page, 'mora'), riku = ariaActor(page, 'riku');
  const key = await mora.getAttribute('data-hall-object');
  assert(key && key !== await riku.getAttribute('data-hall-object'));
  assert.equal(await mora.getAttribute('data-room-key'), '/studio/catalog');
  const states = await page.locator('button.office-actor').evaluateAll(anchors => anchors.map(anchor => ({
    name: anchor.getAttribute('aria-label'), status: anchor.getAttribute('data-behavior'),
    visible: (() => {
      const target = anchor.getBoundingClientRect(), stage = anchor.closest('.space-scene-stage').getBoundingClientRect();
      const x = target.x + target.width / 2, y = target.y + target.height / 2;
      return x >= stage.left && x <= stage.right && y >= stage.top && y <= stage.bottom && document.elementFromPoint(x, y)?.closest('button') === anchor;
    })(),
  })));
  report.initialActors = states;
  assert(new Set(states.map(a => a.status)).size >= 3, 'the scene must exercise several distinct behaviors');
  assert(states.find(a => a.name.startsWith('@mora '))?.visible, 'the available actor needs a real pointer hit on entry');
  assert.equal(await mora.getAttribute('data-studio-signal'), 'available');
  assert.equal(await riku.getAttribute('data-studio-signal'), 'attention');
  async function distinctEntryLabels(target) {
    const separate = await target.evaluate(() => {
      const room = document.querySelector('.hall-room-name')?.getBoundingClientRect();
      const mail = document.querySelector('.office-human span')?.getBoundingClientRect();
      if (!(room && mail && room.width && mail.width)) return true;
      return !(room.left < mail.right && mail.left < room.right && room.top < mail.bottom && mail.top < room.bottom);
    });
    assert(separate, 'mailbox and room labels overlap');
  }
  await distinctEntryLabels(page);
  await screenshot(page, 'desktop-studio');
  report.scene = await capture(page, 'canvas-studio');
  assert.equal(report.scene.objects.filter(o => o.kind === 'actor').length, 6);
  assert(report.scene.objects.some(o => o.kind === 'room'));
  await mora.click();
  if (sdk) await page.getByText(/版片编号是 R-14/).waitFor();
  const activeReads = () => report.api.filter(r => /^\/api\/v1\/agents\/mora(?:\/messages)?$/.test(new URL(r.url).pathname));
  const beforeSwitch = activeReads().length;
  await draft(page).fill('版画工坊 · 给 mora 的草稿');
  await screenshot(page, 'desktop-studio-conversation');
  await button(views(page), '聊天').click();
  await byKey(page, 'data-agent-key', key).waitFor();
  assert.equal(await draft(page).inputValue(), '版画工坊 · 给 mora 的草稿');
  await screenshot(page, 'desktop-studio-list');
  await button(views(page), '场景').click();
  await firstFrame(page).first().waitFor();
  assert.equal(await draft(page).inputValue(), '版画工坊 · 给 mora 的草稿');
  assert.equal(activeReads().length, beforeSwitch);
  await close(page).click();
  await riku.click();
  assert.equal(await draft(page).inputValue(), '');
  await close(page).click();
  await mora.click();
  assert.equal(await draft(page).inputValue(), '版画工坊 · 给 mora 的草稿');
  await close(page).click();
  if (sdk) {
    await button(views(page), '聊天').click();
    await button(page, /^历史(?:\s|$)/).click();
    const historical = page.locator('button[data-agent-key]').filter({ hasText: '@mora' });
    await historical.click();
    await page.getByText(/旧校样已收回/).waitFor();
    assert.equal(await draft(page).inputValue(), '');
    assert(await draft(page).evaluate(e => e.readOnly));
    assert(activeReads().some(r => new URL(r.url).searchParams.get('stopped_event') === '80'));
    assert.equal(new URL(page.url()).searchParams.get('stopped_event'), '80');
  }
  await page.close();
  report.cases.push('Six distinct actors in actual Canvas diagnostics. Pointer opens exact reader; scene/list continuity and separate addressed drafts hold.');

  const narrow = await tab(390, { reducedMotion: 'reduce', isMobile: true, hasTouch: true });
  await enter(narrow);
  await ariaActor(narrow, 'mora').waitFor({ state: 'attached' });
  await distinctEntryLabels(narrow);
  await screenshot(narrow, 'narrow-studio');
  const rosterPath = button(narrow, '名册');
  await rosterPath.tap();
  await byKey(narrow.getByRole('region', { name: 'Agent 名册' }), 'data-agent-key', key).tap();
  await draft(narrow).fill('窄屏草稿');
  assert.equal(await button(narrow, '返回办公室').count(), 1);
  await screenshot(narrow, 'narrow-studio-conversation');
  const box = await draft(narrow).boundingBox();
  assert(box && box.y >= 0 && box.y + box.height <= 844);
  await button(narrow, '返回办公室').click();
  await button(views(narrow), '聊天').click();
  await byKey(narrow, 'data-agent-key', key).focus();
  await narrow.keyboard.press('Enter');
  assert.equal(await draft(narrow).inputValue(), '窄屏草稿');
  await narrow.close();
  report.cases.push('390 px reduced-motion touch/keyboard roster path reaches the full composer and restores the same draft.');

  const empty = await tab(1440);
  await enter(empty, '/studio/annex');
  if (sdk) await empty.getByLabel('工作区范围').locator('option[value="/studio/annex"]').waitFor({ state: 'attached' });
  assert.equal(await empty.locator('button.office-actor').count(), 0);
  await screenshot(empty, 'desktop-empty-annex');
  await empty.close();
  report.cases.push('An empty office presents the same art system with zero invented actors.');

  if (sdk) {
    const requests = report.api.map(r => ({ method: r.method, url: new URL(r.url) }));
    assert(requests.every(r => r.method === 'GET'));
    assert.equal(requests[0]?.url.pathname, '/api/v1/management');
    assert(requests.some(r => r.url.pathname === '/api/v1/workspaces'));
    assert(requests.some(r => r.url.pathname === '/api/v1/agents'));
    assert(requests.some(r => r.url.pathname === '/api/v1/agents/mora' && r.url.searchParams.get('incarnation') === '101'));
    assert(requests.some(r => r.url.pathname === '/api/v1/agents/mora/messages' && r.url.searchParams.get('incarnation') === '101'));
    report.cases.push('Configured Studio uses the real public SDK scope, Workspace and exact Agent history over read-only fixture HTTP.');
  } else assert.deepEqual(report.api, []);
  assert.deepEqual(report.errors, []);
  assert.deepEqual(report.badResponses, []);
  report.status = 'pass';
} catch (error) {
  report.failure = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  await browser.close();
  await writeFile(resolve(out, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({ status: report.status, cases: report.cases, failure: report.failure, errors: report.errors }, null, 2));
}
