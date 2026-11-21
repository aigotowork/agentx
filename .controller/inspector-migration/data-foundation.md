---
schema: task
status: completed
owner: inspector-data-fine
description: Establish which native AgentX facts can supply the migrated Inspector and which require additional APIs or data.
---

# Native data foundation assignment

Read [Delivery](delivery.md), source `/home/long/works/longwork/paseo-retry/AGENTS.md`, `.docs/domain-model.md` and the source contracts owning each Inspector capability. Inspect target AgentX native contracts/storage/query paths plus `.docs/agent-management/design.md` and current `src/management/contract.rs`. Both codebases are read-only for this task. Write only `.docs/inspector/data-foundation.md` and `data-implementation.md` beside this task. No new HTTP/native implementation, no runtime queries to private data, no subagents or broad build/test execution.

Your result is consumed by the UI owner and future API integrator. Produce an evidence-based capability matrix for the entire maintained Inspector: sessions/identity/hierarchy/workspace, statuses/attention/read state, transcripts/tool execution/errors, recovery/manual continuation/auto policy, events/time series, token/cost/cache/quota and model comparison, analytics/insights/reports/jobs, schedules/delivery/archive/history, external Bots-like team/conversation/routine concepts, preferences and UI resources. Distinguish native facts, derivable projections requiring a defined algorithm/history, HTTP surfaces already declared, missing API-only exposure, and genuinely missing storage/behavior. Name units/identity/pagination/coverage where material; unknown is not zero and agent names are not immutable identities.

Link concrete source owners and types/functions rather than promising parity from a similar label. Do not invent public AgentX endpoints. Explain the minimal future adapter/data extensions, priorities and any assumptions needing product decisions. State that this phase deliberately uses mock supply and does not establish live behavior. Record contradictions and source/integration uncertainty with explicit disposition. Notify lube when the usable matrix exists; ordinary working updates stay in your note.

## Accepted result

Lube accepts the source-linked [data foundation assessment](../../.docs/inspector/data-foundation.md), with scope and focused link evidence in [implementation record](data-implementation.md). The matrix distinguishes existing native facts from missing exposure, derived history and absent workflows, and identifies portable monitoring core and unresolved product mapping decisions. This assignment closes at its read-only assessment boundary; no runtime/API parity is established. Frontend migration and independent verification remain active under the parent Delivery.
