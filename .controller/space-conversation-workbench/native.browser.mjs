/** Verify frozen Space assets in Chrome on the same origin as the disposable native server. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';
import { selectAgentXScope } from '../../packages/agentx-client/dist/index.js';

const file = process.argv[2];
const config = JSON.parse(await readFile(file, 'utf8'));
const { url, scope, generation, reference, ids, workspace, build } = config;
const report = { scope, generation, reference, build, status: 'failed', assets: {}, requests: [], pageErrors: [], cases: [] };
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const media = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.webp': 'image/webp' };
let browser;
try {
  const selected = await selectAgentXScope({ baseUrl: url, expectedScope: scope });
  assert.equal(selected.descriptor.supports_message_history, true);
  const agent = selected.agent({ scope: selected.address, reference: { kind: 'current', name: reference.name, incarnation: reference.incarnation } });
  const latest = await agent.messageHistory();
  const older = await agent.messageHistory({ before: latest.nextBefore });
  const remaining = await agent.messageHistory({ before: older.nextBefore });
  const forward = await agent.messages({ limit: 20 });
  assert.deepEqual(latest.messages.map(row => row.id), ids.slice(-20));
  assert.deepEqual(older.messages.map(row => row.id), ids.slice(-40, -20));
  assert.deepEqual(remaining.messages.map(row => row.id).filter(id => ids.includes(id)), ids.slice(0, -40));
  assert.equal(older.liveAfter, null);
  assert.equal(remaining.nextBefore, null);
  assert(BigInt(latest.liveAfter.position) >= BigInt(ids.at(-1)));
  assert.deepEqual(forward.messages.map(row => row.id).filter(id => ids.includes(id)).slice(0, 5), ids.slice(0, 5));
  report.cases.push('Native SDK: 20 recent, 20 older, 7 remaining; oldest-first /messages unaffected');

  browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: 'reduce' });
  page.setDefaultTimeout(12000);
  page.on('pageerror', error => report.pageErrors.push(error.message));
  page.on('request', request => {
    const address = new URL(request.url());
    if (address.origin === url && address.pathname.startsWith('/api/v1')) {
      report.requests.push({ method: request.method(), path: address.pathname, query: Object.fromEntries(address.searchParams) });
    }
  });
  // Only /space assets are supplied from the frozen bundle, to allow replay
  // with the same browser bits independently of the native executable build.
  // All /api traffic goes to the real loopback listener with no browser mock.
  await page.route(url + '/space/**', async route => {
    const path = new URL(route.request().url()).pathname.slice('/space/'.length) || 'index.html';
    const asset = resolve(build, path);
    assert(asset.startsWith(build + sep));
    const bytes = await readFile(asset);
    report.assets[path] = createHash('sha256').update(bytes).digest('hex');
    const body = path === 'index.html' ? bytes.toString().replace('name="agentx-space-scope"', `name="agentx-space-scope" content="${scope}"`) : bytes;
    await route.fulfill({ body, contentType: media[extname(path)] ?? 'application/octet-stream' });
  });
  const route = new URLSearchParams({ agent: reference.name, incarnation: reference.incarnation, record: 'current', scope, generation, office: workspace });
  await page.goto(url + '/space/?' + route);
  const source = page.getByRole('region', { name: '@' + reference.name, exact: true });
  const history = source.locator('.conversation-history');
  const recentText = source.getByText(/原生样例 47：/);
  await recentText.waitFor();
  assert.equal(await source.getByText(/原生样例 07：/).count(), 0);
  assert(await history.evaluate(node => node.scrollHeight - (node.scrollTop + node.clientHeight) <= 6), 'first viewport is anchored to latest');
  const historyPath = '/api/v1/agents/' + reference.name + '/message-history';
  assert(report.requests.some(request => request.path === historyPath && !request.query.before));
  const olderPage = page.waitForResponse(response => new URL(response.url()).pathname === historyPath && new URL(response.url()).searchParams.has('before') && response.ok());
  await history.hover();
  await page.mouse.wheel(0, -15000);
  await olderPage;
  await source.getByText(/原生样例 20：/).waitFor();
  report.cases.push('Chromium + native: direct latest, then older by wheel');

  const draft = '复核完成后，请在原始窗口保留这份中文草稿。\n下一步只打开准确的分叉实例。';
  const textbox = source.getByRole('textbox', { name: 'Agent 消息草稿' });
  await textbox.fill(draft);
  await source.getByRole('button', { name: 'Fork 新 Agent', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Fork 新 Agent', exact: true });
  assert(await dialog.getByText(reference.incarnation, { exact: false }).count(), 'confirmation names exact source');
  const submit = dialog.getByRole('button', { name: /确认.*Fork|^Fork 新 Agent$/ });
  await submit.click();
  const openResult = page.getByRole('button', { name: '打开分叉会话', exact: false });
  await openResult.waitFor({ timeout: 35000 });
  const fork = report.requests.filter(request => request.method === 'POST' && request.path.endsWith('/fork'));
  assert.equal(fork.length, 1);
  const launched = (await selected.agents()).agents.find(row => row.address.reference.name !== reference.name && row.address.reference.kind === 'current');
  assert(launched, 'native scope contains the new fork');
  const forkRelations = await selected.agent(launched.address).launchRelations({ limit: 5 });
  assert.equal(forkRelations.origin, 'external');
  assert.equal(forkRelations.upstream, null);
  assert.deepEqual(forkRelations.forkSource, agent.address.reference);
  assert.equal(await textbox.inputValue(), draft);
  await openResult.click();
  await page.getByRole('region', { name: '@' + launched.address.reference.name, exact: true }).waitFor();
  assert.equal(new URL(page.url()).searchParams.get('incarnation'), launched.address.reference.incarnation);
  assert.equal(await textbox.inputValue(), draft);
  report.cases.push('Native Fork: one exact submission, separate Fork source/creator, deliberate open, original draft remains');
  await page.screenshot({ path: resolve(dirname(file), 'native-workbench.png') });
  assert.deepEqual(report.pageErrors, []);
  report.status = 'passed';
} catch (error) {
  report.error = error.stack ?? String(error);
  throw error;
} finally {
  await browser?.close();
  await writeFile(resolve(dirname(file), 'browser-report.json'), JSON.stringify(report, null, 2) + '\n');
}
