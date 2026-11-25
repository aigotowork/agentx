/** Verify optional local reads and exact send recovery against the built Space. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { serveWorkspace } from './workspace.fixture.mjs';

const [build, output] = process.argv.slice(2);
assert(build && output, 'Usage: node direct-calls.browser.mjs BUILD EVIDENCE');
await mkdir(output, { recursive: true });
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const evidence = { supply: 'isolated synthetic Management over built Space', chrome: browser.version(), cases: [] };

async function withCapabilityMissing(flag, exercise) {
  const fixture = await serveWorkspace(build);
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.setDefaultTimeout(7000);
  try {
    await page.route('**/api/v1/management', async route => {
      const response = await route.fetch();
      const descriptor = await response.json();
      delete descriptor[flag];
      await route.fulfill({ response, json: descriptor });
    });
    await exercise(page, fixture);
    assert.equal(fixture.calls.filter(call => call.path === '/api/v1/management').length, 1);
    assert.deepEqual(errors, []);
    evidence.cases.push({ flag, calls: fixture.calls });
  } catch (error) {
    evidence.failure = { flag, calls: fixture.calls, errors, message: error.message,
      text: await page.locator('body').innerText(), drafts: await page.locator('textarea').evaluateAll(nodes => nodes.map(node => node.value)) };
    throw error;
  } finally {
    await page.close();
    await fixture.close();
  }
}

try {
  await withCapabilityMissing('supports_workspace_members', async (page, fixture) => {
    await page.goto(fixture.base + '?view=chat&office=%2Fprojects%2Fworkspace');
    await page.getByText('此服务不支持成员列表', { exact: true }).waitFor();
    assert.equal(await page.getByRole('button', { name: '重试成员' }).count(), 0);
    await page.getByRole('combobox', { name: '工作区范围' }).selectOption('');
    await page.getByRole('button', { name: /^@luna/ }).first().click();
    await page.getByRole('region', { name: '@luna', exact: true }).getByText(/^\[luna \/ 1\]/).waitFor();
    assert(!fixture.calls.some(call => call.path === '/api/v1/workspaces/members'));
    assert(fixture.calls.some(call => call.path === '/api/v1/agents'));
  });

  await withCapabilityMissing('supports_message_history', async (page, fixture) => {
    await page.goto(fixture.base + '?view=chat&office=%2Fprojects%2Fworkspace');
    await page.getByRole('button', { name: /^@luna/ }).first().click();
    const pane = page.getByRole('region', { name: '@luna', exact: true });
    await pane.getByText('此服务不支持消息历史', { exact: true }).waitFor();
    assert.equal(await pane.getByRole('button', { name: '重试消息' }).count(), 0);
    await page.setViewportSize({ width: 390, height: 675 });
    const draft = pane.getByRole('textbox', { name: 'Agent 消息草稿' });
    const send = pane.getByRole('button', { name: '发送 AgentX 消息' });
    await draft.fill('历史缺失仍可精确发送');
    await page.route('**/api/v1/agents/luna/messages', async route => {
      if (route.request().method() !== 'POST') return route.continue();
      assert.equal((await route.fetch()).status(), 200);
      await route.abort('failed');
    });
    await page.route('**/api/v1/operations/*?*', route => route.abort('failed'));
    await send.click();
    await pane.getByText('发送结果未确定 · 草稿已保留', { exact: true }).waitFor();
    assert.equal(await draft.inputValue(), '历史缺失仍可精确发送');
    await page.unroute('**/api/v1/operations/*?*');
    await pane.getByRole('button', { name: '查询原回执' }).click();
    await page.waitForFunction(() => document.querySelector('[aria-label="Agent 消息草稿"]')?.value === '');
    const sends = fixture.calls.filter(call => call.method === 'POST' && call.path === '/api/v1/agents/luna/messages');
    assert.equal(sends.length, 1);
    assert.equal(sends[0].body.agent.incarnation, '101');
    assert(fixture.calls.some(call => call.method === 'GET' && call.path === '/api/v1/operations/' + sends[0].body.request_id
      && call.query.generation === sends[0].body.generation));
    assert(!fixture.calls.some(call => call.path.endsWith('/message-history')));
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));

    await draft.fill('换代时保留的草稿');
    await page.route('**/api/v1/office-activity?*', route => route.fulfill({ status: 409, json: { error: { code: 'StaleGeneration', message: 'fixture dataset replaced' } } }));
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.getByRole('button', { name: '刷新真实数据', exact: true }).first().click();
    await page.getByRole('button', { name: '重新连接', exact: true }).waitFor();
    await pane.getByText('连接已失效 · 草稿已保留', { exact: true }).waitFor();
    assert.equal(await draft.inputValue(), '换代时保留的草稿');
    assert(await draft.evaluate(node => node.readOnly));
    assert.equal(await send.count(), 0);
  });
} finally {
  await browser.close();
  await writeFile(join(output, 'direct-calls.json'), JSON.stringify(evidence, null, 2) + '\n');
}
