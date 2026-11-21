---
schema: task
status: completed
description: Role-owned auxiliary role discovery without access restrictions
---

# Auxiliary role discovery

User authorized implementation on 2026-11-16 after selecting discovery isolation only. Reuse [current role contracts](../../.docs/roles-workspaces.md), native adopted collaboration snapshots and existing passive IPC. No permission enforcement, instruction inheritance, DB migration or service replacement.

## Selected model and boundaries

A role file named `OWNER.HELPER.toml` is an auxiliary role. Exactly two nonempty name segments use the existing letters/digits/underscore/hyphen grammar. Normal names remain public. Auxiliary roles own complete independent prompts/profiles and cannot be directory-default endpoints. Explicit show/launch resolves all names for any caller. `roles --all` includes auxiliary and shadowed definitions in the existing directory search range.

Default discovery includes public roles and helpers whose owner name and canonical role-directory namespace match the caller's adopted Role snapshot. A helper does not automatically see its owner's other helpers. Persisted membership supplies the default workspace; explicit dir/workspace still wins. No identity means public discovery. CLI owner lookup is passive against an existing daemon; unresolvable implicit identity warns and falls back to public discovery. Explicit caller errors remain errors. Help/show/all remain filesystem-only. Browser office role lists remain public through the existing list function.

## Implementation structure

`config::roles` owns two-level validation and `list_roles_for(..., owner: Option<&ResolvedRole>)`; existing `list_roles` delegates with no owner. CLI roles owns passive native caller resolution and presentation. Existing launch resolver accepts two-level selectors unchanged. Bootstrap/help and usage expose discovery semantics.

## Cases and evidence

Cover public vs adopted-owner vs other-owner discovery; same-name owners in different namespaces; helper callers; --all; explicit resolution and launch dry-run; invalid names; defaults; hidden invalid helper isolation; passive no-daemon human CLI and claimed identity CLI. Run bounded Rust tests and isolated CLI checks. No live service or installed executable adoption is implied.

## Working record

CBM index discovery failed with daemon connection timeout; use targeted current source evidence. Existing shared workspace changes are preserved. Implementation and verification completed; see the final local outcome below.

Source review identified a discovery/launch mismatch if owner filtering precedes name shadowing. Preserve existing shadowing first, then filter; hidden nearer helpers must never make a farther short name appear launchable as the wrong definition. Explicit global/local lookup remains the escape hatch. Added namespace/shadowing coverage.

## Verification progress

Final role/shadowing source passes `just test --bin agentx roles -- --nocapture`: 21 passed, including native management public-view regressions. Startup/subagent capability and exclusion checks also passed (2 cases); the final shadowing-only change does not affect those results. The initial full-optimization package attempt was stopped after the source correction to avoid finishing stale bytes. Final packaging uses the existing `just package` recipe with command-local `CARGO_PROFILE_RELEASE_LTO=false`, `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`, `CARGO_PROFILE_RELEASE_OPT_LEVEL=1`; no persistent Cargo profile edits. Isolated CLI/runtime checks are implemented in `tests/auxiliary_roles.py`, reusing the existing disposable fake-Codex fixture.

## Completed local outcome

Implemented two-level role names, adopted-owner discovery with source-namespace matching, normal shadowing before visibility filtering, unrestricted explicit lookup/launch, passive identity lookup, helper annotations, and shipped startup/help/usage guidance. Public management role discovery remains on the public view. No dependency, database schema, launch lifecycle or permission system was added.

Final isolated command: `python3 tests/auxiliary_roles.py --binary .artifacts/auxiliary-roles/agentx --output .controller/roles-workspaces/evidence/auxiliary-roles/report.json`. All 15 grouped checks passed, including actual fake-Codex native launches of the public parent, its helper and a global namesake; process-bound adopted-owner discovery; explicit caller discovery; persisted workspace versus changed cwd/environment; source namespace and shadowing; explicit global helper launch preview; no-daemon passive queries; shipped help and startup guide. Each fixture process and its disposable data was cleaned up. This is implementer-executed local Linux verification using a fake Provider, not independent acceptance or evidence of real-model behavior, other platforms, or fresh shared app-server role launches.

[Command/effect report](evidence/auxiliary-roles/report.json) and [artifact/source receipt](evidence/auxiliary-roles/artifact.json) retain exact evidence. Release packaging passed in 3m42s with command-local fast settings; version smoke passed (`agentx 0.7.31`). The usable dedicated binary is `.artifacts/auxiliary-roles/agentx`. Existing `.bin/agentx`, system-installed executables, live daemons and frozen Agent instructions were preserved; use the dedicated executable explicitly to exercise this implementation.

## User-authorized executable adoption

The user subsequently requested replacing project `agentx` and installing the same artifact in `~/.local/bin`. Atomically installed the exact verified artifact at `.bin/agentx` and `~/.local/bin/agentx`; both digest, version and auxiliary-role help checks passed. Original bytes are preserved under `.agentx/.tmp/auxiliary-roles-install/`. [Installation receipt](evidence/auxiliary-roles/installation.json) owns exact destinations, hashes and backups. No live daemon or existing Agent was restarted or modified.
