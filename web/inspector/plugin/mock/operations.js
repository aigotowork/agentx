import { coreStats, coreTokens, coreDigest } from "./core-projections";
import { botsFixture } from "./bots-fixture";
import { diagnosticsFixture } from "./diagnostics-fixture";
import { analysisEvidenceFixture } from "./analysis-evidence-fixture";
import { recoveryFixture } from "./recovery-fixture";
import { previewLanguage, setPreviewLanguage } from "./i18n";
import { fixtureStats } from "./stats-fixture";
import { statsBasisFixture, tokenLedgerFixture } from "./data-basis-fixture";
import { metricComparisonFixture } from "./metric-comparison-fixture";
import { seedInsightJobs, fixtureInsightJobsSnapshot, cancelFixtureInsightJob } from "./insight-jobs-fixture";
import { createFixtureLedger } from "./messages-fixture";
import { quickReplyGroup, quickReplyGuard, quickReplyKey } from "../client/message-state";
import { fixtureTokenLive, fixtureTokens } from "./tokens-fixture";
import { fixtureDigest } from "./digest-fixture";
import { fixtureRecapList, fixtureRecapSettings, fixtureRecapStatus, saveRecapSettings, subscribeRecapSettings } from "./recap-fixture";
import { fixtureModels, fixtureOverview, fixtureProfiles, fixtureReportJobs, fixtureRunners, fixtureSuggestions, fixtureSuggestionsState, richInsightScenarios, runnerLabelFor } from "./insights-overview-fixture";




import { defaultViewPreferences } from "../shared/preferences";
import { projectTimelineRows } from "../shared/timeline";
import { normalizeAgentTimelinePage } from "../shared/agentx-timeline";
import { fixtureAnalytics, fixtureArchiveOrigins, fixtureInsight, fixtureInsightState, fixtureProjectAnalytics, fixtureReadReceipts, fixtureTelemetry, fixtureTimeline, agentTimelinePayload, longFirstInput, longLatestReply, longFixtureTimeline, nativeChildren, nativeTimeline, rawAgents, subscribeFixtureAgents, updateFixtureAgent, applyAttentionScenario, fixtureLiveExtras, ackFixtureStop } from "./fixtures";

let insightSequence = 0;
let fixtureRevision = 0;
const revisioned = new Set(["observatory.telemetry.get", "observatory.analytics.get", "observatory.read-state.list", "observatory.results.get", "observatory.stats.get", "observatory.tokens.get", "observatory.digest.get", "observatory.insights.overview"]);
let preferencesRevision=0;const preferencePages=new Map();let preferenceSnoozes=[];let preferenceSettings={childCompletions:false};
// ?storedQuery= simulates search text synced by an older version (the server now strips it; the client must ignore it too).
const storedQuery=typeof location==="undefined"?null:new URLSearchParams(location.search).get("storedQuery");
const preferenceSnapshot=(key)=>structuredClone({revision:preferencesRevision,view:preferencePages.get(key)?.view??{...defaultViewPreferences(key),...(storedQuery&&!key.endsWith("archives")?{query:storedQuery}:{})},savedViews:preferencePages.get(key)?.savedViews??[],snoozes:preferenceSnoozes.filter(s=>Date.parse(s.until)>Date.now()),error:null,settings:{...preferenceSettings}});
globalThis.__inspectorMockRpcCalls = [];
globalThis.__inspectorMockAgentOperations = [];
globalThis.__inspectorMockTimelineCalls = [];
const timelineListeners = new Map();
globalThis.__inspectorMockGetPreferences = preferenceSnapshot;
globalThis.__inspectorMockGetAgent = id => structuredClone(rawAgents.find(agent => agent.id === id));
globalThis.__inspectorMockTimelineActive = id => timelineListeners.get(id)?.size ?? 0;
globalThis.__inspectorMockEmitTimeline = (id,event) => {for(const listener of timelineListeners.get(id)??[])listener({agentId:id,event});};
const scenarioName=()=>new URLSearchParams(globalThis.location?.search??"").get("scenario");
if(scenarioName()==="attention-states")applyAttentionScenario();
/**
 * ?scenario=approvals: in-place approvals (opus-B12) and quick replies (opus-B26) with the host's real request shapes. 审查数据库迁移
 * (Claude) waits on a shell command with no provider actions and a question; 同步订单表结构 (an ACP provider) on an edit with the
 * agent's own choices, one of them lasting. 规划下一轮迭代 asks whether to go on, 整理性能基线 ended on its next step.
 */
function applyApprovalScenario() {
  const now = Date.now(), at = seconds => new Date(now - seconds * 1000).toISOString();
  const review = rawAgents.find(agent => agent.id === "migration-review");
  // Two more after them (codex-obs-send-ui-F1): a command too long to show collapsed, whose end 展开完整内容 brings back, and a Grep
  // whose pattern is its target and whose path is a parameter of its own.
  const tables = `npm run migrate:check -- --dry-run --tables ${Array.from({ length: 450 }, (_, index) => `orders_${String(index).padStart(3, "0")}`).join(",")} && echo 检查结束`;
  review.pendingPermissions = [{ id: "permission-1", provider: "claude", kind: "tool", name: "Bash", title: "执行数据库迁移检查", description: "运行只读检查并核对迁移计划。",
    input: { command: "npm run migrate:check -- --dry-run --env staging", description: "只读检查迁移计划" }, detail: { type: "shell", command: "npm run migrate:check -- --dry-run --env staging" } }, review.pendingPermissions[1],
    { id: "permission-3", provider: "claude", kind: "tool", name: "Bash", title: "检查全部订单分表", input: { command: tables, description: "逐个分表只读检查" }, detail: { type: "shell", command: tables } },
    { id: "permission-4", provider: "claude", kind: "tool", name: "Grep", title: "Grep", input: { pattern: "TODO", path: "db/migrations", output_mode: "content" }, detail: { type: "search", query: "TODO" } }];
  const options = [{ optionId: "allow", name: "Allow", kind: "allow_once" }, { optionId: "allow_always", name: "Always Allow", kind: "allow_always" }, { optionId: "reject", name: "Reject", kind: "reject_once" }];
  rawAgents.splice(5, 0, { ...structuredClone(review), id: "schema-sync", title: "同步订单表结构", provider: "my-acp", model: null, activeTurn: { turnId: "schema-turn", startedAt: at(150) }, updatedAt: at(9),
    pendingPermissions: [{ id: "permission-acp-1", provider: "my-acp", kind: "tool", name: "edit", title: "Edit db/migrations/0042_orders.sql",
      detail: { type: "edit", filePath: "/workspace/commerce-platform/db/migrations/0042_orders.sql", unifiedDiff: "@@ -1,3 +1,4 @@\n CREATE TABLE orders (\n+  tenant_id TEXT NOT NULL,\n   id TEXT PRIMARY KEY," },
      actions: options.map(option => ({ id: option.optionId, label: option.name, behavior: option.kind.startsWith("allow") ? "allow" : "deny" })), metadata: { toolCallId: "acp-call-1", options } }] });
  const telemetry = id => fixtureTelemetry.agents.find(record => record.agentId === id);
  Object.assign(telemetry("iteration-plan"), { replyNudge: "go-ahead" });
  const baseline = telemetry("performance-baseline");
  Object.assign(baseline, { replyAt: baseline.turnEndedAt, replyNudge: "keep-going" });
}
// The quick replies of ?scenario=approvals, sent through a ledger whose host never tells the outcome (send-unknown) or whose answer is lost.
if(["approvals","send-unknown","send-lost-reply"].includes(scenarioName()))applyApprovalScenario();
// ?scenario=deferred: 重构支付服务 runs on a provider that cannot take a message mid-turn, so a send to it would interrupt the turn.
// ?scenario=deferred-held: the same, and the host never tells whether a send arrived (a message queued once the turn ended is held).
if(["deferred","deferred-held"].includes(scenarioName()))Object.assign(rawAgents.find(agent=>agent.id==="payment-root"),{provider:"my-acp",activeTurn:{turnId:"payment-turn",startedAt:rawAgents.find(agent=>agent.id==="payment-root").activeTurn.startedAt}});
// The daemon's message receipts (message-receipts/index.js): a retry of a delivered id succeeds without a second copy, a changed retry is refused.
const deliveredMessages=new Map();const flakyMessages=new Set();
if(richInsightScenarios.has(scenarioName()))fixtureInsightState.jobs.push(...fixtureReportJobs());
if (scenarioName() === "analysis-evidence") fixtureInsightState.jobs.push(analysisEvidenceFixture(fixtureInsight("payment-root"), new URLSearchParams(location.search).get("analysisCase") ?? "known"));
fixtureInsightState.jobs.push(...seedInsightJobs(fixtureInsight));
let createdSequence=0;

const sdk = {
  agents: {
    create: async input => { globalThis.__inspectorMockAgentOperations.push({ type: "create", input }); throw new Error("开发预览不允许创建 Agent"); },
    list: async (options) => ({ entries: rawAgents.filter(agent => options?.filter?.includeArchived || !agent.archivedAt).map(agent => ({ agent: { ...agent } })), pageInfo: { nextCursor: null, hasMore: false } }),
    subscribe: subscribeFixtureAgents,
    ref: agentId => ({ current: () => rawAgents.find(agent => agent.id === agentId) ?? null, refresh: async () => { const agent = rawAgents.find(agent => agent.id === agentId); return agent ? { agent: { ...agent }, project: null } : null; },
      archive: async () => {const agent=rawAgents.find(item=>item.id===agentId);if(!agent)throw new Error("模拟会话不存在");const archivedAt=new Date().toISOString();globalThis.__inspectorMockAgentOperations.push({type:"archive",agentId});const index=fixtureArchiveOrigins.findIndex(item=>item.agentId===agentId);if(index>=0)fixtureArchiveOrigins.splice(index,1);fixtureArchiveOrigins.push({agentId,archivedAt,lastUserMessageAt:agent.lastUserMessageAt??null});updateFixtureAgent(agentId,{archivedAt,updatedAt:archivedAt,status:"closed",requiresAttention:false,attentionReason:null,attentionTimestamp:null});return {archivedAt};},
      send: async (input, options) => {
        globalThis.__inspectorMockAgentOperations.push({ type: "send", agentId, input, ...(options ? { options } : {}) });
        const scenario = scenarioName(), key = options?.messageId;
        if (scenario === "send-error") throw new Error("开发预览不允许发送新提示词");
        if (key) {
          const fingerprint = JSON.stringify([agentId, input, options.activeTurnBehavior ?? "interrupt"]), previous = deliveredMessages.get(key);
          if (previous) { if (previous !== fingerprint) throw new Error("agent_request_key_conflict"); return; }
          // ?scenario=send-unknown: the daemon cannot tell whether it went out; ?scenario=send-flaky: the first try of each message is lost.
          if (scenario === "send-unknown" || scenario === "deferred-held") throw new Error("agent_request_outcome_unknown");
          if (scenario === "send-flaky" && !flakyMessages.has(key)) { flakyMessages.add(key); throw new Error("模拟网络中断，这次没有送达"); }
          deliveredMessages.set(key, fingerprint);
        }
        // Like the host: every send clears the pending requests, and a message to a stopped Agent starts a turn.
        const agent = rawAgents.find(item => item.id === agentId), now = new Date().toISOString();
        if (!agent) throw new Error("模拟 Agent 不存在");
        if (agent) updateFixtureAgent(agentId, { pendingPermissions: [], updatedAt: now, ...(["running", "initializing"].includes(agent.status) ? {} : { status: "running", activeTurn: { turnId: `preview-${key ?? now}`, startedAt: now }, lastUserMessageAt: now }) });
      },
      // Fire and forget like the host (no receipt): the request disappears a moment later; 拒绝并中断 also ends the turn.
      // ?scenario=permission-stuck: the host never drops it, so the panel has to say it is unconfirmed.
      respondToPermission: async ({ requestId, response }) => {
        globalThis.__inspectorMockAgentOperations.push({ type: "permission", agentId, requestId, response: structuredClone(response) });
        if (scenarioName() === "permission-stuck" || !rawAgents.find(item => item.id === agentId)?.pendingPermissions.some(request => request.id === requestId)) return;
        setTimeout(() => {
          const agent = rawAgents.find(item => item.id === agentId);
          if (!agent) return;
          const interrupt = response?.behavior === "deny" && response.interrupt === true;
          updateFixtureAgent(agentId, { pendingPermissions: agent.pendingPermissions.filter(request => request.id !== requestId), updatedAt: new Date().toISOString(), ...(interrupt ? { status: "idle", activeTurn: null } : {}) });
        }, 400);
      }, timeline: {
      refetch: async (input = {}) => {globalThis.__inspectorMockTimelineCalls.push({agentId,...input});return agentTimelinePayload(agentId, scenarioName()==="long-history" ? longFixtureTimeline(agentId,input) : fixtureTimeline(agentId,input.direction), input.direction ?? "tail");},
      subscribe: listener => {const listeners=timelineListeners.get(agentId)??new Set();listeners.add(listener);timelineListeners.set(agentId,listeners);return Object.assign(()=>listeners.delete(listener),{ready:Promise.resolve()});},
    } }),
  },
};
sdk.config = { get: async () => ({ requestId: "fixture-config", config: { agentProfiles: fixtureProfiles } }) };
// The plugin server's ledger for observatory.deferred.* (server/deferred-send.ts there), over the synthetic Agents: it sends with the
// stand-in host's send above, so every delivery is in __inspectorMockAgentOperations. ?scenario=send-lost-reply: the first answer to each
// send never reaches the card (an RPC timeout), though the ledger sent it.
const instant = value => { const at = value ? Date.parse(value) : NaN; return Number.isFinite(at) ? at : null; };
/**
 * The server's check of a quick reply's turn (`answeringGuard`, server/send-ledger.ts there), over the fixture's telemetry: a
 * round its record cannot vouch for (an error reported, no round of this Agent, or an older one) throws.
 */
const answeringGuard = (agentId, guard, agent) => {
  const title = agent.title || "这个 Agent";
  if (instant(agent.lastUserMessageAt) !== instant(guard.prompt) || agent.activeTurn || agent.status === "running") return `「${title}」已经开始新的一轮，这条快捷回复不再适用`;
  if (agent.status !== "idle" && agent.status !== "closed") return `「${title}」的状态变了，这条快捷回复不再适用`;
  const round = instant(guard.round);
  if (round === null) return null;
  if (fixtureTelemetry.persistenceError) throw new Error(`观测台的记录暂时有问题，核实不了「${title}」最近一轮`);
  const seen = instant(fixtureTelemetry.agents.find(record => record.agentId === agentId)?.turnStartedAt);
  if (seen === null || seen < round) throw new Error(`观测台读不到「${title}」最近一轮的记录`);
  return seen > round ? `「${title}」之后又跑了一轮，这条快捷回复答的是之前那一轮，不再适用` : null;
};
const fixtureLedger = createFixtureLedger({ agent: id => { const agent = rawAgents.find(item => item.id === id); return agent ? { ...agent } : null; },
  deliver: (agentId, text, options) => sdk.agents.ref(agentId).send(text, options), capabilities: { steerable: ["claude", "codex"], slashSteerable: ["claude"], bases: {} }, guard: answeringGuard });
/** Another device taps the quick reply `text` for this Agent's latest turn, through the same ledger (codex-obs-attn-R3). */
globalThis.__inspectorMockOtherDeviceReply = async (agentId, text) => {
  const agent = rawAgents.find(item => item.id === agentId), round = fixtureTelemetry.agents.find(record => record.agentId === agentId)?.turnStartedAt ?? null;
  const words = `${text}。`, prompt = agent?.lastUserMessageAt ?? null;
  return fixtureLedger.send({ agentId, text: words, messageId: `other-device-${Date.now()}`, key: quickReplyKey(words, prompt, round), source: `快捷回复「${text}」`, what: "回复", consent: null,
    group: quickReplyGroup(prompt, round), guard: quickReplyGuard(prompt, round) });
};
/** The plugin's store reports an error (`message`; null clears it): its record may be behind (codex-obs-attn-R3-remaining). */
globalThis.__inspectorMockTelemetryError = message => { fixtureTelemetry.persistenceError = message; };
const lostReplies = new Set();
const createdKeys=new Map();
sdk.workspaces = {ref:workspaceId=>({agents:{create:async input=>{
 const key=input.idempotencyKey, fingerprint=JSON.stringify([workspaceId,input]);
 if(key&&createdKeys.has(key)){const previous=createdKeys.get(key);if(previous.fingerprint!==fingerprint)throw new Error('agent_request_key_conflict');return {id:previous.id};}
 const base=rawAgents.find(agent=>agent.workspaceId===workspaceId);if(!base)throw new Error('模拟工作区不存在');
 const id=`mock-created-${++createdSequence}`,at=new Date().toISOString();
 const model=String(input.config?.provider??'codex/default').split('/');
 rawAgents.push({...structuredClone(base),id,title:input.title??'模拟 Agent',provider:model[0],model:model.slice(1).join('/')||null,status:'idle',activeTurn:null,parentAgentId:null,pendingPermissions:[],requiresAttention:false,attentionReason:null,attentionTimestamp:null,lastError:null,createdAt:at,updatedAt:at,archivedAt:null,lastUserMessageAt:at,labels:{...input.labels},previewPrompt:input.prompt,previewReply:'本地模拟对象已创建，尚未模拟执行。'});
 updateFixtureAgent(id,{});if(key)createdKeys.set(key,{id,fingerprint});
 globalThis.__inspectorMockAgentOperations.push({type:'create',workspaceId,input,id});return {id};
 }}})};
sdk.providers = { snapshot: async () => ({entries: [{provider:"codex",available:true,configured:true,displayName:"Codex · 模拟"},{provider:"claude",available:true,configured:true,displayName:"Claude · 模拟"}]}) };
const listSnapshot = sdk.agents.list;
sdk.agents.list = async options => {
  const snapshot = await listSnapshot(options);
  if (!options?.subscribe) return snapshot;
  return {...snapshot, subscription: {subscribe(observer) {observer.snapshot({}); return subscribeFixtureAgents(message => observer.update(message));}, async release() {}}};
};
export { sdk as mockAgentX };
export async function invokeFixture(contract, input) {
    globalThis.__inspectorMockRpcCalls.push({ name: contract.name, input: { ...input } });
    let result;
    switch (contract.name) {
      case "observatory.bots.snapshot": result = botsFixture(); break;
      case "observatory.diagnostics.snapshot": {
        const variant = new URLSearchParams(location.search).get("diagnosticsCase") ?? "healthy";
        if (variant === "error") throw new Error("fixture diagnostic unavailable");
        result = diagnosticsFixture(variant); break;
      }
      case "recovery.continue":
      case "recovery.list":
      case "recovery.configure":
      case "recovery.manage":
        if (scenarioName() === "recovery-error") throw new Error("模拟目录读取失败");
        result = await recoveryFixture(contract.name, input); break;
      case "i18n.languages": result = { setting: previewLanguage, resolved: previewLanguage, system: "zh-CN", device: "zh-CN", folder: "/fixture/plugin-data/agent-observatory/locales", languages: [{ code: "zh-CN", name: "简体中文", builtIn: true, custom: false, coverage: 1, issues: [] }, { code: "en", name: "English", builtIn: true, custom: false, coverage: 1, issues: [] }] }; break;
      case "observatory.preferences.get": result=preferenceSnapshot(input.viewKey);break;
      case "observatory.messages.send": {
        result = await fixtureLedger.send(input);
        if (scenarioName() === "send-lost-reply" && !lostReplies.has(input.messageId)) { lostReplies.add(input.messageId); throw new Error("模拟回包丢失：请求超时"); }
        break;
      }
      case "observatory.messages.list": result = fixtureLedger.list(input); break;
      case "observatory.preferences.update": {
        if(input.expectedRevision!==preferencesRevision){result={...preferenceSnapshot(input.viewKey),status:"conflict"};break;}
        const old=preferenceSnapshot(input.viewKey);const value={view:{...old.view,...input.view},savedViews:[...old.savedViews]};
        if(input.saveView){const id=input.saveView.id??`view-${Date.now()}`;value.savedViews=[...value.savedViews.filter(v=>v.id!==id),{...input.saveView,id,createdAt:new Date().toISOString(),updatedAt:new Date().toISOString()}];}
        if(input.deleteSavedViewId)value.savedViews=value.savedViews.filter(v=>v.id!==input.deleteSavedViewId);
        if(input.snooze)preferenceSnoozes=[...preferenceSnoozes.filter(s=>s.agentId!==input.snooze.agentId||Date.parse(s.completionAt)!==Date.parse(input.snooze.completionAt)),input.snooze];
        if(input.unsnooze)preferenceSnoozes=preferenceSnoozes.filter(s=>s.agentId!==input.unsnooze.agentId||s.completionAt!==input.unsnooze.completionAt);
        if(input.settings)preferenceSettings={...preferenceSettings,...input.settings};
        preferencePages.set(input.viewKey,value);preferencesRevision++;result={...preferenceSnapshot(input.viewKey),status:"updated"};break;
      }
      case "observatory.results.get": {
        const page=fixtureTimeline(input.agentId);const rows=projectTimelineRows(normalizeAgentTimelinePage(agentTimelinePayload(input.agentId, page)).rows);const agent=rawAgents.find(a=>a.id===input.agentId);const first=rows.find(r=>r.item.type==="user_message"),last=[...rows].reverse().find(r=>r.item.type==="assistant_message"),user=[...rows].reverse().find(r=>r.item.type==="user_message");
        const source={lastUserMessageAt:agent?.lastUserMessageAt??null,attentionTimestamp:agent?.attentionTimestamp??null};
        if(input.expectedLastUserMessageAt!==undefined&&input.expectedLastUserMessageAt!==source.lastUserMessageAt||input.expectedAttentionTimestamp!==undefined&&input.expectedAttentionTimestamp!==source.attentionTimestamp)throw new Error("模拟结果版本已改变，请刷新查看最新结果");
        const ex=row=>row?{text:row.item.text||"",ref:`${input.agentId}:${page.epoch}:${row.seqStart}-${row.seqEnd}`,at:row.timestamp||null,truncated:false}:null;
        result={agentId:input.agentId,capturedAt:new Date().toISOString(),epoch:page.epoch,source,firstInput:first?{...ex(first),completeOrigin:true}:null,latestUserInput:ex(user),latestReply:ex(last),isRunning:agent?.status==="running",isFinal:agent?.attentionReason==="finished",artifacts:[],testEvidence:[],metrics:{tools:rows.filter(r=>r.item.type==="tool_call").length,failedTools:0,changedFiles:0,userTurns:1},coverage:{firstInputConfirmed:true,truncated:false,rows:rows.length,hasOlder:false,reason:null}};break;
      }
      case "observatory.insights.list": {
        const jobs=fixtureInsightState.jobs.filter(job=>!input.agentId||job.agentId===input.agentId).map(({report,evidence,...job})=>({...job,reportAvailable:!!report,summary:report?.summary??null,health:report?.health??null,findings:report?.findings.length??0,resolvedFindings:Object.values(job.findingFeedback??{}).filter(f=>f.status==="resolved").length}));
        result={...fixtureInsightState,runnerLabel:runnerLabelFor(fixtureInsightState.settings.runner),jobs,revision:insightSequence,unchanged:false,activeCount:jobs.filter(j=>["queued","collecting","analyzing"].includes(j.status)).length};break;
      }
      case "observatory.insights.jobs.snapshot": result = fixtureInsightJobsSnapshot(); break;
      case "observatory.insights.runners": {
        // Like the server: the built-in Grok option carries the saved model and effort while it is the configured runner.
        const saved = fixtureInsightState.settings.runner;
        result = saved?.kind === "grok" ? { ...fixtureRunners, builtin: { ...fixtureRunners.builtin, runner: saved, label: runnerLabelFor(saved), modelLabel: saved.model ?? fixtureRunners.builtin.modelLabel, thinkingLabel: saved.effort ?? fixtureRunners.builtin.thinkingLabel } } : fixtureRunners;
        break;
      }
      case "observatory.insights.models": result = fixtureModels(input.provider); break;
      // server/profiles.ts there: only the fields the sheets use, never the rest of the daemon config.
      case "observatory.profiles.list": result = { profiles: fixtureProfiles.map(({ id, name, provider, model, modeId, thinkingOptionId, featureValues, notes }) => ({ id, name, provider, model, modeId, thinkingOptionId, featureValues, notes })) }; break;
      case "observatory.insights.overview": result = fixtureOverview(input.days, input.utcOffsetMinutes, scenarioName(), fixtureInsightState.settings, runnerLabelFor(fixtureInsightState.settings.runner)); break;
      case "observatory.insights.suggestions": {
        const all = fixtureSuggestions(Date.now(), scenarioName());
        result = { job: input.jobId ? fixtureSuggestionsState.jobs.find(job => job.id === input.jobId) ?? (all.job?.id === input.jobId ? all.job : null) : all.job, history: all.history }; break;
      }
      case "observatory.insights.suggest": {
        if (scenarioName() === "insights-suggest-error") throw new Error("模拟：分析队列已满，请稍后再试。");
        const now = new Date().toISOString(), label = runnerLabelFor(input.runner ?? fixtureInsightState.settings.runner);
        result = { id: `fixture-suggestions-new-${++insightSequence}`, createdAt: now, updatedAt: now, status: "queued", trigger: "manual", stage: "已加入分析队列",
          window: { days: input.days, from: now.slice(0, 10), to: now.slice(0, 10), reports: 0, findings: 0, agents: 0, timeZone: input.timeZone ?? null, ...(input.utcOffsetMinutes !== undefined ? { utcOffsetMinutes: input.utcOffsetMinutes } : {}) },
          runner: { kind: (input.runner ?? fixtureInsightState.settings.runner).kind, label, provider: null, model: null, thinkingOptionId: null, modeId: null, sessionAgentId: null },
          usage: null, error: null, report: null, basis: [] };
        fixtureSuggestionsState.jobs.unshift(result); break;
      }
      case "observatory.insights.suggest-cancel": {
        const job = fixtureSuggestionsState.jobs.find(item => item.id === input.jobId); if (!job) throw new Error("模拟建议任务不存在");
        Object.assign(job, { status: "canceled", stage: "已取消", updatedAt: new Date().toISOString() }); result = job; break;
      }
      case "observatory.insights.action-feedback": {
        const all = fixtureSuggestions(Date.now(), scenarioName());
        if (!all.job || all.job.id !== input.jobId) throw new Error("模拟建议不存在");
        if (!all.job.report.actions.some(action => action.id === input.actionId)) throw new Error("这条建议已不在报告里");
        fixtureSuggestionsState.feedback[input.actionId] = { status: input.status, updatedAt: new Date().toISOString() };
        result = { ...all.job, feedback: { ...fixtureSuggestionsState.feedback } }; break;
      }
      case "observatory.insights.report": result={job:fixtureInsightState.jobs.find(job=>job.id===input.jobId)??null,revision:insightSequence};break;
      case "observatory.insights.feedback": {const job=fixtureInsightState.jobs.find(j=>j.id===input.jobId);job.findingFeedback={...job.findingFeedback,[input.findingId]:{status:input.status,updatedAt:new Date().toISOString()}};insightSequence++;result=job;break;}
      case "observatory.telemetry.get": result = fixtureTelemetry; break;
      case "observatory.live.get": result = { mode: "full", unchanged: false, revision: `live-${fixtureLiveExtras.length}`, agents: fixtureLiveExtras }; break;
      // Like the server: whether the read can be trusted travels with it (codex-F16), so 归档已读完成 can use it.
      case "observatory.steerable.list": result = { providers: ["claude", "codex"], slashSteerable: ["claude"] }; break;
      case "observatory.providers.bases": result = { bases: {} }; break;
      case "observatory.analytics.get": result = fixtureProjectAnalytics(input.projectId); break;
      case "observatory.stats.get": if (scenarioName() === "stats-error") throw new Error("模拟统计读取失败"); result = ["metric-comparison","data-basis","stats-empty"].includes(scenarioName()) ? fixtureStats(input.range, input.utcOffsetMinutes) : coreStats(input.range,input.utcOffsetMinutes,input.timeZone); if (scenarioName() === "metric-comparison") result = metricComparisonFixture(result); if (scenarioName() === "data-basis") result = statsBasisFixture(result); break;
      case "observatory.tokens.get": if (scenarioName() === "tokens-error") throw new Error("模拟 Token 统计读取失败"); result = ["token-ledger","tokens-empty"].includes(scenarioName()) ? fixtureTokens(input.range, input.utcOffsetMinutes) : coreTokens(input.range,input.utcOffsetMinutes,input.timeZone); if (scenarioName() === "token-ledger") result = tokenLedgerFixture(result); break;
      case "observatory.tokens.live": if (scenarioName() === "tokens-error") throw new Error("模拟 Token 统计读取失败"); result = fixtureTokenLive(input.windowMinutes, input.utcOffsetMinutes, input.detail); break;
      case "observatory.digest.get": result = scenarioName()==="digest-rich" ? fixtureDigest(input.agentId) : coreDigest(input.agentId); break;
      case "observatory.recap.list": { const full = fixtureRecapList(); result = input.revision === full.revision ? { mode: "unchanged", revision: full.revision } : full; break; }
      case "observatory.recap.status": result = fixtureRecapStatus(); break;
      case "observatory.read-state.list": result = { receipts: [...fixtureReadReceipts], archiveOrigins:[...fixtureArchiveOrigins], error: null }; break;
      case "observatory.read-state.mark": {
        const agent = rawAgents.find(candidate => candidate.id === input.agentId);
        const scenario = new URLSearchParams(globalThis.location?.search ?? "").get("scenario");
        if (scenario === "read-error") { result = { agentId: input.agentId, status: "failed", receipt: null, message: "模拟宿主确认失败，未读状态已保留。" }; break; }
        // Like the server: 知道了 (kind "error", a failure or a stop) clears the attention and records no completion receipt.
        // A stop the host raised no attention for (closed mid-turn) is acknowledged by its time alone, without clearing anything.
        if (input.kind === "error" && input.stopAt && agent && agent.requiresAttention !== true) {
          result = ackFixtureStop(agent.id, input.stopAt) ? { agentId: input.agentId, status: "read", receipt: null, message: "已标记为知道了。" }
            : { agentId: input.agentId, status: "stale", receipt: null, message: "停下的原因已有更新，请刷新后再确认。" };
          break;
        }
        const reasons = input.kind === "error" ? ["finished", "error"] : ["finished"];
        if (!agent || !reasons.includes(agent.attentionReason) || agent.requiresAttention !== true || agent.activeTurn || agent.pendingPermissions.length) {
          result = { agentId: input.agentId, status: "not_unread", receipt: null, message: "当前没有可标为已读的完成结果。" }; break;
        }
        if (agent.attentionTimestamp !== input.attentionTimestamp || (agent.lastUserMessageAt ?? null) !== input.lastUserMessageAt) {
          result = { agentId: input.agentId, status: "stale", receipt: null, message: "完成结果已更新，请刷新后确认。" }; break;
        }
        if (input.kind === "error") {
          if (input.stopAt) ackFixtureStop(agent.id, input.stopAt);
          updateFixtureAgent(agent.id, { requiresAttention: false, attentionReason: null, attentionTimestamp: null });
          result = { agentId: input.agentId, status: "read", receipt: null, message: "已标记为知道了，（仅本地模拟）。" };
          break;
        }
        const receipt = { agentId: input.agentId, completionAt: agent.attentionTimestamp, lastUserMessageAt: agent.lastUserMessageAt ?? null, readAt: new Date().toISOString() };
        const old = fixtureReadReceipts.findIndex(item => item.agentId === input.agentId);
        if (old >= 0) fixtureReadReceipts.splice(old, 1);
        fixtureReadReceipts.unshift(receipt);
        updateFixtureAgent(agent.id, { requiresAttention: false, attentionReason: null, attentionTimestamp: null });
        result = { agentId: input.agentId, status: "read", receipt, message: "已标为已读，（仅本地模拟）。" };
        break;
      }
      case "observatory.insights.state": result = { ...fixtureInsightState, jobs: fixtureInsightState.jobs.filter(job => !input.agentId || job.agentId === input.agentId) }; break;
      case "observatory.insights.start": {
        const runner = input.runner ?? fixtureInsightState.settings.runner;
        result = { ...fixtureInsight(input.agentId), id: `fixture-insight-${input.agentId}-${++insightSequence}`,
          runner: { kind: runner.kind === "auto" ? "grok" : runner.kind, label: runnerLabelFor(runner.kind === "auto" ? { kind: "grok" } : runner), provider: runner.kind === "provider" ? runner.provider : null,
            model: runner.kind === "provider" ? runner.model : runner.kind === "grok" || runner.kind === "auto" ? "grok-4.7" : null,
            thinkingOptionId: runner.kind === "provider" ? runner.thinkingOptionId : runner.kind === "grok" || runner.kind === "auto" ? "xhigh" : null, modeId: null,
            sessionAgentId: runner.kind === "grok" || runner.kind === "auto" ? null : "fixture-analysis-session" } };
        const scenario = new URLSearchParams(globalThis.location?.search ?? "").get("scenario");
        if (input.agentId === "mobile-bootstrap") result = { ...result, status: "skipped", report: null, stage: "手动分析至少需要 2 轮对话和 2 次工具调用，当前轨迹尚不足。", eligibility: { eligible: false, turnCount: 1, toolCount: 1, reason: "手动分析至少需要 2 轮对话和 2 次工具调用，当前轨迹尚不足。" } };
        else if (scenario === "insights-progress") result = { ...result, status: "analyzing", report: null, stage: "正在结合工具结果分析执行轨迹" };
        else if (scenario === "insights-failed") result = { ...result, status: "failed", report: null, error: "模拟分析进程中断，请重试。", stage: "分析进程已退出" };
        fixtureInsightState.jobs.unshift(result);
        break;
      }
      case "observatory.insights.configure": if (input.autoEnabled !== undefined) fixtureInsightState.settings.autoEnabled = input.autoEnabled; if (input.runner) fixtureInsightState.settings.runner = input.runner; result = fixtureInsightState.settings; break;
      case "observatory.insights.cancel": {
        if (scenarioName() === "insight-jobs") { result = cancelFixtureInsightJob(input); break; }
        const found = fixtureInsightState.jobs.find(job => job.id === input.jobId);
        if (!found) throw new Error("模拟分析任务不存在");
        Object.assign(found, { status: "canceled", stage: "分析已取消", report: null, updatedAt: new Date().toISOString() });
        result = found;
        break;
      }
      case "observatory.history.evidence": {
        const historical = input.agentId === "archived-cycle" || input.agentId === "old-report";
        const timeline = historical ? fixtureTimeline(input.agentId) : null;
        result = { agentId: input.agentId, turns: historical ? 2 : 4, tools: historical ? 2 : 12, toolFailures: historical ? 0 : 1, errors: 0,
          firstTimelineAt: timeline?.rows[0]?.timestamp ?? new Date(Date.now() - 1800_000).toISOString(), lastTimelineAt: timeline?.rows.at(-1)?.timestamp ?? new Date().toISOString(),
          sampleSize: timeline?.rows.length ?? 28, complete: true, hasOlder: false, coverageReason: null, completionEvidence: { kind: "none", at: null } };
        break;
      }
      case "observatory.history.restore": {
        const agent = rawAgents.find(agent => agent.id === input.agentId);
        const countUsers = () => fixtureTimeline(input.agentId).rows.filter(row => row.item.type === "user_message").length;
        const before = { agentId: agent?.id, agentCount: rawAgents.length, userMessages: countUsers() };
        if(agent?.archivedAt){const existing=fixtureArchiveOrigins.findIndex(item=>item.agentId===agent.id);if(existing>=0)fixtureArchiveOrigins.splice(existing,1);fixtureArchiveOrigins.push({agentId:agent.id,archivedAt:agent.archivedAt,lastUserMessageAt:agent.lastUserMessageAt??null});}
        if (agent) updateFixtureAgent(agent.id, { archivedAt: null });
        globalThis.__inspectorMockRestoreAudit = { before, after: { agentId: agent?.id, agentCount: rawAgents.length, userMessages: countUsers() } };
        result = { status: "restored", agentId: input.agentId, workspaceId: agent?.workspaceId ?? null, workspaceRestored: false, workspaceAction: null, message: "模拟会话已恢复" }; break;
      }
      // Restores answer at once here, so no job is ever polled; a stale id reads as unknown (the hook then asks the host).
      case "observatory.history.restore.status": result = null; break;
      case "observatory.provider-subagents.list": result = { subagents: input.parentAgentId === "payment-root" ? nativeChildren : [], error: null, truncated: false }; break;
      case "observatory.provider-subagents.timeline": result = { page: nativeTimeline(input.parentAgentId, input.subagentId), error: null }; break;
      // 提醒 (2.17): the preview has no Claude logs and no host labels.
      case "observatory.cache.get": result = { mode: "full", revision: "fixture", agents: [] }; break;
      case "observatory.alerts.labels": result = { state: "off", labels: 0, pins: 0, error: null }; break;
      default: throw new Error(`No development fixture for ${contract.name}`);
    }
    if(contract.name==="observatory.results.get"&&scenarioName()==="long-history"){
      const epoch=`long-${input.agentId}`;
      result={...result,epoch,firstInput:{text:longFirstInput,ref:`${input.agentId}:${epoch}:1-1`,at:new Date(Date.now()-3600000).toISOString(),truncated:false,completeOrigin:true},latestReply:{text:longLatestReply,ref:`${input.agentId}:${epoch}:1200-1200`,at:new Date().toISOString(),truncated:false},metrics:{tools:1197,failedTools:0,changedFiles:32,userTurns:2},coverage:{firstInputConfirmed:true,truncated:false,rows:1200,hasOlder:false,reason:null}};
    }
    if(contract.name==="observatory.results.get"&&scenarioName()==="unknown-origin")result={...result,firstInput:result.firstInput?{...result.firstInput,completeOrigin:false}:null,coverage:{...result.coverage,firstInputConfirmed:false,truncated:true,hasOlder:true,reason:"模拟宿主仅保留部分历史"}};
    // Revisioned host replies (AgentX 0.9 plugin 2.1): fixtures are always complete snapshots.
    if(revisioned.has(contract.name)&&result&&typeof result==="object"&&!("mode" in result))result={...result,mode:"full",unchanged:false,revision:`fixture-${++fixtureRevision}`};
    return contract.output.parse(result);
}

