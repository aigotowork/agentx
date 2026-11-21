/** Render frozen SVG inputs for static design discussion; never contact product services.
 * Usage: node render-design.mjs OUTPUT_DIRECTORY SVG...
 * Uses the existing local Playwright/Chrome installation; environment overrides are optional.
 */
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, writeFile, access } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';

const [destination, ...inputs] = process.argv.slice(2);
if (!destination || !inputs.length) throw new Error('Supply OUTPUT_DIRECTORY and SVG paths');
const output = path.resolve(destination);
await mkdir(output, { recursive: true });
const receipt = path.join(output, 'render.json');
const previous = await readFile(receipt, 'utf8').then(JSON.parse).catch(error => {
  if (error.code === 'ENOENT') return { rows: [] };
  throw error;
});
const snapshot = await mkdtemp(path.join(os.tmpdir(), 'agentx-svg-inputs-'));
const rows = [];
let browser;
try {
  for (const input of inputs) {
    const source = path.resolve(input);
    const content = await readFile(source);
    const sha256 = createHash('sha256').update(content).digest('hex');
    const name = path.basename(source, '.svg');
    const png = path.join(output, name + '.png');
    const frozen = path.join(snapshot, name + '.svg');
    await writeFile(frozen, content);
    const old = previous.rows.find(row => row.source === source && row.sha256 === sha256);
    const available = await access(png).then(() => true, () => false);
    if (old && available) {
      rows.push({ ...old, frozen, render: 'unchanged capture reused' });
      continue;
    }
    if (!browser) {
      const require = createRequire(import.meta.url);
      const driver = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
      const { chromium } = require(driver);
      browser = await chromium.launch({ executablePath: process.env.AGENTX_CHROME ?? '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-background-networking'] });
    }
    const dimensions = content.toString().match(/viewBox="[\d.]+ [\d.]+ ([\d.]+) ([\d.]+)"/);
    if (!dimensions) throw new Error('Expected finite SVG viewBox: ' + source);
    const viewport = { width: Number(dimensions[1]), height: Number(dimensions[2]) };
    const context = await browser.newContext({ viewport, deviceScaleFactor: 1 });
    try {
      const page = await context.newPage();
      await page.setContent('<!doctype html><meta charset="utf-8"><style>html,body{margin:0;padding:0;overflow:hidden}svg{display:block}</style>' + content.toString());
      await page.screenshot({ path: png, timeout: 12000 });
      rows.push({ source, frozen, sha256, png, viewport, render: 'changed input captured', svg: await page.locator('svg').boundingBox() });
    } finally {
      await context.close();
    }
  }
} finally {
  await browser?.close();
}
const result = { purpose: 'static SVG design discussion; illustrative data, no product acceptance', browserClosed: true, snapshot, rows };
await writeFile(receipt, JSON.stringify(result, null, 2) + '\n');
process.stdout.write(JSON.stringify(result) + '\n');
