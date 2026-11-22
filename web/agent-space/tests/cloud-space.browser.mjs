/** Run the built Cloud Space against its agreed reader and control routes on loopback. */
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import puppeteer from 'puppeteer-core';

const [output, evidence] = process.argv.slice(2);
if (!output) throw new Error('Pass a built Agent Space directory');
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
const peer = { host: 'runner-02', name: 'operator', incarnation: 'peer-v1' };
const agents = [current, ...Array.from({ length: 20 }, (_, i) => ({ ...peer, name: `worker-${i + 2 < 10 ? '0' : ''}${i + 2}` }))];
const now = 1795334400;
const hosts = [{ name: current.host, online: true, last_seen: now }, { name: peer.host, online: false, last_seen: now - 300 }];
const calls = [], writes = [], operations = new Map();
let loseReply = true;
const assetTypes = { '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const csp = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'";
const exact = (url, agent) => ['host', 'name', 'incarnation'].every(key => url.searchParams.get(key) === agent[key]);

const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://loopback');
  const path = url.pathname;
  const send = (obj, status = 200) => response.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' }).end(JSON.stringify(obj));
  const fail = (status, code) => send({ code, message: code }, status);
  function page(items) {
    const after = url.searchParams.get('after');
    const remaining = after ? items.filter(item => item.name > after) : items;
    const limit = Number(url.searchParams.get('limit') ?? '20');
    const rows = remaining.slice(0, limit);
    return send({ items: rows, next: rows.length < remaining.length ? rows.at(-1).name : null });
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

    if (path === `${scopeApi}/descriptor`) return send({ scope_id: scope, generation, protocol_version: 1 });
    if (path === adminApi) return send({ id: scope, scope_url: `http://127.0.0.1:${server.address().port}${scopePath}`, generation,
      available: true, credential_version: 1, created_at: now, agents: agents.length, hosts: hosts.length, online_hosts: 1 });
    if (/\/(hosts|agents)$/.test(path)) {
      const host = url.searchParams.get('host');
      return page(path.endsWith('/hosts') ? hosts : agents.filter(agent => !host || agent.host === host));
    }
    if (path === `${adminApi}/hosts/${current.host}/capabilities`) return send({
      host: current.host, generation, installation_id: 'fixture-installation', online: true, observed_at: now,
      workspaces: [{ alias: 'studio', default_available: false, default_reason: 'default_unavailable',
        roles: [{ name: 'reviewer', available: true, reason: null }, { name: 'unready', available: false, reason: 'tool_unavailable' }] }],
    });
    if (path === `${adminApi}/operations` && request.method === 'POST') {
      let raw = '';
      for await (const chunk of request) raw += chunk;
      const input = JSON.parse(raw);
      writes.push(input);
      operations.set(input.request_id, { ...input, status: 'queued', claim_id: null, outcome: null });
      if (loseReply) {
        loseReply = false;
        response.writeHead(200, { 'Content-Type': 'application/json', 'Content-Length': 1000 });
        response.write('{"request_id":');
        setTimeout(() => request.socket.destroy(), 20);
        return;
      }
      return send(operations.get(input.request_id));
    }
    if (path.startsWith(`${adminApi}/operations/`) && request.method === 'GET') {
      const value = operations.get(decodeURIComponent(path.split('/').at(-1)));
      return value ? send(value) : fail(404, 'not_found');
    }
    if (path === `${scopeApi}/messages` && request.method === 'GET') {
      if (url.searchParams.get('generation') !== generation) return fail(409, 'stale_generation');
      if (exact(url, current)) return send({ items: [], next: null });
      if (!exact(url, old)) return fail(404, 'not_found');
      const before = url.searchParams.get('before');
      if (before && before !== '9223372036854775807') return fail(400, 'invalid_input');
      return send(before ? {
        next: null, items: [{ id: '9223372036854775806', from: old, to: peer, body: 'Retired instance earlier message', created_at: now - 70, intent: 'ack', thread: null, reply_to: null }],
      } : {
        next: '9223372036854775807', items: [{ id: '9223372036854775807', from: peer, to: old, body: '**Retired instance only**', created_at: now, intent: 'request', thread: 'review', reply_to: null }],
      });
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
  async function button(label, within = 'body') {
    const match = await page.waitForFunction((label, within) => [...document.querySelectorAll(`${within} button`)]
      .find(node => !node.disabled && node.getClientRects().length && !node.closest('dialog:not([open])') && (node.getAttribute('aria-label') ?? node.textContent.trim()) === label), {}, label, within);
    try { await match.asElement().click(); } finally { await match.dispose(); }
  }
  async function connect(token) {
    await page.waitForSelector('.cloud-connect input');
    await page.type('.cloud-connect input', token);
    await button('连接');
  }
  async function exactLink(agent, options = {}) {
    await page.goto(`${origin}${entry}?${new URLSearchParams({ ...agent, view: 'list', ...options })}`);
  }

  await page.goto(`${origin}${entry}?access=admin&supply=demo`);
  await page.waitForSelector('.cloud-connect input');
  assert.equal(calls.length, 0);
  assert.equal(await page.$eval('.cloud-header-actions a', a => a.getAttribute('href')), '/deployment');
  await connect(member);
  await text('[role=alert]', '令牌无效');
  assert.equal(calls.length, 1);
  await connect(admin);
  stage = 'admin roster and scene';
  await text('.cloud-session', '管理员访问');
  await page.waitForSelector('.cloud-scene canvas');
  stage = 'keyboard selection of a presented scene actor';
  const exactActor = JSON.stringify([current.host, current.name, current.incarnation]);
  // React inserts the buttons before the first measured frame. That initial frame
  // can temporarily hide the anchor and drop focus until the camera is laid out.
  const actor = await page.waitForFunction(key => {
    const scene = document.querySelector('.cloud-scene');
    const canvas = scene?.querySelector('canvas');
    const viewport = canvas?.getBoundingClientRect();
    const scale = Math.min(2, devicePixelRatio || 1);
    if (!viewport?.width || !viewport.height || canvas.width !== Math.round(viewport.width * scale)
      || canvas.height !== Math.round(viewport.height * scale)) return false;
    const button = [...scene.querySelectorAll('button.office-actor')].find(node => node.dataset.hallObject === key);
    if (!button || button.hidden || button.disabled || !button.getClientRects().length) return false;
    const bounds = button.getBoundingClientRect();
    return bounds.left >= viewport.left && bounds.top >= viewport.top &&
      bounds.right <= viewport.right && bounds.bottom <= viewport.bottom && button;
  }, {}, exactActor);
  assert.equal(await page.$('.cloud-scene [data-hall-object="human:bigboss"]'), null);
  await screenshot('cloud-space-desktop');
  try {
    await actor.asElement().focus();
    assert(await actor.evaluate(node => document.activeElement === node), 'The exact scene actor must own focus before Enter');
    await page.keyboard.press('Enter');
  } finally {
    await actor.dispose();
  }
  await text('.cloud-reader-heading', current.incarnation);
  assert.equal(new URL(page.url()).searchParams.get('generation'), generation);
  await text('.cloud-admin-agent', '云端消息需要 Scope 令牌');
  await button('下一页', '.cloud-pager');
  await text('.cloud-rail-content', '@worker-21');
  await button('上一页', '.cloud-pager');
  await text('.cloud-rail-content', '@worker-01');
  await button('主机', '.cloud-rail-nav');
  await text('.cloud-rail-content', '心跳离线');
  await button('查看 runner-01 的目录与启动选项', '.cloud-rail-content');
  stage = 'launch and original receipt';
  await page.waitForSelector('dialog[open] select');
  await page.select('dialog[open] select', 'studio');
  await page.waitForSelector('dialog[open] select:nth-of-type(1) option[value="reviewer"]');
  assert.equal(await page.$eval('dialog[open] option[value="unready"]', node => node.disabled), true);
  const role = await page.$$('dialog[open] select');
  await role[1].select('reviewer');
  await page.type('dialog[open] .deployment-control-identity input', 'cloud_worker');
  await page.type('dialog[open] textarea', 'Read the scoped notes.');
  await screenshot('cloud-space-launch');
  await button('确认启动 Agent', 'dialog[open]');
  await text('dialog[open] [role=status]', '提交结果未确认');
  assert.equal(writes.length, 1);
  const [launch] = writes;
  assert.deepEqual(launch, { generation, host: current.host, installation_id: 'fixture-installation',
    request_id: launch.request_id, command: { kind: 'launch', workspace: 'studio', role: 'reviewer', name: 'cloud_worker', initial_prompt: 'Read the scoped notes.' } });
  await button('查询原回执', 'dialog[open]');
  await text('dialog[open] .deployment-control-receipt', '等待主机接收');
  assert.equal(writes.length, 1);
  await button('关闭', 'dialog[open]');
  await button('回执', '.cloud-rail-nav');
  await page.type('.deployment-receipt-search input', launch.request_id);
  await button('查询回执', '.deployment-receipt-search');
  await text('.deployment-receipt-search .deployment-control-receipt', 'cloud_worker');

  stage = 'exact admin stop and replacement fence';
  await button('停止此实例');
  await text('dialog[open] .dialog-heading', current.incarnation);
  await button('确认停止 Agent', 'dialog[open]');
  await text('dialog[open] .deployment-control-receipt', '等待主机接收');
  assert.deepEqual(writes.at(-1).command, { kind: 'stop', name: current.name, incarnation: current.incarnation });
  await button('关闭', 'dialog[open]');
  await exactLink(old, { access: 'admin' });
  await connect(admin);
  await text('.cloud-reader', '此实例不在当前清单中');
  assert.equal(await page.$('.cloud-reader .cloud-danger'), null);
  assert(!calls.some(call => call.audience === 'admin' && call.path.endsWith('/messages')));

  stage = 'scope exact retired messages on narrow screen';
  await page.setViewport({ width: 320, height: 720, isMobile: true, hasTouch: true });
  await exactLink(old);
  await connect(member);
  await text('.cloud-reader', 'Retired instance only');
  await screenshot('cloud-space-narrow');
  assert.equal(await page.$eval('.cloud-grid', node => node.scrollWidth), 320);
  assert.equal(await page.$eval('.cloud-reader', node => node.scrollWidth), 320);
  assert.equal(await page.$('.cloud-control'), null);
  await button('加载较早消息');
  await text('.cloud-reader', 'Retired instance earlier message');
  assert(calls.some(call => call.path === `${scopeApi}/messages` && call.query.before === '9223372036854775807' && call.query.incarnation === old.incarnation));
  await exactLink(current, { generation: 'other-generation' });
  await connect(member);
  await text('.cloud-reader [role=alert]', '代次已变化');
  assert(!calls.some(call => call.path === `${scopeApi}/messages` && call.query.incarnation === current.incarnation));
  assert.equal(await page.evaluate(([admin, member]) => {
    const values = [location.href, document.cookie, ...Object.values(localStorage), ...Object.values(sessionStorage)];
    return values.some(value => value.includes(admin) || value.includes(member));
  }, [admin, member]), false);
  assert(calls.filter(call => call.audience === 'scope').every(call => call.method === 'GET'));
  assert(calls.every(call => call.path === adminApi || call.path.startsWith(adminApi + '/') || call.path.startsWith(scopeApi + '/')));
  stage = 'native Space demo regression';
  const native = await browser.newPage();
  const nativeCalls = [];
  native.on('request', request => nativeCalls.push(new URL(request.url()).pathname));
  await native.goto(`${origin}/space/?supply=demo`);
  await native.waitForFunction(() => document.querySelector('.office-brand')?.textContent.includes('Agent 空间'));
  await native.waitForSelector('.space-scene-stage canvas');
  assert.equal(await native.$('.cloud-workbench'), null);
  assert(!nativeCalls.some(path => path.startsWith('/api/v1/admin/') || path.startsWith('/scopes/')));
  await native.close();
  assert.deepEqual(errors, []);
  console.log('Cloud Space browser: explicit audiences, actual scene and paged roster, host catalogue, launch/receipt recovery, exact stop, retired messages, 320px layout');
} catch (error) {
  if (evidence) await page.screenshot({ path: join(evidence, 'cloud-space-failure.png') }).catch(() => {});
  throw new Error(`Cloud Space browser failed at ${stage} (${writes.length} control writes)`, { cause: error });
} finally {
  await browser.close();
  await new Promise(resolve => server.close(resolve));
}
