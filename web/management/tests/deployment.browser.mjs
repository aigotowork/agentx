/** Exercise the built Management deployment entry against the isolated admin wire only. */
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import puppeteer from 'puppeteer-core';
import { createDeploymentFixture, demoAdminToken } from './deployment.fixture.mjs';

const [, , output, evidence] = process.argv;
if (!output) throw new Error('Pass a built Management directory');
const { server, state, scopes, writes, requests, operations, controlWrites } = createDeploymentFixture(output);
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await puppeteer.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server', '--host-resolver-rules=MAP deployment.test 127.0.0.1'] });
let stage = 'initial entry';

try {
  if (evidence) await mkdir(evidence, { recursive: true });
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.setViewport({ width: 1280, height: 800 });
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  assert.match(page.url(), /\/deployment$/);
  const screenshot = async name => {
    if (!evidence) return;
    await page.evaluate(() => Promise.allSettled((document.querySelector('dialog[open]')?.getAnimations({ subtree: true }) ?? [])
      .filter(animation => animation.effect?.getComputedTiming().endTime !== Infinity).map(animation => animation.finished)));
    await page.screenshot({ path: join(evidence, `${name}.png`) });
  };
  const text = (selector, value) => page.waitForFunction((selector, value) => document.querySelector(selector)?.textContent.includes(value), {}, selector, value);
  const click = async (label, selector = 'button') => {
    const handle = await page.waitForFunction((label, selector) => [...document.querySelectorAll(selector)].find(node => !node.disabled && node.getClientRects().length && !node.closest('dialog:not([open])') && (node.getAttribute('aria-label') ?? node.textContent.trim()) === label), {}, label, selector);
    try { await handle.asElement().click(); } finally { await handle.dispose(); }
  };
  const scope = async id => {
    const row = await page.waitForFunction(id => [...document.querySelectorAll('.deployment-scope-row')].find(node => node.querySelector('strong')?.textContent === id && node.getClientRects().length), {}, id);
    try { await row.asElement().click(); } finally { await row.dispose(); }
    await text('.deployment-scope-title h2', id);
  };
  const field = async (selector, value) => {
    await page.$eval(selector, input => { input.focus(); input.select(); });
    await page.keyboard.type(value);
  };

  stage = 'explicit admin authentication';
  const login = '.deployment-signin input';
  await page.waitForSelector(login);
  assert.equal(requests.length, 0);
  await field(login, 'incorrect-admin-token-for-the-fixture');
  await click('连接');
  await text('.deployment-signin [role=alert]', '管理员令牌无效');
  assert.equal(requests.length, 1);
  await page.select('.deployment-signin select', '0');
  await field(login, demoAdminToken);
  await click('连接');
  await text('.deployment-metrics', '23');
  await page.waitForSelector('.deployment-scope-row');
  await page.click('.deployment-identity summary');
  await text('.deployment-identity[open] dl', 'fixture-deployment');
  await text('.deployment-identity[open] dl', 'v1');
  await page.click('.deployment-identity summary');

  stage = 'scope collection and inventory states';
  await click('Scopes下一页');
  await text('.deployment-scope-row strong', 'stage-18');
  assert(requests.some(request => request.path.endsWith('/scopes') && request.after === 'stage-17'));
  await click('Scopes上一页');
  await scope('quarantine');
  await text('.deployment-detail .empty-state', 'Scope 清单暂不可用');
  assert(!requests.some(request => /quarantine\/(hosts|agents)$/.test(request.path)));
  await scope('field-notes');
  await text('.deployment-detail .empty-state', '暂无主机');
  state.failHosts = true;
  await scope('atlas');
  await text('.deployment-inventory [role=alert]', '无法读取');
  state.failHosts = false;
  await click('重试', '.deployment-inventory button');
  await text('.deployment-table tbody', 'runner-01');
  await screenshot('deployment-desktop');
  await click('主机下一页');
  await text('.deployment-table tbody', 'runner-21');
  await click('查看 runner-21 的 Agents');
  await text('.deployment-host-filter', 'runner-21');
  await text('.deployment-table tbody', 'worker-21');
  assert.equal(await page.$$eval('.deployment-table tbody tr', rows => rows.length), 1);
  assert(requests.some(request => request.path.endsWith('/agents') && request.host === 'runner-21' && request.after === null));
  await click('查看所有 Agents');
  await text('.deployment-table tbody', '18446744073709551600');
  await click('Agents下一页');
  await text('.deployment-table tbody', 'worker-21');
  assert(requests.some(request => request.path.endsWith('/hosts') && request.after === 'runner-20'));
  assert(requests.some(request => request.path.endsWith('/agents') && request.after === 'worker-20'));

  stage = 'live host catalogue and launch receipt recovery';
  await click('主机 22');
  await click('在 runner-01 启动 Agent');
  await text('dialog[open] .deployment-control-catalogue', '目录更新于');
  await page.select('dialog[open] select', 'studio');
  await page.waitForFunction(() => document.querySelectorAll('dialog[open] select').length === 2);
  assert.deepEqual(await page.$$eval('dialog[open] select:nth-of-type(1) option[value=operator]', options => options.map(option => [option.disabled, option.textContent])), [[true, 'operator（tool_unavailable）']]);
  await field('dialog[open] .deployment-control-identity input', 'requested_name');
  await field('dialog[open] .deployment-control-identity label:nth-child(2) input', 'Prepare summary');
  await page.type('dialog[open] textarea', 'Use the lab notes.');
  await screenshot('launch-catalogue');
  state.dropControlReply = true;
  await click('确认启动 Agent');
  await text('dialog[open] .deployment-write-note', '提交结果未确认');
  assert.equal(controlWrites.length, 1);
  const [launch] = controlWrites;
  assert.deepEqual(launch, { generation: 'fixture-generation-atlas', host: 'runner-01', installation_id: 'installation-runner-01',
    request_id: launch.request_id, command: { kind: 'launch', workspace: 'studio', name: 'requested_name', title: 'Prepare summary', initial_prompt: 'Use the lab notes.' } });
  await click('查询原回执');
  await text('dialog[open] .deployment-control-receipt', '等待主机接收');
  assert.equal(controlWrites.length, 1, 'Receipt lookup must not send another operation');
  const receipt = operations.get(launch.request_id);
  Object.assign(receipt, { status: 'succeeded', claim_id: 'claim-1', outcome: { status: 'succeeded',
    agent: { name: 'requested_name', host: 'runner-01', incarnation: '18446744073709551615' } } });
  await click('查询原回执');
  await text('dialog[open] .deployment-control-receipt', '18446744073709551615');
  await click('关闭', 'dialog[open] button');
  await click('操作回执');
  await field('.deployment-receipt-search input', launch.request_id);
  await click('查询回执');
  await text('.deployment-receipt-search .deployment-control-receipt', '已完成');
  await screenshot('original-receipt');

  stage = 'offline host and mobile exact stop';
  await click('主机 22');
  await click('主机下一页');
  await click('在 runner-21 启动 Agent');
  await text('dialog[open] .deployment-control-catalogue', '管理离线');
  assert.equal(await page.$eval('dialog[open] button[type=submit]', button => button.disabled), true);
  await click('取消', 'dialog[open] button');
  await page.setViewport({ width: 390, height: 675 });
  await click('Agents 21');
  await click('停止 worker-01');
  await text('dialog[open] .dialog-heading-copy p', '18446744073709551600');
  await text('dialog[open] .deployment-control-catalogue', 'installation-runner-01');
  assert.equal(await page.$eval('dialog[open]', node => node.scrollWidth <= node.clientWidth), true);
  await screenshot('mobile-exact-stop');
  await click('确认停止 Agent');
  await text('dialog[open] .deployment-control-receipt', '等待主机接收');
  assert.deepEqual(controlWrites[1].command, { kind: 'stop', name: 'worker-01', incarnation: '18446744073709551600' });
  assert.equal(controlWrites[1].installation_id, 'installation-runner-01');
  await click('关闭', 'dialog[open] button');
  await page.setViewport({ width: 1280, height: 800 });

  stage = 'creation and private credential receipt';
  await click('新建 Scope');
  await page.waitForSelector('dialog[open]');
  const idInput = 'dialog[open] input[placeholder]';
  const tokenInput = 'dialog[open] .deployment-credential-form input:not([placeholder])';
  const createToken = await page.$eval(tokenInput, input => input.value);
  assert.match(createToken, /^[A-Za-z0-9_-]{43}$/);
  await click('显示令牌');
  assert.equal(await page.$eval(tokenInput, input => input.type), 'text');
  await click('隐藏令牌');
  await field(idInput, 'studio');
  await click('创建 Scope');
  await text('dialog[open] .deployment-write-note', 'studio · v1');
  assert.deepEqual(Object.keys(writes[0].body).sort(), ['request_id', 'token']);
  assert.equal(writes[0].body.token, createToken);
  assert.equal(writes[0].method, 'PUT');
  assert.equal(await page.$eval(tokenInput, input => input.value), createToken);
  await click('完成');
  await page.waitForSelector('dialog[open]', { hidden: true });
  await text('.deployment-scope-url', 'https://deployment.example/scopes/studio');
  assert.equal(await page.$eval('.deployment-space-link', link => link.getAttribute('href')), '/scopes/studio/space/?access=admin');

  stage = 'mobile scope library and exact rotation recovery';
  await page.setViewport({ width: 390, height: 675 });
  await click('Scopes', '.deployment-back');
  await scope('atlas');
  await text('.deployment-credential', 'v4');
  assert.equal(await page.$eval('.deployment-collection', node => node.getClientRects().length), 0);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await screenshot('deployment-mobile');
  await click('轮换令牌');
  await page.waitForSelector('dialog[open]');
  assert.match(await page.$eval('dialog[open] .dialog-heading-copy p', node => node.textContent), /atlas · 当前版本 v4/);
  const rotationToken = await page.$eval(tokenInput, input => input.value);
  state.dropNextWrite = true;
  await click('确认轮换');
  await text('dialog[open] [role=alert]', '尚未确认操作结果');
  assert.equal(scopes.get('atlas').credential_version, 5);
  assert.equal(writes.length, 2, 'No automatic mutation retry');
  assert.equal(await page.$eval(tokenInput, input => input.readOnly), true);
  await page.keyboard.press('Escape');
  await text('dialog[open] .danger', '关闭并清除重试凭据');
  scopes.get('atlas').credential_version = 6; // Another operator changed it after this write committed.
  await click('重试同一请求');
  await text('dialog[open] .deployment-write-note', 'atlas · v5');
  assert.deepEqual(writes[1], writes[2]);
  assert.equal(writes[2].body.expected_version, 4);
  assert.equal(writes[2].body.token, rotationToken);
  await click('完成');
  await text('.deployment-credential', 'v6');

  stage = 'stale rotation conflict and session lifetime';
  await click('轮换令牌');
  await page.waitForSelector('dialog[open]');
  scopes.get('atlas').credential_version = 7;
  await click('确认轮换');
  await text('dialog[open] [role=alert]', '版本已变化');
  assert.equal(writes[3].body.expected_version, 6);
  assert.equal(scopes.get('atlas').credential_version, 7);
  await click('返回 Scope');
  await text('.deployment-credential', 'v7');
  const retained = await page.evaluate(() => JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage), location.href, document.cookie, document.body.innerText]));
  for (const credential of [demoAdminToken, createToken, rotationToken]) assert(!retained.includes(credential), 'Credentials must not persist or be rendered after dismissing');
  assert(requests.slice(1).every(request => request.authorized && request.path.startsWith('/api/v1/admin')));
  const beforeReload = requests.length;
  await page.reload();
  await page.waitForSelector(login);
  assert.equal(requests.length, beforeReload);
  await page.goto(`http://deployment.test:${server.address().port}/deployment`);
  await text('.deployment-signin [role=alert]', '请通过 HTTPS');
  assert.equal(await page.$(login), null);
  assert.equal(requests.length, beforeReload);
  assert.deepEqual(errors, []);
  console.log('Deployment Web: authentication, inventories, live catalogue, default launch, lost-response receipt lookup, mobile exact stop, credentials and rotation recovery');
} catch (error) {
  throw new Error(`Deployment browser failed at ${stage}`, { cause: error });
} finally {
  await browser.close();
  server.close();
}
