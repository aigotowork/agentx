/** Independent actual-browser investigation; never injects native/API responses. */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';

const args = Object.fromEntries(process.argv.slice(2).reduce((pairs, value, index, all) => index % 2 ? pairs : [...pairs, [value.replace(/^--/, ''), all[index + 1]]], []));
for (const key of ['binary', 'assets', 'seed', 'evidence']) assert(args[key], `missing ${key}`);
const evidence = resolve(args.evidence);
await mkdir(evidence, { recursive: false });
const seed = JSON.parse(await readFile(args.seed, 'utf8'));
const require = createRequire(import.meta.url);
const driverPath = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
const { chromium } = require(driverPath);
const driverVersion = require(`${driverPath}/package.json`).version;
const steps = [];
let stage = 'launch';
let adapter;
let browser;
let context;
let childExit;
let token;
let origin;
let success = false;
let failure;
const record = async (name, facts = {}) => {
  steps.push({ step: name, ...facts });
  await writeFile(`${evidence}/steps.json`, JSON.stringify(steps, null, 2) + '\n');
};
async function cleanup() {
  if (context) { await context.close().catch(() => {}); context = null; }
  if (browser) { await browser.close().catch(() => {}); browser = null; }
  if (adapter && adapter.exitCode === null && adapter.signalCode === null) adapter.kill('SIGINT');
  if (childExit) {
    const timer = setTimeout(() => adapter.kill('SIGKILL'), 5000);
    await childExit;
    clearTimeout(timer);
  }
}
for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, () => { void cleanup().finally(() => process.exit(130)); });
try {
  adapter = spawn(args.binary, ['observe-web', '--scope', seed.scope.canonical_scope, '--assets', args.assets], { env: process.env, stdio: ['ignore', 'pipe', 'pipe'] });
  childExit = once(adapter, 'exit');
  // Handoff is consumed only in memory. Never forward process output or browser errors.
  const handoff = await new Promise((resolveLine, reject) => {
    let buffer = '';
    const timeout = setTimeout(() => reject(new Error('handoff timeout')), 10000);
    adapter.stdout.on('data', chunk => {
      buffer += chunk;
      if (buffer.includes('\n')) { clearTimeout(timeout); resolveLine(buffer.split('\n')[0]); }
    });
    adapter.once('error', () => { clearTimeout(timeout); reject(new Error('adapter launch')); });
    adapter.once('exit', () => { clearTimeout(timeout); reject(new Error('adapter early exit')); });
  });
  const url = new URL(handoff);
  origin = url.origin;
  token = new URLSearchParams(url.hash.slice(1)).get('bootstrap');
  assert.equal(token.length, 43);
  let stderrHasSecret = false;
  adapter.stderr.on('data', chunk => { if (chunk.toString().includes(token)) stderrHasSecret = true; });
  browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
  const page = await context.newPage();
  const pageErrors = [];
  page.on('pageerror', error => { pageErrors.push(error.name); });
  const test = id => page.getByTestId(id);
  const idle = async () => { await test('refresh-observation').waitFor(); await page.waitForFunction(() => !document.querySelector('[data-testid="refresh-observation"]')?.disabled); };
  stage = 'bootstrap';
  await page.goto(handoff, { waitUntil: 'networkidle', timeout: 15000 });
  await idle();
  assert.equal(new URL(page.url()).hash, '');
  assert((await test('scope-context').innerText()).includes(seed.scope.canonical_scope));
  assert((await test('connection-status').innerText()).includes('connected'));
  assert.equal(await page.evaluate(() => localStorage.length + sessionStorage.length), 0);
  assert.equal((await context.cookies()).length, 0);
  await record(stage, { fragment_removed: true, no_persistent_authority: true, browser: browser.version(), playwright: driverVersion });
  stage = 'select-native-request';
  await test('request-search').pressSequentially('browser-request');
  assert.equal(await test('request-search').inputValue(), 'browser-request');
  const request = page.locator(`[data-testid="request-row"][data-request-id="${seed.request.reference.id}"]`);
  await request.click();
  await idle();
  assert((await test('detail-pane').innerText()).includes('browser-request'));
  await test('open-configuration').click();
  assert((await test('detail-pane').innerText()).includes('NotEvaluated'));
  await record(stage, { request_id: String(seed.request.reference.id), missing_capture_remains_explicit: true, details_dropped: String(seed.snapshot.coverage.request_details_dropped) });
  stage = 'actual-publication-operands';
  await test('tab-events').click();
  await test('events-scope').click();
  await idle();
  for (const [index, capture] of seed.captures.entries()) {
    let row = test('event-records').locator('li.event-record').filter({ hasText: capture.attempt }).filter({ hasText: `配置快照 #${capture.configuration.sequence}` });
    for (let pageIndex = 0; await row.count() === 0 && pageIndex < 10; pageIndex++) {
      assert(await test('events-next').isEnabled(), 'native publication not in complete window');
      await test('events-next').click(); await idle();
    }
    assert.equal(await row.count(), 1);
    await row.getByTestId(index === 0 ? 'event-configuration-left' : 'event-configuration-right').click();
    await idle();
    await record(`operand-${index === 0 ? 'a' : 'b'}`, { sequence: String(capture.configuration.sequence), attempt: capture.attempt, source: 'actual SnapshotPublished' });
    if (index === 0) await test('tab-events').click();
  }
  stage = 'native-comparison';
  await test('compare-configurations').click(); await idle();
  const comparison = await test('comparison-result').innerText();
  assert(comparison.includes('17 秒') && comparison.includes('29 秒') && comparison.includes('不同'));
  assert(comparison.includes('Withheld'));
  const body = await page.locator('body').innerText();
  assert(!body.includes('private-browser-credential') && !body.includes('private-browser-terminal'));
  await page.screenshot({ path: `${evidence}/wide-comparison.png`, fullPage: true });
  await writeFile(`${evidence}/wide-comparison.txt`, body);
  await record(stage, { different_timeout: ['17', '29'], withheld_relation_unknown: true });
  stage = 'events-return-context';
  await test('tab-events').click();
  await test('events-open').click(); await idle();
  assert((await test('detail-pane').innerText()).includes(`#${seed.request.reference.id}`));
  await test('return-request').click();
  assert((await test('detail-pane').innerText()).includes('browser-request'));
  await test('tab-configuration').click();
  assert(await test('compare-configurations').isEnabled());
  assert((await test('comparison-result').innerText()).includes('29 秒'));
  await test('subject-connections').click(); await idle();
  await test('subject-requests').click(); await idle();
  assert((await test('detail-pane').innerText()).includes(`#${seed.request.reference.id}`));
  await record(stage, { selected_request_preserved: true, operands_preserved: true });
  stage = 'narrow-investigation';
  await page.setViewportSize({ width: 390, height: 844 });
  await test('narrow-list').click();
  await test('request-search').focus();
  await test('request-search').press('ControlOrMeta+A');
  await test('request-search').pressSequentially('browser-request');
  assert.equal(await test('request-search').inputValue(), 'browser-request');
  await test('narrow-detail').click();
  await test('tab-configuration').click();
  const summary = page.getByText('配置保留范围', { exact: true });
  await summary.focus(); await page.keyboard.press('Enter');
  await test('configuration-head').click(); await idle();
  const geometry = await page.evaluate(() => ({ width: innerWidth, document: document.documentElement.scrollWidth, controls: [...document.querySelectorAll('button')].filter(n => n.getClientRects().length && !n.disabled).length }));
  assert(geometry.document <= geometry.width + 1, 'document horizontal overflow');
  await page.screenshot({ path: `${evidence}/narrow-comparison.png`, fullPage: true });
  await writeFile(`${evidence}/narrow-comparison.txt`, await page.locator('body').innerText());
  await record(stage, { geometry, keyboard_disclosure: true, multi_character_search: true });
  stage = 'adapter-loss-retains-evidence';
  adapter.kill('SIGINT'); await childExit;
  await test('refresh-observation').click();
  await page.waitForFunction(() => document.querySelector('[data-testid="connection-status"]')?.textContent.includes('adapter-lost'));
  assert((await test('comparison-result').innerText()).includes('29 秒'));
  await test('return-request').click();
  assert((await test('detail-pane').innerText()).includes('browser-request'));
  await record(stage, { adapter_exit: adapter.exitCode, historical_evidence_retained: true });
  assert.deepEqual(pageErrors, []);
  assert(!stderrHasSecret);
  success = true;
} catch (error) {
  failure = { stage, type: error.name, diagnostic: String(error.message).replaceAll(token ?? '<no-token>', '<redacted>').replace(/#bootstrap=[^\s]+/g, '#bootstrap=<redacted>') };
} finally {
  await cleanup();
  const receipt = { passed: success, failure, steps, adapter_pid: adapter?.pid, adapter_exit: adapter?.exitCode, adapter_reaped: adapter ? adapter.exitCode !== null || adapter.signalCode !== null : true, browser_closed: browser === null, context_closed: context === null, secrets_retained: false, product_sha256: createHash('sha256').update(await readFile(args.binary)).digest('hex'), limits: 'Actual released browser + native fixture; no injected responses, user study, platform or provider claim.' };
  await writeFile(`${evidence}/browser.json`, JSON.stringify(receipt, null, 2) + '\n');
}
console.log(JSON.stringify({ passed: success, stage, steps: steps.length, cleanup: true }));
process.exitCode = success ? 0 : 1;
