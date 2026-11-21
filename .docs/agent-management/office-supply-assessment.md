---
schema: document
description: Source-verified disposition of visual-office API supply, projection gaps and native query boundaries.
status: assessed
owner: agent-management
last_updated: 2026-11-15
links: [./design.md, ../agent-space/api-supply-request.md, ../agent-space/concept-alignment.md]
---

# Visual office supply assessment

## Decision and evidence boundary

The [consumer request](../agent-space/api-supply-request.md) is compatible with the management architecture. Supply adopted collaboration metadata through the existing exact Agent projections first; do not invent office/person storage or expose raw native records. Current source still has no workspace/Role fields in [AgentView](../../src/management/contract.rs), no Role/workspace selectors in LaunchInput, and no office/catalog/route-preview HTTP operation. The completed r4 acceptance establishes the existing flat management flow only. This assessment inspects current source; it does not establish any new executable support, modify the offline frontend or change the LAN preview on port 18967.

## Disposition

| Requirement | Current support and source | Required work / boundary |
| --- | --- | --- |
| G4 exact selection and safe actions | AgentRef, dataset generation, bounded AgentPage, capability flags and durable Operation are implemented in [contract](../../src/management/contract.rs) and [store](../../src/management/store.rs). | Reuse unchanged identities and request recovery. A selected scope contains local-origin records; page completion is not a machine-wide/remote office inventory. Polling is an observation, not a live-process guarantee. No aggregate count or authoritative per-field observation timestamp is currently supplied. |
| G6 communication/model reading | MessagePage and asynchronous Transcript result exist; exact stopped message windows and successor isolation are accepted. ExchangeView supplies user/assistant/action/files, availability and reason. | Reuse actual supplied granularity. Message IDs/authors are native communication facts, not an exact participant ref on every author. Transcript is a bounded snapshot without a continuation contract; do not imply complete arbitrary-length history or detailed reasoning/tool blocks. Viewer unread remains local; native mailbox unread is a different reader. |
| G2 adopted membership and Role | [LaunchCollaboration](../../src/agents/workspaces.rs) stores workspace and ResolvedRole in launch_context. Stopped events preserve that context; [resume selection](../../src/agents/lifecycle_resume_store.rs) reads it. | Add safe projections to existing live and stopped AgentView; no new membership storage or endpoint is needed for this first slice. Read the exact stopped event, never a current namesake or current Role file. |
| G1 known offices | Descriptor has scope/project. Native [workspace selection](../../src/config/roles.rs) resolves a specified context upward to the nearest `.agentx/roles/` marker; explicit membership needs no marker. | Membership-derived known offices are feasible first, with explicit coverage and completed-page limits. There is no scope-wide registry or recursive discovery API. Empty marked offices require a supplied/bounded discovery context; an arbitrary unvisited directory cannot be discovered from memberships alone. Scope data directory is not the filesystem search root. |
| G3 available posts | Native resolve_role/list_roles/resolve_default own precedence, canonical source, profile, description and defaults. `list_roles(all=false)` supplies effective definitions; `all=true` includes shadowed ones. | Add a bounded host-side native catalog read and safe metadata projection. Do not run filesystem resolution in SQLite dispatch or reimplement shadowing in the browser. Invalid selected definitions/profile references/default conflicts remain errors, not an empty catalog or silent fallback. Current APIs fail a catalog operation rather than provide partial per-file diagnostics; richer diagnostics need an explicit native result policy. |
| G3 actual directory entry | resolve_default returns the declaring workspace and exact Role. [directory send](../../src/cli/commands/send/directory.rs) then calls native EndpointLookup. | A configuration preview exists, but full read-only recipient preview needs a native query change; see below. Role default declaration or one visible Agent is insufficient evidence of an endpoint binding. |
| G7 Role + independent workspace launch | [launcher](../../src/runtime/launcher.rs) already resolves/adopts LaunchCollaboration and accepts cwd separately. Management worker currently selects tool/profile and passes cwd; launcher may discover membership, but HTTP cannot explicitly select Role or independent workspace. | Extend the existing launch command/worker contracts when selected. Reuse native Role/profile validation and canonical reservation; freeze the resolved adopted inputs at execution and preserve the existing one-request receipt. Explicit workspace/global choice must remain separate from directory and Role source. Reject conflicting Role/profile/tool selections under native policy; no profile/prompt substitution. |
| G7 directory contact / fork | Native endpoint reservation and route/send exist outside management. Current HTTP Send targets an exact Agent, Resume continues a saved execution; no Fork command exists. | Later route+send must retain native endpoint arbitration, failed/uncertain occupancy and operation recovery. Preview does not authorize a future recipient; execution revalidates. Fork is a separate future command, not Resume or fresh Launch. Existing exact send/stop/resume/transcript remain reusable. |
| G5 optional details | Native row launch_profile retains adopted provenance; directory is stored/reported state. Descriptor profiles describe the current catalog. | Adopted profile can be a later safe projection; never expose launch env/argv/secrets. Current AgentView provides neither observed model nor verified current cwd/freshness. Such observations require qualified native/Provider data, not profile-name inference. |
| G8 Skills | No unified management Skill catalog or usage history in current contracts. | Defer. Unsupported/unavailable differs from an observed empty set; transcript prose does not establish usage. |

## First read slice: adopted collaboration

Extend the existing native projection and regenerate TypeScript from its canonical contract. Project a safe adopted Role subset: name, original namespace/canonical definition path, profile reference, description and original default declaration. Exclude frozen prompt/instruction additions, environment, endpoint tickets and raw launch_context. The saved default flag is historical declaration, not current route authority. Current role edits/deletion must not affect this projection.

Use an explicit observation state rather than one overloaded nullable workspace or Role. Recommended semantic states are:

| Stored observation | Membership meaning | Role meaning |
| --- | --- | --- |
| Valid collaboration with workspace path | Confirmed adopted directory membership | Adopted safe Role snapshot, or confirmed no Role when role is null |
| Valid collaboration with workspace null | Confirmed global membership | Adopted Role or confirmed no Role, independently |
| Absent historical collaboration/context | Native legacy-unassigned/global policy, provenance explicitly legacy; original intent is not recoverable | No recorded adoption; do not invent a historical Role or assert an explicit user opt-out |
| Invalid JSON, malformed collaboration, incompatible/unsupported shape | Unavailable with bounded reason | Unavailable; never silently global/roleless |

A tagged collaboration observation can encode this once, with workspace/Role nullable only inside a valid snapshot; exact names belong to the implementing native contract. The existing `workspaces::collaboration` helper returns None for both absence and parse failure, so it cannot establish this distinction. Add a checked native decoder shared by the relevant projections rather than changing legacy membership semantics or fabricating history. No database migration is required for valid existing snapshots. Lost historical metadata cannot be reconstructed from current files, cwd, prompts. A damaged stopped record should preserve usable identity/history where possible while marking collaboration unavailable.

This slice is self-contained: same paginated list/details, same generation/exact refs, both current and stopped metadata, and explicit unavailable/legacy cases. It enables truthful grouping of the members actually loaded; until known-office discovery is supplied, the frontend must label offices as membership-derived and partial while paging. It does not promise discovery of empty offices.

## Read-only directory preview requires a native boundary

[Endpoint lookup](../../src/agents/workspaces.rs) requires a write transaction and may bind a sole compatible candidate through `bind_endpoint`. It is unsuitable for an observational HTTP GET or a read-only job. [send --dry-run](../../src/cli/commands/send.rs) calls only directory::resolve; its Address.recipient stays null and does not distinguish no endpoint from an unchecked endpoint. Do not wrap either path and call it complete read-only preview.

Introduce a native non-mutating inspection operation alongside existing endpoint policy, reusing retirement/compatibility predicates. It must distinguish a valid bound exact participant, a sole adoptable but unbound candidate, multiple ambiguous candidates, no candidate, stale/missing binding, Role drift and unavailable/uncertain state. Preserve pending/failed endpoint occupancy; a preview must not adopt, clear, launch, send or infer retirement from absence. Combine that observation with host-side default resolution to return requested directory/workspace, actual declaring workspace/Role and an explicitly observed endpoint disposition. A preview is a point-in-time observation; the later route/send operation re-resolves and arbitrates atomically. This needs a native query contract, not a new registry or a requirement to create a particular HTTP route now.

## Sequence and decisive examples

1. **A: member read supply.** Enrich existing exact live/stopped views and add checked legacy/unavailable decoding. Verify independent cwd/workspace, inherited/global Role origins, edited/deleted Role files, stopped namesakes and damaged/legacy snapshots. No frontend rewrite or lifecycle changes.
2. **B: known contexts and post browsing.** Start from membership and explicitly supplied directory contexts; report provenance/coverage and discover empty markers only within that finite scope. Reuse native filesystem resolver in bounded host work. Add pure endpoint inspection before promising actual recipient preview. No full-disk scan or workspace registry.
3. **C: selected native actions.** Add Role/workspace selectors to existing launch composition, then journaled directory route/send when selected. Preserve fresh/resume/fork distinctions and [existing runtime restrictions](../roles-workspaces.md#验证边界), including unsupported shared Codex app-server stdio/remote fresh Role/workspace launches. Optional profile/model/Skill enrichment does not block A.

| Consumer example | Disposition |
| --- | --- |
| Two inherited-role web members, one cwd `/tmp` | A projects the same adopted web membership and source Role for both exact refs; cwd stays detail. |
| Global-origin Role adopted in web; global member working in web | A keeps Role origin and membership independent; global member appears only in global. |
| Empty `.agentx/roles/`; only `.agentx/`; explicit marker-free membership | B can inspect a supplied empty marked context and inherited posts. `.agentx/` alone is not a marker. A still shows explicit marker-free membership; an unvisited empty office is outside coverage. |
| Roleless, unavailable Role, stopped namesake | A's observation state distinguishes these while preserving exact refs; drafts/actions remain per execution. |
| Adopted Role file edited/deleted | A reads saved provenance; B reads current definitions/errors. No retroactive rewrite. |
| Child inherits parent default; multiple compatible members | B returns the declaring parent/global context. Actual binding/adoptable/ambiguous disposition needs the pure native inspection; no invented child controller. |

## Conflicts, decisions and next authorization

No conflict with the selected office semantics or existing API direction. The principal conflicts are implementation shortcuts: cwd grouping, treating parse failure as legacy global, treating default Role as actual recipient, calling mutating EndpointLookup a read, or replacing Role adoption with prompt/profile launch. These are rejected above.

No user preference is required to complete this assessment. Before implementing B, choose a finite discovery input/budget and specify catalog partial-error policy; neither has a native complete inventory contract today. Before C, define Role/profile/tool conflict admission and resolution timing in the existing command contract. This assessment recommends A first but does not authorize implementation or reopen the completed original management Delivery. Record a selected follow-up there when authorized; retain the existing accepted r4 artifact and evidence.
