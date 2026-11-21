/** Paired visual measurement on the same long-history fixture and viewport. */
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const [base, output] = process.argv.slice(2);
if (!base || !output) throw Error('Usage: node experience-density.mjs LONG_FIXTURE_URL EVIDENCE_DIR');
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox'] });
const results = [];

async function open(page, name) {
  await page.locator('.office-camera-controls button[aria-expanded]').click();
  await page.getByRole('region', { name: '场景名册' }).getByRole('button', { name: new RegExp('^@' + name) }).click();
  const pane = page.getByRole('region', { name: '@' + name, exact: true });
  await pane.getByText(new RegExp('^\\[' + name + ' / 1\\]')).waitFor();
  return pane;
}

async function countVisibleContent(pane) {
  return pane.evaluate(node => {
    const history = node.querySelector('.conversation-history'), bounds = history.getBoundingClientRect();
    const fullyVisible = rect => rect.top >= bounds.top && rect.bottom <= bounds.bottom && rect.width > 0;
    const headers = [...history.querySelectorAll('.message-header')].filter(node => fullyVisible(node.getBoundingClientRect())).length;
    const lines = new Set();
    for (const block of history.querySelectorAll('.markdown')) {
      const traversal = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
      for (let item = traversal.nextNode(); item; item = traversal.nextNode()) {
        const range = document.createRange();
        range.selectNodeContents(item);
        for (const rect of range.getClientRects()) if (fullyVisible(rect)) lines.add(Math.round(rect.top));
      }
    }
    const body = history.querySelector('.markdown');
    return { historyHeight: history.clientHeight, historyWidth: history.clientWidth, headers, bodyLines: lines.size,
      bodyFontSize: body ? getComputedStyle(body).fontSize : null };
  });
}

try {
  for (const [width, height, paired] of [[1280, 800, false], [1280, 800, true], [768, 1024, false], [375, 812, false]]) {
    const page = await browser.newPage({ viewport: { width, height }, hasTouch: width < 768, reducedMotion: 'reduce' });
    await page.goto(base);
    const pane = await open(page, 'luna');
    if (paired) await open(page, 'mora');
    const layout = paired ? 'two' : 'one';
    const reading = await countVisibleContent(pane);
    await page.screenshot({ path: join(output, `${width}-${layout}-reading.png`) });
    const composer = pane.getByRole('textbox', { name: 'Agent 消息草稿' });
    const start = (await composer.boundingBox()).height;
    await composer.focus();
    await page.keyboard.insertText(Array.from({ length: 15 }, (_, i) => `${i + 1}. 把证据贴到正确的会话，逐条确认变更。`).join('\n'));
    const during = await composer.evaluate(node => ({ height: node.getBoundingClientRect().height, scrollHeight: node.scrollHeight }));
    await page.screenshot({ path: join(output, `${width}-${layout}-draft.png`) });
    results.push({ width, height, layout, reading, composer: { start, during }, readingWhileEditing: await countVisibleContent(pane) });
    await page.close();
  }
} finally { await browser.close(); }
await writeFile(join(output, 'density.json'), JSON.stringify({ base, results }, null, 2) + '\n');
console.log(JSON.stringify(results));
