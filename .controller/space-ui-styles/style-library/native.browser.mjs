/** Check that the pixel and illustrated Space share the exact native conversation. */
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const configPath = process.argv[2];
const config = JSON.parse(await readFile(configPath, "utf8"));
const out = dirname(configPath);
const { chromium } = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? "/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright");
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? "/usr/bin/google-chrome", headless: true, args: ["--no-sandbox"] });
const report = { status: "fail", exact: config.reference, requests: [], pageErrors: [], cases: [] };
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: "reduce" });
  page.setDefaultTimeout(20000);
  page.on('pageerror', error => report.pageErrors.push(String(error)));
  page.on("request", request => {
    const path = new URL(request.url()).pathname;
    if (path.startsWith("/api/v1")) report.requests.push({ path, search: new URL(request.url()).search, origin: new URL(request.url()).origin, method: request.method(), body: request.postData() });
  });
  const button = label => page.getByRole("button", { name: label, exact: true });
  const stage = page.locator(".office-scene");
  const actor = page.locator("button.office-actor").filter({ hasText: "@" + config.reference.name });
  const channels = page.getByRole("navigation", { name: "记录类型" });
  const composer = page.getByRole("textbox", { name: "Agent 消息草稿" });
  async function changeStyle(value) {
    const input = page.locator('select[aria-label="界面风格"]:visible').first();
    const disclosure = page.locator('summary[aria-label="显示与反馈"]');
    const open = !(await input.isVisible()) && await disclosure.isVisible();
    if (open) await disclosure.click();
    await input.selectOption(value);
    if (open) await disclosure.click();
  }
  const writes = () => report.requests.filter(request => request.method === "POST");
  const route = () => Object.fromEntries([...new URL(page.url()).searchParams].filter(([key]) => key !== "style"));
  const url = new URL(config.url);
  url.search = new URLSearchParams({ office: config.workspace, style: "classic" });
  if (config.embedded) url.searchParams.set("managementUrl", "http://127.0.0.1:1/");
  await page.goto(url.href);
  await actor.click();
  await composer.waitFor();
  if (config.nativeDomain) {
    assert(report.requests.some(request => request.path === '/api/v1/workspaces/members'
      && new URLSearchParams(request.search).get('workspace') === config.workspace));
    assert(!report.requests.some(request => request.path === '/api/v1/agents'), 'a known office uses the native member feed only');
    report.cases.push('Known workspace opens through the native membership selector using the final Domain SDK');
    const relations = page.locator('.launch-relations');
    const disclosure = page.getByText('启动关系', { exact: true });
    const open = !(await relations.isVisible());
    if (open) await disclosure.click();
    await relations.getByText('暂无启动记录', { exact: true }).waitFor();
    assert(report.requests.some(request => request.path === `/api/v1/agents/${config.reference.name}/launch-relations` && request.method === 'GET'));
    if (open) await disclosure.click();
    report.cases.push('The conversation disclosure reads launch provenance without creating an operation');
  }
  if (config.embedded) {
    assert.equal(await page.locator('meta[name="agentx-space-scope"]').getAttribute('content'), config.scope);
    assert(report.requests.every(request => request.origin === url.origin));
    if (!config.nativeDomain) assert(report.requests.some(request => request.path === '/api/v1/agents' && new URLSearchParams(request.search).get('limit') === '20'));
    report.cases.push('Native Space pins the embedded host scope; an obsolete URL cannot replace the listener; workspace roster uses bounded SDK paging');
  }
  const selected = route();
  assert.equal(selected.agent, config.reference.name);
  assert.equal(selected.incarnation, String(config.reference.incarnation));
  assert.notEqual(await stage.getAttribute("data-scene-art"), "studio");
  assert.deepEqual(await channels.getByRole("button").allTextContents(), ["消息", "Transcript"]);
  assert.equal(writes().length, 0);
  const message = "Space native: exact draft across pixel and watercolor";
  await composer.fill(message);
  assert(await button("发送 AgentX 消息").isEnabled());
  await changeStyle("wash");
  await page.locator('.office-scene[data-scene-world="wash"][data-art-status="ready"]').waitFor();
  assert.deepEqual(route(), selected);
  assert.equal(await composer.inputValue(), message);
  assert(await button("发送 AgentX 消息").isEnabled());
  assert.equal(writes().length, 0);
  report.cases.push("Pixel → watercolor preserves exact route, draft and send capability; no writes on selection or switch");
  const sentResponse = page.waitForResponse(response => response.request().method() === "POST" && /\/messages$/.test(new URL(response.url()).pathname));
  await button("发送 AgentX 消息").click();
  assert.equal((await sentResponse).status(), 202);
  await page.getByText("已发送", { exact: true }).waitFor();
  await page.getByText(message, { exact: true }).waitFor();
  await page.getByText(/fixture reply #/).first().waitFor();
  assert.equal(await composer.inputValue(), "");
  await page.screenshot({ path: join(out, "native-wash-sent.png") });
  report.cases.push("Watercolor sends through the native daemon, resolves its receipt and shows the native Provider fixture reply");
  const held = "unsent draft held during Transcript";
  await composer.fill(held);
  await button("Transcript").click();
  assert.equal(await composer.count(), 0);
  assert.equal(writes().length, 1);
  const readResponse = page.waitForResponse(response => response.request().method() === "POST" && /\/transcript$/.test(new URL(response.url()).pathname));
  await button("读取 Transcript").click();
  assert.equal((await readResponse).status(), 202);
  await page.getByText(/live acceptance initial fixture/).last().waitFor();
  await changeStyle("classic");
  assert.notEqual(await stage.getAttribute("data-scene-art"), "studio");
  assert.equal(await composer.count(), 0);
  await page.getByText(/live acceptance initial fixture/).last().waitFor();
  await page.screenshot({ path: join(out, "native-classic-transcript.png") });
  await button("消息").click();
  assert.deepEqual(route(), selected);
  assert.equal(await composer.inputValue(), held);
  assert(await button("发送 AgentX 消息").isEnabled());
  await button("聊天").click();
  assert.equal(await composer.inputValue(), held);
  assert.equal(route().agent, selected.agent);
  await button("场景").click();
  assert.deepEqual(route(), selected);
  assert.equal(writes().length, 2);
  for (const { body } of writes()) {
    // Preserve the numeric token: native u64 incarnation can exceed JS's exact integer range.
    assert(body.includes(String(config.reference.incarnation)));
    assert.equal(JSON.parse(body).generation, config.generation);
  }
  report.cases.push("Explicit native Transcript stays read-only through a switch to pixel; returning to messages restores the original draft");
  if (config.embedded) {
    await page.goto(config.url);
    await button("聊天").click();
    await page.locator('.hall-directory-person').filter({ hasText: '@' + config.reference.name }).first().click();
    await page.getByText(message, { exact: true }).waitFor();
    await composer.fill('Unsent draft in the all-offices list');
    assert(await button("发送 AgentX 消息").isEnabled());
    assert.equal(writes().length, 2);
    report.cases.push('The unconfigured URL opens the native all-offices roster; the same exact reader shows the sent message without replay');
  }
  assert.deepEqual(report.pageErrors, []);
  report.status = "pass";
} catch (error) {
  report.error = String(error);
  const page = browser.contexts()[0]?.pages()[0];
  await page?.screenshot({ path: join(out, "native-failure.png"), timeout: 3000 }).catch(() => {});
  throw error;
} finally {
  await writeFile(join(out, "browser-report.json"), JSON.stringify(report, null, 2));
  await browser.close();
}
