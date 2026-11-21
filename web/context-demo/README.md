# 订单协作台 · 悬浮 Space 示例

打开 **http://192.168.2.178:38024/**，从右下角展开完整 Space。办公室场景／列表切换、成员导航、原生附件和对话都使用同一套 Space UI。父页面只提供演示订单、页面上下文捕获和浮层开合。

## 使用

在 Space 内选择成员并打开消息对话。点击订单中的「分析异常」「拟一份客户回复」或选中订单文字后点「选中文字」，捕获内容会关联当前精确对话；无可用选择时，内容保留在父页面。在 Space 选择成员后点「关联当前对话」，再点「加入草稿」。最后在 Space 中审阅并主动发送。切换成员不会自动重投，失败内容仍保留；图片和文件直接使用 Space 的原生附件入口。收起再展开保留同一 iframe、草稿和捕获内容。

订单是固定演示数据，Agent 和对话来自真实配置服务；页面不模拟 AI 回复，也不自动发送。

## Chrome / Edge 扩展

下载 [AgentX 扩展 1.2.1 ZIP](http://192.168.2.178:38024/downloads/agentx-browser-context.zip)，解压到一个固定目录。在 `chrome://extensions` 或 `edge://extensions` 开启「开发者模式」，选择「加载已解压的扩展程序」，加载包含 `manifest.json` 的目录。固定工具栏中的扩展图标，在需要捕获的普通网页点击该图标打开侧栏；配置 AgentX 服务，在完整 Space 中选择对话，再捕获选文、页面或截图并加入草稿。手动附件和最终发送使用 Space 原生操作。详细步骤见 ZIP 内中文 README。

当前包：1.2.1，17,042 字节，SHA-256 `495717f7044b023b0195fee1bfc81871a2bd921a8e3593560f2fad47e52d734e`。真实 Chrome 扩展 action → native sidePanel → 页面/选文/截图 → 实际 iframe 草稿 → 主动发送由[独立验收](../../tests/context_clients/evidence/extension-full-r1-121/extension-full-result.json)记录；未另行声称实际 Edge 执行。受限制页面会显示失败，捕获期间切换服务不会自动发送到新目标。

## 运行与服务交接

需要 Node.js 20+，无额外依赖。从仓库根目录运行：

```sh
AGENTX_ORIGIN=http://192.168.2.178:38023 HOST=0.0.0.0 PORT=38024 EXTENSION_ZIP="$PWD/.artifacts/context-clients/agentx-browser-context-1.2.1.zip" EXTENSION_META="$PWD/.artifacts/context-clients/agentx-browser-context-1.2.1.receipt.json" node web/context-demo/server.mjs
```

`AGENTX_ORIGIN` 是无凭据、路径或查询参数的 HTTP(S) origin，只由启动者配置。独立验收可改为隔离 fixture origin 和空闲端口。省略扩展变量则隐藏下载；同时指定 ZIP 和 receipt 时，runner 启动前检查摘要、版本和字节数，并把固定包字节保留在内存，唯一下载路由不暴露目录或其他包。

当前演示 PID **1682783**、start tick **978765996**，持续保留；[当前进程 receipt](evidence/launch-download-121.json)记录精确命令、配置、替换边界和日志。此前 PID 1585864 经身份核验后仅被此进程替换。native `:38023` PID 1563302/start 978621828 由 Space owner 采用 r1 后保持未变。日志为 `web/context-demo/evidence/server-download-121.log`。停止或替换时先核验 PID、命令和 start tick，只处理自有演示进程，不能重启 native 或其他监听器。重载页面或重新连接可能丢失子页草稿；收起浮层不会。

## Design and source ownership

Authoritative scope: [demo Task](../../.controller/context-clients/demo.md), [Delivery](../../.controller/context-clients/delivery.md), [canonical bridge](../../.docs/agent-management/web-context.md) and [one-Space responsive design](../../.docs/agent-space/responsive-space.md). Current composition replaces the rejected wrapper selector/attachment UI and reader-only Space fork. Historical design and evidence remain in [v1 provenance](historical-v1.md), never as served implementation.

| Owner | Responsibility |
| --- | --- |
| `public/scenario.js` | Three illustrative orders: customer claim, items, shipment events and policy; immutable context source. |
| `public/app.js` | Order selection, synchronous page capture, explicit capture binding/add, feedback and floating lifetime. No native reads or member/attachment controls. |
| `public/bridge.js` | One full `/space/` iframe without compact-layout or member query. Origin/source/request correlation, installed-only hello/ready, getTarget binding and fenced addContext. |
| `public/layouts/workspace.css` | Order workspace and responsive floating host; child layout remains canonical Space-owned. |
| `public/style.css` | Existing Chinese page tokens and leaf treatment. |
| `server.mjs` | Fixed origin configuration, allowlisted static assets, one verified immutable ZIP; no native discovery proxy or directory server. |
| Canonical Space | Office/scene/list/member navigation, exact selection revision, drafts/uploads/attachments and deliberate send. |

Parent capture snapshots title/text/HTTP(S) source and obtains the child's current exact target and opaque `selectionRevision`. Missing selection retains content. An explicit association creates a binding; add sends exactly that binding. Space fences admission and commit, including away-and-back selection changes, identity/capability loss and stale generation. Results carry the actual accepted target/revision for feedback. Error clears the binding while retaining content; explicit re-association is required. No polling, default target, broadcast-triggered binding, retry or namesake fallback exists. Unique IDs correlate requests; timeout is unknown outcome, requiring inspection of the original draft before another deliberate add.

The overlay is nonmodal, stays within desktop/narrow viewports and retains one child document through hide/show. Parent Escape and close restore invoker focus. Key events in a cross-origin iframe cannot bubble to the parent; its parent close control remains reachable. Reload/reconnect is disclosed and never silently rebinds pending captures.

For reuse in another site, supply that site's title/text/source to the same parent bridge API. Native selection and manual files stay inside full Space. Keep source/origin checks, exact binding/revision and correlated outcomes; no editable identity JSON is required.

## Verification and evidence

Focused host checks: `node --test web/context-demo/server.test.mjs`; bridge checks: `node --test web/context-demo/bridge.test.mjs`. The download addition changed only server behavior: the three host checks cover no native forwarding/static boundaries, unavailable download, verified exact package metadata/bytes and rejected digest. Accepted public wrapper asset hashes remain unchanged in [final source receipt](evidence/download-121-source.json).

[Canonical adoption receipt](evidence/launch-canonical.json) and [canonical read-only smoke](evidence/canonical-smoke.json) are historical records of the full-Space promotion and temporary download withdrawal. [Current package smoke](evidence/download-121-smoke.json) supersedes that withdrawal: GET 200 with exact accepted 1.2.1 bytes/hash and versioned attachment filename. [Independent final LAN binding](../../tests/context_clients/evidence/final-live-binding-121.json) passed: exact 1.2.1 download/config, six served wrapper assets matching the accepted corrected demo, native/demo process identities and clean disposable fixtures. [Final package check](../../tests/context_clients/evidence/package-final-121.json) owns package parity. Consumer evidence and closure remain owned by [acceptance](../../.controller/context-clients/acceptance.md); original finite evidence does not claim every physical clipboard/drop/stale-target combination or Provider image ingestion.

Stable selectors: `#launcher`, `#assistant`, `#close-assistant`, `#agent-frame`, `#ask-summary`, `#ask-reply`, `#ask-policy`, `#ask-selection`, `#capture-summary`, `#capture-title`, `#rebind-context`, `#add-context`, `#clear-capture`, `#bridge-status`, `#settings-toggle`, `#reconnect`, `#download-extension`. No parent target/attachment selector remains. Automated sends belong only in disposable native fixtures; final LAN checks are read-only.
