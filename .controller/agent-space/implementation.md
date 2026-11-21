---
schema: document
status: completed
owner: halo
description: Frontend transplant design, provenance and executor handoff.
links: [./task.md, ../../web/agent-space/README.md]
---

# Agent space frontend migration

## Current state

Frontend implementation and independent finite verification are complete. Halo delivered the original-source scene/chat migration and both justified camera/layout repairs; mofe independently passed final build/typecheck and all declared browser paths, including the user-requested camera extension. No unresolved substantive defect remains. [Verification](verification.md#final-independent-result--completed) owns commands, exact source/artifact identity, scenario dispositions, retained historical failures, limitations and resource cleanup. Ordinary implementation records below retain their chronological context; their earlier pending statements are superseded by this current state and final handoff.

The usable user preview remains `http://192.168.2.178:18967` under zumi's ownership. No root manifest edit, source-repository write, delegation, live API integration or installed runtime mutation was performed by halo. This completed assignment releases frontend writes to the controller; further extension requires a new explicit assignment.

## Design and method

Apply qualified coder/Web guidance: source-first transplant, layout-first composition, explicit parent-owned selection/drafts, controlled conversation, token-based original visual treatment and bounded scenario verification. Source-only evidence is used: CBM reports only an unrelated xpm index; the migration source is not indexed. No graph completeness claim is made.

User chain: inspect working people → select person → read history/compose → return/switch presentation without losing selection or draft. Keep the source's warm pixel office, animated characters/furniture, world camera, anchored expandable conversation window, compact two-line contact rows and IM conversation layout. Both presentations consume one frontend controller. Scene/list filtering shares selected scope/query/state; closed resources remain discoverable through the cabinet and roster.

```text
web/agent-space/
  src/App.tsx                 composition, view/disclosure, focus return
  src/layouts/                scene/chat and conversation spatial boundaries
  src/scene/                  transplanted canvas, geometry, projection, motion
    art/                      original palette-indexed bitmap assets and helpers
  src/conversation/           adapted source reader/history/composer
  src/offline/                explicit fixture adapter + UI-local state
  public/notices/             licenses, upstream records and local provenance
  scripts/                    isolated dev/typecheck/build entry
```

Dependency direction: App → presentation + UI-local controller → replaceable local supply. No service protocol is defined. The local supply exposes sample people/history and appends explicitly local user notes; it never creates an Agent reply, changes a sample execution status or calls a network endpoint. View/disclosure changes cannot submit. Drafts and local notes live for the page lifetime; reload resets the fixture. This bounded lifetime avoids falsely promising durable delivery. Gabi owns future actual API contracts.

## Provenance inventory and scope decisions

Source root: `/home/long/works/longwork/paseo-bots` (read-only). Its `.docs/upstream/source.json` identifies original Bots import `e9d444e772ecd2da3d4b89d47e9b12f5d6c62cb0`; current source includes later local changes, so the migration inventory records actual current file SHA-256 values rather than claiming that import commit describes every file.

| Source family | Transplant and changes |
| --- | --- |
| `shared/hall/pixel/*`, `art.ts`, `walk.ts` | Original characters, furniture, tiles, emotes, palette, PNG encoder, cache and walk sampling. Agent Lobby fixed revision and original declarations retained. |
| `shared/hall/geometry.ts`, `motion.ts`, `model.ts` | Shared camera/motion and responsive office-seat projection. Replace product ownership/runtime references with fixture presentation identity; retain layout behavior. |
| `client/paseo-hall/office-art.ts` | Original furnished room background, walls, lounge, pantry and fixtures; imports become module-local. |
| `web/hall/HallScene.tsx`, `web/office/ExpandedObject.tsx`, object positioning and layouts | Actual canvas/accessible objects, pan/zoom, anchored expandable reading window and scroll composition; isolate source dependencies. |
| `web/hall/HallHome.tsx`, `HallConversation.tsx`, `web/Conversation.tsx`, `web/conversation/HistoryRecord.tsx` | Preserve contact row markup, conversation header, history message/reasoning/tool disclosure and composer structure; remove plugin, remote mutation, upload, permission and runtime adapters. |
| `web/office/office.css`, `web/hall/hall.css`, `web/conversation/history.css` | Retain source visual families and tokens; scoped overrides support the isolated shell and narrow viewport. |

Retain both scene/chat, character selection, readable history, source-style process/tool disclosure, local drafting/composition, room scope/search/state filters, closed history cabinet, camera control, reduced motion and return continuity. Defer Bot/Team creation, plugin/role/library management, profiles mutation, attachments, voice, runtime lifecycle and permissions because they require application authority or real services outside this task. Do not render inert actions for these features. No source user history, settings, paths or credentials are copied; all conversation data is newly authored synthetic fixture content.

Original MIT copyright remains; adapted rich conversation pieces retain Apache-2.0 notice/license. Required historical product names appear only in documentation/notices, not executable integration. Exact file map/hashes will be in `public/notices/migration-inventory.json`.

## Acceptance scenarios for the independent executor

1. Desktop opens scene with recognizable pixel people, desks, room/lounging art and compact overview controls; no service credentials are needed.
2. Scene person opens readable anchored history and reachable composer. Tool/reasoning records disclose locally; enlarge/close returns to the original world and focus.
3. Type a draft, switch scene/chat, select another person then return: each draft remains attached to its exact fixture identity. Close/reopen retains draft and selection.
4. Add a local note (button and Ctrl/Meta+Enter), see it marked local and no invented Agent response. Whitespace/IME composition cannot accidentally append.
5. Scope/search/status apply to both views. No-match has a clear reset. Closed people remain discoverable/readable without appearing as working actors.
6. At 390px and 320px widths, roster/reader return, both modes, history scrolling, draft and composer remain usable without document horizontal overflow. Short viewport/expanded reading preserves input access.
7. Build/typecheck on final inputs once; serve built output and exercise controls with runtime console/network observation. No executable external runtime imports, hidden fetch/WebSocket or source settings/data.

## Issues and next action

- CBM source index unavailable: targeted direct source reads replace graph exploration.
- AgentX rejects ack without a request reply ID and rejects ack of an informational message; acceptance was delivered as `inform` instead.
- Build must set the existing project's WASI Rolldown environment before importing Vite on this CPU.
- Source rest-seat pitch was too small for the Web labels when the RN backdrop was composed into the DOM canvas. The responsive projection retains original geometry ownership but increases lounge width/rest pitch, and narrow selected-camera zoom is adjusted for doubled pixel sprites. These are consumer adaptations, not a new scene design.
- Next: controller assigns independent executor; implementation inputs are ready below. Halo remains available for ordinary source repairs.


## Implemented inputs and development evidence

- Isolated run entry: `node web/agent-space/scripts/run.mjs dev` → `http://127.0.0.1:18967` (not started by implementer).
- Scoped check: `node web/agent-space/scripts/run.mjs typecheck` passed on current source. One earlier pass preceded a source-projection signature cleanup; the relevant final check passed after that cleanup. No final build or browser execution duplicated.
- Runner normalizes its module root before rejecting build output inside/above source; existing output paths fail creation rather than being emptied.
- Build entry for executor: `node web/agent-space/scripts/run.mjs build /tmp/agent-space-acceptance-<unique>`; output path must not exist. It includes the required final scoped typecheck and all public notices. Serve via `python3 -m http.server 18968 --bind 127.0.0.1 --directory <output>`.
- No root dependency/configuration changes needed. No process started, port acquired, source settings read, source files changed, installed executable modified or API endpoint implemented.
- `public/notices/migration-inventory.json` maps actual source paths/current source SHA-256 to each transplant; original pixel art/PNG/palette/walk modules are copied unchanged. `NOTICE.md` preserves MIT and Apache terms and original declaration records. Static app assets carry the same notices.
- Presentation supply is `src/offline/model.ts` → `fixture.ts`, selected once in `App.tsx`. Source scene/IM/reader pieces are decoupled from all original controllers/plugins and consume only local presentation facts. Future API integration is explicitly unimplemented and remains gabi's boundary.

### Executor exact interaction targets

Default scope is `前端工作室` with five sample resources, including one ended record. Scene has four animated people; ended history is an archive cabinet, not a working person. Use `布局 Agent · 工作中` or `会话 Agent · 就绪` for exact accessible actor/contact buttons. Header mode buttons are `场景` and `聊天`. Input is `Agent 消息草稿`; local append is `添加本页笔记`. Scene frame has `放大阅读` / `收起阅读尺寸` and `关闭并返回办公室`; reader also has `返回总览`. Header filters use `工作区范围`, `搜索 Agent 或工作区`, state buttons and current `匹配会话数`. The scene camera controls are under `办公室镜头`; cabinet and `名册` open local contact windows.

Suggested continuity trace: select layout → write draft A → chat → select conversation Agent → write draft B → scene → return and select layout → verify draft A → append note → verify local label and no Agent response → close/reopen. Repeat a bounded selection/compose/return at 390×844 and 320×640; inspect body overflow, history/composer reachability and focus return. Exercise scope `全部工作区`, workspace-name search, no-match/reset, closed filter/cabinet, tool disclosure and keyboard modifier submission with IME guard.

Browser can observe only page/static asset loading plus deliberate license links/copy actions. The app contains no fetch/WebSocket/EventSource or external runtime imports. Markdown images render inert text references to avoid implicit media requests. Any generic favicon request should remain local and is not an API call.

## User preview resource

Controller message #19946 records the user-requested LAN source preview: `http://192.168.2.178:18967`, listening at `0.0.0.0:18967`, PID `3050612`, exec session `32599`, started by zumi. Preserve this process as a user resource. The host override is runtime-only; no module configuration change or deployment is implied. Mofe's independent acceptance server remains separate. Halo has not started or mutated either runtime.


## Independent-execution repair: short scene composer

Mofe message #19997 reports scene input outside 390×480 and 320×480 viewports after the reader returns from chat. The ordinary scene window retained its 152px character preview gap even when the remaining stage could not fit its window heading, conversation heading and composer. The utility-object path already used a short-stage fallback; the conversation path did not. This is geometry ownership, not a draft, route or transport defect.

Repair: `src/scene/object-position.ts` applies the existing short-stage full-reading-region rule to conversation windows as well. `src/scene/geometry.ts` names the shared 420px stage breakpoint as `VIEW.compactHeight`; normal tall anchored placement remains the same. Short scene windows now start at the stage edge with height limited by the same stage's bottom edge. This creates room for header, independent history scroll and composer without changing selection, drafts or local note semantics.

Invalidated executable inputs: exactly `web/agent-space/src/scene/object-position.ts` and `web/agent-space/src/scene/geometry.ts`. Rebuild their artifact and rerun the two narrow/short scene paths (including the normal-size/expand/chat return transitions they cover). Mofe owns execution; halo has not run a parallel build/browser check. The controller-owned LAN source preview is preserved and may hot-reload the repair. Awaiting affected verification evidence before final closure.


## User-reported selection enlargement and related continuity repair

Zumi request #20031 extends this assignment to the selection-induced background enlargement and directly related camera paths. Mofe independently reproduced it on the preserved LAN preview: desktop canvas scale `0.76293 → 2.2` (about 2.88×), narrow `0.75 → 1.4` (about 1.87×). Desktop fully visible actors fell from four to two. Draft continuity was already passing. Evidence owner: mofe, `/tmp/agent-space-evidence-mofe-20261115/camera/`.

### Governing camera model

- World zoom changes only through explicit zoom controls or panorama/initial-scope framing. Selecting a person translates the existing camera to leave reading room and retains its zoom.
- Opening records the overview camera; closing restores it. Resize translates the saved overview center and repositions the selected anchor at the same zoom.
- The scene component unmounts in IM presentation. Page-owned `SceneMemory` retains current camera, return camera, viewport, scope, expanded window and motion pause through that unmount; it does not keep a duplicate hidden reader or create new application state ownership.
- A new scope gets its own initial framing. Panorama discards the previous return snapshot before closing, so reader-close effects cannot overwrite an explicit panorama command. Filter text/select edits keep focus instead of returning focus to a background actor mid-edit.

### Concrete causes and changed owners

`geometry.ts::focusCamera` formerly set a fixed selected zoom. It now requires the existing camera and preserves its zoom while computing translation. `resizeCamera` follows that same contract. `SceneSpace.tsx` formerly reinitialized camera/return state after a presentation remount, reset an unselected camera on every resize, retained an obsolete return camera during selected resize, and could overwrite panorama with its close/return effect. It now owns explicit initial/scope/resize/open/close handling and saves its presentation memory through `App.tsx`'s ref. Selection/history/drafts/local-note state stays in the original controller.

Combined executable input changes since mofe's original artifact: exactly `src/scene/geometry.ts`, `src/scene/object-position.ts`, `src/scene/SceneSpace.tsx`, and `src/App.tsx` under `web/agent-space`. The first two include the short-scene composer repair recorded above. No CSS, dependencies, fixtures, history/composer logic, API or service process changes. Independent affected build/typecheck/narrow/camera execution is pending; halo has not duplicated it.

Affected cases: choose an explicit overview zoom/pan, select, read/compose and close; scene→chat→scene while reading and after closing; narrow/desktop resize with selected and unselected camera; explicit panorama from an open reader; short 390×480 and 320×480 scene composer after mode return. Reuse unchanged evidence for unaffected local-note/filter/history behavior unless a changed path exposes a new failure.


## Final implementation handoff to zumi

The original user outcome is usable as an isolated offline Web module: actual transplanted pixel art/rooms/furniture/animation, selectable scene and IM presentations, object-anchored expandable conversation windows, readable Markdown/tool/reasoning history, exact-person drafts and explicit unsent local notes. Ended history remains discoverable. Selection preserves chosen world scale; scene/chat return, close, panorama and resize retain coherent camera/draft state. Short 390×480/320×480 scene readers keep input and append fully reachable.

Canonical entry points: [module guide](../../web/agent-space/AGENTS.md), [run commands](../../web/agent-space/README.md), [notices](../../web/agent-space/NOTICE.md), [exact source inventory](../../web/agent-space/public/notices/migration-inventory.json). Gabi's actual API design remains independent: future integration replaces the explicit supply at App composition; no transport contract, fake delivered message or simulated Agent reply was introduced.

Mofe's completion message #20323 and [final verification](verification.md#final-independent-result--completed) establish the documented Linux local Chrome acceptance. Final artifact `/tmp/agent-space-acceptance-mofe-final-20261115` passed build/typecheck after the four-file repair. JS SHA-256: `4f8227c5dc82a509bb91c236065449eadb94a145b932f7008b80b88bcab6add1`. Final input source hashes matched at executor handoff; this closeout changes only implementation.md. Durable evidence and runnable bindings: [tests README](../../web/agent-space/tests/README.md), [evidence directory](../../web/agent-space/tests/evidence/2026-11-15/).

No runtime exception or external/API request was observed. Optional browser-generated local favicon404 remains documented. This result does not claim real Agent delivery, persistence across reload, cross-browser certification or physical mobile keyboard/IME testing. Mofe cleaned up its loopback :18968 servers and browsers. Halo started no processes. Preserve zumi's user LAN preview :18967/PID3050612; no deployment or service cleanup is required from this assignment. Root discoverability is controller-owned and already linked. No unmerged root/dependency work or unresolved source repair remains.
