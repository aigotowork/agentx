/** Read-only selected-service adoption smoke; invoke only after recorded independent acceptance and rollout. */
const { chromium } = require('/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
(async () => {
  const output = process.argv[2];
  if (!output || fs.existsSync(output)) throw new Error('Provide one fresh evidence directory');
  fs.mkdirSync(output, { recursive: true });
  const report = { base: 'http://192.168.2.178:38023', mutations: [], errors: [], assets: {}, responses: [] };
  const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox', '--no-proxy-server'] });
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN', reducedMotion: 'reduce' });
    await page.route('**/*', async route => {
      const request = route.request();
      if (!['GET', 'HEAD', 'OPTIONS'].includes(request.method())) { report.mutations.push({ method: request.method(), url: request.url() }); return route.abort(); }
      return route.continue();
    });
    page.on('pageerror', error => report.errors.push(String(error)));
    page.on('response', response => { if (response.url().includes('/api/v1/')) report.responses.push({ url: response.url(), status: response.status() }); });
    await page.goto(report.base + '/space/');
    await page.waitForSelector('.office-actor', { state: 'attached' });
    await page.waitForFunction(() => document.querySelectorAll('.office-human').length === 1 && document.querySelectorAll('.office-actor-label').length > 0);
    report.descriptor = await page.evaluate(async () => (await fetch('/api/v1/management')).json());
    report.agents = await page.evaluate(async generation => (await fetch('/api/v1/agents?generation=' + encodeURIComponent(generation) + '&limit=100')).json(), report.descriptor.generation);
    report.activity = await page.evaluate(async generation => (await fetch('/api/v1/office-activity?generation=' + encodeURIComponent(generation))).json(), report.descriptor.generation);
    report.rendered = await page.evaluate(() => ({ title: document.title, actorCount: document.querySelectorAll('.office-actor').length, catCount: document.querySelectorAll('.office-human').length, crownCount: document.querySelectorAll('.office-controller-crown').length, statusRows: document.querySelectorAll('.office-actor small').length, labels: [...document.querySelectorAll('.office-actor-name')].slice(0, 6).map(label => ({ text: label.textContent, font: getComputedStyle(label).fontSize, background: getComputedStyle(label).backgroundColor })), rooms: [...document.querySelectorAll('.hall-room-name')].map(label => label.textContent) }));
    const urls = await page.evaluate(() => [...document.querySelectorAll('script[src],link[rel=stylesheet]')].map(node => node.src || node.href));
    for (const url of urls) { const response = await page.request.get(url); if (!response.ok()) throw new Error('Asset failed: ' + url); report.assets[url] = crypto.createHash('sha256').update(await response.body()).digest('hex'); }
    await page.screenshot({ path: path.join(output, 'space-desktop.png') });
    if (report.descriptor.generation !== 'a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb') throw new Error('Selected generation changed');
    if (!report.agents.agents.every(agent => Object.hasOwn(agent, 'title') && Object.hasOwn(agent, 'pending_messages') && Object.hasOwn(agent, 'status_context'))) throw new Error('Native office contract missing');
    if (report.rendered.catCount !== 1 || report.rendered.statusRows !== 0 || !report.rendered.actorCount || !Array.isArray(report.activity.human_messages)) throw new Error('Rendered office contract missing');
    if (report.mutations.length || report.errors.length || report.responses.some(response => response.status >= 400)) throw new Error('Read-only smoke reported errors or attempted mutation');
    report.status = 'passed';
  } catch (error) { report.status = 'failed'; report.failure = String(error.stack || error); throw error; }
  finally { fs.writeFileSync(path.join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n'); await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
