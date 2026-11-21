/** Drive bounded actual workbench actions; scenario/effects stay native-owned. */
import assert from "node:assert/strict";
import { appendFileSync } from "node:fs";
import { createRequire } from "node:module";
import { mkdir, readFile, writeFile, realpath } from "node:fs/promises";
import { join } from "node:path";
import { checkpoint } from "./browser_checkpoint.mjs";

const [inputPath, evidence] = process.argv.slice(2);
assert(inputPath && evidence, "Supply explicit scenario input and new evidence directory");
const input = JSON.parse(await readFile(inputPath, "utf8"));
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? "/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright");
await mkdir(evidence, { recursive: false });
const browser = await chromium.launch({ executablePath: "/usr/bin/google-chrome", headless: true,
  args: ["--no-sandbox", "--disable-dev-shm-usage", "--no-proxy-server"] });
const cdp = await browser.newBrowserCDPSession();
const { processInfo } = await cdp.send("SystemInfo.getProcessInfo");
const ownedBrowser = [];
for (const process of processInfo) { try { const stat = await readFile(`/proc/${process.id}/stat`, "utf8"); ownedBrowser.push({ pid: process.id, type: process.type, start_ticks: stat.slice(stat.lastIndexOf(")") + 1).trim().split(/\s+/)[19], executable: await realpath(`/proc/${process.id}/exe`) }); } catch { /* A short-lived owned child may already be gone. */ } }
await writeFile(join(evidence, "owned-browser-processes.json"), JSON.stringify(ownedBrowser, null, 2));
await cdp.detach();
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, hasTouch: input.phase === "narrow-evidence" });
await context.tracing.start({ screenshots: true, snapshots: true, sources: false });
const page = await context.newPage();
const errors = []; const applies = []; const steps = [];
page.on("pageerror", error => errors.push(error.message));
page.on("request", request => { if (request.url().endsWith("/api/v1/apply")) { applies.push(request.postData()); appendFileSync(join(evidence, "apply-requests.jsonl"), `${JSON.stringify({ url: request.url(), body: request.postData() })}\n`); } });
page.setDefaultTimeout(15000);
function guardedRoute(handler) { return async route => { try { await handler(route); } catch (error) { await writeFile(join(evidence, "route-failure.json"), JSON.stringify({ message: error.message, stack: error.stack }, null, 2)); await route.abort("failed").catch(() => {}); } }; }
const serialize = value => JSON.stringify(value, (_key, item) => typeof item === "bigint" ? { $integer: item.toString() } : item, 2);
const decode = text => JSON.parse(text, (_key, item) => item && typeof item === "object" && Object.keys(item).length === 1 && "$integer" in item ? BigInt(item.$integer) : item);
async function native(route, value) {
  const response = await context.request.post(`${input.url}/api/v1/${route}`, {
    headers: { Origin: input.url, "Content-Type": "application/json" }, data: serialize(value),
  });
  assert.equal(response.status(), 200);
  return decode(await response.text());
}
async function settled() {
  await page.waitForFunction(() => document.querySelector(".collection-scroll")?.getAttribute("aria-busy") === "false");
}
async function clickApply(control) {
  const [response] = await Promise.all([
    page.waitForResponse(item => item.request().method() === "POST" && item.url().endsWith("/api/v1/apply")),
    control.click(),
  ]);
  assert.equal(response.status(), 200);
  const outcome = decode(await response.text()); assert(outcome.Applied, serialize(outcome));
  await page.waitForFunction(expected => { const text = document.querySelector(".detail-identity")?.textContent ?? ""; const match = text.match(/修订 (\d+)/); return text.includes(expected.id.slice(0, 8)) && match && BigInt(match[1]) >= BigInt(expected.revision); }, { id: outcome.Applied.reference.id, revision: String(outcome.Applied.revision) });
  await settled(); return outcome;
}
async function detail() {
  const response = await native("read", { generation: descriptor.generation, read: { Detail: workReference } });
  assert(response.Read?.Detail, serialize(response)); return response.Read.Detail;
}
let descriptor; let workReference;
try {
  const address = input.work ? `${input.url}/#work=${encodeURIComponent(input.work)}&tab=${input.tab ?? "brief"}` : `${input.url}/`;
  if (input.phase === "create") {
    await page.route("**/api/v1/connection", guardedRoute(async route => {
      const response = await route.fetch();
      await page.locator(".collection-scroll[aria-busy=true]").waitFor();
      await checkpoint(page, evidence, "initial-loading", "O11 actual initial connection response held; loading is distinct from empty");
      await route.fulfill({ response });
    }), { times: 1 });
  }
  await page.goto(address, { waitUntil: "networkidle" });
  if (input.phase === "diagnostic") {
    await page.getByRole("navigation", { name: "调查对象", exact: true }).waitFor();
    await page.waitForFunction(() => [...document.styleSheets].some(sheet => sheet.href?.endsWith("/app.css")));
    await checkpoint(page, evidence, "standalone-passive-diagnostic", "O12 original embedded diagnostic modules mount from the same standalone image");
    assert.equal(applies.length, 0); steps.push({ case: "O12-diagnostic", noMutation: true });
  } else {
  await settled();
  descriptor = await native("connection", {}); assert(descriptor.available);
  if (input.work) workReference = { scope: descriptor.scope, project: descriptor.project, generation: descriptor.generation, id: input.work };
  if (input.phase === "create") {
    const empty = await native("read", { generation: descriptor.generation, read: { List: { limit: 64n, after: null } } });
    assert.equal(empty.Read.List.items.length, 0);
    await checkpoint(page, evidence, "empty", "O1 empty actual scope; create entry visible", empty);
    await page.getByRole("button", { name: /新建任务/ }).click();
    await page.getByRole("button", { name: "保存任务", exact: true }).click();
    await page.locator("[aria-invalid=true]").first().waitFor(); assert.equal(applies.length, 0);
    await checkpoint(page, evidence, "invalid-draft", "O1 validation focuses invalid field; no native mutation");
    await page.getByLabel("标题", { exact: true }).fill(input.title);
    await page.getByLabel("目标", { exact: true }).fill(input.goal);
    for (let index = 0; index < input.criteria.length; index++) {
      if (index) await page.getByRole("button", { name: "＋ 条件", exact: true }).click();
      await page.getByLabel(`条件 ${index + 1}`, { exact: true }).fill(input.criteria[index]);
    }
    await page.getByLabel("来源引用（可选）", { exact: true }).fill("TASK.md");
    await page.getByLabel("所报来源修订", { exact: true }).fill(input.baselineCommit);
    for (const [label, name] of [["实现者", input.implementer], ["独立检查者", input.checker]]) {
      const picker = page.getByLabel(label, { exact: true });
      const options = await picker.locator("option").evaluateAll(items => items.map(item => ({ value: item.value, text: item.textContent, disabled: item.disabled })));
      const chosen = options.find(item => !item.disabled && item.text.includes(name)); assert(chosen, serialize(options));
      await picker.selectOption(chosen.value);
    }
    await checkpoint(page, evidence, "valid-draft", "O1 actual role selections, unsaved draft has no native identity");
    await page.route("**/api/v1/apply", guardedRoute(async route => {
      const response = await route.fetch();
      await page.locator(".local-notice[role=status]").filter({ hasText: "正在提交" }).waitFor();
      assert(await page.getByRole("button", { name: "保存任务", exact: true }).isDisabled());
      await checkpoint(page, evidence, "create-in-flight", "O1 actual save response held; pending feedback and duplicate-write prevention", decode(await response.text()));
      await route.fulfill({ response });
    }), { times: 1 });
    await clickApply(page.getByRole("button", { name: "保存任务", exact: true }));
    await page.locator(".work-row").waitFor(); await settled();
    const saved = await native("read", { generation: descriptor.generation, read: { List: { limit: 64n, after: null } } });
    assert.equal(saved.Read.List.items.length, 1); workReference = saved.Read.List.items[0].reference;
    const work = await detail(); assert.equal(work.work.messages.length, 0); assert.equal(work.work.results.length, 0);
    await checkpoint(page, evidence, "saved-before-dispatch", "O1 save creates work and bindings without dispatch", work);
    await clickApply(page.getByRole("button", { name: /^派发给 / }));
    const dispatched = await detail(); assert.equal(dispatched.work.messages.length, 1);
    assert.equal(dispatched.work.messages[0].receipt.recipients[0].canonical_name, input.implementer);
    steps.push({ case: "O1-O2", reference: workReference, saveWithoutDispatch: true, actualDispatch: dispatched });
  } else if (input.phase === "resume-create") {
    const saved = await detail(); assert.equal(saved.work.revision, 1n); assert.equal(saved.work.messages.length, 0); assert.equal(saved.work.results.length, 0);
    await checkpoint(page, evidence, "saved-before-dispatch", "O1 original save reconciled from native state after driver failure; no creation replay", saved);
    await page.route("**/api/v1/apply", guardedRoute(async route => {
      const response = await route.fetch();
      await page.locator(".local-notice[role=status]").filter({ hasText: "正在提交" }).waitFor();
      assert(await page.getByRole("button", { name: /^派发给 / }).isDisabled());
      await checkpoint(page, evidence, "dispatch-in-flight", "O2 actual native dispatch response held; pending state blocks another dispatch", decode(await response.text()));
      await route.fulfill({ response });
    }), { times: 1 });
    await clickApply(page.getByRole("button", { name: /^派发给 / }));
    const dispatched = await detail(); assert.equal(dispatched.work.messages.length, 1);
    assert.equal(dispatched.work.messages[0].receipt.recipients[0].canonical_name, input.implementer);
    steps.push({ case: "O1-O2", originalSavedWorkContinued: true, actualDispatch: dispatched });
  } else if (input.phase === "question") {
    const before = await detail(); assert(before.work.questions.some(question => !question.answer));
    const refresh = page.getByRole("button", { name: "刷新已载入工作", exact: true });
    await refresh.focus(); await page.getByRole("tooltip").waitFor(); await page.keyboard.press("Escape");
    assert.equal(await page.getByRole("tooltip").count(), 0);
    await page.getByLabel("答复内容", { exact: true }).fill(input.answer);
    await page.getByRole("tab", { name: "消息", exact: true }).click();
    await page.getByRole("tab", { name: "任务", exact: true }).click();
    assert.equal(await page.getByLabel("答复内容", { exact: true }).inputValue(), input.answer);
    await checkpoint(page, evidence, "desktop-question", "O3/O10 resources, exact question/recipient and retained draft", before);
    await page.setViewportSize({ width: 390, height: 844 });
    await checkpoint(page, evidence, "narrow-question", "S5 same task/question/draft and resource relationships at390x844", before);
    assert.equal(await page.getByLabel("答复内容", { exact: true }).inputValue(), input.answer);
    await clickApply(page.getByRole("button", { name: `回复 ${input.implementer}`, exact: true }));
    await page.reload({ waitUntil: "networkidle" }); await settled();
    const after = await detail(); assert(after.work.questions.some(question => question.answer?.text === input.answer));
    await checkpoint(page, evidence, "narrow-answer-return", "O3 saved answer survives reload; explicit recipient/message receipt", after);
    steps.push({ case: "O3-O10", draftRetained: true, native: after });
  } else if (input.phase === "readonly") {
    assert.equal(descriptor.collaboration, false);
    await page.locator(".status-label").filter({ hasText: "只读" }).waitFor();
    await page.getByRole("button", { name: "编辑依据 / 分工", exact: true }).first().click();
    assert(await page.getByRole("button", { name: "保存依据", exact: true }).isDisabled());
    await checkpoint(page, evidence, "readonly-existing-work", "O11 actual read-only host retains work; local editor cannot write", await detail());
    assert.equal(applies.length, 0); steps.push({ case: "O11-readonly", noMutation: true });
  } else if (input.phase === "filter-refresh") {
    const before = await detail(); const count = await page.locator(".work-row").count(); assert(count > 0);
    await page.getByRole("searchbox", { name: "搜索已载入资源", exact: true }).fill("no-matching-fixture-resource");
    await page.getByText("当前条件下没有已载入记录", { exact: true }).waitFor();
    assert.equal(await page.locator(".work-row").count(), 0);
    assert((await page.locator(".detail-identity").innerText()).includes(before.work.reference.id.slice(0, 8)));
    await checkpoint(page, evidence, "filter-empty-retains-selection", "O11 filtered-empty is local; selected real work remains", before);
    await page.getByRole("button", { name: "清除条件", exact: true }).click();
    await page.route("**/api/v1/read", route => route.abort("failed"), { times: 1 });
    await page.getByRole("button", { name: "刷新已载入工作", exact: true }).click();
    await page.getByRole("alert").filter({ hasText: "保留" }).waitFor();
    assert.equal(await page.locator(".work-row").count(), count);
    await checkpoint(page, evidence, "refresh-failure-retains-content", "O11 one explicitly dropped read; error retains historical data/time rather than empty", before);
    await page.getByRole("button", { name: "再试", exact: true }).click(); await settled();
    await page.getByRole("alert").filter({ hasText: "保留" }).waitFor({ state: "detached" });
    assert.equal(applies.length, 0); steps.push({ case: "O11-filter-refresh", retainedCount: count });
  } else if (input.phase === "rebind-saved") {
    const before = await detail();
    await page.getByRole("button", { name: "编辑依据 / 分工", exact: true }).first().click();
    const roles = page.locator(".role-bindings");
    await roles.locator(".binding-row").first().getByRole("button", { name: "更换", exact: true }).click();
    const picker = page.getByLabel("更换实现者", { exact: true });
    const options = await picker.locator("option").evaluateAll(items => items.map(item => ({ value: item.value, text: item.textContent, disabled: item.disabled })));
    const chosen = options.find(item => !item.disabled && item.text.includes(input.implementer)); assert(chosen, serialize(options));
    await picker.selectOption(chosen.value);
    await roles.getByRole("button", { name: "应用本角色", exact: true }).click();
    await page.getByRole("dialog", { name: "确认更换角色" }).waitFor();
    await clickApply(page.getByRole("button", { name: "确认更换", exact: true }));
    const after = await detail();
    const bound = after.work.assignments.find(item => item.id === after.work.implementer);
    assert(chosen.value.includes(String(bound.participant.incarnation_bits)));
    assert.equal(after.work.messages.length, before.work.messages.length);
    await checkpoint(page, evidence, "explicit-rebind-without-dispatch", "O9 preparation: saved secondary work explicitly binds current actor; no Provider work dispatched", after);
    steps.push({ case: "O9-role-rebind", native: after });
  } else if (input.phase === "unavailable") {
    const before = await detail();
    const candidates = await native("read", { generation: descriptor.generation, read: { Candidates: { limit: 64n, after: null } } });
    const missing = candidates.Read.Candidates.items.find(item => item.participant.canonical_name === input.implementer);
    assert(!missing || missing.unavailable, serialize(candidates));
    const [response] = await Promise.all([page.waitForResponse(item => item.url().endsWith("/api/v1/apply")), page.getByRole("button", { name: /^派发给 / }).click()]);
    const rejected = decode(await response.text()); assert(rejected.Rejected, serialize(rejected));
    await page.getByRole("alert").waitFor();
    const after = await detail(); assert.equal(after.work.revision, before.work.revision); assert.equal(after.work.messages.length, before.work.messages.length);
    await checkpoint(page, evidence, "unavailable-dispatch-rejected", "O9 actual stopped collaborator; saved work retained and no message admitted", after);
    await page.getByRole("button", { name: "编辑依据 / 分工", exact: true }).first().click();
    assert((await page.locator(".role-bindings .binding-row").first().innerText()).includes(input.implementer));
    await checkpoint(page, evidence, "unavailable-bound-identity", "O9 actual candidate reason visible locally; exact prior identity retained", candidates);
    steps.push({ case: "O9", noNamesakeSubstitution: true, native: after });
  } else if (input.phase === "narrow-evidence") {
    const current = await detail(); assert(current.work.checks.length >= 2);
    await page.getByRole("tab", { name: "检查", exact: true }).click();
    await checkpoint(page, evidence, "desktop-result-check", "O4-O6 actual current result/check/criterion/decision relationships", current);
    await page.setViewportSize({ width: 390, height: 844 });
    const refresh = page.getByRole("button", { name: "刷新已载入工作", exact: true });
    await refresh.tap(); await settled();
    await page.getByRole("tooltip").waitFor();
    await page.keyboard.press("Escape");
    assert.equal(await page.getByRole("tooltip").count(), 0);
    assert(await refresh.evaluate(element => element === document.activeElement));
    const geometry = await page.evaluate(() => {
      const heading = document.querySelector(".evidence-block > .panel-heading");
      const picker = heading?.querySelector("select");
      const identity = document.querySelector(".work-row.selected .work-identity");
      const box = element => element && ({ x: element.getBoundingClientRect().x, right: element.getBoundingClientRect().right, width: element.getBoundingClientRect().width, height: element.getBoundingClientRect().height });
      return { viewport: innerWidth, documentWidth: document.documentElement.scrollWidth, heading: box(heading), picker: box(picker), identity: box(identity) };
    });
    assert(geometry.documentWidth <= geometry.viewport, serialize(geometry));
    assert(geometry.heading.height <= 100 && geometry.picker.right <= geometry.viewport, serialize(geometry));
    assert(geometry.identity.width >= 120, serialize(geometry));
    await checkpoint(page, evidence, "narrow-result-check", "O10 touch refresh + Tooltip/Escape keeps selected work and independent evidence", current);
    const criterion = page.locator(".criterion-reports summary").first();
    await criterion.tap();
    assert(await criterion.evaluate(element => element.parentElement.open));
    await checkpoint(page, evidence, "narrow-criterion-disclosure", "O10 actual criterion verdict/evidence is reachable in the current work", current);
    await page.getByRole("tab", { name: "产物", exact: true }).tap();
    const reference = page.locator(".result-summary .reference-disclosure summary").filter({ hasText: "export_orders.py" }).first();
    await reference.tap();
    assert(await reference.evaluate(element => element.parentElement.open));
    await checkpoint(page, evidence, "narrow-artifact-disclosure", "O10 touch opens exact artifact reference in current work without route loss", current);
    assert.equal(applies.length, 0); steps.push({ case: "O10-results-touch", noMutation: true });
  } else if (input.phase === "history") {
    const current = await detail(); assert(current.eligibility.closed); assert(current.work.results.length >= 2);
    await page.getByRole("tab", { name: "检查", exact: true }).click();
    const first = current.work.results[0]; await page.getByRole("combobox", { name: "结果版本", exact: true }).selectOption(first.id);
    assert(await page.getByRole("button", { name: `接受 ${first.id}`, exact: true }).isDisabled());
    await checkpoint(page, evidence, "retained-failed-baseline", "O4-O6 historical immutable baseline and failed check remain distinct after accepted repair", current);
    assert.equal(applies.length, 0); steps.push({ case: "O4-O6-history", native: current });
  } else if (input.phase === "conflict") {
    const before = await detail();
    await page.getByRole("button", { name: "编辑依据 / 分工", exact: true }).first().click();
    await page.getByLabel("标题", { exact: true }).fill(input.localTitle);
    const brief = structuredClone(before.work.briefs.at(-1).input); brief.goal = input.serverGoal;
    const external = await native("apply", { generation: descriptor.generation,
      command_id: `fixture-${Date.now()}-conflict`, work: workReference,
      expected_revision: before.work.revision, command: { UpdateBrief: brief } });
    assert(external.Applied, serialize(external));
    await page.getByRole("button", { name: "刷新已载入工作", exact: true }).click(); await settled();
    await page.getByRole("button", { name: "查看差异", exact: true }).waitFor();
    assert.equal(await page.getByLabel("标题", { exact: true }).inputValue(), input.localTitle);
    await checkpoint(page, evidence, "conflict-retains-draft", "O7 real second operator changes brief while local fields remain", await detail());
    await page.getByRole("button", { name: "查看差异", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "任务已更新 · 字段对比" });
    await dialog.getByRole("group", { name: "标题", exact: true }).getByRole("radio", { name: /保留本地/ }).check();
    await dialog.getByRole("group", { name: "目标", exact: true }).getByRole("radio", { name: /使用服务器/ }).check();
    await dialog.getByRole("button", { name: "基于更新继续编辑", exact: true }).click();
    await clickApply(page.getByRole("button", { name: "保存依据", exact: true }));
    const after = await detail(); assert.equal(after.work.briefs.at(-1).input.title, input.localTitle);
    assert.equal(after.work.briefs.at(-1).input.goal, input.serverGoal);
    steps.push({ case: "O7", realSecondOperator: true, perFieldRebase: true, native: after });
  } else if (input.phase === "unknown") {
    await page.getByRole("tab", { name: "消息", exact: true }).click();
    const before = await detail();
    await page.locator(".recipient-options input[type=checkbox]").first().check();
    await page.getByLabel("补充内容", { exact: true }).fill(input.note);
    let dropped;
    const routeHandler = async route => {
      const response = await route.fetch();
      dropped = { command: route.request().postData(), status: response.status(), body: await response.text() };
      await writeFile(join(evidence, "committed-dropped-response.json"), `${serialize(dropped)}\n`);
      await route.abort("failed");
    };
    await page.route("**/api/v1/apply", guardedRoute(routeHandler), { times: 1 });
    await page.getByRole("button", { name: "发送补充", exact: true }).click();
    await page.locator(".status-label").filter({ hasText: "结果待确认" }).waitFor();
    assert.equal(dropped.status, 200); assert(decode(dropped.body).Applied);
    await checkpoint(page, evidence, "unknown-after-commit", "O8 actual native write committed; only browser delivery was dropped", await detail());
    assert.equal(applies.length, 1);
    await page.reload({ waitUntil: "networkidle" }); await settled();
    await page.locator(".status-label").filter({ hasText: "结果待确认" }).waitFor();
    assert.equal(await page.getByRole("button", { name: "按原操作重试", exact: true }).count(), 0);
    await page.getByRole("button", { name: "查询原操作", exact: true }).click();
    await page.locator(".status-label").filter({ hasText: "结果待确认" }).waitFor({ state: "detached" });
    const after = await detail();
    assert.equal(after.work.messages.length, before.work.messages.length + 1);
    assert.equal(after.work.questions.filter(question => !question.answer).length, before.work.questions.filter(question => !question.answer).length);
    assert.equal(applies.length, 1);
    await checkpoint(page, evidence, "reconciled-after-return", "O8 reload keeps original identity only; command lookup confirms one effect", after);
    steps.push({ case: "O8", actualCommittedReplyDropped: true, noDuplicateApply: true, noQuestionAnsweredByNote: true, native: after });
  } else if (["request-check", "return"].includes(input.phase)) {
    await page.getByRole("tab", { name: "检查", exact: true }).click();
    const before = await detail();
    await checkpoint(page, evidence, "result-before-action", "O4/O5 immutable result and actual independent findings", before);
    await page.getByRole("button", { name: input.phase === "return" ? "要求修改" : "请求独立检查", exact: true }).click();
    await page.getByLabel(input.phase === "return" ? "修改理由" : "请求检查理由", { exact: true }).fill(input.reason);
    await clickApply(page.getByRole("button", { name: input.phase === "return" ? "发送修改要求" : "发送检查请求", exact: true }));
    const after = await detail(); steps.push({ case: input.phase === "return" ? "O5" : "O4", native: after });
  } else if (input.phase === "accept") {
    await page.getByRole("tab", { name: "检查", exact: true }).click();
    const before = await detail(); assert(before.eligibility.selection); assert(!before.eligibility.closed);
    await page.getByRole("button", { name: /^接受 / }).click();
    await checkpoint(page, evidence, "acceptance-confirmation", "O6 exact current result/check/brief and consequence before decision", before);
    await page.getByRole("button", { name: "取消", exact: true }).click();
    const cancelled = await detail(); assert.equal(cancelled.work.decisions.length, before.work.decisions.length);
    await page.getByRole("button", { name: /^接受 / }).click();
    if (input.staleProbe) {
      const external = await native("apply", { generation: descriptor.generation,
        command_id: `fixture-${Date.now()}-acceptance-revision`, work: workReference, expected_revision: cancelled.work.revision,
        command: { SendNote: { roles: ["Checker"], text: "Operator consistency probe only. The completed check remains unchanged; no new check or report is requested. Await explicit operator action." } } });
      assert(external.Applied, serialize(external));
      const [response] = await Promise.all([page.waitForResponse(item => item.url().endsWith("/api/v1/apply")), page.getByRole("button", { name: "确认接受", exact: true }).click()]);
      const stale = decode(await response.text()); assert(stale.Rejected?.RevisionConflict, serialize(stale));
      await page.getByText("任务已更新，请关闭确认并查看最新依据。", { exact: true }).waitFor();
      const unchanged = await detail(); assert(!unchanged.eligibility.closed); assert.equal(unchanged.work.decisions.length, cancelled.work.decisions.length);
      await checkpoint(page, evidence, "stale-acceptance-rejected", "O7 real concurrent revision rejects old captured confirmation; no acceptance before explicit fresh decision", unchanged);
      assert(await page.getByRole("button", { name: "确认接受", exact: true }).isDisabled());
      await page.getByRole("button", { name: "取消", exact: true }).click();
      await page.getByRole("button", { name: /^接受 / }).click();
      steps.push({ case: "O7-stale-acceptance", realSecondOperatorRevision: true, oldDecisionRejected: true });
    }
    await clickApply(page.getByRole("button", { name: "确认接受", exact: true }));
    const after = await detail(); assert(after.eligibility.closed);
    assert.equal(after.work.decisions.at(-1).result_id, before.eligibility.selection.result_id);
    assert.equal(after.work.decisions.at(-1).check_id, before.eligibility.selection.check_id);
    await checkpoint(page, evidence, "accepted", "O6 explicit operator acceptance closes exact result/check/brief", after);
    steps.push({ case: "O6", cancelHadNoEffect: true, native: after });
  } else throw new Error(`Unsupported bounded phase ${input.phase}`);
  }
  assert.deepEqual(errors, []);
  await writeFile(join(evidence, "result.json"), `${serialize({ input, browser: browser.version(), descriptor, reference: workReference, applies, steps, errors })}\n`);
  console.log(serialize({ phase: input.phase, reference: workReference, steps: steps.length }));
} catch (error) {
  await writeFile(join(evidence, "failure.json"), `${serialize({ message: error.message, stack: error.stack, input, applies, steps, errors })}\n`);
  await page.screenshot({ path: join(evidence, "failure.png"), fullPage: false }).catch(() => {});
  throw error;
} finally {
  await context.tracing.stop({ path: join(evidence, "trace.zip") });
  await browser.close();
  await writeFile(join(evidence, "browser-cleanup.json"), JSON.stringify({ closed: true, ownedBrowser }, null, 2));
}
