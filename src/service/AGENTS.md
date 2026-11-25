# Native AgentX service boundary

Read [architecture](../../.docs/architecture/README.md), [contracts](../../.docs/architecture/contracts.md), [principles](../../.docs/architecture/principles.md) and the [source ownership handoff](../../.controller/domain-daemon/contracts-handoff.md).

Use the [storage-boundary model](../../.docs/model/storage-boundary.md) for exact constructor exceptions, borrowed daemon adapters/helpers and remaining mixed host call paths. A store filename or runtime directory does not authorize storage construction or client fallback.

| Responsibility | Authority |
| --- | --- |
| Native aggregate operations and application injection | `mod.rs`; architecture coordinates additive shared edits |
| Per-invocation configuration and observed caller authority | `context.rs`; transport observes process evidence, domains validate claims |
| Public application failure meaning | `error.rs`; transport failures remain separately owned |
| Participant/session/attachment and messaging/inbox operations | `agents` and `messaging` native declarations |
| Concrete persistence composition | `application.rs`; constructed only by daemon entry, never clients |
| Local client transport | `client.rs`; connects through the daemon lifecycle adapter, never opens SQLite |
| Product read models and maintenance | `queries.rs` / `maintenance.rs` own native contracts; `queries_store.rs` / `maintenance_store.rs` run only inside daemon composition |
| Bounded work delivery | [Collaboration](../collaboration/AGENTS.md) owns its new native Request/Response leaf and aggregate invariants; `application.rs` injects daemon-owned storage, never a browser-native proxy |
| Inspector collection and analysis | [Inspection](../inspection/AGENTS.md) owns durable sampled observations, pinned jobs and episode-fenced continuation audit; runtime reads Provider files outside native transactions. |
| Agent management HTTP operations | [Management](../management/AGENTS.md) owns exact Agent projections and durable operation admission; host effects run in separate workers, while the HTTP adapter maps bounded product inputs rather than exposing this aggregate |

Capture client invocation with `InvocationContext::capture` or map a previously captured effective host context. Apply parsed identity/session overrides explicitly; environment is configuration rather than authentication. Participant lifecycle stays under `agents`, not a second implementation here. New aggregate variants require a real new capability; sub-operations of an existing domain extend that domain's native leaf.

`queries::AgentFilter` scopes candidate and TUI Agent reads within the already acquired data scope. Native workspace matching uses saved collaboration data; label selection reuses the current-label query and exact participant references. Explicit filtered snapshots and terminal queries use distinct variants so an older daemon cannot silently ignore the scope. Legacy unfiltered reads and scope-wide timeline/search remain separate. See the [CLI selection design](../../.docs/proposals/agent-target-selection.md).

[Agent Info queries](queries_info.rs) carry optional exact references on TUI snapshots and run detail reads on the existing read lane. Local views pin the selected scope and database generation; current targets require exact incarnation bits, stopped targets require the exact retained event. A current link may follow its indexed stop event after retirement. Bootstrap reads the retained participant snapshot without persisting aliases. The file worker matches current Task/Delivery source observations by scope, generation and exact assignment; the TUI reads document bodies only when opened.

Rust declarations and their serde projections are one contract. Do not create editable JSON schemas or a generic SQL/argv API. Do not use this directory as a second domain implementation: invariants stay in the offered domain's implementation. A lost reply after submission is an unknown outcome unless that operation provides durable lookup/deduplication. Full environment forwarding is explicit request configuration and grants no identity authority.

## Observation insertion boundaries

`codex_hook.rs` owns the combined native PTY callback; keep Provider JSON in the [adapter](../integrations/hooks/codex/client.rs), identity and status in `agents`, and delivery/receipts in `messaging`. The hook sends a direct Call with its retained generation when available. Only an explicit unsupported rejection allows the older hook path; an unknown result never authorizes a second request. See the [implementation task](../../.controller/observability/codex-hook-performance.task.md).

Read the [native observation guide](../observability/AGENTS.md), [configuration loader guide](../config/AGENTS.md) and [composition handoff](../../.controller/observability/implementation-tasks/composition.md). `observation.rs` owns bounded origin-local attempt capture before fallible acquisition/write and before error flattening. It retains <=12 ordered spans and the native bounded report queue; no environment, body, argv, capability, raw error text, file spool, background publication or business retry enters it. Call knowledge stays separate from transport failure; a possible write remains uncertain until an actual native reply is received. `InvocationContext.observation_origin` is an optional caller correlation claim only.

`context.rs::ActiveObservation` is an instrumentation-only per-dispatch adapter. It freezes exact epoch, shared daemon clock, request/origin/accepted-peer refs and narrow recorder/sink ports. TLS RAII restores the previous state during normal return, nested scopes and unwind. Business identity, configuration selection and authorization never consult this TLS. No active context means skipped/unavailable observation with unchanged business result.

Passive CLI input acquisition uses `InvocationContext::capture_observed`; it shares the ordinary capture's one-pass input assembly but returns original structured LocalFailure before service conversion. Known IO cause/stage/OS code survives a missing clock timestamp. Required attempt start/end/span times remain mandatory; skip that report when unavailable instead of synthesizing chronology. A direct native failure result does not depend on retention of a local report.

Same-pass declaration/effective/participant/dev-root projections buffer at most eight safe configuration bodies and publish after the handler and SQLite guards leave scope. Historical capture never rereads sources. Subject/stage/purpose and actual consumed subset remain native Configuration policy. Positive host callbacks follow successful native host transactions; ports receive only policy-filtered originating inputs and validated participant/execution/process refs, never control tokens. Current production host dispatch is top-level under Application; a new nested host transaction must defer its projection to outer commit before adding such a caller.

Domain postcommit preparation captures epoch, correlation, clock, sink and safe typed fact now; later publication must not consult current TLS to relabel it. `../db/post_commit.rs` owns nested rollback and final commit publication. The original daemon Application/maintenance contracts retain all durable generation/reset authority. Authored origin and buffered-source cases live in `observation.rs::tests` and `observation_context_tests.rs`; independent native integration owns their execution.

Maintenance dataset replacement temporarily removes only this instrumentation context while writing the new dataset. Its new rows must not be projected with the admitted old epoch before Application publishes the committed replacement. RAII restores the old request's context, and runtime chronology retains its original decode generation plus actual generation-after result. This deliberate selective gap does not replace reset/domain authority or fabricate new-era facts.
