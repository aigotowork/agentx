---
schema: task
status: completed
description: Readable playback pacing and synchronized relationship, trace-flow and sequence views using the reusable viewer.
links: [../runtime-atlas.md, ./runtime-atlas-tool.md, ../../../.docs/architecture/runtime-atlas/README.md]
---

# Runtime atlas pacing and views

## User need and design

The user finds the latest motion useful but too fast and asks to compare complementary diagrams over one data model. Current facts, source-grounded lifecycle semantics and illustrative timing remain. Reuse the same scenario, completed checkpoint, in-flight transition and selected object across views; a view switch must not restart or apply an event. Default pacing becomes 0.5× with 0.25×/0.5×/1×/2× controls. Rate changes affect subsequent elapsed presentation time, never canonical states or packet position at the instant of change. Paused transitions remain paused; all phases and autoplay holds scale together. Expose the current beat's ordinal for orientation. Browser backgrounding still pauses.

| View | User question | Projection |
| --- | --- | --- |
| Relationships | What exists and how is it connected? | Existing process/module/channel topology and current visible snapshot. |
| Flow | What are the stages in this scenario and where am I? | Ordered scenario steps with actor→handler and fact-change counts, selectable step checkpoints and in-flight highlight; this is one authored trace, not invented branches. |
| Sequence | Who interacts with whom, in which order? | Participant lifelines and ordered authored beats; reverse arrows, local work, disconnects and retention use distinct symbols. Current beat follows the same playback clock. |

Full-width map and compact event/fact dock stay primary; view tabs and speed fit existing controls. Long descriptions stay in the local inspector. Selection, keyboard/touch, reduced motion and offline operation remain. Flow selection deliberately starts that selected event; view switching does not. Hash adds `view` and `speed` as presentation values, invalid values fall back safely. Hash checkpoints still represent applied steps, not transient timing.

## Structure and ownership

Controller owns this Task, `tools/flow-atlas/viewer/**`, runtime model/builder/docs, tests and actual integration/browser execution. Existing presentation leaves are moved (single source, no copies) into the tool's viewer. New alternate diagram projections belong in `viewer/views.js`; they receive the shared state and never own timers or fold a competing scenario. The specialist owns only the files named in [tool Task](runtime-atlas-tool.md). Model adds fact definitions/default selection and moves existing topology coordinates/routes out of viewer code as optional layout hints. Generic fallback layout must display a different system without code edits.

## Cases and limits

Verify normal-motion marker speed ratios and rate continuity, pause/resume, slower default, flow navigation, sequence direction/local/disconnect, switching all three views during a paused/running beat, cancellation, custom-fact unrelated example from unrelated cwd, rejected generic references, source-grounded runtime invariants, offline/no-network and 1366×768/390×844 geometry. Preserve earlier artifacts and failures at `/tmp/agentx-atlas-multiview-8u8mqhdv`. No product execution, dependencies, user `.agentx`, `.bin`, installed/default programs or services are part of this work; existing atlas listener18957 reads generated HTML in place.

## Working record

Current defect: timing, five fact names, default appserver IDs and layout are embedded in the UI. Selected remedy extracts only the finite trace model, pure fold/diff helpers and offline builder while reusing the actual viewer. Defer editors, automatic architecture discovery, live telemetry, graph engines, workflow execution and all-possible-branch modeling. One specialist provides genuine model/tool isolation and parallelism with UI integration. Paseo binding: `{serverId:srv_yDbm9zuqIMiM,agentId:21f5f935-f2da-40ae-bce4-302e2545315c}` using freshly inspected default profile. Controller independently exercises the specialist's core and generated example.

## Accepted pacing, views and reusable outcome

Completed and accepted within the existing explanatory Delivery. The current runtime HTML is189168bytes, SHA256 `e08e54e03ac011a7920a276747b17d8739e10f2593fb46ffd0a0ef79202b2dbf`; [build receipt](../../../.docs/architecture/runtime-atlas/build-receipt.json) owns all10 exact source inputs. The existing http://192.168.2.178:18957/ listener serves those same bytes without restart. Its recorded PID/start ticks/argv are unchanged. Canonical evidence is `/tmp/agentx-atlas-multiview-8u8mqhdv/final-handoff.json`.

- Relationship, flow and sequence projections use the same trace/transition and selection. Flow shows the selected full scenario with clickable events; sequence unfolds the declared beats and direction. Switching views mid-flow does not cancel, complete or restart the event. Flow uses three/two/one columns according to available width; narrow one-column chains fit without horizontal clipping. Sequence headers stay visible during internal vertical scrolling and current-beat following keeps active participant names readable.
- Default0.5×, selectable0.25×/0.5×/1×/2×, scales the existing illustrative clock. Actual normal-motion Chrome147 frames confirm the4× ratio between0.5× and2×. Paused rate changes preserve exact packet coordinates; paused/running view switches preserve progress and pre-application facts; resume reaches the same patch. Timing and traces make no live-execution claims.
- The reusable [Flow Atlas](../../../tools/flow-atlas/README.md) is an actual offline tool with generic validation, pure fold/diff and the single viewer source. AgentX-only defaults/fact names/topology moved to model data; local source/provenance checks stay in the runtime wrapper. [Unrelated generated example](../../../tools/flow-atlas/examples/request.html) uses actor/service/store,3 custom facts, no source/layout metadata, the same renderer and all three projections.

### Executed checks and feedback disposition

`python3 tests/runtime_atlas_model.py` passed current reference/source/lifecycle counterexamples. Exact comparison against preserved prior model confirms original16 nodes/23 edges/49 steps/128 beats unchanged; only fact/default/layout metadata was added. `python3 tests/flow_atlas_model.py` passed isolated normalization, invalid references/IDs/facts/beat directions, unrelated-cwd CLI generation, model text containing script/template syntax, and rejected-input output preservation. `tests/flow_atlas_core.mjs` passed explicit catalog checkpoints, partial and tone-only patches, independent snapshots, invalid indexes and runtime disconnect/headless/mail boundaries.

Actual `tests/runtime_atlas_multiview_browser.mjs` passed speed-frame sampling, pause/rate/view continuity, flow keyboard navigation, reverse/disconnect and cancellation, projections of all14 traces, offline unrelated example, fallback hash, custom facts, touch/reduced motion and1366×768/390×844 layouts. Results and screenshots: `browser-r1/result.json`. The first image inspection found a substantive narrow horizontal-flow defect; the responsive vertical flow, sequence follow-centering and actual-kind legend correction then passed only their affected cases in `browser-layout-final/result.json`. Earlier motion/state checks are reused because those final changes do not alter the clock/reducer. The original complete14-scenario step walkthrough was not repeated.

Retained driver-only issue: the old runtime semantic test expected the phrase “declared edge”, while the generic validator correctly rejected the same invalid beat with its exact field path and offending edge. The assertion now checks that contextual rejection; `/tmp/agentx-atlas-multiview-8u8mqhdv/model-driver-adjustment.json` preserves the failure and resolution. No model relaxation or retry loop was used.

### Visual assessment and closure

At1366×768, the graph viewport is1324×410.56px, current-event dock104px, facts48px; all stay visible. Flow exposes all6 mail stages, actors/handlers, action/change counts and direction at once. Sequence shows participant names and ordered actions with local/retained work distinct from arrows; long traces scroll internally. At390×844, flow cards fit the width as a vertical chain, current event/facts remain available, and sequence topology uses internal horizontal scrolling with a retained header. Full topology is not simultaneously visible on phones. Browser emulation does not certify physical touch or assistive technology.

Both actual browser runs closed their browsers. The one fresh specialist completed source handoff and was archived through Paseo (`success:true`); tool Task now links this actual acceptance. Only the intentionally retained atlas listener remains. No AgentX/Provider execution, dependency install, native build, `.agentx`, `.bin`, installed/default program, user workbench18947 or protected demo18731 operation occurred. No further source/check/review work is selected.
