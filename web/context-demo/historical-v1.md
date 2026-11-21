# Historical v1 composition — rejected

The wrapper picker/attachment controls and reader-only query composition below were rejected by the user on 2026-11-15. These instructions and prior receipts are historical provenance, not current acceptance or the correction design. See [current design](README.md) and [correction Task](../../.controller/context-clients/demo.md).

# 订单协作台 · 悬浮 AgentX 示例

一个无需安装依赖的中文售后场景：查看订单异常、选中文字或附加截图/文件，把上下文加入已选择 Agent 的草稿，再在真实对话里审阅并发送。订单是固定演示数据；Agent 列表与对话来自配置的 AgentX 服务。

## 运行与复用

需要 Node.js 20 或更新版本。从仓库根目录运行：

```sh
AGENTX_ORIGIN=http://192.168.2.178:38023 HOST=0.0.0.0 PORT=38024 node web/context-demo/server.mjs
```

用其他空闲端口和独立 AgentX fixture origin 即可隔离验收。`AGENTX_ORIGIN` 必须是无路径的 HTTP(S) origin，由启动者配置；页面不能用任意 URL 改写服务器代理。更换配置须启动独立实例，不会重启 AgentX。`EXTENSION_ZIP=/absolute/exact/package.zip` 可配置唯一下载文件；`EXTENSION_META=/absolute/metadata.json` 可配置包元信息。未提供可用 ZIP 时下载入口明确显示未就绪。

在别的网站复用：保留 `bridge.js` 的 source/origin 校验、精确 target 和请求回执，把该站点的标题、文字、HTTP(S) URL 与 File 数组交给 `bridge.add(context)`；嵌入 `/space/` 的 URL 由实际 descriptor/member 读取构造。跨源 API 读取由本站服务器的限定 discovery 端点完成；iframe 直接访问 AgentX，同源上传和最终发送都归子页面。

## Design and ownership

Scenario: a support operator investigates order AX-20261115-0182 (partial delivery and a reported damaged coffee grinder). The order list selects a coherent subject; details show customer claim, purchased goods, shipment events and refund policy. The primary contextual actions request an exception summary or a response draft, capture page selection, and attach actual files. No fabricated AI output is rendered.

Layout: top product header/download, left order queue, central selected-order detail, bottom-right collapsed launcher. Expanded nonmodal conversation floats above the page and retains the iframe when hidden. Desktop width is 460px with viewport-bound height; narrow screens use available width/height without remount. Close restores invoker focus; parent Escape closes the overlay. Escape within cross-origin child cannot bubble to parent; always-reachable parent close control remains available.

| Owner | Contract |
| --- | --- |
| `public/scenario.js` | Immutable illustrative order facts, selected-order rendering and context text. |
| `public/app.js` | Selection, file intake, discovery selection, overlay and status; no native mutations. |
| `public/bridge.js` | `ContextBridge(iframe, origin, onState)`, `setTarget(target)`, `add(context)`, `dispose()`; canonical `agentx.context.v1`, unique correlated request IDs, exact source/origin. |
| `server.mjs` | `createDemoServer(config)` and executable runner; finite GET discovery, no unrestricted proxy, allowlisted assets, one exact ZIP. |
| Existing AgentX `/space/` | Per-target draft/uploads, scope/incarnation fencing, deliberate send and real answers. |

Discovery: `/api/discovery` reads `/api/v1/management`, then bounded `/api/v1/agents?generation=...&limit=...&after=...`, stopping at current-complete or 20 pages. It returns only descriptor identity/limits and current sendable member labels/exact references. No history or transcript is proxied. It reports incomplete coverage, unavailability and generation mismatch. All identities stay strings. A refresh preserves selection only for the same complete exact reference, never namesakes; missing targets disable add until an explicit selection. Target changes navigate the iframe and clear parent pending files, visibly disclosing that the previous child draft is not retained across target changes. Collapse alone retains the iframe and draft. No cross-target context queue exists.

| State/event | Result and recovery |
| --- | --- |
| No selection / discovering | No add; choose current real member after discovery. |
| Iframe loading → ready | Hello retries; ready means bridge installed, not data-loaded. Child rejection remains visible and user may retry after data loads. |
| Add clicked | Snapshot exact target and context; disabled while pending; no automatic retry or send. |
| Correlated added/error | Report draft addition or actionable error; successful files clear, failed files remain. |
| Unknown outcome after timeout | Keep input, report unknown; no automatic resend. Refresh/reload/target change loses child-local drafts. |
| Collapse/reopen | Preserve child document, pending operation and draft; restore focus on close. |

Security/lifetime: GET-only discovery adapter has a fixed configured origin and redirect rejection, finite page/size/time bounds. Assets are exact allowlisted paths; no repository directory server. Download serves one configured ZIP with attachment disposition; no directory listing. Parent page does not store drafts/files in localStorage. Parent file intake enforces 8 files and 10 MiB/file; child/native remain authoritative. LAN HTTP is allowed by the existing canonical contract; browser embedding restrictions remain native/browser-owned.

## Focused verification and independent consumption

Development check: `node --test web/context-demo/server.test.mjs` covers fixed-origin discovery, pagination/current filtering, forbidden mutation/proxy/static paths, and exact ZIP serving. Syntax checks cover browser modules. Independent actual-browser acceptance belongs to [acceptance Task](../../.controller/context-clients/acceptance.md); use fixture-only sends. Stable selectors: `#launcher`, `#assistant`, `#close-assistant`, `#target`, `#refresh-targets`, `#ask-summary`, `#ask-reply`, `#ask-selection`, `#attachments`, `#add-files`, `#bridge-status`, `#agent-frame`, `#download-extension`. Frame is created on explicit target selection; ask actions never enqueue while unavailable.

## 当前 LAN 实例

打开 **http://192.168.2.178:38024/**。右下角「Agent 协作」展开，选择真实当前 Agent；点击「分析异常」或「拟一份客户回复」，内容只进入草稿，在对话里确认后发送。选中订单页面文字后可点击「选中文字」；「图片 / 文件」可选取附件，也支持在底部区域拖入或粘贴图片。收起后重开保留同一子页和草稿。连接详情中可查看当前精确身份和服务地址。

[launch receipt](evidence/launch.json) records retained PID 1278016, detached process session, command, log and native-service preservation. [LAN read-only smoke](evidence/lan-smoke.json) records HTTP/discovery and exact ZIP retrieval. Log: `web/context-demo/evidence/server.log`. This process is intentionally retained; only stop the recorded PID after verifying `/proc/PID/stat` start ticks match the receipt. It has no AgentX service ownership. The fixed configured package path may be repaired by the extension owner before final acceptance; package real-host acceptance remains recorded by the extension/acceptance Tasks. The [extension owner download check](../../.artifacts/context-clients/download-check.json) independently confirms HTTP 200, exact 15,930 bytes and pinned package digest on the retained LAN route.

页头「Chrome / Edge 扩展」可下载 ZIP。解压后，在 `chrome://extensions` 或 `edge://extensions` 打开开发者模式，选择「加载已解压的扩展程序」，加载含 `manifest.json` 的目录；详细使用步骤见包内中文 README。当前公开下载的精确版本及校验值见 [包下载验证](../../.artifacts/context-clients/download-check.json)。

Source changes are served without a build. A browser reload picks up changed browser assets and loses child-local draft; server code/config changes require replacing only this owned demo process. To restart, first verify the receipt identity, stop only that process, then use the recorded command with stdout/stderr redirected to an owned log and a detached session. Never stop the :38023 listener.

Independent actual-browser acceptance: [Task](../../.controller/context-clients/acceptance.md), [consumer tests](../../tests/context_clients/README.md). [demo-01 result](../../tests/context_clients/evidence/demo-01/demo-result.json) is independently passed on Linux real Chrome with a disposable accepted-r3 fixture: retained floating iframe/draft/focus, parent keyboard close/reopen, page title/text/source and real image/file draft additions, explicit exact-recipient send with native Succeeded receipt, 390px controls and failed-discovery recovery. No live user Agent received test messages. [source receipt](evidence/source.json) matches the browser/native source bytes exercised independently. This finite evidence does not claim physical clipboard/drop, every stale-target combination, Edge execution or automatic AI image ingestion.

[Layout observation](evidence/layout.json) and [desktop](evidence/desktop.png)/[narrow overlay](evidence/narrow-overlay.png) capture the actual page at 1280×900 and 390×844. The desktop keeps three orders, customer claim, purchased items, recent shipment records, rule and three contextual actions grouped around the selected order; narrow overlay controls remain within the viewport (374×764 at x8/y8) with focus restored on close. These author observations selected no live target and performed no mutation; independent fixture evidence above owns actual conversation behavior. The initial capture script waited for visibility of a native option inside a closed select and timed out; correcting its wait to DOM attachment resolved the observation harness without product changes.
