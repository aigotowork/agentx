# Roles/workspaces implementation

For the user-selected Git/project workspace defaults, inherited membership and independent cwd correction, read [accepted discovery design](../../.docs/proposals/workspace-discovery.md) and [active implementation Task](tasks/workspace-discovery.task.md). It supersedes original discovery priority details; native membership and endpoint ownership remain unchanged.

Read root [AGENTS.md](../../AGENTS.md), [Proposal](../../.docs/proposals/roles-workspaces.md), [design](../../.sketches/role-workspace-team.md) and [Delivery](delivery.md). User authorized full implementation. Existing working tree is shared and dirty: do not reset, commit or replace live binaries/services. Agents must use codex/gpt-6.1-sol with max reasoning; files carry handoff contracts. Do not delegate to existing unrelated agents.

The [original local implementation is accepted](delivery.md#accepted-local-outcome). The user's [guidance follow-up](tasks/guidance.md) owns startup/help discoverability; it does not reopen earlier implementation assignments. The ownership table below records the original implementation split. Maintenance follows the linked current usage, native declarations and module guides. Exact executable evidence and unsupported paths remain in Delivery and the acceptance Task.

## Source ownership and structural contracts

| Task | Exclusive files/responsibility |
| --- | --- |
| roles | `src/config/roles.rs`, `src/config.rs` module declaration, `src/cli/commands/roles.rs`; pure role/workspace discovery and role CLI |
| launch | `src/runtime/launcher.rs`, `src/runtime/launch_options.rs`, `src/cli/commands/launch.rs`, `src/cli/commands/resume.rs`; role/profile composition, child environment, frozen instructions, historical launch |
| membership | `src/agents/workspaces.rs`, `src/agents/launch_service.rs`, `src/agents/launch_store.rs`, `src/agents/mod.rs`, `src/agents/invocation_binding.rs`; persisted collaboration metadata, atomic endpoint binding and native lookup |
| integration/controller | routing/help/module registration, list/send commands, bootstrap context filtering, integration fixes and docs |
| acceptance | independent checks and fixtures after source release; no competing implementation edits |

No new DB columns are required: persist the collaboration object under `launch_context.collaboration`, and retain it across terminal context updates and snapshots. A role is adopted data, not identity authority. Prefer existing durable KV for endpoint bindings and existing native launch transaction for unique reservation.

## Agreed interfaces (refine only by notifying consumers)

`config::roles::ResolvedRole` is Clone/Debug/Serialize/Deserialize with fields `name: String`, `path: PathBuf`, `namespace: String`, `profile: String`, `prompt: String`, `description: Option<String>`, `default: bool`. Explicit prompt_file paths use the TOML parent; convention Markdown prompts use their own parent. File prompts include a canonical absolute source-file header and parent-relative reference guidance followed by the literal UTF-8 body; inline prompts stay unchanged. Global selectors are `global:NAME`; `local:NAME` means selected workspace only.

Role functions return `anyhow::Result`: `select_workspace(cwd: &Path, env: &HashMap<String,String>, dir: Option<&str>, workspace: Option<&str>) -> Result<Option<PathBuf>>`; `resolve_role(scope: &Path, workspace: Option<&Path>, selector: &str) -> Result<ResolvedRole>`; `list_roles(scope: &Path, workspace: Option<&Path>, all: bool) -> Result<Vec<RoleDefinition>>`; `resolve_default(scope: &Path, workspace: Option<&Path>) -> Result<DefaultRoute>`. `DefaultRoute { pub role: ResolvedRole, pub workspace: Option<PathBuf> }`. CLI `roles::run(argv: &[String]) -> Result<i32>` captures its own invocation; passive discovery does not acquire a daemon, while explicit endpoint reset does. CLI `roles [show NAME] [--dir PATH] [--workspace PATH|global] [--all] [--json]`.

`agents::workspaces::LaunchCollaboration { pub workspace: Option<String>, pub role: Option<ResolvedRole>, pub endpoint: bool, pub instruction_additions: Option<String> }` is Clone/Debug/Serialize/Deserialize. `instruction_additions` defaults to None for older records and stores raw fresh-launch fixed instructions, never resume/fork identity text. `LaunchParams` and native `launch_service::Reserve` gain `collaboration: Option<LaunchCollaboration>`. None means legacy/unset; launch runtime resolves a default collaboration once before reservation. Explicit global uses Some with workspace None. Native store persists the object under `launch_context.collaboration`; endpoint bool is a launch request and not participant authority. Expose `workspaces::collaboration(row: &InstanceRow) -> Option<LaunchCollaboration>` and `workspace(row: &InstanceRow) -> Option<String>` for listing/runtime consumers.

Native `launch_service::Request::EndpointLookup { workspace: Option<String>, role_path: String }` returns `launch_service::Response::Endpoint(Option<ParticipantRef>)`. Lookup reuses a valid binding or identifies exactly one existing compatible instance, rejects ambiguous matches and role drift, and does not spawn. `Reserve` rechecks endpoint uniqueness within its write transaction and binds the exact new ParticipantRef atomically. Concurrent duplicate reservations fail with a recognizable conflict and must not spawn; send can reread EndpointLookup to use the winner. Persist endpoint lease by workspace, validate incarnation, and do not replace a live/error/uncertain participant. Only definitively retired bindings may be reused. Expose `EndpointReset {workspace: Option<String>}` only if needed for explicit retired-role reconfiguration; never reset live recipient implicitly.

Controller owns send orchestration: resolve exact default route and profile, inspect endpoint, launch with endpoint collaboration when absent using the supplied Session client and original caller, then submit existing durable message to exact returned name. Keep arbitrary CLI provider and native mailbox work out of daemon transactions. Preserve sender, intent/thread/bundle semantics and existing plain sends. Route failures expose actual state rather than retrying blind.

## Handoff protocol

Implement a typed skeleton first, then behavior and targeted tests. Send the controller the final symbols, changed files, checks, and limitations through AgentX; update the owning Task. Do not run full Cargo builds concurrently: controller schedules compile/test and independent acceptance after source release. Unit tests can be authored by implementers; acceptance execution belongs to a different agent. Mark runtime support only after executed evidence.

The user-authorized [auxiliary discovery follow-up](auxiliary-roles.task.md) owns two-level role names and caller-specific discovery without access restrictions.

For the accepted Markdown convention design and same-name TOML customization, read the [design model](../../.sketches/convention-roles.md) and the follow-up maintenance entry below.

The user approved convention-derived Markdown discovery. [Current implementation and evidence](../convention-roles/AGENTS.md) own the follow-up; [role usage](../../.docs/roles-workspaces.md) is the current contract. Earlier explicit-TOML evidence remains historical.
