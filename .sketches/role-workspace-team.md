---
scope: Proposed directory-scoped roles, workspace membership and default message routing
status: accepted-design
owner: agentx-design
last_updated: 2026-11-14
related: ["../.docs/launch-profiles.md", "../.docs/agent-context.md", "../.docs/architecture/environment.md", "../.docs/model/README.md"]
---

# Roles, workspaces and implicit teams

For the subsequent exploratory clarification of workspace membership versus execution directory and role namespaces, see [workspace and directory discussion](workspace-directory/README.md). It does not change this design's accepted semantics.

## Working state and authority

The user first requested a complete design discussion and then accepted the design and authorized implementation on 2026-11-15. The [Proposal](../.docs/proposals/roles-workspaces.md) records the selected delivery boundary; [Delivery](../.controller/roles-workspaces/delivery.md) owns implementation and evidence. This sketch retains the model and rationale; executable support must be read from Delivery and current usage rather than inferred from examples.

Examined: existing profile selection/provenance, fixed instructions versus refreshed runtime context, originating environment ownership, and CLI scope conventions. Outcome: the model and deciding cases below form the implementation basis. Native contracts and source ownership are recorded in the [team entry](../.controller/roles-workspaces/AGENTS.md).

## Domain model

| Concept | Owns | Identity and invariants |
| --- | --- | --- |
| AgentX scope | Runtime database, daemon and shared configuration | Existing canonical `AGENTX_DIR`; never implicitly changed by a workspace selection |
| Profile | Provider/tool and native launch arguments | Existing profile key within the AgentX scope; reuse current resolver |
| Role definition | Profile reference, fixed role instructions, optional default-recipient declaration | `(definition namespace, role name)`; whole-file shadowing, no field merge |
| Workspace | Stable collaboration boundary associated with a canonical directory | Canonical path within the current device and AgentX scope; not process cwd |
| Agent instance | Runtime identity, adopted role snapshot, workspace membership, state | Existing participant/incarnation identity; many instances may use one role |
| Team | Current members of one workspace | Derived grouping initially; no duplicate membership file or new team identity |
| Default endpoint | One reusable recipient slot for a workspace's selected default role | At most one current bound instance; ordinary role launches need not acquire this slot |

A role's definition namespace, its instance's workspace, and its process cwd are distinct. A global role used in workspace A produces an A member. Using a parent role definition in child workspace B produces a B member. Parent/child process lineage is also separate from team membership. Existing collaboration-workbench assignment/report roles retain their domain meaning; role templates do not automatically grant those authorities.

## Definition layout and resolution

Global definitions: `$AGENTX_DIR/roles/*.toml`. Local definitions: `<directory>/.agentx/roles/*.toml`. Names derive from filenames, using the existing profile-style safe identifier grammar; no duplicated editable name field. Example:

```toml
description = "Coordinate the workspace and receive its incoming requests"
profile = "codex-deep"
default = true
prompt_file = "../prompts/controller.md"
```

`prompt` is an inline alternative to `prompt_file`; exactly one is required. Relative prompt paths resolve against the role TOML's directory. File prompts prepend their canonical absolute source path and tell the Agent to resolve relative references from that file's parent; the original body stays literal. This source context is adopted with the role snapshot, independently of process cwd. The role references an existing profile in the current AgentX scope; it does not duplicate tool/model fields. No role inheritance, executable templates, includes, or additional local profile discovery is introduced.

Resolve an unqualified role from the selected workspace through its ancestor directories, then global definitions. The nearest same-name definition replaces the entire farther definition. A selected malformed file or missing profile is an error; never silently fall back. Namespace identity uses the canonical definition directory, not just a display label such as `local`. Proposed explicit selectors are `global:controller` and `local:controller`, with `local:` meaning exactly the selected workspace. A qualified shadowed ancestor role can be deferred until a concrete caller needs it; `roles` must still expose its source and shadowed status.

Keep global and local namespaces logically distinct even if their physical role directory is the same because `AGENTX_DIR` equals a project `.agentx`; deduplicate file reads. An explicit selector chooses the logical origin; an unqualified match uses the nearest local origin. Moving `AGENTX_DIR` intentionally selects different shared profiles/roles and runtime scope.

## Workspace and cwd selection

Proposed rules, highest priority first:

1. Explicit `--workspace PATH` pins membership; explicit `--workspace global` chooses no directory membership. Reserve `global` as a selector token; `./global` denotes a real directory of that name.
2. Explicit `--dir PATH` selects a target by directory discovery, even if the caller inherited a workspace.
3. Otherwise use the bound caller's stable workspace, transported by `AGENTX_WORKSPACE` to its child processes.
4. For an unbound shell, discover from cwd.

Discovery finds the nearest ancestor, including the starting directory, containing `.agentx/roles/`. That directory is the workspace root; an empty roles directory is sufficient to mark a workspace. If none exists, use the shared global context, preserving unconfigured usage. Search follows filesystem parents through `/`, with canonicalized existing paths; there is no implicit Git-root stop. Explicit `--workspace PATH` supports a directory without a roles marker.

For launch, explicit `--dir` is process cwd; without it, use the selected directory workspace root, or caller cwd for global context. With both flags, `--workspace` owns membership and role lookup while `--dir` owns cwd. Relative CLI paths resolve from the invoking process cwd. Directory discovery may therefore select a parent workspace while retaining a deeper explicit launch cwd.

Workspace membership is persisted at registration. For a bound agent, changing cwd or editing an environment variable does not mutate its recorded membership; the environment transports the default to unbound descendants and is not identity/authentication evidence. An explicit cross-workspace command changes that operation's target, not the caller's membership. A child explicitly launched elsewhere keeps its original parent identity but joins the target workspace.

## Discovery and command model

All syntax below is proposed. Common `--dir` and `--workspace` selectors apply to affected discovery, launch and messaging commands. Keep the existing device and AgentX-scope boundaries.

| Intent | Command | Result |
| --- | --- | --- |
| Discover usable definitions | `agentx roles` | Effective roles, profile, origin and default-route declaration; no process launch |
| Inspect shadowing | `agentx roles --all --json` | Visible local/ancestor/global definitions including shadowed definitions; no recursive disk scan |
| Inspect role | `agentx roles show controller` | Resolved definition and prompt source |
| Start another role instance | `agentx launch --role controller` | New instance in selected workspace, not implicit reuse |
| Preview launch | `agentx launch --role controller --dry-run --json` | Role/profile provenance, instructions composition, membership and cwd; no launch |
| Inspect current team | `agentx list` | Current workspace members plus explicitly marked global members |
| Inspect another team | `agentx list --dir /repo/b` | B's discovered workspace members plus global members |
| Inspect every team | `agentx list --all` | All members in the current AgentX scope/device; no cross-scope federation |
| Message existing instance | `agentx send @bime -- 'Review this'` | Exact existing recipient; no automatic creation |
| Message directory endpoint | `agentx send --dir /repo/b -- 'Review this'` | Resolve/reuse/create B's default endpoint and durably enqueue one message |

Agent lists show actual instances, never rows fabricated from role files. Include workspace, role origin and existing runtime status. Team membership includes agents without roles when explicitly registered in that workspace. A local list does not recursively include child workspaces. Listing from a parent project or `/` is not an all-members operation; `--all` is explicit. Global definitions are visible everywhere, but only globally assigned instances are global members. Preserve existing inactive/history filtering rather than inventing a new history policy here. No wildcard recipient or broadcast is implied by a directory target.

An explicit `@instance` identifies an existing recipient using current scope-wide identity rules; workspace defaults filter discovery, not authorization or exact-ID addressability. If a directory selector accompanies an explicit recipient, require membership in that selected workspace or global context and reject mismatch. Do not reinterpret an explicit instance name as a role or auto-create a replacement.

## Default-directory messaging

At each directory namespace, allow zero or one `default = true` declarations. Multiple defaults are a configuration error. To resolve a directory endpoint, inspect the target workspace then ancestors, finally the global namespace. Select the nearest namespace that declares a default. Resolve the exact declaring role definition, even if a child shadows its name. Malformed candidate configuration is an error, not permission to route farther upward. No default anywhere means an actionable error with no launch.

Crucial distinction: role lookup inherits a definition; default routing delegates to an owning endpoint. If `/repo/ui` has no default and `/repo` declares a default controller, a message for `/repo/ui` goes to `/repo`'s endpoint and controller. It does not create one parent-role controller per child directory. Return both requested directory and resolved owner; provide requested-directory context alongside the message without silently rewriting its text.

The daemon owns endpoint binding and race-free creation admission; do not implement CLI `list` followed by unguarded `launch`. Reuse a valid bound recipient, including a busy one, through its existing durable mailbox. With no binding, adopt a single unambiguous compatible live instance of the selected role/workspace if available; if several qualify, require explicit selection rather than guessing. Otherwise reserve the endpoint and initiate one launch. Concurrent sends share this reservation and retain separate messages. `launch --count N` does not create N default endpoints.

An unresolved launch, model error or uncertain delivery is not permission to spawn a replacement or replay input. Report the existing recipient/state or recover through existing lifecycle rules. A definitively stopped recipient permits a new binding after retirement is confirmed. Reuse the native request/message identity, incarnation fencing, launch receipts and mailbox delivery rules. An enqueue/launch failure must preserve operation and message identities for diagnosis; a recipient may have been created even when the command ultimately reports an error.

`send` success establishes durable message acceptance with recipient/endpoint identity, not completion of the requested work. JSON results should expose requested/resolved workspace, role origin, recipient, creation disposition, message ID and acceptance state. Diagnostics use stderr. An inspection/dry-run path resolves the route without creating the recipient.

Editing a default declaration changes new routing resolution. Rebinding an existing endpoint to a different role must reject with a clear conflict until an explicit lifecycle operation replaces its binding; it must not silently spawn a second controller. Editing prompt/profile contents does not mutate a running recipient. Reuse its frozen launch snapshot and expose drift, or explicitly replace it through existing lifecycle controls.

## Instructions, profile overrides and history

Compose the builtin operating guide, role instructions and explicit per-launch instruction additions through the existing fixed-instruction channel. Role selection must not accidentally remove AgentX communication guidance. Do not promise uniform literal system-message priority: providers expose different native channels, as documented in the current context contract. Current peers, role availability and endpoint state belong in refreshed runtime context, not frozen role text.

`--role R --profile P` explicitly overrides R's referenced profile for this instance; record both the declared and selected profile. Role selection suppresses ambient `AGENTX_PROFILE` and launch-profile defaults. Preserve current native-argument merge/replace behavior and reject a conflicting explicit tool. Reject `--role` with `--no-profile`, since this role model requires a profile. `launch --role` creates a fresh agent; `send --dir` performs endpoint reuse.

Persist role namespace/name, adopted definition and rendered instruction snapshot, selected profile provenance, workspace, cwd, and parent identity in the existing launch record. Resume uses adopted history, not edited role files. Fork follows existing new-identity semantics and retains compatible role/profile content while regenerating identity-bearing instructions; it must not copy another participant's identity or endpoint lease. Remote resolution happens on the target device, including canonical paths, role/profile lookup and prompt files; local definitions are not silently uploaded.

## Deciding cases and open choices

| Case | Recommended outcome |
| --- | --- |
| Global `reviewer` launched in A | A member with global role provenance |
| Same role launched twice | Two distinct participant identities |
| A member changes cwd to B | Remains A; explicit B-targeted operation is required |
| Child role absent, parent definition present | Launch in child workspace using parent definition |
| Child default absent, parent default present | Route to parent endpoint, disclose delegation |
| Child shadows parent default's role name but declares no default | Parent endpoint still uses its own exact role definition |
| Two concurrent first sends | One endpoint creation, two accepted messages |
| Busy endpoint | Queue message for the same recipient |
| Endpoint in unresolved error/launch state | Surface state; no speculative replacement |
| Empty roles marker | Separate workspace with inherited available definitions |
| No workspace marker and no explicit workspace | Global context |
| Invalid selected role/profile or duplicate defaults | Fail with source diagnostics; no silent fallback |

Product choices still open: whether global instances should be included in every default list (recommended here to match the user's visibility request); whether default declarations stay in role files or move to one workspace manifest if additional team policy actually appears; and whether role names need fully qualified ancestor selectors in the first release. The default endpoint binding will need a concrete native inspection/replacement operation before implementation; existing instance kill alone must not become an ambiguous endpoint-reconfiguration protocol.

Rejected for the initial design: one daemon/database per workspace, equating global role origin with global membership, automatic tree-wide list, deriving membership from mutable cwd, treating role names as unique instance names, role/profile field-level inheritance, launching every declared role merely because a team exists, and introducing a team workflow engine. Discovery is a usability boundary, not a filesystem/process security boundary.
