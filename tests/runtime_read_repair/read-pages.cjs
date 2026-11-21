/** Bounded ordinary read navigation; no production writes, launch or saturation. */
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_MODULE || 'playwright');
const fs = require('node:fs');
const base = process.env.AGENTX_PROBE_URL || 'http://192.168.2.178:38000';
const output = process.env.AGENTX_PROBE_OUTPUT;
if (!output) throw new Error('AGENTX_PROBE_OUTPUT is required');
(async () => {
  const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox'] });
  const report = { base, at: new Date().toISOString(), pages: {} };
  try {
    await Promise.all(['/', '/space/', '/inspector/'].map(async route => {
      const page = await browser.newPage({ viewport: { width: route === '/space/' ? 390 : 1280, height: 900 } });
      const started = Date.now(), sent = new Map(), pending = [], responses = [], writes = [], errors = [];
      let firstDirectoryRenderMs = null;
      page.on('pageerror', error => errors.push(String(error)));
      await page.route('**/*', async request => {
        if (!['GET', 'HEAD', 'OPTIONS'].includes(request.request().method())) {
          writes.push({ method: request.request().method(), path: new URL(request.request().url()).pathname });
          await request.abort();
        } else await request.continue();
      });
      page.on('request', request => sent.set(request, Date.now()));
      page.on('response', response => {
        if (!response.url().includes('/api/')) return;
        const item = { path: new URL(response.url()).pathname, query: new URL(response.url()).search, status: response.status(), atMs: Date.now() - started, durationMs: Date.now() - (sent.get(response.request()) || Date.now()) };
        responses.push(item);
        pending.push((async () => {
          item.requestId = (await response.allHeaders())['x-request-id'];
          if (response.status() >= 400) item.error = await response.text().catch(() => 'unreadable');
        })());
      });
      await page.goto(base + route, { waitUntil: 'domcontentloaded' });
      if (route === '/inspector/') {
        await page.waitForFunction(() => /已加载\s+[1-9]\d*/.test(document.body.innerText), undefined, { timeout: 18000 }).then(() => { firstDirectoryRenderMs = Date.now() - started; }).catch(error => errors.push('directory render: ' + String(error)));
      }
      const remaining = Math.max(0, 16000 - (Date.now() - started));
      if (remaining) await page.waitForTimeout(remaining);
      await Promise.all(pending);
      const body = await page.locator('body').innerText();
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
      report.pages[route] = { responses, writes, errors, firstDirectoryRenderMs, loaded: body.match(/已加载\s+\d+/)?.[0], overflow };
      await page.close();
    }));
  } finally { await browser.close(); }
  fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { mode: 0o600 });
  for (const [route, page] of Object.entries(report.pages)) {
    console.log(JSON.stringify({ route, requests: page.responses.length, errors: page.errors, failed: page.responses.filter(item => item.status >= 400), firstDirectoryRenderMs: page.firstDirectoryRenderMs, overflow: page.overflow }));
  }
  if (Object.values(report.pages).some(page => page.errors.length || page.writes.length || page.responses.some(item => item.status >= 400))) process.exitCode = 1;
})().catch(error => { console.error(error); process.exitCode = 1; });
