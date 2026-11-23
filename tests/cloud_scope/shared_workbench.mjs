/** Browser side of the isolated native Space round trip in shared_workbench.py. */
import assert from 'node:assert/strict';
import puppeteer from 'puppeteer-core';

const browser = await puppeteer.launch({executablePath: process.env.CHROME_BIN || '/usr/bin/google-chrome',
  args: ['--no-sandbox', '--no-proxy-server', '--disable-dev-shm-usage', '--ignore-certificate-errors-spki-list=' + process.env.SPACE_PIN]});
try {
  const page = await browser.newPage();
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
  await button('启动');
  const editor = '.workspace-reader [aria-label="Agent 消息草稿"]';
  await page.waitForSelector(editor, {visible: true});
  const selected = new URL(page.url());
  assert.equal(selected.searchParams.get('host'), 'east');
  assert.equal(selected.searchParams.get('name'), 'receiver');
  assert.ok(selected.searchParams.get('incarnation'));
  await page.type(editor, process.env.SPACE_MESSAGE);
  await button('发送 AgentX 消息');
  const reply = () => page.waitForFunction((body, answer) => {
    const text = document.querySelector('.workspace-reader')?.innerText ?? '';
    return text.includes(body) && text.includes(answer);
  }, {}, process.env.SPACE_MESSAGE, process.env.SPACE_REPLY);
  await reply();
  await page.reload();
  await reply();
  assert.equal(page.url(), selected.href);
  console.log('Shared Space opened the role launch, sent an operator message, received a Codex reply and restored the conversation on reload.');
} finally {
  await browser.close();
}
