/** @aiPurpose Compose a compact one-scope investigation workbench with persistent subject and evidence context. */
import type { ObservationApi } from "./api/client.js";
import { Investigation } from "./state.js";
import type { DetailTab, Subject } from "./state.js";
import { badge, button, disclosure, el, empty, facts } from "./elements.js";
import { evidence, exact, time, variant } from "./presentation.js";
import { coverage, RelatedSubjects, ReportDetail, RequestList, RequestOverview } from "./subjects.js";
import { ConfigurationPanel } from "./configuration.js";
import { EventsPanel } from "./events.js";

function ScopeHeader(work: Investigation): HTMLElement {
  const { state } = work;
  const heading = el("header", "scope-header",
    el("div", "brand", el("span", "brand-mark", "A"), el("div", "brand-copy", el("strong", "brand-name", "AgentX"), el("span", "brand-purpose", "运行调查"))),
    el("div", "scope-context", el("span", "eyebrow", "当前作用域"), el("strong", "scope-path", state.scope.canonical_scope)),
    el("div", "scope-actions", button(state.busy ? "读取中…" : "刷新观测", () => { void work.refresh(); }, { disabled: state.busy || state.fenced || state.availability === "closed", testid: "refresh-observation" }),
      button("关闭调查", () => { void work.close(); }, { disabled: state.availability === "closed", testid: "close-investigation" })));
  heading.dataset.testid = "scope-context";
  return heading;
}

function ConnectionContext(work: Investigation): HTMLElement {
  const { state } = work;
  const status = el("div", state.historical || state.fenced ? "connection-context connection-warning" : "connection-context",
    el("div", "connection-message", badge(state.availability, state.availability === "connected" && !state.fenced ? "good" : "warning"), el("span", "", state.message)),
    el("div", "era-context", el("span", "", `daemon ${state.daemon?.daemon_instance ?? "未绑定"}`), el("span", "", `代际 ${state.generation ?? "未知 / 不可用"}`)),
    state.fenced && button("进入当前上下文", () => { void work.enterCurrentContext(); }, { disabled: state.busy, testid: "context-rebind" }),
    state.ready && disclosure("观测依据", facts([["构建", `${state.ready.build_version} · ${state.ready.build_id}`], ["协议", exact(state.ready.protocol_version)], ["daemon PID", exact(state.ready.process.pid)], ["进程身份", state.ready.process.incarnation ?? "未知"], ["Ready 采样", time(state.ready.sampled_at)], ["客户端接收", time(state.receivedAt)], ["代际来源", state.ready.generation.stamp.origin], ["代际证据", evidence(state.ready.generation.value)]])));
  status.dataset.testid = "connection-status";
  status.setAttribute("role", "status");
  status.setAttribute("aria-live", "polite");
  return status;
}

function RuntimeStrip(work: Investigation): HTMLElement {
  const snapshot = work.state.snapshot;
  if (!snapshot) return el("section", "runtime-strip", empty("尚无 daemon 快照；此页面不会启动 daemon。"));
  const gauges = snapshot.gauges;
  return el("section", "runtime-strip",
    el("div", "runtime-metric", el("span", "metric-label", "生命周期"), el("strong", "metric-value", snapshot.lifecycle)),
    el("div", "runtime-metric", el("span", "metric-label", "调度器"), el("strong", "metric-value", variant(snapshot.activity))),
    el("div", "runtime-metric", el("span", "metric-label", "排队"), el("strong", "metric-value", exact(gauges.queued_jobs))),
    el("div", "runtime-metric", el("span", "metric-label", "执行"), el("strong", "metric-value", exact(gauges.executing_jobs))),
    el("div", "runtime-metric", el("span", "metric-label", "业务 / 检查连接"), el("strong", "metric-value", `${gauges.business_connections} / ${gauges.inspection_connections}`)),
    disclosure("运行证据", facts([["采样", time(snapshot.sampled_at)], ["接收进度", time(snapshot.admission_tick_at)], ["调度进度", time(snapshot.dispatcher_progress_at)], ["最早排队", time(snapshot.oldest_queued_at)], ["累计解码 / 准入 Busy", `${snapshot.lifetime.decoded_calls} / ${snapshot.lifetime.admission_busy}`], ["累计应用成功 / 错误", `${snapshot.lifetime.application_ok} / ${snapshot.lifetime.application_error}`], ["累计回包写入 / 失败", `${snapshot.lifetime.reply_write_ok} / ${snapshot.lifetime.reply_write_failed}`]]), coverage(snapshot.coverage)));
}

const subjects: Array<[Subject, string]> = [["requests", "请求"], ["connections", "连接"], ["peers", "进程"], ["hosts", "主机"], ["reports", "客户端报告"]];
const tabs: Array<[DetailTab, string]> = [["overview", "执行过程"], ["configuration", "配置对比"], ["events", "关联事件"]];

function SubjectNavigation(work: Investigation): HTMLElement {
  const navigation = el("nav", "subject-navigation", ...subjects.map(([subject, label]) => {
    const control = button(label, () => { work.navigate(subject); void work.loadList(); }, { active: work.state.subject === subject, disabled: work.state.busy, testid: `subject-${subject}` });
    return control;
  }));
  navigation.setAttribute("aria-label", "调查对象");
  return navigation;
}

function SubjectPane(work: Investigation): HTMLElement {
  const { state } = work;
  const heading = subjects.find(([subject]) => subject === state.subject)![1];
  const pane = el("aside", "subject-pane", el("div", "pane-heading", el("h2", "pane-title", heading), button("读取列表", () => { void work.loadList(); }, { disabled: state.busy || state.fenced, testid: "load-subjects" })),
    state.subject === "requests" ? RequestList(work) : RelatedSubjects(work));
  pane.setAttribute("aria-label", "对象列表");
  pane.dataset.testid = "subject-pane";
  return pane;
}

function InvestigationDetail(work: Investigation): HTMLElement {
  const { state } = work;
  const row = state.selected;
  const historic = state.historical || state.fenced || Boolean(row && (!work.currentRequest(row.reference) || (state.generation !== null && row.generation_at_decode !== state.generation)));
  const title = row ? `#${row.reference.id} · ${row.operation.domain} / ${row.operation.variant}` : "调查详情";
  const tabbar = el("nav", "detail-tabs", ...tabs.map(([tab, label]) => button(label, () => { state.report = null; work.detail(tab); }, { active: state.tab === tab, testid: `tab-${tab}` })));
  tabbar.setAttribute("aria-label", "请求证据");
  let content: HTMLElement;
  if (state.report) content = ReportDetail(state);
  else if (state.tab === "configuration") content = ConfigurationPanel(work);
  else if (state.tab === "events") content = EventsPanel(work);
  else content = RequestOverview(work);
  const pane = el("section", "detail-pane",
    el("header", "detail-heading", el("div", "", el("span", "eyebrow", row ? "选中请求" : "证据工作区"), el("h2", "detail-title", title)), historic ? badge("历史证据", "warning") : null,
      row && button("返回请求", () => { state.report = null; work.detail("overview"); }, { testid: "return-request" })),
    tabbar, state.issue ? el("p", "evidence-warning", state.issue) : null, el("div", "detail-content", content));
  pane.setAttribute("aria-label", "选中对象调查详情");
  pane.dataset.testid = "detail-pane";
  return pane;
}

function InvestigationLayout(work: Investigation): HTMLElement {
  const { state } = work;
  const narrow = el("nav", "narrow-navigation", button("请求列表", () => { state.pane = "list"; work.changed(); }, { active: state.pane === "list", testid: "narrow-list" }),
    button(state.selected ? `调查 #${state.selected.reference.id}` : "调查详情", () => { state.pane = "detail"; work.changed(); }, { active: state.pane === "detail", testid: "narrow-detail" }));
  narrow.setAttribute("aria-label", "窄屏工作区");
  return el("div", "investigation-body", SubjectNavigation(work), narrow,
    el("div", `investigation-layout pane-${state.pane}`, SubjectPane(work), InvestigationDetail(work)));
}

/** Preserve native local disclosure, scroll and keyboard focus across state projections. */
function renderWorkbench(root: HTMLElement, work: Investigation): void {
  const open = new Set(Array.from(root.querySelectorAll("details[open]")).map((node) => disclosureKey(node)));
  const focused = document.activeElement instanceof HTMLElement && root.contains(document.activeElement) ? document.activeElement : null;
  const focusKey = focused ? controlKey(focused) : null;
  const inputSelection = focused instanceof HTMLInputElement ? [focused.selectionStart, focused.selectionEnd] : null;
  const scroll = new Map(Array.from(root.querySelectorAll<HTMLElement>(".subject-rows, .related-list, .detail-content")).map((node) => [node.className, node.scrollTop]));
  root.replaceChildren(ScopeHeader(work), ConnectionContext(work), RuntimeStrip(work), InvestigationLayout(work));
  for (const node of root.querySelectorAll<HTMLDetailsElement>("details")) node.open = open.has(disclosureKey(node));
  for (const node of root.querySelectorAll<HTMLElement>(".subject-rows, .related-list, .detail-content")) node.scrollTop = scroll.get(node.className) ?? 0;
  if (focusKey) {
    const target = Array.from(root.querySelectorAll<HTMLElement>("button, input, select, summary")).find((node) => controlKey(node) === focusKey && node.getClientRects().length > 0);
    target?.focus({ preventScroll: true });
    if (target instanceof HTMLInputElement && inputSelection && inputSelection[0] !== null && inputSelection[1] !== null) target.setSelectionRange(inputSelection[0], inputSelection[1]);
  }
}

function controlKey(node: HTMLElement): string {
  return `${node.tagName}:${node.dataset.testid ?? node.getAttribute("aria-label") ?? node.textContent}:${node.dataset.requestId ?? ""}`;
}

function disclosureKey(node: Element): string {
  const section = node.closest("section");
  return `${section?.getAttribute("aria-label") ?? ""}:${node.querySelector("summary")?.textContent ?? ""}`;
}

/**
 * @aiPurpose Mount the real authenticated native investigation console.
 * @aiUsage Call once after connect() with the root and actual ObservationApi; no mock is selected by default.
 * @aiLayout Scope/era and runtime remain visible while local subject and evidence panes retain scroll/disclosure.
 */
export function mountInvestigation(root: HTMLElement, api: ObservationApi): void {
  const investigation = new Investigation(api);
  root.dataset.testid = "investigation";
  root.className = "observation-workspace";
  investigation.subscribe(() => renderWorkbench(root, investigation));
  window.addEventListener("pagehide", () => { void investigation.close(); }, { once: true });
  renderWorkbench(root, investigation);
  void investigation.refresh();
}
