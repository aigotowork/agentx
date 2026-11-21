/** Compare two presentations over the same public SDK and operate both UIUX candidate groups. */
import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { join } from "node:path";

const [product, canvas, output] = process.argv.slice(2);
assert(product && canvas && output, "Pass product base, canvas base and a new evidence directory");
const { chromium } = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? "playwright");
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? "/usr/bin/google-chrome", args: ["--no-sandbox", "--disable-dev-shm-usage"] });
await mkdir(output);
const report = { status: "fail", product, canvas, checks: [], issues: [], absentOptionalResources: [] };
const optional = new Set(["/dancingai/service.json", "/__uiux/visual", "/__uiux/sitemap"]);
const button = (page, name) => page.getByRole("button", { name, exact: typeof name === "string" });
const reader = page => page.getByRole("textbox", { name: "Agent 消息草稿" });
const actor = page => page.locator("button.office-actor").and(button(page, /^@mora /));
const ready = page => page.locator('[data-scene-art="studio"][data-art-status="ready"]').first().waitFor();
const theme = page => page.locator(".space-system").first().getAttribute("data-space-style");
const switcher = page => page.getByRole("navigation", { name: "界面风格" });
const shot = async (page, name) => {
  await page.evaluate(async () => { await document.fonts.ready; await new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))); });
  await page.screenshot({ path: join(output, name + ".png") });
};
function address(style) {
  const url = new URL(product);
  url.search = new URLSearchParams({ art: "studio", ...(style ? { style } : {}), office: "/studio/catalog" });
  return url.href;
}
function monitor(page) {
  const api = [];
  page.setDefaultTimeout(9000);
  page.on("pageerror", error => report.issues.push(error.message));
  page.on("response", response => {
    const path = new URL(response.url()).pathname;
    if (response.status() === 404 && optional.has(path)) {
      if (!report.absentOptionalResources.includes(path)) report.absentOptionalResources.push(path);
    } else if (response.status() >= 400) report.issues.push(response.status() + " " + response.url());
  });
  page.on("request", request => {
    if (new URL(request.url()).pathname.startsWith("/api/v1/")) api.push({ method: request.method(), url: new URL(request.url()) });
  });
  return api;
}
function noWrites(api) {
  assert(api.every(trace => trace.method === "GET" && trace.url.origin === new URL(product).origin));
}
async function focused(page, locator) {
  await locator.evaluate(node => new Promise((resolve, reject) => {
    const stop = performance.now() + 1500;
    function check() { if (node.ownerDocument.activeElement === node) resolve(); else if (performance.now() > stop) reject(new Error("Focus did not return to " + node.outerHTML.slice(0, 180))); else requestAnimationFrame(check); }
    check();
  }));
}
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: "reduce" });
  const api = monitor(page);
  const dock = page.locator('.office-expanded-object[data-kind="conversation"]:visible');
  await page.goto(address("folio"));
  await ready(page);
  await actor(page).waitFor();
  await shot(page, "folio-office");
  await actor(page).click();
  await page.getByText(/版片编号是 R-14/).waitFor();
  const subject = new URL(page.url()).searchParams.get("incarnation");
  assert.equal(subject, "101");
  assert.equal(await theme(page), "folio");
  const draft = "下轮校样保留 R-14 的新署名";
  await reader(page).fill(draft);
  const folio = await dock.boundingBox();
  await shot(page, "folio-reader");
  const exact = () => api.filter(r => /\/agents\/mora(?:\/messages)?$/.test(r.url.pathname));
  const before = exact().length;
  await button(page.getByRole("navigation", { name: "界面风格" }), "联络台").click();
  assert.equal(await theme(page), "signal");
  const signal = await dock.boundingBox();
  assert(folio && signal && signal.x > folio.x + 100 && folio.width > signal.width + 60, "variants must place the real reader differently");
  assert.equal(await reader(page).inputValue(), draft);
  assert.equal(new URL(page.url()).searchParams.get("incarnation"), subject);
  await shot(page, "signal-reader");
  await button(switcher(page), "保留原版").click();
  assert.equal(await theme(page), null);
  assert.equal(await reader(page).inputValue(), draft);
  assert.equal(new URL(page.url()).searchParams.get("style"), "classic");
  await button(switcher(page), "联络台").click();
  await button(page.getByRole("navigation", { name: "呈现方式" }), "聊天").click();
  assert.equal(await reader(page).inputValue(), draft);
  assert.equal(exact().length, before, "appearance and list switches reuse the same exact reader");
  await shot(page, "signal-list");
  await button(page.getByRole("navigation", { name: "呈现方式" }), "场景").click();
  await button(page, "放大阅读").click();
  assert.equal(await dock.getAttribute("data-expanded"), "true");
  assert.equal(await reader(page).inputValue(), draft);
  await button(page, "关闭并返回办公室").click();
  await focused(page, actor(page));
  await shot(page, "signal-office");
  for (const [style, name] of [["folio", "信笺"], ["signal", "联络台"]]) {
    await button(page.getByRole("navigation", { name: "界面风格" }), name).click();
    const command = button(page, /Agent$/).and(page.locator(".space-new-agent"));
    await command.click();
    const dialog = page.getByRole("dialog", { name: "新建 Agent" });
    await dialog.waitFor();
    assert(await button(dialog, "启动").isDisabled());
    await shot(page, style + "-modal");
    await dialog.screenshot({ path: join(output, style + "-modal-detail.png") });
    await page.keyboard.press("Escape");
    await dialog.waitFor({ state: "hidden" });
    await focused(page, command);
  }
  assert(exact().some(r => r.url.pathname.endsWith("/messages") && r.url.searchParams.get("incarnation") === subject));
  await page.goto(address());
  assert.equal(await theme(page), "signal", "the last explicit choice is saved in this browser");
  await page.goto(address("classic"));
  assert.equal(await theme(page), null, "a valid style link wins over the saved choice");
  noWrites(api);
  report.checks.push("Desktop Folio and Signal: different real window geometry; in-place theme switch, exact reader and draft shared with list; expand/return focus; new-Agent modal opens, cancels and cannot submit.");
  await page.close();
  for (const style of ["folio", "signal"]) {
    const phone = await browser.newPage({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, reducedMotion: "reduce" });
    const mobileApi = monitor(phone);
    await phone.goto(address(style));
    await ready(phone);
    await actor(phone).waitFor({ state: "attached" });
    await button(phone, "名册").tap();
    await button(phone.getByRole("region", { name: "Agent 名册" }), /^@mora /).tap();
    await phone.getByText(/版片编号是 R-14/).waitFor();
    await reader(phone).fill("手机草稿");
    const box = await reader(phone).boundingBox();
    assert(box && box.x >= 0 && box.x + box.width <= 390 && box.y + box.height <= 844);
    assert.equal(await button(phone, "返回办公室").count(), 1);
    assert.equal(await button(phone, "关闭并返回办公室").count(), 0);
    await shot(phone, style + "-mobile");
    const mobileExact = () => mobileApi.filter(r => /\/agents\/mora(?:\/messages)?$/.test(r.url.pathname));
    const beforeMobile = mobileExact().length;
    await button(switcher(phone), "原版").tap();
    assert.equal(await theme(phone), null);
    assert.equal(await reader(phone).inputValue(), "手机草稿");
    await button(switcher(phone), style === "folio" ? "信笺" : "联络台").tap();
    assert.equal(await theme(phone), style);
    assert.equal(mobileExact().length, beforeMobile);
    await button(phone, "返回办公室").tap();
    await button(phone.getByRole("navigation", { name: "呈现方式" }), "聊天").tap();
    const target = phone.locator("button[data-agent-key]").and(button(phone, /^@mora /));
    await target.focus();
    await phone.keyboard.press("Enter");
    assert.equal(await reader(phone).inputValue(), "手机草稿");
    noWrites(mobileApi);
    await phone.close();
    report.checks.push(style + ": narrow touch and keyboard reach the same draft after the single room return.");
  }

  const board = await browser.newPage({ viewport: { width: 1600, height: 1000 }, reducedMotion: "reduce" });
  const canvasApi = monitor(board);
  await board.goto(new URL("#candidates", canvas).href);
  const copies = style => board.locator(`[data-space-style="${style}"]`);
  for (const style of ["folio", "signal"]) {
    const visual = copies(style).filter({ has: board.getByRole("textbox", { name: "Agent 消息草稿" }) });
    await visual.getByText(/版片编号是 R-14/).waitFor();
    await reader(visual).fill("候选稿 " + style);
    const command = copies(style).filter({ has: button(board, "终止进程") });
    // Loaded lazily as the comparison card enters the viewport.
    await button(command, "终止进程").click();
    const dialog = command.getByRole("dialog", { name: "终止进程" });
    await focused(board, button(dialog, "取消"));
    await shot(board, style + "-canvas-confirmation");
    await dialog.screenshot({ path: join(output, style + "-confirmation-detail.png") });
    await board.keyboard.press("Escape");
    await focused(board, button(command, "终止进程"));
    await button(command, "终止进程").click();
    await button(dialog, "确认终止").click();
    await command.getByText("已确认 · 仅演示").waitFor();
    assert.equal(await reader(visual).inputValue(), "候选稿 " + style);
  }
  assert.deepEqual(canvasApi, []);
  report.checks.push("Design Canvas registers four real component candidates. Independent drafts, safe default Cancel focus, Escape restoration and explicit local confirmation all work.");
  assert.deepEqual(report.issues, []);
  report.status = "pass";
} catch (error) {
  report.failure = String(error.stack || error);
  process.exitCode = 1;
} finally {
  await writeFile(join(output, "report.json"), JSON.stringify(report, null, 2) + "\n");
  console.log(JSON.stringify(report, null, 2));
  await browser.close();
}
