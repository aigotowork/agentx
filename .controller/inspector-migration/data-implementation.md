---
schema: document
status: completed
owner: inspector-data-fine
description: Read-only evidence and handoff for the Inspector native data foundation assessment.
links: [data-foundation.md, delivery.md, ../../.docs/inspector/data-foundation.md]
---

# Data foundation assessment record

## Authorized boundary

Read [assignment](data-foundation.md) and [Delivery](delivery.md) first. Write only this record and [the canonical assessment](../../.docs/inspector/data-foundation.md). Source Agent Inspector and AgentX native code are read-only; no runtime/private-data queries, native/API edits, subagents or broad tests. The current UI uses explicit offline mock supply; this assessment establishes source support, not live integration.

## Method and progress

The assessment traces each maintained Inspector capability from its source domain/contracts through AgentX native records, writers and read projections. The [completed canonical matrix](../../.docs/inspector/data-foundation.md#完整能力矩阵) distinguishes existing facts, derivations requiring an algorithm/coverage, existing HTTP, missing public exposure and missing retained facts/behavior. It covers sessions, hierarchy/workspace/team/roles, statuses/attention/read state, content/tool/error evidence, recovery/manual/automatic behavior, delivery, business and diagnostic events, statistics/analytics, token/cache/cost/quota/model comparison, insights/reports/jobs/recap, schedules/resume/archive/history, external Bots concepts, preferences and UI resources.

Initial source evidence: AgentX has native transcript token/cache/model/optional USD facts in `src/transcript/shared.rs`; management `ExchangeView` excludes them. This is partly an API exposure gap, while historical accounting remains a separate concern. Exact `AgentRef` and dataset generation must survive mapping; canonical/display names and provider session IDs cannot substitute for incarnation identity. Sent these findings directly to frontend owner `@beta` and acknowledged the assignment to `@lube` as `inspector-data-fine`.

The [portable-core table](../../.docs/inspector/data-foundation.md#监控核心的保留与替换) names reusable domain rules, attention/analytics helpers, lifecycle statistics, token contribution admission and snapshot queries, recovery state/guards/backoff, and passive management semantics. It separately identifies source SDK/IO/registration paths to replace. The assessment does not claim that these modules have already been copied into the new frontend; that evidence belongs to the UI implementation inventory.

Key dispositions: retain exact incarnation/stopped-event and generation; workspace adoption differs from cwd and parent relationships; management local-origin pagination differs from complete global history; model-request error support remains Hook-only; transcript usage is partial source evidence, not an accounting ledger; diagnostic ObservationJournal is volatile and distinct from durable business events; unknown permission/active-turn/prompt-baseline inputs must not become permissive recovery defaults. Bots Conversation, native Resume, generic Operation and collaboration Work retain their own meanings. Product decisions and minimal P0–P4 obligations live in the canonical assessment rather than a duplicate API catalog.

## Friction and disposition

The launch instruction arrived as ordinary user text with no AgentX request ID. `agentx send --intent ack` requires a request ID and rejects an informational message as an acknowledgement target. Used `--intent inform` to communicate the explicit acknowledgement rather than inventing a request ID. No task authority or scope changed.

## Verification and handoff

Completed static read-through against concrete native/source declarations and writers/query paths. Ran one focused Python check over Markdown links in both assigned documents: all local targets exist (zero missing targets). The assessment contains 139 lines at initial completion; paragraphs intentionally remain unwrapped. No build, test suite, browser, HTTP, provider, daemon or private-data query was run. The local-link check establishes navigability, not runtime behavior or semantic parity.

The usable matrix was sent to `@lube` and `@beta` before final closure. Final handoff releases these two documentation files to integration. This Task contribution is complete within its read-only boundary; Delivery acceptance, actual core/UI transplant and runnable Mock verification remain with their assigned owners. No native/API or source-repository file was edited.
