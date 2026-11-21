---
schema: task
status: completed
owner: kera
last_updated: 2026-11-14
description: Implement hook-observed model request errors as governable participant state.
---

# Outcome

Implement the user-selected [state model](../../.docs/model/model-request-errors.md), including Codex hooks, durable state/events, query visibility and yellow TUI presentation. No recovering state, app-server connection, automatic retry engine or live runtime replacement.

# Work

1. Establish native observation/state types and additive storage skeleton.
2. Implement fenced transactional state transitions and sticky error lifecycle integration.
3. Connect Codex synchronous hooks, existing query surfaces and TUI presentation.
4. Validate failure/recovery/ordering/persistence and consumer behavior in disposable scope.

# Evidence and limits

Design established from current source plus official Codex hooks and tagged 0.169.0 request-hook implementation, linked in the existing feedback. Same-turn arbitrary callback correlation is limited by absence of a native attempt identifier. No native delegation tool is exposed in this session; implement and run bounded checks inline, retaining this limitation rather than spawning through a different product.


# Implementation checkpoint

Native types, v25 additive JSON projection, participant observation operation, sticky error lifecycle updates, synchronous Codex model hooks, CLI JSON/event fields and TUI error styling are implemented. The existing generic status string already included error; the TUI enum previously did not and now recognizes it. Error source is `model_request`, context is `model:request`, and other errors retain red styling. Per-attempt usage is intentionally not persisted in this state slice.

Validation: 119 relevant tests passed, zero failed, one process acceptance test deferred until the candidate executable is built. Covered v24 migration/stamped schema compatibility, lifecycle, error persistence/recovery/ordering, hook registration and rendered yellow TUI indicator. `git diff --check` passed for the affected paths. The strict DB-ownership scan recognizes the new model-request adapter and reports three unrelated violations in concurrently authored `src/agents/context_store.rs`; none are attributed to this change. Do not modify that separate work merely to clear the scan. Native process acceptance and build are in progress.

End-to-end correction: the first native process case confirmed error transitions, then detected that the single-agent CLI JSON path omitted structured model metadata. That path is now fixed; the same exact process case will be rerun against the rebuilt candidate. This is separate from the already-passing domain and TUI checks.


Native process acceptance passed using the rebuilt development binary: exact test `daemon::acceptance_tests::process_codex_model_request_error_lifecycle` drives real Hook child processes against a disposable daemon, checks failure -> sticky error across Stop/retry -> success -> listening, and verifies single-Agent CLI JSON exposes model error metadata. Candidate cohort `native-build-v1:73c3813b-6881-4f8a-9372-898405b8a213`, SHA-256 `dd52019921ce9757130fd01f43f45c51031c4d5d9b772a28c95cdc498f37801a`. The final source follow-up adds source precedence so a successful model request cannot clear a different error; its bounded unit/static regression is still running. No native vendor outage or app-server was used.


Final boundary regression found and corrected a preservation bug: the branch retaining an unrelated error reused the approval branch's `blocked` literal. It now preserves the original status/context/detail exactly. The new regression exercises request start, failure and success while a separate runtime error remains. Request metadata is boxed in InstanceRow to avoid inflating every participant and its enclosing response variants; wire JSON is unchanged. Final scoped checks are running on these changes.


# Completed local outcome

The Hook-only slice is complete. The initial 119-test affected suite passed; after the final source-precedence and boxed-projection changes, the focused final run `just test --bin agentx -- model_request agents::lifecycle::tests` passed 46 tests, with the one native process case excluded from that run because it was explicitly executed separately. The corrected native Hook -> daemon -> single-agent CLI case passed as recorded above. `cargo clippy --locked --bin agentx --no-deps` completed with warnings in existing larger enums/router/launcher; no new model-request module warning was reported. The later boxed projection avoids adding the entire error state inline to existing participant response carriers. Affected source diff whitespace checks passed.

No App Server integration, recovery scheduler, installed program replacement or live user daemon change was performed. The development acceptance executable recorded above predates the final source-precedence-only refinement; the final Rust test executable contains that refinement and its passing regression. Version 25 migration was tested with disposable v24 data, including preservation of participant/session records. Actual vendor model failures, arbitrary same-turn callback disorder and platforms beyond this Linux fixture remain unverified. Existing live sessions require newly configured hooks through a later launch/resume after adoption of the build. The DB ownership scan's unrelated context_store findings remain outside this task.
