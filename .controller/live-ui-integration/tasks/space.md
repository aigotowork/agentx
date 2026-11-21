---
schema: task
status: completed
description: Integrate existing office scene and chat UI with actual native API.
---

# Space

Read [team entry](../AGENTS.md), [design](../design.md) and [Delivery](../delivery.md). Record scenario-to-native supply, unsupported/redundant concepts, selected design/skeleton, source changes and exact verification here. Implementation is authorized; no separate staged approval is required.

## Scenario / supply / gaps (2026-11-15)

Current source is authoritative; CBM lists only the unrelated xpm checkout, so this investigation uses targeted current-source reads (no graph completeness claim).

| Scenario | Native supply | Consumer / gap decision |
| --- | --- | --- |
| Known empty/nested office, exact/global/legacy/unavailable membership | [office projections](../../../src/management/offices.rs), `WorkspaceCatalog`, `CollaborationView` in [native contract](../../../src/management/contract.rs) | Preserve scene/chat navigation; known directories separate from exact adopted membership; legacy and unavailable visibly distinct; cwd never assigns office. Discovery caps/errors visible. |
| Current and stopped namesakes, selection/drafts | `AgentPage`, `AgentView.agent`, `AgentRef` | Exact key includes scope, generation, name, nullable incarnation, stopped_event and current/stopped kind. Never manufacture incarnation from name. Reset invalidates reads/selection; old exact drafts remain isolated. |
| Effective posts / adopted Role | `GET /api/v1/roles`, adopted `CollaborationView.role` | Current definitions and adopted source separate; no revision, actual profile/model/creation timestamp supplied: unavailable, not invented. |
| Actual communication | `GET agents/:name/messages`, `MessagePage` | Independent message cursor and coverage; show author/recipient/thread/reply/intent; no simulated response or optimistic Agent reply. |
| Provider transcript | Explicit receipt-backed `POST agents/:name/transcript`, `TranscriptPage` | Separate channel; available/empty/unavailable/error distinct. Native bounded exchanges do not supply richer reasoning/tool events. |
| Deliberate launch / send / stop / resume | `SubmitLaunch`, `SubmitMessage`, `SubmitTarget`, `Operation` | Persist exact request before submit; recover by original-ID GET only. Unknown/unconfirmed effects block repetition. Message acceptance is not model completion. |
| Directory contact / inherited entry | `DirectoryEntry`, `SubmitDirectoryMessage` | Pure preview then deliberate send using actual resolved entry; ambiguous/stale/unknown routes visibly unavailable. |
| Skills | No supplied catalog or use facts | Retain honest unavailable resource view; no fixture Skills or authority claims. |

HTTP routes and validation: [native HTTP adapter](../../../src/management/web.rs). Shared binding source: [generated TS](../../../web/management/api/native.ts); shared [transport](../../../web/management/api/client.ts). Integration owner gabi notified of gaps and import/hosting dependency before implementation.

## Structural design / skeleton

Keep `App → existing office navigation + scene/chat + conversation`; keep page-owned `SpaceController` for selection, exact drafts, local notes and presentation and per-office `SceneMemory` in App. Add `src/live/projection.ts` (native facts → existing presentation supply), `src/live/controller.ts` (generation-fenced reads, exact selection records, durable receipt journal and deliberate native commands), and `src/live/Actions.tsx` (compact contextual operation/receipt and directory-contact disclosure). Reuse generated native types and existing transport, without inventing HTTP schemas or importing another management page. Default composition is real same-origin API; fixtures require explicit `?supply=demo`.

Read transition: disconnected → loading → actual snapshot or actionable error; scope/generation change clears selection/read cache and invalidates outstanding results. Office/view/camera changes preserve drafts. Write transition: deliberate action → persist original scope/generation/request/body → POST once → Accepted/Running/Succeeded/Failed/Unknown; missing response → lookup same ID only. Unknown has no replay control. Reload recovers journal and never POSTs automatically. Terminal receipts retain actual result/error. No automatic transcript acquisition or launch/send on selection.

Layout remains existing compact office scene/IM list and anchored reader. Role launch lives in post detail; stop/resume/transcript beside selected Agent; directory contact lives in office-local disclosure; receipt recovery stays in a compact header disclosure. No third page, new server, dependency installation or running-preview changes.

Verification boundary: author may run scoped ordinary typecheck after edits; final tests/build/browser are separately owned. Protected :18967 remains untouched.

## Implemented source handoff

Integration gabi confirmed stable generated imports and same-origin `/api/v1` in message #24533; fresh asset contract `/space/` confirmed in #24603. Implemented [native projection](../../../web/agent-space/src/live/projection.ts), [read/operation owner](../../../web/agent-space/src/live/controller.ts) and [contextual actions](../../../web/agent-space/src/live/Actions.tsx), composed by existing [App](../../../web/agent-space/src/App.tsx), [posts](../../../web/agent-space/src/OfficeResources.tsx) and [conversation](../../../web/agent-space/src/conversation/Conversation.tsx). [Presentation model](../../../web/agent-space/src/offline/model.ts) now preserves nullable native incarnation and stopped_event independently and a distinct legacy membership bucket. Existing SceneSpace camera/geometry files remain unchanged; page controller accepts refreshed supply without recreating drafts or controller. [History](../../../web/agent-space/src/conversation/History.tsx) distinguishes actual native records from explicit demo records and displays intent even without a thread.

Default supply is native; `?supply=demo` is the explicit historical fixture entry. Current office catalog and selected-office effective Role reads remain separate. All-office posts show the global Role catalog rather than pretending that every office has been queried. Loaded-member pagination and bounded discovery coverage are explicit. Failed member reads withdraw stale capabilities. Message reads retain a separate per-selection cursor and actual authors/envelopes. Provider records require explicit acquisition and retain unavailable/empty/error separately; native exchange action/file text is not fabricated into richer tool or reasoning events.

All actual operations persist original scope/generation/request/body before the first POST. Reload/poll perform GET recovery only; Unknown and unconfirmed effects retain their identity and block another submission to the same subject. No repeat/replay button exists. Receipt detail shows native status/result/error and exact recovery identity. Stop has a confirmation on the selected exact instance. Role launch is in post detail; directory contact previews native resolution and requires deliberate send to a resolvable bound/adoptable/absent entry. Messages can carry intent/thread/reply reference. Draft text is retained after submission and across view/office switches; success is visible in native receipts and subsequent actual message reads, without synthetic replies.

[Fresh build entry](../../../web/agent-space/build.mjs) accepts a nonexistent absolute external directory, invokes the existing scoped runner, sets `/space/` base and preserves existing public notices/licenses. Existing dev port/config defaults remain unchanged. [Module entry](../../../web/agent-space/AGENTS.md) and [README](../../../web/agent-space/README.md) route the actual default, demo entry, native ownership and hosting boundary.

### Verification actually executed

`node web/agent-space/scripts/run.mjs typecheck` passed once after source edits on 2026-11-15. This is ordinary static evidence only. No Cargo, dependency installation, final build/test/browser execution, daemon restart, server startup or protected :18967 operation occurred. No native/shared generated files were modified.

### Independent integration/acceptance handoff

Build through the fresh entry, embed under `/space/`, then verify default real scope reads and disposable effects. Target cases: known empty/nested offices; exact/global/legacy/unavailable membership; same-name current/stopped selection and drafts; adopted/current Role source differences; loaded paging/discovery caps; real sent and received envelopes; explicit available/empty/unavailable/failed Provider exchange reads; native launch/stop/resume; inherited directory preview/contact; dropped POST response and reload recovery; Unknown preventing repeated effects; reset during pending reads; and existing scene/chat/office/short-screen camera continuity. Historical fixture browser tests must explicitly use `?supply=demo`; their old default offline assumption does not apply to the live default. Existing pure fixture tests remain historical/offline evidence. User-authorized actual paid Provider outcomes must not be inferred from fixture mutation evidence.

Source implementation is ready for independently owned final verification; this record does not claim runtime or browser acceptance. Gaps remain intentionally unavailable rather than new schema requests: Skills, actual observed model/profile, creation timestamp, Role revision, richer Provider tool/reasoning events, complete arbitrary history and complete filesystem discovery beyond native limits.


## Integrated acceptance closure

Contribution accepted in the [completed Delivery](../delivery.md#accepted-local-outcome-and-running-lan-service). That entry links exact native, independent API/browser and final LAN adoption evidence and their separate limits. Earlier source-only handoff statements remain historical; maintenance follows the native declarations and module guides, without reviving this delivery team.
