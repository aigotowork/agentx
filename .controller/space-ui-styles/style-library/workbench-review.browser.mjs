/** Verify the formal app's style switch over a disposable workspace. */
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { SPACE_STYLES, spaceWorld, usesStudioArt } from "../../../web/agent-space/uiux/systems/space/components/presentation.ts";

const [base, output] = process.argv.slice(2);
assert(base && output, "Usage: node workbench-review.browser.mjs SYNTHETIC_SPACE_URL OUTPUT");
await mkdir(output, { recursive: true });
const { chromium } = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? "/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright");
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? "/usr/bin/google-chrome", headless: true, args: ["--no-sandbox"] });
const report = { base, browser: browser.version(), styles: [], problems: [] };
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, reducedMotion: "reduce" });
const page = await context.newPage();
page.setDefaultTimeout(8000);
const requestPaths = [];
await context.route("**/*", async route => {
  const req = route.request();
  const url = new URL(req.url());
  if (req.method() !== "GET" || url.origin !== new URL(base).origin) {
    report.problems.push([req.method(), url.pathname]);
    return route.abort();
  }
  requestPaths.push(url.pathname);
  return route.continue();
});
page.on("pageerror", error => report.problems.push(error.message));
page.on("response", response => {
  const path = new URL(response.url()).pathname;
  if (response.status() >= 400 && path !== "/dancingai/service.json" && path !== "/favicon.ico") {
    report.problems.push([response.status(), path]);
  }
});

try {
  const descriptor = await (await page.request.get(new URL("/api/v1/management", base).href)).json();
  assert.equal(descriptor.scope, "/fixture/space-workspace");
  const entry = new URL("?style=classic&office=%2Fprojects%2Fworkspace", base);
  await page.goto(entry.href);

  const scene = page.locator(".office-scene");
  const showScene = async style => {
    const world = spaceWorld(style);
    if (usesStudioArt(style)) {
      const extra = world ? `[data-scene-world="${world}"]` : ":not([data-scene-world])";
      await scene.and(page.locator(`[data-scene-art="studio"][data-art-status="ready"]${extra}`)).waitFor({ state: "attached" });
    } else {
      await scene.and(page.locator(':not([data-scene-art="studio"])')).waitFor({ state: "attached" });
    }
  };
  await showScene("classic");
  await page.locator(".office-camera-controls button[aria-expanded]").click();
  await page.getByRole("region", { name: "场景名册" }).getByRole("button", { name: /^@luna/ }).click();
  const reader = page.getByRole("region", { name: "@luna", exact: true });
  await reader.locator('[data-history-record="120"]').waitFor();
  const draft = reader.getByRole("textbox", { name: "Agent 消息草稿" });
  await draft.fill("风格切换保留这份草稿。");
  const ref = new URL(page.url()).searchParams.get("incarnation");
  assert.equal(ref, "101");
  const exact = new URL(page.url());
  const artCanvas = await scene.locator("canvas").elementHandle();
  const latestReads = () => requestPaths.filter(path => path === "/api/v1/agents/luna/message-history").length;
  const before = latestReads();
  await page.locator(".workspace-display-menu > summary").press("Enter");
  const select = page.locator('.workspace-display-menu select[aria-label="界面风格"]');
  assert.deepEqual(await select.locator("option").evaluateAll(options => options.map(option => option.value)), SPACE_STYLES.map(style => style.value));
  for (const { value } of SPACE_STYLES) {
    await select.selectOption(value);
    await showScene(value);
    assert.equal(new URL(page.url()).searchParams.get("style"), value);
    assert.equal(new URL(page.url()).searchParams.get("incarnation"), ref);
    assert.equal(await draft.inputValue(), "风格切换保留这份草稿。");
    assert(await scene.locator("canvas").evaluate((node, original) => node === original, artCanvas));
    report.styles.push(value);
  }
  assert.equal(latestReads(), before);
  report.latestReadsAcrossStyleChanges = latestReads() - before;

  for (const oldStyle of ["classic", "folio", "signal"]) {
    const legacy = new URL(exact);
    legacy.searchParams.set("style", oldStyle);
    legacy.searchParams.set("art", "studio");
    await page.goto(legacy.href);
    await showScene("studio");
    await reader.locator('[data-history-record="120"]').waitFor();
    await page.locator(".workspace-display-menu > summary").click();
    assert.equal(await select.inputValue(), oldStyle === "classic" ? "studio" : oldStyle);
    await select.selectOption("classic");
    await showScene("classic");
    const canonical = new URL(page.url());
    assert.equal(canonical.searchParams.has("art"), false);
    assert.equal(canonical.searchParams.get("style"), "classic");
    assert.equal(canonical.searchParams.get("incarnation"), ref);
  }
  report.legacy = "Studio and paired Folio/Signal links open; switching to pixels removes the old art hint";

  await select.selectOption("studio");
  await showScene("studio");
  const implicit = new URL(exact);
  implicit.searchParams.delete("style");
  await page.goto(implicit.href);
  await showScene("studio");
  report.saved = "a fresh link without a style uses the last explicit selection";

  const phone = await browser.newPage({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, reducedMotion: "reduce" });
  await phone.route("**/*", route => {
    if (route.request().method() !== "GET" || new URL(route.request().url()).origin !== entry.origin) {
      report.problems.push("Unexpected phone request");
      return route.abort();
    }
    return route.continue();
  });
  phone.on("pageerror", error => report.problems.push(error.message));
  await phone.goto(exact.href);
  const phoneDraft = phone.getByRole("textbox", { name: "Agent 消息草稿" });
  await phoneDraft.fill("手机草稿保持原位");
  await phone.locator(".workspace-display-menu > summary").tap();
  const phoneSelect = phone.locator('.workspace-display-menu select[aria-label="界面风格"]');
  const hit = await phoneSelect.boundingBox();
  assert(hit && hit.x >= 0 && hit.x + hit.width <= 390, "style selector fits the phone viewport");
  await phoneSelect.selectOption("studio");
  await phone.locator('.office-scene[data-scene-art="studio"][data-art-status="ready"]').waitFor();
  assert.equal(await phoneDraft.inputValue(), "手机草稿保持原位");
  assert.equal(new URL(phone.url()).searchParams.get("incarnation"), ref);
  report.phone = "390px touch emulation retains reader and draft";
  await phone.close();
  assert.deepEqual(report.problems, []);
} finally {
  await browser.close();
  await writeFile(join(output, "styles.json"), JSON.stringify(report, null, 2) + "\n");
}
