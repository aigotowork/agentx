# Browser host and conversation design

> Current 1.2.1 source replaces the rejected first-candidate wrapper UI. It embeds the same full responsive Space, with service configuration and webpage capture only. Historical 1.0.0 package evidence stays with [owned Task](../../.controller/context-clients/extension.md); current child support is owned by [responsive Space](../../.docs/agent-space/responsive-space.md).

Owner: @zanota. Scope: dependency-free Chrome/Edge MV3 side panel, unpacked package. Canonical conversation protocol: [web context](../../.docs/agent-management/web-context.md). No native or running service changes.

## Current structure and responsibilities

The same full Space is the only office/member/conversation/attachment interface. `panel.html` composes Connection disclosure → webpage Capture toolbar → local status → retained full Space iframe. `panel.js` owns service/document epochs and capture actions; `capture.js` owns Chrome active-tab snapshot/capture; `bridge.js` owns ready/correlated context requests. `service.js` only validates configurable HTTP(S) service origin; the wrapper never polls native Agent data. `background.js` configures toolbar → sidePanel using the public host API. `package.py` creates deterministic ZIP/unpacked payloads with SHA-256 receipts. Local CSS tokens own only narrow host chrome; Space owns its own responsive design.

```mermaid
sequenceDiagram
    participant U as User
    participant P as Extension side panel
    participant C as Chrome APIs
    participant F as Full /space/ iframe
    U->>P: Connect configured service
    P->>F: Load /space/ (no compact query); hello
    F-->>P: ready
    U->>F: Navigate office/scene/list/member
    U->>P: Capture selection/page/screenshot
    P->>F: Snapshot selected exact current target (contract coordinated)
    F-->>P: Selected identity / current-selection fencing value
    P->>C: Bound active-tab read / visible capture
    C-->>P: JSON text or bounded data URL
    P->>P: Verify service/document/tab; reconstruct screenshot File
    P->>F: addContext with original exact-selection fence
    F-->>P: Correlated added/error
    U->>F: Review attachments/draft and deliberately send
```

The canonical current-selection contract is owned by @rerove in [responsive Space](../../.docs/agent-space/responsive-space.md). `getTarget(requestId)` returns `type:target,status:selected,target,selectionRevision,title` or `status:unavailable,error`. Ready means listener installed only. Older `result/error` responses distinguish unsupported bridges. Parent snapshots once before browser I/O and sends unchanged target/revision in addContext; child checks admission and pre-commit, including selection departure/return. Added result must return the exact target/revision. No parent polling, selector or blind current-target fallback.

## Current state and invariants

| State/event | Effect |
| --- | --- |
| Connect changed service | Advance document/service epoch, discard waiting capture; load full `/space/`. No Agent discovery/read gate in parent. |
| Same service Connect | Preserve iframe, member selection, camera and draft. Connection settings remain local disclosure. |
| Member/office/scene/list navigation | Child only, preserving same complete Space composition and exact-instance drafts. |
| Capture | Snapshot exact child selection before browser I/O; service/document/tab/selection fence prevents retargeting at admission/commit. One pending context operation only. |
| Bridge unready | Controls disabled, hello retries; no silent context queue. |
| addContext sent | Unique request ID; timeout retains original envelope for same-document same-ID retry, never fresh automatic resend. |
| Child document reload | Invalidate original request retry because child dedup cache lifetime ended; ask user to inspect draft. |
| Failed captured context | Keep last failed/uncertain capture copy locally, with copyable text or screenshot download in collapsed disclosure. No automatic rebinding/retry on selection change; user discards or explicitly recaptures. |
| Wrong/repeated result | Check exact iframe window/origin/request; accept first matching result once. |
| Attachment intake | Child's canonical composer only. Screenshot File originates in parent then structured-clones directly to child, no Chrome runtime File hop. |
| Restricted page/permission failure | Visible recoverable error; toolbar invocation grants activeTab and can renew it. |
| Native503 / daemon busy | Child native data owner shows actual endpoint/body and finite read recovery. Removing redundant parent Agent polling reduces duplicate work but is not a native overload fix. |

No content scripts, runtime binary message hops, wrapper Agent selector/file picker, generic network proxy, remote extension scripts, automatic send or new-Agent operation. Optional selection context menu omitted; core toolbar capture completes the flow.

## Host contract evidence

Official sources, checked 2026-11-15:

- [sidePanel](https://developer.chrome.com/docs/extensions/reference/api/sidePanel): `sidePanel` permission; `setPanelBehavior({openPanelOnActionClick:false})` preserves normal action dispatch; a synchronous `action.onClicked` handler calls `sidePanel.open({windowId:tab.windowId})` directly within the action gesture. No await precedes open. Earlier automatic panel behavior opened the real panel without exposing activeTab metadata on the observed host; 1.2.1 explicitly owns the standard action path. Minimum Chrome 116 (Edge Chromium sidePanel availability must be exercised separately).
- [activeTab](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab), [scripting](https://developer.chrome.com/docs/extensions/reference/api/scripting): toolbar invocation grants temporary host access; `executeScript` needs scripting + activeTab or host permission. Injection targets main frame and reads bounded rendered text/selection only on explicit click.
- [tabs.captureVisibleTab](https://developer.chrome.com/docs/extensions/reference/api/tabs#method-captureVisibleTab): activeTab permits visible-tab capture, returns data URL. Snapshot must still match active tab/window/URL after capture. No entire document screenshot claim; PNG bounded at 10 MiB decoded.
- [Messaging serialization](https://developer.chrome.com/docs/extensions/develop/concepts/messaging#serialization): Chrome runtime messaging uses JSON serialization. Files are never passed there. Screenshot data URL is decoded and reconstructed as File inside the side-panel parent; iframe `postMessage` uses structured clone.
- [MV3 CSP](https://developer.chrome.com/docs/extensions/reference/manifest/content-security-policy): local external JS, no eval/inline script; extension script-src self. Explicit `frame-src http: https:` permits configurable service frame. Remote child retains its own CSP, not extension execution privileges.
- [Cross-origin requests](https://developer.chrome.com/docs/extensions/develop/concepts/network-requests): extension pages can fetch hosts covered by host_permissions. The parent does not request host access or poll native APIs; configuring the origin only controls the iframe URL. activeTab alone is not arbitrary service API permission. The original native Agent-fetch boundary was independently exercised in 1.0.0 host probes, but current wrapper performs no native Agent reads. Only iframe loading and activeTab page capture remain actual-host concerns; no service permission prompt gates Connect.

Permissions: `sidePanel`, `activeTab`, `scripting`, `storage`; No host permission is requested: the parent performs no service fetch; `frame-src http: https:` loads the full Space, whose document owns same-origin API reads. No tabs, all_urls install-time grant, cookies, clipboard, downloads, debugger or webRequest. Storage holds service URL only, no draft/files/Agent identities. CSP permits HTTP LAN frame/connect intentionally; actual host mixed-content/network behavior is acceptance-owned.

## Artifact and finite checks

ZIP root contains manifest/local JS/CSS/HTML and Chinese README/design/AGENTS only; source packaging script remains source-only. Fixed ZIP timestamps/order/mode, no caches/user data. Run `python3 extensions/browser-context/package.py`; artifact receipt records source hashes and ZIP hash. Independent @lodize exercises actual unpacked host from exact package: toolbar panel, full-Space office/scene/list/member navigation, selection/page/screenshot draft and child-local file intake, restricted pages, stale target/unready/duplicate/error, narrow layout and no automatic sends. Implementer runs syntax/package consistency and scoped host probes only; neither static checks nor iframe readiness proves native loading or successful context acceptance.
