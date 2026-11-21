/** Read-only smoke of the adopted appearance selector against the real project. */
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFile, writeFile } from "node:fs/promises";
import { SPACE_STYLES } from "../../web/agent-space/uiux/systems/space/components/presentation.ts";

const [config, output] = process.argv.slice(2);
const { origin, scope, generation, name, reference } = JSON.parse(await readFile(config, "utf8"));
const query = new URLSearchParams({ scope, generation, agent: name, record: reference.stopped_event ? "stopped" : "current", ...reference, style: "classic" });
const { chromium } = createRequire(import.meta.url)(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? "/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright");
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? "/usr/bin/google-chrome", headless: true, args: ["--no-sandbox"] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: "reduce" });
page.setDefaultTimeout(10000);
const report = { origin, status: "failed", styles: [], latestReads: 0, issues: [] };
const historyPath = "/api/v1/agents/" + encodeURIComponent(name) + "/message-history";
await page.route("**/*", route => {
  const request = route.request(), url = new URL(request.url());
  if (request.method() !== "GET" || url.origin !== origin) {
    report.issues.push("unexpected non-read or external request");
    return route.abort();
  }
  if (url.pathname === historyPath && !url.searchParams.has("before")) report.latestReads++;
  return route.continue();
});
page.on("pageerror", () => report.issues.push("uncaught page error"));
page.on("response", response => {
  const path = new URL(response.url()).pathname;
  if (response.status() >= 400 && (path.startsWith("/api/") || path.startsWith("/space/assets/"))) {
    report.issues.push([response.status(), path.startsWith("/api/") ? "API" : "asset"]);
  }
});
try {
  await page.goto(origin + "/space/?" + query);
  const reader = page.getByRole("region", { name: "@" + name, exact: true });
  await reader.locator("[data-history-record]").first().waitFor();
  await page.getByLabel("显示与反馈").click();
  const picker = page.locator(".workspace-display-menu").getByRole("combobox", { name: "界面风格" });
  assert.deepEqual(await picker.locator("option").evaluateAll(options => options.map(option => option.value)), SPACE_STYLES.map(style => style.value));
  const scene = page.locator(".office-scene");
  for (const [style, art] of [
    ["studio", '[data-scene-art="studio"][data-art-status="ready"]:not([data-scene-world])'],
    ["lunar", '[data-scene-world="lunar"][data-art-status="ready"]'],
    ["classic", ':not([data-scene-art="studio"])'],
  ]) {
    await picker.selectOption(style);
    await scene.and(page.locator(art)).waitFor({ state: "attached" });
    const current = new URL(page.url()).searchParams;
    for (const [key, value] of Object.entries(reference)) assert.equal(current.get(key), value);
    assert.equal(current.get("scope"), scope);
    assert.equal(current.get("style"), style);
    assert(await reader.locator("[data-history-record]").count() > 0);
    report.styles.push(style);
  }
  assert.equal(report.latestReads, 1);
  assert.deepEqual(report.issues, []);
  report.status = "passed";
} finally {
  await browser.close();
  await writeFile(output, JSON.stringify(report, null, 2) + "\n");
}
