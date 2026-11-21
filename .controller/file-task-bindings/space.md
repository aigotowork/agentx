---
schema: task
status: completed
description: Show file-owned Task/Delivery associations in the existing Agent Space conversation.
---

# Space binding context

User selected Agent Space only; Inspector is unchanged. Reuse `AgentView.file_bindings`, already joined by native exact scope/generation/incarnation. No new HTTP route, file-body reader, task database, arbitrary-path access or write action is required for this member-oriented slice. A project-wide unassigned-task browser remains outside this task.

Model: native AgentView → existing live projection → optional presentation bindings → conversation-local `BoundWork`. Native status and file status remain separately labelled. Compact collapsed summary preserves reading/composing space; expanding reveals Task/Delivery rows, file-declared state and nested path/raw status/checked/last-success details. Diagnostics mean retained prior success, never current validated state. Empty supplied list differs from unavailable supply. Stopped rows do not acquire current namesake assignments. Refresh cadence remains the native memory projection's actual contract; timestamps are visible and no real-time promise is made.

Skeleton: optional AgentResource.fileBindings uses generated FileBindingView; projection forwards it; conversation owns only composition; BoundWork owns disclosure and localized presentation, with bounded scrolling. No draft/camera/channel state changes.

Verification: scoped TypeScript, fresh isolated build and browser checks of supplied current/stopped/error/empty/unknown states, keyboard disclosure, narrow composer reachability and preserved draft. Source-only checks do not establish live rollout. Preserve existing services and unrelated dirty files.


## Implemented and verified

`src/offline/model.ts` carries optional generated `FileBindingView` facts; `src/live/projection.ts` forwards exact native associations. `src/conversation/BoundWork.tsx` renders the compact disclosure within the existing conversation history, sharing the scene/chat/embedded reader. No native/API/schema changes were needed; no Inspector files were edited. Absent demo/older supply hides the section; supplied empty lists say no association is observed, without claiming exhaustive filesystem coverage. Unknown values and raw states remain visible. Observation timestamps reflect cached native facts, not a promise of continuous file watching.

Scoped `node web/agent-space/scripts/run.mjs typecheck` passed. Fresh actual product build `node web/agent-space/build.mjs /tmp/agentx-space-bindings-build-20261115` passed, including the existing build-required typecheck. [Browser report](evidence/space/browser-report.json) verifies the production bundle with explicitly synthetic API fixtures: file/runtime state separation, retained read-error snapshots, keyboard/path disclosure, empty stopped namesake, new-incarnation unknown/raw states, 390px composer visibility and draft retention, zero mutation requests/page errors. The first browser run passed current/error/narrow checks but its stopped-record driver failed to switch to the existing History tab; [original report](evidence/space/browser-initial.json) is retained. Corrected only the test navigation and affected run passed; no product assertion was weakened.

Visual check of the [390px capture](evidence/space/narrow.png) confirms the existing header, channels, compact work summary, history and composer remain visible together; member identity and the diagnostic count remain local to the conversation. The screenshot captures the collapsed state; expanded semantics are checked by the browser case. This is bounded UI evidence with synthetic supply, not native bind execution or live-service adoption.

Source implementation is complete. The fresh standalone assets remain at the path above. The running embedded daemon has not been rebuilt/replaced by this follow-up; protected services and real Task files/Agents were not mutated. Native binding acceptance and its supported exact-identity semantics remain owned by the original Delivery. A later combined package/adoption must include this source before the live LAN page displays it.

## Selected LAN adoption follow-up

The user asked whether the UI is connected on the actual page. Selected adoption is now authorized. Existing web-context producer tuli owns the active shared r2 build; zomu/libu own the single selected :38023 rollout, and muvu owns affected iframe recovery acceptance. This Task owns binding-only candidate browser checks and read-only live verification. Do not start duplicate packaging or restart over their selected service. Five affected inputs are frozen in [source-freeze](evidence/space/source-freeze.json); candidate inclusion must follow actual embedded asset/source receipts, not package filename/start time. If the active build captured old Space assets, coordinate one necessary combined candidate with the existing producer. Source readiness does not imply deployed UI.

Actual combined r3 build-output snapshot `.bin/agentx-web-context-r3-space` now passes [the same bounded browser cases](evidence/space/combined-r3-browser/report.json), with zero POSTs/page errors. `.bin/agentx-web-context-r3-space.json` records all fourteen asset hashes and positive binding JS/CSS plus repaired popup markers. This establishes asset consumer behavior only, using synthetic API fixtures; reuse is conditional on final executable embedded assets matching the captured hashes. Tuli owns that reconciliation, muvu owns affected context regression, libu owns selected adoption; actual live disclosure check remains pending.

Final combined candidate `.bin/agentx-web-context-r3` is packaged, version 0.7.31, SHA-256 `4f175b5d39ed02852dca6f493a336a5bb1d7f6e596f1e3c826eee7f4adff005a`. Producer [exact receipt](../../.bin/agentx-web-context-r3.receipt.json) establishes fourteen embedded assets equal the tested snapshot and no affected-source drift. Binding candidate acceptance is therefore released without repeating passed checks; actual selected adoption and post-adoption native/DOM verification remain pending the separately owned affected context acceptance. No claim of live UI visibility yet.

## Adopted running outcome

Libu completed the single selected-service adoption of the exact accepted combined r3, SHA-256 `4f175b5d39ed02852dca6f493a336a5bb1d7f6e596f1e3c826eee7f4adff005a`, after muvu's bounded web-context acceptance. [Rollout receipt](../office-details/verification/lounge/rollout-after.json) records PID 1149566 and daemon instance `3532d3fa-70dd-44cc-8d03-c30267a90e1c`; existing scope/generation retained. This supersedes the earlier source-only/not-deployed statements for current running support.

[Actual read-only LAN browser check](evidence/space/live-r3/report.json) passed at `http://192.168.2.178:38023/space/`: native exact current beta incarnation `4745314276086617385`, generation `a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb`, BoundWork disclosure count equals API (zero for this checked Agent), separate runtime label, visible 390px composer, zero mutations/page errors. [Desktop](evidence/space/live-r3/desktop.png) and [narrow](evidence/space/live-r3/narrow.png) captures retain actual rendered state. No fabricated task assignments, real file edits, Agent mutation or message sends were used to produce live evidence. Non-empty/error/unknown binding presentation remains established by the matching immutable asset browser fixtures, not misrepresented as live user tasks.

Task complete including selected adoption. Inspector binding integration remains outside this scope. Maintenance follows generated FileBindingView, native exact binding semantics and the existing conversation composition; no new service or task database is introduced.
