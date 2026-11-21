/** @aiPurpose Compose request chronology and related native subject projections. */
import type * as N from "./api/native.js";
import type { Investigation, Workspace } from "./state.js";
import { badge, button, disclosure, el, empty, facts, region } from "./elements.js";
import { duration, evidence, exact, sameRequest, time, variant } from "./presentation.js";

/** @aiPurpose Reveal bounded collection coverage and losses. @aiUsage Attach to the exact returned runtime page or snapshot. */
export function coverage(value: N.RuntimeCoverage): HTMLElement {
  return disclosure("覆盖范围与丢失", facts([
    ["请求详情未捕获", exact(value.request_details_dropped)], ["保留行已淘汰", exact(value.retained_rows_evicted)],
    ["进程投影丢弃", exact(value.peer_projections_dropped)], ["主机投影丢弃", exact(value.host_projections_dropped)],
    ["客户端报告丢弃", exact(value.local_reports_dropped)], ["主机全量覆盖", value.full_host_census ? "是" : "否 · 不能推断主机总数或存活"],
  ]));
}

function requestLabel(row: N.RequestObservation): string { return `${row.operation.domain} / ${row.operation.variant}`; }

/** @aiPurpose Select exact retained requests without losing investigation context. @aiUsage Render in SubjectPane; effects remain in Investigation. */
export function RequestList(work: Investigation): HTMLElement {
  const state = work.state;
  const page = state.requests;
  const filter = el("input", "search-input");
  filter.type = "search";
  filter.placeholder = "筛选已加载的操作、请求 ID";
  filter.setAttribute("aria-label", "筛选已加载请求");
  filter.dataset.testid = "request-search";
  filter.value = state.search;
  filter.addEventListener("input", () => { state.search = filter.value; work.changed(); });
  const controls = el("div", "list-controls", filter,
    button("仅活动", () => { state.activeOnly = !state.activeOnly; void work.loadList(); }, { active: state.activeOnly, disabled: state.busy || state.fenced, testid: "active-only" }));
  const list = el("div", "subject-rows");
  const needle = state.search.toLocaleLowerCase();
  const rows = page?.rows.filter((row) => `${requestLabel(row)} ${row.request_id} ${row.reference.id}`.toLocaleLowerCase().includes(needle)) ?? [];
  for (const row of rows) {
    const selected = state.selected !== null && sameRequest(state.selected.reference, row.reference);
    const item = button("", () => { void work.selectRequest(row.reference); }, { active: selected, disabled: state.busy, testid: "request-row" });
    item.className = selected ? "subject-row subject-row-selected" : "subject-row";
    item.dataset.requestId = String(row.reference.id);
    item.append(el("span", "subject-row-top", el("strong", "subject-name", requestLabel(row)), badge(variant(row.execution))),
      el("span", "subject-row-meta", `#${row.reference.id} · ${row.request_id}`),
      el("span", "subject-row-meta", `连接 ${row.connection.id} · 代际 ${row.generation_at_decode}`));
    list.append(item);
  }
  if (!rows.length) list.append(empty(page ? "此筛选下没有已保留请求。捕获与保留范围见覆盖信息。" : "尚未读取请求；刷新读取已存在的 daemon。"));
  return el("section", "subject-list", controls, list, page ? el("footer", "list-footer",
    el("span", "muted", `已加载 ${page.rows.length} 条 · 修订 ${page.revision}`),
    page.next_before != null && button("更早请求", () => { void work.loadList(true); }, { disabled: state.busy || state.fenced, testid: "more-subjects" }),
    disclosure("采样依据", facts([["采样", time(page.sampled_at)]]), coverage(page.coverage))) : null);
}

function linkedRequest(work: Investigation, reference: N.RequestObservationRef | null | undefined): HTMLElement {
  return reference ? button(`调查请求 #${reference.id}`, () => { void work.selectRequest(reference); }, { disabled: work.state.busy, testid: "related-request" }) : el("span", "muted", "无已捕获请求关联");
}

/** @aiPurpose Connect native connections, peers, hosts and client reports to investigation. @aiUsage Render the selected subject using already loaded pages. */
export function RelatedSubjects(work: Investigation): HTMLElement {
  const state = work.state;
  const subject = state.subject;
  const page = state[subject];
  const list = el("div", "related-list");
  if (subject === "connections") for (const row of state.connections?.rows ?? []) list.append(region(`连接 #${row.reference.id}`,
    badge(row.phase), facts([["进程观察", evidence(row.peer, (ref) => `#${ref.id}`)], ["接受", time(row.accepted_at)],
      ["结束", row.closed ? `${row.closed[1]} · ${time(row.closed[0])}` : "未观测到关闭"]]), linkedRequest(work, row.request)));
  if (subject === "peers") for (const row of state.peers?.rows ?? []) {
    const connections = state.connections?.rows.filter((connection) => typeof connection.peer === "object" && "Present" in connection.peer && connection.peer.Present.id === row.reference.id) ?? [];
    list.append(region(`进程观察 #${row.reference.id}`, facts([["PID", exact(row.process.pid)], ["进程身份", row.process.incarnation ?? "未知 · 不能按 PID 合并"], ["开放连接", exact(row.open_connections)], ["最后观测", time(row.last_seen_at)]]),
      connections.length ? el("div", "action-row", ...connections.map((row) => linkedRequest(work, row.request))) : empty("关联连接尚未加载；可切换“连接”读取。")));
  }
  if (subject === "hosts") for (const row of state.hosts?.rows ?? []) list.append(region(row.participant.canonical_name,
    badge(row.state), facts([["主机观察", `#${row.reference.id} · ${row.kind}`], ["参与者身份", exact(row.participant.incarnation_bits)], ["进程观察", `#${row.peer.id}`], ["最后验证", time(row.last_validated_at)], ["数据代际", row.epoch.database_generation]]),
    linkedRequest(work, row.last_request), disclosure("注册关联依据", facts([["执行", row.target?.execution ?? "未关联"], ["绑定", row.target?.binding ?? "未关联"], ["子进程", row.child ? `${row.child.pid} · ${row.child.incarnation ?? "身份未知"}` : "未捕获"]]))));
  if (subject === "reports") for (const row of state.reports?.rows ?? []) {
    const report = row.observation.report;
    const control = button(`报告 #${row.reference.id} · ${report.kind}`, () => { void work.selectReport(row.reference); }, { disabled: state.busy || state.fenced, testid: "report-row" });
    list.append(region(`客户端尝试 ${report.attempt}`, control, badge("客户端自报"), facts([["调用知识", variant(report.knowledge)], ["请求 ID", report.request_id ?? "未分配"], ["接收", time(row.received_at)]])));
  }
  if (!page?.rows.length) list.append(empty("尚无已加载记录。点击“读取列表”获取有限投影；空列表不代表完整范围内不存在对象。"));
  return el("section", "related-subjects", list, page ? el("footer", "list-footer", el("span", "muted", `已加载 ${page.rows.length} 条`),
    page.next_before != null && button("更早记录", () => { void work.loadList(true); }, { disabled: state.busy || state.fenced, testid: "more-subjects" }), coverage(page.coverage)) : null);
}

function milestone(label: string, at: N.ObservationTime | null | undefined, note?: string): HTMLElement {
  return el("li", at ? "milestone milestone-observed" : "milestone", el("strong", "milestone-name", label), el("span", "milestone-time", time(at)), note ? el("span", "muted", note) : null);
}

/**
 * @aiPurpose Distinguish queue, application and reply observations for the selected request.
 * @aiUsage Render in the local overview tab; contextual actions preserve exact request selection.
 */
export function RequestOverview(work: Investigation): HTMLElement {
  const row = work.state.selected;
  if (!row) return empty("从左侧选择一个请求，调查排队、执行、回包及其关联证据。");
  const execution = row.execution;
  const started = typeof execution === "object" && ("Running" in execution || "Returned" in execution)
    ? ("Running" in execution ? execution.Running.started_at : execution.Returned.started_at) : null;
  const returned = typeof execution === "object" && "Returned" in execution ? execution.Returned : null;
  const reply = typeof row.reply === "object" ? row.reply.Observed : null;
  const replyFinished = reply && typeof reply.write === "object" ? ("Finished" in reply.write ? reply.write.Finished.at : reply.write.Failed.at) : null;
  const chronology = el("ol", "chronology", milestone("解码", row.decoded_at), milestone("入队", row.enqueued_at), milestone("出队", row.dequeued_at), milestone("执行开始", started), milestone("应用返回", returned?.ended_at ?? null, returned ? variant(returned.result) : variant(execution)), milestone("回包写入", replyFinished, reply ? variant(reply.write) : "尚无回包观测"));
  const notRunReason = typeof execution === "object" && "NotRun" in execution ? execution.NotRun.reason : null;
  const notRun = notRunReason ? `未执行 · ${variant(notRunReason)} / ${"Admission" in notRunReason ? notRunReason.Admission : notRunReason.Dispatch}` : null;
  return el("div", "overview-content",
    region("请求链路", el("div", "action-row", badge(variant(execution)), badge(reply ? `回包 ${variant(reply.write)}` : "回包 Pending"), notRun && badge(notRun, "warning")), chronology,
      facts([["排队耗时", duration(row.enqueued_at, row.dequeued_at)], ["应用耗时", duration(started, returned?.ended_at ?? null)], ["服务到回包", duration(row.decoded_at, replyFinished)]])),
    region("关联身份", facts([["逻辑请求 ID", row.request_id], ["请求观察", `#${row.reference.id}`], ["连接", `#${row.connection.id}`], ["进程观察", evidence(row.peer, (ref) => `#${ref.id}`)], ["客户端尝试", row.origin?.attempt ?? "未捕获"], ["调用修订", row.origin ? `${row.origin.invocation.invocation} / ${row.origin.invocation.revision}` : "未捕获"]])),
    disclosure("代际、截止与最后观测", facts([["解码代际", row.generation_at_decode], ["调度代际", row.generation_at_dispatch ?? "未调度"], ["返回后代际", evidence(row.generation_after_return)], ["调用方截止 Unix ms", exact(row.caller_deadline_unix_ms)], ["有效截止", evidence(row.effective_deadline, time)], ["最后更新", time(row.last_updated_at)], ["daemon", row.reference.daemon.daemon_instance]])),
    region("继续调查", el("div", "action-row", button("查看捕获配置", () => work.detail("configuration"), { testid: "open-configuration" }), button("查看关联事件", () => { work.detail("events"); if (!work.state.events) void work.openEvents(); }, { testid: "open-events", disabled: work.state.busy || work.state.fenced || !work.currentRequest(row.reference) }))),
  );
}

/** @aiPurpose Present client-reported chronology with its intake peer and authority. @aiUsage Render a selected retained report without asserting daemon-observed truth. */
export function ReportDetail(state: Workspace): HTMLElement {
  const retained = state.report;
  if (!retained) return empty("尚未选择客户端报告");
  const report = retained.observation.report;
  const stages = el("ol", "chronology", ...report.stages.map((span) => milestone(span.stage, span.started_at, `${duration(span.started_at, span.ended_at)}${span.failure ? ` · ${span.failure}` : ""}`)));
  return region(`客户端报告 #${retained.reference.id}`, badge("ClientReported · 自报证据"), facts([["尝试", report.attempt], ["调用知识", variant(report.knowledge)], ["请求 ID", report.request_id ?? "未分配"], ["请求作用域", report.requested_scope], ["解析作用域", evidence(report.resolved_scope, (scope) => scope.canonical_scope)], ["接收时进程", `${retained.observation.submitted_by.process.pid} · ${retained.observation.submitted_by.process.incarnation ?? "身份未知"}`], ["省略阶段", exact(report.omitted_stages)], ["失败", report.failure ? `${report.failure.stage} / ${report.failure.kind}` : "未报告失败"]]), stages);
}
