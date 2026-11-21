/** Slice-1 independent browser cases against an already served static artifact; no server ownership. */
import { chromium } from '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
const base = process.env.AGENT_SPACE_URL ?? 'http://127.0.0.1:18968';
const output = process.env.AGENT_SPACE_EVIDENCE ?? '/tmp/agent-space-slice1-browser';
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox'] });
const results = [], requests = [], errors = [], geometry = [];
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.on('pageerror', error => errors.push(String(error)));
page.on('request', request => requests.push({ url: request.url(), type: request.resourceType(), method: request.method() }));
const button = name => page.getByRole('button', { name, exact: typeof name === 'string' });
const draft = () => page.getByRole('textbox', { name: 'Agent 消息草稿' });
const history = () => page.getByRole('log', { name: '会话记录' });
const web = '/projects/atlas/web';
const scope = id => page.getByLabel('工作区范围').selectOption(id);
const view = label => page.getByRole('navigation', { name: '办公室内容' }).getByRole('button', { name: new RegExp(`^${label}( |$)`) }).click();
const person = name => page.locator('.hall-directory-person').filter({ hasText: `@${name}` }).click();
async function loaded() { await page.goto(base); await page.locator('canvas').waitFor(); }
async function chatOffice(id = web) { await loaded(); await scope(id); await button('聊天').click(); }
async function check(name, action) {
  if (process.env.AGENT_SPACE_CASES && !new RegExp(process.env.AGENT_SPACE_CASES).test(name)) return;
  try { await action(); results.push({ name, status: 'passed' }); }
  catch (error) { results.push({ name, status: 'failed', error: String(error) }); await page.screenshot({ path: `${output}/failure-${results.length}.png` }); }
  console.log(JSON.stringify(results.at(-1)));
}
async function available(label) {
  const metrics = await page.evaluate(() => ({ width: innerWidth, height: innerHeight, document: document.documentElement.scrollWidth, body: document.body.scrollWidth }));
  assert(metrics.document <= metrics.width && metrics.body <= metrics.width, JSON.stringify(metrics));
  for (const locator of [draft(), button('发送 AgentX 消息'), button('返回总览')]) {
    const box = await locator.boundingBox(); assert(box && box.x >= 0 && box.y >= 0 && box.x + box.width <= metrics.width + 1 && box.y + box.height <= metrics.height + 1, `${label}: ${JSON.stringify(box)}`);
    if (await locator.isEnabled()) await locator.click({ trial: true, timeout: 3000 });
  }
  geometry.push({ label, ...metrics, composer: await draft().boundingBox() });
}
await check('S1 default known office scene, exact counts, empty/global/unknown', async () => {
  await loaded(); assert.equal(await page.getByLabel('工作区范围').inputValue(), '');
  assert.equal(await page.locator('.hall-room-name').count(), 5);
  assert.equal(await page.locator('.office-actor').count(), 8);
  assert(await page.locator('canvas').evaluate(canvas => canvas.width > 0 && canvas.height > 0));
  await page.screenshot({ path: `${output}/desktop-offices.png` });
  for (const [id, count] of [['/projects/atlas', 1], [web, 5], ['/projects/empty', 0], ['$global', 1], ['$unknown', 1]]) {
    await scope(id); assert.equal(await page.getByLabel('匹配会话数').innerText(), `${count}/${count}`);
    assert.equal(await page.locator('.hall-room-name').count(), 1);
    assert.equal(await page.locator('.office-actor').count(), count);
  }
  await scope('/projects/empty'); await view('岗位'); assert(await page.getByText('暂无成员使用此岗位', { exact: true }).isVisible());
  assert.equal(await page.locator('.office-actor').count(), 0);
});
await check('S2 posts origin/profile/snapshot and two named members; no fake person', async () => {
  await chatOffice(); await view('岗位');
  await page.getByRole('region', { name: '可用岗位' }).getByRole('button', { name: /fronter/ }).click();
  const detail = page.getByRole('region', { name: 'fronter 岗位详情' });
  assert((await detail.innerText()).includes('继承')); assert((await detail.innerText()).includes('codex-deep-current'));
  assert.equal(await detail.locator('.post-members button').count(), 2);
  assert((await detail.innerText()).includes('snapshot-1')); assert((await detail.innerText()).includes('definition-2'));
  await page.getByRole('region', { name: '可用岗位' }).getByRole('button', { name: /controller/ }).click();
  assert(await page.getByText('暂无成员使用此岗位', { exact: true }).isVisible());
  assert.equal(await draft().count(), 0);
  await page.getByRole('region', { name: '可用岗位' }).getByRole('button', { name: /fronter/ }).click();
  await detail.locator('.post-members button').filter({ hasText: '@luna' }).click();
  assert(await page.getByRole('heading', { name: '@luna · fronter', exact: true }).isVisible());
});
await check('S3/S4 membership versus cwd/role and unavailable Skills', async () => {
  await chatOffice('/projects/atlas'); await person('tobi'); await page.getByLabel('会话详情').click();
  const facts = page.locator('.reader-facts'); assert((await facts.innerText()).includes(web)); assert((await facts.innerText()).includes('未采用 Role'));
  await scope(web); await person('sira'); await page.getByLabel('会话详情').click();
  assert((await facts.innerText()).includes('/example/agentx-data/roles/reviewer.toml'));
  await button('返回总览').click(); await person('nora'); assert(await page.getByRole('heading', { name: '@nora · 未采用 Role', exact: true }).isVisible());
  await button('返回总览').click(); await person('riku'); assert(await page.getByRole('heading', { name: '@riku · 岗位未知', exact: true }).isVisible());
  await view('技能'); assert(await page.getByText('技能列表未提供', { exact: true }).isVisible());
  await scope('$unknown'); await person('keni'); await page.getByLabel('会话详情').click(); assert((await facts.innerText()).includes('归属未知'));
});
await check('S5 exact namesake history, failed/idle current and drafts across office/view', async () => {
  await chatOffice(); assert.equal(await page.locator('.hall-directory-person').count(), 5);
  await person('mora'); await draft().fill('current-mora'); await view('历史');
  assert.equal(await page.locator('.hall-directory-person').count(), 1); await person('mora'); assert.equal(await draft().getAttribute('readonly'), '');
  assert((await page.locator('.hall-conversation-heading').innerText()).includes('停止历史'));
  assert((await history().innerText()).includes('旧实例'));
  await view('成员'); await person('mora'); assert.equal(await draft().inputValue(), 'current-mora');
  await scope('/projects/atlas'); await view('岗位'); await button('继续阅读 @mora 当前实例').click();
  assert.equal(await draft().inputValue(), 'current-mora');
  await scope(web); await view('历史'); await person('mora'); assert.equal(await draft().inputValue(), '');
});
await check('S6 separate authored AgentX envelopes, Provider blocks, draft-only preview and unavailable branches', async () => {
  await chatOffice(); await person('luna');
  assert((await history().innerText()).includes('@tobi')); assert((await history().innerText()).includes('→ @luna')); assert((await history().innerText()).includes('→ @tobi'));
  assert.equal(await page.locator('.history-tool').count(), 0);
  await draft().fill('local-only-note'); await button('Transcript').click();
  assert.equal(await draft().count(), 0);
  await page.locator('.history-reasoning summary').click(); await page.locator('.history-tool summary').click(); assert(await page.locator('.tool-details pre').isVisible());
  await button('消息').click(); assert.equal(await draft().inputValue(), 'local-only-note'); assert(await button('发送 AgentX 消息').isDisabled()); assert.equal(await history().locator('article').count(), 2); assert(!(await history().innerText()).includes('local-only-note'));
  await button('返回总览').click(); await person('mora'); await button('Transcript').click(); assert(await page.getByText('记录未提供', { exact: true }).isVisible());
  await button('返回总览').click(); await person('sira'); await button('Transcript').click(); assert(await page.getByText('Transcript 为空', { exact: true }).isVisible());
  await button('返回总览').click(); await person('riku'); await button('Transcript').click(); assert(await page.getByText('读取失败', { exact: true }).isVisible());
  await button('消息').click(); assert(await page.getByText('暂无 AgentX 消息', { exact: true }).isVisible());
});
await check('S7 office-specific camera memory and resources return', async () => {
  await loaded(); await scope(web); await button('暂停动作').click(); await button('放大大厅').click();
  async function landmark() { await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))); return page.locator('.hall-room-name').first().boundingBox(); }
  const before = await landmark();
  await scope('/projects/atlas'); await button('缩小大厅').click(); await scope(web); assert.deepEqual(await landmark(), before);
  await view('岗位'); await view('成员'); assert.deepEqual(await landmark(), before);
  await button('聊天').click(); await button('场景').click(); assert.deepEqual(await landmark(), before);
});
for (const viewport of [{ width: 390, height: 844 }, { width: 320, height: 640 }]) {
  await check(`S8 narrow ${viewport.width}: office/posts/Skills reader modes, tabs, short composer`, async () => {
    await page.setViewportSize(viewport); await chatOffice(); await view('岗位');
    await page.getByRole('region', { name: '可用岗位' }).getByRole('button', { name: /fronter/ }).click();
    await page.getByRole('region', { name: 'fronter 岗位详情' }).locator('.post-members button').filter({ hasText: '@luna' }).click();
    await draft().fill(`narrow-${viewport.width}`); await available('narrow chat');
    await button('场景').click(); await available('narrow scene'); await button('放大阅读').click(); await available('expanded scene');
    await page.screenshot({ path: `${output}/narrow-${viewport.width}.png` });
    await page.setViewportSize({ width: viewport.width, height: 480 }); await available('short scene');
    await button('聊天').click(); await available('short chat'); assert.equal(await draft().inputValue(), `narrow-${viewport.width}`);
    assert(await button('发送 AgentX 消息').isDisabled()); assert(!(await history().innerText()).includes(`narrow-${viewport.width}`));
    await button('返回总览').click(); await view('技能'); assert(await page.getByText('技能列表未提供', { exact: true }).isVisible());
    await view('成员'); await person('luna'); assert.equal(await draft().inputValue(), `narrow-${viewport.width}`);
  });
}
await check('S6 no runtime requests or invented effects', async () => {
  assert.deepEqual(errors, []);
  assert.deepEqual(requests.filter(request => !request.url.startsWith(base) && !request.url.startsWith('data:')), []);
  assert.deepEqual(requests.filter(request => ['fetch', 'xhr', 'websocket', 'eventsource'].includes(request.type)), []);
});
await writeFile(`${output}/result.json`, JSON.stringify({ base, browser: browser.version(), results, geometry, errors, requests }, null, 2));
await context.close(); await browser.close(); if (results.some(result => result.status === 'failed')) process.exitCode = 1;
