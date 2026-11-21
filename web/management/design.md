---
scope: Agent management Web console
status: implementation
owner: management-web-fava
related: [../../.docs/agent-management/design.md, ../../.controller/agent-management/tasks/web.md]
last_updated: 2026-11-15
---

# UX model

The operator selects an exact live execution or stopped snapshot, sends work, follows messages, opens a transcript snapshot, then stops or resumes from that same resource. Agent execution state, operation receipt state and task completion remain separate. The generated public SDK supplies native types and lossless codecs; identities and cursors remain exact strings.

The [current redesign Task](../../.controller/management-ui-craft/task.md) records the implemented structure and evidence. Agents, Workspaces and Operations are independent destinations below the persistent listener scope. Agents keeps the index next to exact subject and conversation. Workspaces combines a known path index, effective roles, loaded members and directory route preview. Operations presents a local journal index and an exact selected receipt; the originating page shows a concise receipt before navigation. On mobile, the Agent index is a picker and the composer remains with the selected conversation. Secondary facts use local disclosure; launch and stop use focused dialogs. The earlier [layout sketch](ux.svg) is historical.

| Action | Input owner | Result / recovery |
| --- | --- | --- |
| Launch | Descriptor tools/profiles, directory, prompt | POST 202 receipt; poll operation; list refresh discovers Agent |
| Send | Exact reference, per-reference draft, intent/reply/thread | Clear an unchanged draft after the original operation succeeds, including recovery after reload. Actual communication is shown separately in messages. |
| Stop / resume | Capability on exact selected reference | Stop requires local consequence confirmation; poll receipt and refresh; never target a reused name implicitly |
| Transcript | Exact reference async job | Receipt → bounded exchanges / unavailable reason / empty / failed; retain snapshot by reference |
| Lost reply / reload | Durable browser record of original path, ID, generation and body, written before POST | GET operation first; explicit replay only after NotFound, using identical body and ID; Unknown is never replayed |
| Database reset | Descriptor generation | Refresh read scope; preserve old operation receipts with original generation; no cross-generation replay |

## Structure and contracts

`api/client.ts` selects the named public SDK and listener namespace. `model.ts` owns browser recovery records and exact identity helpers. `use-management.ts` owns bounded read polling, exact selection and durable mutation lifecycle. See the [module guide](AGENTS.md) for presentation and build ownership.

## Verification cases and limits

Primary cases: live/stopped selection, empty list, separate exact histories sharing a name, paginated list/messages, draft surviving polling/selection, async transcript completion, capability-disabled actions, lost POST reply and reload recovery, not-found receipt same-ID replay, Unknown no replay, generation reset, network read errors, narrow-screen continuity. Frontend build/typecheck verify static contracts; fixture browser checks are separate from native integration and real Provider support. Integration owner validates actual HTTP/worker behavior.

## Feedback

The generated header initially mentioned bigint/markers; the integration owner corrected it to ordinary JSON and string identities. No generated file was edited by the Web worker. A socket-destroying HTTP fixture also exposed Chromium transport-level retries with identical POST body/ID; the fixture deduplicates receipts like the native journal, and the assertion checks no new operation identity is created. Native integration and real Provider evidence belong to the independent validation owner.

## Earlier frontend evidence

`node web/management/build.mjs /tmp/agentx-management-web-fava-final-20261115` passed strict TypeScript and Vite production build, generating the finite native-compatible manifest plus hashed JS/CSS. `node web/management/tests/browser.mjs /tmp/agentx-management-web-fava-final-20261115` passed fixture scenarios for exact live/stopped string references beyond JavaScript integer precision, per-reference drafts, async transcript results, lost reply lookup without a new operation identity, reload plus identical-body/ID replay only after missing lookup, Unknown replay protection, UUID generation on actual insecure HTTP, mobile selection continuity and stopped resume. The fixture uses its own ephemeral port and shuts down its browser/server. It does not contact a daemon or Provider.

Rendered inspection used 1280×800 desktop and 390×844 mobile screenshots at that output directory (`desktop.png`, `mobile.png`). Desktop shows both resource identities/status/tool/directory, selected exact subject, message sender/recipients/intent/time and persistent composer together; selection, scope filters, launch, stop, transcript and operations are immediately reachable. Mobile locally discloses the same resource collection while retaining the selected subject, tabs, conversation and composer in one viewport; the test checked no document horizontal overflow. Operation recovery remains a local panel. These observations establish the fixture layout and interaction scope only. Stop outcome `sent` is displayed as a stop request awaiting actual status update, with no optimistic execution change.

## Holistic office extension — layout and skeleton

This section records the earlier office implementation. For current navigation and verification, use the [redesign Task](../../.controller/management-ui-craft/task.md). Workspace catalog selection and the Agent membership filter now have separate owners.

Source: [office contract/design](../../.docs/agent-management/offices.md), [owned Task](../../.controller/office-management/tasks/web.md), generated `api/native.ts`. Offices are exact saved memberships, independent of cwd; adopted Role provenance is separate from today's effective Role catalog. Global, legacy and unavailable are distinct selectors. Counts describe loaded Agent rows only; discovery coverage has its own bounded disclosure.

Desktop retains the resource rail and selected Agent/message/composer workspace. The rail gains a compact office selector above existing search/live/stopped controls. A sibling “办公室 / 岗位” tab opens an in-place scrollable office work surface: office identity + discovery coverage → effective Role rows with declaring namespace/profile/default and local launch → independent directory route preview → deliberate directory message. Agent selection and message draft remain mounted while browsing offices. Mobile reuses the expandable picker and the same office tab; forms scroll inside the workspace, not the document. Role provenance is disclosed alongside selected Agent exact identity.

| Module / skeleton | Public boundary / ownership |
| --- | --- |
| `use-offices.ts` | `useOffices(generation, refreshVersion)` owns cancellable generation-fenced workspace/catalog queries, explicit workspace selector and refresh; no mutation authority |
| `offices.tsx` | `OfficeWorkspace` renders coverage, effective Role rows, directory preview and directory-send form; `MembershipFacts` presents saved collaboration with no cwd inference |
| `resources.tsx` | Existing rail applies exact membership filter, preserves exact Agent selection; toolbar shows adopted facts |
| `launch.tsx` | Existing durable draft adds independent workspace and optional native Role selector (NAME/local:NAME/global:NAME); Role excludes tool/profile; contextual prefill is deliberate |
| `use-management.ts`, `model.ts` | Add directory-send to the existing receipt admission/recovery union; identical persisted body/ID and Unknown protections stay shared |
| `operations.tsx` | Native DirectorySent receipt shows execution-resolved directory/Role/workspace, exact recipient, created flag and message ID |

State contracts: catalog changes clear stale effective roles before new reads; generation changes invalidate route previews; preview is GET-only and never permits implicit send. Directory-send has its own editable draft, byte limit and explicit action; execution may resolve differently and its receipt owns actual outcome. Send admission persists the full directory/body before POST through the existing journal. Catalog/read errors remain local and do not disrupt existing lifecycle or message polling. No backend/generated-contract edits, no agent-space/LAN changes.

Verification plan: one strict production build followed by the existing isolated browser fixture extended with office coverage/membership filtering, effective Role launch with independent cwd/workspace, pure directory preview, directory-send native receipt and identical-body recovery. Existing exact identity/draft/transcript/Unknown/mobile/resume fixture remains in that same run. Native execution evidence belongs to integration.

### Office source result and bounded evidence

Implemented the planned office modules and existing-console wiring. Effective Role reads retain the requested query key separately from canonical returned workspace, so symlink/canonical path resolution cannot strand the catalog in loading. The rail offers discovered empty offices and explicit saved membership paths, with loaded-member counts and separate legacy/unavailable groups. Adopted facts include the original Role path/namespace/profile/default/description. Route preview shows requested versus default-declaring workspace and exact endpoint identity; execution receipts show actual native directory/Role, exact recipient, created/reused and sent message ID. Directory contact uses the same durable receipt union, lookup, generation check and explicit original-ID replay as lifecycle/message operations.

`node web/management/build.mjs /tmp/agentx-management-office-novu-20261115` passed strict TypeScript and Vite production build. The single initial `tests/browser.mjs` run exercised the existing exact-reference/draft/transcript/lost-reply/Unknown/mobile/stopped-resume path, then stopped at the newly added close-after-resume fixture step: durable local receipt creation precedes async drawer mounting. The fixture now waits for the existing close control before clicking; no product change was needed. Office fixture cases are authored, but not established by that interrupted run. The one-check constraint was reported to integration before any retry. Independent native/browser acceptance remains with its assigned owner; no Cargo, generated types, agent-space or LAN changes were made by this worker.

Integration authorized one failure-driven fixture-only retry after the wait correction (AgentX #23327). `node web/management/tests/browser.mjs /tmp/agentx-management-office-novu-20261115` then passed against the identical built assets: membership independent of cwd; known empty office; bounded coverage; exact Role/workspace launch with independent cwd; GET-only route preview; DirectorySent receipt; directory reload recovery with identical body/ID; office draft continuity, local catalog error and mobile overflow; plus the existing exact references/drafts/transcript/lost-reply/Unknown/HTTP UUID/mobile/resume cases. The strict build was not repeated. The first fixture failure above remains part of the evidence. Product source is frozen for integration and separate native browser acceptance.

### Native Role selector correction

Integration source review (#23389) identified that `roles::resolve_role` accepts a Role name with optional local/global qualifier, not its source path. Contextual launch now submits the effective `role.name` plus the selected canonical catalog workspace, allowing native execution to resolve the current definition. Original paths remain visible as provenance only. The launch placeholder now documents valid selector grammar, and the fixture checks both the prefilled name and submitted name/workspace while rejecting absolute paths as selectors. Earlier fixture evidence above tested the frontend transport with an incorrect path assumption; the following verification supersedes that launch claim.

The affected strict TypeScript/Vite build and complete console fixture passed with corrected name selectors on `/tmp/agentx-management-office-novu-selector-20261115` (authorized by #23389). Fixture launch admission now rejects source paths, asserts contextual `role.name` and selected catalog workspace, and retains independent cwd assertions. Other office and existing console baseline cases passed in the same run. Source is frozen again; native selector execution and adopted provenance remain independently checked by zilu.

## Scalar labels — management consumer

Owner: [labels Web Task](../../.controller/agent-labels/tasks/web.task.md); semantics: [classification model](../../.docs/agent-classification.md); actual types: `api/native.ts` (surfaces-owned generated declarations). `labels/controls.tsx` owns rail selector drafts and loaded-page grouping controls; `labels/model.ts` derives partial edits and grouping; `labels/editor.tsx` owns selected-target editing. Existing `resources.tsx` composes these blocks and existing `use-management.ts`/`model.ts` own read and durable operation lifecycles. No new state framework or alternate journal.

Label selection submits repeated native `label` query values with `current_only=true` and resets pagination before reading; it never filters a fetched page and claims completeness. Existing office/search/status filters retain their loaded-row meaning. Grouping by one key is explicitly “仅已加载”; scalar values, missing key and unrecorded historical/service data form distinct groups. It does not request or infer a full count.

The toolbar disclosure shows sorted scalar text safely and supports several editable rows. Drafts persist by generation plus exact AgentRef. Editing freezes a revision baseline; saving sends changed set/remove keys only with `if_revision`, preserving unmentioned plugin keys. Polling does not overwrite draft input. Revision change/failed receipt retains the draft and requires explicit “刷新并保留草稿”; this rereads native LabelsSnapshot and reapplies only the user's partial intent to fresh facts, then requires review/save. Native validation remains authoritative; local duplicate/blank checks avoid ambiguous rows.

The existing journal saves the exact labels path/body/ID/target before POST, and associates that ID with the durable label draft before network admission. Reload recovers via the same operation lookup and explicit original-ID replay after NotFound. Pending/Unknown outcomes disable fresh save, discard and rebase to prevent accidental new-ID overwrite. No automatic mutation retry; success follows Succeeded receipt only. Missing `Descriptor.supports_labels` displays “当前服务不支持标签” and disables label writes/selection. Unrecorded historical labels are not shown as empty and remain read-only. Clearing an applied selector remains available if a service loses labels support.

Frontend checks and preview use `tests/labels.fixture.mjs` synthetic HTTP state against the actual production console; they cannot establish native persistence or independent acceptance. Run instructions and executed evidence live in the Web Task. Actual candidate/browser acceptance is owned by [acceptance](../../.controller/agent-labels/tasks/acceptance.task.md).


Definite label admission refusal (native InvalidInput/Forbidden/NotFound/Conflict/StaleReference/Unsupported) is retained as browser `Receipt.rejected`, separate from native operation state. It is terminal for pending polling/replay and keeps the draft available for correction or explicit refresh/rebase. Transport failure, timeout, OutcomeUnknown and server errors continue through unresolved original-ID recovery. The synthetic fixture uses actual HTTP409 Conflict to protect this distinction.


Expanded labels keep toolbar height bounded by `--label-toolbar-max`, preserve its intrinsic action region with `flex-shrink:0`, and allow local vertical scrolling. This retains both selected-subject context and reachable editor actions at narrow sizes while leaving conversation/tabs in the existing workspace. The synthetic browser check scrolls the action region into view and verifies its bounds inside the toolbar.
