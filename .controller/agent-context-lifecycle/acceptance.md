# Independent lifecycle acceptance

Status: **CHECKPOINT_RELEASED**. Work stopped at the user's explicit handoff instruction; continuation belongs to an AgentX successor, with no Paseo orchestration. **No final candidate handoff received, no candidate or native execution performed**. Authority: [verification task](verification.md), [selected design](design.md). Product changes and plugin-owned tests remain outside this assignment.

## Cases and evidence boundaries

| Case | Input/state transition | Required observable result | Verification surface |
| --- | --- | --- | --- |
| L01 | Legacy absent/empty/disabled/file/mixed selector; explicit instruction selector | Preserve compatibility partition, independent lists and order | [composition.rs](../../tests/agent_context_lifecycle/composition.rs) |
| L02 | Fixed and dynamic builtin contributors at initial and compact boundaries | Typed identity/session/reason passed only to matching port; wrong entry rejected | Composition suite |
| L03 | Fixed file serialized, edited then removed; dynamic file edited, emptied, removed | Frozen value roundtrips unchanged; fresh dynamic values and explicit clearing; missing dynamic source fails | Composition suite; durable recovery still requires candidate |
| L04 | Launch then changed/invalid instruction selection and removed file | Actual stored fixed value read before source/config evaluation | Candidate hooks and resume |
| L05 | Same-name exact-session resume; new identity/fork; same name different incarnation | Resume reuses fixed content; foreign identity never inherits identity text | Candidate native lifecycle |
| L06 | Prior name/bootstrap receipt, repeated initial callback, repeated compact | Initial dynamic delivered once independently; compact refresh bypasses once gate | Candidate hooks/native requests |
| L07 | Missing/empty/foreign/stale native session | No output, rebinding, state mutation, mailbox acknowledgement or user turn | Candidate hooks with before/after state |
| L08 | Real Codex initial request then manual/automatic native compact | Only fixed text in developer channel; fresh dynamic context before model request; unchanged user message count | Existing loopback fixture adapted under new test directory |
| L09 | Provider channel capability and plugin refresh concurrency | Explicit unsupported diagnostics; native base prompt retained; no extra user messages; empty clears; stale replies fenced | Plugin-owned source tests plus candidate capability checks |

The isolated Rust crate imports the production composer and supplies only the two-field participant identity DTO seam. It does not test native authentication, persistence, receipt transactions, or provider delivery. A serialization test is not durable recovery evidence. Existing fixtures in `tests/agent_context` remain unchanged and are reused by reference.

## Execution record

Isolated composition execution passed: 3 tests, 0 failed. Command: `CARGO_TARGET_DIR=/tmp/agentx-lifecycle-independent-target cargo test --manifest-path tests/agent_context_lifecycle/Cargo.toml --offline`. This compiled only the separate test workspace and imported composer, not the main product. [Native compact runner](../../tests/agent_context_lifecycle/native_compact.py) is authored but unexecuted: it reuses the existing loopback protocol/launcher fixtures and adds initial developer-channel separation, frozen-file deletion, retained fixed content and post-compact channel assertions.

Main Cargo builds are prohibited for this verifier until final handoff. Candidate target: `/tmp/agentx-context-lifecycle-candidate`; existence is not handoff. Candidate evidence must retain SHA-256, disposable scope and native home, exact session/participant identity, requests/events, assertions and exact owned-process cleanup.

## Findings and pending risks

F01 — source finding, pending owner disposition: [`PrepareInitial`](../../src/agents/context_store.rs) checks only the delivered key before issuing a new receipt. Two preparations for the same participant/session before either confirmation receive different valid receipts; both confirmations pass. This permits duplicate initial delivery under concurrent startup callbacks unless the caller supplies a separate serialization/deduplication guarantee. The storage write transaction serializes each call but does not close the interval between prepare and confirm. Please provide the caller guarantee or enforce one outstanding delivery at the native owning boundary. This is source evidence, not an executed candidate reproduction.

Source review is ongoing while core changes land. Durable read-before-config behavior and alias scoping must also be checked at their native owning boundaries; no conclusion follows from the pure composer.

## Released checkpoint and successor actions

The last completed atomic edit added [store.rs](../../tests/agent_context_lifecycle/store.rs) and registered it in the isolated [Cargo manifest](../../tests/agent_context_lifecycle/Cargo.toml), with `sha2` and `uuid` dependencies. **These new store tests have not been compiled or run.** The existing lockfile predates those two additions and may require regeneration in the isolated workspace. No tests, builds or further source edits were performed after the stop instruction.

The store harness imports production `context_store.rs`, `context_service.rs` and `agent_context.rs`. Its in-memory KV and pre-resolved actor seams intentionally do not prove real SQLite transactions, actor authentication or process/session attachment. Authored cases cover immutable/empty snapshots, exact-session same-name resume, fork/tool/session/generation separation, foreign actor rejection, byte limits, receipt process/session/incarnation fences, legacy receipt independence, sequential deduplication and the F01 outstanding-grant race. The final case asserts that a second prepare cannot grant delivery before the first confirms; it is expected to expose F01 against the currently reviewed implementation, but this has not been executed. The legacy flag keys in the seam are illustrative and do not establish actual legacy storage-key integration.

Prior verification remains: 3 composition tests passed before this checkpoint's store additions; native runner syntax parsed successfully without execution. The complete expanded suite has not run. Files owned here are only `tests/agent_context_lifecycle/` excluding `plugins*`, plus this acceptance file. Existing fixtures and all product files were left unchanged.

Reported upstream build blocker: the core owner reports `cargo check --all-targets` encounters an unrelated missing `InstanceRow.model_request` field in a `src/agents/identity.rs` test initializer. This verifier did not reproduce that report and must not edit that file. Main-project Cargo remains forbidden until explicit final candidate handoff.

Successor remaining work: compile/run the isolated store suite when authorized; disposition F01 with the core owner; verify actual config precedence and read-before-config recovery; implement/run candidate hook cases for missing/foreign/stale sessions, receipt separation and absence of status/mail/user-turn effects; test real resume/fork identities and persistent snapshots after source removal; after explicit final handoff, run manual and automatic native compact against the fixed candidate in disposable scopes/native homes, retaining digest, requests and exact-process cleanup; reconcile plugin-owned results and document each provider's supported or unsupported native delivery channel. Candidate path remains `/tmp/agentx-context-lifecycle-candidate`; file existence alone is not a handoff.

Owner continuation: the user required documentation cleanup and no new external scheduling; provisional workers are archived with released files. Mika resumed their checkpoints. F01 is addressed with one live outstanding grant and explicit release on generation failure; the expanded independent 3 composition + 5 store cases passed. The store seam gained only a process-liveness stub and module alias to match the native module tree. Corrected the authored native assertion: dynamic context SHOULD be a separate developer message, not absent from the developer role; the lifetime distinction is static slot versus refresh message. No production change was made to satisfy the incorrect role assumption. Native execution remains pending final candidate.

Executed candidate checks at the initial lifecycle candidate: manual and automatic real Codex 0.167.0 requests passed with separate dynamic developer messages and unchanged fixed instructions after file deletion; initial dynamic content was absent from the fixed instruction message. Ordinary Codex (6 compact/session cases) and Claude (2 repeated compact cases) passed. The first Codex hook fixture used obsolete six-hook preflight output; the owned fixture now includes actual PreModelRequest/PostModelRequest hooks. Five instruction-source CLI persistence/rejection cases passed. Final candidate, clearing snapshot and final source checks remain pending; see Delivery for artifact identities. Hermes 71 regression tests passed after preserving old-binary bootstrap fallback.
