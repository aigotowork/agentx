---
scope: Default-on Canvas discovery and actions in the Space browsing context
status: independently-verified-local-source
owner: scene-runtime-vidore
related: [../../../../../.docs/agent-space/canvas-tooling.md, ../../../../../.controller/office-details/scene-runtime.task.md]
last_updated: 2026-11-15
---

# Discover and operate the scene

## DancingAI entry

The default Space HTML now ships the public no-service DancingAI browser tool. Expand its toolbar and choose Objects / 对象与数据, then the `Agent Space` source. `dancingai-provider.ts` maps each mounted Canvas to a provider; `src/inspection.ts` owns registration, route-scope renewal and cleanup. Discovery does not change Agent membership, send messages or restart services.

```js
const inspection = window.dancingai.inspection;
const source = inspection.sources().find(item => item.id.startsWith('agent-space:'));
if (!source) throw Error('No mounted Space canvas; switch to the scene view');
const listing = inspection.list(source.id, 2000);
if (listing.truncated) throw Error('Inspect a bounded subset first');
const root = listing.items.find(item => item.type === 'AgentSpaceCanvas');
const observation = inspection.inspect(root.handle);
const api = inspection.objects.get(root.handle); // actual window.agentxSceneDebug
const canvasId = observation.data.canvasId;
console.log(api.describe(), api.list({kind: 'actions', canvasId}));
const image = await inspection.capture(root.handle, 'captureScene');
console.log(image.scope, image.gaps); // scene; Canvas-only PNG
```

Scene children are detached semantic observations from the last presented frame, not mutable native Node instances; native get is intentionally unavailable for those children. Read their original objectId/canvasId and current action availability from inspection data, then use the existing facade below. Actions remain validated and callback-owned. Invoking an action returns acceptance only; use waitForFrame and compare the resulting state. Native references stay inside the target Document. URL changes retire old DancingAI handles; discover the renewed source instead of reusing IDs across scopes/remounts. Switching to chat removes the canvas source and switching back creates a new mount/source. Default tool loading is shared by dev, static and native embedded builds; it does not update an already running binary.

The [overall delivery plan](../../../../../.docs/agent-space/canvas-tooling.md), [specialist design and evidence boundary](../../../../../.controller/office-details/scene-runtime.task.md#diagnostics-structural-and-state-design--vidore) and [scene ownership](../AGENTS.md) explain the selected outcome. Exact implemented declarations live in [contracts.ts](contracts.ts); [registry.ts](registry.ts) declares the global discovery methods and [space-adapter.ts](space-adapter.ts) declares the actual action promises, validation and callback routing. Integrated source has [independent finite Linux Chrome/demo acceptance](../../../../../.controller/office-details/scene-runtime-verification.task.md#completed-independent-outcome), including generic discovery, real callback invocation, actual Canvas/DOM projection, PNG correlation and lifecycle cleanup. This is source evidence; the selected LAN service was not changed.

`window.agentxSceneDebug` is available by default in development and production, with no debug query. It is a frozen discovery/call facade, not a runtime/controller reference. A caller in a parent page must evaluate inside the Space frame. Cross-origin postMessage and remote RPC are outside this tool.

| Call | Parameters | Result meaning |
| --- | --- | --- |
| `describe()` | None | Version 1, method argument/result contracts, semantic model, coordinates and errors. |
| `list({kind:"canvases"})` | No prior ID | Mounted instances, lifecycle state and last frame metadata. Empty list means no mounted Canvas. |
| `list({kind:"objects",canvasId,objectKind?})` | Discovered canvas ID; optional kind from describe | Serializable semantic objects with opaque IDs, parents/relations, details, visibility and coordinates. |
| `inspect({canvasId,objectId})` | Discovered object ID | One object from the last presented frame; absent/stale object is an explicit error. |
| `list({kind:"actions",canvasId,objectId?})` | Optional discovered target | Action purpose, scope, target kinds, parameter constraints, result field types/meanings, effects and current availability/reason. |
| `invoke({canvasId,actionId,objectId?,args?})` | Discovered action, validated arguments | Callback acceptance receipt with thresholds; does not confirm a presented or successful state change. |
| `snapshot({canvasId})` | Discovered canvas ID | Actual last presented frame plus on-demand semantic objects and current layout metrics. |
| `waitForFrame({canvasId,afterFrame,afterInputRevision?,timeoutMs?})` | Thresholds from receipt/snapshot; timeout 1–10000ms, default 2000 | Promise of a matching actual presentation or timeout/disposal error. No second animation loop. |
| `capture({canvasId})` | Discovered canvas ID | Actual PNG data URL and correlated frame/layout metadata, read in one synchronous task. |

All calls return `{ok:true,value}` or `{ok:false,error:{code,message}}`; `waitForFrame` resolves that shape asynchronously. Error codes are discoverable. `no_scene` means no adapters are mounted; `unknown_canvas` identifies an unknown ID while mounted or one of the most recent 64 disposed IDs. `no_frame` means a mount exists before its first presentation. `unknown_object` means the requested ID is absent from that presented frame. `unavailable_action` includes a reason such as missing target/control, pending input, filtered/offscreen target or non-interactive furniture. Unsupported actions, invalid arguments, callback failure, PNG failure, disposal and timeout are separate outcomes. IDs are opaque and canvas-scoped; never construct them or reuse them across remounts.

## An Agent starting without object IDs

Run this example inside the Space page or the Playwright frame that hosts it. It discovers the contract, selects an available zero-argument object operation, waits for a committed presentation and reads actual state and pixels. No fixture name/ID is embedded.

```js
const result = await page.evaluate(async () => {
  const api = window.agentxSceneDebug;
  const read = result => {
    if (!result.ok) throw new Error(`${result.error.code}: ${result.error.message}`);
    return result.value;
  };
  const contract = read(api.describe());
  const canvases = read(api.list({ kind: "canvases" }));
  if (!canvases.length) return { contract, status: "no mounted scene" };
  const canvasId = canvases[0].id;
  if (!canvases[0].lastFrame) {
    read(await api.waitForFrame({ canvasId, afterFrame: 0 }));
  }
  const objects = read(api.list({ kind: "objects", canvasId }));
  let chosen;
  for (const object of objects) {
    const actions = read(api.list({ kind: "actions", canvasId, objectId: object.id }));
    const action = actions.find(item => item.scope === "object" && item.targetKinds.includes(object.kind)
      && item.availability.available && Object.keys(item.parameters).length === 0);
    if (object.kind === "actor" && !object.details.selected && action) { chosen = { object, action }; break; }
  }
  if (!chosen) return { contract, canvasId, status: "no eligible actor", objects };
  const before = read(api.inspect({ canvasId, objectId: chosen.object.id }));
  const receipt = read(api.invoke({ canvasId, objectId: chosen.object.id, actionId: chosen.action.id, args: {} }));
  const presented = read(await api.waitForFrame({ canvasId, afterFrame: receipt.afterFrame,
    afterInputRevision: receipt.afterInputRevision }));
  // All three reads below run synchronously: no RAF can intervene between them.
  const after = read(api.inspect({ canvasId, objectId: chosen.object.id }));
  const snapshot = read(api.snapshot({ canvasId }));
  const capture = read(api.capture({ canvasId }));
  return { contract, before, receipt, presented, after, snapshot, capture,
    observedSelection: after.details.selected === true,
    captureMatchesSnapshot: capture.frame.frameId === snapshot.frame.frameId };
});
```

The actor kind is discovered through `describe().value.model.objectKinds`; it describes the current adapter vocabulary, not a generic requirement for every possible Canvas implementation. An empty office can have no eligible actor; inspect furniture/human/actions or switch the normal UI to a populated office. For an actor already selected, activation may be a no-op and waiting for a newer input can time out. For general automation choose a currently unselected actor, or discover an applicable camera/pause action with a changed value.

To save the capture in Node without downloading browser URLs: `await fs.writeFile(path, Buffer.from(result.capture.dataUrl.split(",")[1], "base64"))`. A Playwright `page.screenshot()` captures DOM labels, tooltips, reader and chat that are deliberately absent from the Canvas PNG. In an embedded host use the Space frame's evaluate and the host's ordinary screenshot separately.

## Actions and presentation

The current adapter declares object activation, pan, center-relative zoom, panorama and pause. Object activation uses the same exact actor/human/prop callbacks as normal interaction. Human activation toggles its existing history feed; it does not select an Agent or acknowledge messages. Furniture, zones and effects are inspectable without inventing actions. Camera and pause callbacks are supplied by SceneSpace; absence is discoverably unavailable. Arguments reject unknown fields, wrong types, nonfinite/out-of-range numbers and unsupported actions. Nothing sends messages, launches/stops Agents or mutates native facts.

Invocation returns `accepted:true`, a request ID, `afterFrame`, `afterInputRevision` and `presentation:"not-confirmed"`. It confirms only that the existing callback returned. A later committed-input/frame pair can be awaited, then compare the actual intended result (selected key, camera, paused state or feed DOM). An unrelated newer frame does not prove an action succeeded. A no-op, hidden document or unavailable state change may never produce a newer input/frame and will time out. Calling wait does not wake or advance a paused/hidden scene. Input revision and frame ID are local to this mount, not native versions or durable receipts.

## Truthful geometry and capture

Snapshots observe the last frame after both Canvas rendering and semantic DOM anchor synchronization. Sampling animation again is forbidden. Registry and adapter add no RAF; only pending `waitForFrame` calls use bounded timeout handles. Frames retain references privately; detached JSON projection occurs only when a caller reads. Returned values can be changed by a caller without affecting rendering or controller state.

| Space | Meaning |
| --- | --- |
| `world` | Scene coordinate units; actor details distinguish goal/home/rest from sampled position. |
| `canvas` | Canvas-local CSS pixels from the sampled frame camera and viewport. |
| `client` / `page` | Current Canvas axis-aligned DOM rect and document scroll; layout is measured at read time, not claimed as historical frame layout. |
| `backing` | Actual Canvas pixel dimensions and per-axis CSS-to-backing ratios, including DPR rounding. |
| Image bounds | Actual sprite image boxes, including transparent pixels; not pixel-opaque hit regions. Shared placement math matches the renderer. |
| Composite furniture | Room-local rounded/clipped image boxes; furniture stays semantic even when baked into a room bitmap. |
| Model bounds | Rooms/zones explicitly label geometric bounds; no fabricated pixel precision. |
| DOM bounds | Current semantic target rect when mounted; `null` otherwise. Pending React input can make it newer than the presented Canvas. |
| Visibility | Filter, renderer draw decision and rectangular Canvas intersection; not wall/other-sprite/DOM-overlay occlusion or real user visibility. |

General rotated/skewed/perspective CSS transforms are not reconstructed by axis-aligned rect ratios; the current scene uses ordinary layout/camera transforms. Pending-letter effects aggregate native pending counts or total native unhandled human messages; individual letter/text pixel bounds are explicitly unavailable. Message effects describe only the runtime-presented first six flights, including scheduled entries; they do not expose the private queue. Missing endpoint anchors have explicit unavailable geometry. A Canvas is still described if it has no first frame; reads then return `no_frame`.

Capture uses `canvas.toDataURL("image/png")`, synchronously paired with the adapter's last presented frame ID, committed input revision, elapsed/still/paused state, camera/viewport, backing size and live page/client rect. It neither redraws nor includes DOM labels/tooltips/chat. Zero backing size or unavailable/tainted encoding returns `capture_failed`. Capture and snapshot from separate browser tasks can legitimately refer to different frames; use one evaluate task to correlate them.

The global facade remains after unmount with an empty canvas list. Mount-specific registration, last frame, callbacks and pending waiters are disposed; waiters resolve `disposed`, stale calls fail and remount gets a new instance ID. No runtime motion/cache maps or controller/transport objects are published.

## Verification

Authored [diagnostic cases](../../../tests/scene-debug.test.mjs) use `node --experimental-transform-types --test web/agent-space/tests/scene-debug.test.mjs`. The runtime cases use the same transform flag because Node24 strip-only cannot load TypeScript parameter properties. Final test/build/browser evidence belongs to [the independent verification Task](../../../../../.controller/office-details/scene-runtime-verification.task.md). The implementer performs one scoped static check after coherent extension edits. No service deployment/adoption is implied.


The mailbox/cat extension retains the stable `human` nest/accessible endpoint and human addressed-flight anchors. Its `unhandledMessages` is explicit native pending-handling history, never Agent delivery or read evidence. Separate decorative cat/yarn `effect` objects project the actual `SceneFrame.cat` position, activity and finite sprite bounds; they grant no mutation actions. Opening the human feed settles decorative motion and does not mark messages handled. Deliberate handling remains ordinary feed UI through native contracts; the Canvas facade does not expose it. See [current feature/evidence](../../../../../.controller/office-details/bigboss-cat.task.md).

### Live object observation

Open DancingAI **React / Runtime Inspection**, choose the **Agent Space** source and **点选页面对象**. Move over the real Canvas object to preview its bounds, then click to inspect without activating its business action. **浏览对象** is secondary discovery. Use **开始实时观察** in the heading. The separate **对象与数据** entry remains available. The local object panel samples its latest published observation every 200ms; it does not invoke actions or capture screenshots. **暂停实时观察** retains the displayed sample. Returning to the list, closing/inactivating the panel or losing the object stops observation; hidden documents skip sampling. This is sampled inspection, not a recording of every render frame. The cached remote iframe workspace does not expose this control yet.

### Source-enabled inspection preview

To rebuild an isolated `/space/` preview with original React definition locations and standard Vite maps, use the explicit checkout recipe:

```sh
node web/agent-space/scripts/vendor-dancingai.mjs /absolute/dancingai
node web/agent-space/scripts/build-inspection.mjs /absolute/dancingai /tmp/new-inspection-preview
```

This requires the matching Inspection/Page/Vite plugin builds in that checkout. The source plugin records application component definitions before minification; the existing Runtime overview receives `component.source`. It does not change panel layout or synthesize Hook variable names. Merely copying the browser tool cannot retrofit source definitions into an old application bundle. Normal builds continue to use the vendored browser artifact without a sibling-checkout requirement.

### Feedback endpoint

The browser entry now uses `/dancingai/` by default (optional `meta[name="dancingai-endpoint"]` override). Static assets alone do not persist feedback. The :18976 preview uses DancingAI `examples/agent-space-service/server.mjs` with file-backed feedback outside the build directory and the bundled management UI at `/dancingai/ui/`. Native AgentX service deployment remains separate.
