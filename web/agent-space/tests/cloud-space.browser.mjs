/** Exercise the built Namespace entry and shared UI with only disposable cloud HTTP fixtures. */
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import puppeteer from 'puppeteer-core';

const [output, evidence] = process.argv.slice(2);
assert(output, 'Pass a built Agent Space directory');
const namespace = 'atlas';
const admin = 'AdminTest123';
const member = 'MemberTest12';
const namespacePath = `/namespaces/${namespace}`;
const entry = `${namespacePath}/space/`;
const adminApi = `/api/v1/admin/namespaces/${namespace}`;
const overviewPath = '/api/v1/admin/overview';
const namespaceApi = `${namespacePath}/api/v1`;
const generation = 'c2b45d62-3f39-48d1-9ba2-63a83792c6e4';
const projectId = 'WS0000000001';
const emptyId = 'WS0000000002';
const peerId = 'WS0000000003';
const addedId = 'WS0000000004';
const old = { host: 'runner-01', name: 'worker-01', incarnation: 'AgentRun0015' };
const current = { ...old, incarnation: 'AgentRun0014' };
const launched = { host: current.host, name: 'cloud_worker', incarnation: 'AgentRun0013' };
const unattended = { ...launched, name: 'unattended', incarnation: 'AgentRun0011' };
const replacement = { ...launched, incarnation: 'AgentRun0012' };
const launchFacts = { role: 'reviewer', title: 'Cloud review', cwd: '/srv/work/studio/src/review', status: 'active' };
const facts = new Map([[launched.incarnation, launchFacts], [old.incarnation, { ...launchFacts, title: null, status: 'stopped' }]]);
const transcripts = new Map([
  [launched.incarnation, [{ id: '1', role: 'user', text: 'Read the scoped notes.' }]],
  [old.incarnation, [{ id: '4', role: 'user', text: 'Earlier model input' }, { id: '5', role: 'assistant', text: 'Retired model result' }]],
  [replacement.incarnation, [{ id: '6', role: 'assistant', text: 'Replacement model result' }]],
]);
const peer = { host: 'runner-02', name: 'peer', incarnation: 'PeerAgent001' };
const human = { host: '~space', name: 'operator', incarnation: '~human' };
const globalAgent = { ...current, name: 'global-member', incarnation: 'GlobalRun001' };
const unexposedAgent = { ...current, name: 'private-member', incarnation: 'PrivateRun01' };
const agents = [globalAgent, unexposedAgent, current, ...Array.from({ length: 20 }, (_, i) => ({ ...peer, name: `worker-${String(i + 2).padStart(2, '0')}` }))];
function agentView(agent) {
  const workspace = agent.incarnation === globalAgent.incarnation ? { kind: 'global' }
    : [human.incarnation, unexposedAgent.incarnation].includes(agent.incarnation) ? { kind: 'unexposed' }
      : { kind: 'project', workspace_id: agent.host === current.host ? projectId : peerId };
  return { ...agent, workspace };
}
const now = 1795334400;
const hosts = [{ name: current.host, online: true, last_seen: now }, { name: peer.host, online: true, last_seen: now }];
const reviewer = { name: 'reviewer', available: true, reason: null };
const projects = [
  { host: current.host, workspace_id: projectId, name: 'Studio', online: true, default_available: false, default_reason: 'default_unavailable',
    roles: [reviewer, { name: 'unready', available: false, reason: 'tool_unavailable' }] },
  { host: current.host, workspace_id: emptyId, name: 'Empty workshop', online: true, default_available: true, default_reason: null, roles: [] },
  { host: peer.host, workspace_id: peerId, name: 'Studio', online: true, default_available: true, default_reason: null, roles: [{ ...reviewer, name: 'builder' }] },
];
const calls = [], controls = [], sends = [], operations = new Map(), accepted = new Map();
let loseControl = false, loseSend = true, installation = 'fixture-installation', namespaceGeneration = generation, protocol = 2;
let changesFault, failTranscript = false, missingMessages = false, revokeMember = false;
const match = (a, b) => ['host', 'name', 'incarnation'].every(key => a[key] === b[key]);
const message = (id, from, to, body, reply_to = null) => ({ id, from: agentView(from), to: agentView(to), body, created_at: now, intent: reply_to ? 'inform' : 'request', thread: null, reply_to });
const messages = [
  message('9223372036854775803', old, peer, 'Retired instance earlier message'),
  message('9223372036854775804', peer, old, '**Retired instance only**'),
];
const observers = new EventTarget();
const revisions = { epoch: 'fixture', roster: 0, messages: 0, transcript: 0 };
function signalMessage() { revisions.messages++; observers.dispatchEvent(new Event('change')); }
function signalTranscript() { revisions.transcript++; observers.dispatchEvent(new Event('change')); }
function signalRoster() { revisions.roster++; observers.dispatchEvent(new Event('change')); }
const assetTypes = { '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };
const csp = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'";
const complete = (input, agent) => operations.set(input.request_id, { ...input, status: 'succeeded', claim_id: 'fixture-claim', outcome: { status: 'succeeded', agent: agentView(agent) } });

const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://loopback');
  const path = url.pathname;
  const send = (obj, status = 200) => response.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' }).end(JSON.stringify(obj));
  const fail = (status, code) => send({ code, message: code }, status);
  function page(items, key = item => item.name, max = 100) {
    if (url.searchParams.get('generation') !== namespaceGeneration) return fail(409, 'stale_generation');
    const after = url.searchParams.get('after');
    const remaining = items.filter(item => !after || key(item) > after).sort((a, b) => key(a).localeCompare(key(b), 'en'));
    const rows = remaining.slice(0, Math.min(max, Number(url.searchParams.get('limit') ?? '20')));
    return send({ generation: namespaceGeneration, items: rows, next: rows.length < remaining.length ? key(rows.at(-1)) : null });
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
    if (path.startsWith('/scopes/') || path.startsWith('/api/v1/admin/scopes/')) return fail(404, 'retired_path');
    const isAdmin = path === overviewPath || path === adminApi || path.startsWith(adminApi + '/');
    const isNamespace = path.startsWith(namespaceApi + '/');
    const audience = request.headers.authorization === `Bearer ${admin}` ? 'admin' : request.headers.authorization === `Bearer ${member}` ? 'namespace' : 'none';
    calls.push({ path, method: request.method, query: Object.fromEntries(url.searchParams), audience });
    if (!isAdmin && !isNamespace) return fail(404, 'unsupported_path');
    if (isAdmin ? audience !== 'admin' : audience !== 'namespace' || revokeMember) return fail(401, 'unauthorized');
    if (path.endsWith('/changes')) {
      if (url.searchParams.get('generation') !== namespaceGeneration) return fail(409, 'stale_generation');
      if (changesFault === 'missing') { changesFault = 'recovered'; return fail(404, 'not_found'); }
      if (changesFault === 'recovered') { changesFault = undefined; return send(revisions); }
      if (url.searchParams.get('cursor') === Object.values(revisions).join('.')) {
        await new Promise(resolve => {
          const finish = () => { clearTimeout(timer); observers.removeEventListener('change', finish); resolve(); };
          const timer = setTimeout(finish, 25_000);
          response.once('close', finish); observers.addEventListener('change', finish, { once: true });
        });
      }
      if (!response.destroyed) send(revisions);
      return;
    }
    if (path === overviewPath) return send({ deployment_id: 'fixture-deployment', protocol_version: protocol, observed_at: now,
      namespaces: 1, hosts: hosts.length, online_hosts: 2, agents: agents.length, unavailable: 0 });
    if (path === `${namespaceApi}/descriptor`) return send({ namespace_id: namespace, generation: namespaceGeneration, protocol_version: protocol });
    if (path === adminApi) return send({ namespace_id: namespace, namespace_url: `http://127.0.0.1:${server.address().port}${namespacePath}`, generation: namespaceGeneration,
      available: true, credential_version: 1, created_at: now, agents: agents.length, hosts: hosts.length, online_hosts: 1 });
    const host = url.searchParams.get('host');
    const workspaceId = url.searchParams.get('workspace_id');
    if (path.endsWith('/hosts')) return page(hosts);
    if (path.endsWith('/workspaces')) return page(projects.filter(project => !host || project.host === host), item => `${item.host}/${item.workspace_id}`, 1);
    if (path.endsWith('/agents')) {
      const rows = agents.map(agentView).filter(agent => (!host || agent.host === host) && (!workspaceId || agent.workspace.workspace_id === workspaceId));
      return page(rows.map(agent => url.searchParams.get('facts') === 'true' && facts.has(agent.incarnation) ? { ...agent, facts: facts.get(agent.incarnation) } : agent));
    }
    if (path.endsWith('/capabilities') && isAdmin) {
      const selected = hosts.find(item => item.name === path.split('/').at(-2));
      if (!selected) return fail(404, 'not_found');
      const workspaces = projects.filter(project => project.host === selected.name).map(({ host, online, ...project }) => project);
      return send({ host: selected.name, generation: namespaceGeneration, installation_id: installation, online: selected.online, observed_at: now,
        workspaces, project_roots: ['default'], workspace_management: true, directory_launch: true });
    }
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
    if (path.endsWith('/transcript') && request.method === 'GET') {
      if (url.searchParams.get('generation') !== namespaceGeneration) return fail(409, 'stale_generation');
      if (failTranscript) return fail(404, 'not_found');
      const exact = { host: url.searchParams.get('host'), name: url.searchParams.get('name'), incarnation: url.searchParams.get('incarnation') };
      if (![old, launched, ...agents].some(item => match(exact, item))) return fail(404, 'not_found');
      const before = url.searchParams.get('before'), after = url.searchParams.get('after');
      const all = transcripts.get(exact.incarnation) ?? [];
      const sorted = all.filter(item => (!before || BigInt(item.id) < BigInt(before)) && (!after || BigInt(item.id) > BigInt(after)));
      if (!after) sorted.reverse();
      const limit = match(exact, old) ? 1 : Number(url.searchParams.get('limit'));
      const items = sorted.slice(0, limit);
      return send({ agent: agentView(exact), available: facts.has(exact.incarnation), items,
        next: items.length < sorted.length ? items.at(-1).id : null,
        latest: all.at(-1)?.id ?? null, facts: facts.get(exact.incarnation) ?? null });
    }
    if (path.endsWith('/messages') && request.method === 'GET') {
      if (missingMessages) return fail(404, 'unsupported_operation');
      if (url.searchParams.get('generation') !== namespaceGeneration) return fail(409, 'stale_generation');
      const exact = { host: url.searchParams.get('host'), name: url.searchParams.get('name'), incarnation: url.searchParams.get('incarnation') };
      if (![old, launched, ...agents].some(item => match(exact, item))) return fail(404, 'not_found');
      const before = url.searchParams.get('before');
      const after = url.searchParams.get('after');
      const sorted = messages.filter(item => (!before || BigInt(item.id) < BigInt(before)) && (!after || BigInt(item.id) > BigInt(after)) && (match(exact, item.from) || match(exact, item.to)))
        .sort((a, b) => (BigInt(a.id) < BigInt(b.id) ? -1 : 1) * (after ? 1 : -1));
      const limit = Number(url.searchParams.get('limit') ?? '20');
      const rows = sorted.slice(0, match(old, exact) ? 1 : limit);
      return send({ generation: namespaceGeneration, items: rows, next: rows.length < sorted.length ? rows.at(-1).id : null });
    }
    if (path === `${adminApi}/messages` && request.method === 'POST') {
      const input = await body();
      sends.push(input);
      if (input.generation !== namespaceGeneration) return fail(409, 'stale_generation');
      if (accepted.has(input.request_id)) {
        const original = accepted.get(input.request_id);
        assert.deepEqual(original.input, input);
        return send(original.row);
      }
      if (!agents.some(agent => match(agent, input.to))) return fail(404, 'not_found');
      const row = message('9223372036854775806', human, input.to, input.body);
      accepted.set(input.request_id, { input, row });
      messages.push(row); signalMessage();
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
  const assertRoute = (agent, workspaceId = projectId) => {
    const url = new URL(page.url());
    assert.equal(url.pathname, entry);
    for (const [key, value] of Object.entries({ ...agent, generation })) assert.equal(url.searchParams.get(key), value);
    assert.equal(url.searchParams.get('workspace_id'), workspaceId);
  };
  const chooser = '[aria-label="工作区范围"]';
  const projectKey = (host, workspaceId) => `project:${host}:${workspaceId}`;
  const projectLink = id => `.cloud-project-link[title="${id}"]`;
  async function selectProject(host, id) {
    const key = projectKey(host, id);
    await page.waitForFunction((selector, value) => [...document.querySelector(selector).options].some(option => option.value === value), {}, chooser, key);
    await page.select(chooser, key);
  }

  await page.goto(`${origin}${entry}?access=admin&supply=demo`);
  await page.waitForSelector('.cloud-connect input');
  assert.equal(calls.length, 0);
  assert.equal(await page.$eval('.cloud-deployment-link', a => a.getAttribute('href')), '/deployment');
  await connect('retired-token-not-for-production');
  await text('[role=alert]', '请输入 12 位');
  assert.equal(calls.length, 0);
  await connect(member);
  await text('[role=alert]', '令牌无效');
  assert.equal(calls.length, 1);
  await connect(admin);
  stage = 'shared admin scene and roster';
  await page.waitForSelector('.space-scene-stage canvas');
  await text('.office-brand', '云端 Space');
  await page.waitForSelector(projectLink(peerId));
  assert.equal(await page.$('.cloud-workbench'), null);
  assert(calls.some(call => call.path === `${adminApi}/workspaces` && call.query.after === `${current.host}/${projectId}`));
  assert.deepEqual(calls.slice(0, 2).map(call => call.path), [adminApi, adminApi]);
  assert(!calls.some(call => call.path === overviewPath));
  assert(calls.filter(call => /\/(hosts|agents|workspaces)$/.test(call.path)).every(call => call.query.generation === generation));
  await screenshot('cloud-space-desktop');
  await button('聊天', '.presentation-switch');
  await button('加载更多成员');
  await text('.hall-directory-items', '@worker-21');
  await text('.hall-directory-items', '全局成员');
  await text('.hall-directory-items', '项目未开放');
  const idle = calls.length;
  await new Promise(resolve => setTimeout(resolve, 6200));
  assert.deepEqual(calls.slice(idle).filter(call => !call.path.endsWith('/changes')), [], 'An idle roster does not refetch loaded members, hosts or their Namespace metadata');
  changesFault = 'missing';
  observers.dispatchEvent(new Event('change'));
  await text('.live-status [role=status]', '无法监视云端更新');
  await page.waitForSelector('.live-status [role=status]', { hidden: true });
  assert.deepEqual(calls.slice(idle).filter(call => !call.path.endsWith('/changes')), [], 'A 404 retries the same watcher without entering a fallback read loop');
  stage = 'copyable project offices and isolated roles';
  const emptyLink = await page.$eval(projectLink(emptyId), node => node.href);
  assert.equal(new URL(emptyLink).searchParams.get('host'), current.host);
  assert.equal(new URL(emptyLink).searchParams.get('workspace_id'), emptyId);
  await page.goto(emptyLink);
  await text('.space-read-status[aria-label="成员覆盖"]', '没有成员记录');
  assert.equal(await page.$eval(chooser, node => node.value), projectKey(current.host, emptyId));
  assert.equal(await page.$('.hall-directory-person'), null);
  await page.click(projectLink(peerId));
  await text('.hall-directory-items', '@worker-02');
  assert(!await page.$eval('.hall-directory-items', row => row.textContent.includes('@worker-01')));
  await button('岗位 1');
  await text('.cloud-project-roles', 'builder');
  assert(!await page.$eval('.cloud-project-roles', row => row.textContent.includes('reviewer')));
  assert(await page.$eval('dialog.space-resource-window', node => node.getBoundingClientRect().height < innerHeight / 2));
  await screenshot('cloud-space-roles');
  await button('关闭办公室资源');
  await page.waitForSelector('dialog.space-resource-window', { hidden: true });
  await page.click(projectLink(projectId));
  await text('.hall-directory-items', '@worker-01');
  assert(!await page.$eval('.hall-directory-items', row => row.textContent.includes('@worker-21')));
  assert(calls.some(call => call.path === `${adminApi}/agents` && call.query.host === current.host && call.query.workspace_id === projectId));
  await button('岗位 2');
  await text('.cloud-project-roles', 'reviewer');
  await button('关闭办公室资源');
  await page.waitForSelector('dialog.space-resource-window', { hidden: true });
  await button(`@${current.name} · ${current.host} · ${current.incarnation}`, '.hall-directory');
  await text(reader, '@worker-01');
  assertRoute(current);

  stage = 'shared role picker and original cloud launch receipt';
  await button('新建 Agent');
  await page.waitForSelector(`${dialog} option[value="reviewer"]`);
  assert.equal(await page.$eval(`${dialog} [aria-label="新 Agent 主机"]`, node => node.value), current.host);
  assert.equal(await page.$eval(`${dialog} [aria-label="新 Agent 工作区"]`, node => node.value), projectId);
  assert.equal(await page.$(`${dialog} option[value="builder"]`), null);
  assert.equal(await page.$eval(`${dialog} option[value="unready"]`, node => node.disabled), true);
  assert.equal(await page.$eval(`${dialog} button[type=submit]`, node => node.disabled), true, 'Unavailable default role');
  await page.select(`${dialog} [aria-label="新 Agent 岗位"]`, 'reviewer');
  await page.type(`${dialog} [aria-label="新 Agent 名称"]`, launched.name);
  await page.type(`${dialog} [aria-label="新 Agent 职责标题"]`, launchFacts.title);
  await page.type(`${dialog} textarea`, 'Read the scoped notes.');
  await page.click(`${dialog} .launch-directory summary`);
  await page.type(`${dialog} [aria-label="新 Agent 工作目录"]`, 'src/review');
  await screenshot('cloud-space-launch');
  await openCommand('启动');
  await text(dialog, '等待主机接收');
  const [launch] = controls;
  assert.deepEqual(launch, { generation, host: current.host, installation_id: installation,
    request_id: launch.request_id, command: { kind: 'launch', workspace_id: projectId, role: 'reviewer', name: launched.name, title: launchFacts.title, initial_prompt: 'Read the scoped notes.', working_directory: 'src/review' } });
  assert.equal(controls.length, 1);
  complete(launch, launched);
  agents.push(launched);
  await page.waitForSelector(dialog, { hidden: true });
  // The exact successful receipt itself opens the launched reader.
  const pane = `[aria-label="${launchFacts.title}"] ${reader}`;
  await text(`${pane} h1`, launchFacts.title);
  assertRoute(launched);
  assert.equal(new URL(page.url()).searchParams.get('channel'), 'model');
  await text('.hall-directory-items', launchFacts.title);
  await text(pane, 'Read the scoped notes.');
  await text(`${pane} .reader-state`, '工作中');
  await page.click(`${pane} [aria-label="会话详情"]`);
  await text(`${pane} .reader-facts`, launchFacts.cwd);
  await text(`${pane} .reader-facts`, 'Studio');
  await text(`${pane} .reader-facts`, launchFacts.role);
  await page.click(`${pane} [aria-label="会话详情"]`);
  const beforeProgress = calls.length;
  transcripts.get(launched.incarnation).push({ id: '2', role: 'assistant', text: 'Checking the scoped notes.' });
  signalTranscript();
  await text(pane, 'Checking the scoped notes.');
  assert(!await page.$eval(`${pane} .conversation-history`, node => node.textContent.includes('示例')));
  const progressReads = calls.slice(beforeProgress).filter(call => call.path.endsWith('/transcript'));
  assert(progressReads.length && progressReads.every(call => call.query.after === '1' && call.query.incarnation === launched.incarnation));
  await button('消息', pane);

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
  await button(`${launchFacts.title} · ${launched.host} · ${launched.incarnation}`, '.hall-directory');

  stage = 'shared composer, replay identity and agent reply';
  await page.waitForSelector(`${pane} ${draft}`);
  assert.equal(await page.$eval(`${pane} [aria-label="添加附件"]`, node => node.disabled), true);
  const outgoing = 'Please review the scoped change.';
  await page.type(`${pane} ${draft}`, outgoing);
  await button('Transcript', pane);
  await text(pane, 'Checking the scoped notes.');
  await button('消息', pane);
  assert.equal(await page.$eval(`${pane} ${draft}`, node => node.value), outgoing);
  await button('发送 AgentX 消息', pane);
  await text(pane, '发送结果未确定');
  assert.equal(await page.$eval(`${pane} ${draft}`, node => node.value), outgoing);
  await button('确认原发送', pane);
  await text(pane, '云端已接收 · #9223372036854775806');
  assert.equal(sends.length, 2);
  assert.deepEqual(sends[0], sends[1]);
  assert.deepEqual(sends[0], { generation, request_id: sends[0].request_id, to: agentView(launched), body: outgoing });
  assert.equal(accepted.size, 1);
  assert.equal(await page.$eval(`${pane} ${draft}`, node => node.value), '');
  await page.waitForFunction(selector => document.querySelector(selector)?.textContent.includes('Please review the scoped change.'), {}, `${pane} .conversation-history`);
  // Allow the post-send catch-up to finish. The next reply must arrive via the selected reader alone.
  await new Promise(resolve => setTimeout(resolve, 300));
  const beforeReply = calls.length;
  messages.push(message('9223372036854775807', launched, human, 'Agent reply persisted.', '9223372036854775806'));
  signalMessage();
  await page.waitForFunction(selector => document.querySelector(selector)?.textContent.includes('Agent reply persisted.'), { timeout: 20_000 }, pane);
  const replyReads = calls.slice(beforeReply).filter(call => !call.path.endsWith('/changes'));
  assert(replyReads.length > 0 && replyReads.every(call => call.path === `${adminApi}/messages` && call.query.incarnation === launched.incarnation));
  assert.equal(replyReads[0].query.after, '9223372036854775806', 'Waking the selected reader fetches only newer messages');
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
  assert(!calls.some(call => /\/(messages|transcript)$/.test(call.path) && call.query.incarnation === replacement.incarnation));

  stage = 'retained offline catalogue and Admin project creation';
  hosts.find(item => item.name === peer.host).online = false;
  projects.find(item => item.workspace_id === peerId).online = false;
  signalRoster();
  await text(`.cloud-office-group[aria-label="${peer.host}"]`, '离线');
  await selectProject(peer.host, peerId);
  await text('.hall-directory-items', '@worker-02');
  await button('新建 Agent');
  await text(dialog, '主机管理离线');
  assert.equal(await page.$eval(`${dialog} button[type=submit]`, node => node.disabled), true);
  await page.keyboard.press('Escape');
  await page.waitForSelector(dialog, { hidden: true });
  await selectProject(current.host, emptyId);
  await button('新建 Agent');
  await openCommand('添加工作区');
  assert.equal(await page.$eval(`${dialog} [aria-label="项目根目录"]`, node => node.value), 'default');
  await page.type(`${dialog} [aria-label="项目目录"]`, 'projects/second-studio');
  await page.type(`${dialog} [aria-label="工作区名称"]`, 'Studio');
  await page.click(`${dialog} .launch-directory-create input`);
  await openCommand('保存工作区');
  await text(dialog, '等待主机接收');
  const registration = controls.at(-1);
  assert.deepEqual(registration.command, { kind: 'add_workspace', parent: 'default', directory: 'projects/second-studio', name: 'Studio', create_directory: true });
  projects.push({ ...projects[0], workspace_id: addedId, roles: [], default_available: true, default_reason: null });
  operations.set(registration.request_id, { ...registration, status: 'succeeded', claim_id: 'fixture-claim', outcome: { status: 'workspace_added', workspace_id: addedId } });
  signalRoster();
  await page.waitForFunction((selector, id) => document.querySelector(selector)?.value === id, {}, `${dialog} [aria-label="新 Agent 工作区"]`, addedId);
  await page.keyboard.press('Escape');
  await page.waitForSelector(dialog, { hidden: true });
  await page.waitForSelector(projectLink(addedId));
  assert.match(await page.$eval(projectLink(addedId), node => node.textContent), /Studio · WS000000/);

  stage = 'read-only Namespace project link, mobile layout, older records';
  await page.setViewport({ width: 320, height: 720, isMobile: true, hasTouch: true });
  await exactLink(old, { workspace_id: projectId });
  const memberEntry = calls.length;
  protocol = 3;
  failTranscript = true;
  await connect(member, '0');
  await text(reader, 'Retired instance only');
  assertRoute(old);
  assert.equal(await page.$eval('[aria-label="新建 Agent"]', node => node.disabled), true);
  assert.equal(await page.$eval(`${reader} ${draft}`, node => node.readOnly), true);
  assert.equal(await page.$eval(chooser, node => node.value), projectKey(current.host, projectId));
  assert(calls.some(call => call.path === `${namespaceApi}/workspaces` && call.audience === 'namespace'));
  assert.deepEqual(calls.slice(memberEntry).filter(call => call.path.endsWith('/descriptor')).map(call => call.path), [`${namespaceApi}/descriptor`]);
  assert(calls.slice(memberEntry).filter(call => /\/(hosts|agents|workspaces)$/.test(call.path)).every(call => call.query.generation === generation));
  await screenshot('cloud-space-narrow');
  assert.equal(await page.$eval('body', node => node.scrollWidth), 320);
  await button('查看更早消息', reader);
  await text(reader, 'Retired instance earlier message');
  assert(calls.some(call => call.path === `${namespaceApi}/messages` && call.query.before === '9223372036854775804' && call.query.incarnation === old.incarnation));
  await button('Transcript', reader);
  await text(`${reader} .record-availability`, '读取失败');
  failTranscript = false;
  await button('重试记录', reader);
  await text(reader, 'Retired model result');
  await button('查看更早记录', reader);
  await text(reader, 'Earlier model input');
  assert(calls.some(call => call.path === `${namespaceApi}/transcript` && call.query.before === '5' && call.query.incarnation === old.incarnation));
  assert(!await page.$eval(reader, node => node.textContent.includes('Replacement model result')));
  await page.reload();
  await page.waitForSelector('.cloud-connect input');

  stage = 'stale generation link and authorization separation';
  await exactLink(current, { generation: 'other-generation' });
  await connect(member, '0');
  await text('.context-error', '已失效');
  assert(!calls.some(call => call.path === `${namespaceApi}/messages` && call.query.incarnation === current.incarnation));
  assert.equal(await page.evaluate(([admin, member]) => {
    const values = [location.href, document.cookie, ...Object.values(sessionStorage)];
    return values.some(value => value.includes(admin) || value.includes(member));
  }, [admin, member]), false);
  assert(calls.filter(call => call.audience === 'namespace').every(call => call.method === 'GET'));
  assert(calls.every(call => call.path === overviewPath || call.path === adminApi || call.path.startsWith(adminApi + '/') || call.path.startsWith(namespaceApi + '/')));
  assert(calls.some(call => call.path === `${adminApi}/messages` && call.method === 'GET' && call.query.incarnation === launched.incarnation));
  for (const path of [`/scopes/${namespace}/space/`, `/scopes/${namespace}/api/v1/descriptor`, `/api/v1/admin/scopes/${namespace}`]) {
    assert.equal((await fetch(origin + path)).status, 404);
  }

  stage = 'missing message history leaves the exact Transcript and roster available';
  missingMessages = true;
  await exactLink(old, { workspace_id: projectId });
  await connect(member, '0');
  await text(reader, '此服务不支持读取消息历史');
  assertRoute(old);
  await button('Transcript', reader);
  await text(reader, 'Retired model result');
  await text('.hall-directory-items', '@worker-01');
  missingMessages = false;

  stage = 'direct inventory continuation discards stale rows while retaining the original draft';
  await page.setViewport({ width: 1280, height: 800 });
  await page.evaluate(() => localStorage.clear());
  await exactLink(current, { access: 'admin' });
  await connect(admin, '0');
  await text('.hall-directory-items', '@worker-01');
  await page.waitForSelector(projectLink(addedId));
  await page.waitForSelector(`${reader} ${draft}`);
  await page.type(`${reader} ${draft}`, 'Preserve this exact draft');
  const beforeContinuation = calls.length;
  namespaceGeneration = 'replacement-generation';
  await button('加载更多成员');
  await text('.live-status', 'Namespace 代次已变化');
  assert.equal(await page.$('.hall-directory-person'), null);
  const pinned = new URL(page.url()).searchParams;
  for (const [key, value] of Object.entries({ ...current, generation })) assert.equal(pinned.get(key), value);
  assert.equal(await page.$eval(`${reader} ${draft}`, node => node.value), 'Preserve this exact draft');
  const continuation = calls.slice(beforeContinuation);
  assert(!continuation.some(call => call.path === adminApi || call.path.endsWith('/descriptor')));
  assert.deepEqual(continuation.filter(call => call.path.endsWith('/agents')).map(call => [call.path, call.query.generation, Boolean(call.query.after)]),
    [[`${adminApi}/agents`, generation, true]]);

  stage = 'credential revocation stops member reads without readmission';
  await page.goto(`${origin}${entry}?access=namespace&view=list`);
  await connect(member, '0');
  await text('.hall-directory-items', '@worker-01');
  await page.waitForSelector(projectLink(addedId));
  const beforeRevocation = calls.length;
  revokeMember = true;
  await button('加载更多成员');
  await text('.live-status', '令牌无效');
  assert.equal(await page.$('.hall-directory-person'), null);
  assert(!calls.slice(beforeRevocation).some(call => call.path.endsWith('/descriptor')));
  revokeMember = false;

  stage = 'native App demo entry remains separate from cloud credentials';
  const native = await browser.newPage();
  const nativeCalls = [];
  native.on('request', request => nativeCalls.push(new URL(request.url()).pathname));
  await native.goto(`${origin}/space/?supply=demo`);
  await native.waitForFunction(() => document.querySelector('.office-brand')?.textContent.includes('Agent 空间'));
  await native.waitForSelector('.space-scene-stage canvas');
  assert.equal(await native.$('.cloud-connect'), null);
  assert(!nativeCalls.some(path => path.startsWith('/api/v1/admin/') || path.startsWith('/namespaces/')));
  await native.close();
  assert.deepEqual(errors, []);
  console.log('Cloud Space browser: Namespace overview, project paging, empty/same-name/offline offices, focused roles and nested launch, original Admin receipts, exact retired readers and a 320px read-only entry.');
} catch (error) {
  if (evidence) await page.screenshot({ path: join(evidence, 'cloud-space-failure.png') }).catch(() => {});
  console.error('Recent fixture requests:', calls.slice(-10), 'Browser errors:', errors);
  throw new Error(`Cloud Space browser failed at ${stage} (${controls.length} controls, ${sends.length} sends)`, { cause: error });
} finally {
  await browser.close();
  await new Promise(resolve => server.close(resolve));
}
