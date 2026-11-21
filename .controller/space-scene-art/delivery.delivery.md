---
schema: delivery
status: completed
description: Deliver a coherent original SVG art pack, integrated as a playable AgentX Web Space office.
stages:
  scene:
    description: Compose, integrate and operate one representative busy office and its complete conversation loop.
    tasks: ["./tasks/art.task.md", "./tasks/integration.task.md"]
---

# Space scene art and integration

The user wants to open and operate Space, judge the experience in use, and assign dedicated Agents to design the map, cast, props and decoration. They explicitly confirmed that the redesigned interaction must consume the public object SDK and that a restyled legacy shell does not constitute adoption of the [original prototype](../space-svg-redesign/delivery.delivery.md). The integrated preview therefore uses the real [SDK-backed Space data path](../space-domain-sdk/delivery.delivery.md) over a read-only synthetic Management listener. A separate no-network sample is useful for disconnected art development only. The current production deployment is separate.

Our first substantial implementation is one busy studio with legible subspaces, varied characters, status/attention cues, furnishings and scenic decoration; verify an adjacent or empty room too where the existing projection supports it. Design for entering the office, finding someone available, opening a conversation and returning to that person. Use the user's game-like direction: original hand-inked/woodcut forms and a subtle mosaic or stepped edge. Avoid long explanation copy, UI-only status dependent on color, crowded decorations around hits, permanent tutorial text, and a static single-scene image with painted-in inhabitants. Previous SVG prototype is a reviewable prior proposal, not reference implementation; design the art cleanly. The customer wants a fine playable experience, not just an asset sheet.

| Owner | Shared boundary | Primary consumer |
| --- | --- | --- |
| Prototyper | Original, editable SVG symbols and asset inventory with dimensions, anchors, layering and art IDs. Owns visual composition and interaction variants. | Implementation Agent resolves the asset definitions into the runtime Canvas; no business logic in assets. |
| Implementation | Uses current Room/geometry, actor placement/pose, camera and the exact roster card. Stable visual lookups and one presented frame. Art source stays separate. | Actual Space page, its Canvas, DOM targets, list, conversation and diagnostic view. |
| Controller | Start reachable isolated review URL, source/built browser & visual evidence on the actual integrated page; drive cohesive handoff. | User opening a reviewable artifact, with explicit data/evidence limits. |

The art/engineering split is warranted by independent specialized visual asset authoring and production Canvas/React integration; don't fragment either into per-asset tasks. Share the asset interface early. Renderer can make a placeholder probe without borrowing earlier artwork into design. No new backend, SDK, Contract Canvas or native operations; keep existing camera, routing, exact drafts, scene frame and available local affordances. The demo may use richer fixture facts/labels but no real participant data or fake capabilities. Other teams and installed/live service ports stay untouched. A longer-term theme catalog or a separate runtime is out of scope unless current evidence demands it.

## Accepted preview and ownership

[Playable Space Studio](http://192.168.2.178:18989/?art=studio&office=%2Fstudio%2Fcatalog) runs in Controller-owned tmux session `agentx-space-art-18989`. [The preview server](verification/preview.py) serves the accepted static build and read-only Management fixtures on the same origin. The build pointer `.artifacts/space-scene-art/site` selects `/tmp/space-studio-sdk-candidate-02` (`index-CW1I5A_N.js`, `index-CN7aXKao.css`). Refresh loads the accepted build. Select @mora, read the R-14 note, start a draft, and switch to Chat to see the same conversation and draft. [The original SVG prototype](http://192.168.2.178:18985/) remains a separate design artifact.

Nezire (`prototyper`, batch `90c5e055`) completed the art and its design proof. Vunura (`implementation`, batch `f65639c0`) completed the renderer integration, richer sample and app behavior. [The 24 self-contained SVGs](../../web/agent-space/src/scene/art/studio/AGENTS.md) supply tiles and thresholds that follow room geometry, foot-centered cast and props with world anchors, and distinct status glyphs. Their Tasks own the final contracts. Kula accepted the built composition for the isolated preview.

Scoped TypeScript and the build passed. The [independent Chrome report](../../.artifacts/space-scene-art/studio-04/report.json) records six actors drawn in the actual Canvas, scene/list continuity for an exact conversation and its draft, distinct drafts and a same-name stopped instance with its own read-only history, narrow touch and keyboard return, and an empty Annex. The [LAN browser probe](../../.artifacts/space-scene-art/studio-04/lan-smoke.json) confirms that the published URL itself draws Studio, resolves @mora, and reads the R-14 note using same-origin Management GETs with no page, API or asset errors. [Desktop](../../.artifacts/space-scene-art/studio-04/desktop-studio.png) and [narrow reader](../../.artifacts/space-scene-art/studio-04/narrow-studio-conversation.png) captures were visually inspected. This closes the scoped scene-to-reader integration; the preview has no real agents or sends, and deployed Space and daemon services were not changed.

## Prototype to product correspondence

| Prior prototype validates | Selected production binding | Remaining product design |
| --- | --- | --- |
| Room-first entry and distinct jobs/social areas; scene and roster lead to people | Original Studio vectors use actual room/seat geometry. Role-based costume and signal silhouettes are Space presentation data. Room and list resolve the same observed member. | Scene-first shell/navigation hierarchy, richer office identities, and meaningful multi-room movement are separate from rendering one studio and an empty annex. |
| Click a person, see their recognizable portrait, read their letter, return to the room | Actual Canvas/DOM anchor and exact app intent; shared reader with matching portrait and app-owned draft. Narrow room reading is a focused workspace with one contextual return. | Desktop letter surface, compact everyday discovery bar/room dock, and design of secondary actions need separate prototypes and production parity before replacing the old shell. |
| Invented prototype conversation, room transitions and moving cast express a game-like experience | Actual SDK scope/workspace/agent governs facts/history. Scene has one renderer/frame and no transport. Demo backend serves synthetic read-only data; prototype send/reply is not carried over. | Deliberate send and receipt recovery remain on the existing app operation path. Richer motion or interactions must describe concrete user tasks and available result states rather than inventing service facts. |

The actual composition is `selectAgentXScope → Workspace.members / Agent.snapshot / Agent.messages → DomainOfficeReads → SpaceController and app projections → Studio Canvas / list / reader`. `art=studio` changes presentation only. Only the trusted host chooses the listener and expected scope. This distinction allows the same redesigned controls to work over sample or actual SDK data without a second UI/runtime; the configured review deliberately exercises the SDK branch.
