---
schema: task
status: completed
owner: vore
description: Discover published Markdown roles, customize through scoped TOML, and teach the actual CLI and management consumers.
---

# Outcome and design

Implement the user-approved [compatibility model](../../.sketches/convention-roles.md). Keep unresolved RoleDefinition separate from frozen ResolvedRole; config owns Markdown/TOML discovery and namespace precedence, runtime launch_profile owns one effective role/profile composition consumed by CLI and management. Stored collaboration and endpoint identities retain existing contracts. Use existing serde_yaml_ng and native profile selection; no dependency or registry is needed.

## Structure and cases

The configuration entry keeps selection/catalog contracts; a private reader owns literal Markdown metadata and TOML overrides. A logical workspace groups both directories. TOML claims a same-workspace name; disabled entries suppress fallback; an explicit prompt replaces the Markdown base. Only default-route selection or launch resolves a named profile. Catalogs can contain null profile; adopted views retain a concrete profile.

Cover Markdown-only discovery/workspace, helpers across source formats, explicit/thin/replacing overrides, metadata errors, disabled fallback, nearest scope precedence, no-default discovery and launch errors, explicit profile precedence and frozen snapshots. Reuse existing role/profile/default/HTTP cases and regenerate browser contracts. Build a project executable without restarting running services.

## Progress and evidence

Design accepted by the user. Implementation and bounded local integration completed. Graph service has no index for this checkout; targeted current source is the evidence basis. Existing dirty/untracked work is preserved.

## Compatibility refinements

Default-recipient resolution accepts the caller's AGENTX_PROFILE when its role omits profile, consistently in CLI directory routes and management previews; an explicit role profile still wins. A project-local AGENTX_DIR that aliases the local TOML directory inherits the same physical Markdown prompt when selected globally, without adding Markdown-only roles to the global catalog. [Recorded friction](../../.inbox/convention-roles.feedback.md) explains the profile/snapshot separation and old case updates.

## Initial verification

The final behavior source passed native role/CLI/management selection tests (26), launch-profile cases (4), management office cases (9), and bootstrap cases (27); the role filter includes two office cases, so these are 64 distinct tests. Fresh browser typechecks/assets are required by the native build. An additional help grammar correction teaches role and profile as combinable options; its final bootstrap/help check and package are in progress. Executable CLI/HTTP fixtures run after that immutable package is ready.

The wider CLI help suite has 14 passing and one unrelated failure: `config_help_covers_registry_keys` reports existing `instruction_sources` missing from config help/exception registry. See [feedback](../../.inbox/convention-roles.feedback.md). Targeted role/profile grammar and shipped help are checked separately.

## Adoption plan

The immutable candidate is `.bin/agentx-convention-roles`, built with the existing `just package` recipe. After isolated CLI/HTTP verification, install that exact file into `.bin/agentx`. Preserve `.agentx/config.toml`'s existing `codex-gpt-6-1-sol-high` default, remove six redundant TOML wrappers, and reduce `controller.toml` to its default-recipient declaration while retaining its exact path. Backups and the finite adoption manifest are in `.harness/.local/convention-roles-before-20261116/`. No installed PATH program, live daemon or existing Agent snapshot is restarted or rewritten.

## Completed local outcome

The existing `just package .bin/agentx-convention-roles` release build completed and the exact candidate was adopted as `.bin/agentx` (0.7.31). [Build evidence](evidence/package.log) and [project adoption](evidence/project-adoption.json) identify the exact SHA-256. All eight published roles are discovered. Six redundant project TOML wrappers were removed; `.agentx/roles/controller.toml` now contains only `default = true`, at its original source path. The existing default profile stays `codex-gpt-6-1-sol-high`. Its coder dry-run freezes that actual profile into the adopted role. Live services, installed PATH programs and historical Agent snapshots were not restarted or rewritten.

The exact candidate passed [convention CLI and native launch checks](evidence/convention/report.json), including no-default discovery, explicit override, persisted snapshot, mixed-source auxiliary discovery, disabled fallback, default-directory reuse and shipped CLI/bootstrap reference; the same run passed [public HTTP checks](evidence/convention/http/result.json), including Markdown-only workspace discovery, deferred-profile error, actual default-profile launch and thin default route preview. [Legacy regressions](evidence/roles-regression.json) passed discovery, invalid definitions, composition and fork/resume history. [Auxiliary-role execution](evidence/auxiliary.json) passed all 15 checks. Disposable fixture cleanup completed; no paid Provider was used.

[Native cases](evidence/native-tests.log) and [final guidance/help cases](evidence/guidance-and-help-tests.log) retain exact execution. Broader help checking has the single previously recorded unrelated `instruction_sources` documentation coverage failure; the other 14 help cases passed. This does not establish Windows/macOS, external paid-model or live-service adoption. The browser consumers passed required build-time TypeScript checks and fresh embedded asset compilation; no visual/browser interaction acceptance is claimed for this bounded catalog-label change.

A fresh Agent starts from [root AGENTS.md](../../AGENTS.md), [role usage](../../.docs/roles-workspaces.md), the source module guides and `agentx roles --help`. Shared methods remain harness-owned; this native compatibility change introduces no duplicated `.agent-guides/local` methods.
