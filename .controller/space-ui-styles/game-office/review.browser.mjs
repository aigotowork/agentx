/** Capture the published game office for visual review; the independent runner owns acceptance. */
import { mkdir } from "node:fs/promises";
import assert from "node:assert/strict";
import { join } from "node:path";
import puppeteer from "puppeteer-core";

const [base, output] = process.argv.slice(2);
assert(base && output, "Pass the product origin and a new output directory");
await mkdir(output);
const browser = await puppeteer.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? "/usr/bin/google-chrome", headless: true, args: ["--no-sandbox", "--disable-dev-shm-usage"] });
const worlds = ["inkpost", "windward", "jadecourt"];
const avatar = 'button.office-actor[aria-label^="@mora "]', lamp = ".game-scene-lantern";
const draft = 'textarea[aria-label="Agent 消息草稿"]';
function address(style) {
  const url = new URL(base);
  url.search = new URLSearchParams({ art: "studio", style, office: "/studio/catalog" });
  return url.href;
}
try {
  const page = await browser.newPage();
  await page.setViewport({ width: 1440, height: 900, deviceScaleFactor: 1 });
  const reduced = value => page.emulateMediaFeatures([{ name: "prefers-reduced-motion", value }]);
  await reduced("reduce");
  const nextPaint = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  let clip;
  for (const world of worlds) {
    await page.goto(address(world), { waitUntil: "domcontentloaded" });
    await page.waitForSelector(`[data-scene-world="${world}"][data-art-status="ready"] ${avatar}`, { visible: true, timeout: 15000 });
    await nextPaint();
    await page.screenshot({ path: join(output, world + "-office.png") });
    const light = await page.$(lamp);
    const { x, y } = await light.boundingBox();
    if (world === "inkpost") clip = { x: Math.floor(x - 27), y: Math.floor(y - 45), width: 460, height: 265 };
    await page.focus(lamp);
    await page.keyboard.press("Space");
    await page.waitForSelector(lamp + '[aria-pressed="false"]');
    await nextPaint();
    await page.screenshot({ path: join(output, world + "-lamp-off.png") });
    await page.keyboard.press("Space");
    await page.waitForSelector(lamp + '[aria-pressed="true"]');
    await page.click(avatar);
    await page.waitForFunction(() => document.body.innerText.includes("版片编号是 R-14"));
    await page.type(draft, "R‑14 · 下轮校样");
    await page.screenshot({ path: join(output, world + "-reader.png") });
  }
  await reduced("no-preference");
  await page.goto(address("inkpost"), { waitUntil: "domcontentloaded" });
  await page.waitForSelector('[data-scene-world="inkpost"] ' + lamp);
  await page.focus(lamp);
  for (let index = 0; index < 26; index++) {
    if (index === 8 || index === 17) await page.keyboard.press("Space");
    await nextPaint();
    await page.screenshot({ path: join(output, "frame-" + String(index).padStart(2, "0") + ".png"), clip });
  }
} finally {
  await browser.close();
}
