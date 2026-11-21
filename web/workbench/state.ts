/** @aiPurpose Keep finite UI projections, draft validation and original-command recovery pure. */
import type { Assignment, BriefInput, Candidate, Failure, Limits, ParticipantRef, Role, Work, WorkDetail } from "./api/native";
import type { BriefDraft, DetailTab, PendingReference, ResourceView, Selection, WorkFilter } from "./contracts";

/**
 * @aiPurpose Identify exact native participants.
 * @aiUsage Never compare participants by display name alone.
 */
export function participantKey(participant: ParticipantRef): string { return `${participant.canonical_name}:${participant.incarnation_bits}`; }
/**
 * @aiPurpose Give an unsaved criterion stable page-local identity on HTTP LAN.
 * @aiUsage This creates no native work or command identity.
 */
export function newCriterionId(): string { return Array.from(crypto.getRandomValues(new Uint8Array(16)), value => value.toString(16).padStart(2, "0")).join(""); }
/**
 * @aiPurpose Resolve a current role binding.
 * @aiUsage Historical assignments stay in the native work.
 */
export function assignment(work: Work, role: Role): Assignment | undefined { return work.assignments.find(item => item.id === (role === "Implementer" ? work.implementer : work.checker)); }
/**
 * @aiPurpose Read the current brief.
 * @aiUsage Native appends immutable brief history.
 */
export function currentBrief(work: Work): BriefInput { return work.briefs[work.briefs.length - 1]!.input; }
/**
 * @aiPurpose Name a source group from a saved reference.
 * @aiUsage This does not read source files.
 */
export function sourceGroup(work: Work): string { const reference = currentBrief(work).source?.reference; if (!reference) return "未关联来源"; const slash = reference.lastIndexOf("/"); return slash >= 0 ? reference.slice(0, slash + 1) : reference; }
/**
 * @aiPurpose Present native failures at their action.
 * @aiUsage Do not convert uncertainty to a domain rejection.
 */
export function failureText(failure: Failure | string): string {
  if (typeof failure === "object") {
    if ("InvalidInput" in failure) return `${failure.InvalidInput.field}：${failure.InvalidInput.reason}`;
    if ("RevisionConflict" in failure) return `任务已更新 · 修订 ${failure.RevisionConflict.current_revision}`;
    if ("Ineligible" in failure) return failure.Ineligible.reasons.join("；");
  }
  const labels: Record<string, string> = { Forbidden: "当前入口不允许此操作", StaleReference: "数据已更换，请刷新工作", NotFound: "记录未找到", Capacity: "已达到原生记录上限", Closed: "工作已接受", CommandConflict: "原命令与输入不一致", Unavailable: "服务不可用" };
  return labels[String(failure)] ?? String(failure);
}
/**
 * @aiPurpose Seed a page-local editor.
 * @aiUsage New drafts have no native identity and do not enter resource counts.
 */
export function makeDraft(detail: WorkDetail | null): BriefDraft {
  const input = detail ? structuredClone(currentBrief(detail.work)) : { title: "", goal: "", criteria: [{ id: newCriterionId(), text: "", required: true }], source: null };
  return { input, base: detail ? structuredClone(input) : null, revision: detail?.work.revision ?? null, implementer: detail ? participantKey(assignment(detail.work, "Implementer")!.participant) : "", checker: detail ? participantKey(assignment(detail.work, "Checker")!.participant) : "", touched: false, dirty: false };
}
/**
 * @aiPurpose Validate fields using native bounds and exact candidates.
 * @aiUsage Native remains authoritative at submission.
 */
export function draftErrors(draft: BriefDraft, candidates: Candidate[], limits: Pick<Limits, "text" | "title" | "criteria" | "source_reference" | "source_revision">, creating: boolean): Record<string, string> {
  const errors: Record<string, string> = {};
  const check = (key: string, value: string, limit: bigint) => { if (!value.trim()) errors[key] = "请填写此项"; else if (BigInt(new TextEncoder().encode(value).length) > limit) errors[key] = `最多 ${limit} UTF-8 字节`; };
  check("title", draft.input.title, limits.title); check("goal", draft.input.goal, limits.text);
  if (!draft.input.criteria.length || BigInt(draft.input.criteria.length) > limits.criteria) errors.criteria = `条件数需为 1–${limits.criteria}`;
  else if (!draft.input.criteria.some(item => item.required)) errors.criteria = "至少一项条件必须为必需";
  draft.input.criteria.forEach(item => check(`criterion-${item.id}`, item.text, limits.text));
  if (draft.input.source) { check("source", draft.input.source.reference, limits.source_reference); check("sourceRevision", draft.input.source.revision, limits.source_revision); }
  if (creating) for (const role of ["implementer", "checker"] as const) { const candidate = candidates.find(item => participantKey(item.participant) === draft[role]); if (!candidate) errors[role] = "请选择已运行的 Agent"; else if (candidate.unavailable) errors[role] = candidate.unavailable; }
  if (creating && draft.implementer && draft.implementer === draft.checker) errors.checker = "检查者不能与实现者为同一身份";
  return errors;
}
/**
 * @aiPurpose Filter only loaded work.
 * @aiUsage Search terms never imply full-library or file-content search.
 */
export function filterWork(details: WorkDetail[], query: string, source: string | null, filter: WorkFilter): WorkDetail[] {
  const needle = query.trim().toLocaleLowerCase();
  return details.filter(detail => {
    const work = detail.work; const brief = currentBrief(work);
    const searchable = [brief.title, brief.goal, brief.source?.reference ?? "", ...work.assignments.map(item => item.participant.canonical_name), ...work.results.flatMap(item => [item.id, item.input.summary, ...item.input.artifacts.map(artifact => Object.values(artifact).map(value => Object.values(value).join(" ")).join(" "))])].join(" ").toLocaleLowerCase();
    const mine = work.questions.some(question => !question.answer) || !!detail.eligibility.selection || !assignment(work, "Implementer")?.dispatched_brief || !!work.results.length && !work.check_requests.length;
    const checking = work.check_requests.some(request => !work.checks.some(check => check.input.request_id === request.id));
    const active = !detail.eligibility.closed && (work.messages.some(message => message.kind === "Assignment") || !!work.progress.length);
    return (!needle || searchable.includes(needle)) && (!source || sourceGroup(work) === source) && (filter === "all" || filter === "mine" && mine || filter === "checking" && checking || filter === "active" && active);
  });
}
/**
 * @aiPurpose Parse bounded copyable object addresses.
 * @aiUsage Payloads and draft text never enter the URL.
 */
export function readAddress(hash: string): { selection: Selection; view: ResourceView; generation: string | null } {
  const params = new URLSearchParams(hash.replace(/^#/, ""));
  const tabs: DetailTab[] = ["brief", "results", "checks", "messages", "edit"];
  const views: ResourceView[] = ["work", "agents", "results", "checks"];
  const tab = params.get("tab") as DetailTab; const view = params.get("view") as ResourceView;
  return { selection: { workId: params.get("work"), resultId: params.get("result"), tab: tabs.includes(tab) ? tab : "brief", questionId: null, participant: null }, view: views.includes(view) ? view : "work", generation: params.get("generation") };
}
/**
 * @aiPurpose Retain only a finite pending reference.
 * @aiUsage No command input or user work is restored for replay.
 */
export function parsePending(value: string | null): PendingReference | null {
  if (!value || value.length > 2048) return null;
  try { const parsed = JSON.parse(value) as PendingReference; if (![parsed.commandId, parsed.scope, parsed.project, parsed.generation].every(item => typeof item === "string" && item.length > 0 && item.length < 512)) return null; const work = parsed.work && typeof parsed.work === "object" && parsed.work.scope === parsed.scope && parsed.work.project === parsed.project && parsed.work.generation === parsed.generation && typeof parsed.work.id === "string" && parsed.work.id.length < 512 ? { scope: parsed.scope, project: parsed.project, generation: parsed.generation, id: parsed.work.id } : null; return { commandId: parsed.commandId, scope: parsed.scope, project: parsed.project, generation: parsed.generation, work }; } catch { return null; }
}
