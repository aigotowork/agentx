import assert from 'node:assert/strict';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';

const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
await mkdir(config.evidence, {recursive: false});
const driverPath = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
const {chromium} = createRequire(import.meta.url)(driverPath);
const browser = await chromium.launch({executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',
  headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server']});
const context = await browser.newContext({viewport: {width: 1440, height: 900}, locale: 'zh-CN'});
const page = await context.newPage();
const traffic = [], errors = [];
page.on('console', message => { if (message.type() === 'error') errors.push({kind: 'console', message: message.text()}); });
page.on('pageerror', error => errors.push({kind: 'pageerror', message: error.message}));
page.on('requestfailed', request => traffic.push({kind: 'failed', method: request.method(), url: request.url(), error: request.failure()?.errorText}));
page.on('request', request => { if (new URL(request.url()).pathname.startsWith('/api/')) traffic.push({kind: 'request', method: request.method(), url: request.url(), body: request.postData()}); });
page.on('response', response => { if (new URL(response.url()).pathname.startsWith('/api/')) traffic.push({kind: 'response', method: response.request().method(), url: response.url(), status: response.status()}); });
let result;
try {
  await page.goto(`${config.url}/space/`, {waitUntil: 'domcontentloaded'});
  await page.getByRole('button', {name: '聊天', exact: true}).click();
  await page.getByRole('combobox', {name: '工作区范围', exact: true}).selectOption('');
  await page.getByRole('navigation', {name: '办公室内容'}).getByRole('button', {name: /^成员/}).click();
  await page.getByRole('region', {name: 'Agent 名册'}).getByRole('button', {name: new RegExp(`^@${config.agent.name} ·`)}).click();
  const composer = page.getByRole('textbox', {name: 'Agent 消息草稿', exact: true});
  await composer.fill('isolated send dispatch diagnostic');
  await composer.press('Control+Enter');
  await page.waitForTimeout(1800);
  result = {url: page.url(), composer: await composer.inputValue().catch(error => `unavailable: ${error.message}`),
    bodyText: (await page.locator('body').innerText()).slice(-12000),
    journal: await page.evaluate(() => Object.fromEntries(Object.keys(localStorage).filter(key => key.includes('agentx')).map(key => [key, localStorage.getItem(key)]))),
    actionButtons: await page.getByRole('button').evaluateAll(items => items.map(item => ({text: item.innerText, disabled: item.disabled, title: item.title, aria: item.getAttribute('aria-label')})).filter(item => /发送|消息/.test(item.text + item.title + item.aria))),
    traffic, errors};
  await page.screenshot({path: `${config.evidence}/viewport.png`, fullPage: true});
  await writeFile(`${config.evidence}/diagnostic.json`, JSON.stringify(result, null, 2) + '\n');
  assert(result, 'diagnostic result was captured');
} finally {
  await context.close();
  await browser.close();
}
