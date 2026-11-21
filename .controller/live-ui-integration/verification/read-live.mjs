/** Read-only LAN adoption smoke; wait for rendered records, not only HTTP readiness. */
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';

const [base, output] = process.argv.slice(2);
assert(base && output);
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
await mkdir(output, { recursive: false });
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const report = { base, pages: [], mutations: [], errors: [] };
try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
  for (const app of ['space', 'inspector']) {
    const page = await context.newPage();
    const responses = [];
    let names = [];
    page.on('pageerror', error => report.errors.push({ app, message: error.message }));
    page.on('request', request => {
      if (!['GET', 'HEAD', 'OPTIONS'].includes(request.method())) report.mutations.push({ app, method: request.method(), url: request.url() });
    });
    page.on('response', response => {
      if (new URL(response.url()).pathname.startsWith('/api/')) responses.push({ url: response.url(), status: response.status() });
    });
    const initial = page.waitForResponse(response => new URL(response.url()).pathname === '/api/v1/agents' && response.status() === 200, { timeout: 30000 });
    await page.goto(`${base}/${app}/`, { waitUntil: 'domcontentloaded' });
    names = (await (await initial).json()).agents.map(row => row.agent.name);
    assert(names.length > 0, 'real-data scope must contain native records');
    await page.waitForFunction(({ app, names }) => {
      const text = document.body.innerText;
      const ready = app === 'inspector' ? /已加载\s+[1-9]\d*/.test(text) && !text.includes('正在读取会话') : text.includes('真实 API');
      return ready && names.some(name => text.includes(app === 'space' ? `@${name}` : name));
    }, { app, names }, { timeout: 45000 });
    await page.screenshot({ path: `${output}/${app}.png`, fullPage: false });
    const title = await page.title();
    assert(!title.includes('离线样例'), 'live document must not claim offline supply');
    report.pages.push({ app, url: page.url(), title, renderedNativeRecord: true, responses, screenshot: `${app}.png` });
    await page.close();
  }
  assert.equal(report.mutations.length, 0);
  assert.equal(report.errors.length, 0);
  report.status = 'pass';
} catch (error) {
  report.status = 'fail';
  report.failure = String(error);
  process.exitCode = 1;
} finally {
  await browser.close();
  await writeFile(`${output}/report.json`, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report));
}
