---
schema: document
status: active
owner: rerove
related: [../../web/agent-space/AGENTS.md, ../../.controller/context-clients/space-responsive.md, ../agent-management/web-context.md]
---

# One complete Space in embedded containers

## Recovered foundation and diagnosis

Space is the transplanted pixel office and IM reader, not an independent compact app. `src/main.tsx` imports `source.css` (foundation spacing, type, form and reader primitives), `scene/office.css` (warm paper/ink, office geometry and controls), `scene/hall.css` (directory and reader), `conversation/history.css` (record rendering) and `space.css` (canonical shell and responsive adaptation). Original artwork and source provenance remain in `NOTICE.md` and `public/notices/`. No new palette, component library, UIUX resource collection or second workbench is needed. UIUX design-system/method/components/tokens/scenes and usage guidance were read; the existing actual public App with explicit `?supply=demo` supplies the repeatable composed workbench. No UIUX preview or schema validation is claimed for this non-UIUX-managed existing system.

`App` owns one SpaceController plus per-office scene/capacity memories. `OfficeLayout` owns header, shared overview and fill-height stage. `SpaceLayout` owns desktop office tree versus working content. `SceneSpace` owns world/camera and object-anchored reader; `ChatLayout` owns roster versus reader. `ConversationLayout` owns heading, local feedback, independently scrolling history and composer. Conversation, Directory, OfficeResources and native contextual actions are canonical Blocks. Native data and operations remain LiveSpace-owned.

The rejected `EmbeddedLayout` bypasses header, office overview, resources and scene/chat mode. Its query-selected return path is the structural defect. Existing narrow rules already reuse office select instead of desktop tree and roster/reader disclosure, but fixed header sizing plus inline native error/receipt content can collide; overview filters can overflow into unrelated controls; chat has a desktop roster assumption based only on outer viewport; unbounded composer/feedback can consume history. Receipt panels anchored to their summary can leave narrow frames. These are repaired at existing layout/component style owners, without changing world geometry or selecting a replacement page.

## Structural skeleton before implementation

```text
App (single return; no layout query branch)
  OfficeLayout
    header: brand | scene/chat | native connection/receipt disclosure
    overview: office select + search; member/post/Skill/history + local filters/contact
    SpaceLayout
      office tree (desktop; select remains same authority at narrow widths)
      working content (container boundary)
        SceneSpace + existing object reader OR ChatLayout roster/reader OR OfficeResources
          ConversationLayout: identity / channels+actions / scrolling history / bounded composer
```

At 320/360/390/450px the same header and two overview groups remain available. Native connection details/errors/receipts disclose locally, with a visible concise failure cue. Office select exposes all known offices; resource tabs stay visible; occasional filters/contact/resume may wrap. Working-region width, not parent viewport alone, decides roster/reader disclosure. At narrow width only the selected reader or roster occupies the body; reader return retains selection/draft and restores the selected roster control. Scene selection uses the existing object reader and camera behavior. Short stages retain the existing full-stage reading geometry. Header and overview never scroll away; body owns its roster/resource scroll; history owns record scroll; composer and disclosed feedback are bounded independently. Long names wrap/truncate only where complete identity remains available in existing details. Long errors and receipts wrap and scroll inside their local panels. No global scale transform or clipping-based compact mode.

App-owned exact drafts, pending attachments and per-office cameras survive mode/resource changes. Resizing changes layout only. Focus on explicit chat select moves into the reader; explicit return restores the selected member. Existing scene focus and Escape/return are retained. Selection/view changes do not send or upload anything.

## Minimal selected-context bridge for wrapper owners

Canonical protocol remains `agentx.context.v1`, parent-only messages and correlated `requestId`, no new HTTP endpoint or member polling. `hello` → `ready` means listener installed only. New `getTarget` → `{type:'target', status:'selected', target, selectionRevision, title}` when an exact current or stopped conversation is actively open. This is read-only selection observation; it does not establish message/capture capability. `addContext` separately requires the current messages channel with native can_send available and unblocked. Otherwise `{type:'target', status:'unavailable', error}`. `target` uses the existing scope/generation/AgentRef shape. Revision is an opaque page-local selection token; clients do not parse it.

A deliberate capture obtains getTarget once before asynchronous capture, retains that target/revision, then sends `{type:'addContext', requestId, target, selectionRevision, context}`. Space validates exact current selection and revision at admission and immediately before committing uploaded references/text. Leaving/returning to that selection also invalidates the old revision; changing presentation alone preserves it. Capability loss or identity/generation change blocks commit. Success is `{type:'result',status:'added',target,selectionRevision}`; failure retains `{type:'result',status:'error',error}` and never silently retargets or retries. Legacy explicit-target calls without revision remain compatible but are still bound to the current selection at admission and commit. Deduplication is page-local by requestId; retries with that same ID retrieve the original result. Wrapper closes must retain iframe lifetime when retaining a pending capture. No target-change broadcast or background discovery loop is needed.

SpaceController.addContext gains an optional synchronous pre-commit guard after upload and before draft/reference mutation; native attachments use the same canonical upload path. Failed guards may leave unreferenced native upload bytes but never append text/references to another target or move the reader. Wrappers retain failed capture material and request another deliberate user action. Success does not send a message. Parent must use exact source/origin correlation and keep source identity with asynchronous capture. Full protocol baseline remains linked above; this scoped extension is owned beside actual source at `src/live/context.ts`.

## Execution and evidence boundary

Implement structure/styles/bridge, run scoped typecheck and focused bridge/state regression once. Independent client acceptance owns true narrow iframe/extension operation, deliberate context, long content/failure and desktop regression; screenshot alone is insufficient. No production/LAN restart, new daemon, native rebuild or live polling is authorized here. Existing generated embedded bundles may still contain the deleted source fork until controller-owned candidate adoption. Demo/extension owners inventory and remove their old wrapper artifacts. Historical acceptance records remain historical.


## Accepted implementation and running contribution

The [completed responsive Task](../../.controller/context-clients/space-responsive.md#completed-selected-running-outcome) owns actual evidence: same full Space narrow iframe and corrected demo independently passed; accepted r1 is adopted at `http://192.168.2.178:38023/space/`. One authorized native same-scope restart retained persisted generation, all14 served assets match the immutable package, and actual read-only1440/360px scene/chat/office/roster smoke passed with no API/page errors or attempted mutations. Independent disposable evidence owns selected-conversation/draft/capture/upload races. The browser extension optional-permission/capture gap remains separate and is not certified by canonical Space acceptance. This replaces the execution section's earlier pending-adoption state, without rewriting historical evidence.
