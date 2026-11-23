/** Exercise the built scoped entry and shared UI with only disposable cloud HTTP fixtures. */
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import puppeteer from 'puppeteer-core';

const [output, evidence] = process.argv.slice(2);
assert(output, 'Pass a built Agent Space directory');
const scope = 'atlas';
const admin = 'fixture-admin-token-not-for-production';
const member = 'fixture-scope-token-not-for-production';
const scopePath = `/scopes/${scope}`;
const entry = `${scopePath}/space/`;
const adminApi = `/api/v1/admin/scopes/${scope}`;
const scopeApi = `${scopePath}/api/v1`;
const generation = 'c2b45d62-3f39-48d1-9ba2-63a83792c6e4';
const old = { host: 'runner-01', name: 'worker-01', incarnation: '18446744073709551615' };
const current = { ...old, incarnation: '18446744073709551614' };
const launched = { host: current.host, name: 'cloud_worker', incarnation: '18446744073709551613' };
const unattended = { ...launched, name: 'unattended', incarnation: '18446744073709551611' };
const replacement = { ...launched, incarnation: '18446744073709551612' };
const peer = { host: 'runner-02', name: 'peer', incarnation: 'peer-v1' };
const human = { host: '~space', name: 'operator', incarnation: '~human' };
const agents = [current, ...Array.from({ length: 20 }, (_, i) => ({ ...peer, name: `worker-${String(i + 2).padStart(2, '0')}` }))];
const now = 1795334400;
const hosts = [{ name: current.host, online: true, last_seen: now }, { name: peer.host, online: false, last_seen: now - 300 }];
const calls = [], controls = [], sends = [], operations = new Map(), accepted = new Map();
let loseControl = false, loseSend = true, installation = 'fixture-installation', scopeGeneration = generation;
const match = (a, b) => ['host', 'name', 'incarnation'].every(key => a[key] === b[key]);
const message = (id, from, to, body, reply_to = null) => ({ id, from, to, body, created_at: now, intent: reply_to ? 'inform' : 'request', thread: null, reply_to });
const messages = [
  message('9223372036854775803', old, peer, 'Retired instance earlier message'),
  message('9223372036854775804', peer, old, '**Retired instance only**'),
];
const assetTypes = { '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const csp = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'";
const complete = (input, agent) => operations.set(input.request_id, { ...input, status: 'succeeded', claim_id: 'fixture-claim', outcome: { status: 'succeeded', agent } });

const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://loopback');
  const path = url.pathname;
  const send = (obj, status = 200) => response.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' }).end(JSON.stringify(obj));
  const fail = (status, code) => send({ code, message: code }, status);
  function page(items) {
    const after = url.searchParams.get('after');
    const remaining = items.filter(item => !after || item.name > after).sort((a, b) => a.name.localeCompare(b.name, 'en'));
    const rows = remaining.slice(0, Number(url.searchParams.get('limit') ?? '20'));
    return send({ items: rows, next: rows.length < remaining.length ? rows.at(-1).name : null });
  }
  async function body() {
    let raw = '';
    for await (const chunk of request) raw += chunk;
    return JSON.parse(raw);
  }
  try {
    if (path === entry || path === '/space/') {
      const index = await readFile(join(output, 'index.html'), 'utf8');
      return response.writeHead(200, { 'Content-Type': 'text/html', 'Content-Security-Policy': csp, 'Cache-Control': 'no-store' }).end(index);
    }
    if (/^\/space\/(?:assets|tools)\/[A-Za-z0-9_./-]+$/.test(path)) {
      const extension = path.slice(path.lastIndexOf('.'));
      if (!assetTypes[extension]) return response.writeHead(404).end();
      return response.writeHead(200, { 'Content-Type': assetTypes[extension] }).end(await readFile(join(output, path.slice(7))));
    }
    if (path === '/favicon.ico') return response.writeHead(404).end();
    const isAdmin = path === adminApi || path.startsWith(adminApi + '/');
    const isScope = path.startsWith(scopeApi + '/');
    const audience = request.headers.authorization === `Bearer ${admin}` ? 'admin' : request.headers.authorization === `Bearer ${member}` ? 'scope' : 'none';
    calls.push({ path, method: request.method, query: Object.fromEntries(url.searchParams), audience });
    if (!isAdmin && !isScope) return fail(404, 'unsupported_path');
    if (isAdmin ? audience !== 'admin' : audience !== 'scope') return fail(401, 'unauthorized');
    if (path === `${scopeApi}/descriptor`) return send({ scope_id: scope, generation: scopeGeneration, protocol_version: 1 });
    if (path === adminApi) return send({ id: scope, scope_url: `http://127.0.0.1:${server.address().port}${scopePath}`, generation: scopeGeneration,
      available: true, credential_version: 1, created_at: now, agents: agents.length, hosts: hosts.length, online_hosts: 1 });
    if (/\/(hosts|agents)$/.test(path)) {
      const host = url.searchParams.get('host');
      return page(path.endsWith('/hosts') ? hosts : agents.filter(agent => !host || agent.host === host));
    }
    if (path === `${adminApi}/hosts/${current.host}/capabilities`) return send({
      host: current.host, generation: scopeGeneration, installation_id: installation, online: true, observed_at: now,
      workspaces: [{ alias: 'studio', default_available: false, default_reason: 'default_unavailable',
        roles: [{ name: 'reviewer', available: true, reason: null }, { name: 'unready', available: false, reason: 'tool_unavailable' }] }],
    });
    if (path === `${adminApi}/operations` && request.method === 'POST') {
      const input = await body();
      controls.push(input);
      operations.set(input.request_id, { ...input, status: 'queued', claim_id: null, outcome: null });
      if (loseControl) { loseControl = false; return fail(503, 'fixture_unconfirmed'); }
      return send(operations.get(input.request_id));
    }
    if (path.startsWith(`${adminApi}/operations/`) && request.method === 'GET') {
      const value = operations.get(decodeURIComponent(path.split('/').at(-1)));
      return value ? send(value) : fail(404, 'not_found');
    }
    if (path.endsWith('/messages') && request.method === 'GET') {
      if (url.searchParams.get('generation') !== scopeGeneration) return fail(409, 'stale_generation');
      const exact = { host: url.searchParams.get('host'), name: url.searchParams.get('name'), incarnation: url.searchParams.get('incarnation') };
      if (![old, launched, ...agents].some(item => match(exact, item))) return fail(404, 'not_found');
      const before = url.searchParams.get('before');
      const sorted = messages.filter(item => (!before || BigInt(item.id) < BigInt(before)) && (match(exact, item.from) || match(exact, item.to)))
        .sort((a, b) => BigInt(a.id) < BigInt(b.id) ? 1 : -1);
      const rows = match(old, exact) ? sorted.slice(0, 1) : sorted;
      return send({ items: rows, next: rows.length < sorted.length ? rows.at(-1).id : null });
    }
    if (path === `${adminApi}/messages` && request.method === 'POST') {
      const input = await body();
      sends.push(input);
      if (input.generation !== scopeGeneration) return fail(409, 'stale_generation');
      if (accepted.has(input.request_id)) {
        const original = accepted.get(input.request_id);
        assert.deepEqual(original.input, input);
        return send(original.row);
      }
      if (!agents.some(agent => match(agent, input.to))) return fail(404, 'not_found');
      const row = message('9223372036854775806', human, input.to, input.body);
      accepted.set(input.request_id, { input, row });
      messages.push(row);
      if (loseSend) { loseSend = false; return fail(503, 'fixture_unconfirmed'); }
      return send(row);
    }
    return fail(404, 'unsupported_path');
  } catch (error) {
    if (!response.headersSent) fail(500, 'fixture_error');
    else response.end();
    console.error(error.message);
  }
});

await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await puppeteer.launch({ executablePath: process.env.CHROME_BIN ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const page = await browser.newPage();
const errors = [];
page.on('pageerror', error => errors.push(error.message));
page.setDefaultTimeout(12000);
let stage = 'entry';
try {
  if (evidence) await mkdir(evidence, { recursive: true });
  const screenshot = async name => { if (evidence) await page.screenshot({ path: join(evidence, `${name}.png`) }); };
  await page.setViewport({ width: 1280, height: 800 });
  await page.emulateMediaFeatures([{ name: 'prefers-reduced-motion', value: 'reduce' }]);
  const text = (selector, value) => page.waitForFunction((selector, value) => document.querySelector(selector)?.textContent.includes(value), {}, selector, value);
  const reader = '.workspace-reader';
  const dialog = 'dialog[open]';
  const draft = '[aria-label="Agent 消息草稿"]';
  const openCommand = label => button(label, dialog);
  async function button(label, within = 'body') {
    const match = await page.waitForFunction((label, within) => [...document.querySelectorAll(`${within} button`)]
      .find(node => !node.disabled && node.getClientRects().length && (node.getAttribute('aria-label') ?? node.textContent.trim()) === label), {}, label, within);
    try { await match.asElement().click(); } finally { await match.dispose(); }
  }
  async function connect(token, retention = '90') {
    await page.waitForSelector('.cloud-connect input');
    await page.select('.cloud-connect label:nth-of-type(3) select', retention);
    await page.$eval('.cloud-connect input', input => { input.focus(); input.select(); });
    await page.type('.cloud-connect input', token);
    await button('连接');
  }
  async function exactLink(agent, options = {}) {
    await page.goto(`${origin}${entry}?${new URLSearchParams({ ...agent, view: 'list', ...options })}`);
  }
  async function openInfo() {
    const selector = `${reader} [aria-label="会话详情"]`;
    await page.waitForSelector(selector);
    const open = await page.$eval(selector, element => element.parentElement.open);
    if (!open) await page.click(selector);
  }
  const assertRoute = agent => {
    const url = new URL(page.url());
    assert.equal(url.pathname, entry);
    for (const [key, value] of Object.entries({ ...agent, generation })) assert.equal(url.searchParams.get(key), value);
  };

  await page.goto(`${origin}${entry}?access=admin&supply=demo`);
  await page.waitForSelector('.cloud-connect input');
  assert.equal(calls.length, 0);
  assert.equal(await page.$eval('.cloud-deployment-link', a => a.getAttribute('href')), '/deployment');
  await connect(member);
  await text('[role=alert]', '令牌无效');
  assert.equal(calls.length, 1);
  await connect(admin);
  stage = 'shared admin scene and roster';
  await page.waitForSelector('.space-scene-stage canvas');
  await text('.office-brand', '云端 Space');
  assert.equal(await page.$('.cloud-workbench'), null);
  await screenshot('cloud-space-desktop');
  await button('聊天', '.presentation-switch');
  await button('加载更多成员');
  await text('.hall-directory-items', '@worker-21');
  await page.select('[aria-label="工作区范围"]', 'host:runner-01');
  await text('.hall-directory-items', '@worker-01');
  assert(!await page.$eval('.hall-directory-items', row => row.textContent.includes('@worker-21')));
  await button(`@${current.name} · ${current.host} · ${current.incarnation}`, '.hall-directory');
  await text(reader, '@worker-01');
  assertRoute(current);

  stage = 'shared role picker and original cloud launch receipt';
  await button('新建 Agent');
  await page.waitForSelector(`${dialog} option[value="reviewer"]`);
  assert.equal(await page.$eval(`${dialog} option[value="unready"]`, node => node.disabled), true);
  assert.equal(await page.$eval(`${dialog} button[type=submit]`, node => node.disabled), true, 'Unavailable default role');
  await page.select(`${dialog} [aria-label="新 Agent 岗位"]`, 'reviewer');
  await page.type(`${dialog} [aria-label="新 Agent 名称"]`, launched.name);
  await page.type(`${dialog} textarea`, 'Read the scoped notes.');
  await screenshot('cloud-space-launch');
  await openCommand('启动');
  await page.waitForSelector(dialog, { hidden: true });
  const [launch] = controls;
  assert.deepEqual(launch, { generation, host: current.host, installation_id: installation,
    request_id: launch.request_id, command: { kind: 'launch', workspace: 'studio', role: 'reviewer', name: launched.name, title: null, initial_prompt: 'Read the scoped notes.' } });
  await page.click('.live-receipts summary');
  await button('查询原回执', '.live-receipts');
  await text('.live-receipts', '等待主机接收');
  assert.equal(controls.length, 1);
  complete(launch, launched);
  agents.push(launched);
  await button('查询原回执', '.live-receipts');
  await text('.live-receipts', '启动 · 已完成');
  await page.click('.live-receipts summary');
  // The exact successful receipt itself opens the launched reader.
  await page.waitForFunction(label => [...document.querySelectorAll('.workspace-reader h1')].some(node => node.textContent === label), {}, '@' + launched.name);
  assertRoute(launched);
  await button('刷新真实数据', '.live-status');
  await text('.hall-directory-items', '@' + launched.name);

  stage = 'late launch receipt respects deliberate navigation';
  loseControl = true;
  await button('新建 Agent');
  await page.waitForSelector(`${dialog} option[value="reviewer"]`);
  await page.select(`${dialog} [aria-label="新 Agent 岗位"]`, 'reviewer');
  await page.type(`${dialog} [aria-label="新 Agent 名称"]`, unattended.name);
  await openCommand('启动');
  await text(dialog, '提交结果未确认');
  const lateLaunch = controls.at(-1);
  await page.keyboard.press('Escape');
  await page.waitForSelector(dialog, { hidden: true });
  await button(`@${current.name} · ${current.host} · ${current.incarnation}`, '.hall-directory');
  assertRoute(current);
  complete(lateLaunch, unattended);
  agents.push(unattended);
  await page.click('.live-receipts summary');
  await button('查询原回执', '.live-receipts');
  await page.waitForFunction(id => [...document.querySelectorAll('.live-receipts section')].some(row => row.textContent.includes(id) && row.textContent.includes('已完成')), {}, lateLaunch.request_id);
  assertRoute(current);
  assert.equal(controls.length, 2);
  await page.click('.live-receipts summary');
  await button(`@${launched.name} · ${launched.host} · ${launched.incarnation}`, '.hall-directory');

  stage = 'shared composer, replay identity and agent reply';
  const pane = `[aria-label="@${launched.name}"] ${reader}`;
  await page.waitForSelector(`${pane} ${draft}`);
  assert.equal(await page.$eval(`${pane} [aria-label="添加附件"]`, node => node.disabled), true);
  assert.equal(await page.$eval(`${pane} .record-channels button:last-child`, node => node.disabled), true);
  const outgoing = 'Please review the scoped change.';
  await page.type(`${pane} ${draft}`, outgoing);
  await button('发送 AgentX 消息', pane);
  await text(pane, '发送结果未确定');
  assert.equal(await page.$eval(`${pane} ${draft}`, node => node.value), outgoing);
  await button('确认原发送', pane);
  await text(pane, '云端已接收 · #9223372036854775806');
  assert.equal(sends.length, 2);
  assert.deepEqual(sends[0], sends[1]);
  assert.deepEqual(sends[0], { generation, request_id: sends[0].request_id, to: launched, body: outgoing });
  assert.equal(accepted.size, 1);
  assert.equal(await page.$eval(`${pane} ${draft}`, node => node.value), '');
  messages.push(message('9223372036854775807', launched, human, 'Agent reply persisted.', '9223372036854775806'));
  await button('刷新真实数据', '.live-status');
  await text(pane, 'Agent reply persisted.');
  await text(pane, outgoing);
  await screenshot('cloud-space-chat');

  stage = 'reload admitted admin on exact long identity';
  const shared = page.url();
  const beforeReload = sends.length;
  await page.reload();
  await text(reader, 'Agent reply persisted.');
  assertRoute(launched);
  assert.equal(page.url(), shared);
  assert.equal(sends.length, beforeReload);
  await openInfo();
  await text(`${reader} .reader-facts`, launched.incarnation);
  await text(`${reader} .reader-facts`, generation);

  stage = 'installation rotation and deliberate exact stop';
  await button('停止 Agent', reader);
  await text(dialog, installation);
  installation = 'new-fixture-installation';
  await openCommand('确认停止 Agent');
  await text(dialog, '主机安装已变化');
  assert.equal(controls.length, 2);
  await page.keyboard.press('Escape');
  await page.waitForSelector(dialog, { hidden: true });
  await button('停止 Agent', reader);
  await text(dialog, installation);
  await openCommand('确认停止 Agent');
  await page.waitForSelector(dialog, { hidden: true });
  assert.equal(controls.length, 3);
  const stop = controls.at(-1);
  assert.deepEqual(stop.command, { kind: 'stop', name: launched.name, incarnation: launched.incarnation });
  assert.equal(stop.host, launched.host);
  assert.equal(stop.installation_id, installation);
  complete(stop, launched);
  agents.splice(agents.findIndex(agent => match(agent, launched)), 1, replacement);
  await page.click('.live-receipts summary');
  const stopReceipt = await page.waitForFunction(() => [...document.querySelectorAll('.live-receipts section')].find(node => node.textContent.startsWith('停止'))?.querySelector('button'));
  await stopReceipt.asElement().click();
  await stopReceipt.dispose();
  await text('.live-receipts', '停止 · 已完成');
  await page.click('.live-receipts summary');
  await page.reload();
  await text(reader, 'Agent reply persisted.');
  assertRoute(launched);
  await openInfo();
  assert.equal(await page.$eval(`${reader} .reader-facts button`, node => node.disabled), true);
  assert.equal(await page.$eval(`${reader} ${draft}`, node => node.readOnly), true);
  assert(!calls.some(call => call.path.endsWith('/messages') && call.query.incarnation === replacement.incarnation));

  stage = 'read-only scoped link, mobile layout, older records';
  await page.setViewport({ width: 320, height: 720, isMobile: true, hasTouch: true });
  await exactLink(old);
  await connect(member, '0');
  await text(reader, 'Retired instance only');
  assertRoute(old);
  assert.equal(await page.$eval('[aria-label="新建 Agent"]', node => node.disabled), true);
  assert.equal(await page.$eval(`${reader} ${draft}`, node => node.readOnly), true);
  await screenshot('cloud-space-narrow');
  assert.equal(await page.$eval('body', node => node.scrollWidth), 320);
  await button('查看更早消息', reader);
  await text(reader, 'Retired instance earlier message');
  assert(calls.some(call => call.path === `${scopeApi}/messages` && call.query.before === '9223372036854775804' && call.query.incarnation === old.incarnation));
  await page.reload();
  await page.waitForSelector('.cloud-connect input');

  stage = 'stale generation link and authorization separation';
  await exactLink(current, { generation: 'other-generation' });
  await connect(member, '0');
  await text('.context-error', '已失效');
  assert(!calls.some(call => call.path === `${scopeApi}/messages` && call.query.incarnation === current.incarnation));
  assert.equal(await page.evaluate(([admin, member]) => {
    const values = [location.href, document.cookie, ...Object.values(sessionStorage)];
    return values.some(value => value.includes(admin) || value.includes(member));
  }, [admin, member]), false);
  assert(calls.filter(call => call.audience === 'scope').every(call => call.method === 'GET'));
  assert(calls.every(call => call.path === adminApi || call.path.startsWith(adminApi + '/') || call.path.startsWith(scopeApi + '/')));
  assert(calls.some(call => call.path === `${adminApi}/messages` && call.method === 'GET' && call.query.incarnation === launched.incarnation));

  stage = 'native App demo entry remains separate from cloud credentials';
  const native = await browser.newPage();
  const nativeCalls = [];
  native.on('request', request => nativeCalls.push(new URL(request.url()).pathname));
  await native.goto(`${origin}/space/?supply=demo`);
  await native.waitForFunction(() => document.querySelector('.office-brand')?.textContent.includes('Agent 空间'));
  await native.waitForSelector('.space-scene-stage canvas');
  assert.equal(await native.$('.cloud-connect'), null);
  assert(!nativeCalls.some(path => path.startsWith('/api/v1/admin/') || path.startsWith('/scopes/')));
  await native.close();
  assert.deepEqual(errors, []);
  console.log('Cloud Space browser: shared App, role launch, real control receipts, message replay and reply, retained login, exact long identities, stop, older scope history and 320px reader.');
} catch (error) {
  if (evidence) await page.screenshot({ path: join(evidence, 'cloud-space-failure.png') }).catch(() => {});
  throw new Error(`Cloud Space browser failed at ${stage} (${controls.length} controls, ${sends.length} sends)`, { cause: error });
} finally {
  await browser.close();
  await new Promise(resolve => server.close(resolve));
}
