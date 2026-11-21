# AgentX Inspector

Inspector 保留自己的八标签、UIUX 组件、原版/像素风与独立明暗主题，默认从同源 AgentX 管理 API 读取真实数据。集成部署入口为 `/inspector/`；运行状态和验收范围见[当前 Task](../../.controller/live-ui-integration/tasks/inspector.md)与[Delivery](../../.controller/live-ui-integration/delivery.md)。原生数据语义见[实时模型](../../.docs/inspector/live-model.md)，旧[离线评估](../../.docs/inspector/data-foundation.md)属于集成前的研究输入。

| 标签 | 实时路径 |
| --- | --- |
| 会话 | 精确当前实例/停止记录、采用的 workspace/Role、即时消息、停止/恢复会话、最近模型对话及独立操作回执 |
| 看板 | 当前已加载目录状态计数与采集控制；不推断任务完成率 |
| Token | 最新采集的 Provider 会话累计检查点；未知保持未知，精确十进制计数不经 Number 转换 |
| 明细 | 启用/暂停采集、明确历史起点、发现覆盖、保留边界、缺口及采样详情 |
| 横评 | 当前实例、工具、采用归属与可用用量对照；不制造速度/成功率排名 |
| 事件 | 采集启用后保留的原生状态/模型请求采样；不是完整生命周期日志 |
| 分析 | 选择 1–16 条保留观测，生成持久的确定性证据摘要、查看固定输入覆盖与作业报告；不是 AI 模型报告 |
| 恢复 | 默认关闭的持续错误自动继续策略、可配置秒数、同实例/错误 episode 的继续请求和审计；queued 只证明消息排队 |

先在「明细」启用采集，历史才开始。不会回填旧历史，也不会把 Provider 当前累计总量当作启用后的增量。采集是 daemon 的职责，关闭浏览器不会接管或停止它。持续错误表示原生未解除错误 episode 的年龄，仍须在动作时核对当前 failed 状态；不是仅凭页面计时重试。原生模型错误观测目前是 Hook-only，未提供的 Provider/模型状态保持未知。

浏览器仅保存展示偏好、精确数据集/实例下的消息草稿与待核对请求身份。操作身份先保存后提交；丢失响应只能查询原回执，Unknown 不自动重发。采集配置使用原生 revision CAS。数据代次更换会隔离旧选择、草稿、缓存和请求。跨浏览器共享事实以 daemon 为准；清除浏览器存储会丢失本机待核对索引。

实时路径不包含源产品无原生对应的 Bots catalog、审批、已读完成、workspace 归档/还原、handover。原生停止历史和会话恢复仍保留。账户额度/reset、金额、计划任务/延迟发送及其运行历史已退出[选定范围](../../.controller/inspector-migration/scope-trim.md)。

```sh
cd web/inspector
npm run typecheck
node tests/runtime-read-repair.mjs
```

集成构建由集成方执行：`node build.mjs /absolute/nonexistent/output`。它只写新的输出目录，资源使用 `/inspector/` 前缀，包含 LICENSE/NOTICE；后续由 native 宿主嵌入并提供同源 `/api/v1`。本模块不会启动 daemon 或代理任意 API。

历史静态开发入口仍可用：`npm run build`、`INSPECTOR_PORT=18972 npm start`，再访问 `http://127.0.0.1:18972/?source=demo`。默认端口 18971 已有受保护的运行预览，不要重启或在其运行期间刷新 dist。静态开发服务没有真实业务 API，默认 live 页面会诚实显示连接错误；不存在失败后回退模拟数据的逻辑。

`source=demo` 才会加载原迁移模拟 provider；模拟动作仅改变页面内存，不证明 native 支持。保留的 `scenario`、组件预览、`source=demo&experience=recovery`、原始 `payment-root` 等场景均属历史/演示输入。原迁移的使用与证据见[迁移模型](docs/migration.md)、[清理记录](../../.controller/inspector-migration/scope-trim-implementation.md)、[来源清单](docs/source-inventory.json)。保留 [MIT LICENSE](LICENSE) 和 [NOTICE](NOTICE)。

实时目录默认读取当前成员的第一页（100 条）；「加载更多当前成员」才追加下一页。「全部（含停止历史）」切换到独立缓存的完整目录分页，保留停止历史入口，历史页只在手动刷新时重读。会话、看板和横评才读取成员目录，其他页签按需读取采集数据；当前成员轮询仅刷新已请求的当前页，页面隐藏时暂停定时读取。筛选和统计针对已加载记录，界面明确显示部分覆盖。实现与验证边界见 [Web repair Task](../../.controller/runtime-read-repair/web.task.md)。定向 SDK 分页检查：`node tests/runtime-read-repair.mjs`。
