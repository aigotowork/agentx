import assert from 'node:assert/strict';
import puppeteer from 'puppeteer-core';

const env = process.env;
const browser = await puppeteer.launch({ executablePath: env.CLOUD_BROWSER, headless: true, userDataDir: env.CLOUD_CHROME_PROFILE,
  args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-crash-reporter', '--no-proxy-server',
    `--ignore-certificate-errors-spki-list=${env.CLOUD_CERT_PIN}`] });
const page = await browser.newPage();
const errors = [];
page.on('pageerror', error => errors.push(error.message));
await page.setViewport({ width: 1400, height: 850 });
page.setDefaultTimeout(12_000);

async function button(text) {
  const matching = await page.waitForFunction(text => [...document.querySelectorAll('button')].find(button =>
    (button.getAttribute('aria-label') === text || button.textContent.trim() === text)
    && button.getClientRects().length && !button.disabled && !button.closest('dialog:not([open])')), {}, text);
  try { await matching.asElement().click(); } finally { await matching.dispose(); }
}

async function fill(label, value) {
  const matching = await page.waitForFunction(label => [...document.querySelectorAll('label')].find(item =>
    item.textContent.trim() === label && item.getClientRects().length)?.querySelector('input,textarea'), {}, label);
  try {
    const input = matching.asElement();
    await input.click();
    await page.keyboard.down('Control');
    try { await input.press('a'); } finally { await page.keyboard.up('Control'); }
    await input.type(value);
    assert.ok(await input.evaluate((element, intended) => element.value === intended, value), `Could not replace ${label}`);
  } finally { await matching.dispose(); }
}

async function text(value, selector = 'body') {
  const match = await page.waitForFunction((value, selector) => document.querySelector(selector)?.textContent.includes(value), {}, value, selector);
  await match.dispose();
}

async function launchOnHost() {
  const { scope, host, workspace, role, name, title, initial_prompt } = JSON.parse(env.CLOUD_CONTROL_LAUNCH);
  const selected = await page.waitForFunction(scope => [...document.querySelectorAll('.deployment-scope-row')].find(row =>
    row.querySelector('strong')?.textContent === scope && row.getClientRects().length), {}, scope);
  try { await selected.asElement().click(); } finally { await selected.dispose(); }
  await button(`在 ${host} 启动 Agent`);
  await text('管理在线', 'dialog[open] .deployment-control-catalogue');
  assert.deepEqual(await page.select('dialog[open] select', workspace), [workspace]);
  const roleSelector = 'dialog[open] label:has(select) + label select';
  const readyRole = await page.waitForFunction((selector, role) => [...(document.querySelector(selector)?.options ?? [])]
    .some(option => option.value === role && !option.disabled), {}, roleSelector, role);
  await readyRole.dispose();
  assert.deepEqual(await page.select(roleSelector, role), [role]);
  await fill('Agent 短名（可选）', name);
  await fill('标题（可选）', title);
  await fill('初始任务（可选）', initial_prompt);
  const operationPath = `/api/v1/admin/scopes/${scope}/operations`;
  const [response] = await Promise.all([
    page.waitForResponse(result => result.request().method() === 'POST' && new URL(result.url()).pathname === operationPath),
    button('确认启动 Agent'),
  ]);
  assert.equal(response.status(), 200, 'Cloud did not admit the Web operation');
  const receipt = await response.json();
  assert.equal(receipt.host, host);
  assert.deepEqual(receipt.command, { kind: 'launch', workspace, role, name, title, initial_prompt });
  assert.equal(receipt.status, 'queued');
  await text(receipt.request_id, 'dialog[open] .deployment-control-receipt');
  await text('等待主机接收', 'dialog[open] .deployment-control-receipt');
  await page.screenshot({ path: `${env.CLOUD_EVIDENCE}/deployment-launch-receipt.png` });
  assert.deepEqual(errors, []);
  return { browser: await browser.version(), secureContext: true, receipt };
}

async function manageCredentials() {
  await text('暂无 Scope');
  await button('新建 Scope');
  await fill('Scope ID', 'web-managed');
  await fill('通信令牌', env.CLOUD_CREATE_TOKEN);
  await button('创建 Scope');
  await text('已确认 · web-managed · v1', '[data-saved]');
  const initial = await page.evaluate(async token => (await fetch('/scopes/web-managed/api/v1/descriptor',
    { headers: { Authorization: `Bearer ${token}` } })).status, env.CLOUD_CREATE_TOKEN);
  assert.equal(initial, 200, 'The created scope must admit the selected credential');
  await button('完成');
  await text('web-managed', '#deployment-scope-title');
  await text('暂无主机');
  await button('Agents 0');
  await text('暂无 Agent');
  await button('轮换令牌');
  await fill('通信令牌', env.CLOUD_ROTATED_TOKEN);
  await button('确认轮换');
  await text('已确认 · web-managed · v2', '[data-saved]');
  await button('完成');
  await text('v2', '.deployment-credential');
  await page.screenshot({ path: `${env.CLOUD_EVIDENCE}/deployment.png` });
  for (const key of ['CLOUD_ADMIN_TOKEN', 'CLOUD_CREATE_TOKEN', 'CLOUD_ROTATED_TOKEN']) {
    assert.equal((await page.evaluate(() => JSON.stringify({ ...localStorage, ...sessionStorage }))).includes(env[key]), false,
      'Browser credentials must stay out of Web storage');
  }
  await page.reload();
  await text('连接云端部署', '#deployment-title');
  assert.deepEqual(errors, []);
  return { browser: await browser.version(), secureContext: true, credentialVersions: [1, 2], pageErrors: errors };
}

try {
  await page.goto(env.CLOUD_BROWSER_URL);
  assert.equal(new URL(page.url()).pathname, '/deployment', 'Actual deployment root must redirect to its embedded console');
  assert.equal(await page.evaluate(() => isSecureContext), true);
  await fill('管理员令牌', env.CLOUD_ADMIN_TOKEN);
  await button('连接');
  const outcome = env.CLOUD_CONTROL_LAUNCH ? await launchOnHost() : await manageCredentials();
  console.log(JSON.stringify(outcome));
} finally {
  await browser.close();
}
