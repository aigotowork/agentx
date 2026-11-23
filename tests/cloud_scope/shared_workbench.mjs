/** Browser side of the isolated native Space round trip in shared_workbench.py. */
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { writeFile } from 'node:fs/promises';
import puppeteer from 'puppeteer-core';

const browser = await puppeteer.launch({executablePath: process.env.CHROME_BIN || '/usr/bin/google-chrome',
  args: ['--no-sandbox', '--no-proxy-server', '--disable-dev-shm-usage', '--ignore-certificate-errors-spki-list=' + process.env.SPACE_PIN]});
try {
  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 800 });
  page.setDefaultTimeout(45000);
  const button = async label => {
    const match = await page.waitForFunction(value => [...document.querySelectorAll('button')].find(node =>
      !node.disabled && node.getClientRects().length && (node.getAttribute('aria-label') ?? node.textContent.trim()) === value), {}, label);
    try { await match.asElement().click(); } finally { await match.dispose(); }
  };
  await page.goto(process.env.SPACE_ENTRY);
  await page.waitForSelector('.cloud-connect input');
  await page.type('.cloud-connect input', process.env.SPACE_TOKEN);
  await button('连接');
  await button('新建 Agent');
  await page.waitForSelector('dialog[open] option[value="acceptance"]');
  await page.select('[aria-label="新 Agent 岗位"]', 'acceptance');
  await page.type('[aria-label="新 Agent 名称"]', 'receiver');
  assert.equal(await page.$eval('[aria-label="新 Agent 主机"]', node => node.value), 'east');
  assert.equal(await page.$eval('[aria-label="新 Agent 工作区"]', node => node.value), 'work');
  await page.click('dialog[open] .launch-directory summary');
  const path = 'dialog[open] [aria-label="新 Agent 工作目录"]';
  await page.type(path, '../home');
  assert.equal(await page.$eval(path, node => node.getAttribute('aria-invalid')), 'true');
  assert.equal(await page.$eval('dialog[open] button[type="submit"]', node => node.disabled), true);
  await page.$eval(path, node => node.select());
  await page.keyboard.type('runs/first');
  assert.equal(await page.$eval('dialog[open] .launch-directory summary code', node => node.textContent), 'work/runs/first');
  await button('启动');
  await page.waitForFunction(() => document.querySelector('dialog[open] [role="alert"]')?.textContent.includes('工作目录不存在'));
  await page.click('dialog[open] .launch-directory-create input');
  await page.screenshot({ path: join(process.env.SPACE_EVIDENCE, 'directory-launch-desktop.png') });
  await page.setViewport({ width: 320, height: 620 });
  assert.equal(await page.$eval('dialog[open]', node => node.scrollWidth <= node.clientWidth), true);
  await page.screenshot({ path: join(process.env.SPACE_EVIDENCE, 'directory-launch-narrow.png') });
  await button('启动');
  const editor = '.workspace-reader [aria-label="Agent 消息草稿"]';
  await page.waitForSelector('dialog[open]', { hidden: true });
  await page.setViewport({ width: 1280, height: 800 });
  await page.waitForSelector('.workspace-reader [aria-label="会话详情"]', { visible: true });
  await page.click('.workspace-reader [aria-label="会话详情"]');
  await page.waitForFunction(cwd => {
    const facts = [...document.querySelectorAll('.workspace-reader .reader-facts dt')];
    return facts.some(node => node.textContent === '独立工作目录' && node.nextElementSibling?.textContent === cwd)
      && facts.some(node => node.textContent === '采用岗位' && node.nextElementSibling?.textContent.includes('acceptance'));
  }, {}, process.env.SPACE_EXPECTED_CWD);
  await page.click('.workspace-reader [aria-label="会话详情"]');
  await button('消息');
  await page.waitForSelector(editor, {visible: true});
  const selected = new URL(page.url());
  assert.equal(selected.searchParams.get('host'), 'east');
  assert.equal(selected.searchParams.get('name'), 'receiver');
  assert.ok(selected.searchParams.get('incarnation'));
  await page.type(editor, process.env.SPACE_MESSAGE);
  await button('发送 AgentX 消息');
  await button('Transcript');
  await page.waitForFunction(text => document.querySelector('.workspace-reader')?.innerText.includes(text), {}, process.env.SPACE_PROGRESS);
  await writeFile(process.env.SPACE_PROGRESS_SEEN, 'visible');
  await button('消息');
  const reply = () => page.waitForFunction((body, answer) => {
    const text = document.querySelector('.workspace-reader')?.innerText ?? '';
    return text.includes(body) && text.includes(answer);
  }, {}, process.env.SPACE_MESSAGE, process.env.SPACE_REPLY);
  await reply();
  await page.reload();
  await reply();
  assert.equal(page.url(), selected.href);
  console.log('Shared Space validated and created the selected launch directory, opened the role launch, and restored the real Codex reply on reload.');
} finally {
  await browser.close();
}
