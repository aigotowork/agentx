/** Capture the actual appearance registry and operate the same conversation across the four new worlds. */
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { SPACE_LIBRARY_WORLDS, SPACE_STYLES, spaceWorld, usesStudioArt } from "../../../web/agent-space/uiux/systems/space/components/presentation.ts";

const [product, design, output] = process.argv.slice(2);
assert(product && design && output, "Pass the isolated product, Design Canvas and evidence directory");
await mkdir(output);
const { chromium } = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? "/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright");
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? "/usr/bin/google-chrome", headless: true, args: ["--no-sandbox"] });
const report = { status: "fail", product, design, styles: [], designScenes: [], faults: [], optionalMetadata: [] };
const button = (root, label) => root.getByRole("button", { name: label, exact: typeof label === "string" });
const actor = root => root.locator("button.office-actor").filter({ hasText: "@mora" });
const composer = root => root.getByRole("textbox", { name: "Agent 消息草稿" });
const story = root => root.getByText(/版片编号是 R-14/).first();
const lamp = root => button(root, "场景灯").first();
const styleSelect = page => page.locator('select[aria-label="界面风格"]:visible').first();
const stage = root => root.locator(".office-scene");
function link(style) {
  const url = new URL(product);
  url.search = new URLSearchParams({ office: "/studio/catalog", style });
  return url.href;
}
function recordFaults(page, label) {
  page.on("pageerror", error => report.faults.push({ label, error: error.message }));
  const optional = label === "design" ? new Set(["/__uiux/visual", "/__uiux/sitemap"]) : new Set(["/dancingai/service.json"]);
  page.on("response", response => {
    const path = new URL(response.url()).pathname, status = response.status();
    if (status < 400) return;
    const result = { label, status, path };
    // Static review artifacts have no optional tools backend or standalone sitemap/visual model.
    if (status === 404 && optional.has(path)) report.optionalMetadata.push(result);
    else report.faults.push(result);
  });
}
async function waitStage(root, style) {
  if (usesStudioArt(style)) {
    const world = spaceWorld(style);
    await stage(root).and(root.locator(`[data-scene-art="studio"][data-art-status="ready"]${world ? `[data-scene-world="${world}"]` : ":not([data-scene-world])"}`)).waitFor();
  }
  else assert.notEqual(await stage(root).getAttribute("data-scene-art"), "studio");
}
async function frame(page, name, target = page) {
  await page.evaluate(() => document.fonts.ready.then(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))));
  await target.screenshot({ path: join(output, name + ".png"), timeout: 9000 });
}

try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: "reduce" });
  page.setDefaultTimeout(12000);
  recordFaults(page, "product");
  page.on("request", request => { if (request.method() === "POST") report.faults.push({ error: "Read-only review attempted a POST", url: request.url() }); });
  for (const { value } of SPACE_STYLES) {
    await page.goto(link(value), { waitUntil: "domcontentloaded" });
    await actor(page).waitFor();
    await waitStage(page, value);
    await frame(page, value + "-office");
    if (value === "citrus") {
      await lamp(page).focus();
      const before = await stage(page).locator("canvas").evaluate(canvas => canvas.toDataURL());
      await page.keyboard.press("Space");
      await page.waitForFunction(previous => document.querySelector(".office-scene canvas").toDataURL() !== previous, before);
      assert.equal(await lamp(page).getAttribute("aria-pressed"), "false");
      await frame(page, value + "-lamp-off");
      await page.keyboard.press("Space");
    }
    await actor(page).click();
    await story(page).waitFor();
    await composer(page).fill("R-14 · 下轮校样");
    assert(await button(page, "发送 AgentX 消息").isDisabled());
    await frame(page, value + "-reader");
    report.styles.push(value);
  }

  const chatLink = new URL(link("citrus"));
  chatLink.searchParams.set("view", "chat");
  await page.goto(chatLink.href, { waitUntil: "domcontentloaded" });
  await page.locator("button[data-agent-key]").filter({ hasText: "@mora" }).click();
  await story(page).waitFor();
  assert.deepEqual(await page.getByRole("navigation", { name: "记录类型" }).getByRole("button").allTextContents(), ["消息", "Transcript"]);
  await frame(page, "citrus-chat");
  report.chat = "Citrus chat list opens the same @mora messages and Transcript tabs";

  const phone = await browser.newPage({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 1, reducedMotion: "reduce" });
  phone.setDefaultTimeout(12000);
  recordFaults(phone, "phone");
  await phone.goto(link("reverie"), { waitUntil: "domcontentloaded" });
  await waitStage(phone, "reverie");
  await actor(phone).tap();
  await composer(phone).fill("手机草稿");
  const exact = new URL(phone.url()).searchParams.get("incarnation");
  for (const { value } of SPACE_LIBRARY_WORLDS) {
    await styleSelect(phone).selectOption(value);
    assert.equal(new URL(phone.url()).searchParams.get("incarnation"), exact);
    assert.equal(await composer(phone).inputValue(), "手机草稿");
    assert.equal(await button(phone, "Transcript").count(), 1);
  }
  await button(phone, "Transcript").tap();
  assert.equal(await composer(phone).count(), 0);
  await button(phone, "消息").tap();
  assert.equal(await composer(phone).inputValue(), "手机草稿");
  assert(await phone.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  await frame(phone, "collage-phone-reader");
  report.phone = { width: 390, exact, styles: SPACE_LIBRARY_WORLDS.map(({ value }) => value) };

  const board = await browser.newPage({ viewport: { width: 1600, height: 1000 }, reducedMotion: "reduce" });
  board.setDefaultTimeout(14000);
  recordFaults(board, "design");
  board.on("request", request => { if (new URL(request.url()).pathname.startsWith("/api/v1")) report.faults.push({ error: "Design Canvas invoked native API", url: request.url() }); });
  await board.goto(new URL("#candidates", design).href, { waitUntil: "domcontentloaded" });
  for (const { value, label } of SPACE_LIBRARY_WORLDS) {
    const card = board.getByRole("article", { name: label, exact: true });
    await card.scrollIntoViewIfNeeded();
    const root = board.locator(`[data-space-world="${value}"]`);
    await root.scrollIntoViewIfNeeded();
    await waitStage(root, value);
    if (value === "reverie") {
      await actor(root).click();
      await composer(root).fill("本地演示草稿");
      assert(await button(root, "发送 AgentX 消息").isDisabled());
      await button(root, "Transcript").click();
      assert.equal(await composer(root).count(), 0);
      await button(root, "消息").click();
      assert.equal(await composer(root).inputValue(), "本地演示草稿");
      await frame(board, "design-reverie-reader", root);
    }
    report.designScenes.push(value);
  }
  assert.deepEqual(report.faults, []);
  report.status = "pass";
} catch (error) {
  report.error = String(error);
  throw error;
} finally {
  await writeFile(join(output, "report.json"), JSON.stringify(report, null, 2));
  await browser.close();
}
