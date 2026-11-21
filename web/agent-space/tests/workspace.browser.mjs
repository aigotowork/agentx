/** Exercise in-canvas workflows against an actual isolated Space build and authenticated synthetic SDK supply. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { serveWorkspace } from './workspace.fixture.mjs';

const [build, output] = process.argv.slice(2);
assert(build && output, 'Usage: node workspace.browser.mjs BUILD EVIDENCE');
await mkdir(output, { recursive: true });
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const fixture = await serveWorkspace(build);
const data = { supply: 'synthetic SDK over actual build', cases: [], errors: [] };
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: 'no-preference' });
page.setDefaultTimeout(7000);
page.on('pageerror', error => data.errors.push(error.message));
const button = (name, root = page) => root.getByRole('button', { name, exact: typeof name === 'string' });
const pane = name => page.getByRole('region', { name: '@' + name, exact: true });
const draft = root => root.getByRole('textbox', { name: 'Agent 消息草稿' });
const history = root => root.locator('.conversation-history');
const manager = page.getByRole('navigation', { name: '已打开的会话' });
const roster = page.getByRole('region', { name: '场景名册' });
const sceneRoster = page.locator('.office-camera-controls button[aria-expanded]');
async function openFromScene(name) {
  await sceneRoster.click();
  await button(new RegExp('^@' + name), roster).click();
  await pane(name).waitFor();
  await pane(name).getByText(new RegExp('^\\[' + name + ' / 1\\]')).waitFor();
}
async function noOverflow() {
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth && document.documentElement.scrollHeight <= innerHeight));
}
async function insideViewport(locator) {
  const box = await locator.boundingBox();
  const { width, height } = page.viewportSize();
  assert(box && box.width > 0 && box.height > 0 && box.x >= 0 && box.y >= 0 && box.x + box.width <= width + 1 && box.y + box.height <= height + 1, JSON.stringify(box));
  assert(await locator.evaluate(node => {
    const box = node.getBoundingClientRect(), hit = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2);
    return node.contains(hit);
  }), 'control must be reachable, including through ancestor clipping');
}
async function snap(name) { await page.screenshot({ path: join(output, name + '.png') }); }
try {
  await page.goto(fixture.base + '?workspace=classic');
  await sceneRoster.click();
  await button(/^@luna/, roster).click();
  await page.getByText(/^\[luna \/ 1\]/).waitFor();
  data.classic = await history(page).boundingBox();
  await snap('classic-desktop');

  await page.goto(fixture.base);
  await page.locator('canvas').waitFor();
  await page.locator('canvas').evaluate(canvas => { canvas.dataset.workspaceWitness = 'unchanged'; });
  await openFromScene('luna');
  await openFromScene('mora');
  const baselineRelations = fixture.calls.filter(call => call.path.endsWith("launch-relations")).length;
  const a = pane('luna'), b = pane('mora');
  assert((await a.boundingBox()).x + (await a.boundingBox()).width <= (await b.boundingBox()).x + 1, 'two columns');
  assert.equal(await page.locator('canvas').getAttribute('data-workspace-witness'), 'unchanged');
  assert.equal(new URL(page.url()).pathname, '/space/');
  assert.equal(new URL(page.url()).searchParams.get('agent'), 'mora');
  data.twoReaders = [await history(a).boundingBox(), await history(b).boundingBox()];
  await noOverflow();
  await snap('two-readers-desktop');
  await draft(a).fill('luna 的私人草稿');
  await draft(b).fill('mora 的私人草稿');
  await history(a).hover();
  await page.mouse.wheel(0, 10_000);
  await a.getByText(/^\[luna \/ 18\]/).waitFor();
  assert.equal(await button('更多消息', a).count(), 0, 'ordinary continuation should not require a paging button');
  assert.equal(await b.getByText(/^\[mora \/ 18\]/).count(), 0);
  await history(a).evaluate(element => {
    element.scrollTop = element.scrollHeight - element.clientHeight;
    element.dispatchEvent(new Event('scroll', { bubbles: true }));
  });
  const firstReadableMessage = root => history(root).evaluate(element => {
    const top = element.getBoundingClientRect().top;
    return [...element.querySelectorAll('[data-history-record]')].find(message => message.getBoundingClientRect().bottom > top)?.getAttribute('data-history-record');
  });
  const readingAnchor = await firstReadableMessage(a);
  assert(readingAnchor);
  data.cases.push('same Canvas node, two independent exact readers, pages and drafts');

  await button(/^聚焦会话/, b).click();
  await a.waitFor({ state: 'hidden' });
  await snap('focused-desktop');
  await button(/^恢复并排/, b).click();
  await a.waitFor();
  await a.getByText(/^\[luna \/ 18\]/).waitFor();
  assert.equal(await firstReadableMessage(a), readingAnchor, 'focus restore returns to the same visible message');
  assert.equal(await draft(a).inputValue(), 'luna 的私人草稿');
  assert.equal(await draft(b).inputValue(), 'mora 的私人草稿');
  await button(/^最小化会话/, a).click();
  await a.waitFor({ state: 'hidden' });
  const parked = button(/@luna.*已最小化/, manager);
  assert(await parked.evaluate(element => document.activeElement === element));
  await parked.press('Enter');
  await a.waitFor();
  assert.equal(await draft(a).inputValue(), 'luna 的私人草稿');
  await button(/^关闭会话/, b).click();
  await b.waitFor({ state: 'hidden' });
  assert.equal(await button('@mora', manager).count(), 0);
  data.cases.push('focus, tile restore, keyboard shelf restore and close');

  const relationReads = () => fixture.calls.filter(call => call.path.endsWith('launch-relations')).length;
  assert.equal(relationReads(), baselineRelations, 'disclosed relations alone should read');
  await a.getByText('启动关系', { exact: true }).click();
  await a.getByText('我启动的 Agent', { exact: true }).waitFor();
  await snap('related-agents-desktop');
  await button('打开 mora 的精确会话', a).click();
  await b.waitFor();
  assert.equal(await draft(b).inputValue(), 'mora 的私人草稿');
  assert.equal(new URL(page.url()).searchParams.get('incarnation'), '102');
  await page.getByLabel('显示与反馈').click();
  const sound = button('窗口音效');
  assert.equal(await sound.getAttribute('aria-pressed'), 'false');
  await page.keyboard.press('Escape');
  data.cases.push('demand-loaded launch relations open another exact conversation');

  const newAgent = button('新建 Agent');
  const before = page.url();
  await newAgent.click();
  const modal = page.getByRole('dialog', { name: '新建 Agent' });
  await modal.waitFor();
  await modal.getByRole('textbox', { name: '新 Agent 职责标题' }).fill('隔离预览的新 Agent');
  await modal.getByRole('textbox', { name: '新 Agent 任务' }).fill('从画布完成指派');
  assert.equal(page.url(), before);
  await snap('launch-desktop');
  await button('启动', modal).click();
  await modal.waitFor({ state: 'hidden' });
  assert(await newAgent.evaluate(element => document.activeElement === element));
  assert.equal(page.url(), before);
  assert.equal(await page.locator('canvas').getAttribute('data-workspace-witness'), 'unchanged');
  const launch = fixture.calls.filter(call => call.method === 'POST' && call.path === '/api/v1/agents');
  assert.equal(launch.length, 1);
  assert.equal(launch[0].body.launch.prompt, '从画布完成指派');
  assert.equal(await draft(a).inputValue(), 'luna 的私人草稿');
  data.cases.push('in-place launch, original SDK request, no route or Canvas remount, focus return');

  await button(/^技能$/, page.getByRole('navigation', { name: '办公室内容' })).click();
  const skills = page.getByRole('dialog', { name: '办公室技能' });
  await skills.waitFor();
  assert.equal(await page.locator('canvas').getAttribute('data-workspace-witness'), 'unchanged');
  await page.keyboard.press('Escape');
  await skills.waitFor({ state: 'hidden' });
  await b.waitFor();
  assert.equal(await draft(b).inputValue(), 'mora 的私人草稿');
  data.cases.push('office resources use bounded overlay; returning preserves the workspace');

  await page.setViewportSize({ width: 390, height: 675 });
  await a.waitFor({ state: 'hidden' });
  await b.waitFor();
  await noOverflow();
  await insideViewport(draft(b));
  await insideViewport(button(/^关闭会话/, b));
  await snap('narrow-reader');
  const narrowHistory = await history(b).boundingBox();
  await button('场景').focus();
  await page.keyboard.press('/');
  const search = page.getByLabel('搜索 Agent 或工作区');
  await search.waitFor();
  await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === '搜索 Agent 或工作区');
  assert((await history(b).boundingBox()).height < narrowHistory.height);
  await button('筛选').click();
  await button('@luna', manager).click();
  await a.waitFor();
  assert.equal(await draft(a).inputValue(), 'luna 的私人草稿');
  await button('聊天').click();
  await button('名册').click();
  await button(/^@sira/, page.getByRole('complementary', { name: '会话名单' })).click();
  await pane('sira').waitFor();
  await insideViewport(draft(pane('sira')));
  await noOverflow();
  await button(/^最小化会话/, pane('sira')).click();
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await button(/@sira.*已最小化/, manager).click();
  assert.equal(await pane('sira').locator('[data-window-motion-header]').first().evaluate(node => getComputedStyle(node).animationName), 'none');
  const sira = pane('sira');
  await draft(sira).fill('只发送给 sira');
  const siraMessages = '**/api/v1/agents/sira/messages';
  await page.route(siraMessages, async route => {
    if (route.request().method() !== 'POST') return route.continue();
    const response = await route.fetch();
    assert.equal(response.status(), 200, 'fixture committed the original send');
    await route.abort('failed');
  });
  await button('发送 AgentX 消息', sira).click();
  await sira.getByText('发送结果未确定 · 草稿已保留', { exact: true }).waitFor();
  assert.equal(await draft(sira).inputValue(), '只发送给 sira');
  const inlineRecovery = sira.getByRole('button', { name: /查询.*回执/ }).first();
  if (await inlineRecovery.count()) await inlineRecovery.click();
  else {
    const receipts = page.locator('.live-receipts');
    await receipts.locator('summary').click();
    await button('查询原回执', receipts.locator('section').filter({ hasText: 'send' })).click();
    await receipts.locator('summary').click();
  }
  await page.waitForFunction(() => document.querySelector('.space-window-pane textarea')?.value === '');
  const sends = fixture.calls.filter(call => call.method === 'POST' && call.path.endsWith('/messages'));
  assert.equal(sends.length, 1);
  assert.equal(sends[0].body.agent.incarnation, '103');
  assert.equal(sends[0].body.message.text, '只发送给 sira');
  assert(fixture.calls.some(call => call.method === 'GET' && call.path.endsWith('/operations/' + sends[0].body.request_id)), 'only query the original operation');
  await page.setViewportSize({ width: 320, height: 550 });
  await noOverflow();
  await insideViewport(draft(sira));
  await insideViewport(button('发送 AgentX 消息', sira));
  await snap('short-reader');
  data.cases.push('390×675 and 320×550 single reader, local filters, roster, reduced motion and original exact receipt after lost response');
  assert.deepEqual(data.errors, []);
  data.status = 'passed';
} catch (error) {
  data.status = 'failed';
  data.failure = String(error);
  await snap('failure').catch(() => {});
  process.exitCode = 1;
} finally {
  await browser.close();
  await fixture.close();
  await writeFile(join(output, 'report.json'), JSON.stringify(data, null, 2) + '\n');
  await writeFile(join(output, 'requests.json'), JSON.stringify(fixture.calls, null, 2) + '\n');
  console.log(JSON.stringify(data, null, 2));
}
