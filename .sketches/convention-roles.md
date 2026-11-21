---
schema: document
status: accepted
description: Discussion model for convention-derived Markdown roles with scoped TOML overrides and launch-time profile resolution.
---

# Convention-derived roles

The user proposes discovering roles directly from `.agent-guides/roles/NAME.md` and `OWNER.HELPER.md`, using configured launch defaults unless a same-name `.agentx/roles/NAME.toml` customizes them. This note records the recommended compatibility model and source-grounded constraints. The user approved implementation; actual progress and verification are owned by [the implementation Task](../.controller/convention-roles/implementation.md). Existing [role usage](../.docs/roles-workspaces.md) and [profile usage](../.docs/launch-profiles.md) describe current behavior.

## Outcome and responsibility

Installing a guidance release supplies reusable professional instructions; selecting an actual profile supplies the Provider/model/runtime policy. Ordinary consumers should not duplicate role/profile/prompt-file wrappers. Project TOML remains the explicit environment/customization layer. Preserve existing complete TOML definitions, local/global selectors, nearest-workspace precedence, two-level auxiliary discovery and adopted snapshots.

| Concern | Recommended rule |
| --- | --- |
| Markdown discovery | Scan only the role directory's immediate `NAME.md` and `OWNER.HELPER.md` entries with explicit `schema: role`; validate filename identity and any declared `role`. Read description from metadata and retain literal prompt content/source-path context. Do not recursively convert reference documents into roles |
| Same-workspace resolution | `.agentx/roles/NAME.toml` first, then `.agent-guides/roles/NAME.md`; a near workspace's Markdown definition precedes a farther workspace's TOML definition. Retain explicit local/global scope restrictions |
| TOML customization | Preserve old complete definitions. Permit a thin override to reuse the same workspace's Markdown prompt when neither prompt field is supplied. An explicit prompt/prompt_file replaces the base prompt; do not silently concatenate instructions or merge across unrelated scopes |
| Profile selection | Omission means use the existing launch-profile selection rules, after explicit CLI profile and explicit role profile. Do not encode a special literal profile name `default`. Freeze the actual effective profile at launch |
| Listing without configuration | Discover and inspect Markdown roles even if no default profile exists. Launch/dry-run reports an unresolved or invalid selected profile; do not let one missing launch default hide the whole catalog |
| Default recipient | A launch default profile does not select a directory's default role. Keep explicit `default = true` in a project TOML override; do not infer an endpoint from `controller.md`, alphabetical order or package publication |
| Disablement | A small explicit TOML opt-out such as `disabled = true` should suppress that name at the selected scope, including fallback; deleting its override would otherwise reveal the shared definition again. An invalid selected override remains an error, not silent fallback |
| Workspace discovery | Recognize a project role source without requiring redundant TOML files. Preserve explicit/inherited workspace precedence and identify the effect on previously global directories |
| Auxiliary ownership | NAME and OWNER.HELPER belong to a logical workspace namespace, independent of whether each definition came from Markdown or TOML. Keep source-path provenance separate; an override must not accidentally hide same-workspace helpers or expose another workspace's helpers |
| History and endpoints | Retain adopted role text/profile and original launch provenance on resume/fork. Moving a default role's definition path must not silently reroute or replace an existing endpoint; preserve its TOML entry or require the existing explicit retirement/reset path |
| Actual consumers | CLI role list/show/launch, directory send, HTTP/management discovery and browser projections must consume one role resolution model; no separate browser-only Markdown scan |

A representative setup would have shared `coder.md`, `coder.test.md` and `controller.md`, no coder TOML by default, an optional `coder.toml` containing only `profile = "deep"`, and `controller.toml` containing only `default = true`. Thin override syntax is part of the accepted implementation scope; the implementation Task owns executable verification. Configuring `agentx profile default NAME` would supply the ordinary role launch base; it would not make a directory endpoint implicitly available.

## Pre-change source-grounded compatibility seams

| Source before this follow-up | Observed constraint |
| --- | --- |
| [Role resolver](../src/config/roles.rs) | Workspace discovery recognizes `.agentx/roles/`; namespace identity is the canonical physical role-directory path; only TOML files are scanned |
| [Role resolver](../src/config/roles.rs) | Definition requires a profile; prompt/prompt_file is exactly-one; reading a role validates its profile immediately; all are resolved into one `ResolvedRole` |
| [Launch command](../src/cli/commands/launch.rs) and [profile resolver](../src/runtime/launch_profile.rs) | An adopted role currently supplies a concrete named profile before runtime preparation. Optional selection needs to reuse profile precedence and preserve actual launch provenance |
| [Saved collaboration and endpoint policy](../src/agents/workspaces.rs) | Saved roles require nonempty profile/source identity; endpoint compatibility compares role source paths. An unresolved definition must not be persisted as an adopted snapshot |
| [Management roles](../src/management/offices.rs) | Management projects the native resolved role; source/discovery changes must reach the existing public consumer rather than create another catalog |

The session graph service had only an xpm index, not the AgentX checkout. These findings come from current direct source inspection; no graph completeness or runtime experiment is claimed. These observations describe the pre-change source; follow the implementation Task for current executable evidence.

## Accepted implementation decisions

This recommendation favors thin TOML customization over requiring complete shadow definitions, and explicit recipient selection over a filename convention for controller. Disablement and Markdown metadata should remain bounded fields with a clear error/fallback policy. Keep these choices together when implementation is selected; avoid adding a plugin registry, generated TOML files, recursive imports or a new profile system.

Meaningful implementation cases are: Markdown-only project plus default profile; no profile with successful listing but actionable launch failure; CLI profile override without a global default; profile-only and prompt-replacing TOML; malformed/disabled same-name override; local Markdown versus parent/global TOML; mixed-source auxiliary ownership; unchanged complete TOML behavior; saved role/profile restoration; bound default endpoint continuity. Run focused native tests and exercise the actual CLI/management consumer only within the subsequent implementation scope.
