/** @aiPurpose Present finite native event windows, correlated facts and explicit collection/retention loss. */
import type * as N from "./api/native.js";
import type { EventEvidence, Investigation } from "./state.js";
import { badge, button, disclosure, el, empty, facts, region } from "./elements.js";
import { exact, sameRequest, time, variant } from "./presentation.js";

function lossEvidence(head: N.ObservationHead): HTMLElement {
  const { dropped, evicted } = head.losses;
  const droppedTotal = dropped.contended + dropped.oversize + dropped.invalid_input + dropped.retired_epoch + dropped.collector_unavailable + dropped.preparation_budget;
  const evictedTotal = evicted.age + evicted.count_pressure + evicted.byte_pressure;
  return el("div", "loss-evidence", el("div", "action-row", badge(`捕获丢失 ${droppedTotal}`, droppedTotal > 0n ? "warning" : "neutral"), badge(`保留淘汰 ${evictedTotal}`, evictedTotal > 0n ? "warning" : "neutral"), badge(`抑制 ${head.losses.suppressed}`)),
    disclosure("丢失计数依据", facts([["锁争用", exact(dropped.contended)], ["条目过大", exact(dropped.oversize)], ["输入无效", exact(dropped.invalid_input)], ["旧代际", exact(dropped.retired_epoch)], ["收集器不可用", exact(dropped.collector_unavailable)], ["准备预算", exact(dropped.preparation_budget)], ["时间淘汰", exact(evicted.age)], ["条数压力", exact(evicted.count_pressure)], ["字节压力", exact(evicted.byte_pressure)], ["采样", time(head.losses.sampled_at)]])));
}

function runtimeFact(value: N.RuntimeFact): string {
  if (typeof value === "string") return value;
  if ("LifecycleChanged" in value) return `生命周期 ${value.LifecycleChanged}`;
  if ("ConnectionClosed" in value) return `连接结束 ${value.ConnectionClosed}`;
  if ("CallDecoded" in value) return `解码 ${value.CallDecoded.domain} / ${value.CallDecoded.variant}`;
  if ("AdmissionRejected" in value) return `准入拒绝 ${value.AdmissionRejected}`;
  if ("DispatchRejected" in value) return `调度拒绝 ${value.DispatchRejected}`;
  if ("ApplicationReturned" in value) return `应用返回 ${variant(value.ApplicationReturned)}${typeof value.ApplicationReturned === "object" ? ` / ${value.ApplicationReturned.ReturnedError}` : ""}`;
  if ("ReplyWriteFinished" in value) return `回包写入 ${value.ReplyWriteFinished.failure ?? "已完成"}`;
  if ("ReplyWaitAbandoned" in value) return `回包等待结束 ${value.ReplyWaitAbandoned}`;
  return `主机关联 #${value.HostAssociationObserved.host.id} · ${value.HostAssociationObserved.state}`;
}

function recordBody(record: N.ObservationRecord, work: Investigation): HTMLElement {
  const value = record.input.payload;
  if ("Runtime" in value) {
    const { correlation, fact } = value.Runtime;
    return el("div", "event-body", el("strong", "event-title", runtimeFact(fact)), facts([["请求观察", correlation.request === null ? "未关联" : `#${correlation.request}`], ["连接", exact(correlation.connection)], ["尝试", correlation.client_attempt ?? "未关联"]]));
  }
  if ("Configuration" in value) {
    const fact = value.Configuration;
    if ("SnapshotPublished" in fact) {
      const capture = fact.SnapshotPublished;
      return el("div", "event-body", el("strong", "event-title", `配置快照 #${capture.snapshot.sequence} · ${capture.stage}`),
        facts([["来源", capture.origin], ["尝试", capture.attempt ?? "未关联"], ["捕获", time(capture.captured_at)]]),
        el("div", "action-row", button("快照设为 A", () => { void work.loadConfiguration(capture.snapshot, "left"); }, { disabled: work.state.busy, testid: "event-configuration-left" }),
          button("快照设为 B", () => { void work.loadConfiguration(capture.snapshot, "right"); }, { disabled: work.state.busy, testid: "event-configuration-right" })));
    }
    if ("CaptureSkipped" in fact) return el("div", "event-body", el("strong", "event-title", `配置捕获跳过 · ${fact.CaptureSkipped.reason}`), facts([["阶段", fact.CaptureSkipped.stage], ["尝试", fact.CaptureSkipped.attempt ?? "未关联"]]));
    return el("div", "event-body", el("strong", "event-title", `配置退休至 #${fact.SnapshotsRetired.through_sequence}`), el("span", "muted", `${fact.SnapshotsRetired.count} 条 · ${fact.SnapshotsRetired.reason}`));
  }
  if ("ClientReported" in value) return el("div", "event-body", el("strong", "event-title", `客户端自报 · ${value.ClientReported.report.kind}`), facts([["尝试", value.ClientReported.report.attempt], ["知识", variant(value.ClientReported.report.knowledge)], ["接收时进程", exact(value.ClientReported.submitted_by.process.pid)]]));
  const { fact, correlation } = value.DomainCommitted;
  const refs: Array<[string, string]> = [["领域事实", variant(fact)], ["请求观察", exact(correlation.request)], ["尝试", correlation.client_attempt ?? "未关联"]];
  if ("EventAppended" in fact) refs.push(["持久事件", `${fact.EventAppended.event.event} · ${fact.EventAppended.event.database_generation}`]);
  if ("MessageAccepted" in fact) refs.push(["接收请求", fact.MessageAccepted.acceptance.request_id], ["持久事件", String(fact.MessageAccepted.event.event)], ["重放", String(fact.MessageAccepted.replayed)]);
  if ("InboxPhaseStored" in fact) refs.push(["收件批次", fact.InboxPhaseStored.claim.batch], ["阶段", fact.InboxPhaseStored.phase]);
  if ("DeliveryConfirmationStored" in fact) refs.push(["收件批次", fact.DeliveryConfirmationStored.claim.batch], ["边界", fact.DeliveryConfirmationStored.boundary], ["确认", fact.DeliveryConfirmationStored.confirmation]);
  if ("OutputConfirmationStored" in fact) refs.push(["输出观察", fact.OutputConfirmationStored.output.observation], ["确认至事件", String(fact.OutputConfirmationStored.through)]);
  return el("div", "event-body", el("strong", "event-title", variant(fact)), facts(refs), el("span", "context-note", "领域原始详情未提供授权读取入口"));
}

function JournalEvidence(events: EventEvidence, work: Investigation, archived = false): HTMLElement {
  const journal = el("ol", "event-journal");
  journal.dataset.testid = archived ? "historical-event-records" : "event-records";
  for (const record of events.records) journal.append(el("li", "event-record", el("div", "event-marker", `#${record.cursor.sequence}`, badge(variant(record.input.payload))),
    recordBody(record, work), disclosure("事件身份与时刻", facts([["来源权威", record.input.source.origin], ["源时刻", time(record.input.source.at)], ["收集时刻", time(record.collected_at)], ["收集器", record.cursor.stream.nonce], ["代际", record.input.epoch.database_generation]]))));
  const window = events.window;
  const exhausted = window !== null && window.after.sequence >= window.through;
  const filter = window?.filter;
  return region(archived ? "历史事件窗口" : "关联事件窗口",
    facts([["请求观察", events.request ? `#${events.request.id} · ${events.request.daemon.daemon_instance}` : "未固定请求"], ["选择器", filter?.request !== null && filter?.request !== undefined ? `请求 #${filter.request}` : filter?.client_attempt ? `客户端尝试 ${filter.client_attempt}` : "当前作用域全部保留事件"], ["固定上界", window ? String(window.through) : "未建立"], ["已扫描至", window ? String(window.after.sequence) : "未建立"], ["保留区间", `${events.head.oldest} … ${events.head.newest}`]]),
    lossEvidence(events.head),
    ...events.gaps.map((gap) => { const line = el("p", "evidence-warning", `保留缺口：#${gap.first} … #${gap.last}。缺失内容不可恢复。`); line.dataset.testid = "event-gap"; return line; }),
    events.boundary && badge(`读取停止 · ${events.boundary}`, "warning"),
    events.records.length ? journal : empty(exhausted ? "此固定窗口已扫描完毕，没有匹配的保留事件；捕获丢失另计。" : "此窗口尚无已返回记录。"),
    !archived && el("div", "action-row",
      !events.boundary && button(exhausted ? "窗口已读完" : "继续读取此窗口", () => { void work.nextEvents(); }, { disabled: exhausted || work.state.busy || work.state.fenced, testid: "events-next" }),
      events.boundary === "Gap" && button("跳过缺口，读取保留余段", () => { void work.continueRetainedEvents(); }, { disabled: work.state.busy || work.state.fenced, testid: "events-continue-retained" })),
    disclosure("窗口身份", facts([["收集器", events.head.stream.nonce], ["daemon", events.head.stream.epoch.daemon.daemon_instance], ["数据代际", events.head.stream.epoch.database_generation], ["Head 观测", time(events.head.observed_at)]])));
}

/** @aiPurpose Expose explicit window open/continue/recovery without manufacturing continuity. @aiUsage Render the event tab; new windows archive prior evidence and preserve the request. */
export function EventsPanel(work: Investigation): HTMLElement {
  const { state } = work;
  const reference = state.selected?.reference;
  const mismatch = reference && state.events?.request && !sameRequest(reference, state.events.request);
  return el("div", "events-panel", region("事件选择", el("div", "action-row",
    button(reference ? "打开请求事件窗口" : "打开作用域事件窗口", () => { void work.openEvents(reference ? "request" : "all"); }, { disabled: state.busy || state.fenced || Boolean(reference && !work.currentRequest(reference)), testid: "events-open" }),
    state.selected?.origin && button("按此客户端尝试读取", () => { void work.openEvents("attempt"); }, { disabled: state.busy || state.fenced || !work.currentRequest(state.selected.reference), testid: "events-by-attempt" }),
    button("打开作用域事件窗口", () => { void work.openEvents("all"); }, { disabled: state.busy || state.fenced, testid: "events-scope" })),
    mismatch ? badge("当前显示另一请求的已保留窗口；打开新窗口才会改变选择", "warning") : null,
    el("p", "context-note", "窗口上界固定，继续读取不会追赶新事件。新开窗口会保留旧窗口；不会自动跨越缺口或代际。")),
    state.events ? JournalEvidence(state.events, work) : empty("尚未打开事件窗口。"),
    state.archivedEvents.length ? disclosure(`此前窗口（${state.archivedEvents.length}）`, ...state.archivedEvents.map((events) => JournalEvidence(events, work, true))) : null);
}
