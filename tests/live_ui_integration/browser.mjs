/** One actual candidate, both maintained embedded UIs, native fixtures and receipts. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { checkpoint } from '../collaboration_workbench/browser_checkpoint.mjs';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { randomUUID } from 'node:crypto';

const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
const origin = new URL(config.url);
assert.equal(origin.hostname, '127.0.0.1');
assert(Number(origin.port) > 1024 && ![18967, 18971, 38023].includes(Number(origin.port)));
assert(config.generation && config.reference && config.recovery && config.scope);
await mkdir(config.evidence, { recursive: false });
const require = createRequire(import.meta.url);
const driver = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
const { chromium } = require(driver);
const browser = await chromium.launch({ executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',
  headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server'] });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
const traffic = [], errors = [], steps = [];
let stage = 'open', failure, active;
const space = await context.newPage(), inspector = await context.newPage();
for (const [app, page] of [['space', space], ['inspector', inspector]]) {
  page.setDefaultTimeout(30000);
  page.on('pageerror', error => errors.push({ app, message: error.message }));
  page.on('request', request => {
    const url = new URL(request.url());
    if (url.protocol === 'data:' || url.protocol === 'blob:') return;
    assert.equal(url.origin, origin.origin, `external request ${url}`);
    if (url.pathname.startsWith('/api/')) traffic.push({ app, kind: 'request', method: request.method(), url: request.url(), body: request.postData(), at: Date.now() });
  });
  page.on('response', response => {
    if (new URL(response.url()).pathname.startsWith('/api/')) traffic.push({ app, kind: 'response', url: response.url(), status: response.status(), at: Date.now() });
  });
}
function button(page, name) { return page.getByRole('button', { name, exact: true }); }
async function until(description, observe, timeout = 45000) {
  const end = Date.now() + timeout;
  while (Date.now() < end) {
    const value = await observe();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 150));
  }
  throw new Error(`${description}: timed out`);
}
async function api(path, query = {}) {
  const url = new URL(path, origin);
  for (const [key, value] of Object.entries({ generation: config.generation, ...query })) if (value != null) url.searchParams.set(key, value);
  const response = await context.request.get(url.href);
  const result = await response.json();
  assert.equal(response.status(), 200, JSON.stringify(result));
  return result;
}
async function agents() { return (await api('/api/v1/agents')).agents; }
async function inspection() { return api('/api/v1/inspection'); }
async function messages(agent) {
  return (await api(`/api/v1/agents/${agent.name}/messages`, { incarnation: agent.incarnation, stopped_event: agent.stopped_event })).messages;
}
async function receipt(id) {
  return until(`terminal receipt ${id}`, async () => {
    const value = await api(`/api/v1/operations/${id}`);
    assert(!['Failed', 'Unknown'].includes(value.status), JSON.stringify(value));
    return value.status === 'Succeeded' ? value : null;
  });
}
async function live(name) { return until(`live ${name}`, async () => (await agents()).find(row => row.agent.name === name && row.agent.incarnation && row.session_id && row.can_send)); }
async function capture(page, id, meaning, state) {
  active = page;
  steps.push(await checkpoint(page, config.evidence, id, meaning, state));
  await writeFile(`${config.evidence}/steps.json`, JSON.stringify(steps, null, 2) + '\n');
}
async function visible(page, text) { await until(`visible ${text}`, async () => (await page.locator('body').innerText()).includes(text)); }
async function submit(page, control, suffix, status = 202) {
  const observed = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname.endsWith(suffix));
  await control.click();
  if (page === space && suffix === '/kill') await page.getByRole('dialog', { name: '终止进程', exact: true }).getByRole('button', { name: '确认终止进程', exact: true }).click();
  const response = await observed;
  const body = await response.json();
  assert.equal(response.status(), status, JSON.stringify(body));
  return status === 202 ? receipt(body.request_id) : body;
}
function posts(app) { return traffic.filter(row => row.app === app && row.kind === 'request' && row.method === 'POST'); }
async function spaceSelect(name, history = false) {
  await button(space, '聊天').click();
  await space.getByRole('combobox', { name: '工作区范围', exact: true }).selectOption('');
  await space.getByRole('navigation', { name: '办公室内容' }).getByRole('button', { name: history ? /^历史/ : /^成员/ }).click();
  await space.getByRole('region', { name: 'Agent 名册' }).getByRole('button', { name: new RegExp(`^@${name} ·`) }).click();
  await space.getByRole('textbox', { name: 'Agent 消息草稿', exact: true }).waitFor();
}
async function inspectorSelect(agent) {
  await inspector.goto(`${origin.origin}/inspector/`, { waitUntil: 'domcontentloaded' });
  const listing = inspector.getByRole('complementary', { name: '原生会话' });
  const matching = listing.getByRole('button').filter({ hasText: agent.incarnation ? `实例 ${agent.incarnation}` : `停止 #${agent.stopped_event}` });
  await matching.click();
  await visible(inspector, agent.incarnation ?? agent.stopped_event);
}
async function inspectTab(label) {
  // This is the retained Inspector toolbar, not an alternate test page.
  await inspector.getByRole('tab', { name: label, exact: true }).click();
}
async function openDetailsContaining(page, text) {
  const details = page.locator('details').filter({ hasText: text });
  const count = await details.count();
  for (let index = 0; index < count; index++) {
    const item = details.nth(index);
    if (await item.isVisible()) {
      const summary = item.locator(':scope > summary');
      if (!(await item.getAttribute('open')) && await item.evaluate(element => !element.open)) await summary.click();
    }
  }
}

async function uncertainSend(page, app, agent, composer, send) {
  stage = `${app}-lost-admitted-response`;
  const text = `${app} admitted response deliberately lost`;
  // Prior receipt-backed reads must finish in the UI before the next fault is
  // injected. Independent receipt success does not advance browser state.
  await composer.fill(text);
  await until('prior operation reconciled before fault injection', async () => !(await send.isDisabled()));
  let dropped;
  let attempts = 0;
  const messagePattern = '**/api/v1/agents/*/messages';
  const operationPattern = '**/api/v1/operations/**';
  await page.route(operationPattern, route => route.abort('failed'));
  await page.route(messagePattern, async route => {
    if (route.request().method() !== 'POST') { await route.continue(); return; }
    attempts++;
    const body = route.request().postDataJSON();
    assert.deepEqual(body.agent, agent);
    assert.equal(body.generation, config.generation);
    const journal = await page.evaluate(() => Object.values(localStorage));
    assert(journal.some(value => value.includes(body.request_id)), 'request identity was not persisted before POST');
    const upstream = await route.fetch();
    assert.equal(upstream.status(), 202);
    dropped = { body, admission: await upstream.json() };
    await route.abort('failed');
  });
  await composer.fill(text);
  await send.click();
  await until('actual admission before response loss', async () => dropped);
  await page.unroute(messagePattern);
  const recovered = await receipt(dropped.body.request_id);
  await until('actual native reply to lost receipt', async () => (await messages(agent)).find(message => message.reply_to === recovered.result.Sent.message_id));
  await page.reload({ waitUntil: 'domcontentloaded' });
  if (app === 'space') await spaceSelect(agent.name);
  else {
    // Exact ref query survives reload. Re-selection is harmless and must not POST.
    if (!(await page.getByRole('textbox', { name: '即时消息内容' }).count())) await inspectorSelect(agent);
  }
  await composer.waitFor();
  await composer.fill('replacement must remain blocked');
  assert(await send.isDisabled(), 'uncertain original operation allowed replacement send');
  const bodyText = await page.locator('body').innerText();
  const retained = await page.evaluate(id => Object.values(localStorage).some(value => value.includes(id)), dropped.body.request_id);
  assert(retained, 'uncertain operation lost its original persisted identity');
  await capture(page, `${app}-L06-pending`, 'Actual admitted result lost; reload retains uncertainty and blocks replacement action.', { original: dropped.body, receiptReadIndependently: recovered });
  await page.unroute(operationPattern);
  await openDetailsContaining(page, app === 'space' ? '连接 · 回执' : dropped.body.request_id);
  const originalReceipt = app === 'space' ? page.locator('.live-receipts section').filter({ hasText: dropped.body.request_id })
    : page.getByRole('region', { name: '操作回执' }).locator('details').filter({ hasText: dropped.body.request_id });
  await originalReceipt.getByRole('button', { name: '查询原回执', exact: true }).click();
  await until('receipt reconciled and send available', async () => !(await send.isDisabled()));
  assert.equal(attempts, 1);
  assert.equal(posts(app).filter(row => row.body && JSON.parse(row.body).request_id === dropped.body.request_id).length, 1, 'reload replayed a mutation');
  assert.equal((await messages(agent)).filter(message => message.text === text).length, 1);
  await capture(page, `${app}-L06-reconciled`, 'Original-ID receipt lookup resolves exactly one actual native message; no replay.', { recovered, messages: await messages(agent) });
  await composer.fill('');
}

try {
  let agent, composer;
  if (!config.start) {
  stage = 'default-live-supply';
  active = space;
  await space.goto(`${origin.origin}/space/`, { waitUntil: 'domcontentloaded' });
  await spaceSelect(config.reference.name);
  await inspectorSelect(config.reference);
  assert.equal(posts('space').length, 0, 'selection caused a write');
  assert.equal(posts('inspector').length, 0, 'selection caused a write');
  await capture(space, 'space-L02-default', 'Default Space reads actual native exact member without demo flags or automatic mutations.', { expected: config.reference, supplied: await agents() });
  await capture(inspector, 'inspector-L02-default', 'Default Inspector reads the same native scope and exact incarnation.', { expected: config.reference, supplied: await agents() });

  stage = 'space-role-launch';
  active = space;
  await space.getByRole('combobox', { name: '工作区范围', exact: true }).selectOption(config.paths.empty);
  await visible(space, '暂无成员');
  const emptyFacts = await api('/api/v1/workspaces');
  assert(emptyFacts.workspaces.some(row => row.path === config.paths.empty));
  await space.getByRole('combobox', { name: '工作区范围', exact: true }).selectOption(config.paths.child);
  await space.getByRole('navigation', { name: '办公室内容' }).getByRole('button', { name: /^岗位/ }).click();
  await space.getByRole('region', { name: '可用岗位' }).getByRole('button').filter({ hasText: 'reviewer' }).click();
  await space.getByRole('textbox', { name: '岗位启动任务', exact: true }).fill('browser live role fixture initial');
  const launched = await submit(space, button(space, '按此岗位启动'), '/api/v1/agents');
  assert.equal(launched.result.Launched.failed, 0);
  const row = await live(launched.result.Launched.agents[0]);
  agent = row.agent;
  assert.equal(row.collaboration.status, 'adopted');
  assert.equal(row.collaboration.workspace, config.paths.child);
  assert.equal(row.collaboration.role.path, `${config.paths.child}/.agentx/roles/reviewer.toml`);
  await spaceSelect(agent.name);
  await capture(space, 'space-L04-launched', 'Deliberate effective Role launch produces an actual adopted native member in the preserved UI.', { launched, row });

  stage = 'space-real-message-draft';
  composer = space.getByRole('textbox', { name: 'Agent 消息草稿', exact: true });
  await composer.fill('exact member retained draft');
  const pollStart = traffic.filter(item => item.app === 'space' && item.kind === 'request' && new URL(item.url).pathname === '/api/v1/agents').length;
  await button(space, '场景').click();
  await capture(space, 'space-L05-office-scene', 'Preserved visual office scene displays actual supplied members and the exact selected member/draft.', { selected: agent, members: await agents() });
  await space.setViewportSize({ width: 390, height: 844 });
  await button(space, '放大大厅').waitFor();
  await capture(space, 'space-L09-office-scene-narrow', 'Preserved actual office scene and contextual camera controls remain present on a narrow viewport.', { selected: agent, viewport: space.viewportSize() });
  await space.setViewportSize({ width: 1440, height: 900 });
  await button(space, '全景').click();
  await button(space, '暂停动作').click();
  await button(space, '放大大厅').click();
  const roomPositions = () => space.locator('.hall-room-name').evaluateAll(elements => elements.map(element => ({ room: element.textContent, style: element.getAttribute('style') })));
  const cameraProjection = await roomPositions();
  assert(cameraProjection.length > 0);
  await button(space, '聊天').click();
  await button(space, '场景').click();
  await until('same office camera projection after presentation remount', async () => JSON.stringify(await roomPositions()) === JSON.stringify(cameraProjection));
  assert.equal(await button(space, '播放动作').count(), 1, 'paused scene preference lost on presentation switch');
  await capture(space, 'space-L05-camera-retained', 'Deliberate panorama/zoom and pause survive chat/scene remount in the same office.', { roomProjection: cameraProjection, selected: agent });
  await button(space, '聊天').click();
  await spaceSelect(agent.name);
  await until('two actual directory polls', async () => traffic.filter(item => item.app === 'space' && item.kind === 'request' && new URL(item.url).pathname === '/api/v1/agents').length >= pollStart + 2);
  assert.equal(await composer.inputValue(), 'exact member retained draft');
  const text = 'browser Space actual native request';
  await composer.fill(text);
  const sent = await submit(space, button(space, '发送 AgentX 消息'), '/messages');
  await until('real generated-hook reply', async () => (await messages(agent)).find(message => message.reply_to === sent.result.Sent.message_id));
  await visible(space, `fixture reply #${sent.result.Sent.message_id}`);
  await button(space, 'Transcript').click();
  assert.equal(await composer.count(), 0, 'Transcript must not offer a message editor');
  await button(space, '读取 Transcript').click();
  await until('transcript operation receipt', async () => posts('space').some(item => new URL(item.url).pathname.endsWith('/transcript')));
  await visible(space, 'browser live role fixture initial');
  await capture(space, 'space-L05-reply-transcript', 'Actual message/reply and explicit Provider transcript remain separate channels.', { sent, messages: await messages(agent) });
  await button(space, '消息').click();
  await uncertainSend(space, 'space', agent, composer, button(space, '发送 AgentX 消息'));
  } else {
    agent = config.reference;
    await space.goto(`${origin.origin}/space/`, { waitUntil: 'domcontentloaded' });
    await spaceSelect(agent.name);
    composer = space.getByRole('textbox', { name: 'Agent 消息草稿', exact: true });
  }

  if (config.start !== 'stop-history-resume') {
  stage = 'inspector-message-transcript';
  active = inspector;
  await inspectorSelect(agent);
  const inspectorComposer = inspector.getByRole('textbox', { name: '即时消息内容', exact: true });
  if (config.start !== 'inspector-lost-admitted-response') {
  await inspectorComposer.fill('Inspector actual native message');
  const inspectedSend = await submit(inspector, button(inspector, '发送'), '/messages');
  await until('Inspector request native reply', async () => (await messages(agent)).find(message => message.reply_to === inspectedSend.result.Sent.message_id));
  // Native query data is independently confirmed; reload is a deliberate fresh read.
  await inspector.reload({ waitUntil: 'domcontentloaded' });
  await visible(inspector, agent.incarnation);
  await until('reply loaded in actual DOM', async () => (await inspector.locator('body').textContent()).includes(`fixture reply #${inspectedSend.result.Sent.message_id}`));
  await openDetailsContaining(inspector, `fixture reply #${inspectedSend.result.Sent.message_id}`);
  await visible(inspector, `fixture reply #${inspectedSend.result.Sent.message_id}`);
  const transcript = await submit(inspector, button(inspector, '读取最近模型对话'), '/transcript');
  assert(transcript.result.Transcript.available && transcript.result.Transcript.exchanges.length);
  await visible(inspector, '最近模型对话');
  await capture(inspector, 'inspector-L05-live-read', 'Inspector reads the exact Space-launched member, actual message reply and bounded native transcript.', { inspectedSend, transcript });
  }
  await uncertainSend(inspector, 'inspector', agent, inspectorComposer, button(inspector, '发送'));

  stage = 'directory-preview-send';
  active = space;
  await space.locator('.live-directory-contact > summary').click();
  await space.getByRole('textbox', { name: '联系目录路径' }).fill(config.paths.nested);
  const beforePreview = posts('space').length;
  await button(space, '预览入口').click();
  await visible(space, config.paths.project);
  await until('native directory preview loaded', async () => space.getByRole('textbox', { name: '目录联系消息' }).isVisible());
  assert.equal(posts('space').length, beforePreview, 'preview mutated');
  const preview = await api('/api/v1/directory-entry', { directory: config.paths.nested });
  assert.equal(preview.resolved_workspace, config.paths.project);
  assert.equal(preview.role.name, 'controller');
  await space.getByRole('textbox', { name: '目录联系消息' }).fill('browser directory contact request');
  const directorySent = await submit(space, button(space, '发送到此入口'), '/directory-messages');
  await capture(space, 'space-L03-directory', 'Nested directory previews inherited real default before deliberate contact; native receipt owns launch/send.', { preview, directorySent });

  stage = 'inspector-collection';
  active = inspector;
  await inspectTab('明细');
  let snapshot = await inspection();
  assert(snapshot.settings.enabled);
  const disabled = await submit(inspector, button(inspector, '暂停采集与自动继续'), '/inspection/configure', 200);
  assert.equal(disabled.settings.enabled, false);
  const enabled = await submit(inspector, button(inspector, '启用采集'), '/inspection/configure', 200);
  assert.equal(enabled.settings.enabled, true);
  assert(enabled.settings.revision > disabled.settings.revision);
  assert.equal(enabled.settings.collected_from, snapshot.settings.collected_from);
  await capture(inspector, 'inspector-I01-collection', 'UI explicitly pauses/re-enables actual durable collection and retains its enable-time origin.', { disabled, enabled });
  await inspectTab('Token');
  await visible(inspector, '未知');
  await visible(inspector, '101');
  await capture(inspector, 'inspector-I02-tokens', 'Native parsed decimal cumulative counters coexist with unknown usage and source coverage; no mock totals.', await inspection());

  stage = 'inspector-analysis';
  await inspectTab('分析');
  await inspector.getByText(/^选择采集证据（/).click();
  await inspector.getByRole('checkbox').first().check();
  const job = await submit(inspector, button(inspector, '生成证据摘要'), '/inspection/analyze', 200);
  assert(job.observation_ids.length === 1 && ['queued', 'succeeded'].includes(job.status));
  const finished = await until('browser job executed by native host', async () => (await inspection()).jobs.find(row => row.id === job.id && row.status === 'succeeded'));
  await until('durable report rendered', async () => (await inspector.locator('body').textContent()).includes('native-evidence-v1'));
  await openDetailsContaining(inspector, job.id);
  await visible(inspector, 'native-evidence-v1');
  await capture(inspector, 'inspector-I04-report', 'Existing analysis destination shows a pinned deterministic native report, not an AI model result.', { submitted: job, finished });

  stage = 'inspector-recovery-queued';
  await inspectTab('恢复');
  await inspector.getByRole('textbox', { name: '错误持续秒数', exact: true }).fill('1');
  await inspector.getByRole('checkbox', { name: '自动请求继续' }).check();
  const policy = await submit(inspector, button(inspector, '保存策略'), '/inspection/configure', 200);
  assert.deepEqual(policy.settings.recovery, { enabled: true, error_duration_seconds: 1 });
  const attempt = await until('actual native queued continuation', async () => (await inspection()).attempts.find(value => value.episode.agent.name === config.recovery.name));
  assert.equal(attempt.status, 'queued');
  assert(attempt.message_id);
  assert.deepEqual(attempt.episode.agent, config.recovery.agent);
  assert.equal(attempt.episode.session_id, config.recovery.session);
  const acceptedContinuation = await messages(config.recovery.agent);
  assert.equal(acceptedContinuation.filter(message => message.id === attempt.message_id).length, 1, 'queued continuation lacks actual native mailbox effect');
  await visible(inspector, `${config.recovery.name} · queued`);
  await openDetailsContaining(inspector, attempt.id);
  await visible(inspector, attempt.message_id);
  const queuedSnapshot = await inspection();
  const errorObservation = [...queuedSnapshot.observations].reverse().find(value => value.agent.name === config.recovery.name);
  assert(errorObservation.model_request.error && errorObservation.model_request.phase === 'failed', 'mailbox acceptance falsely cleared model error');
  await capture(inspector, 'inspector-I05-queued', 'Configured actual Hook failure yields one queued continuation audit; model error remains unresolved.', { policy, attempt, latest: errorObservation });
  const replayResponse = await context.request.post(`${origin.origin}/api/v1/inspection/continuation`, { data: { RequestContinuation: { generation: config.generation, episode: attempt.episode } } });
  assert.equal(replayResponse.status(), 200);
  assert.deepEqual(await replayResponse.json(), attempt, 'same episode repeated effect');
  assert.equal((await messages(config.recovery.agent)).filter(message => message.id === attempt.message_id).length, 1);
  await writeFile(config.recovery.control, JSON.stringify({ phase: 'completed', turn: config.recovery.turn }) + '\n');
  const cleared = await until('matching generated successful model observation', async () => (await inspection()).attempts.find(value => value.id === attempt.id && value.status === 'observed_error_cleared'));
  assert.equal(cleared.message_id, attempt.message_id);
  await visible(inspector, `${config.recovery.name} · observed_error_cleared`);
  await capture(inspector, 'inspector-I05-observed-success', 'Only a later matching real generated model hook changes queued audit to observed success.', { cleared });
  }

  stage = 'stop-history-resume';
  active = space;
  await spaceSelect(agent.name);
  await composer.fill('draft owned by retired exact ref');
  space.once('dialog', dialog => dialog.accept());
  const stopped = await submit(space, button(space, '终止进程'), '/kill');
  const history = await until('stopped exact record', async () => (await agents()).find(row => row.agent.name === agent.name && row.agent.stopped_event));
  await spaceSelect(agent.name, true);
  await capture(space, 'space-L07-stopped', 'Stopped history remains an exact separate object with resume action.', { stopped, history });
  await inspectorSelect(history.agent);
  assert.equal(await button(inspector, '恢复会话').isDisabled(), false);
  await capture(inspector, 'inspector-L07-stopped', 'Inspector reads the same stopped exact native reference.', history);
  const resumed = await submit(space, button(space, '恢复'), '/resume');
  let successor = await live(resumed.result.Launched.agents[0]);
  assert.notDeepEqual(successor.agent, agent);
  assert.equal(successor.session_id, history.session_id);
  await spaceSelect(successor.agent.name);
  assert.equal(await composer.inputValue(), '', 'retired draft leaked into successor');
  const stale = await context.request.post(`${origin.origin}/api/v1/agents/${agent.name}/messages`, { data: {
    generation: config.generation, request_id: randomUUID(), agent,
    message: { text: 'stale must reject', intent: 'request', thread: null, reply_to: null } } });
  assert.equal(stale.status(), 409);
  await inspectorSelect(successor.agent);
  await capture(space, 'space-L07-successor', 'Resume creates a distinct native incarnation; exact old draft cannot cross the boundary.', { history, successor, resumed });

  stage = 'inspector-stop-resume';
  active = inspector;
  const inspectorRetiring = successor;
  await inspector.getByRole('textbox', { name: '即时消息内容', exact: true }).fill('Inspector draft owned by retired exact ref');
  const inspectorStopped = await submit(inspector, button(inspector, '停止'), '/kill');
  const inspectorHistory = await until('Inspector stopped exact record', async () => (await agents()).find(row => row.agent.name === inspectorRetiring.agent.name
    && row.agent.stopped_event && row.agent.stopped_event !== history.agent.stopped_event));
  await inspectorSelect(inspectorHistory.agent);
  await capture(inspector, 'inspector-L07-stopped-action', 'Deliberate Inspector stop uses a valid native UUID receipt and exposes exact stopped history.', { inspectorStopped, inspectorHistory });
  const inspectorResumed = await submit(inspector, button(inspector, '恢复会话'), '/resume');
  successor = await live(inspectorResumed.result.Launched.agents[0]);
  assert.notDeepEqual(successor.agent, inspectorRetiring.agent);
  assert.equal(successor.session_id, inspectorHistory.session_id);
  await inspectorSelect(successor.agent);
  assert.equal(await inspector.getByRole('textbox', { name: '即时消息内容', exact: true }).inputValue(), '', 'Inspector retired draft leaked into successor');
  await spaceSelect(successor.agent.name);
  await capture(inspector, 'inspector-L07-resumed-action', 'Deliberate Inspector resume starts a distinct native successor and isolates exact-reference drafts.', { inspectorHistory, successor, inspectorResumed });

  stage = 'narrow-ui';
  for (const [app, page, selected] of [['space', space, successor.agent], ['inspector', inspector, successor.agent]]) {
    await page.setViewportSize({ width: 390, height: 844 });
    await visible(page, selected.name);
    const geometry = await page.evaluate(() => ({ width: innerWidth, scroll: document.documentElement.scrollWidth }));
    assert(geometry.scroll <= geometry.width + 1, JSON.stringify({ app, geometry }));
    await capture(page, `${app}-L09-narrow`, 'Same native selected resource and contextual actions remain reachable at narrow viewport.', { geometry, selected });
    await page.setViewportSize({ width: 1440, height: 900 });
  }

  stage = 'generation-reset-late-read';
  const oldGeneration = config.generation;
  const resetReference = successor.agent;
  const held = [];
  const finishedReads = [];
  let releaseReads;
  const release = new Promise(resolve => { releaseReads = resolve; });
  // Delay real upstream message pages, leaving descriptor polling independent.
  // Selection starts these reads; no successful domain response is fabricated.
  for (const [app, page] of [['space', space], ['inspector', inspector]]) {
    let intercepted = false;
    await page.route('**/api/v1/agents/*/messages?*', async route => {
      const url = new URL(route.request().url());
      if (intercepted || route.request().method() !== 'GET'
        || url.pathname !== `/api/v1/agents/${resetReference.name}/messages`
        || url.searchParams.get('incarnation') !== resetReference.incarnation) { await route.continue(); return; }
      intercepted = true;
      const upstream = await route.fetch();
      assert.equal(upstream.status(), 200);
      held.push({ app, body: await upstream.json() });
      await release;
      try { await route.fulfill({ response: upstream }); } catch { /* Dataset unmount may cancel this real old read. */ }
      finally { finishedReads.push(app); }
    });
  }
  await spaceSelect(config.recovery.name);
  await spaceSelect(resetReference.name);
  await inspectorSelect(resetReference);
  await until('both old-generation real reads pinned', async () => held.length === 2);
  assert(held.every(value => value.body.generation === oldGeneration));
  const reset = await promisify(execFile)(config.candidate, ['reset'], { cwd: config.project, env: config.environment, timeout: 30000 });
  await writeFile(`${config.evidence}/reset.json`, JSON.stringify({ stdout: reset.stdout, stderr: reset.stderr, scope: config.scope }, null, 2) + '\n');
  const next = await until('native reset generation', async () => {
    const response = await context.request.get(`${origin.origin}/api/v1/management`);
    assert.equal(response.status(), 200);
    const value = await response.json();
    return value.generation !== oldGeneration ? value : null;
  });
  // Descriptor fences unmount each selected old object before the delayed body
  // is released. DOM cannot keep actionable controls for the retired scope.
  await until('Space reset removes old target actions', async () => await space.getByRole('textbox', { name: 'Agent 消息草稿' }).count() === 0);
  await until('Inspector reset removes old target actions', async () => await inspector.getByRole('textbox', { name: '即时消息内容' }).count() === 0);
  releaseReads();
  await until('delayed real reads released or cancelled', async () => finishedReads.length === 2);
  await until('both reset directories empty', async () => await space.locator('.hall-directory-person').count() === 0
    && await inspector.locator('aside[aria-label="原生会话"] .native-resource').count() === 0);
  assert.equal(await space.getByRole('textbox', { name: 'Agent 消息草稿' }).count(), 0);
  assert.equal(await inspector.getByRole('textbox', { name: '即时消息内容' }).count(), 0);
  const staleGeneration = await context.request.get(`${origin.origin}/api/v1/inspection?generation=${encodeURIComponent(oldGeneration)}`);
  assert.equal(staleGeneration.status(), 409);
  const staleWrite = await context.request.post(`${origin.origin}/api/v1/inspection/configure`, { data: { Configure: {
    generation: oldGeneration, expected_revision: 0, enabled: true, recovery: { enabled: false, error_duration_seconds: 600 } } } });
  assert.equal(staleWrite.status(), 409);
  config.generation = next.generation;
  assert.deepEqual(await agents(), []);
  await capture(space, 'space-L08-generation', 'Real generation reset retires selected target; late real old-generation read cannot restore it.', { oldGeneration, next, delayed: held.find(value => value.app === 'space') });
  await capture(inspector, 'inspector-L08-generation', 'Inspector independently discards the same delayed old-generation message page after reset.', { oldGeneration, next, delayed: held.find(value => value.app === 'inspector') });

  stage = 'api-failure-no-mock-fallback';
  for (const [app, page] of [['space', space], ['inspector', inspector]]) {
    active = page;
    const before = posts(app).length;
    await page.route('**/api/v1/**', route => route.abort('failed'));
    await page.goto(`${origin.origin}/${app}/`, { waitUntil: 'domcontentloaded' });
    await until('visible actual connection failure', async () => /失败|Error|Failed|fetch/i.test(await page.locator('body').innerText()));
    assert.equal(posts(app).length, before, 'failure path posted work');
    if (app === 'space') assert.equal(await page.locator('.hall-directory-person').count(), 0, 'API error produced fixture members');
    else assert.equal(await page.locator('aside[aria-label="原生会话"] .native-resource').count(), 0, 'API error produced fixture sessions');
    await capture(page, `${app}-L02-no-fallback`, 'Unavailable real API remains an error; no fallback fixture resources or mutation.', { posts: posts(app).length });
    await page.unroute('**/api/v1/**');
  }
  assert.deepEqual(errors, []);
} catch (error) {
  failure = { stage, message: error.message, stack: error.stack };
  try { await capture(active ?? space, 'failure', 'Failure state, not accepted evidence.', failure); }
  catch (captureError) { failure.captureError = captureError.message; }
} finally {
  await writeFile(`${config.evidence}/traffic.json`, JSON.stringify(traffic, null, 2) + '\n');
  await writeFile(`${config.evidence}/report.json`, JSON.stringify({ status: failure ? 'fail' : 'pass', failure, errors, steps,
    driver: { playwright: require(`${driver}/package.json`).version, chromium: browser.version() },
    limits: ['Native hooks/transcript fixtures only; no paid Provider/model interoperability', 'DOM/state/bitmap captures are sequential; no React private state or assistive-technology inspection'] }, null, 2) + '\n');
  await context.tracing.stop({ path: `${config.evidence}/trace.zip` });
  await context.close();
  await browser.close();
}
if (failure) { console.error(JSON.stringify(failure)); process.exitCode = 1; }
