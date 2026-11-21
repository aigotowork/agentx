# Agent labels implementation

Read [the implementation Delivery](implementation.delivery.md), [Controller assignment](tasks/controller.task.md), [accepted Proposal](../../.docs/proposals/agent-labels.proposal.md) and [owning classification model](../../.docs/agent-classification.md). The user has now authorized implementation with a dedicated Controller and explicitly selected the default Codex `gpt-6.1-sol` high profile. The [completed design Task](design.task.md) retains its earlier design-only evidence boundary; that boundary does not block the newly authorized Delivery.

[Proposed DDL](../../.sketches/agent-labels/storage.sql) and [disposable SQLite evidence](storage-evidence.json) remain design evidence, not a production migration or end-to-end acceptance. Coordinate actual source/build release with [metadata cleanup](../agent-metadata/AGENTS.md) and [CLI selection](../cli-selection.task.md), preserve their existing work, and protect installed executables, managed data and running user services. The Controller is the single labels dispatcher; domain/design collaboration remains with raze.

The [discussion preview](../../.sketches/agent-labels/index.html) uses synthetic records only. It explains proposed filtering and grouping; it is not evidence of current AgentX API support. The Task owns preview run instructions and bounded verification.

## Implementation ownership and recovery

The [Delivery](implementation.delivery.md#collaboration-model) owns capability composition and actual bindings. Specialist assignments: [native](tasks/native.task.md), [CLI/HTTP](tasks/surfaces.task.md), [management UI](tasks/web.task.md), [independent acceptance](tasks/acceptance.task.md). nezi is sole dispatcher; workers have no further delegation. Use absolute project executable and explicit codex-gpt-6-1-sol-high profile. Source/build lanes released by tara #42579 and maho #42613; labels Cargo/package is coordinated by nezi. Preserve all existing edits, immutable selection/metadata candidates/runners, tara's default-binary adoption and live services.

## Current orchestration executable

Use absolute `/home/long/works/longwork/agentx/.bin/agentx` with default high profile. During dispatch tara adopted protocol3 bytes while the existing daemon still spoke protocol2, so the team temporarily used the retained `.bin/agentx.before-selection-20261116-200321` bridge. The scoped daemon is now independently observed as protocol3 PID3400255, and current project binary messaging succeeds; the old bridge now refuses. No labels worker restarted or mutated the live daemon. Do not infer further runtime-transition authority from this observation; preserve both binaries and live services.

## Accepted candidate and maintenance

Labels implementation is complete and independently accepted at [the fixed local candidate boundary](implementation.delivery.md#accepted-local-outcome). Follow native module guides and [actual case/evidence entry](../../tests/agent_labels/README.md) for maintenance. `.bin/agentx-labels-r2` and `.artifacts/agent-labels-lavu` are immutable accepted artifacts; current installed/default/live services are separate. Earlier pending implementation/dispatch notes in Task histories are historical. Message wake/distribution incidents are owned separately by [message delivery repair](../message-delivery/AGENTS.md), not a remaining labels defect.
