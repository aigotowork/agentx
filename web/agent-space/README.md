# Agent 空间

## 画布内会话工作台

点击画布人物或「名册」打开会话。宽屏最多并排显示两个会话；点击频道右侧的聚焦、恢复并排、最小化或关闭按钮调整当前工作区。窄屏保留单窗与会话栏，点击会话栏即可切换或恢复。会话草稿和已读位置按精确实例保留。「启动关系」在 Agent 名称旁，可查看谁启动了该 Agent、它又启动了谁。新建 Agent、岗位和技能在当前工作区内展开。「显示与反馈」里可以换风格和开启窗口音效，默认静音。

支持最新页的服务会从当前进展打开。向上翻页时，原消息留在原处；有更新时可点「回到最新」。加载失败可以就地重试。「查找已读入记录」只搜索已加载的内容。旧服务按时间从最早读取，页面会说明无法直接跳到最新。草稿框与附件、更多选项和发送共用一个外框；长稿会增高，可拖动上缘或通过「调整草稿高度」改变高度。普通回车换行，发送用按钮或 Ctrl/⌘+Enter。

鼠标停留、键盘聚焦或触屏轻点「启动关系」可查看上下游；Esc 或点击外部关闭。有明确来源的 Fork 会另外显示来源实例。支持 Fork 的会话在头部提供「Fork 新 Agent」，确认后仍留在原会话，收到精确回执才可选择打开分叉会话。

当前激活的精确会话可以分享链接；其他窗口和草稿属于当前页面。设计与验收见[会话工作台交付](../../.controller/space-conversation-workbench/delivery.md)。

## 切换风格

打开 AgentX 原生服务的 `/space/`，在页头选择「显示与反馈 → 界面风格」。原版像素办公室、工坊信笺、夜班联络台、图录工作室和其余十个场景都在同一个菜单。切换保留当前办公室、会话和草稿，可在场景或聊天中使用。

浏览器会记住上次选择。分享指定风格时，用同一页面的 `style` 参数，例如 `?style=studio`。已有 `art=studio` 链接仍可打开；主动切换后地址使用新的 `style`。风格清单、产品范围与发布状态见[正式合并记录](../../.controller/space-ui-styles/formal-adoption.task.md)。

## 工作区 SDK 接入

原生服务托管的 `/space/` 默认启用面向对象 SDK。原生页面提供预期作用域，页面使用当前监听地址并确认服务代次。独立托管或本地开发连接真实服务时，由宿主明确提供管理地址和预期作用域：

```sh
VITE_AGENT_SPACE_MANAGEMENT_URL=http://127.0.0.1:19741 \
VITE_AGENT_SPACE_EXPECTED_SCOPE=/absolute/disposable/.agentx \
node web/agent-space/scripts/run.mjs build /tmp/space-sdk-build-unique
```

请使用监听器实际公布的原生作用域，并由宿主独立配置该值；首次响应只用于校对和锁定代次。这里的 URL 和作用域都是占位示例。Vite 会将这些公开配置写入静态产物；更换配置需重新构建（开发服务器需重启）。稀疏分页测试可额外设置 `VITE_AGENT_SPACE_PAGE_LIMIT=1`；指定办公室时默认每页读取至多 20 条，并遵循监听器的上限；全部办公室、旧记录和归属未知使用同一个 SDK 的作用域名册。配置无效时页面提供「重试连接」；已锁定的监听器代次改变后，点「重新连接」进入新页面会话，开始新的办公室选择。

在已知办公室或「全局成员」中，「加载更多成员」继续原扫描。某页可能没有这个办公室的成员，仍可继续。场景和聊天名册共用同一批已观察成员。「历史」可打开停止记录；它们不出现在场景人物里。精确深链可读取尚未翻到的成员，但不会把它计入名册。切换场景与聊天保留当前会话、草稿和阅读位置；在同一页面返回再打开同一实例，草稿仍在。草稿不会随链接分享。成员发现、精确消息与现有发送回执各有自己的状态，另见 [Space Task](../../.controller/space-domain-sdk/tasks/space.task.md)。

## 开发与验证

纯前端样例和早期评审版本见[历史与复现记录](../../.controller/space-ui-styles/formal-adoption.task.md#historical-material)。画布调试使用[检查工具](src/scene/debug/README.md)，手绘源文件及放置约定见[素材目录](src/scene/art/studio/AGENTS.md)。


默认 Web 已内置 DancingAI Inspection。从页面顶部「显示与反馈」展开「检查」，打开工具，再展开工具栏 → **React / Runtime Inspection** → **Agent Space** 来源 → **点选页面对象**，在画布人物或家具上移动查看边界，点击后查看数据。点选仅用于检查，不打开人物对话；列表折叠在「浏览对象」里作辅助查找。右上波形按钮开启 200ms 实时采样，暂停后保留数据；展开字段详情也会实时更新。原「对象与数据」入口保留。演示请显式使用 `?supply=demo`，既有 :38023 服务未替换。展开后点击左上角「收起检查」可收起工具；默认保持检查提供器注册并隐藏浮动工具，避免右下工具栏遮挡消息发送。

接入使用公开 `window.dancingai.inspection.register(provider)`。精确对象快照及操作说明来自原 `agentxSceneDebug`；只有 Canvas 根节点的原生对象是这个真实门面，人物/家具快照不伪装成可修改实例。Agent 应先读根节点 data 中的 canvasId，再通过原门面 describe/list/invoke/waitForFrame 验证操作后的实际帧。截图仅含 Canvas，排除 DOM 标签、阅读窗和工具 UI。详细说明见[场景工具](src/scene/debug/README.md)。

`public/tools/dancingai/distribution.json` 记录固定版本及原始产物、提交到仓库的运行文件各自的大小和 SHA256。刷新时先构建对应 DancingAI Page 包，再运行 `node web/agent-space/scripts/vendor-dancingai.mjs /absolute/dancingai-checkout`。普通 AgentX 构建直接使用仓库中的运行文件。

接入回归：`node --experimental-transform-types --test web/agent-space/tests/scene-debug.test.mjs`。真实浏览器入口为 `tests/dancingai.browser.mjs`，通过 `AGENTX_PLAYWRIGHT_PACKAGE` 指定现有 Playwright 包、`AGENT_SPACE_URL` 指定带 `?supply=demo` 的页面；脚本拒绝在真实供给上执行场景操作。已验证静态 `/space/` 构建、61 个对象、原生方法/参数、调用与后续帧、真实 PNG、路由换代、聊天往返、前进后退及 390px 面板交互。带持久反馈服务的预览：<http://192.168.2.178:18976/space/?supply=demo>。浮动工具栏可通过移动手柄避让页面导航。

从原 Web 应用移植的独立离线前端：像素办公室、场景人物、IM 名单、锚定会话窗口和本页草稿共用一份前端状态。保留原始人物、家具、墙面、休息区和动作素材，并保留会话中的思考/工具折叠与 Markdown 阅读。

在仓库根目录运行（复用已有锁定依赖，无需修改根 manifest）：

```sh
node web/agent-space/scripts/run.mjs dev
# http://127.0.0.1:18967
node web/agent-space/scripts/run.mjs typecheck
node web/agent-space/scripts/run.mjs build /tmp/agent-space-build-unique
python3 -m http.server 18968 --bind 127.0.0.1 --directory /tmp/agent-space-build-unique
```

构建目录必须是不存在的绝对路径，且位于模块源码之外。构建会执行 scoped typecheck，随后生成可静态托管的页面、脚本、CSS、原始来源清单及完整许可。脚本通过[共享加载器](../vite.mjs)导入 Vite：Windows 使用原生 Rolldown 绑定，其他平台使用 WASI。不会构建 Rust、改写安装、重启服务或调用 Agent API。离线指无需业务服务/外部网络，仍需通过本地静态 HTTP 打开；不支持直接双击 `file://`。

默认显示合成 scope 的全部已知办公室：`/projects/atlas`、独立的嵌套目录 `web`、空办公室 `empty`，以及单独的全局成员和未知归属。目录选择按精确成员归属计数；工作目录和岗位来源分别显示。成员、岗位、技能、历史在当前办公室内切换；岗位只显示定义和实际采用该来源的成员。技能目录未提供。`@luna` 与当前 `@mora` 采用同一继承岗位，停止的旧 `@mora` 只在历史中出现。

阅读器有两个入口：AgentX 消息保留发送者、收件人、线程和回复引用；Provider Transcript 只读，并分别显示为空、不可用和读取失败。详情区可复制精确实例引用，查看采用的 Role/启动配置、工作目录和观测模型。

页面顶部的“离线样例”可打开数据与生命周期说明。人物、状态和历史都是静态示例。样例草稿可编辑，不能发送，也不会进入消息历史。关闭页面或刷新会清除草稿。场景和聊天互切、返回总览、切换人物均不丢草稿。选人只平移镜头，不自动放大背景；关闭阅读恢复总览镜头，切换呈现方式保留镜头与阅读尺寸。短屏会优先留出可用阅读和输入区域。停止历史可经办公室“历史”、名册或档案道具打开；失败/等待状态不会自动变成历史。每个办公室分别记住镜头，选择成员不改变缩放。

当前边界不包含 Bot/Team/插件管理、附件/语音、权限操作或运行时生命周期。后续真实 API 工作由独立所有者负责；[模块入口](AGENTS.md)说明替换位置。

当前适配设计、案例、执行状态及独立执行交接见 [frontend-work.md](../../.controller/agent-space-alignment/frontend-work.md) 和 [Task](../../.controller/agent-space-alignment/implementation.md)。[原迁移记录](../../.controller/agent-space/implementation.md)及其验证仅覆盖历史版本。素材与来源见 [NOTICE.md](NOTICE.md) 及 [逐文件清单](public/notices/migration-inventory.json)。本模块不读取原仓库的数据或配置。

所有真实业务视图共用 [Domain SDK](../../.docs/model/typescript-domain-sdk.md)：Agent 对象处理消息、Transcript、启动关系和实例操作；工作区负责成员、岗位与按岗启动，作用域提供附件、信箱、目录及启动。支持原生成员查询的服务会先按工作区筛选再分页；旧服务仍走明确区分的全局分页投影。浏览器保留原回执，恢复时只查询原操作。使用 `node web/agent-space/build.mjs /absolute/new/output` 生成 `/space/` 前缀的全新资源，与原生管理 API 一起托管。构建保留完整素材和许可，不覆盖已有目录或改动正在运行的预览。

独立开发或静态预览未配置真实宿主时，可显式打开 `?supply=demo`。真实供给失败会显示读取错误，不回退到样例。启动岗位、发送消息、停止/恢复和读取 Provider 记录都必须主动操作；目录联系先预览原生解析入口再发送。回执保存在当前浏览器，刷新后只查询原请求，不自动重复效果；Unknown 保持未确定状态。草稿只属于当前页面；场景、聊天列表和风格之间可以切换，发送与读取仍走同一个 SDK。完整场景、供给缺口和当前验证边界见 [Live Task](../../.controller/live-ui-integration/tasks/space.md)。


成员会话中的「关联工作」读取管理 API 的 `file_bindings`，显示绑定 Task/Delivery、文件声明状态和独立的 Agent 运行状态。展开后可查看路径、原始状态、最近检查及成功读取时间。读取异常明确标为上次成功快照；此处只读，不修改文件状态。供给是原生有界缓存观测，不代表文件实时监控或完整任务清单。实现、构建验证及运行版本边界见[关联工作 Task](../../.controller/file-task-bindings/space.md)。

窄 iframe/sidebar 使用同一个 `/space/` 页面，保留办公室、成员/岗位/技能/历史、场景/聊天和原生会话，不需要 `layout` 参数。`view=scene|chat` 随页面呈现同步更新，完整导航会写入地址栏；直接打开、刷新和浏览器前进/后退恢复办公室、资源页、精确会话、频道及主要阅读面板。详见[可分享路由](../../.docs/agent-space/routes.md)。已有 `layout=embedded|sidebar` 链接仍打开完整页面；历史专用嵌入布局已移除。窄容器通过办公室选择器代替左侧树，通过同一名册/会话的返回操作保留上下文。回执和连接错误在页头「连接 · 回执」或「连接异常」内展开。

宿主页只负责打开/收起和捕获页面内容。先通过 `agentx.context.v1` 的 `getTarget` 获取当前原生会话的精确引用和 `selectionRevision`，再将同一引用/版本随 `addContext` 传回；选择改变时明确失败，用户重新确认，不自动改投或发送。`ready` 只代表桥已安装。协议扩展、结构/滚动所有权及实际验证边界见[响应式设计](../../.docs/agent-space/responsive-space.md)；当前实际客户端验证由[纠正 Task](../../.controller/context-clients/space-responsive.md)关联。

完整页面窄 iframe 与修正版示例的独立验证见[客户端交付记录](../../.controller/context-clients/space-responsive.md#completed-selected-running-outcome)。浏览器扩展的权限授予和捕获验证由客户端验收记录单独说明。


Space 的 canonical UIUX 资源现由 [uiux/README.md](uiux/README.md) 维护：Layout、输入/事实/反馈控件、Conversation、场景锚点/控制条、对象阅读窗与信箱 Block 被实际页面与 UIUX scenes 共同消费。主题源为 `styles/foundation.css` 的现有浅色方案。发送草稿只在精确原请求取得 `Succeeded` 后清理；新编辑通过草稿修订号保留，失败/未知保留原内容，恢复仅查询原回执。UIUX 工作台可用 `uiux dev web/agent-space --port 18973` 启动；当前源码和检查不等于独立浏览器验收或 LAN 更新。

导航地址可直接复制到新标签页。搜索/状态筛选使用替换历史，其余导航添加历史；草稿、附件、镜头和操作对话框不进入 URL。精确目标未加载时保留原链接并显示状态，不会自动打开同名新实例。岗位详情使用实际供给的 ID，信箱页面使用原生游标；链接恢复只读取，不会发送、启动、处理信件或主动获取 Provider 记录。此源码能力不代表选定 LAN 服务已更新。

### 反馈后端

默认 `src/inspection.ts` 使用同源 `/dancingai/`；也可通过 `meta[name="dancingai-endpoint"]` 指向已有服务。只有静态文件时仍可检查与整理草稿，但不能发送反馈。18976 已使用 DancingAI 的公共 Node binding 同时提供页面、反馈和 `/dancingai/ui/` 管理后台，数据持久存放在 DancingAI checkout 的 `.local/agent-space/dancingai/`，与 `/tmp` 构建目录分开。运行例子见相邻 DancingAI 项目的 `examples/agent-space-service/README.md`。反馈是对该页面的设计/调试意见，不会调用 AgentX 的消息、启动或停止操作。


### Production inspection asset packaging

The [DancingAI importer](scripts/vendor-dancingai.mjs) removes only the final inline source map from the browser script, preserving runtime code and embedded legal notices. Production, static and native Space builds copy the pinned script unchanged; [the packaging check](scripts/production-assets.mjs) verifies its hash and records it in `tools/dancingai/production.json`. The source inspection builder still generates maps for Agent Space itself. To debug DancingAI with its own map, build the recorded upstream version separately. Native limits remain 8 MiB per file and 32 MiB per bundle.

## 云端 Space

从部署控制台 `/deployment` 中选定 Scope 后进入云端 Space；也可直接打开同一部署的 `/scopes/{id}/space/`。打开后选择访问方式并输入对应令牌。选择 30 天或 90 天会在本浏览器保存令牌，并在重开时通过同源 API 自动验证。选择「仅本次」或点击「断开」可清除对应凭据。场景和名册打开同一个精确实例；从已选实例复制地址可保留主机、名称、实例标识与 Scope 代次。

| 令牌 | 可用功能 |
| --- | --- |
| 管理员 | 查看注册主机和 Agent，使用「新建 Agent」选择主机、工作区和岗位。成功回执会打开新实例的会话；如果已切换页面内的目标，会保留当前选择。可在会话中发消息，在「会话详情」精确停止实例，并在「回执」查询原操作。 |
| Scope | 查看注册清单与精确实例的云端消息，可继续读较早记录。该访问方式只读。 |

云端消息已保存时会显示消息 ID，送达状态由 AgentX 单独处理。云端暂不提供附件、Fork 和 Provider Transcript。主机是否能接收启动和停止操作由其管理租约决定。接口与权限细节见 [Cloud Space wire](../../.controller/cloud-space/wire.md)。
