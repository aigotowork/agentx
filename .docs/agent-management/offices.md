---
schema: document
description: Unified office management contracts and ownership for real membership, effective posts, route inspection and native actions.
status: active
owner: agent-management
last_updated: 2026-11-17
links: [./design.md, ./office-supply-assessment.md, ../../.controller/office-management/delivery.md]
---

# Unified office management

## Outcome and composition

The user authorized all assessed A/B/C capabilities together. They form one Delivery, not three release/approval gates. Native participant policy owns saved collaboration and endpoint authority; host reads own bounded filesystem observation and Role resolution; the existing management journal owns once-only command execution; React consumes generated contracts. Source and final receipts, not this target design, establish support. The earlier r4 candidate and visual office LAN :18967 remain unchanged. The existing daemon management console is the usable Web consumer; web/agent-space remains an independently owned offline frontend with a replaceable future supply adapter.

Dependency direction: web/management → management HTTP → typed native management and bounded host office queries → native participant/config/runtime. No new office registry, person identities, Provider protocol or workflow framework. Native declaration src/management/contract.rs owns exact DTO names. Main boundaries:

| Owner | Modules and contracts |
| --- | --- |
| Integration | contract.rs public types, store.rs exact projections/admission, web.rs HTTP/composition and generated native.ts |
| Native policy/read | agents/workspaces.rs checked decoder + pure endpoint inspection; launch_service/store additive inspection transport; management/offices.rs bounded host reads |
| Runtime | worker.rs Role/workspace launch + directory send; shared native directory orchestration, no HTTP dependency |
| Web | Existing management console components/controller plus local office disclosure; one operation recovery system |

## Facts and identities

AgentView gains `collaboration: CollaborationView`. Status is `adopted`, `legacy` or `unavailable`. An adopted record has independent nullable workspace/role. Null workspace means explicit global; null Role means confirmed roleless. Legacy absence follows native unassigned/global behavior but is visibly legacy/no recorded Role, not an explicit user choice. Malformed/unsupported collaboration or invalid absolute membership/source cannot become global; projection returns unavailable with a bounded reason. Current and stopped projections use the same checked decoder, with stopped data pinned to the requested stopped event. Preserve original Role name/path/namespace/profile/description/default; namespace is the native origin plus canonical role directory (for example `global:/scope/roles` or `local:/project/.agentx/roles`), not a Role selector; omit prompts, instruction additions, raw context/env/argv. Current catalog definitions remain separate from adoption.

Existing generation, exact AgentRef and page/operation behavior stay intact. Existing directory field is stored/reported cwd, not membership. Frontend filters only exact adopted workspace or explicit legacy/global policy; unavailable records occupy their own group. Existing communication versus transcript granularity and their limits remain unchanged. No invented Skill/model observation or human unread count.

## Read surface and finite discovery

| HTTP | Meaning |
| --- | --- |
| GET /api/v1/workspaces?generation=… | WorkspaceCatalog: known adopted live/stopped memberships plus bounded marker discovery from configured project; explicit coverage |
| GET /api/v1/roles?generation=…&workspace=… | RoleCatalog for one explicit canonical workspace or literal `global`; effective native resolver order, safe RoleDefinitionView metadata (nullable profile), whole-query error on invalid catalog |
| GET /api/v1/directory-entry?generation=…&directory=… | DirectoryEntry: requested canonical directory, selected workspace using the shared project-root priority, exact default declaring workspace/Role, non-mutating endpoint inspection |

Optional directory input for workspace discovery is deliberately omitted: the configured project is the finite discovery root. Client can inspect roles at explicit absolute membership paths independently; catalog discovery is not registration. Offices include discovered Git/worktree roots, project `.agentx/` markers and existing `.agent-guides/roles/` boundaries, including empty configuration directories. The personal `~/.agentx` anchor does not mark all of HOME. Explicit marker-free memberships remain valid. No recursive scan outside project, follow-symlink recursion, node_modules/.git/target/.agentx internals, or hidden host home walk. Native upward workspace/default resolution remains independent from downward discovery. The [workspace discovery design](../proposals/workspace-discovery.md) owns the shared policy; its [Task](../../.controller/roles-workspaces/tasks/workspace-discovery.task.md) records current candidate support separately from earlier live adoption.

WorkspaceCatalog has generation, workspace rows `{path, sources}` and coverage `{agent_pages, agent_limit_reached, directories_examined, scan_limit_reached, errors}`. Collect at most 10 native list pages of 100, at most 512 directories and 8192 examined directory entries at depth ≤3 under project, and at most 100 workspace entries; any cap sets corresponding incomplete coverage (workspace cap uses scan_limit_reached). Membership paths come from adopted metadata only; legacy/global and unavailable are not invented path entries. Include the explicitly selected project only when marked or already adopted, not merely because it is cwd. Bound error text/count; missing/deleted paths stay known from saved membership but catalog reads report native filesystem errors. The frontend can group loaded AgentViews and show the independently bounded known-office catalog; it must not call their counts complete while agent pages remain.

Role catalog inputs are absolute existing directories or literal global, canonicalized by native roles::select_workspace with explicit workspace selection. Return canonical workspace and safe effective roles. Origin comes from original canonical namespace/path and selected workspace; no browser shadowing algorithm. `default` is declaration metadata; resolve_default independently determines actual inherited route. Malformed selected definitions and default conflicts return an actionable whole-query error. Catalog profile references are not resolved during discovery; missing/invalid selected profiles fail when a launch or default route actually needs one. Bound roles to 100 and reject overflow explicitly instead of silently truncating a claimed effective catalog. Host file reads run in bounded blocking HTTP tasks, never SQLite dispatch.

## Pure endpoint inspection

Native launch Request::EndpointInspect carries workspace and exact role_path; response is EndpointInspection. Implement alongside current endpoint policy without writing KV or touching participant/events. Preserve canonical workspace and exact Role identity comparison by canonical definition path. Same-file description/prompt/profile edits update the current catalog/default resolution but do not invalidate an existing endpoint or rewrite its adopted snapshot; this is not a content digest comparison. States: `bound`, `adoptable`, `absent`, `ambiguous`, `role_changed`, `stale`, `unknown`. Optional exact participant, bounded candidate count and reason distinguish binding versus observation. A current binding remains bound even when participant is failed/pending; readiness is reported separately through current AgentView/native send checks. Missing without exact retirement is unknown; namesake successor is stale; A changed default Role path or incompatible adopted membership/path is role_changed; retired exact binding permits inspecting current candidates without clearing or adopting it. One candidate is adoptable, never silently made bound. Share retired/compatible predicates with actual endpoint lookup.

DirectoryEntry combines host default resolution and native inspection. The requested directory is an explicit directory selector: search its ancestors for Git first, then AgentX markers; if none are found, continue through the originating environment, invocation cwd and personal fallback. Preview and execution use the same shared selector. No default or malformed files are explicit errors, not an absent Agent. Preview cannot grant future authority; real send re-resolves and uses native endpoint reservation/adoption. No endpoint reset button is added implicitly.

## Launch and directory send

LaunchInput gains optional Role selector and optional workspace (`global` or absolute path), using serde defaults for compatibility. Directory remains independently chosen cwd/default project. Role conflicts with explicit tool or profile at admission; A role's optional profile supplies an explicit base; omission uses ordinary runtime profile defaults. The adopted snapshot always records the actual named profile selected at launch. Without Role, existing tool/profile behavior remains supported; workspace can still be explicit. Resolve native workspace/Role/profile in the isolated once-claimed worker before reservation. The receipt freezes the accepted command and execution resolves the selected configuration once; replay returns the original receipt even after file edits/deletion. Current selected config can differ from an earlier preview and must be reflected in adopted metadata. Do not re-resolve profiles during resume.

POST /api/v1/directory-messages accepts SubmitDirectoryMessage `{request_id,generation,directory,message}`. Command::DirectorySend contains canonical-intent directory string and MessageInput. Execute in the same isolated worker/journal. Reuse native directory resolve → endpoint lookup/adopt/reserve → usable-recipient checks; do not nominate a visible member by name/count. Native reservation arbitrates concurrent creators. After exact participant resolution, submit through existing fenced management Send using the stable operation request ID. Return CommandResult::DirectorySent with `{entry, recipient: AgentRef, sent: SendOutcome, created: bool}`; entry is the resolved target/Role observed for this execution. A lost reply returns the original operation; do not re-run launch/send. A crash after a side effect is Unknown, not retry permission. Freeze operation cwd to the requested directory for env-file resolution; default-role ancestor endpoint membership and launch cwd follow existing native directory send semantics. Working cwd and selected workspace are not interchangeable.

Admission validates UUID/generation, all text/metadata/selector limits, absolute workspace and directory shape, role/tool/profile conflict and message intent; do not perform file reads inside admission. Native worker validates existence and canonical context after claim. Queries/commands use existing same-origin/body/deadline/concurrency controls. No generic CLI/shell input. Existing stop/resume/transcript stay exact; fork remains distinct and is not added by pretending resume creates a fork.

## Usage

The [office usage guide](offices-usage.md) describes browser actions, read paths, native observation states and JSON examples. Actual support is bounded by the linked Delivery and immutable candidate.

## Complete acceptance

One candidate verifies: known empty/nested/explicit marker-free offices and finite coverage; real members independent of cwd and Role origin; current/stopped/same-name/legacy/corrupt states; adopted provenance after Role edit/delete; effective inherited/global/shadowed catalog and explicit errors; pure endpoint inspection unchanged DB/event/binding state including unique/ambiguous/failed/Role drift; Role + independent workspace/cwd launch; directory default parent/global route + actual reply + replay without duplicate; existing exact history/lifecycle; usable browser office/post navigation, deliberate launch/contact, native receipts, draft/selection and narrow viewport continuity. Native-focused cases may seed test DB; public clients/fixtures use native APIs/hooks and never open managed storage. Paid Provider, unrestricted inventories, unified Skills and live service replacement remain outside evidence.
