import assert from "node:assert/strict";
import { mkdir, readFile, readdir, unlink, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

const [root, ...scenarios] = process.argv.slice(2);
const template = await readFile(new URL("./viewer.html", import.meta.url), "utf8");
const exports = [];
for (const scenario of scenarios) {
    const dir = path.resolve(root, scenario);
    const source = await readFile(path.join(dir, "trace.json"), "utf8");
    const trace = JSON.parse(source);
    const html = path.join(dir, "index.html");
    // Keep terminal data inert inside the inline script element, even if a
    // pasted string contains a literal HTML closing tag.
    await writeFile(html, template.replace("__TRACE_JSON__", () => source.replaceAll("<", "\\u003c")));
    exports.push({dir, html, trace});
    console.log(`${scenario}: ${html}`);
}

if (process.env.AGENTX_TUI_REPLAY_NO_PNG !== "1") {
    const require = createRequire(import.meta.url);
    const explicitPackage = process.env.AGENTX_PLAYWRIGHT_PACKAGE;
    const localPackage = path.resolve(root, "../tui-replay-tools/node_modules/playwright");
    let chromium;
    for (const packagePath of explicitPackage ? [explicitPackage] : ["playwright", localPackage]) {
        try {
            ({ chromium } = require(packagePath));
            break;
        } catch (error) {
            if (error.code !== "MODULE_NOT_FOUND") throw error;
        }
    }
    if (!chromium) throw new Error("Playwright is required for PNG export. See tests/tui_replay/README.md or set AGENTX_TUI_REPLAY_NO_PNG=1.");
    const executablePath = process.env.AGENTX_BROWSER_EXECUTABLE || undefined;
    const browser = await chromium.launch({headless: true, executablePath}).catch(async (error) => {
        if (executablePath || !error.message.includes("Executable doesn't exist")) throw error;
        try {
            return await chromium.launch({headless: true, channel: "chrome"});
        } catch (chromeError) {
            if (!chromeError.message.includes("not found")) throw chromeError;
            throw error;
        }
    });
    try {
        for (const artifact of exports) await capture(browser, artifact);
    } finally {
        await browser.close();
    }
}

async function capture(browser, {dir, html, trace}) {
    const png = path.join(dir, "png");
    await mkdir(png, {recursive: true});

    const page = await browser.newPage({viewport: {width: 1320, height: 850}, offline: true});
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error));
    try {
        await page.goto(pathToFileURL(html).href);
        const next = page.getByRole("button", {name: "Next", exact: true});
        const before = page.getByRole("button", {name: "Before input", exact: true});
        const after = page.getByRole("button", {name: "After input", exact: true});
        const title = page.locator("#step-title");
        const written = new Set();
        const shot = async (index, phase) => {
            const name = `${String(index).padStart(2, "0")}-${phase}.png`;
            await page.locator(".stage").screenshot({path: path.join(png, name)});
            written.add(name);
        };
        for (const [index, step] of trace.steps.entries()) {
            assert.ok((await title.textContent()).endsWith(step.label));
            assert.equal(await after.getAttribute("aria-pressed"), "true");
            assert.equal(await page.locator(".terminal-row").count(), step.frame.lines);
            assert.equal(await page.locator(".text-cursor").count(), Number(Boolean(step.frame.cursor)));
            await shot(index, "after");
            if (step.pointer) {
                await before.click();
                assert.equal(await before.getAttribute("aria-pressed"), "true");
                const pointer = page.locator(".pointer");
                const origin = trace.steps[index - 1].frame.origin ?? [0, 0];
                const overlay = await pointer.evaluate((node) => ["--x", "--y"].map((key) => Number(node.style.getPropertyValue(key))));
                assert.deepEqual(overlay, step.pointer.map((value, axis) => value - origin[axis]));
                assert.equal(await page.locator(".hit-region").count(), Number(Boolean(step.hit)));
                await shot(index, "before");
                await after.click();
            }
            if (index + 1 < trace.steps.length) await next.click();
        }
        assert.equal(await next.isDisabled(), true);

        // Verify narrow layout keeps the full terminal grid available in its
        // own scroller while step selection and keyboard navigation work.
        await page.setViewportSize({width: 420, height: 780});
        await page.getByRole("button", {name: /Initial render/}).click();
        await page.keyboard.press("ArrowRight");
        assert.ok((await title.textContent()).endsWith(trace.steps[1].label));
        assert.equal(await page.locator(".terminal-scroll").evaluate((node) => node.scrollWidth > node.clientWidth), true);
        assert.deepEqual(pageErrors, []);

        for (const name of await readdir(png)) {
            if (/^\d+-(before|after)\.png$/.test(name) && !written.has(name)) await unlink(path.join(png, name));
        }
        console.log(`${trace.scenario}: ${written.size} PNGs in ${png}`);
    } finally {
        await page.close();
    }
}
