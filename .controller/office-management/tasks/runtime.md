---
schema: task
status: completed
description: Compose Role/workspace launch and directory routing/send in isolated management workers using native policy.
---

# Runtime actions

Read team/design and runtime/native roles guidance. Own src/management/worker.rs and a cohesive shared directory-addressing module under src/runtime/ with minimal CLI directory adapter changes if needed. Add role/workspace launch using native resolve_launch_collaboration, Role-derived profile and independent cwd. Reuse native directory arbitration; preserve endpoint exact participant, failed/uncertain state and at-most-once message receipt. Do not parse CLI text or invoke shell; promote/reuse native directory orchestration if needed. Coordinate any runtime/mod.rs registration with integration. Do not edit contract/store/web, frontend, decoder/endpoint inspection. No Cargo/build; hand off source and native-focused cases if justified.

## Design and skeleton

The existing management journal remains the only execution authority: Claim → native runtime effects → Finish. Extract the existing CLI directory recipient orchestration into `src/runtime/directory.rs`; native endpoint lookup/reservation owns adoption and concurrency. CLI keeps workspace selection and presentation in a thin adapter. Shared runtime accepts `Client` and returns the exact `ParticipantRef`, never a name-only identity. Existing placeholder usability and reservation-conflict-only winner reuse remain unchanged. Unknown native launch effects become management Unknown, never permission to rerun.

| Boundary | Intended signatures and responsibility |
| --- | --- |
| Shared runtime | `Address`; `resolve(client, requested_directory, selected_workspace) -> (Address, DefaultRoute)`; `recipient(client, &mut Address, launcher) -> ParticipantRef` |
| CLI adapter | Existing `resolve(Session, dir, workspace)` and `recipient(Session, Address) -> String`; select native CLI workspace, then delegate |
| Worker launch | Resolve collaboration with independent frozen cwd/workspace; reject Role/tool/profile conflicts; Role-derived profile and adopted collaboration pass to native launcher |
| Worker directory send | `directory_send(client, directory, message, request_id)`; check frozen cwd, obtain native entry and resolved route, reject route drift before effects, resolve exact recipient, call shared fenced `send` with operation request ID, return DirectorySent |
| Composition | gabi registers runtime directory module and freezes requested directory into worker invocation before environment-file loading; hima supplies `offices::entry` |

The entry is a bounded execution-time observation, not future send authority. Shared route resolution supplies full frozen Role/prompt for launch; entry safe metadata must match that route before endpoint effects. At submission, admission is filesystem-free; worker validates existence/canonical cwd after Claim. Existing claim/replay/abandon behavior is unchanged. Do not add shell invocation, HTTP dependencies, frontend edits, or file-binding edits.

Verification handoff: integration should cover Role + independent cwd/workspace, inherited/global default cwd, unique adoption, placeholder/error endpoint, concurrent reservation collision, exact-incarnation send, request replay and uncertain/crash outcomes. This owner performs source-only inspection; Cargo/build belongs to integration.

Friction recorded: WorkerInput intentionally omits command until Claim, so operation cwd must be frozen upstream; worker cannot select dotenv cwd from the claimed command after environment loading. Integration has been informed through AgentX.

## Source-ready outcome

Implemented `src/management/worker.rs`, promoted native directory orchestration into `src/runtime/directory.rs`, and reduced `src/cli/commands/send/directory.rs` to selection/presentation. gabi registered the runtime module and confirmed DirectorySend cwd freezing in the upstream submit path. DirectorySend rejects a mismatched canonical operation cwd before effects. Launch uses the adapter-resolved frozen absolute cwd, validated by establish_host; its original directory input may be relative to the configured project and must not be resolved again inside the worker. Role launch adopts native collaboration and Role profile provenance while preserving independent cwd; roleless tool/profile launch remains supported. Directory execution checks the native entry against its frozen shared route before mutation, retains exact incarnation through fenced management Send, and returns an execution binding observation with `created`.

The runtime preserves native placeholder usability, failed endpoint refusal and collision-only winner reuse. Post-launch uncertain failures retain OutcomeUnknown; a definite send refusal after creating a recipient is also Unknown because the operation already had a launch effect. Native lost-reply OutcomeUnknown survives anyhow wrapping. Existing file-task binding additions and contract/store/web/native inspection ownership were preserved. No new endpoint, shell path or journal was introduced.

Source checks completed: standalone rustfmt parsed/formatted the three owned Rust files successfully; `git diff --check` for owned files reported no whitespace errors. Cargo, builds and test execution were deliberately not run. The existing reservation-collision case moved with its implementation; one native-focused error classification case was authored in worker tests to distinguish wrapped lost-reply uncertainty from definite Role conflict. These cases are source-only until integration executes them.

Integration verification remains in the Delivery: Role/workspace/cwd adoption, parent/global endpoint launch cwd versus requested dotenv cwd, exact recipient send, existing failed/placeholder endpoints, collision winner and replay/crash behavior. Source readiness is this Task's result; it does not claim executable or Delivery acceptance.

### Relative launch directory correction

Integration identified that original HTTP launch admission supports relative directories rooted at the configured project. Removed launch's comparison against the raw LaunchInput.directory: the upstream adapter already freezes its resolved absolute path into invocation.cwd, and establish_host validates that host input. DirectorySend retains its absolute-path and canonical-match check. Re-ran standalone rustfmt and the owned-file whitespace check for this affected correction only; no Cargo/build/tests.

Integration executed shared directory collision and wrapped-uncertainty cases with the [37-case native suite](../verification/native-office-r1-receipt.json), passing. End-to-end host effects remain candidate acceptance-owned.
