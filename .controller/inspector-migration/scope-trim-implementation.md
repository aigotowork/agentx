---
schema: document
status: completed
owner: inspector-trim-dele
links: [scope-trim.md, ../../web/inspector/docs/migration.md]
---

# Scope cleanup implementation

## Affected-path plan

Remove account quota hooks/cards/provider cases/fixtures and Token/dashboard bindings. Remove schedule surfaces, heartbeat badges, scheduled continuation, deferred execution management and schedule forms/provider cases/fixtures together. Preserve ordinary immediate sending with its confirmation and uncertain-outcome guards, stop evidence, archive/restore, timeline, recovery audit and analysis job history. Remove money fields from current usage/analysis normalization and presentation while retaining Token counters. Historical design resources remain explicitly historical. Source-only dependency inspection is used because the available codebase-memory index belongs to another checkout.

Verify all eight tabs, Token-only data, Agent detail, recovery/analysis paths and removed-query absence using the module browser tools after scoped typecheck/build. Keep LAN PID 3311357 alive; successful build replaces served dist. Update current module model, provenance inventory and data assessment. Recovery duration is a documented future native rule, not an executable collector or real action.

## Friction

The module guide's old independently owned test restriction predates this explicitly authorized scope-trim verification. This task owns affected test updates; old acceptance remains historical. AgentX requires reply-to for ack; identity was delivered using inform instead.

## Delivered result

Removed executable quota cards/hooks/fixtures/query operations, account reset presentation, monetary usage/analytics/report fields, schedule surfaces/links/forms/fixtures/provider cases, heartbeat badges and wakeups, scheduled continuation and deferred execution/run-history management. Immediate messaging now has its own `messages` modules and finite send/list operations: no enqueue/cancel/resume descriptor or 300 ms queue flusher. Source generic historical types and UIUX/source-design snapshots remain explicit historical resources. Provenance retains original hashes and records removed/renamed/adapted current dispositions.

Preserved all eight main tabs; Token counters/cache and usage algorithms; ordinary archive/restore, stop evidence and explicit Provider handover; timeline/tool/event evidence; recovery incidents/attempt audit and accepted-versus-recovered/unknown barriers; analysis reports/feedback/jobs/recap. Archive candidates retain directory completeness and descendant checks without heartbeat queries. Provider handover is separate from removed scheduled-resume cancellation. No native/API/background action was implemented or performed.

Current capability assessment: [data foundation](../../.docs/inspector/data-foundation.md) records collection-start history (no backfill), optional Provider facts, remaining safe projections/history/read-preferences/analysis/recovery persistence, and configurable continuous unresolved-error duration (example/default proposal 600 seconds). Native Started preserves the error episode and may mean an active Provider retry; blocked/inactive take precedence. Future execution must use a runtime-supported action with same-instance/episode/phase checks, one outstanding reservation and no duplicate after unknown. Native error never authorizes ordinary PTY injection. Existing source recovery/backoff remains visibly simulated and does not implement this future rule.

## Executed checks and evidence

- Scoped `npm run typecheck` and `npm run build` passed on the final runtime change (removing residual reset-time text from attention reasons). Logs: [typecheck](../../web/inspector/tests/evidence/2026-11-15/scope-trim/typecheck.log), [build](../../web/inspector/tests/evidence/2026-11-15/scope-trim/build.log). Initial compile/import fallout from removed/renamed schemas was fixed; no package/dependency changes.
- `node_modules/.bin/tsx tests/scope-contracts.ts` passed: 11 removed operations reject, exported usage is Token-only including known zero, immediate ledger has only send/list, and uncertain same-ID replay delivers exactly once.
- Six affected browser groups passed: eight-tab navigation/Token-only display, selected detail/timeline/theme/draft continuity, analysis report/job history, recovery accepted/unknown audit, retained auxiliary and removed-route fallback, and 390×844 reachability/no horizontal overflow. [Initial result](../../web/inspector/tests/evidence/2026-11-15/scope-trim/result.json), adjacent screenshots/text. The initial result statuses were reconstructed transparently from captured successful executor stdout after a zero-match filter overwrote the summary; existing captures remain original. No browser result was inferred from screenshots.
- The additional immediate-send-and-handover group passed against final dist: one simulated send and one simulated create, no removed operation/UI path. [Result](../../web/inspector/tests/evidence/2026-11-15/scope-trim/result-immediate-send-and-handover.json). Two earlier failed attempts are preserved (`result-immediate-attempt1.json`, `result-immediate-attempt2.json`): test selectors said “加入这一轮”/“额度耗尽” while the source labels are “插入这一轮”/“额度用尽”; correcting selectors resolved these failures.
- Actual esbuild inputs contain none of the removed executable modules and retain immediate messaging, stop/handover, analysis jobs and recovery core: [input report](../../web/inspector/tests/evidence/2026-11-15/scope-trim/build-inputs.json). Browser evidence reported zero page errors and zero fetch/XHR/WebSocket/EventSource traffic.

Visual inspection question: after card removal, does narrow detail still prioritize identity, result evidence and the main action? The 390×844 pixel-dark capture shows Agent title/status, Token/tool totals, local detail tabs, latest prompt/reply and bottom “打开会话” clearly; no removed-card gap or horizontal clipping. This supports that viewport only; it is not a physical mobile/accessibility acceptance claim.

## Process cleanup and limits

LAN PID 3311357 (`node scripts/serve.mjs`, port 18971) remains alive and was never restarted. Successful normal build refreshed its served dist. Obsolete generated JS/CSS chunks absent from the final metafile were removed so historical dist assets do not linger. No temporary servers were created; Chrome contexts closed and one-time transformation scripts were removed. Original migration evidence remains historical; current checks live in `tests/scope-trim.mjs` and `tests/scope-contracts.ts`. No broad S01–S12 rerun or separate reviewer/subagent was used. Native/live HTTP integration and real sustained-error execution remain unimplemented.
