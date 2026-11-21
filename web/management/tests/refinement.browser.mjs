/** Operate the polished dialogs and conversation under desktop, small mobile and reduced motion. */
import assert from 'node:assert/strict';
import puppeteer from 'puppeteer-core';
import { createManagementFixture } from './browser.fixture.mjs';

const [output, capture = output] = process.argv.slice(2);
if (!output) throw new Error('Pass the built Management directory and optional screenshot directory');
const fixture = createManagementFixture(output, { preview: true });
await new Promise(resolve => fixture.server.listen(0, '127.0.0.1', resolve));
const browser = await puppeteer.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox','--disable-dev-shm-usage','--no-proxy-server'] });
const page = await browser.newPage();
const errors = [];
page.on('pageerror', error => errors.push(error.message));
let stage = 'launch keyboard and motion';
const screenshot = name => page.screenshot({ path: `${capture}/${name}.png` });
const focusIs = selector => page.waitForFunction(selector => document.activeElement?.matches(selector), {}, selector);
const close = (selector) => page.waitForSelector(`${selector}[open]`, { hidden: true });
const url = `http://127.0.0.1:${fixture.server.address().port}/`;
const exactId = 'message-18446744073709551605';
try {
  await page.setViewport({ width: 1440, height: 900 });
  await page.goto(url);
  await page.waitForSelector('.header-actions button:not(:disabled)');
  await page.$eval('.launch-dialog', node => {
    globalThis.modalOpacities = [];
    node.addEventListener('transitionend', event => {
      if (event.target === node && event.propertyName === 'opacity' && !event.pseudoElement) globalThis.modalOpacities.push(node.open);
    });
  });
  await page.focus('.header-actions button'); await page.keyboard.press('Enter');
  await page.waitForSelector('.launch-dialog[open]');
  await focusIs('.launch-dialog h2');
  await page.waitForFunction(() => globalThis.modalOpacities.includes(true));
  if (await page.$('.launch-settings-toggle')) {
    await page.waitForFunction(() => !document.querySelector('.launch-settings-toggle').getAttribute('aria-expanded').includes('true'));
    await page.waitForSelector('[data-launch-workspace]', { hidden: true });
  }
  await screenshot('launch-desktop');

  stage = 'modal validation, backdrop, focus and draft';
  await page.click('.launch-modes button:nth-child(2)');
  await page.click('.launch-dialog button.primary');
  await page.waitForSelector('[data-launch-source][aria-invalid=true]');
  await focusIs('[data-launch-source]');
  assert.equal(fixture.posts.length, 0);
  await page.type('[data-launch-source]', 'reviewer');
  await page.click('.launch-settings-toggle');
  await page.type('[data-launch-workspace]', 'relative');
  await page.click('.launch-settings-toggle');
  assert.match(await page.$eval('.setting-value', node => node.textContent), /relative/);
  await page.click('.launch-dialog button.primary');
  await page.waitForSelector('[data-launch-workspace][aria-invalid=true]');
  await focusIs('[data-launch-workspace]');
  await page.mouse.click(8, 8);
  await close('.launch-dialog');
  await focusIs('.header-actions button');
  await page.waitForFunction(() => globalThis.modalOpacities.includes(false));
  await page.keyboard.press('Enter');
  await page.waitForSelector('.launch-dialog[open]');
  assert.equal(await page.$eval('[data-launch-source]', node => node.value), 'reviewer');
  assert.equal(await page.$eval('[data-launch-workspace]', node => node.value), 'relative');
  await page.keyboard.press('Escape');
  await close('.launch-dialog');
  assert.equal(fixture.posts.length, 0);

  stage = 'exact stop decision';
  await page.click('.agent-row');
  await page.waitForSelector('.message-row');
  await page.waitForSelector('.lifecycle-actions button:not(:disabled)');
  await page.click('.lifecycle-actions button');
  await page.waitForSelector('.stop-dialog[open]');
  await focusIs('.stop-dialog [data-dialog-focus]');
  assert.match(await page.$eval('.stop-decision', node => node.textContent), /18446744073709551601/);
  await page.waitForFunction(() => getComputedStyle(document.querySelector('.stop-dialog')).opacity === '1');
  await screenshot('stop-desktop');
  await page.keyboard.press('Escape');
  await close('.stop-dialog');
  await focusIs('.lifecycle-actions button');
  assert.equal(fixture.posts.length, 0);

  stage = 'quoted reply';
  await page.click('.message-row .message-quote');
  await focusIs(`#${exactId}`);
  await page.click(`#${exactId} .reply-action`);
  await focusIs('#agent-message');
  await page.waitForFunction(() => document.querySelector('.reply-preview')?.textContent.includes('请核对本次管理台交付'));
  await page.type('#agent-message', '回复内容会独立保留。');
  await screenshot('reply-desktop');
  await page.click('[aria-label="移除引用"]');
  await page.waitForSelector('.reply-preview', { hidden: true });
  assert.equal(await page.$eval('#agent-message', node => node.value), '回复内容会独立保留。');

  stage = 'mobile dialog and reduced motion';
  await page.setViewport({ width: 390, height: 675, isMobile: true, hasTouch: true });
  await page.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: 'dark' }, { name: 'prefers-reduced-motion', value: 'reduce' }]);
  await page.reload();
  await page.waitForSelector('.header-actions button:not(:disabled)');
  await page.click('.header-actions button');
  await page.waitForSelector('.launch-dialog[open]');
  const dimensions = await page.$eval('.launch-dialog', dialog => {
    const buttons = dialog.querySelector('.dialog-actions').getBoundingClientRect();
    const body = dialog.querySelector('.dialog-body');
    return { bottom: buttons.bottom, height: innerHeight, body: body.clientHeight, scroll: body.scrollHeight, duration: getComputedStyle(dialog).transitionDuration, overflow: document.documentElement.scrollWidth > innerWidth };
  });
  assert(dimensions.bottom <= dimensions.height && dimensions.scroll <= dimensions.body + 1);
  assert.equal(dimensions.overflow, false);
  assert.equal(dimensions.duration, '0s');
  await screenshot('launch-mobile-dark');
  await page.click('.launch-settings-toggle');
  await page.$eval('[data-launch-workspace]', node => { node.focus(); node.select(); });
  await page.keyboard.type('/workspace/office');
  assert.equal(await page.$eval('[data-launch-workspace]', node => node.value), '/workspace/office');
  await page.keyboard.press('Escape');

  stage = 'new messages while reading older history';
  const message = i => ({ id: String(90000 + i), at: '2026-11-15T11:00:00Z', from: 'fava', text: `跟进记录 ${i}：` + '可从完整回执核对原始信息，阅读历史时保留滚动位置。'.repeat(3), recipients: ['bigboss'], intent: 'inform', thread: '交付跟进', reply_to: null });
  fixture.state.addedMessages.push(...Array.from({ length: 12 }, (_, i) => message(i)));
  await page.waitForFunction(() => document.querySelectorAll('.message-row').length === 15);
  await page.$eval('.conversation-scroll', node => { node.scrollTop = 0; });
  await page.waitForSelector('.latest-message:not([disabled])');
  fixture.state.addedMessages.push(message(12));
  await page.waitForFunction(() => document.querySelector('.latest-message')?.textContent.includes('1 条新消息'));
  assert(await page.$eval('.conversation-scroll', node => node.scrollTop < 16), 'Incoming rows must not move a reader away from older messages');
  await page.click('.latest-message');
  await page.waitForSelector('.latest-message', { hidden: true });
  assert(await page.$eval('.conversation-scroll', node => node.scrollHeight - node.scrollTop - node.clientHeight < 50));
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await screenshot('conversation-mobile-dark');
  await page.click('.composer-options');
  const sendReachable = await page.$eval('.composer .send-button', button => {
    const bounds = button.getBoundingClientRect();
    return bounds.bottom <= innerHeight && document.elementFromPoint(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2)?.closest('button') === button;
  });
  assert.equal(sendReachable, true, 'Composer controls stay reachable on a short phone with options open');
  assert.deepEqual(errors, []);
  console.log('PASS refinement: real modal enter/exit, safe focus and exit paths, draft/validation, exact stop target, reply quote and focus, mobile scroll and reduced motion, new-message anchor');
} catch (error) {
  console.error(`Refinement failed at ${stage}`);
  if (!page.isClosed()) await screenshot('refinement-failure').catch(() => {});
  throw error;
} finally {
  await browser.close();
  await new Promise(resolve => fixture.server.close(resolve));
}
