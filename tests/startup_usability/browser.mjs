/** Verify the real short-command page loads fixed read-only project resources. */
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdir, readFile, realpath, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { checkpoint } from "../collaboration_workbench/browser_checkpoint.mjs";

const [inputPath, evidence] = process.argv.slice(2);
const input = JSON.parse(await readFile(inputPath, "utf8"));
await mkdir(evidence);
const require = createRequire(import.meta.url);
const { chromium } = require("/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright");
const browser = await chromium.launch({ executablePath: "/usr/bin/google-chrome", headless: true,
  args: ["--no-sandbox", "--disable-dev-shm-usage", "--no-proxy-server"] });
const errors = [];
const cdp = await browser.newBrowserCDPSession();
const { processInfo } = await cdp.send("SystemInfo.getProcessInfo");
const owned = [];
for (const process of processInfo) {
  try {
    const stat = await readFile(`/proc/${process.id}/stat`, "utf8");
    owned.push({ pid: process.id, start_ticks: stat.slice(stat.lastIndexOf(")") + 1).trim().split(/\s+/)[19], executable: await realpath(`/proc/${process.id}/exe`) });
  } catch { /* An owned short-lived child already exited. */ }
}
await writeFile(join(evidence, "owned-browser-processes.json"), JSON.stringify(owned, null, 2));
await cdp.detach();
let page;
try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  page = await context.newPage();
  page.on("pageerror", error => errors.push(error.message));
  await page.goto(input.origin);
  await page.locator(".status-label").filter({ hasText: "只读" }).waitFor();
  assert(await page.getByText(input.project_label, { exact: true }).count() > 0);
  await page.getByRole("button", { name: /^全部工作/ }).waitFor();
  await checkpoint(page, evidence, "default-page", "Short command selects current project and renders its fixed read-only resources", input.descriptor);
  assert.deepEqual(errors, []);
  await writeFile(join(evidence, "result.json"), JSON.stringify({ result: "passed", errors, title: await page.title(), origin: input.origin }, null, 2));
} catch (error) {
  if (page) await checkpoint(page, evidence, "failed-page", "Retain the actual failed startup observation before closing", input.descriptor);
  await writeFile(join(evidence, "failure.json"), JSON.stringify({ message: error.message, errors }, null, 2));
  throw error;
} finally {
  await browser.close();
  await writeFile(join(evidence, "browser-closed.json"), JSON.stringify({ closed: true }));
}
