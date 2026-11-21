---
schema: task
status: completed
agent: f747da8b-9eb8-485a-9ce1-2390457b11ae
description: Define and implement cohesive native environment loading across early CLI configuration and local child composition.
---

# Native environment implementation

## Authority and professional result

The user authorized immediate implementation of [Proposal](../../../.docs/proposals/environment-loading.md) in [Delivery](../delivery.md). Read [team context](../AGENTS.md), [runtime ownership](../../../src/runtime/AGENTS.md), [config ownership](../../../src/config/AGENTS.md), [CLI guide](../../../src/cli/AGENTS.md), [runtime-host model](../../../.docs/architecture/runtime-hosts.md), current main/config/launcher/stdio declarations and the relevant CLI/data-state/engineering guidance. Restore them after compaction. Apply a proportional structure-first workflow: publish a concise top-down model/contracts/skeleton, then behavior; no separate architecture commission.

Own one cohesive originating-environment pipeline. It must let callers use ordered files from repeated CLI options, AGENTX_ENV_FILES or environment.files, configure override through consistent CLI/ENV/TOML including reversal, and obtain the actual environment before dependent configuration/Provider/invocation capture. Implement normal local CLI, workbench and local launch/stdio integrations, preserving help/version and internal serve/runner contracts. Bootstrap external file selection may choose AGENTX_DIR; scoped config file selection may not redirect scope. No recursive selectors. Preserve valid empties, later-file precedence, caller-wins default and explicit override. Select a mature suitable parser if available rather than inventing a shell evaluator; errors must not dump secret source lines. Do not import .envrc as dotenv. Reconcile existing stdio direnv/legacy env order explicitly, and retain loaded values through nested shell cleaning. No daemon-global environment mutation.

Exact native types, module layout, reversible flag grammar/config editing and diagnostic source projection are your professional decisions inside the accepted intent. Prefer a small local model and reuse current parsers/ports. Publish actual declarations/usable skeleton + owned design early, so acceptance can consume contracts before behavior is complete. Compatible decisions do not need controller permission. A substantive conflict should include scenario, competing guarantees and recommendation.

## Owned writes and receiving destinations

Own this original Task; src/runtime environment module(s)/mod/main/bootstrap/launcher/stdio seams; src/config.rs and relevant config/file support; CLI parser/router/launch/config/help seams; minimal typed invocation support if required, with corresponding native source-authored cases. Small existing daemon/observe passive insertion changes are permitted only as required for the declared local environment contract, preserving their scope/identity/read-only guarantees. No DB schema or new service authority.

Own `.docs/architecture/environment.md` (create current design/usage/reference with linked native types), relevant README/src/README/module AGENTS usage routes and source-linked `.docs/architecture/runtime-hosts.md`/`.docs/model/README.md` updates. Those are named receiving responsibilities, not an unowned future docs task. Root AGENTS, roadmap, Proposal, Delivery/controller/catalog are controller-owned. Acceptance owns tests/environment_loading and evidence.md, Cargo.lock and all executable resources. If a minimal dependency is necessary, select an exact justified Cargo.toml region and release it directly for acceptance's lock resolution; do not edit Cargo.lock yourself. Preserve all unrelated dirty work.

No compiler, formatter, Cargo, Node, product, browser or test execution. Author meaningful cases including interacting precedence/empty/scope/security/direnv/nested behavior; hand exact filters to acceptance. No spawning or nested delegation, live `.agentx`, `.bin`, installed binaries or running-service operations.

## Handoff and closure

Use peer bindings in controller.md for direct actionable contract/releases/defects. Native chooses/refines safe diagnostic visibility; do not publish raw environment values. Release coherent source+cases+docs and exact changed input list/hash manifest to acceptance. Mark completed for source handoff and release all writes; independent outcome support remains acceptance-owned. Concrete execution defects return here, not to new microtasks. Own any necessary model/support receiving update after actual evidence without rerunning checks.

## Coherent source release — 2026-11-10

Source implementation and authored cases are complete for independent receiving. The canonical [native contract](../../../.docs/architecture/environment.md) links actual types, CLI/config editing, ordering, parser behavior and failure/diagnostic policy. The exact [release manifest](../native-release.json) identifies every touched source/case/document input by SHA-256; whole-file identities include pre-existing accepted dirty work and do not claim ownership of those unrelated regions. The manifest excludes its own bytes to avoid a circular hash. Native releases all listed source writes to the sole independent executable owner; actual support remains [acceptance-owned](acceptance.md), with no successful execution inferred here.

### Native model and composition

| Boundary | Released structure |
| --- | --- |
| Originating pipeline | `runtime/environment.rs` owns pure `resolve`, CLI extraction, source/provenance receipts, main-only early `prepare`, metadata/internal-entry bypass and selected-key restoration. `runtime/environment/tests.rs` owns finite authored cases. |
| Bootstrap order | Authorized stdio direnv once → frozen CLI/caller/control-scope selectors → dotenv values → final scope/ordinary config/Provider pin → explicit invocation capture. Only supported scoped commands interpret `--scope`; a Provider's native argument cannot select local sources. |
| Config | `config::EnvironmentSettings` keeps startup selectors outside ordinary export; `config/file.rs` reuses locked/private edits for JSON string arrays and booleans. `config environment [--json]` is passive/local and publishes metadata without values. |
| Child composition | Invocation/host contexts retain selected variable names and literal values separately from identity. Launcher restores selected values after clean-shell reconstruction, then applies existing config/legacy/explicit overlays and identity scrubbing. Per-child reserve requests retain the same names. Internal bounded name-only retention metadata survives fresh nested hosts. |
| Workbench | Capture prepared local configuration once; keep fixed scope/project, clear participant/session/capability claims, and clone that operator invocation for HTTP requests. No browser environment selector or daemon-global mutation. |
| Usage/model | README, source/module entry guides, runtime-host model and domain-model navigation route to the one detailed native contract. |

### Resolved semantic feedback

The early skeleton left a bootstrap circularity between external `AGENTX_DIR` and TOML override selection. Independent consumption identified it before execution. Final rule: freeze list and boolean independently from explicit supported scope, otherwise caller `AGENTX_DIR`/HOME after direnv. CLI > ENV > that scope TOML; later file-selected scope does not reread selectors. Thus A's TOML override can govern an external file choosing B; without override an inherited A wins, while an absent caller key can be bootstrapped. Scoped TOML-selected files declaring `AGENTX_DIR` fail before acquisition even when shadowed. Explicit command scope is final and external scope data is withheld without rewriting the literal caller snapshot.

One empty CLI selector or an empty ENV selector disables lower lists; mixed CLI empty/nonempty is invalid. Later files/assignments win among file data; caller values, including empty, win unless override is true. Dotenv selector/retention declarations are withheld and cannot recurse. Paths use selected local project/cwd, not daemon cwd or a remote target path. `.envrc` is never dotenv, including a selected path with that name before symlink normalization. Missing/unreadable/invalid input errors omit raw source/parser content and values.

Native read-only feedback also found the initial independent legacy fixture used `AGENTX_*`, which the existing legacy parser filters. Acceptance corrected its fixture to ordinary variables before execution. Legacy scope `env` stays nonempty/non-`AGENTX_*`, child-only and late. No permission or protocol change was required.

### Authored cases and execution boundary

The manifest lists the exact 18 new native resolver cases, 2 config editing/selection cases, the strengthened workbench operator/configuration case and affected launch/JSON/shell regression filters. Cases cover interacting precedence/empties, independent list/boolean shadowing, control/final scope, unsupported native scope and remote paths, selector non-recursion, safe errors, no shell execution, metadata bypass, distinct snapshots and nested restoration. Independent outcome fixtures own actual workflow/foreground/stdio/background/direnv/concurrent-host observations.

Only dependency change owned here: `dotenvy = "=0.15.7"` in Cargo.toml. Actual upstream parser/iterator source was read to select its data-only read iterator and document substitution; no shell evaluator or homegrown dotenv grammar was introduced. Cargo.lock is untouched by Native and its pre-resolution identity is recorded for acceptance's narrow lock update.

Native performed source/document reads, edits and hashing only. No compiler/Cargo/formatter/Node/test/product/browser/package or protected runtime operations ran. Existing dirty work, live `.agentx`, workbench18947, demo18731, project `.bin/agentx` and installed/default programs remain protected. Concrete independent defects return to this original Task with a narrow source amendment; current-model support receiving is Native-owned after evidence.

### Independent input receiving — 2026-11-10

Acceptance matched all 30 original release hashes and the lock-before hash, then returned its [six-file formatting delta](/tmp/agentx-environment-loading-acceptance-20261110/r1-format-delta.json) and [minimal lock delta](/tmp/agentx-environment-loading-acceptance-20261110/r1-lock-delta.json). The latter adds only dotenvy 0.15.7 plus AgentX's direct dependency, retaining 347 other package identities; resulting Cargo.lock SHA-256 is `5c0c4ec8a74326dd8dc4a95b5312b40d28e523e828eedb48ae3607e2d485658d`. Acceptance owns the retained offline cache-miss attempt and subsequent lock resolution evidence. Native consumed these identities read-only and made no product-source or lock edits.

Acceptance reports a race-free immutable `r1-fixed-source` snapshot of 552 inputs and compilation in progress. This is an input/ownership receipt, not executed support or a passing compile claim. The original native-release manifest remains the historical pre-format release; acceptance's deltas identify the derived input. This Task-only receiving note is outside that historical hash set. No correction is requested and Native keeps product source frozen.

### ENV09 causal correction — native design

Acceptance's fixed artifact `d4f71209a3b44f3454e02b7b536cb798c1bd89d34f5ce1f6c5c201caac2c1147` passes 52 scoped cases and packages successfully. Its first outcome attempt passed ENV01–08/INS01 but [ENV09 root help](/tmp/agentx-environment-loading-acceptance-20261110/product-r1/ENV09-root-help.json) created the absent selected scope despite successful help output and dotenv bypass. The automatic update notice still reached `update::get_update_info` → `spawn_background_check` → `claim_check`, which creates the flag directory and may spawn a worker. This violates the existing passive metadata requirement.

Correction structure: the router owns one lazy automatic-notice boundary, before invoking the update effect. Reuse the originating lightweight classification for root/topic/command/launch help and native stdio version, and the existing local metadata classification for environment configuration diagnostics. Ordinary commands/launches retain their notice; explicit update retains its own output. No update subsystem/cache policy or executable resource changes. Author regressions through the actual lazy boundary, verifying zero cache/worker callback effects across metadata scenarios and one retained callback for ordinary commands. Acceptance owns the real absent-scope ENV09 rerun and any affected packaging.

The narrow [ENV09 correction manifest](../native-correction-env09.json) releases `src/cli/router.rs`, the CLI guide, detailed contract and this Task. `command_update_notice` is the actual lazy dispatch gate; two authored router regressions cover metadata side-effect exclusion and ordinary/explicit-update behavior. Static source inspection only; no native execution or Cargo.lock changes. All correction writes are released to acceptance. Its passed native/source outcomes and prior artifact stay historical evidence; only this affected boundary requires new source execution and artifact verification.

### Completed independent support receiving — 2026-11-10

Acceptance released final execution facts for Native's owned documentation. The corrected candidate compiled, passed the two new router cases, and packaged/version-checked as `agentx 0.7.31`; the original 52 scoped passes were retained without repetition. [Canonical independent evidence](../evidence.md) owns exact artifact identity, immutable inputs, receipts, retained failures and resource cleanup. Native read the final outcome/audit receipts without executing any toolchain or product command and updated the [current support boundary](../../../.docs/architecture/environment.md#independently-exercised-support), root usage README, source entry, runtime-host model and domain-model navigation.

Actual retained results cover ENV01–08/INS01 selection/override/empty/scope/secrecy/provenance, finite foreground/background/nested and marked clean-shell composition, exact 16,384-byte stdio/exit37, same unchanged daemon with concurrent distinct invocation environments, a second scope, legacy overlays and real authorized direnv default/override. The corrected package passed all 13 metadata shapes with absent scopes/poisoned selectors and a strict no-selector baseline. A real finite workbench used a project-relative file to bootstrap scope/project and answered HTTP200 after source deletion. Workbench evidence is HTTP/startup only; no browser rendering or new Provider-native conversation support follows.

The final independent audit passed: 552 frozen inputs unchanged, zero shared product-source differences, no recorded fixture incarnation/owned image still executing, disposable roots/listener/endpoints cleaned, four protected runtime identities and prior repository image preserved. These are acceptance facts linked through its evidence, not operations performed by Native. Linux local finite/HTTP boundaries remain explicit; macOS/Windows, all Provider/terminal/window combinations, remote source distribution, paid sessions and browser rendering remain outside this acceptance. Controller alone owns repository artifact promotion and any project live cutover.

Native source, causal correction, authored cases and named model/usage receiving are complete. No product source, Cargo.lock, binary, live scope or service was changed during this final documentation receiving; no test/package rerun is requested.
