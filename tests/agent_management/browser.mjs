/** Actual embedded management page. No API mocks, injected state or paid Provider. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { checkpoint } from '../collaboration_workbench/browser_checkpoint.mjs';

const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
const origin = new URL(config.url);
assert.equal(origin.hostname, '127.0.0.1');
assert(origin.port && Number(origin.port) > 1024);
assert(config.evidence && config.generation && config.project);
await mkdir(config.evidence, { recursive: false });
const require = createRequire(import.meta.url);
const driverPath = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
const { chromium } = require(driverPath);
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',
  headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
const page = await context.newPage();
page.setDefaultTimeout(25000);
const steps = [];
const traffic = [];
const errors = [];
let stage = 'open';
let failure;
page.on('pageerror', error => errors.push(error.message));
page.on('request', request => {
  const url = new URL(request.url());
  assert.equal(url.origin, origin.origin, `unexpected external request: ${url}`);
  if (url.pathname.startsWith('/api/')) traffic.push({ kind: 'request', method: request.method(), url: request.url(), body: request.postData() });
});
page.on('response', response => {
  if (new URL(response.url()).pathname.startsWith('/api/')) traffic.push({ kind: 'response', status: response.status(), url: response.url() });
});

async function api(path, query = {}) {
  const url = new URL(path, origin);
  for (const [key, value] of Object.entries({ generation: config.generation, ...query })) if (value != null) url.searchParams.set(key, value);
  const response = await context.request.get(url.href);
  const body = await response.json();
  assert.equal(response.status(), 200, JSON.stringify(body));
  return body;
}
async function until(description, observe, timeout = 30000) {
  const end = Date.now() + timeout;
  let result;
  while (Date.now() < end) {
    result = await observe();
    if (result) return result;
    await new Promise(resolve => setTimeout(resolve, 150));
  }
  throw new Error(`${description}: timed out; last=${JSON.stringify(result)}`);
}
async function agents() { return (await api('/api/v1/agents')).agents; }
function queryRef(agent) { return { incarnation: agent.incarnation, stopped_event: agent.stopped_event }; }
async function messages(agent) { return (await api(`/api/v1/agents/${agent.name}/messages`, queryRef(agent))).messages; }
async function receipt(id) {
  return until(`operation ${id}`, async () => {
    const value = await api(`/api/v1/operations/${id}`);
    if (['Failed', 'Unknown'].includes(value.status)) throw new Error(JSON.stringify(value));
    return value.status === 'Succeeded' ? value : null;
  });
}
function button(name, root = page) { return root.getByRole('button', { name, exact: true }); }
async function capture(id, meaning, facts) {
  steps.push(await checkpoint(page, config.evidence, id, meaning, facts));
  await writeFile(`${config.evidence}/steps.json`, JSON.stringify(steps, null, 2) + '\n');
}
async function visibleText(text) {
  await until(`visible ${text}`, async () => (await page.locator('body').innerText()).includes(text));
}
async function submitByClick(control, suffix) {
  const responsePromise = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname.endsWith(suffix));
  await control.click();
  const response = await responsePromise;
  const body = await response.json();
  assert.equal(response.status(), 202, JSON.stringify(body));
  return receipt(body.request_id);
}

try {
  await page.goto(config.url, { waitUntil: 'domcontentloaded' });
  await button('＋ 启动').waitFor();
  await capture('B00-empty', 'Empty scope exposes launch action and scoped connection.', { agents: await agents() });

  stage = 'browser-launch';
  await button('＋ 启动').click();
  const dialog = page.getByRole('dialog');
  await dialog.waitFor();
  await dialog.getByRole('combobox', { name: '配置 / 工具', exact: true }).selectOption('tool:qoder');
  await dialog.getByLabel('职责标题', { exact: true }).fill('browser-lifecycle');
  await dialog.getByLabel('初始任务', { exact: true }).fill('browser initial fixture prompt');
  const launched = await submitByClick(button('启动', dialog), '/api/v1/agents');
  const name = launched.result.Launched.agents[0];
  assert(name && launched.result.Launched.failed === 0, JSON.stringify(launched));
  const live = await until('browser launched Agent session', async () => (await agents()).find(view => view.agent.name === name && view.can_send && view.session_id));
  await page.getByRole('button', { name: new RegExp(name) }).first().click();
  const composer = page.getByRole('textbox', { name: '消息内容', exact: true });
  await composer.waitFor();
  await composer.fill('browser draft retained across polling');
  await until('two polling rounds', async () => traffic.filter(item => item.kind === 'request' && item.url.includes(`/agents/${name}?`)).length >= 3);
  assert.equal(await composer.inputValue(), 'browser draft retained across polling');
  await page.reload({ waitUntil: 'domcontentloaded' });
  await composer.waitFor();
  assert.equal(await composer.inputValue(), 'browser draft retained across polling');
  await capture('B01-selection-draft', 'Selected live execution and draft survive polling and ordinary reload.', { agent: live.agent });

  stage = 'browser-message-reply';
  const text = 'browser public message one';
  await composer.fill(text);
  const sent = await submitByClick(button('发送'), '/messages');
  const sentId = sent.result.Sent.message_id;
  await until('browser native Provider reply', async () => (await messages(live.agent)).find(item => item.reply_to === sentId));
  await visibleText(`fixture reply #${sentId}`);
  await capture('B02-send-reply', 'Send receipt and actual native fixture reply are visible on the selected execution.', { sent, messages: await messages(live.agent) });

  stage = 'browser-lost-response';
  let dropped;
  let intercepted = 0;
  const lostText = 'browser response dropped after admitted message';
  const pattern = '**/api/v1/agents/*/messages';
  await page.route(pattern, async route => {
    if (route.request().method() !== 'POST') { await route.continue(); return; }
    intercepted++;
    const body = route.request().postDataJSON();
    const upstream = await route.fetch();
    assert.equal(upstream.status(), 202);
    dropped = { body, response: await upstream.json() };
    await route.abort('failed');
  });
  await composer.fill(lostText);
  await button('发送').click();
  await until('upstream admission before dropped reply', async () => dropped);
  await page.unroute(pattern);
  await page.reload({ waitUntil: 'domcontentloaded' });
  const recovered = await receipt(dropped.body.request_id);
  await until('one reply to recovered request', async () => (await messages(live.agent)).find(item => item.reply_to === recovered.result.Sent.message_id));
  const matchingPosts = traffic.filter(item => item.kind === 'request' && item.method === 'POST' && item.body && JSON.parse(item.body).request_id === dropped.body.request_id);
  assert.equal(intercepted, 1);
  assert.equal(matchingPosts.length, 1, 'reload retransmitted an admitted write');
  assert.equal((await messages(live.agent)).filter(item => item.text === lostText).length, 1);
  await visibleText(`fixture reply #${recovered.result.Sent.message_id}`);
  await capture('B03-lost-reply-reload', 'Reload recovers the original operation after a real admitted POST response is lost; one effect.', { recovered, originalRequest: dropped.body, intercepted });

  stage = 'browser-transcript';
  await button('模型记录').click();
  await button('读取记录').click();
  await visibleText('browser initial fixture prompt');
  await capture('B04-transcript', 'Async transcript displays the local Provider conversation.', { agent: live.agent });

  stage = 'browser-mobile';
  await page.setViewportSize({ width: 390, height: 844 });
  await visibleText(name);
  const geometry = await page.evaluate(() => ({ width: innerWidth, scroll: document.documentElement.scrollWidth,
    controls: [...document.querySelectorAll('button')].filter(element => element.getBoundingClientRect().width > 0).map(element => ({ text: element.innerText, name: element.getAttribute('aria-label') })) }));
  assert(geometry.scroll <= geometry.width + 1, JSON.stringify(geometry));
  await capture('B05-mobile', 'Same selected execution, contextual actions and transcript remain reachable on a narrow viewport.', geometry);
  await page.setViewportSize({ width: 1440, height: 900 });

  stage = 'browser-stop-resume';
  await button('停止').click();
  await submitByClick(button('确认停止'), '/stop');
  const history = await until('stopped snapshot', async () => (await agents()).find(view => view.agent.name === name && view.agent.stopped_event));
  await page.getByRole('button', { name: new RegExp(name) }).first().click();
  await button('恢复').waitFor();
  await capture('B06-stopped-history', 'Stopped execution remains selectable with its conversation and resume action.', history);
  const resumed = await submitByClick(button('恢复'), '/resume');
  const successorName = resumed.result.Launched.agents[0];
  const successor = await until('resumed live execution', async () => (await agents()).find(view => view.agent.name === successorName && view.agent.incarnation && view.can_stop));
  assert.notDeepEqual(successor.agent, live.agent);
  assert.equal(successor.session_id, history.session_id);
  await page.getByRole('button', { name: new RegExp(`${successorName}.*(active|listening|pending)`) }).first().click();
  await capture('B07-resumed', 'Resume starts a successor and preserves the original stopped snapshot.', { history, successor, resumed });
  assert.deepEqual(errors, []);
} catch (error) {
  failure = { stage, message: error.message, stack: error.stack };
  try { await capture('failure', 'Failure state; not an accepted checkpoint.', failure); } catch (captureError) { failure.captureError = captureError.message; }
} finally {
  await writeFile(`${config.evidence}/traffic.json`, JSON.stringify(traffic, null, 2) + '\n');
  await writeFile(`${config.evidence}/report.json`, JSON.stringify({ status: failure ? 'fail' : 'pass', failure, errors, steps,
    driver: { playwright: require(`${driverPath}/package.json`).version, chromium: browser.version() },
    limits: 'Local fixture Provider only. No mocked HTTP result. Physical mobile, screen reader and paid-model behavior are unverified.' }, null, 2) + '\n');
  await context.tracing.stop({ path: `${config.evidence}/trace.zip` });
  await context.close();
  await browser.close();
}
if (failure) { console.error(JSON.stringify(failure)); process.exitCode = 1; }
