/** Authored unit scenarios only: injected finite API is never a product entry. */
import assert from "node:assert/strict";
import test from "node:test";
import { isAbsolute, join } from "node:path";
import { pathToFileURL } from "node:url";

const bundle = process.env.OBSERVABILITY_BUNDLE;
if (!bundle || !isAbsolute(bundle)) throw new Error("Set OBSERVABILITY_BUNDLE to the newly built absolute disposable directory");
const { Investigation, eventWindow } = await import(pathToFileURL(join(bundle, "state.js")));
const { AdapterDisconnected, AdapterError } = await import(pathToFileURL(join(bundle, "bindings.js")));
const { configKey, duration, evidence, exact, relation } = await import(pathToFileURL(join(bundle, "presentation.js")));

const scope = { canonical_scope: "/test/explicit-scope" };
const daemon = { scope, daemon_instance: "daemon-one" };
const epoch = { daemon, database_generation: "generation-one" };
const at = { clock: "clock-one", monotonic_ms: 9007199254740993n, unix_ms: null };
const ready = { daemon, protocol_version: 1n, build_version: "test", build_id: "test", process: { pid: 1n, incarnation: "one" }, generation: { value: { Present: epoch.database_generation }, stamp: { origin: "DaemonBoundary", at } }, sampled_at: at };
const coverage = { request_details_dropped: 1n, retained_rows_evicted: 0n, peer_projections_dropped: 0n, host_projections_dropped: 0n, local_reports_dropped: 0n, full_host_census: false };
const snapshot = { daemon, sampled_at: at, generation: ready.generation, lifecycle: "Serving", activity: "Idle", coverage };
const page = { rows: [], next_before: null, sampled_at: at, revision: 1n, coverage };
const request = {
  reference: { daemon, id: 9007199254740995n }, connection: { daemon, id: 2n }, peer: { Unknown: "NotCaptured" }, origin: { invocation: { invocation: "invocation-one", revision: 1n }, attempt: "attempt-one" },
  request_id: "logical-one", operation: { domain: "Queries", variant: "Watermark" }, decoded_at: at,
  enqueued_at: null, dequeued_at: null, caller_deadline_unix_ms: null, effective_deadline: { Unknown: "NotCaptured" }, generation_at_decode: epoch.database_generation, generation_at_dispatch: null,
  generation_after_return: { Unknown: "NotCaptured" }, execution: "Decoded", reply: "Pending", configuration: "NotEvaluated", last_updated_at: at,
};
const refA = { epoch, sequence: 9007199254740997n };
const refB = { epoch, sequence: 9007199254740998n };
const losses = { sampled_at: at, dropped: { contended: 0n, oversize: 0n, invalid_input: 0n, retired_epoch: 0n, collector_unavailable: 0n, preparation_budget: 0n }, evicted: { age: 0n, count_pressure: 0n, byte_pressure: 0n }, suppressed: 0n };
const head = { stream: { epoch, nonce: "stream-one" }, oldest: 9007199254741000n, newest: 9007199254741010n, observed_at: at, losses };
const inspect = (response, override = {}) => ({ Inspection: { ready: { ...ready, ...override }, response, received_at: at } });
const runtime = (value, override) => inspect({ Runtime: { Ok: value } }, override);
const events = (value) => inspect({ Events: { Ok: value } });
const configuration = (value) => inspect({ Configuration: { Ok: value } });

function harness() {
  const replies = [];
  const calls = [];
  let closed = false;
  const api = { scope, async read(call, signal) {
    calls.push(call);
    const reply = replies.shift();
    assert.notEqual(reply, undefined, "unexpected read beyond the finite scenario");
    if (reply instanceof Error) throw reply;
    return typeof reply === "function" ? reply(signal) : reply;
  }, async close() { closed = true; } };
  const work = new Investigation(api);
  return { work, replies, calls, closed: () => closed };
}

async function bind(h) {
  h.replies.push(runtime({ Snapshot: snapshot }), runtime({ Requests: page }));
  await h.work.refresh();
  h.work.state.selected = request;
}

test("exact identity, duration and withheld relation remain independent", () => {
  assert.equal(exact(request.reference.id), "9007199254740995");
  assert.equal(duration(at, { ...at, monotonic_ms: at.monotonic_ms + 7n }), "7 ms");
  assert.equal(duration(at, { ...at, clock: "clock-two" }), "时钟不同 · 不可相减");
  assert.equal(evidence("Withheld"), "已隐去");
  assert.equal(evidence("Absent"), "明确缺席");
  assert.equal(relation({ Unknown: "Withheld" }), "未知 · Withheld");
  assert.notEqual(configKey(refA), configKey({ ...refA, epoch: { ...epoch, database_generation: "generation-two" } }));
});

test("frozen native event window preserves bigint and exact attempt selector", () => {
  const window = eventWindow(head, request.reference.id);
  assert.equal(window.after.sequence, 9007199254740999n);
  assert.equal(window.through, head.newest);
  assert.equal(window.filter.request, request.reference.id);
  const attempt = eventWindow(head, null, request.origin.attempt);
  assert.equal(attempt.filter.request, null);
  assert.equal(attempt.filter.client_attempt, "attempt-one");
});

test("adapter and daemon loss preserve selected evidence but keep different meanings", async () => {
  const h = harness(); await bind(h);
  h.replies.push(new AdapterDisconnected(new Error("test network")));
  await h.work.refresh();
  assert.equal(h.work.state.availability, "adapter-lost");
  assert.equal(h.work.state.selected, request);
  assert.equal(h.work.state.historical, true);
  h.replies.push({ LocalFailure: { stage: "Connect", kind: "ConnectionRefused", os_code: null, at } });
  await h.work.refresh();
  assert.equal(h.work.state.availability, "daemon-lost");
  assert.equal(h.work.state.selected, request);
  assert.match(h.work.state.message, /适配器可达/);
});

test("restart fences reads; explicit rebinding keeps old request historical without rewriting its ref", async () => {
  const h = harness(); await bind(h);
  const next = { ...daemon, daemon_instance: "daemon-two" };
  h.replies.push(inspect({ DaemonChanged: { expected: daemon, actual: next } }, { daemon: next }));
  await h.work.refresh();
  assert.equal(h.work.state.fenced, true);
  const count = h.calls.length;
  await h.work.loadList();
  assert.equal(h.calls.length, count);
  h.replies.push(runtime({ Snapshot: { ...snapshot, daemon: next } }, { daemon: next }), runtime({ Requests: page }, { daemon: next }));
  await h.work.enterCurrentContext();
  assert.equal(h.calls[count].expected_daemon, null);
  assert.equal(h.work.state.selected.reference.daemon.daemon_instance, "daemon-one");
  assert.equal(h.work.currentRequest(request.reference), false);
  assert.equal(h.work.state.fenced, false);
});

test("reset fences dataset joins and leaves captured operands unchanged", async () => {
  const h = harness(); await bind(h);
  h.work.state.left = refA;
  h.replies.push(runtime({ Snapshot: snapshot }, { generation: { ...ready.generation, value: { Present: "generation-two" } } }));
  await h.work.refresh();
  assert.equal(h.work.state.fenced, true);
  assert.equal(h.work.state.left, refA);
  assert.equal(h.work.state.generation, "generation-one");
});

test("A/B survives request navigation and uses native withheld comparison instead of inferred equality", async () => {
  const h = harness(); await bind(h);
  const lookup = (ref) => configuration({ Snapshot: { epoch, observed_at: at, result: { Missing: { reference: ref, reason: "NeverIssued" } } } });
  h.replies.push(lookup(refA)); await h.work.loadConfiguration(refA, "left");
  h.replies.push(runtime({ RequestDetail: { ...request, reference: { daemon, id: 7n } } }));
  await h.work.selectRequest({ daemon, id: 7n });
  h.replies.push(lookup(refB)); await h.work.loadConfiguration(refB, "right");
  const native = { Compared: { basis: {}, fields: [{ field: { Setting: "terminal" }, left: null, right: null, value: { Unknown: "Withheld" }, provenance: "Unknown" }] } };
  h.replies.push(configuration({ Comparison: { epoch, observed_at: at, result: native } }));
  await h.work.compare();
  assert.equal(h.work.state.left, refA);
  assert.equal(h.work.state.right, refB);
  assert.equal(h.work.state.comparison.result, native);
  assert.equal(h.calls.at(-1).read.ConfigurationCompare.fields, "All");
});

test("event gap halts until explicit retained continuation and preserves exact original through", async () => {
  const h = harness(); await bind(h);
  const retainedHead = { ...head, oldest: head.oldest + 3n };
  h.replies.push(events({ Head: head }), events({ Read: { Gap: { first_missing: head.oldest, last_missing: head.oldest + 2n, head: retainedHead } } }));
  await h.work.openEvents();
  assert.equal(h.work.state.events.boundary, "Gap");
  const before = h.calls.length;
  await h.work.nextEvents();
  assert.equal(h.calls.length, before);
  const original = h.work.state.events.window;
  const continuation = { ...original, after: { stream: head.stream, sequence: retainedHead.oldest - 1n } };
  h.replies.push(events({ Read: { Page: { records: [], next: { ...continuation, after: { ...continuation.after, sequence: head.newest } }, head: retainedHead } } }));
  await h.work.continueRetainedEvents();
  assert.equal(h.calls.at(-1).read.EventsRead.window.through, head.newest);
  assert.equal(h.calls.at(-1).read.EventsRead.window.after.sequence, retainedHead.oldest - 1n);
  assert.equal(h.work.state.events.gaps.length, 1);
  assert.equal(h.work.state.events.window.after.sequence, head.newest);
});

test("stream replacement never automatically opens a replacement window", async () => {
  const h = harness(); await bind(h);
  const changed = { ...head, stream: { ...head.stream, nonce: "stream-two" } };
  h.replies.push(events({ Head: head }), events({ Read: { StreamChanged: { requested: head.stream, head: changed } } }));
  await h.work.openEvents();
  assert.equal(h.work.state.events.boundary, "StreamChanged");
  const count = h.calls.length;
  await h.work.nextEvents();
  assert.equal(h.calls.length, count);
  assert.equal(h.work.state.events.window.after.stream.nonce, "stream-one");
});

test("adapter rejection retains safe evidence; close cancels no product lifetime", async () => {
  const h = harness(); await bind(h);
  h.replies.push(new AdapterError(503, "Busy"));
  await h.work.refresh();
  assert.equal(h.work.state.availability, "connected");
  assert.equal(h.work.state.historical, true);
  assert.equal(h.work.state.message, "适配器拒绝读取 · Busy");
  assert.equal(h.work.state.selected, request);
  await h.work.close();
  assert.equal(h.closed(), true);
  assert.equal(h.work.state.availability, "closed");
  const count = h.calls.length;
  await h.work.refresh();
  assert.equal(h.calls.length, count);
});
