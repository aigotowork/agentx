---
schema: task
status: completed
description: Supply checked adopted collaboration, pure endpoint policy and bounded host office/catalog/preview queries.
---

# Native policy and host reads

Read team/design and native agents/config guidance. Own src/agents/workspaces.rs, src/agents/launch_service.rs, src/agents/launch_store.rs only for additive EndpointInspect, plus new src/management/offices.rs and its tests. Supply checked collaboration decoding with explicit Missing/Valid/Unavailable preserving old helper behavior, and a pure endpoint inspection that never binds/clears/launches/sends. Share compatibility/retirement predicates rather than duplicating policy. Native inspection must preserve role drift/stale/unknown and failed/pending occupancy. Host offices reads compose the fixed public management contracts through Client, current role resolver and bounded filesystem traversal outside SQLite dispatch. No Cargo/build or management contract/store/web writes. Native tests may be authored here but executed only by integration.

Integration-facing signatures in management/offices.rs (publish skeleton first): `collaboration(context: Option<&str>) -> CollaborationView`; `collaboration_value(snapshot: &serde_json::Value) -> CollaborationView` handles stopped launch_context string/object; `catalog(client: &Client, project: &Path) -> ServiceResult<WorkspaceCatalog>`; `roles(client: &Client, workspace: &str) -> ServiceResult<RoleCatalog>`; `entry(client: &Client, directory: &str) -> ServiceResult<DirectoryEntry>`. Native contract types are already frozen in contract.rs. Inform gabi of exact EndpointInspect response types for execution-owner consumption. Never change existing file-binding additions in the shared contract.


## Native working model and cases

The existing native endpoint binding is authority; filesystem roles are current definitions; launch collaboration is frozen provenance. Host queries compose those sources without granting authority. Skeleton was published to integration/runtime before behavior. Inspection shares native `compatible` and `retired` predicates and bypasses write scopes. Checked projection is additive; the legacy `collaboration(row)` helper remains unchanged.

| Authored case | Required result/invariant |
| --- | --- |
| Missing/context corrupt/unsupported/relative membership or source | Missing is legacy; invalid is unavailable; no raw prompt/context leaks |
| Current and stopped object/string context | Same adopted metadata, independent workspace/Role, no current-file reads |
| Absent/unique/ambiguous endpoint | absent/adoptable/ambiguous, never adopted by inspection |
| Pending/failed bound participant | Remains bound; no readiness fabrication |
| Bound disappeared/namesake/role drift | unknown/stale/role_changed; exact incarnation retained |
| Exact retirement then unique candidate | adoptable; old binding unchanged |
| Every inspection fixture | DB total_changes, event frontier and endpoint KV unchanged |
| Empty/nested markers, excluded dirs, symlinks, depth/row/directory caps | Finite discovery and truthful coverage; no implicit cwd office |
| Native effective roles/inherited defaults/catalog errors | Native shadowing and origins preserved; errors/overflow are whole query |

Execution belongs to integration; no Cargo/build runs in this Task. Authored tests are source coverage until integration reports receipts.


Role catalog ambiguity resolved with integration: “invalid catalog” uses native `list_roles(false)` validation over effective definitions; invalid shadowed definitions stay shadowed, matching the existing resolver. No stricter parallel catalog algorithm is introduced.


Finite scan follow-up from integration: cap total enumerated directory entries at 8192 across traversal as well as 512 directories/depth 3/100 workspaces. The entry cap sets existing `scan_limit_reached` and exits traversal promptly; authored flat-directory fixture protects this bound.


## Source-ready handoff

All five integration-facing office functions and additive EndpointInspect transport/policy are implemented. Added 14 authored tests: five native checked-decoder/endpoint tests and nine host projection/discovery/role tests. `rustfmt --edition 2024 --emit stdout` parsed all four owned Rust files successfully on 2026-11-15 without modifying sources. No Cargo, build, daemon acquisition/restart, HTTP execution, or test execution was performed. Integration owns type checking and test receipts; host Client/IPC composition remains pending that execution. Shared management contracts, store, web, file bindings, installed binaries and live services were not edited.

Integration executed all authored native cases in [37-case combined receipt](../verification/native-office-r1-receipt.json), passing. Native contribution is complete; public HTTP/browser composition remains independent acceptance-owned.

## Independent stop-race regression

Following the [office-04 investigation](integration.md#stop-completion-race-office-04), `runtime::hosts_store::acceptance_tests::late_process_registration_cannot_replace_issued_kill_identity` models an existing runner identity, native PrepareKill and BeginKill, then attempts a late registration using a different live Provider PID. It requires StaleFence and unchanged PID, process identity, killed marker, launch context and event frontier/count, then successful CompleteKill and identical replay without duplicate events. Existing ordinary registration coverage remains. The product guard is integration-owned; this authored case has not been executed here, and integration owns the affected Cargo run.
