/** Actual direct LAN page, refresh, independent client and real-data acceptance. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
const [url, evidence] = process.argv.slice(2);
assert(url && evidence, 'Supply LAN URL and new evidence directory');
await mkdir(evidence, { recursive: false });
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const steps = [];
const errors = [];
try {
  const first = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const second = await browser.newContext();
  const page = await first.newPage();
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    assert(!request.headers().authorization, 'Unexpected browser credential');
  });
  const idle = async target => {
    await target.getByTestId('refresh-observation').waitFor();
    await target.waitForFunction(() => !document.querySelector('[data-testid="refresh-observation"]')?.disabled);
  };
  await page.goto(url, { waitUntil: 'networkidle' });
  await idle(page);
  assert((await page.getByTestId('connection-status').innerText()).includes('connected'));
  const rows = await page.getByTestId('request-row').count();
  assert(rows > 0, 'No real request data');
  assert.equal(new URL(page.url()).hash, '');
  steps.push({ step: 'direct-lan-no-credentials', requests: rows });
  await page.reload({ waitUntil: 'networkidle' });
  await idle(page);
  assert((await page.getByTestId('connection-status').innerText()).includes('connected'));
  steps.push({ step: 'ordinary-reload' });
  const other = await second.newPage();
  await other.goto(url, { waitUntil: 'networkidle' });
  await idle(other);
  assert((await other.getByTestId('connection-status').innerText()).includes('connected'));
  assert((await other.getByTestId('request-row').count()) > 0);
  steps.push({ step: 'independent-browser-context' });
  await page.getByTestId('request-row').first().click();
  await idle(page);
  await page.getByTestId('tab-events').click();
  await page.getByTestId('events-scope').click();
  await idle(page);
  const eventRows = await page.getByTestId('event-records').locator('.event-record').count();
  assert(eventRows > 0, 'No real event data');
  steps.push({ step: 'real-request-and-event-investigation', eventRows });
  await page.getByTestId('close-investigation').click();
  await other.getByTestId('refresh-observation').click();
  await idle(other);
  assert((await other.getByTestId('connection-status').innerText()).includes('connected'));
  assert.equal(await first.cookies().then(c => c.length), 0);
  assert.equal(await second.cookies().then(c => c.length), 0);
  assert.equal(await page.evaluate(() => localStorage.length + sessionStorage.length), 0);
  assert.deepEqual(errors, []);
  steps.push({ step: 'page-close-does-not-revoke-other-client' });
  await writeFile(`${evidence}/browser.json`, JSON.stringify({ url, browser: browser.version(), steps, errors }, null, 2) + '\n');
  console.log(`PASS: ${steps.length} actual LAN browser scenarios`);
} finally {
  await browser.close();
}
