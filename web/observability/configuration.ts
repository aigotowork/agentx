/** @aiPurpose Compose captured configuration, retained operands and native comparison evidence. */
import type * as N from "./api/native.js";
import type { Investigation } from "./state.js";
import { badge, button, disclosure, el, empty, facts, region } from "./elements.js";
import { configKey, configValue, evidence, exact, fieldName, relation, time, variant } from "./presentation.js";

function fieldSource(field: N.FieldEvidence): string {
  return evidence(field.resolution, (resolution) => {
    if (typeof resolution === "string") return resolution;
    if ("Effective" in resolution) return `${resolution.Effective.source.kind}${resolution.Effective.normalized ? " · 已规范化" : ""}`;
    if ("Defaulted" in resolution) return `默认值 · ${resolution.Defaulted.issue}（拒绝 ${resolution.Defaulted.rejected.kind}）`;
    return `未知 · ${resolution.Unknown}`;
  });
}

function fieldSteps(field: N.FieldEvidence): HTMLElement {
  return disclosure("解析依据", el("ol", "resolution-steps", ...field.steps.map((step) => el("li", "resolution-step",
    el("strong", "", step.source.kind), el("span", "muted", `${variant(step.disposition)} · ${variant(step.validation)} · 源读取 ${exact(step.source.read)}`)))));
}

/** @aiPurpose Present safe retained fields with capture/provenance and missing reasons. @aiUsage Supply the exact lookup and ref; never replace missing evidence with current configuration. */
export function ConfigSnapshotEvidence(lookup: N.SnapshotLookup | undefined, reference: N.ConfigSnapshotRef, label: string): HTMLElement {
  if (!lookup) return region(label, empty("此快照尚未加载，或属于历史上下文；不会用当前配置重建。"));
  if ("Missing" in lookup) return region(label, badge("快照不可用", "warning"), facts([["序号", exact(reference.sequence)], ["保留状态", variant(lookup.Missing.reason)],
    ["已退休至", typeof lookup.Missing.reason === "object" ? exact(lookup.Missing.reason.NoLongerRetained.retired_through) : "从未签发"]]));
  const snapshot = lookup.Found;
  const body = snapshot.body;
  const table = el("table", "evidence-table");
  table.append(el("thead", "", el("tr", "", el("th", "", "字段"), el("th", "", "捕获值"), el("th", "", "来源 / 解析"))));
  const rows = el("tbody", "");
  for (const field of body.fields) rows.append(el("tr", "", el("th", "", fieldName(field.field)), el("td", "value-cell", configValue(field.value)), el("td", "", el("span", "", fieldSource(field)), fieldSteps(field))));
  table.append(rows);
  return region(label, facts([["快照", `#${reference.sequence}`], ["阶段 / 用途", `${body.stage} / ${body.purpose}`], ["来源权威", variant(snapshot.provenance)], ["解析作用域", evidence(body.resolution_scope, (scope) => scope.canonical_scope)], ["捕获时刻", time(body.captured_at)], ["读取一致性", body.consistency]]),
    body.fields.length ? el("div", "table-scroll", table) : empty("该快照未包含可披露字段。"),
    disclosure("身份、读取与规则", facts([["daemon", reference.epoch.daemon.daemon_instance], ["数据代际", reference.epoch.database_generation], ["保留时刻", time(snapshot.retained_at)], ["消费字段", body.consumed_fields.map(fieldName).join("、") || "未声明"], ["规则 build", body.rules.native_build_id], ["字段 / 解析 / 披露修订", `${body.rules.field_catalog_revision} / ${body.rules.resolution_revision} / ${body.rules.disclosure_revision}`]]),
      ...body.source_reads.map((read) => facts([["源读取", `${read.id} · ${read.resource}`], ["读取修订", read.read_revision], ["状态", variant(read.state)], ["读取时刻", time(read.at)]]))));
}

function configurationUses(work: Investigation): HTMLElement {
  const request = work.state.selected;
  if (!request) return empty("请选择请求以查看它实际使用的配置捕获。");
  if (request.configuration === "NotEvaluated") return empty("配置未求值（NotEvaluated）；没有默认快照可供推断。");
  const trace = request.configuration.Captured;
  const uses = el("div", "configuration-uses");
  for (const use of trace.uses) {
    if ("Skipped" in use.capture) uses.append(el("div", "configuration-use", badge(use.stage), badge(`捕获跳过 · ${use.capture.Skipped}`, "warning")));
    else {
      const reference = use.capture.Recorded;
      uses.append(el("div", "configuration-use", el("strong", "", `${use.stage} · #${reference.sequence}`),
        button("设为 A", () => { void work.loadConfiguration(reference, "left"); }, { disabled: work.state.busy, testid: "configuration-left" }),
        button("设为 B", () => { void work.loadConfiguration(reference, "right"); }, { disabled: work.state.busy, testid: "configuration-right" })));
    }
  }
  if (!trace.uses.length) uses.append(empty("本请求没有保留的配置使用记录。"));
  if (trace.omitted_uses > 0n) uses.append(badge(`另有 ${trace.omitted_uses} 次使用未保留`, "warning"));
  return uses;
}

function operand(work: Investigation, side: "left" | "right", label: string): HTMLElement {
  const selected = work.state[side];
  const select = el("select", "operand-select");
  select.setAttribute("aria-label", `${label} 配置快照`);
  select.dataset.testid = side === "left" ? "operand-a" : "operand-b";
  const blank = el("option", "", `选择${label}`);
  blank.value = "";
  select.append(blank);
  for (const [key, ref] of work.state.configRefs) {
    const option = el("option", "", `#${ref.sequence} · ${ref.epoch.database_generation} · ${ref.epoch.daemon.daemon_instance}`);
    option.value = key;
    option.selected = selected !== null && configKey(selected) === key;
    select.append(option);
  }
  select.disabled = work.state.busy;
  select.addEventListener("change", () => {
    const ref = work.state.configRefs.get(select.value);
    if (ref) void work.loadConfiguration(ref, side);
    else { work.state[side] = null; work.state.comparison = null; work.changed(); }
  });
  return el("label", "operand", el("span", "field-label", label), select);
}

function comparison(work: Investigation): HTMLElement | null {
  const result = work.state.comparison;
  if (!result) return null;
  const value = result.result;
  if ("Incomparable" in value) return region("对比结果", badge(`不可比较 · ${value.Incomparable}`, "warning"));
  if ("Unavailable" in value) return region("对比结果", badge("部分证据不可用", "warning"),
    work.state.left && ConfigSnapshotEvidence(value.Unavailable.left, work.state.left, "A · 保留结果"),
    work.state.right && ConfigSnapshotEvidence(value.Unavailable.right, work.state.right, "B · 保留结果"));
  const { basis, fields } = value.Compared;
  const table = el("table", "evidence-table comparison-table");
  table.dataset.testid = "comparison-result";
  table.append(el("thead", "", el("tr", "", ...["字段", "A", "B", "值关系", "来源关系"].map((text) => el("th", "", text)))));
  table.append(el("tbody", "", ...fields.map((field) => el("tr", field.value === "Different" ? "comparison-different" : "",
    el("th", "", fieldName(field.field)), el("td", "value-cell", field.left ? configValue(field.left.value) : "未观测"), el("td", "value-cell", field.right ? configValue(field.right.value) : "未观测"),
    el("td", "", badge(relation(field.value), field.value === "Different" ? "warning" : "neutral")), el("td", "", field.provenance)))));
  return region("字段对比", el("div", "table-scroll", table), disclosure("对比依据", facts([["阶段", basis.stage], ["A 用途 / 来源", `${basis.left_purpose} / ${variant(basis.left_provenance)}`], ["B 用途 / 来源", `${basis.right_purpose} / ${variant(basis.right_provenance)}`], ["A 捕获", time(basis.left_captured_at)], ["B 捕获", time(basis.right_captured_at)], ["对比观测", time(result.observed_at)]])));
}

/** @aiPurpose Keep request captures and A/B comparison in one investigation. @aiUsage Render the configuration tab; operands survive selecting another request. */
export function ConfigurationPanel(work: Investigation): HTMLElement {
  const state = work.state;
  const head = state.configurationHead;
  return el("div", "configuration-panel",
    region("请求使用的配置", configurationUses(work)),
    region("保留快照对比", el("div", "comparison-controls", operand(work, "left", "A"), operand(work, "right", "B"),
      button("比较 A / B", () => { void work.compare(); }, { disabled: !state.left || !state.right || state.busy || state.fenced, testid: "compare-configurations" })),
      el("p", "context-note", "A / B 随调查保留。选择另一个请求可继续取它的快照；对比依据以原生返回为准。")),
    comparison(work),
    el("div", "snapshot-pair", state.left ? ConfigSnapshotEvidence(state.configurations.get(configKey(state.left)), state.left, "A · 捕获证据") : null,
      state.right ? ConfigSnapshotEvidence(state.configurations.get(configKey(state.right)), state.right, "B · 捕获证据") : null),
    disclosure("配置保留范围", button("读取保留边界", () => { void work.loadConfigurationHead(); }, { disabled: state.busy || state.fenced, testid: "configuration-head" }),
      head ? facts([["已保留快照", exact(head.retained_count)], ["最早保留", exact(head.oldest_retained)], ["最后签发", exact(head.last_issued)], ["已退休至", exact(head.retired_through)], ["压力 / 过大 / 其他跳过", `${head.skipped_pressure} / ${head.skipped_oversize} / ${head.skipped_other}`], ["观测", time(head.observed_at)]]) : null));
}
