/** Read-only selected-service check after deployment; never creates assignments or posts operations. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const [base, evidence] = process.argv.slice(2);
assert(base && evidence, 'usage: node bindings.live.mjs BASE NEW_EVIDENCE');
await mkdir(evidence, { recursive: false });
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const report = { base, mutations: [], errors: [] };
try {
 const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
 const response = await context.request.get(`${base}/api/v1/management`); assert(response.ok());
 const descriptor = await response.json();
 const listing = await context.request.get(`${base}/api/v1/agents?generation=${descriptor.generation}&limit=100`); assert(listing.ok());
 const rows = (await listing.json()).agents;
 const native = rows.find(row => row.agent.incarnation && !row.agent.stopped_event);
 assert(native, 'first page must contain a current exact instance for read-only focus');
 const page = await context.newPage();
 page.on('pageerror', error => report.errors.push(error.message));
 page.on('request', request => { if (!['GET','HEAD','OPTIONS'].includes(request.method())) report.mutations.push({ method: request.method(), url: request.url() }); });
 const query = new URLSearchParams({ scope: descriptor.scope, generation: descriptor.generation, agent: native.agent.name, incarnation: native.agent.incarnation, view: 'chat' });
 await page.goto(`${base}/space/?${query}`, { waitUntil: 'domcontentloaded' });
 const work = page.locator('.bound-work');
 await work.locator(':scope > summary').click({ timeout: 45000 });
 await work.getByText(`Agent 运行状态 · ${native.status} · 当前实例`, { exact: true }).waitFor();
 assert.equal(await work.locator('article').count(), native.file_bindings.length);
 if (!native.file_bindings.length) await work.getByText('当前观测未提供此精确实例的文件绑定').waitFor();
 await page.screenshot({ path: resolve(evidence, 'desktop.png') });
 await page.setViewportSize({ width: 390, height: 844 });
 const composer = page.getByRole('textbox', { name: 'Agent 消息草稿' });
 const box = await composer.boundingBox(); assert(box && box.y >= 0 && box.y + box.height <= 844);
 await page.screenshot({ path: resolve(evidence, 'narrow.png') });
 assert.deepEqual(report.mutations, []); assert.deepEqual(report.errors, []);
 report.status = 'pass'; report.generation = descriptor.generation; report.agent = native.agent; report.bindingCount = native.file_bindings.length;
 report.limits = 'Actual native GETs and rendered UI; no assignment creation, task state edit or provider operation.';
} catch (error) { report.status = 'fail'; report.failure = String(error); process.exitCode = 1; }
finally { await browser.close(); await writeFile(resolve(evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n'); console.log(JSON.stringify(report)); }
