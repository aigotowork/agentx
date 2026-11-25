/** Browser consumer of the two-host isolated Namespace fixture in shared_workbench.py. */
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { writeFile } from 'node:fs/promises';
import puppeteer from 'puppeteer-core';

const env = process.env;
const scenario = JSON.parse(env.SPACE_INPUTS);
const phase = env.SPACE_PHASE;
const evidence = name => join(env.SPACE_EVIDENCE, `${phase}-${name}`);
const browser = await puppeteer.launch({ executablePath: env.SPACE_BROWSER, headless: true,
  userDataDir: env.SPACE_CHROME_PROFILE, args: ['--no-sandbox', '--no-proxy-server', '--disable-dev-shm-usage',
    '--disable-crash-reporter', '--ignore-certificate-errors-spki-list=' + env.SPACE_PIN] });
const page = await browser.newPage();
const pageErrors = [];
const writes = [];
const apiRequests = [];
const viewport = { width: 1400, height: 850 };
page.on('pageerror', error => pageErrors.push(error.message));
page.on('request', request => {
  const { pathname, search } = new URL(request.url());
  if (pathname.includes('/api/v1/')) {
    apiRequests.push(pathname + search);
    if (request.method() !== 'GET') writes.push([request.method(), pathname]);
  }
});
await page.setViewport(viewport);
page.setDefaultTimeout(30_000);

const projectRoute = (host, id) => ({ host, workspace_id: id });
const projectControl = '[aria-label="新 Agent 工作区"]';
const hostControl = '[aria-label="新 Agent 主机"]';
const roleControl = '[aria-label="新 Agent 岗位"]';
const roles = () => page.$eval(roleControl, node => [...node.options].slice(1).map(option => ({
  value: option.value, disabled: option.disabled,
})));

async function until(predicate, ...args) {
  const observed = await page.waitForFunction(predicate, {}, ...args);
  await observed.dispose();
}

async function click(predicate, ...args) {
  const observed = await page.waitForFunction(predicate, {}, ...args);
  try { await observed.asElement().click(); } finally { await observed.dispose(); }
}

async function button(label) {
  await click(value => [...document.querySelectorAll('button')].find(node => !node.disabled
    && node.getClientRects().length && (node.getAttribute('aria-label') ?? node.textContent.trim()) === value), label);
}

async function text(selector, content) {
  await until((selector, content) => [...document.querySelectorAll(selector)].some(node =>
    node.getClientRects().length && node.textContent.includes(content)), selector, content);
}

async function screenshot(name) {
  await page.screenshot({ path: evidence(name + '.png') });
}

function exactRoute(url = page.url()) {
  const params = new URL(url).searchParams;
  return Object.fromEntries(['host', 'name', 'incarnation'].map(key => [key, params.get(key)]));
}

async function observeProjectDirectory(projects) {
  await until(projects => projects.every(([host, id]) => [...document.querySelectorAll('a[href]')].some(link => {
    const p = new URL(link.href).searchParams;
    return link.getClientRects().length && p.get('host') === host && p.get('workspace_id') === id;
  })), projects);
  return page.evaluate(projects => projects.map(([host, id]) => {
    const link = [...document.querySelectorAll('a[href]')].find(link => {
      const p = new URL(link.href).searchParams;
      return link.getClientRects().length && p.get('host') === host && p.get('workspace_id') === id;
    });
    return { host, workspace_id: id, path: new URL(link.href).pathname, name: link.textContent.trim(),
      group: link.closest('section[aria-label]')?.getAttribute('aria-label'),
      hostStatus: link.closest('section[aria-label]')?.querySelector('.cloud-host-link')?.textContent.trim() };
  }), projects);
}

async function openProject(host, id) {
  const matchingRoster = page.waitForResponse(response => {
    const url = new URL(response.url());
    return response.ok() && url.pathname.endsWith('/agents') && url.searchParams.get('host') === host
      && url.searchParams.get('workspace_id') === id;
  });
  await click((host, id) => [...document.querySelectorAll('a[href]')].find(link => {
    const p = new URL(link.href).searchParams;
    return link.getClientRects().length && p.get('host') === host && p.get('workspace_id') === id;
  }), host, id);
  await until(({ host, workspace_id }) => {
    const query = new URL(location.href).searchParams;
    return query.get('host') === host && query.get('workspace_id') === workspace_id && !query.has('name');
  }, projectRoute(host, id));
  return (await matchingRoster).json();
}

async function openLauncher(host, id, role) {
  await button('新建 Agent');
  await page.waitForSelector(`dialog[open] option[value="${role}"]`);
  assert.equal(await page.$eval(hostControl, element => element.value), host);
  assert.equal(await page.$eval(projectControl, element => element.value), id);
  const available = (await roles()).filter(option => !option.disabled).map(option => option.value);
  assert.deepEqual(available, [role]);
  return available;
}

async function observeOfficeRoles(expected) {
  const tab = prefix => click(prefix => [...document.querySelectorAll('.local-views button')].find(button =>
    button.getClientRects().length && button.textContent.trim().startsWith(prefix)), prefix);
  const region = '[aria-label="项目岗位"]';
  await tab('岗位');
  await text(region, expected);
  const listed = await page.$$eval(`${region} li span`, roles => roles.map(role => role.textContent).sort());
  assert.deepEqual(listed, [expected, 'unavailable'].sort());
  await screenshot(expected + '-office-roles');
  await button('关闭办公室资源');
  await page.waitForSelector(region, { hidden: true });
  return listed;
}

async function submit(label, command) {
  const [response] = await Promise.all([
    page.waitForResponse(response => {
      if (response.request().method() !== 'POST' || !new URL(response.url()).pathname.endsWith('/operations')) return false;
      return JSON.parse(response.request().postData() ?? '{}').command?.kind === command;
    }),
    button(label),
  ]);
  assert.equal(response.status(), 200, `Server did not admit ${command}`);
  return response.json();
}

async function addProject(name, directory, create) {
  await button('添加工作区');
  await page.waitForSelector('[aria-label="项目目录"]', { visible: true });
  const parent = await page.$eval('dialog[open]', dialog => [...dialog.querySelectorAll('select')].find(select =>
    [...select.options].some(option => option.value === 'work'))?.value);
  assert.equal(parent, 'work', 'Select the parent root alias independently of the Workspace ID');
  await page.type('[aria-label="项目目录"]', directory);
  await page.type('[aria-label="工作区名称"]', name);
  if (create) await page.click('dialog[open] input[type=checkbox]');
  const receipt = await submit('保存工作区', 'add_workspace');
  assert.equal(receipt.host, 'east');
  assert.deepEqual(receipt.command, { kind: 'add_workspace', parent, name, directory, create_directory: create });
  await page.waitForSelector('[aria-label="项目目录"]', { hidden: true });
  return receipt.request_id;
}

async function showMessageReply() {
  await until(({ message, reply }) => {
    const body = document.querySelector('.workspace-reader')?.innerText ?? '';
    return body.includes(message) && body.includes(reply);
  }, scenario);
}

async function adminPath() {
  const links = await observeProjectDirectory([['east', scenario.east], ['west', scenario.west]]);
  assert(links.every(link => link.name.includes('shared') && link.group === link.host),
    'Both installations must group the same project name under their own host');
  await text('body', 'globalmember');
  await text('body', 'hiddenmember');
  await screenshot('overview');
  const empty = await openProject('west', scenario.west);
  assert.deepEqual(empty, { generation: scenario.generation, items: [], next: null });
  assert.ok(!(await page.$eval('body', body => body.innerText)).includes('localmember'));
  const westOfficeRoles = await observeOfficeRoles('west-special');
  const westRoles = await openLauncher('west', scenario.west, 'west-special');
  await screenshot('empty-west-role');
  await page.keyboard.press('Escape');
  await page.waitForSelector('dialog[open]', { hidden: true });

  const nativeMembers = await openProject('east', scenario.east);
  assert.deepEqual(nativeMembers.items.map(({ host, name, incarnation }) => ({ host, name, incarnation })), [scenario.local]);
  const eastOfficeRoles = await observeOfficeRoles('east-special');
  const eastRoles = await openLauncher('east', scenario.east, 'east-special');
  const addRequests = {
    'Deep project': await addProject('Deep project', 'staging/deep-project', false),
    'New project': await addProject('New project', 'new-project', true),
  };
  await page.keyboard.press('Escape');
  await page.waitForSelector('dialog[open]', { hidden: true });
  await openLauncher('east', scenario.east, 'east-special');
  await page.select(roleControl, 'east-special');
  await page.type('[aria-label="新 Agent 名称"]', 'receiver');
  await page.click('dialog[open] .launch-directory summary');
  const path = 'dialog[open] [aria-label="新 Agent 工作目录"]';
  await page.type(path, '../home');
  assert.equal(await page.$eval(path, node => node.getAttribute('aria-invalid')), 'true');
  assert.equal(await page.$eval('dialog[open] button[type="submit"]', node => node.disabled), true);
  await page.$eval(path, node => node.select());
  await page.keyboard.type('nested/web');
  await text('dialog[open] .launch-directory summary code', 'shared/nested/web');
  const missing = await submit('启动', 'launch');
  assert.equal(missing.command.working_directory, 'nested/web');
  await text('dialog[open] [role="alert"]', '工作目录不存在');
  await page.click('dialog[open] .launch-directory-create input');
  await screenshot('nested-launch-desktop');
  await page.setViewport({ width: 320, height: 620 });
  assert.equal(await page.$eval('dialog[open]', node => node.scrollWidth <= node.clientWidth), true);
  await screenshot('nested-launch-narrow');
  const launch = await submit('启动', 'launch');
  assert.equal(launch.command.workspace_id, scenario.east);
  assert.equal(launch.command.create_directory, true);
  assert.equal(launch.command.role, 'east-special');

  await page.waitForSelector('dialog[open]', { hidden: true });
  await page.setViewport(viewport);
  await page.waitForSelector('.workspace-reader [aria-label="会话详情"]', { visible: true });
  const launched = new URL(page.url());
  const exact = exactRoute(launched);
  assert.equal(exact.host, 'east');
  assert.equal(exact.name, 'receiver');
  assert.match(exact.incarnation, /^[A-Za-z0-9]{12}$/);
  assert.equal(launched.searchParams.get('generation'), scenario.generation);
  assert.equal(launched.searchParams.get('workspace_id'), scenario.east);
  assert.equal(launched.searchParams.get('channel'), 'model', 'Launch must open Transcript automatically');
  await page.click('.workspace-reader [aria-label="会话详情"]');
  await until(cwd => {
    const facts = [...document.querySelectorAll('.workspace-reader .reader-facts dt')];
    return facts.some(node => node.textContent === '独立工作目录' && node.nextElementSibling?.textContent === cwd)
      && facts.some(node => node.textContent === '采用岗位' && node.nextElementSibling?.textContent.includes('east-special'));
  }, scenario.cwd);
  await screenshot('exact-transcript-facts');
  await page.click('.workspace-reader [aria-label="会话详情"]');
  await button('消息');
  const editor = '.workspace-reader [aria-label="Agent 消息草稿"]';
  await page.waitForSelector(editor, { visible: true });
  await page.type(editor, scenario.message);
  await button('发送 AgentX 消息');
  await button('Transcript');
  await until(progress => document.querySelector('.workspace-reader')?.innerText.includes(progress), scenario.progress);
  await writeFile(scenario.progressSeen, 'visible');
  await button('消息');
  await showMessageReply();
  const selected = page.url();
  await page.reload();
  await showMessageReply();
  assert.equal(page.url(), selected);
  return { links, emptyWest: empty, eastRoles, westRoles, eastOfficeRoles, westOfficeRoles, exact, launched: launched.href,
    launchRequest: launch.request_id, missingDirectoryRequest: missing.request_id, addRequests, restoredOnReload: selected };
}

async function readonlyPath() {
  const links = await observeProjectDirectory([['east', scenario.east], ['west', scenario.west],
    ...Object.values(scenario.added).map(id => ['east', id])]);
  const offline = link => /离线/.test(`${link.name} ${link.hostStatus}`);
  assert.ok(links.filter(link => link.host === 'east').every(offline),
    `Offline state is not visible on the east project links: ${JSON.stringify(links)}`);
  assert.ok(links.filter(link => link.host === 'west').every(link => !offline(link)));
  await screenshot('offline-catalogue');
  const projectPage = await openProject('east', scenario.east);
  assert.equal(projectPage.next, null);
  assert.ok(projectPage.items.every(row => row.host === 'east' && row.name === scenario.local.name &&
    row.incarnation === scenario.local.incarnation && row.workspace.workspace_id === scenario.east));
  assert.deepEqual(await openProject('west', scenario.west), { generation: scenario.generation, items: [], next: null });
  assert.equal(await page.$eval('.space-new-agent', button => button.disabled), true);
  const exact = new URL(scenario.readUrl);
  exact.searchParams.set('access', 'namespace');
  await page.goto(exact.href);
  await showMessageReply();
  assert.equal(await page.$eval('.workspace-reader [aria-label="Agent 消息草稿"]', node => node.readOnly), true);
  assert.equal(await page.$('.workspace-reader [aria-label="发送 AgentX 消息"]'), null);
  assert.equal(await page.$eval('.space-new-agent', button => button.disabled), true);
  await screenshot('read-only-exact-history');
  const selected = exactRoute();
  const oldProject = new URL(env.SPACE_ENTRY);
  oldProject.search = new URLSearchParams({ access: 'namespace', host: 'east', workspace_id: scenario.retiredWorkspace });
  const oldIncarnation = new URL(exact.href);
  oldIncarnation.searchParams.set('incarnation', 'ffffffff-ffff-4fff-8fff-ffffffffffff');
  const retiredLinks = {};
  for (const [kind, url] of [['workspace', oldProject], ['incarnation', oldIncarnation]]) {
    const before = apiRequests.length;
    const [response] = await Promise.all([
      page.waitForResponse(response => response.url() === url.href && response.request().isNavigationRequest()),
      page.goto(url.href),
    ]);
    assert.equal(response.status(), 400, 'Native Space must refuse a retired link before serving the shell');
    assert.deepEqual(apiRequests.slice(before), [], 'A retired link must not fetch a resource under an old or translated ID');
    retiredLinks[kind] = response.status();
  }
  assert.deepEqual(writes, []);
  return { links, offlineEast: projectPage, exact: selected, writes, retiredLinks };
}

async function verifyDiagnosticVersion() {
  const resource = page.waitForResponse(response => response.ok() && new URL(response.url()).pathname === scenario.apiPath);
  const inventory = Promise.all(['hosts', 'agents', 'workspaces'].map(name => page.waitForResponse(response =>
    response.ok() && new URL(response.url()).pathname === scenario.resourceBase + name)));
  await button('连接');
  const admission = await (await resource).json();
  if (phase === 'diagnostic-namespace') assert.equal(admission.protocol_version, 3);
  assert.equal(admission.generation, scenario.generation);
  for (const response of await inventory) {
    assert.equal(new URL(response.url()).searchParams.get('generation'), scenario.generation);
    assert.equal((await response.json()).generation, scenario.generation);
  }
  assert.deepEqual(apiRequests.filter(path => path === scenario.apiPath || path === scenario.overview || path === scenario.descriptor),
    [scenario.apiPath], 'Read the selected identity once; resource pages must not query it again');
  return { expectedGeneration: scenario.generation, apiRequests };
}

try {
  const entry = new URL(env.SPACE_ENTRY);
  entry.searchParams.set('access', phase.endsWith('admin') ? 'admin' : 'namespace');
  entry.searchParams.set('view', 'list');
  const response = await page.goto(entry.href);
  assert.equal(response.status(), 200);
  assert.equal(await page.evaluate(() => isSecureContext), true);
  await page.waitForSelector('.cloud-connect input');
  await page.type('.cloud-connect input', env.SPACE_TOKEN);
  let details;
  if (phase.startsWith('diagnostic-')) details = await verifyDiagnosticVersion();
  else {
    await button('连接');
    details = phase === 'admin' ? await adminPath() : await readonlyPath();
  }
  assert.deepEqual(pageErrors, []);
  console.log(JSON.stringify({ browser: await browser.version(), viewport, secureContext: true, ...details }));
} catch (error) {
  await screenshot('failure').catch(() => {});
  throw new Error(`${error.message}; URL=${page.url()}; page errors=${JSON.stringify(pageErrors)}`, { cause: error });
} finally {
  await browser.close();
}
