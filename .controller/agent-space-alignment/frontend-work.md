---
schema: task-work
status: executor-verified
owner: kazu
links: [./implementation.md, ../../.docs/agent-space/concept-alignment.md, ../../web/agent-space/AGENTS.md]
---

# Slice 1 frontend work

## Accepted ownership and constraints

Kazu owns the complete offline semantic adaptation and ordinary repairs. No delegation, native/API/root changes, real Agent actions or preview restart. Preserve user LAN :18967, art/provenance, camera and short-screen behavior. Final build/browser/state execution belongs to the independently bound executor; implementation may run one scoped development typecheck.

## Structural design before implementation

The accepted concept design owns domain meaning. This frontend consumes explicit synthetic facts, never discovers directories or resolves Roles itself. `offline/model.ts` owns presentation-local supplied office/post/member/read declarations. `fixture.ts` supplies canonical example paths, exact scope/generation/incarnation keys and provenance. `controller.ts` owns one page-lifetime selection, office view, exact-key drafts/notes and reader channel. Pure projection functions own exact membership, current/stopped separation and adopted-source post matching. App composes existing scene/chat layouts with compact office navigation and a local resource panel. The reader separates AgentX message envelopes, Provider model records and local notes. The scene projects all known rooms including empty offices, a single global area and a separate unknown-membership area; source art/geometry remain reused.

Layout: existing header and compact context rows, then a dedicated workspace layout with a directory navigator beside the working scene/roster on desktop; native details disclosure collapses navigation on narrow screens. Members remain primary. Office-local members/posts/Skills/history switch locally; posts expose origin/profile and real matching members, never fake people or launch. Exact member details disclose adopted Role/profile separately from observed model/cwd. Per-office scene-memory slots preserve camera/return framing through resources/view changes; selecting a person retains the current scale. A recent exact selection remains recoverable after filtering or office changes. Reader tabs are exact-key state and never append/send. Local notes have a distinct tab and composer.

## Cases designed before implementation

| ID | Starting facts / action | Required result and owning layer |
| --- | --- | --- |
| S1 offices | Open default overview, then atlas/web/empty/global/unknown | Each known canonical office visible once, empty office has no sprite, parent excludes nested members, global once, unknown never global; projection + browser. |
| S2 posts | Open web posts: two inherited fronter users, global reviewer, vacant controller; inspect adopted/current facts | Two named identities; origin/profile paths; zero-use post has no person; adopted snapshot remains separate; supply/projection + browser. |
| S3 membership | Local reviewer has global Role; atlas member cwd is web; global member cwd is web | Membership alone decides exact counts and rooms; model + browser details. |
| S4 unknown | No-Role member, missing Role and missing membership; Skills tab | Confirmed none differs from unknown; Skills unavailable rather than empty/used; model + browser. |
| S5 exact history | Write current @mora draft, open stopped namesake and write different draft, return | Scope/generation/incarnation/current-stopped keys separate drafts/notes/history; inactive remains current; controller + browser. |
| S6 readers | Open messages/model/notes; inspect two authored directions, missing/empty/error model branches | Sender/recipient/thread/reply distinct from Provider role/session, notes never create reply/delivery/state changes; controller + browser. |
| S7 continuity | Select at manual zoom, close, scene/chat, office A/B/A, resources, resize, panorama | No select zoom; overview/office return and exact draft retained, no implicit append; camera/browser. |
| S8 viewport | Desktop, 390×844, 320×640 and 320/390×480, reader tabs and expanded reader | Navigation accessible, local scroll, composer reachable, page has no horizontal overflow; browser. |

## Source evidence and friction

The available codebase-memory index belongs only to `/home/long/works/longwork/xpm`; no graph covers this checkout. Use targeted current-source inspection (module entry, App/controller/scene/reader and existing camera cases), without indexing a broad unrelated tree. CLI `ack` requires a received message ID; initial acceptance was sent as `inform` after the normal-chat assignment supplied no message ID. No product requirement is blocked.

## Current state

Design/cases recorded; implementation begins. Final execution not yet requested or run.

## Executor-ready source handoff

Slice-1 source, module entry/README, current design status and runnable cases are ready for independent execution. Composition is `App` → `SpaceLayout` + `OfficeNavigation`/`OfficeResources` + existing `SceneSpace`/`ChatLayout` → one `SpaceController` and replaceable fixture supply. `offline/projection.ts` owns membership and record-kind filtering; post membership matches the adopted source. Scene groups include supplied empty offices and explicit global/unknown buckets, while stopped records are cabinets/history and do not create current people. Camera memory is keyed by office at the App boundary.

Exact browser targets and expected checks are bound in [`tests/browser-acceptance.mjs`](../../web/agent-space/tests/browser-acceptance.mjs) (S1–S8), [`tests/semantic-state.mjs`](../../web/agent-space/tests/semantic-state.mjs) and [`tests/selection-camera.mjs`](../../web/agent-space/tests/selection-camera.mjs), with invocation and evidence instructions in [tests README](../../web/agent-space/tests/README.md). Use the scope selector `工作区范围` with `/projects/atlas/web`; local tabs are `成员`, `岗位`, `技能`, `历史`; the two inherited-post people are `@luna` and current `@mora`, with the stopped `@mora` under history. Reader tabs are `消息`, `模型记录`, `本页笔记`; `会话详情` discloses exact reference, membership, cwd, adopted Role/profile and model. `riku` supplies the model-read error, `sira` the empty model record, and `mora` unavailable model data. Select `fronter` in web posts to inspect both members and definition/snapshot differences; `controller` demonstrates a vacant post. The separate unknown and global scopes are `$unknown` and `$global`.

Verification performed by kazu: `node web/agent-space/scripts/run.mjs typecheck` passed once after coherent source edits. The state, build, browser, screenshots and camera scenarios are **authored but not executed**; an independently bound executor owns them against a fresh static artifact, including desktop/390/320/short visual assessment. Existing :18967 Vite stays untouched. Earlier tests/evidence belong to the prior transplant and were updated/referenced as historical. Real API/native supply, live Agent actions and durable delivery remain deferred. Kazu retains ordinary repair ownership until executor findings are resolved; no source writes released yet.

## Independent execution binding

Zumi accepted the source handoff and bound runtime-ready executor `pura` (batch `ff113645`) through the controller-owned `verification.md` to S1–S8, semantic state, scoped build, browser/camera and short-screen execution. Kazu retains source repairs and will coordinate substantive findings directly. Final acceptance/evidence remain executor/controller owned; LAN :18967 and native/API boundaries remain preserved.

## Ordinary repair: S1 layout relocation

Pura's initial fresh-static desktop visual assessment found current web members briefly drawn in other offices despite correct supplied membership and room grouping (request #22167; screenshot `tests/evidence/semantic-alignment/browser-initial/desktop-offices.png`). Source cause: `HallScene` initialized motion positions from the pre-measure projection, then treated newly measured home/rest coordinates as walking destinations, animating across office boundaries. The same mechanism could recur on responsive layout changes.

Repair in `HallScene.tsx`: each motion entry now retains its home/rest seat geometry. A changed geometry initializes directly at the newly projected goal; unchanged seats still allow normal state-driven work/rest motion. This leaves exact identity, selection, camera/return, fixture, controller and records unchanged. Focused required evidence: fresh desktop initial S1 room/person positions before animation could settle, plus responsive layout relocation at narrow width; existing camera and short-screen cases need only affected renderer regression confirmation. No broad state rerun is warranted because pure state/supply owners are unchanged. Independent executor owns any regression binding and repaired-artifact build/typecheck/execution; kazu has not repeated the prior passed typecheck.


## Final source handoff and write release

Pura reported final repaired-artifact success in message #22301: affected S1/S7/S8 and no-runtime checks (5), all 30 camera captures/assertions at 1440/390/320 including 480px short traces, and direct seat-placement assertions for all eight current actors at first desktop render, resize390, resize320×480 and desktop return. Repaired visual assessment places web members in web and leaves the empty office empty. Initial independent results already establish 7/7 semantic state cases, scoped static build/typecheck and all 9 S1–S8 browser checks; only invalidated renderer checks were repeated after the HallScene repair. See the executor-owned [verification record](verification.md) and its [retained evidence](../../web/agent-space/tests/evidence/semantic-alignment/) for exact commands, identity, initial failures, tooling/test binding corrections and final evidence. This record summarizes reported independent results; kazu did not execute or independently duplicate them.

No substantive findings remain according to the executor. Slice-1 implementation and ordinary repair contribution are complete; kazu releases frontend/design-status writes to zumi at this final handoff. Controller acceptance and executor resource cleanup/final record closure remain their respective owners. User LAN :18967 was preserved; no native/API/root/dependency changes or real Agent actions were introduced. Real authoritative data, native actions, durable message delivery and physical mobile keyboard behavior remain outside this slice.
