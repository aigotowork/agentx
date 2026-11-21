---
schema: task
status: completed
agent: nizeda
---

# Agent Space → DancingAI integration

User authorized understanding the existing Canvas system and integrating the default Web entry with DancingAI. This is a direct follow-up, independent of the historical scene-refactor execution assignments. Preserve native services, especially selected LAN :38023, installed binaries and existing drafts.

## Model and implementation

- Existing `SceneRuntime` presents one sample to Canvas and DOM anchors. `SpaceDebugAdapter` observes that presented frame; `sceneDebugRegistry.facade` owns bounded inspection/action/capture contracts.
- `src/scene/debug/dancingai-provider.ts` maps one mounted canvas to one public v1 provider: canvas root, semantic observations, contains/related edges, current client bounds, actual Canvas-only scene PNG, and original action descriptors.
- Only the actual existing frozen public scene facade is exposed through native `getObject` on the canvas root. Semantic records are detached observations, not native actor/furniture instances. No private runtime/controller, new command protocol, rendering loop or business mutation implementation.
- `src/inspection.ts` mounts the same public no-service browser artifact and owns registrations. Registry lifecycle and navigation events update sources without polling; URL scope changes retire old handles. App data, context and native operations keep their original owners.
- The default HTML ships a self-contained public Page browser distribution under `public/tools/dancingai`; provenance and digest are in distribution.json. Normal dev/static/native builds require no sibling checkout. The explicit refresh script consumes only DancingAI's public built artifact; embedded legal notices are retained.

## Verification / next action

Source typecheck, fresh `/space/` static build and actual Chrome discovery/panel/native-action/frame/capture/navigation/lifecycle checks are owned by this direct task. Use explicit demo supply and disposable preview :18975, no real send/launch/stop. Distinguish these from live adoption or paid-Provider evidence. Keep this record current on completion.

## Completed result

- Scoped typecheck passed, including the native description hook. Fresh production build `/tmp/agent-space-dancingai-20261117-nizeda` passed and carries the matching browser artifact SHA256 `ec9d14f000a874bac60f6f69e43084fe3696b9ee26feebd2103310a2c8602423`.
- `node --experimental-transform-types --test web/agent-space/tests/scene-debug.test.mjs`: 9 passed, including provider pre-frame/relations/bounds/capture/removed-object/native-facade boundaries; remaining existing diagnostic cases still pass.
- Actual Chrome on the production static artifact: 61 discovered objects; original native facade identity and invoke parameter metadata; semantic actor native get explicitly unavailable; actor activation through original callback followed by newer frame/input and observed selection; route renews provider and invalidates old handle; real scene PNG; original object panel and native method controls; closing panel preserves toolbar; native toolbar drag preserves application navigation; chat removes sources, scene remount obtains new ID; back/forward renew sources; 390px tool controls. No page exceptions. Test entry `web/agent-space/tests/dancingai.browser.mjs`, browser dependency supplied explicitly through `AGENTX_PLAYWRIGHT_PACKAGE`.
- Static preview remains at `http://192.168.2.178:18976/space/?supply=demo`; initial dev :18975 process ended, so use the static preview for review. Screenshot `/tmp/agent-space-dancingai.png` captures the real object panel's native method metadata.
- Normal default source entry and future native builds contain the integration. Existing installed executables and selected :38023 live daemon were not replaced/restarted. Full Rust packaging, live native mutations and real Provider behavior are outside this bounded frontend integration evidence.

## Follow-up: live observation and empty-box report

Implemented opt-in 200ms sampling in the local DancingAI object panel, reusing provider inspect and its presented-frame revision. Pause retains the displayed sample; selection change/inactive panel stops the timer, hidden Documents skip reads and stale handles stop with an error. No new provider protocol. Cached remote iframe workspaces intentionally do not offer this control. Updated the vendored public distribution and fresh preview candidate `/tmp/agent-space-dancingai-20261117-live` on :18976. Page build passed; scene-debug 9/9 passed; actual Chrome verified changing frame text, frozen paused text, route/panel cleanup and existing navigation/mobile cases. Browser regression source now retains live/pause assertions.

The reported empty white box above the Canvas object remains unresolved: ordinary actor hover produced a factual tooltip with name/role/state; object-list selection produced no empty overlay in the probed scene. Do not claim the white-box cause or a fix from these observations. Earlier progress attribution to the inspection layer was premature. Existing selected live :38023 remains untouched.

## Follow-up: source locations in the existing React overview

User screenshot `~/.tmp/xx3.png` selected a browser React component, not a semantic Canvas provider object. The earlier source-association capability was not adopted by :18976 and the browser React observer did not consume definition metadata. Fixed that consumer path without changing panel layout or styles: optional `objectSources({reactComponents:true,includeInBuild:true})` records named application definitions before minification; browser observer restores source/name from the same realm-shared WeakMap and the existing Page schema preserves `component.source`. Hook names/stable source identities remain explicitly unsupported. Existing overview StudioValues renders the added data.

`web/agent-space/scripts/build-inspection.mjs DANCINGAI_CHECKOUT /tmp/NEW_OUTPUT` builds an isolated source-enabled artifact with the existing WASI bundler setup, typecheck, /space/ base and standard maps. The matching browser artifact was refreshed by the existing vendor script. Candidate `/tmp/agent-space-sources-20261117-magu-r2` built successfully. Actual Chrome discovery on that artifact found 41/41 components with original definition positions; ActorAnchor points to `web/agent-space/uiux/systems/space/blocks/SceneObjectLayer.tsx:12`. Twelve scoped plugin/observer scenarios passed, including imported aliases, wrappers, anonymous function names and preserved uninstrumented limits.

Deployment coordination: nizeda requested ownership of the :18976 symlink for the concurrently authorized unified Runtime/persistent feedback update (#46212). magu did not switch it; nizeda must build the combined candidate using the source-enabled recipe rather than ordinary build, then verify original overview and notify magu. :38023 untouched. Candidate evidence does not claim :18976 adoption.

## Runtime workflow and feedback follow-up adopted on :18976

The same source-enabled candidate now admits the Agent Space provider into React / Runtime Inspection, authoritative hitTest, hover bounds, point selection without business action and existing data/live/pause/material flow. The list is secondary folded discovery. Source mount uses same-origin `/dancingai/` (optional meta endpoint override), so persistent feedback is available when the host provides the backend. The :18976 preview now serves both `/space/` and DancingAI's public Node binding with durable file storage in sibling DancingAI `.local/agent-space/dancingai/`, plus bundled `/dancingai/ui/`. Current candidate `/tmp/agent-space-runtime-feedback-nizeda` was built with `scripts/build-inspection.mjs`, preserving magu's source markers/maps. Adopted LAN Chrome passed point selection, no business activation, live full-field changes/pause, actual feedback batch save/material/readback after reload and 390px entry. Exact evidence and restart example are owned by sibling DancingAI `.controller/tasks/agent-space-runtime-feedback.task.md` and `examples/agent-space-service/`. Selected :38023 remains untouched; the previous empty-box report is still unconfirmed.

Source adoption verified on actual :18976 after nizeda's combined deployment (#46576): Chrome loaded `http://127.0.0.1:18976/space/?supply=demo`, found 41/41 local React definitions with source metadata, opened existing Runtime, selected React from 检查数据来源, activated the original point picker and clicked an office actor. The unchanged overview displayed FactButton with `source.file=web/agent-space/uiux/systems/space/components/FactButton.tsx`, line 11, column 13; missing-source warning replaced by the truthful remaining Hook/stable-identity limit. No uncaught page errors; no business actions or feedback submissions made by this source verification. Screenshot `/tmp/magu-space-sources-18976.png` now captures actual :18976, superseding the earlier candidate-only image. Original :38023 untouched.


### Observation/connection follow-up

Source-enabled :18976 preview now serves `/tmp/agent-space-observation-connections-nizeda` through the existing DancingAI Node listener. Semantic provider promotes activity/animation/state/position; actor sprite frame calculation is shared with rendering; human mailbox observation includes explicitly decorative cat samples and related effect ID. `scene/debug/source-locations.ts` maintains real projection/render/behavior file positions for semantic feedback. Page connection disclosure supports explicit reconnect/space preferences; textual JSON handoff works offline. Scene-debug 10 cases, Agent Space typecheck and source-enabled build passed. Actual Chrome on candidate :18981 and primary :18976 passed selection/live/pause, offline handoff with source metadata, reconnect/preferences, real durable feedback/readback/reload and narrow Runtime. Native :38023 unchanged. Detailed evidence and clipboard sink limits belong to DancingAI `.controller/tasks/agent-space-runtime-feedback.task.md`.
