// DESIGN PROJECTION: a future consumer supplies its installed Playwright Test.
import { defineConfig } from '@playwright/test';
import { resolve } from 'node:path';

const target = process.env.PX_EXAMPLE_ORIGIN;
const runId = process.env.PX_EXAMPLE_RUN_ID;
if (process.env.PX_EXAMPLE_OWNED !== 'yes' || !target || !runId || !process.env.PX_EXAMPLE_BUILD) {
  throw new Error('Select an owned disposable host, run ID and exact build; see README.');
}
const url = new URL(target);
if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || !url.port
    || url.username || url.password || url.search || url.hash || url.pathname !== '/') {
  throw new Error('Example requires an explicit loopback HTTP origin and port.');
}
if (!/^[a-zA-Z0-9_-]+$/.test(runId)) throw new Error('Use a path-safe unique run ID.');
const out = process.env.PX_EXAMPLE_OUTPUT;
if (!out) throw new Error('Choose a fresh owned output directory for this run.');
export default defineConfig({
  testDir: '.', testMatch: 'search.spec.mjs', workers: 1, retries: 0,
  timeout: 15000, expect: { timeout: 4000 },
  outputDir: resolve(out, 'artifacts'),
  reporter: [
    ['html', { outputFolder: resolve(out, 'html'), open: 'never' }],
    ['json', { outputFile: resolve(out, 'results.json') }]
  ],
  use: {
    baseURL: url.origin, browserName: 'chromium', viewport: { width: 1280, height: 800 },
    locale: 'zh-CN', serviceWorkers: 'block', trace: 'on', screenshot: 'only-on-failure'
  }
});
