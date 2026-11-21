---
schema: task
description: Implement isolated management execution using existing launch, messaging, resume, kill and transcript capabilities.
status: completed
---

# Runtime adapter

Read the team guide, design and `src/management/contract.rs`. `src/management/worker.rs` implements the early internal `__management-worker` dispatch. Consume bounded stdin, verify scope/generation, claim the operation using its private ticket, execute the exact command returned by that claim, and finish with a bounded result. No managed SQLite, CLI text parsing or global mutation in the daemon. The worker process alone establishes its operation cwd/environment before capture. Runtime implementation was handed to integration; `store/tests.rs` independently covers journal invariants that public HTTP cannot directly inspect. Integration owns all Cargo/build execution.

Source handoff and all 9 journal cases are complete; execution evidence is [native-journal.log](../verification/native-journal.log). Native Provider effects are covered separately by the Delivery's public acceptance, not by these store tests.

M02 follow-up: `store/tests.rs::local_null_and_empty_origins_are_listed_and_selectable_across_pages` adds an independent regression for native local origin represented by either SQL NULL or an empty string. Interleaved nonempty remote origins must be excluded from listing and exact selection; page sizes one and two must preserve the same ordered local set, advancing cursors and finite termination. Source is handed to gabi for integration execution against the owning list-predicate repair; this follow-up has not been executed by the runtime worker. Earlier nine-case evidence remains historical for this added case.

F03 follow-up: [native hook-session cases](../../../src/agents/hook_store/session_tests.rs) independently cover launched callbacks with missing, deleted or dangling process bindings, successor-session selection after retirement, killed-marker HookSession/Status rejection, retained HookSession replay after a kill marker, and preservation of current first binding/session switching plus genuinely unlaunched adoption. Rejection snapshots include all participant rows, process/session binding timestamps and event count/frontier. Both creation modes are covered; callbacks without creation permission must not select a successor through a lost launch binding. Tests are source-authored and handed to gabi for scoped native execution, with no worker-side Cargo/build or Provider effects. The native binding/status guards and any production repair remain gabi's ownership.

Integration executed the six new admission groups with affected existing native cases: [26 passed](../verification/native-retirement-r4-receipt.json). Source handoff and this follow-up are complete; public HTTP/browser F03 acceptance remains with the independent validator.
