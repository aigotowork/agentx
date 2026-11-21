/** Actual DOM/HTTP extension acceptance; all mutation targets are disposable native actors. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { chromium } from '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright/index.mjs';
const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
assert.equal(new URL(config.url).hostname, '127.0.0.1');
assert.notEqual(new URL(config.url).port, '38023');
await mkdir(config.out, { recursive: true });
const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--no-proxy-server'] });
const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, locale: 'zh-CN' });
const page = await context.newPage(), errors = [], mutations = [], evidence = {};
page.on('pageerror', error => errors.push(String(error)));
page.on('request', request => { if (request.method() !== 'GET') mutations.push({ url: request.url(), body: request.postDataJSON() }); });
let failure;
const human = page.getByRole('button', { name: '@bigboss 人类通信端', exact: true });
const mailbox = page.getByRole('region', { name: '发给 @bigboss', exact: true });
const dialog = page.getByRole('dialog');
try {
  await page.goto(`${config.url}/space/`);
  await human.click();
  await mailbox.locator('article').nth(1).waitFor();
  assert.equal(await mailbox.locator('.human-sender-group').count(), 2);
  assert.equal(await mailbox.getByRole('button', { name: `@${config.actor.agent.name}`, exact: true }).count(), 1);
  assert.equal(await mailbox.locator('.markdown strong').count(), 2);
  assert.equal(await mailbox.locator('script').count(), 0);
  assert.equal(await page.evaluate(() => window.mailboxInjected), undefined);
  assert.equal(mutations.length, 0);
  await mailbox.getByRole('button', { name: `@${config.actor.agent.name}`, exact: true }).click();
  await mailbox.waitFor({ state: 'hidden' });
  const reader = page.getByRole('region', { name: '人物对话', exact: true });
  await reader.waitFor();
  evidence.exactSender = config.actor.agent;
  const editor = reader.locator('textarea').first();
  await editor.fill('retained draft');
  await editor.press('Control+Shift+N'); assert.equal(await dialog.count(), 0);
  await editor.press('Escape'); await reader.waitFor({ state: 'hidden' });
  const actor = page.locator('.office-actor').filter({ hasText: config.actor.agent.name });
  await actor.focus();
  await page.keyboard.press('Control+Shift+N');
  await dialog.waitFor();
  const title = dialog.getByRole('textbox', { name: '新 Agent 职责标题' });
  await title.fill('Disposable keyboard Agent');
  await title.press('Control+Shift+K'); assert.equal(await dialog.count(), 1);
  await title.press('Escape'); await dialog.waitFor({ state: 'hidden' });
  assert.equal(await actor.evaluate(node => node === document.activeElement), true);
  await page.keyboard.press('/'); assert.equal(await page.getByRole('searchbox').evaluate(node => node === document.activeElement), true);
  await actor.click(); await page.keyboard.press('Escape');
  await actor.focus(); await page.keyboard.press('Control+Shift+K');
  await dialog.getByText('终止此精确实例的进程。历史记录会保留。', { exact: true }).waitFor();
  await page.keyboard.press('Escape'); await dialog.waitFor({ state: 'hidden' });
  assert.equal(mutations.length, 0, 'keyboard opening/cancelling performs no native action');
  // Short tap opens ordinary conversation; hold opens inert action menu and consumes click.
  const bounds = await actor.boundingBox(), x = bounds.x + bounds.width / 2, y = bounds.y + bounds.height / 2;
  await page.mouse.move(x, y); await page.mouse.down(); await page.waitForTimeout(550); await page.mouse.up();
  const menu = page.getByRole('menu'); await menu.waitFor();
  assert.equal(await reader.isVisible(), false);
  assert.equal(await menu.getByRole('menuitem', { name: '分叉', exact: true }).isDisabled(), false);
  await page.keyboard.press('Escape'); await menu.waitFor({ state: 'hidden' });
  await page.mouse.move(x, y); await page.mouse.down(); await page.mouse.move(x + 25, y); await page.waitForTimeout(550); await page.mouse.up();
  assert.equal(await menu.count(), 0, 'drift cancels hold');
  // Cancelled browser gesture cannot open after its deadline.
  await actor.dispatchEvent('pointerdown', { pointerId: 90, button: 0, clientX: x, clientY: y, pointerType: 'touch' });
  await actor.dispatchEvent('pointercancel', { pointerId: 90 });
  await page.waitForTimeout(550); assert.equal(await menu.count(), 0);
  await actor.focus(); await page.keyboard.press('Shift+F10'); await menu.waitFor();
  await menu.getByRole('menuitem', { name: '终止进程', exact: true }).click(); await dialog.waitFor();
  await page.keyboard.press('Escape'); await dialog.waitFor({ state: 'hidden' });
  assert.equal(await actor.evaluate(node => node === document.activeElement), true);
  await actor.click({ button: 'right' }); await menu.waitFor(); await page.keyboard.press('Escape');
  await actor.click(); await reader.waitFor(); assert.equal(await editor.inputValue(), 'retained draft');
  await page.keyboard.press('Escape');
  // Native disclosure is foremost; IME Escape preserves it until composition finishes.
  const details = page.locator('.live-receipts'); await details.locator('summary').click();
  await page.evaluate(() => document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, isComposing: true })));
  assert.equal(await details.evaluate(node => node.open), true);
  await page.keyboard.press('Escape'); assert.equal(await details.evaluate(node => node.open), false);
  await human.click(); await mailbox.locator('article').nth(1).waitFor();
  await page.setViewportSize({ width: 390, height: 844 });
  const group = mailbox.getByRole('button', { name: /^处理此组/ }).first(); await group.scrollIntoViewIfNeeded();
  const box = await group.boundingBox(); assert.ok(box.x >= 0 && box.x + box.width <= 391);
  await page.screenshot({ path: `${config.out}/grouped-mailbox-narrow.png` });
  await group.click(); await mailbox.getByText('@bigboss 信箱 · 1 待处理', { exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await page.setViewportSize({ width: 1280, height: 900 });
  // Deliberate new-Agent submit and deliberate kill go through the same actual journal.
  await page.getByRole('button', { name: '＋ Agent', exact: true }).click();
  await dialog.getByRole('textbox', { name: '新 Agent 职责标题' }).fill('Disposable keyboard Agent');
  await page.screenshot({ path: `${config.out}/new-agent-dialog.png` });
  await dialog.getByRole('button', { name: '启动', exact: true }).click();
  await dialog.waitFor({ state: 'hidden' });
  await page.getByRole('button', { name: '刷新真实数据' }).click();
  // The launch journal admission is observed; live roster refresh is separately native-owned.
  // The kill dialog was opened/cancelled above; native participation retirement is exercised by the fixture.
  evidence.mutations = mutations;
  assert.equal(mutations.filter(item => item.url.endsWith('/agents')).length, 1);
  assert.equal(mutations.filter(item => item.url.endsWith('/kill')).length, 0);
  assert.equal(mutations.filter(item => item.url.endsWith('/human-mailbox/handle')).length, 1);
  assert.deepEqual(errors, []);
} catch (error) { failure = error; await page.screenshot({ path: `${config.out}/failure.png` }).catch(() => {}); }
finally {
  await writeFile(`${config.out}/report.json`, JSON.stringify({ pass: !failure, error: failure?.stack, errors, ...evidence, mutations }, null, 2));
  await browser.close();
}
if (failure) throw failure;
