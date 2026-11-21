---
schema: task
status: completed
description: Implement role and workspace discovery, passive role inspection and explicit retired endpoint reset.
---

Read [team contracts](../AGENTS.md). Own the roles slice in that table. Follow the selected design, including duplicate defaults, invalid selected files, namespace provenance and explicit directory/workspace precedence. Add focused cases without running Cargo until coordinated. Report source handoff and limitations here.

## Working model and contracts

Owner: levi. `src/config/roles.rs` owns pure filesystem selection/resolution; `src/cli/commands/roles.rs` captures the invoking environment and renders inspection without daemon acquisition. The explicit `reset-endpoint` mutation alone connects to the native daemon. `ResolvedRole`, `DefaultRoute`, and the four agreed resolver functions are published as typed skeletons. Namespace identity is the logical origin (`local:` or `global:`) followed by the canonical definition-directory path, preserving distinct origins when scope and local role directories coincide. Default routing returns the exact declaring role plus its declaring workspace.

Cases to author: explicit workspace/dir versus inherited workspace and cwd discovery; empty marker and global context; nearest whole-definition shadowing and explicit namespace selection; invalid selected files/profile/prompt; duplicate defaults and ancestor default ownership; coincident global/local storage; read-only CLI grammar and JSON shadow metadata. Compilation and executable evidence remain with integration/acceptance.

## Source release

The role implementation is released to bime for integration. Changed files: `src/config/roles.rs`, the module declaration in `src/config.rs`, `src/cli/commands/roles.rs`, and this Task. Public symbols match the team contract; CLI also exposes `help_command()` and `help_text()` for controller-owned discovery/routing. No dependencies, daemons, installed executables or live scopes changed.

`select_workspace` uses canonical existing directories and the agreed priority. `AGENTX_WORKSPACE=global` carries explicit global context to descendants; empty/unset values allow cwd discovery. Role lookup supports unqualified, exact `local:` and exact `global:` selectors. One resolution pass caches physical definition reads while preserving separate logical origins. Each selected definition checks its referenced scoped profile and renders exactly one UTF-8 inline/file prompt without interpolation. Effective listings skip shadowed definitions; `--all` also resolves them and therefore reports errors in invalid shadowed definitions. Default routing inspects all candidates in each visited namespace, rejects malformed candidates or multiple defaults there, and returns the exact declaring workspace and role without child name re-resolution.

CLI JSON: list returns `{workspace, roles: [{...ResolvedRole, shadowed}]}`; show returns `{workspace, role: ResolvedRole}`. Text listing exposes role, profile, namespace, default/shadow status and description; show includes canonical definition path and literal prompt. List/show capture the originating invocation directly and never create a Session/client or open managed storage.

Authored 12 filesystem resolver tests and 2 CLI grammar/projection tests. They cover precedence, global and unmarked workspaces, empty markers, invalid marker paths, whole-definition shadowing, explicit namespaces, invalid selected files/profile, literal relative prompts and non-UTF-8 rejection, inherited exact default ownership, malformed/duplicate default candidates, same physical local/global directories, unsafe identifiers, canonical symlink workspace selection, missing defaults, option placement and JSON shadow metadata. Integration can select them with `cargo test roles::tests` after coordinating the shared build; they have not been executed by the implementer.

Static checks completed: file-scoped `rustfmt --edition 2024` and `git diff --check` for owned paths. Cargo compilation, execution, live CLI behavior and remote behavior remain unverified here and belong to integration/acceptance. Controller owns root/router/help/module wiring. No remote support is implied by these local resolver and inspection functions.

## Explicit endpoint recovery follow-up

Controller request #18003 authorizes `agentx roles reset-endpoint [--dir PATH] [--workspace PATH|global] [--json]`. It selects one exact workspace using the same canonical selection rules, then captures a native Client in the originating scope and submits `Participants(Launch(EndpointReset { workspace }))`. No role/default file resolution is required, so an edited or invalid default definition cannot block clearing a definitively retired binding. The selected workspace is not replaced by an inherited default's owner; pass the owning workspace explicitly when resetting a delegated parent endpoint. Native retirement/incarnation checks own admission: live and uncertain recipients remain rejected. `--all` fails before client acquisition, and unexpected native responses fail without a retry or speculative success.

Only the explicit reset branch acquires the daemon. JSON success is `{workspace, endpoint_reset: true}`; text confirms the selected workspace. The root and subcommand help expose the operation and its retirement condition. One additional CLI test covers selector placement, unexpected operands, help and `--all` rejection before acquisition (15 total authored role tests). This follow-up changes only `src/cli/commands/roles.rs` and this Task. File-scoped rustfmt and whitespace checks are complete; no Cargo command or reset mutation was executed by the implementer.

Integration executed all 15 role resolver/CLI checks. Independent discovery, invalid-definition and exact parent-default routing cases passed; native endpoint reset/retirement policy passed its focused unit coverage. See the [integration record](integration.md) and [independent evidence](acceptance.md) for exact executed scope and artifact attribution. This source slice is complete; it does not claim remote role support or real-model execution.

## User-requested file prompt source context

Maintenance owner: bime. User reported that file-prompt relative references lose their origin when injected into an Agent with a different cwd. The existing loader resolves the file itself against the role TOML correctly but discards its location from the injected text. The bounded correction canonicalizes the selected prompt file and prepends `[Role prompt file: ABSOLUTE_PATH]` plus one instruction to resolve relative references from its parent. The original UTF-8 body remains literal; inline prompts are unchanged. The composed text uses existing adopted role history, so no new persistence fields or changes to cwd are needed. The existing literal-file expectations are updated to this requested contract.

Verification completed with the combined [prompt origin/profile report](../evidence/prompt-origin-profile/report.json). The existing discovery case passes with canonical headers and literal UTF-8, and a real native fake-Codex launch from another cwd receives the header/body. Deleting both role and prompt files followed by resume retains the adopted path and body. This establishes supplied text/history, not model interpretation of relative paths. Missing/invalid definitions still use the existing fail-closed loader path; no broad native behavior was changed.

Final combined candidate: `.bin/agentx-role-context`, SHA-256 `ccd0edc74d9fdbd0cb6beb58a879b33d1c71a928088857159d815a1045f07965`, version `agentx 0.7.31`, command-local fast release build passed in 3m51s. Seven grouped output/launch/history checks passed across this correction and the [profile-first guidance](guidance.md#user-selected-roleprofile-launch-guidance). Verified bytes promoted atomically to `.bin/agentx`, backup `.agentx/.tmp/agentx-before-role-context-158e1c50`. [Source/artifact receipt](../evidence/prompt-origin-profile-inputs.json) records no drift in selected files. No live daemon or existing frozen prompt was rewritten.
