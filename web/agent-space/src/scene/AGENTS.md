# Scene ownership

For the illustrated game offices, read [render and preview ownership](../../../../.controller/space-ui-styles/game-office/design.md). `ambience.ts` owns the shared lamp anchor and foot-based pose transform; `runtime/studio-atmosphere.ts` draws the selected pack on the existing frame clock. `SceneSpace` owns per-room visual light state and the diagnostic adapter exposes the same local toggle. Keep decoration and visible game poses independent of native Agent effects.

For Catalog Studio, start at the [integration Task](../../../../.controller/space-scene-art/tasks/integration.task.md) and [SVG asset contract](art/studio/AGENTS.md). `model.ts` owns the room, seats and solid prop contact footprints; `studio-art.ts` maps that model and the existing actor frame to anchored SVG image boxes. `studio-assets.ts` decodes the finite local set once, and `runtime/studio-render.ts` draws it through `SceneRenderer`. The mounted diagnostics use the same placement functions. `SceneSpace` fits the first usable pinned roster once for an untouched camera; later polls never reset it. Native/default pixel presentation retains the original painter. Studio's mailbox/cat is a single static vector; no independent actor clock or motion capability is supplied by artwork.

For the user-authorized default Web DancingAI integration, follow [the direct Task](../../../../.controller/office-details/dancingai-integration.task.md) and [usage](debug/README.md#dancingai-entry). `debug/dancingai-provider.ts` consumes the existing serializable facade; `src/inspection.ts` owns the public Page artifact mount, provider leases and route renewal. The direct task owns its focused checks/build/demo preview; historical independent-executor limits below do not apply to this separate integration. Preserve running :38023 and existing native operations.

This module projects supplied Agent facts into the existing pixel office. It never operates Agents or sends messages. See the [scene runtime Task and design](../../../../.controller/office-details/scene-runtime.task.md), [office behavior](../../../../.controller/office-details/design.md), [navigation/lounge model](../../../../.controller/office-details/lounge-refinement.md) and [responsive composition](../../../../.docs/agent-space/responsive-space.md).

The default-on [Canvas tool usage and discovery example](debug/README.md) implements the [selected overall plan](../../../../.docs/agent-space/canvas-tooling.md). Exact serializable contracts live in `debug/contracts.ts`; current actions and validation are declared in `debug/space-adapter.ts`. Final independent execution remains recorded in the Task.

## Entry and responsibility map

| Entry | Owns | Retain |
| --- | --- | --- |
| `SceneSpace.tsx` | App-owned per-office camera memory, selection/reader composition, viewport measurement and controls | Camera scale, return view and exact selection across resize/view/resource changes. |
| `HallScene.tsx` | React semantic objects/facts, pointer pan and runtime/renderer composition | Existing public callbacks and `HallPropAnchor` type export. Canvas and actor DOM positions consume one presented frame without per-frame React state. |
| `debug/registry.ts`, `debug/contracts.ts` | Default global `window.agentxSceneDebug`, small mounted-canvas registry and serializable discovery/call contracts | No enablement query; no mutable controller/runtime API. Registry remains available after unmount; stale instance IDs fail. |
| `debug/space-adapter.ts`, `debug/objects.ts` | Last-presented-frame inspection, semantic furniture/actors/effects, existing-owner action routing and Canvas PNG | JSON projection only on reads; accepted actions are distinct from observed input/frame. Dispose unregisters and completes pending waits. |
| `runtime/sprite.ts` | Exact sprite selection/placement value operations shared by renderer and on-demand object bounds | Bounds label transparent image boxes, never opaque-pixel hit regions. |
| `runtime/contracts.ts` | Private `SceneInput`, `SceneFrame`, actor samples, flights and environment operations | Explicit elapsed decorative vs wall-clock milliseconds; world `frame` vs projected `screen` coordinates. |
| `runtime/runtime.ts` | One `SceneRuntime` per mounted HallScene effect; browser environment adapter | `start` once, `update` supplied input, `point` last actor sample, permanent/idempotent `dispose`. Runtime owns its RAF and subscription cleanup. |
| `runtime/actors.ts` | Exact-key movement and arrival memory | Sample existing motion/navigation functions; changed seat coordinates snap without replaying arrival. Selected point holds/release remain visual only. |
| `runtime/messages.ts` | Deduplication, recent addressed flight queue and geometry/visibility invalidation | Messages/pending letters/human history remain distinct native facts. Effects never acknowledge delivery. |
| `runtime/renderer.ts` | Canvas backing dimensions, fixed mount palette, bounded bitmap cache and draw order | Renderer consumes a SceneFrame; it must never resample actor movement or own another clock. |
| `model.ts`, `population.ts` | Full-roster geometry, stable capacity/seat memory and actor-local navigation geometry | Filters dim the existing projection; status changes never redefine room population. |
| `motion.ts`, `navigation.ts`, `geometry.ts` | Pure behavior/path/frame/camera value operations | Existing route constraints, Leg timing and deterministic shared-time sampling. |
| `office-art.ts`, `art/`, `office.css` | Pixel assets/palette and visual tokens/semantic anchor styling | Source provenance, palette/cache keys, feet/head alignment and labels at screen scale. |

Dependency direction: React composition → runtime → actor/effect owners → existing pure value modules. Renderer consumes runtime value contracts plus art; runtime has no Canvas/React dependency. Runtime files remain private scene implementation. The only global API is the deliberately bounded diagnostic facade, which reads a presented frame and calls declared existing interactions. `index.ts` remains the existing pure value export surface.


For the [SDK-backed office/list path](../../../../.controller/space-domain-sdk/tasks/space.task.md), `HallRoom.actors`, `office.fixtures` and `cabinet` already express the useful Room → Actor/Prop relationships. `HallScene` carries the room key on actor/room/cabinet DOM entries; the actor carries the existing exact roster card key in `data-hall-object`. Furniture and the scope-owned human anchor have no Agent address. The mounted diagnostic facade keeps its own IDs and reads the same presented frame. `ActorAnchor` activation checks the current visible projection and forwards only the exact roster-key intent through `SceneSpace`; it neither owns the history read nor depends on a sprite reaching a point. After a resize during reading, the return view keeps zoom and reframes a clipped target before returning focus to the current exact anchor or the directory button.

## State and lifetime rules

`SceneRuntime` owns created → started → disposed. Started may be animated or quiescent because of pause, reduced motion or hidden document. Input updates and environment changes replace the pending RAF and reset its timestamp baseline. Visible animated ticks advance elapsed time by at most 100ms; drawing retains the 40ms cadence. Pause/reduced motion renders settled state once and has no continuation. Visibility transitions settle paths, discard flights and consume supplied messages, avoiding a hidden-time jump/replay. Dispose cancels pending work, removes listeners and clears actor/effect state; HallScene also disposes the renderer cache.

A presented `SceneFrame` is the only sample used for Canvas actors and imperative DOM actor anchors. React owns node membership, accessible labels and callbacks; no frame runs React state setters. App/SceneSpace retain office memory/camera/selection; runtime state expires with the mounted scene. Exact actor keys fence namesakes/incarnations; coordinates are never identity. Do not publish or mutate motion maps, arrival sets, message queues or cache from consumers.

## Diagnostic lifetime and action ownership

HallScene updates a mount-specific SpaceDebugAdapter with the same input object supplied to SceneRuntime. Only after both renderer draw and actor DOM synchronization complete does it publish that actual frame. Input revision and frame ID are local observation identities, separate from native source versions. Each invocation routes to existing HallScene/SceneSpace callbacks, returns an acceptance-only receipt and permits event-driven waiting for a newer committed input/frame; callers still compare intended state. No diagnostic call resamples animation or adds RAF. On effect cleanup, runtime, renderer and adapter registration are disposed together; pending waiters resolve disposal and remount gets a new Canvas ID. The frozen global facade stays present with an empty registry. Use [the tool README](debug/README.md) for coordinate/capture limits and an ID-free browser example.

## Verification route

[Deterministic runtime cases](../../tests/scene-runtime.test.mjs) cover scheduler/disposal/remount, browser subscription cleanup, pause/reduced/visibility time, shared world/screen samples, exact incarnation removal, arrival/reflow/selection and bounded flight deduplication. Run with `node --experimental-transform-types --test web/agent-space/tests/scene-runtime.test.mjs` from repository root using the existing Node24 transform mode: runtime constructor parameter properties require transformation beyond the strip-only loader. Resolution hooks follow `office-details.test.mjs`. Tests are behavior-based; do not assert private field names or scrape HallScene source to verify moved responsibilities.

[Diagnostic cases](../../tests/scene-debug.test.mjs) add registry/error/detachment, object geometry, action validation/availability/callbacks, accepted-vs-presented waits and correlated capture-port behavior; they do not prove actual browser PNGs. Run `node --experimental-transform-types --test web/agent-space/tests/scene-debug.test.mjs`.

Keep existing office-details and camera/browser cases for geometry and real DOM/Canvas integration. The active Task assigns final test/build/browser execution to an independent executor; implementer owns one scoped `node web/agent-space/scripts/run.mjs typecheck`. Authored cases are not execution evidence. Builds use fresh external output paths, disposable hosts use free loopback ports, and selected LAN :38023 plus installed/default programs remain untouched.


## Human mailbox and decorative cat

[Current Task/design/evidence](../../../../.controller/office-details/bigboss-cat.task.md) owns the cohesive follow-up. `HumanMailbox.tsx` projects native handling state/actions supplied by LiveSpace; scene/runtime never mutate mail. Reading/opening does not handle messages. Human envelopes/count mean total unhandled history, supplied independently of paged letters. Native receipt, human handling, transient inspection and retained correspondence are distinct.

`cat.ts::sampleCat` is a private pure decorative sampler. One per-mount seed and existing runtime elapsed clock choose finite rest/jump/entrance-patrol/yarn phases; `SceneFrame.cat` is the only sample drawn and inspected. `office-art.ts::humanCatNest(part,pose,facing)` splits stable nest and finite cat sprites; `humanYarnBall` supplies bounded roll art. Nest/accessible label and flights remain anchored at `scene.human`, outside Agent membership. Cat motion cannot process mail. Feed open, pause and reduced motion settle at nest. Current geometry supports only open entrance patrol; south-only room doors lack an exterior connecting aisle, so do not fabricate wall-crossing room entry. Debug `human` represents stable communication entry; decorative cat/yarn `effect` objects expose actual presented positions/art bounds, with no operation/Agent lifecycle.

Focused cat/runtime/debug checks are executed by this feature's assigned owner under its Task, superseding the historical source-refactor verifier-only execution rule only for this follow-up. Do not replay prior broad acceptance. Native/browser mutations must use disposable scope fixtures. Source checks do not imply selected LAN adoption.

Actor context interactions route through optional `onActorContext`; `HallScene` owns gesture cancellation/click suppression only. Commands remain in the parent interaction host. The human feed registers above the conversation reader; directory and reader register with the common overlay owner so Escape closes one foremost interactive surface, including focused inputs. Current verification is owned by the linked bigboss Task, not the historical refactor's independent-executor rule.


## UIUX presentation integration

Canonical DOM presentation resources now live in [the Space UIUX system](../../uiux/README.md): `HallScene` supplies exact refs, gesture handlers and frame-owned positions to `ActorAnchor`/`SceneObjectLayer`; `SceneSpace` supplies existing callbacks to `SceneControls`; `ExpandedObject` supplies its original geometry to `ObjectWindow`; `HumanMailbox` adapts the native feed and safe Markdown into `HumanMailboxBlock`. Runtime/debug APIs, LongPress lifecycle and cat samplers remain here. Styles are maintained under `uiux/systems/space/styles/`; historical scene CSS files are compatibility imports. Source/static checks and current independent acceptance status belong to [the UIUX Task](../../../../.controller/office-details/space-uiux.task.md).
