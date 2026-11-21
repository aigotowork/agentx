/** @aiPurpose Author regression cases for identity, local drafts, byte limits and bounded recovery. */
import assert from "node:assert/strict";
import test from "node:test";
import { draftErrors, filterWork, makeDraft, parsePending, participantKey, readAddress } from "../state.ts";
import type { Candidate, WorkDetail } from "../api/native.ts";

const limits = { text: 8192n, title: 240n, criteria: 32n, source_reference: 2048n, source_revision: 256n };
const candidate = (incarnation: bigint, unavailable: string | null = null): Candidate => ({ participant: { canonical_name: "Lin", incarnation_bits: incarnation }, label: "Lin", tool: "codex", runtime_status: "busy", unavailable });

test("create draft has no native identity and same-name distinct incarnations remain distinct", () => {
  const draft = makeDraft(null); const first = candidate(9007199254740993n); const second = candidate(9007199254740994n);
  draft.input.title = "CSV"; draft.input.goal = "Export"; draft.input.criteria[0]!.text = "No silent overwrite"; draft.implementer = participantKey(first.participant); draft.checker = participantKey(second.participant);
  assert.notEqual(draft.implementer, draft.checker); assert.equal("reference" in draft, false); assert.deepEqual(draftErrors(draft, [first, second], limits, true), {});
  draft.checker = draft.implementer; assert.match(draftErrors(draft, [first, second], limits, true).checker!, /同一身份/);
});

test("native byte limits, required criteria and unavailable candidates are meaningful local validation", () => {
  const draft = makeDraft(null); draft.input.title = "界".repeat(81); draft.input.goal = "Goal"; draft.input.criteria[0]!.text = "Check"; draft.input.criteria[0]!.required = false; const first = candidate(1n, "Identity expired"); const second = candidate(2n); draft.implementer = participantKey(first.participant); draft.checker = participantKey(second.participant);
  const errors = draftErrors(draft, [first, second], limits, true); assert.match(errors.title!, /UTF-8/); assert.match(errors.criteria!, /必需/); assert.equal(errors.implementer, "Identity expired");
});

test("pending restore admits only a bounded reference and cannot recreate an intent", () => {
  const ref = { commandId: "cmd-1", generation: "g1", scope: "s", project: "p", work: null };
  assert.equal(parsePending(JSON.stringify(ref))?.commandId, "cmd-1"); assert.equal(parsePending('{"commandId":3}'), null); assert.equal(parsePending("x".repeat(2049)), null); assert.equal(parsePending(null), null);
});

test("addresses retain exact evidence identity and reject unsupported views/tabs", () => {
  const address = readAddress("#work=w1&result=r1&tab=checks&generation=g1&view=results"); assert.equal(address.selection.resultId, "r1"); assert.equal(address.selection.workId, "w1"); assert.equal(address.generation, "g1"); assert.equal(readAddress("#tab=terminal&view=files").selection.tab, "brief");
});

test("loaded-work filters use explicit questions/dispatch and never runtime busy", () => {
  const detail: WorkDetail = { eligibility: { selection: null, reasons: ["No result"], closed: false }, work: { reference: { scope: "s", generation: "g", project: "p", id: "w" }, revision: 1n, briefs: [{ revision: 1n, input: { title: "CSV export", goal: "Goal", criteria: [{ id: "c", text: "Check", required: true }], source: { reference: ".tasks/cli/csv.md", revision: "rev" } } }], assignments: [{ id: "a", role: "Implementer", participant: { canonical_name: "Lin", incarnation_bits: 1n }, dispatched_brief: null }, { id: "b", role: "Checker", participant: { canonical_name: "Moss", incarnation_bits: 2n }, dispatched_brief: null }], implementer: "a", checker: "b", messages: [], progress: [], questions: [], results: [], check_requests: [], checks: [], decisions: [], created_at: "" } };
  assert.equal(filterWork([detail], "Lin", ".tasks/cli/", "mine").length, 1); assert.equal(filterWork([detail], "", null, "active").length, 0); assert.equal(filterWork([detail], "missing", null, "all").length, 0);
  const draft = makeDraft(detail); draft.input.title = "Local only"; assert.equal(detail.work.briefs[0]!.input.title, "CSV export");
});
