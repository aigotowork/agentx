#!/usr/bin/env node
// Independent operator replay. Run --help for cases and evidence boundaries.
import { createHash } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { setTimeout as pause } from 'node:timers/promises';
import { parseArgs } from 'node:util';
import puppeteer from 'puppeteer-core';

const { values } = parseArgs({ options: {
  url: { type: 'string' }, cases: { type: 'string', default: 'density,current,boundary,draft,resize,compact,receipt,short' },
  out: { type: 'string', default: '.artifacts/space-canvas-workspace/independent' },
  chrome: { type: 'string', default: process.env.CHROME_BIN || '/usr/bin/google-chrome' },
  help: { type: 'boolean' },
} });
if (values.help || !values.url) {
  console.log('node .controller/space-canvas-workspace/conversation-evaluation.browser.mjs --url http://127.0.0.1:18985/space/ [--cases density,current,boundary,draft,resize,compact,receipt,short] [--out PATH] [--chrome PATH]');
  console.log('Use only the assigned synthetic build. All browser writes are blocked; receipt tests transport ambiguity, not durability. A failed experience check appears in results.json; a replay error exits 1.');
  process.exit(values.help ? 0 : 2);
}
const url = new URL(values.url);
const out = resolve(values.out);
const digest = (body) => createHash('sha256').update(body).digest('hex');
const viewports = {
  desktop: { width: 1280, height: 800 },
  comparison: { width: 1440, height: 900 },
  tablet: { width: 768, height: 1024 },
  mobile: { width: 375, height: 812, isMobile: true, hasTouch: true },
};
const pane = (name) => `section[aria-label="@${name}"]`;
const reader = (name) => `${pane(name)} .conversation-history`;
const editor = (name) => `${pane(name)} textarea[aria-label="Agent 消息草稿"]`;
const send = (name) => `${pane(name)} button[aria-label="发送 AgentX 消息"]`;
const subject = (name) => `button[aria-label^="@${name} ·"]`;
const windowAction = (name, verb) => `button[aria-label="${verb}会话：@${name}"]`;
const tab = (name, label) => `${pane(name)} button ::-p-text(${label})`;
const shelf = (name) => `button.space-window-shelf-button[title="@${name}"]`;
const routeInfo = (request) => {
  const u = new URL(request.url());
  const agent = u.pathname.match(/\/api\/v1\/agents\/([^/]+)\/messages$/)?.[1];
  return { method: request.method(), path: u.pathname, agent, after: u.searchParams.get('after'), incarnation: u.searchParams.get('incarnation'), generation: u.searchParams.get('generation') };
};
const draftText = [
  '请比较 Luna 的最终决定与 Mora 的回执，保留分歧和各自的证据。',
  '逐项写出负责人、当前状态、下一步，并标明哪些结论仍待验证。',
  '', '```sh', 'node --test tests/conversation.mjs --test-name-pattern="cursor|receipt"', '```', '',
  ...Array.from({ length: 25 }, (_, i) => `第 ${i + 1} 项：保留窗口坐标、原始操作身份和精确回执。遇到未确认状态时先查回执。`),
].join('\n');
const artifacts = {};
const pageErrors = {};
const browser = await puppeteer.launch({ executablePath: values.chrome, headless: true, args: ['--no-sandbox'] });
await mkdir(out, { recursive: true });
const result = { url: url.href, browser: await browser.version(), time: new Date().toISOString(), artifacts, pageErrors, cases: {} };

async function replay(key, viewport, run, { faultRead, holdWrite, shortPages } = {}) {
  const context = await browser.createBrowserContext();
  const page = await context.newPage();
  const reads = [];
  const attempts = [];
  const operationQueries = [];
  pageErrors[key] = [];
  page.setDefaultTimeout(5000);
  await page.setViewport(viewport);
  await page.setCacheEnabled(false);
  page.on('pageerror', (error) => pageErrors[key].push(error.message));
  page.on('response', (response) => {
    const u = new URL(response.url());
    if (u.origin === url.origin && u.pathname.startsWith('/api/v1/operations/')) {
      operationQueries.push({ requestId: u.pathname.split('/').at(-1), generation: u.searchParams.get('generation'), status: response.status() });
    }
    if (u.origin !== url.origin || !(/\/space\/assets\/.+\.(?:js|css)$/.test(u.pathname) || response.request().isNavigationRequest())) return;
    void response.buffer().then((body) => { artifacts[u.pathname] = digest(body); }).catch(() => {});
  });
  await page.setRequestInterception(true);
  page.on('request', async (request) => {
    if (request.isInterceptResolutionHandled()) return;
    const info = routeInfo(request);
    try {
      if (!['GET', 'HEAD', 'OPTIONS'].includes(info.method)) {
        let body;
        try { body = JSON.parse(request.postData()); } catch {}
        attempts.push({ ...info, body });
        if (holdWrite && info.agent && attempts.length === 1) await holdWrite.promise;
        return await request.abort('connectionfailed');
      }
      if (info.agent && info.method === 'GET') {
        reads.push(info);
        if (faultRead?.(info, reads)) {
          await pause(850);
          return await request.respond({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: { code: 'Unavailable', message: '历史记录暂时不可用' } }) });
        }
        if (shortPages && info.agent === 'luna') {
          // Retain the authoritative fixture's records/generation, expose one per successful page.
          const response = await fetch(request.url(), { signal: AbortSignal.timeout(3000) });
          if (!response.ok) return await request.continue();
          const raw = await response.json();
          const messages = raw.messages.slice(0, 1);
          const body = { ...raw, messages, has_more: raw.messages.length > 1 || raw.has_more, next_after: messages[0]?.id ?? raw.next_after };
          return await request.respond({ status: 200, contentType: 'application/json', body: JSON.stringify(body) });
        }
      }
      return await request.continue();
    } catch (error) {
      if (!page.isClosed()) pageErrors[key].push(error.message);
    }
  });
  const shot = (suffix) => page.screenshot({ path: resolve(out, `${key}-${suffix}.png`), fullPage: true });
  try {
    await page.goto(url.href, { waitUntil: 'domcontentloaded' });
    await page.waitForSelector(subject('luna'), { visible: true });
    return await run({ page, reads, attempts, operationQueries, shot });
  } finally {
    holdWrite?.release();
    await context.close();
  }
}
async function open(page, name, touch = false) {
  if (touch) await page.tap(subject(name));
  else await page.locator(subject(name)).click();
  await page.waitForSelector(`${reader(name)} article`, { visible: true });
  if (!touch) await page.mouse.move(4, 4);
}
async function wheel(page, name, deltaY) {
  const box = await (await page.$(reader(name))).boundingBox();
  if (!box) throw new Error(`No visible scroll region for @${name}`);
  await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.62);
  await page.mouse.wheel({ deltaY });
  await pause(100);
}
async function gesture(page, name) {
  const b = await (await page.$(reader(name))).boundingBox();
  const x = b.x + b.width / 2;
  await page.touchscreen.touchStart(x, b.y + b.height * 0.82);
  for (let i = 1; i <= 5; i++) await page.touchscreen.touchMove(x, b.y + b.height * (0.82 - i * 0.12));
  await page.touchscreen.touchEnd();
  await pause(250);
}
async function visibleText(page, name, pattern) {
  return page.$eval(pane(name), (p, source) => [...p.querySelectorAll('[role=status],[aria-live],button,[data-operation-state]')]
    .filter((e) => e.getClientRects().length && e.closest('[aria-hidden=true]') === null && !e.closest('article'))
    .flatMap((e) => e.innerText.split('\n')).filter((s) => new RegExp(source, 'i').test(s)), pattern);
}
const readState = (page, name) => page.$eval(reader(name), (h) => {
  const box = h.getBoundingClientRect(), top = Math.max(0, box.top), bottom = Math.min(innerHeight, box.bottom);
  const styles = getComputedStyle(h);
  const complete = (r) => r.height > 0 && r.top >= top - 0.5 && r.bottom <= bottom + 0.5;
  const content = [...h.querySelectorAll('article')].map((a) => {
    const label = a.innerText.match(/\[[^\]\n]+ \/ \d+\]/)?.[0] || a.querySelector('time')?.dateTime;
    const head = a.querySelector('header');
    const rect = a.getBoundingClientRect();
    return { a, label, header: head ? complete(head.getBoundingClientRect()) : false, full: complete(rect), y: Math.round(rect.top - box.top), visible: rect.bottom > top && rect.top < bottom };
  });
  const fullLines = new Set();
  let bodyCharacters = 0;
  const firstBody = h.querySelector('article .markdown');
  for (const { a, label, visible } of content) {
    if (!visible) continue;
    const markdown = a.querySelector('.markdown');
    if (!markdown) continue;
    const walker = document.createTreeWalker(markdown, NodeFilter.SHOW_TEXT);
    const range = document.createRange();
    while (walker.nextNode()) {
      if (!walker.currentNode.textContent.trim()) continue;
      range.selectNodeContents(walker.currentNode);
      const rects = [...range.getClientRects()];
      const lines = rects.filter((r) => complete(r) && r.width > 0);
      for (const r of lines) fullLines.add(`${label}:${Math.round((r.top + r.bottom) / 2)}`);
      if (!lines.length) continue;
      if (lines.length === rects.length) {
        bodyCharacters += walker.currentNode.textContent.replace(/\s/g, '').length;
      } else {
        const node = walker.currentNode;
        for (let i = 0; i < node.length; i++) {
          if (/\s/.test(node.data[i])) continue;
          range.setStart(node, i);
          range.setEnd(node, i + 1);
          const r = range.getBoundingClientRect();
          if (complete(r) && r.width > 0) bodyCharacters++;
        }
      }
    }
  }
  return {
    first: content[0]?.label, last: content.at(-1)?.label, records: content.length,
    firstVisible: content.find((x) => x.visible)?.label, lastVisible: content.findLast((x) => x.visible)?.label,
    anchorY: content.find((x) => x.visible)?.y, completeHeaders: content.filter((x) => x.header).length, completeRecords: content.filter((x) => x.full).length, contentLines: fullLines.size, bodyCharacters,
    top: Math.round(h.scrollTop), remaining: Math.round(h.scrollHeight - h.scrollTop - h.clientHeight),
    width: h.clientWidth, height: h.clientHeight, padding: [styles.paddingTop, styles.paddingRight],
    bodyWidth: firstBody?.clientWidth, font: firstBody ? getComputedStyle(firstBody).fontSize : null,
  };
});
const draftState = (page, name) => page.$eval(editor(name), (t) => {
  const b = t.getBoundingClientRect(), p = t.closest('section[aria-label^="@"]');
  const button = p?.querySelector('button[aria-label="发送 AgentX 消息"]');
  const hit = (el) => {
    if (!el) return false;
    const r = el.getBoundingClientRect();
    return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
  };
  return { value: t.value, height: t.clientHeight, contentHeight: t.scrollHeight, top: t.scrollTop, y: Math.round(b.y), editable: hit(t), sendReachable: hit(button), sendDisabled: button?.disabled };
});
const sampleOcclusion = (page, name) => page.$eval(reader(name), (h) => {
  const rect = h.getBoundingClientRect();
  const blockers = [];
  for (const x of [0.2, 0.5, 0.8]) for (const y of [0.25, 0.5, 0.75]) {
    const hit = document.elementFromPoint(rect.left + x * rect.width, rect.top + y * rect.height);
    if (!hit || h.contains(hit)) continue;
    blockers.push({ x, y, tag: hit.tagName, className: hit.getAttribute('class'), text: hit.textContent.trim().slice(0, 90) });
  }
  return blockers;
});
const gate = () => {
  let release;
  const promise = new Promise((done) => { release = done; });
  return { promise, release };
};
async function until(condition, time = 2200) {
  const start = performance.now();
  while (performance.now() - start < time) {
    if (await condition()) return true;
    await pause(50);
  }
  return false;
}
const attemptSummary = (attempt) => {
  const { body, ...route } = attempt;
  return { ...route, target: body?.agent, generation: body?.generation, requestId: body?.request_id, textHash: typeof body?.message?.text === 'string' ? digest(body.message.text) : null };
};
const safeDraft = ({ value, ...rest }) => ({ ...rest, length: value.length });

const cases = {
  async density() {
    const results = {};
    for (const [size, vp] of Object.entries(viewports)) {
      results[size] = await replay(`density-${size}`, vp, async ({ page, shot }) => {
        await open(page, 'luna', Boolean(vp.hasTouch));
        await shot('one');
        const luna = await readState(page, 'luna');
        if (size === 'comparison') {
          await open(page, 'mora');
          await shot('two');
          return { luna, two: { luna: await readState(page, 'luna'), mora: await readState(page, 'mora') } };
        }
        if (size === 'mobile') {
          await pause(1100);
          const coveredBeforeTouch = await sampleOcclusion(page, 'luna');
          await shot('settled');
          await gesture(page, 'luna');
          await shot('touch');
          return { luna, coveredBeforeTouch, afterTouch: await readState(page, 'luna'), coveredAfterTouch: await sampleOcclusion(page, 'luna') };
        }
        return { luna };
      });
    }
    return results;
  },
  current: () => replay('current', viewports.desktop, async ({ page, reads, shot }) => {
    const start = performance.now();
    await open(page, 'luna');
    const entry = await readState(page, 'luna');
    await shot('opening');
    const jumps = [];
    let stop = 'bounded replay limit';
    for (let i = 0; i < 14; i++) {
      const before = await readState(page, 'luna');
      await wheel(page, 'luna', 20000);
      let grew = await until(async () => (await readState(page, 'luna')).records > before.records, 1100);
      let manual = false;
      if (!grew) {
        const more = await page.$(tab('luna', '更多消息'));
        if (more) {
          await more.click();
          manual = true;
          grew = await until(async () => (await readState(page, 'luna')).records > before.records);
        }
      }
      const state = await readState(page, 'luna');
      jumps.push({ step: i + 1, manual, grew, ...state });
      if (!grew) {
        stop = 'no further growth after boundary action';
        break;
      }
    }
    await shot('arrival');
    const endFeedback = await visibleText(page, 'luna', '已到|没有更多|结束|最新|已加载');
    const readCount = reads.length;
    await wheel(page, 'luna', 20000);
    await pause(300);
    return { entry, jumps, stop, endFeedback, requestsAfterEnd: reads.length - readCount, elapsedMs: Math.round(performance.now() - start), reads };
  }),
  boundary: () => replay('boundary', viewports.comparison, async ({ page, reads, shot }) => {
    await open(page, 'luna');
    await open(page, 'mora');
    const moraBefore = await readState(page, 'mora');
    await wheel(page, 'luna', 20000);
    const before = await readState(page, 'luna');
    const paged = (name) => reads.filter((r) => r.agent === name && r.after !== null);
    const autoRequested = await until(() => paged('luna').length > 0, 450);
    if (!autoRequested) {
      const more = await page.$(tab('luna', '更多消息'));
      if (more) await more.click();
    }
    await until(async () => (await visibleText(page, 'luna', '加载|读取')).length > 0, 400);
    const loading = await visibleText(page, 'luna', '加载|读取|更多');
    await shot('pending');
    const showsFailure = await until(async () => (await visibleText(page, 'luna', '失败|重试')).length > 0);
    const afterFailure = await readState(page, 'luna');
    const failed = await visibleText(page, 'luna', '失败|重试|503|更多');
    const moraAfter = await readState(page, 'mora');
    await shot('failure');
    const retry = await page.$(tab('luna', '重试')) || await page.$(tab('luna', '更多消息'));
    if (retry) await retry.click();
    const recovered = await until(async () => (await readState(page, 'luna')).records > before.records);
    const afterRetry = await readState(page, 'luna');
    await shot('recovered');
    return { autoRequested, loading, showsFailure, failed, retryReachable: Boolean(retry), recovered, before, afterFailure, afterRetry, moraBefore, moraAfter, reads };
  }, { faultRead: (info, reads) => info.agent === 'luna' && info.after && reads.filter((r) => r.agent === 'luna' && r.after === info.after).length === 1 }),
  async draft() {
    const output = {};
    for (const size of ['desktop', 'mobile']) {
      output[size] = await replay(`draft-${size}`, viewports[size], async ({ page, attempts, shot }) => {
        await open(page, 'luna', size === 'mobile');
        const blank = await draftState(page, 'luna');
        await page.click(editor('luna'));
        await page.keyboard.sendCharacter(draftText); // Browser input event and React controlled draft; like multiline paste.
        await page.keyboard.press('Enter');
        const large = await draftState(page, 'luna');
        await shot('long');
        const readerWithDraft = await readState(page, 'luna');
        const resizeControls = await page.$eval(pane('luna'), (p) => [...p.querySelectorAll('[aria-label],[title]')].filter((x) => /高度|大小|调整.*编辑|resize/i.test([x.getAttribute('aria-label'), x.getAttribute('title')].join(' '))).map((x) => ({ tag: x.tagName, label: x.getAttribute('aria-label'), title: x.title, tabIndex: x.tabIndex })));
        let ime = { exercised: false };
        if (size === 'desktop') {
          const cdp = await page.createCDPSession();
          await cdp.send('Input.imeSetComposition', { text: '核对', selectionStart: 2, selectionEnd: 2 });
          const composition = await draftState(page, 'luna');
          await page.keyboard.press('Enter');
          await cdp.send('Input.imeSetComposition', { text: '', selectionStart: 0, selectionEnd: 0 });
          ime = { exercised: true, compositionVisible: composition.value.includes('核对'), blockedSubmissions: attempts.length };
          await cdp.detach();
        }
        const edited = await draftState(page, 'luna');
        const reading = await readState(page, 'luna');
        if (size === 'mobile') await gesture(page, 'luna');
        else await wheel(page, 'luna', 610);
        const anchor = await readState(page, 'luna');
        await page.locator(windowAction('luna', '最小化')).click();
        await open(page, 'mora', size === 'mobile');
        await page.click(editor('mora'));
        await page.keyboard.sendCharacter('仅对应 Mora 的草稿。');
        await page.locator(shelf('luna')).click();
        await page.waitForSelector(editor('luna'), { visible: true });
        const returned = await draftState(page, 'luna');
        const returnAnchor = await readState(page, 'luna');
        await shot('returned');
        await page.click(editor('luna'));
        await page.keyboard.down('Control');
        await page.keyboard.press('a');
        await page.keyboard.up('Control');
        await page.keyboard.press('Backspace');
        const cleared = await draftState(page, 'luna');
        return { blank: safeDraft(blank), large: safeDraft(large), readerWithDraft, resizeControls, enterPreserved: large.value === draftText + '\n', ime, reading, anchor, returned: safeDraft(returned), returnAnchor, sameDraft: edited.value === returned.value, cleared: safeDraft(cleared), attempts: attempts.map(attemptSummary) };
      });
    }
    return output;
  },
  async resize() {
    const output = {};
    for (const size of ['desktop', 'mobile']) {
      const touch = size === 'mobile';
      output[size] = await replay(`resize-${size}`, viewports[size], async ({ page, attempts, shot }) => {
        await open(page, 'luna', touch);
        if (touch) await page.tap(editor('luna'));
        else await page.click(editor('luna'));
        await page.keyboard.sendCharacter(draftText);
        const state = async () => ({ editor: safeDraft(await draftState(page, 'luna')), reader: await readState(page, 'luna') });
        const before = await state();
        const grip = `${pane('luna')} [aria-label="拖动调整草稿高度"]`;
        const gripHandle = await page.$(grip);
        const gripVisible = Boolean(await gripHandle?.boundingBox());
        const menu = `${pane('luna')} summary[aria-label="调整草稿高度"]`;
        if (!await page.$(menu) && !gripVisible) return { before, gripVisible };
        async function drag(dy) {
          const b = await (await page.$(grip)).boundingBox();
          const x = b.x + b.width / 2, y = b.y + b.height / 2;
          if (touch) {
            await page.touchscreen.touchStart(x, y);
            for (let k = 1; k <= 8; k++) await page.touchscreen.touchMove(x, y + dy * k / 8);
            await page.touchscreen.touchEnd();
          } else {
            await page.mouse.move(x, y);
            await page.mouse.down();
            await page.mouse.move(x, y + dy, { steps: 8 });
            await page.mouse.up();
          }
          await pause(120);
          return state();
        }
        const draggedSmaller = gripVisible ? await drag(100) : null;
        const draggedLarger = gripVisible ? await drag(-145) : null;
        await shot('dragged');
        if (touch) await page.tap(menu);
        else {
          await page.focus(grip); // Start at the actual focusable grip; subsequent traversal uses Tab.
          await page.keyboard.press('Enter');
        }
        const openHeightMenu = await page.$eval(menu, (e) => e.parentElement.open);
        const focusPath = [];
        if (openHeightMenu) {
          if (touch) await (await page.$(tab('luna', '收紧'))).tap();
          else for (let k = 0; k < 9; k++) {
            const current = await page.evaluate(() => document.activeElement?.textContent.trim() || document.activeElement?.getAttribute('aria-label'));
            focusPath.push(current);
            if (current === '收紧') {
              await page.keyboard.press('Enter');
              break;
            }
            await page.keyboard.press('Tab');
          }
        }
        const selectedCompact = await state();
        await shot('compact-choice');
        return { before, gripVisible, draggedSmaller, draggedLarger, openHeightMenu, focusPath, selectedCompact, intact: (await draftState(page, 'luna')).value === draftText, attempts: attempts.map(attemptSummary) };
      });
    }
    return output;
  },
  compact: () => replay('compact', viewports.mobile, async ({ page, shot, attempts }) => {
    await open(page, 'luna', true);
    await page.tap(editor('luna'));
    await page.keyboard.sendCharacter(draftText);
    const before = { editor: safeDraft(await draftState(page, 'luna')), reader: await readState(page, 'luna') };
    // Model reduced available height without making a claim about physical mobile keyboards.
    await page.setViewport({ ...viewports.mobile, height: 520 });
    await pause(150);
    const short = { editor: safeDraft(await draftState(page, 'luna')), reader: await readState(page, 'luna') };
    await shot('short');
    await gesture(page, 'luna');
    const shortAfterSwipe = { editor: safeDraft(await draftState(page, 'luna')), reader: await readState(page, 'luna') };
    await shot('short-after-swipe');
    let shortAfterCompact;
    const sizeControl = `${pane('luna')} summary[aria-label="调整草稿高度"]`;
    if (await page.$(sizeControl)) {
      await page.tap(sizeControl);
      await (await page.$(tab('luna', '收紧'))).tap();
      shortAfterCompact = { editor: safeDraft(await draftState(page, 'luna')), reader: await readState(page, 'luna') };
      await shot('short-compact-choice');
    }
    await page.setViewport(viewports.mobile);
    await pause(150);
    const restored = await draftState(page, 'luna');
    return { before, short, shortAfterSwipe, shortAfterCompact, restored: safeDraft(restored), intact: restored.value === draftText, attempts: attempts.map(attemptSummary) };
  }),
  receipt() {
    const holdWrite = gate();
    return replay('receipt', viewports.comparison, async ({ page, attempts, operationQueries, shot }) => {
      await open(page, 'luna');
      await open(page, 'mora');
      await page.click(editor('luna'));
      await page.keyboard.sendCharacter(draftText);
      await page.locator(send('luna')).click();
      if (!await until(() => attempts.length === 1)) throw new Error('Intentional submission never reached browser interception');
      const pending = await draftState(page, 'luna');
      const pendingText = await visibleText(page, 'luna', '提交|发送|回执|处理中');
      await page.locator(windowAction('luna', '关闭')).click();
      await page.click(editor('mora'));
      const moraText = 'Mora：保留核对中的第二份草稿。';
      await page.keyboard.sendCharacter(moraText);
      holdWrite.release();
      await pause(450);
      const mora = await draftState(page, 'mora');
      await page.locator(windowAction('mora', '最小化')).click();
      await open(page, 'luna');
      const unknown = await until(async () => (await visibleText(page, 'luna', '回执未确认|重复提交|unknown|保留原凭据|发送结果未确定')).length > 0);
      const failure = await draftState(page, 'luna');
      const failureText = await visibleText(page, 'luna', '回执|失败|重试|提交|保留原凭据|发送结果未确定');
      await shot('unknown-after-return');
      const query = await page.$(tab('luna', '查询原回执'));
      if (query && await query.boundingBox()) {
        await query.click();
        await until(() => operationQueries.length > 0);
      }
      const afterQuery = await draftState(page, 'luna');
      const afterQueryText = await visibleText(page, 'luna', '回执|失败|重试|提交|保留原凭据|发送结果未确定');
      // Deliberately exercise enabled submit affordance after unknown outcome. Writes remain blocked.
      if (!afterQuery.sendDisabled && afterQuery.sendReachable) {
        await page.locator(send('luna')).click();
        await pause(450);
      }
      return { expectedHash: digest(draftText), pending: safeDraft(pending), pendingText, moraUntouched: mora.value === moraText, unknown, failure: safeDraft(failure), failureText, afterQuery: safeDraft(afterQuery), afterQueryText, originalDraftRetained: afterQuery.value === draftText, attempts: attempts.map(attemptSummary), operationQueries };
    }, { holdWrite });
  },
  async short() {
    const output = {};
    for (const input of ['wheel', 'keyboard', 'touch']) {
      output[input] = await replay(`short-${input}`, input === 'touch' ? viewports.mobile : viewports.desktop, async ({ page, reads, shot }) => {
        await open(page, 'luna', input === 'touch');
        const initial = await readState(page, 'luna');
        await pause(600);
        const idle = await readState(page, 'luna');
        const requestsAtIdle = reads.map((r) => r.after);
        let keyboardFocus = null;
        if (input === 'wheel') await wheel(page, 'luna', 1000);
        if (input === 'touch') await gesture(page, 'luna');
        if (input === 'keyboard') {
          const scrollTabIndex = await page.$eval(reader('luna'), (e) => e.tabIndex);
          if (scrollTabIndex >= 0) await page.focus(reader('luna')); // Begin the keystroke on the reachable reader.
          keyboardFocus = await page.$eval(reader('luna'), (e) => document.activeElement === e);
          await page.keyboard.press('PageDown');
        }
        await pause(500);
        const afterInput = await readState(page, 'luna');
        const requestsAfterInput = reads.map((r) => r.after);
        const fallback = await page.$(tab('luna', '更多消息')) || await page.$(tab('luna', '继续'));
        const fallbackReachable = Boolean(await fallback?.boundingBox());
        if (fallbackReachable && afterInput.records === idle.records) {
          if (input === 'touch') await fallback.tap();
          else if (input === 'keyboard') {
            await fallback.focus();
            await page.keyboard.press('Enter');
          } else await fallback.click();
          await until(async () => (await readState(page, 'luna')).records > afterInput.records);
        }
        const afterFallback = await readState(page, 'luna');
        await shot('settled');
        return { initial, idle, requestsAtIdle, keyboardFocus, afterInput, requestsAfterInput, fallbackReachable, afterFallback, reads };
      }, { shortPages: true });
    }
    return output;
  },
};

try {
  for (const name of values.cases.split(',')) {
    if (!Object.hasOwn(cases, name)) throw new Error(`Unknown case: ${name}`);
    try { result.cases[name] = await cases[name](); }
    catch (error) {
      result.cases[name] = { replayError: error.message, stack: error.stack?.split('\n').slice(0, 8).join('\n') };
      process.exitCode = 1;
    }
    console.log(`${name}: ${result.cases[name].replayError || 'evidence recorded'}`);
  }
} finally {
  await browser.close();
  await writeFile(resolve(out, 'results.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(resolve(out, 'results.json'));
}
