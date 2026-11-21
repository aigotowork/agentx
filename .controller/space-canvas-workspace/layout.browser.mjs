// Exercise the actual Space App in Studio demo mode and compare shell-only CSS on the same paused scene.
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { createRequire } from "node:module";
import { join } from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? "playwright");
const base = process.env.AGENT_SPACE_LAYOUT_URL ?? "http://127.0.0.1:19147/";
const output = process.argv[2];
assert(output, "Pass an output directory for the three comparison screenshots");
await mkdir(output, { recursive: true });

const directions = {
  "field-desk": `html[data-pane-study] .space-window-region[data-overlay] {
    --window-trim: #efe6d1; --window-rule: #c5b99c; --window-edge: #8d7a5f;
    --window-selected: #dfdfc6; --window-overlay-shadow: 0 3px 0 #47341b21, 0 9px 24px #47341b30;
  } html[data-pane-study] .space-window-actions { background: var(--window-trim); border-radius: var(--radius-sm); }`,
  "reading-room": `html[data-pane-study] .space-window-region[data-overlay] {
    --window-trim: #f6f8f8; --window-rule: #dbe3e3; --window-edge: #c3cece;
    --window-selected: #e4edec; --window-overlay-shadow: 0 2px 8px #20343914;
  }`,
  shared: "",
};
const recordContinuation = "\n\n校样对比以实物为准：先读边缘的细线，再核对字样与出处。两张样张都保留原纸和完整编号，方便下一位同事逐项复查。\n\n- 第一轮：把可确认的差异写入对照表，有疑问的标作待查。\n- 第二轮：按纸纹、线宽、干燥时间核对同一位置。\n- 交接：将确认页和待查页分开放置，下次只补未关闭的条目。\n\n图录的页码暂时保持原样。等封面比对结束后，再一起检查正文里的交叉引用。";

const browser = await chromium.launch({ headless: true, executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? "/usr/bin/google-chrome", args: ["--no-proxy-server"] });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: "reduce" });
  const failures = [];
  page.on("pageerror", error => failures.push(error.message));
  await page.route(/\/src\/offline\/studio\.ts(?:\?|$)/, async route => {
    const response = await route.fetch();
    const source = await response.text();
    // Extend only the two Studio sample replies used for the within-frame comparison.
    let extended = 0;
    const body = source.replace(/reply: "([^"\n]+)"/g, (original, reply) => {
      if (++extended > 2) return original;
      return `reply: ${JSON.stringify(reply + recordContinuation.repeat(2))}`;
    });
    assert(extended >= 2, "Studio fixture reply format changed");
    await route.fulfill({ response, body });
  });

  async function visit(style = "classic") {
    const url = new URL(base);
    url.search = new URLSearchParams({ supply: "studio", office: "/studio/catalog", style }).toString();
    await page.goto(url.toString());
    await page.locator(".space-scene-stage canvas").waitFor();
  }
  const panes = page.locator(".space-window-pane");
  const active = page.locator('.space-window-pane[data-active="true"]');
  const shelf = page.getByRole("navigation", { name: "已打开的会话" });
  const camera = page.getByRole("navigation", { name: "办公室镜头" });
  async function openFromScene(name) {
    await camera.getByRole("button", { name: "名册", exact: true }).click();
    await page.getByRole("region", { name: "场景名册" }).getByRole("button", { name: new RegExp(`^@${name} ·`) }).click();
    await active.getByRole("button", { name: new RegExp(`会话：@${name}$`) }).first().waitFor();
  }
  const action = name => active.locator(`[data-window-action="${name}"]`);

  await visit();
  await page.evaluate(() => {
    window.paneStudyCues = [];
    const capture = node => {
      if (!(node instanceof HTMLElement) || !node.dataset.windowMotion) return;
      const controls = node.querySelector("[data-window-motion-header]");
      window.paneStudyCues.push({ event: node.dataset.windowMotion, id: node.dataset.paneId ?? node.dataset.shelfId, animation: getComputedStyle(controls ?? node).animationName });
    };
    new MutationObserver(records => {
      for (const mutation of records) {
        if (mutation.type === "attributes") capture(mutation.target);
        for (const node of mutation.addedNodes) {
          capture(node);
          if (node instanceof HTMLElement) node.querySelectorAll("[data-window-motion]").forEach(capture);
        }
      }
    }).observe(document, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-window-motion"] });
  });
  const sawCue = (event, id) => page.evaluate(({ event, id }) => window.paneStudyCues.some(cue => cue.event === event && cue.id === id && cue.animation === "none"), { event, id });
  await camera.getByRole("button", { name: "暂停动作" }).click();
  await openFromScene("mora");
  await openFromScene("luna");
  assert.equal(await panes.count(), 2);
  assert.equal(await panes.locator(".workspace-reader-channelbar [data-window-motion-header]").count(), 2);
  assert.equal(await panes.locator(".space-window-header").count(), 0);
  assert.equal(await panes.locator(".history-standard-record").filter({ hasText: "校样对比以实物为准" }).count(), 2);
  const luna = await active.getAttribute("data-pane-id");
  assert.equal(await sawCue("open", luna), true);
  const bounds = await panes.evaluateAll(list => list.map(element => element.getBoundingClientRect().toJSON()));
  assert(bounds[0].right < bounds[1].left && bounds[0].y === bounds[1].y, "wide panes share one row");
  await page.mouse.move(0, 0);
  for (const [direction, css] of Object.entries(directions)) {
    await page.locator("html").evaluate(element => { element.dataset.paneStudy = "true"; });
    const layer = css && await page.addStyleTag({ content: css });
    await page.locator(".space-scene-stage").screenshot({ path: join(output, `${direction}.jpg`), quality: 90 });
    if (layer) await layer.evaluate(element => element.remove());
  }

  const wideIds = await panes.evaluateAll(list => list.map(element => element.dataset.paneId));
  await page.getByRole("region", { name: "@mora", exact: true }).getByRole("textbox").click();
  assert.equal(await active.getAttribute("aria-label"), "@mora");
  await page.getByRole("region", { name: "@luna", exact: true }).getByRole("textbox").focus();
  assert.equal(await active.getAttribute("aria-label"), "@luna");
  assert.deepEqual(await panes.evaluateAll(list => list.map(element => element.dataset.paneId)), wideIds);

  await action("focus").press("Enter");
  assert.equal(await panes.count(), 1);
  await shelf.getByRole("button", { name: "@mora", exact: true }).press("Enter");
  assert.equal(await active.getAttribute("data-focused"), "true");
  await active.getByRole("button", { name: "恢复并排：@mora" }).press("Enter");
  assert.equal(await panes.count(), 2);
  const mora = await active.getAttribute("data-pane-id");
  await action("minimize").press("Enter");
  const minimized = shelf.getByRole("button", { name: "@mora（已最小化）" });
  assert.equal(await sawCue("minimize", mora), true);
  assert.equal(await minimized.evaluate(element => element === document.activeElement), true);
  await minimized.press("Enter");
  assert.equal(await sawCue("restore", mora), true);
  await action("close").press("Enter");
  assert.equal(await active.getAttribute("data-pane-id"), luna);
  assert.equal(await shelf.getByRole("button", { name: "@luna" }).evaluate(element => element === document.activeElement), true);
  await openFromScene("riku");
  await openFromScene("mora");
  assert.equal(await shelf.getByRole("button").count(), 3);
  assert.equal(await panes.count(), 2);

  await active.locator(".conversation-history").evaluate(element => { window.paneStudyHistory = element; element.scrollTop = 20; });
  await page.setViewportSize({ width: 390, height: 780 });
  await panes.nth(1).waitFor({ state: "detached" });
  assert.equal(await panes.count(), 1);
  assert.equal(await active.locator(".conversation-history").evaluate(element => window.paneStudyHistory === element && element.scrollTop > 0), true);
  await camera.getByRole("button", { name: "名册", exact: true }).click();
  await page.getByRole("button", { name: "关闭名册", exact: true }).click();
  const emulation = await page.context().newCDPSession(page);
  await emulation.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
  await page.setViewportSize({ width: 320, height: 780 });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), 320);
  assert.equal(await page.evaluate(() => matchMedia("(any-pointer: coarse)").matches), true);
  assert(await action("close").evaluate(element => {
    const bounds = element.getBoundingClientRect();
    return bounds.width >= 44 && bounds.height >= 44 && bounds.right <= innerWidth;
  }), "close remains a 44px target inside the 320px viewport");
  await camera.getByRole("button", { name: "名册", exact: true }).click();
  await page.getByRole("button", { name: "关闭名册", exact: true }).click();
  await page.getByRole("button", { name: "聊天", exact: true }).click();
  assert.equal(await page.locator('.space-chat-reader .space-window-region[data-overlay="false"]').count(), 1);
  assert.equal(await panes.count(), 1);
  await page.setViewportSize({ width: 1440, height: 900 });
  await panes.nth(1).waitFor();
  assert.equal(await page.getByRole("complementary", { name: "会话名单" }).isVisible(), true);

  await visit("signal");
  await openFromScene("mora");
  assert.notEqual(await active.evaluate(element => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)");
  assert.deepEqual(failures, []);
  console.log("Space shell: same-scene visuals, tiled/width continuity, focus, shelf, scene controls, chat and Signal passed");
} finally {
  await browser.close();
}
