/** Finite presentation checks on a fresh actual Space build; all API fixtures are explicit synthetic data. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
const [build, evidence] = process.argv.slice(2);
assert(build && evidence, 'usage: node bindings.browser.mjs FRESH_BUILD NEW_EVIDENCE');
await mkdir(evidence, { recursive: false });
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', args: ['--no-sandbox', '--disable-dev-shm-usage'], headless: true });
const descriptor = { scope: '/synthetic/scope', generation: 'synthetic-generation', project: '/synthetic/project', tools: [], profiles: [], supports_workspace_members: true, supports_message_history: true, max_page: 100, max_text: 65536 };
const binding = { path: '/synthetic/project/.controller/task.md', schema: 'task', description: '实现消息 API', raw_status: 'in progress', status: 'ongoing', diagnostic: null, checked_at: 1730000000, last_success_at: 1730000000 };
const agent = (incarnation, file_bindings, stopped_event = null) => ({ agent: { name: 'toku', incarnation, stopped_event }, file_bindings, status: stopped_event ? 'stopped' : 'listening', stored_status: 'listening', status_context: '', request_error: null, pending_messages: 0, title: null, detail: '', directory: '/synthetic/project', tool: 'fixture', session_id: 'synthetic-session', collaboration: { status: 'adopted', workspace: null, role: null, reason: null }, can_send: true, can_stop: true, can_resume: false, has_transcript: false });
let rows = [agent('123', [binding, { ...binding, path: '/synthetic/delivery.md', schema: 'delivery', status: 'completed', raw_status: 'done', diagnostic: 'file missing; retained previous success', description: '交付消息 API' }])];
const report = { supply: 'synthetic API fixtures, actual production Space bundle', cases: [], mutations: [], errors: [] };
try {
 const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
 page.on('pageerror', error => report.errors.push(error.message));
 await page.route('http://space-bindings.test/**', async route => {
  const url = new URL(route.request().url());
  if (route.request().method() !== 'GET') report.mutations.push(route.request().method());
  if (url.pathname.startsWith('/api/')) {
   const path = url.pathname;
   const result = path === '/api/v1/management' ? descriptor : path === '/api/v1/agents' ? { generation: descriptor.generation, agents: rows, next: null, current_complete: true }
    : path === '/api/v1/workspaces' ? { generation: descriptor.generation, workspaces: [], coverage: { errors: [], scan_limit_reached: false, agent_limit_reached: false } }
    : path === '/api/v1/roles' ? { generation: descriptor.generation, workspace: null, roles: [] }
    : path.endsWith('/launch-relations') ? { generation: descriptor.generation, origin: 'unknown', upstream: null, downstream: [], next: null }
    : path.endsWith('/message-history') ? { generation: descriptor.generation, messages: [], next_before: null, live_after: '0' }
    : path === '/api/v1/office-activity' ? { generation: descriptor.generation, next_after: '1', messages: [], created: [], human_messages: [], human_has_more: false, has_more: false }
    : { generation: descriptor.generation, messages: [], has_more: false, next_after: '0' };
   return route.fulfill({ json: result });
  }
  const relative = url.pathname.replace(/^\/space\//, '') || 'index.html';
  const target = resolve(build, relative); assert(target.startsWith(resolve(build) + '/'));
  const type = { '.js': 'text/javascript', '.css': 'text/css', '.html': 'text/html', '.png': 'image/png', '.svg': 'image/svg+xml' }[extname(target)] ?? 'application/octet-stream';
  try { await route.fulfill({ body: await readFile(target), contentType: type }); } catch { await route.fulfill({ status: 404 }); }
 });
 async function open() {
  await page.goto('http://space-bindings.test/space/');
  await page.getByRole('button', { name: '聊天', exact: true }).click();
  if (rows[0].agent.stopped_event) await page.getByRole('button', { name: /^历史/ }).click();
  await page.getByRole('button', { name: /^@toku/ }).click();
 }
 await open();
 const work = page.locator('.bound-work');
 await work.locator(':scope > summary').focus(); await page.keyboard.press('Enter');
 await page.getByText('实现消息 API', { exact: true }).waitFor();
 assert((await work.innerText()).includes('Agent 运行状态 · listening'));
 assert((await work.innerText()).includes('文件状态 · 进行中'));
 assert((await work.innerText()).includes('上次成功快照 · 已完成'));
 assert((await work.innerText()).includes('读取异常'));
 await work.locator('article').first().locator('summary').click();
 await work.getByText(binding.path, { exact: true }).waitFor();
 const composer = page.getByRole('textbox', { name: 'Agent 消息草稿' });
 await composer.fill('精确实例草稿');
 await page.setViewportSize({ width: 390, height: 844 });
 await work.locator(':scope > summary').click();
 assert.equal(await composer.inputValue(), '精确实例草稿');
 const box = await composer.boundingBox(); assert(box && box.y >= 0 && box.y + box.height <= 844);
 await page.screenshot({ path: resolve(evidence, 'narrow.png') });
 report.cases.push('keyboard disclosure; distinct file/runtime states; retained-error snapshot/path; narrow composer and draft');
 rows = [agent(null, [], '44')]; await open();
 await page.locator('.bound-work > summary').click();
 await page.getByText('当前观测未提供此精确实例的文件绑定').waitFor();
 assert(!(await page.locator('.bound-work').innerText()).includes('实现消息 API'));
 report.cases.push('same-name stopped record does not reuse current bindings');
 rows = [agent('456', [{ ...binding, status: 'unknown', raw_status: 'custom-state' }])]; await open();
 await page.locator('.bound-work > summary').click();
 await page.getByText('文件状态 · 未识别', { exact: true }).waitFor();
 await page.locator('.bound-work article summary').click();
 await page.getByText('custom-state', { exact: true }).waitFor();
 report.cases.push('successor unknown status retains raw declaration');
 assert.deepEqual(report.mutations, []); assert.deepEqual(report.errors, []);
 report.status = 'pass';
} catch (error) { report.status = 'fail'; report.failure = String(error); process.exitCode = 1; }
finally { await browser.close(); await writeFile(resolve(evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n'); console.log(JSON.stringify(report)); }
