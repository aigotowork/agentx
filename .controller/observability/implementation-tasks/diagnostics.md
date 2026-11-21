---
schema: task
status: completed
agent: CLI diagnostic consumer implementation owner
---

# Passive CLI investigation path

## Authority and entry

Read the root `AGENTS.md`, the [observability team entry](../AGENTS.md), [implementation Delivery](../implementation.md), [feature entry](../../../.docs/observability/README.md), [engineering map](../../../.docs/observability/engineering.md) and the specific contracts below. The user has authorized the implementation Delivery. Follow the [current implementation team contract](../implementation-team.md) and [actual assignments](../controller.md#implementation-assignments); this Task becomes active only under its assigned owner. Preserve the completed daemon baseline; execution uses explicitly disposable scopes and does not include installed/live cutover.

## Inputs and result contract

Actual composition release, [consumer boundary](../../../.docs/observability/consumer-boundary.md), [scenarios](../../../.docs/observability/scenarios.md), [runtime](../../../.docs/observability/runtime.md), [configuration](../../../.docs/observability/configuration.md) and [events](../../../.docs/observability/events.md). Apply the command-line design guide reachable from the root guidance.

Deliver one complete non-Web diagnosis path that starts with originating failure evidence, selects one scope, reads daemon/request/configuration/event facts and ends with a supported explanation or explicit unknown. It must work without normal Session acquisition, identity resolution, cleanup, mailbox confirmation or automatic daemon start.

## Write and resource boundary

Own `src/cli/commands/observe.rs` with cohesive private leaves as needed, additive pre-setup dispatch in `src/main.rs`, and only the named additions to `cli/{router,commands/mod,commands/help}.rs` plus CLI usage/context. Native query/clock/disclosure/retention rules remain with their owners. Composition's [explicit early region release](composition.md#early-passive-api-and-cli-region-release) transfers these additions to this Task; release them to the local-adapter owner afterward. Preserve existing declarations and unrelated executable behavior; no whole-file rewrite is authorized.

## Required work and decisions

Implement the [selected command tree/input-output contract](../../../.docs/observability/consumer-boundary.md#planned-cli-surface-and-output), completing any domain-specific option only after consuming its native type. Include explicit scope selection, text/JSON output, detail/compare and event continuation, size/limit validation, stdout/stderr, exit semantics and interruption. Do not invent a parallel opaque ref or cursor namespace. Print local attempted facts even if daemon evidence is unavailable; distinguish cached/historical facts from current reads. Verify ref-file field selectors and page/window inputs against their exact owner operations before claiming ready CLI support.

## Authored cases and acceptance

Author CLI cases for OBS-01..10 consumer branches, JSON precision/ref round-trip, empty versus gap/loss, unsupported/stale/withheld/unknown results, explicit scope aliases and missing scope without filesystem changes. Verify closed stdout/SIGINT stops observation only and preserves product executions. Exit success means a successful read/investigation, never a successful business mutation.

The [Delivery](../implementation.md) owns formal stage dependencies. Native/domain owners author cases; the assigned independent integration owner executes the necessary checks. Keep source analysis, authored fixtures, executed evidence and installed support distinct. Do not rerun a passed check without a changed relevant input or explicit requirement.

## Handoff and completion

Release command contract, actual parser/adapter implementation and authored cases to native integration. Document the local recovery read for uncertain domain operations without automatically invoking mutations. Browser/HTTP is excluded.

Maintain this original Task's status and source/evidence basis. Replace promoted design declarations with native links, update the owning guide, identify remaining limitations, and release exact writes/processes/endpoints/roots before completion. No installed/live release is implied.

## Callable entry and implementation assignment

Composition's [early passive API and CLI region release](composition.md#early-passive-api-and-cli-region-release) supplies an actual stable source contract before its complete source release. This enables independent CLI implementation alongside the remaining composition cases and projection/guide work; it does not complete composition or authorize executable checks on a moving candidate. A dedicated CLI owner is proportional because parsing, output, local file operands and interruption form a separate consumer responsibility that can proceed without racing daemon wiring. No nested delegation is authorized.

Consume [`daemon::observation`](../../../src/daemon/observation.rs) directly: `resolve_existing_scope(ExistingScopeInput) -> ScopeLookup`, `inspect_existing(&ScopeRef, Option<DaemonRef>, InspectionRequest) -> Result<InspectionResult, LocalFailure>` and `originating_clock()`. The aggregate embeds the native Runtime/Configuration/Events requests and results unchanged; neutral `DaemonChanged` precedes leaf results. `InspectionResult` preserves ready, response and received_at. `LocalFailure.at` is optional; no missing time may be replaced by a sentinel, inferred timestamp or new clock origin.

Available originating helpers are `service::observation::{AttemptGuard,stage,drain_local_reports}` and `config::observation::NativeConfigResolutionSource`; inspect their native signatures before consumption. Dispatch `observe` before ordinary acquisition, dev-root/identity setup, reexec and other business command behavior. Do not acquire a Session, start a daemon, read live managed storage, publish reports automatically or change ordinary configuration precedence. Native implementation references replace corresponding planned declaration copies when this consumer is promoted.

First restore root/team/Task context, the selected CLI contract and actual source signatures, then record the local module/parser/output structure and materialize it before filling behavior. Author meaningful local parser/output and consumer cases, marking them unexecuted. Coordinate exact grammar/output with [native integration](native-integration.md) so it can adapt its five existing artifact case families without introducing a substitute command. The integration owner retains all source-check/formatter/Cargo/Node/test/package/probe resources; this owner performs source implementation and reading only.

The controller records the actual Paseo binding after successful creation, then transfers this Task's future status/current-work/handoff writes to that owner. Preserve the accepted baseline, installed programs, default scopes and all sibling writes. Full composition plus diagnostics releases still precede the independently frozen backend candidate; the adapter and Web entry guards are unchanged.

## Actual binding and current entry

The assigned binding is `{serverId: srv_yDbm9zuqIMiM, agentId: 209a7d4f-b655-43d4-9adb-dd858ae8f150}`, workspace `wks_35140a9c49235329`. Current profile discovery selected `default`, materialized as `codex/gpt-6.1-sol`, `full-access`, `max`. Creation returned this checkout/workspace, running status and full-access mode. The controller has activated this Task and transferred its subsequent current-work/status/handoff writes to this owner.

Entry is the actual early source/region release linked above, not complete composition or executable evidence. First consume those signatures and the selected CLI contract, record the bounded structure, then implement the native consumer and author cases. Maintain concrete blockers here and send only actionable dependency releases/diagnostics through the paired Paseo bindings. No compiler, formatter, fixture process, build/package resource or runtime evidence is acquired by activation.

## Consumer structure and decisions

The command is a native read adapter. Its dependency direction is CLI grammar/local operands → native passive aggregate → unchanged Runtime/Configuration/Events leaves. No business Session, ordinary configuration initialization, identity, update check, dev-root read/reexec, managed storage or daemon acquisition participates. The direct main route is before all normal setup; router repeats that guard for callers entering it directly.

| Home | Owned contract |
| --- | --- |
| `src/cli/commands/observe.rs` | Family entry, clap help/errors, captured invocation, output/signal lifetime and final exit. |
| `observe/args.rs` | Canonical clap tree; pure prepared command; explicit bounded UTF-8 native ref/window files, unknown-key and native field-selection validation before scope lookup. |
| `observe/client.rs` | Existing-scope resolution, source-clock local attempts, finite native reads, explicit current reevaluation and same-stream event continuation. |
| `observe/output.rs` | Typed consumer envelope embedding native values, exact serde numbers/refs, readable source/era/time/unknown meaning, stdout framing and exit classification. |
| `observe/tests.rs` | Authored parser/input/output/continuation cases; integration alone executes them and its existing five CLI artifact families. |

`--scope` and `--json` are family-global options, accepted before or after the selected leaf. No new ENV/config keys are introduced; omitted scope uses the existing originating environment/cwd path resolver. Explicit ref/window files are at most 32 KiB and must be regular UTF-8 JSON files. Native serde parsing plus recursive key validation rejects ignored unknown fields without copying domain types. Configuration field selectors use native closed fields and native selection validation. Invalid input exits 2 with empty stdout before scope or transport calls.

`ObserveOutput` retains command, original requested selection, native scope lookup, safe native input capture/local attempt and the native inspection result or local failure. Never serialize InvocationContext/environment or replace a missing clock. Successful one-shot diagnosis (including an explicit missing/gap/incomparable result) exits 0; local/transport/native rejection exits 1. Event follow uses original immutable windows until exhausted, then opens the next same-stream interval on the user's explicit `--follow` decision. Gap/epoch/stream changes are emitted before stopping; no automatic rebase/retry of product effects occurs. SIGINT returns 130 and closed stdout stops reads immediately. Only observation-local handles/timers are owned.

The independent integration owner approved a bounded transfer of consumer-boundary.md's `Current source basis:` paragraph and existing `Planned CLI surface and output` section, preserving its inbound fragment. Remaining browser/security/lifetime/gate regions stay integration-owned. The existing five `tests/observability_cli.py` families are consumed, not replaced. All source/case/document work below remains uncompiled and unexecuted until independent fixed-candidate integration.

## Concrete contract feedback

Source inspection found that ordinary `InvocationContext::capture` flattens cwd I/O before the CLI can retain its exact failure, while the optional report can disappear when its required clock is unavailable. This was sent directly to composition rather than reconstructed in the CLI. Composition released native `InvocationContext::capture_observed() -> Result<Self, LocalFailure>` sharing its one-pass private input assembly; the CLI now consumes that method and preserves the returned cause/code/optional time unchanged. No CLI copy of invocation capture policy was introduced.

Event nonce, era and numeric-window checks remain with the native journal, including its era-before-frontier ordering. The CLI validates JSON/native shapes, file size, closed fields/filter and page-limit syntax, and rejects operand scope mismatch before transport. Comparing different operand eras uses aggregate `expected=None`, so the configuration owner's actual `Incomparable` result is not preempted by pinning the aggregate to one operand. These choices preserve the offered native operations rather than silently strengthening them.

## Final callable source release

The complete native CLI consumer, authored cases, usage and context are released for the independently frozen candidate. Main calls `router::dispatch_passive_observation` immediately after argv capture, before daemon/default/project/configuration setup; router applies the same guard for direct callers. `observe` and `help observe [leaf]` bypass ordinary identity/dev-root/reexec/update/Session/acquisition. Existing ordinary behavior and all sibling writes are preserved. Composition separately released its final stable native producer/lifecycle/adapter graph; this Task consumes its actual `capture_observed`, passive scope/inspection and native leaf APIs.

The exact canonical tree is `observe [inspect]`, `observe request --ref-file FILE`, `observe config --ref-file FILE`, `observe config-current`, `observe compare --left-file FILE --right-file FILE [--field NAME]...`, and `observe events [--window-file FILE] [--limit N] [--follow]`. `--scope PATH` and `--json` are inherited before/after every leaf. [Parser declarations](../../../src/cli/commands/observe/args.rs) own exact spelling/closed selectors and [usage](../../../.docs/observability/cli.md) supplies complete input/output/recovery examples. Explicit files are regular bounded 32KiB UTF-8 JSON native values; Unix nonblocking open prevents a raced FIFO from blocking before regular-file revalidation. Validation never prints arbitrary JSON payloads or native arbitrary error text.

[`ObserveOutput` and `ObserveResult`](../../../src/cli/commands/observe/output.rs) own only CLI framing: `{command,requested_scope,scope,local:{inputs,attempt},result}`. Native external tags appear as `result.Inspection.{ready,response,received_at}`, `result.LocalFailure.{stage,kind,os_code,at}` or `result.ConfigurationCurrent.{Ok,Err}`. Native result enums continue inside response unchanged. `scope.Existing.canonical_scope` is the selected canonical root; `requested_scope` remains the lexical requested selection. `local.inputs` is optional native safe input-capture Result; `local.attempt` is optional Probe/DiscoveryOnly with no product request ID. The raw invocation/environment is never serialized. Required report times are omitted rather than fabricated; independently known failure time may be null. A scope failure is retained at its actual first observed lookup rather than recaptured on later presentation.

One-shot typed diagnoses (including missing configuration/incomparable/event gap or changed event epoch/stream) exit 0; local/native errors and aggregate DaemonChanged exit 1; parser/file/selection rejection exits 2 with empty stdout. Follow emits a native gap/era boundary then exits 1, keeping the original fixed windows and no automatic rebasing. JSON is complete-value/newline framing, text retains safe facts/identity/time/coverage meaning, and stderr stays separate. SIGINT ends only observation at 130; detected BrokenPipe ends observation at 0. The [actual native client](../../../src/cli/commands/observe/client.rs) performs no report publication, business dispatch, storage fallback or effect replay. Current configuration reevaluation uses the selected existing canonical scope and native owner source explicitly; retained reads do not reread current settings.

## Authored filters and evidence limits

All 21 tests in [`observe/tests.rs`](../../../src/cli/commands/observe/tests.rs) are authored and **unexecuted**. The exact common Rust filter is `cli::commands::observe::tests::`; the owner-local test names are:

```text
family_globals_work_before_and_after_every_leaf
family_rejects_business_identity_flags_and_invalid_limits
generated_help_has_family_options_at_every_leaf_without_input_reads
native_operands_reject_utf8_unknown_nested_duplicate_and_oversize
explicit_file_operands_validate_before_scope_or_transport_work
native_field_selection_rejects_arbitrary_keys_and_duplicate_fields
obs01_known_failure_without_time_remains_structured_and_nonzero
obs02_obs03_request_and_reply_facts_never_become_effect_confirmation
json_round_trip_keeps_exact_u64_request_ref_and_one_value_per_line
obs04_withheld_comparison_is_not_equal_or_current_reevaluation
obs04_current_capture_keeps_diagnostic_purpose_and_safe_native_projection
obs04_explicit_current_resolution_uses_selected_scope_without_daemon_or_database
obs05_empty_page_retains_collection_loss_and_differs_from_gap
obs05_continuation_keeps_filter_frontier_and_does_not_rebase_loss
obs06_operands_never_select_another_scope_implicitly
obs01_obs06_passive_scope_alias_and_missing_selection_have_no_setup_effects
obs07_pid_reuse_and_unknown_incarnation_stay_distinct_native_rows
obs08_obs09_follow_does_not_rebind_restart_reset_or_replaced_stream
obs08_obs09_native_era_outcomes_remain_visible
obs10_unavailable_unsupported_evicted_and_busy_are_not_empty_success
interruption_and_closed_stdout_stop_only_the_consumer_path
```

The existing independent [`tests/observability_cli.py`](../../../tests/observability_cli.py) retains its five families `missing-scope`, `existing-scope-without-daemon`, `scope-is-file`, `relative-alias`, `malformed-reference-files`. Integration has already consumed the actual wrapper/grammar and adjusted these same authored families; no replacement runner or mock success was introduced. The additional [actual process scenarios CLI-01..07](../../../src/cli/commands/observe/cases.md) specify the real diagnostic path under held/saturated product work, retained configuration versus explicit reevaluation, continuation/era loss and actual SIGINT/BrokenPipe isolation. These authored scenarios remain required evidence for corresponding process/platform claims; the small owner-local writer/timer tests do not prove process isolation.

This owner performed source reading/implementation and context/case authorship only. **No formatter, Cargo, Node, compiler, test, source-check script, build, package, probe, installed AgentX command or live/default operation was executed.** The source is unformatted/uncompiled/unexecuted by this owner. Independent native integration alone freezes actual inputs and records executable results/cleanup. Source completion here does not complete the diagnosis/backend Delivery stages, browser prerequisites, installed cutover, native-platform support or whole product acceptance. No active implementation blocker or unresolved native API request remains.

## Completion and receiving owner

The complete write set is `src/cli/commands/observe.rs`, its private `observe/{args,client,output,tests}.rs`, `observe/{AGENTS,cases}.md`, additive observation regions in `src/main.rs` and `src/cli/{router,commands/mod,commands/help}.rs`, `src/cli/AGENTS.md`, `.docs/observability/cli.md`, and only the two approved regions of `.docs/observability/consumer-boundary.md` (`Current source basis:` and `Planned CLI surface and output`). All are released; no background process, test resource, endpoint, scope root, build resource or source lock is retained. The original Task is completed for its assigned callable source/case/context release, and remains the handoff authority.

The two consumer-boundary regions return to native integration. The additive main/CLI route is released for the later local-adapter owner only under the controller's existing backend-acceptance gate; no adapter route is implemented by implication. Integration may apply its required scoped formatting to this exact source. If its fixed-candidate checks find a substantive defect, send the exact diagnostic to this assigned owner and transfer only the necessary source back for repair. Controller owns stage/catalog/root navigation updates; this owner does not edit those shared areas. Paired receiving bindings are controller `{srv_yDbm9zuqIMiM,7c3af077-454b-4d39-9c3d-71de3fbc9598}`, composition `{srv_yDbm9zuqIMiM,b56613b3-d267-4b49-996b-f80a3fbec937}` and independent native integration `{srv_yDbm9zuqIMiM,c85e6101-0cb7-46a8-824a-2433173743a0}`.

## Candidate03 fixture correction

Native integration reported its first exact117 execution on strict-Clippy-passing frozen candidate03: 110 passed and seven failed. Its preserved receipt is `/tmp/agentx-obs-native-8hyqm9g2/observation-native-tests-03.log`; this is integration-reported execution, not an owner-run result. The only reported owned CLI failure is `cli::commands::observe::tests::obs04_explicit_current_resolution_uses_selected_scope_without_daemon_or_database`, where the fixture's top-level `timeout = 37` was ignored and the native default 86400 was observed.

This Task is temporarily active under the bounded return of `src/cli/commands/observe/tests.rs` and this original Task only. Source inspection confirms the canonical `FIELDS` entry in `src/config.rs` maps timeout to `preferences.timeout`. Correct the fixture to the actual `[preferences]` TOML table while preserving the expected 37-second value, safe scope/DiagnosticReevaluation assertions and no-database assertion. No resolver/product change, frozen03 mutation, owner formatting or execution is authorized. Native integration retains the 110 successes and will rerun only the affected failed case after release.

Correction released: the fixture now writes `[preferences]\ntimeout = 37\n`. The assertion and all other test behavior are unchanged. Only that fixture literal and this Task were edited; no formatter/test/build/check/probe was run and frozen03 remains untouched. The bounded writes are released back to native integration and this Task returns to completed for source/case release. The exact sole rerun filter is `cli::commands::observe::tests::obs04_explicit_current_resolution_uses_selected_scope_without_daemon_or_database`; its corrected outcome is pending integration execution, and the other six reported native failures remain with their respective owners.

