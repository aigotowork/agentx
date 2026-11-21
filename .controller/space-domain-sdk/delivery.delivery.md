---
schema: delivery
status: completed
description: Implement and verify one complete SDK-backed Space office/list conversation path and evaluate the scene/object boundary.
stages:
  space:
    description: Bind the public SDK, application state and scene interaction through the actual Space.
    tasks: ["./tasks/space.task.md"]
---

# Space domain SDK first scenario

The user authorized continuing design and implementation of AgentX Web Space and wants evidence whether the layered design improves UI/Canvas implementation clarity. Deliver one complete bounded consumer path: choose a known office, observe its bounded roster, select an exact actor in the scene or list, read its AgentX history, and close/reopen or switch views with the same addressed draft and camera context. Use actual public object SDK handles over the existing transport. Inspect one concrete scene object intent and measure the boundary through a real browser path, rather than proposing a second Canvas implementation.

The [selected scenario](../../.docs/web-product-design/domain-sdk-scenarios.md#space--one-conversation-in-office-and-list) supplies the design and acceptance cases. Existing Space already has one controller and one presented scene frame; reuse those instead of duplicating state or animation. Keep the SDK scope, application selection/roster/draft and view/scene object identity separate. An explicitly configured trusted listener/expected scope enables the bounded path; without it keep existing entry behavior. A configured failure never silently falls through. Member and history pages are bounded and independent; preserve exact identity, render failure and continuation honestly, and retain stopped history as read-only. UI/backend operations outside this slice remain with their current owners.

One focused coder owns the complete Space authoring/integration. The split is warranted by coherent application/browser/scene integration complexity; no separate author per file. Kula owns independent built-browser execution and final composition, so author and verifier are distinct. Work in the current workspace while withstanding all pre-existing dirty source; do not stage, reset, clean or overwrite other edits. No release/restart, production-site rollout, SDK/native redesign, Management implementation or Contract Canvas change is selected. Use fresh outside-build paths and an unused isolated preview port with synthetic input; preserve :38023, :38000 and other previews. User-facing evidence should make the model's benefit, tradeoffs and remaining work checkable.

Author `humove` (`coder`) owned the Space design, implementation and narrow interaction repair. Kula prepared the synthetic Management fixture and independently verified the built consumer. The [Space Task](tasks/space.task.md) owns source handoff; this Delivery owns the integrated browser outcome.

## Independent verification

[Replay and fixture](verification/README.md) exercise the static Space artifact in Chrome through the public SDK. The first build's desktop path passed, but its [390 px return capture](../../.artifacts/space-domain-sdk/run-01/narrow-office.png) showed an off-screen room and actor after resizing an open conversation and closing it. The owner repaired camera return to retain the saved zoom and recover the exact actor only on a clipped axis; keyboard focus returns to that visible anchor. Fixture timestamps were also corrected before the final run.

The final build `/tmp/space-domain-sdk-candidate-04` passed scoped TypeScript compilation and the [eight browser scenarios](../../.artifacts/space-domain-sdk/run-04/report.json). It includes the focus/camera correction, reconnect event and final long-record scroll repair. The run covered:

- One bounded member page per continuation, including an empty projected page with another page available. An actor's semantic button belongs to the exact workspace room. A stopped namesake has its own read-only history and no avatar.
- An exact opened conversation shared by scene and list. The same draft, scope, generation and subject survive switching views without another exact snapshot or message read. The explicit message continuation appends records using the checkpoint. Closing restores the camera; after resize to 390×844 it returns the actor to view, and Enter reopens the same draft.
- A long forward history starts at its beginning. Loading another page and switching views keep the reader's scroll position instead of following the newly appended tail.
- A direct link can resolve an exact subject outside the loaded roster. A delayed message response does not reopen the conversation after a workspace switch. A wrong configured scope refuses after the descriptor. Changed generation makes the old draft read-only; deliberate reconnect clears the old subject and publishes the cleaned location.

There were no page errors, invalid wire requests or mutations. Visual inspection covered the [desktop scene](../../.artifacts/space-domain-sdk/run-04/desktop-office.png), [scene conversation](../../.artifacts/space-domain-sdk/run-04/desktop-actor-conversation.png), [list conversation](../../.artifacts/space-domain-sdk/run-04/desktop-conversation-list.png), [narrow reader](../../.artifacts/space-domain-sdk/run-04/narrow-actor-conversation.png) and [narrow return](../../.artifacts/space-domain-sdk/run-04/narrow-office.png). The screenshots are from the actual built app with fixture data. They also show the next visual work: small offices leave much of the viewport unused, and technical or infrequent controls compete with the conversation at narrow widths. This verifies continuity and the model boundary; it does not establish native send, active-Provider behavior, larger scene density, or live-host adoption. The static test server and browser were released; no existing service was changed.

## What the application model expresses

| Owner | Introduces and owns | Hands to the next layer |
| --- | --- | --- |
| [Public SDK](../../packages/agentx-client/src/domain/contracts.ts) | The pinned listener/scope/generation, exact `Workspace` and current/stopped `Agent` addresses, snapshots and owned cursors. | Native facts and bounded reads. |
| [Space's read adapter](../../web/agent-space/src/live/domain-office.ts) and [page controller](../../web/agent-space/src/offline/controller.ts) | A workspace exploration and an independently addressed reader; one `openConversation(exactKey, origin)` from the actor/directory/list; shared selection, draft and edit revision. | The observed roster, selected key and reader state. A direct exact route can supply a reader subject separately from the roster. |
| [Scene projection](../../web/agent-space/src/scene/model.ts) and [presented frame](../../web/agent-space/src/scene/HallScene.tsx) | Rooms, live actors and props, seat/navigation geometry, camera and animation. The DOM anchor and bitmap render from the same presented frame. | A click or keyboard action on an actor passes the card key back to Space. Effects, furniture and cabinet do not gain Agent capabilities. |

Composition is enough for this interaction: an actor references a Space card, and the read adapter binds the exact SDK handle. Role-specific placement, appearance or animation can be modeled in Space without creating SDK subclasses or assuming the current Role catalog describes saved adoption. The [Contract Canvas application declarations](../../.docs/object-atlas-v2/layers.md) are conceptual examples; use the source above for implemented contracts. The two canvases have different jobs: one shows the type contracts, while the other presents a mounted office and its interactions.

The main constraints remain the global scan behind `Workspace.members`, the forward-only message cursor, and the difference between restoring an exact subject and observing office membership. Do not turn an exhausted scan into a claim that an office roster is complete or turn a read into command authority. Larger scenes need representative role/activity layouts and an appropriately filtered data read before claiming complete office coverage.

The next design slice can reuse this seam: show an office during a busy shift with several distinct roles, let the user identify who is available and start a conversation, then return to the same position. Give each recurring scene object a clear affordance at its point of use. Compare approachability, reading space and keyboard/touch return against the same task in the list. Use a scene behavior model only for distinct presentation behavior; keep service capabilities and message authority at their current boundaries.
