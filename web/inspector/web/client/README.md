# 恢复工作区 / Web

本次交付拥有并行 Web 的恢复客户端和公共视觉/交互资源，不改变 Paseo 原侧栏。恢复入口为 `/?experience=recovery`，经共享 `main.tsx` 挂载 `recovery/screen.tsx`；完整 Inspector 的 `host/` 与 `inspector.tsx` 由另一个已授权工作项维护。公共视觉/交互资源归 `design-system/`，恢复查询、筛选、选择与命令归 `recovery/`。React 页面只消费 `../shared/contracts.ts` 的 API 类型和现行纯领域恢复守卫；没有样例回退或模拟发送开关。

## 布局与操作决定

主任务：当前主机队列 → 选中 Agent → 停止事实/原始错误/尝试与审核 → 明确确认 → 真实投递与执行观察 → 返回原队列。“关注”包含 detected 异常、领域未决尝试、活动等待链和本地待核实请求；首次读取无关注项时进入真实操作记录。“全部”保留被替代/受阻/历史对象，不解析英文受阻原文猜生命周期。列表按真实项目与 Workspace 分组，搜索和筛选只作用当前返回集合。没有自动选择目标、没有“未选择就发送全部”。

桌面为 56px 紧凑顶栏、搜索/范围工具栏、队列与邻接检查器。没有 hero、宣传句、巨大汇总卡或解释设计的标签。状态计数直接作为过滤控件。窄屏选中后在原工作区聚焦对象，底部动作保持可达，错误和记录在主体滚动；返回保留查询、固定勾选、展开与滚动。每次确认都固定 incidentId/revision；数据变化使旧目标失效，不能悄悄替换。

优先显示标题、类别、状态、时间、原始错误摘要、投递事实。操作记录行只保留标题及 workspace/provider/time 两层，phase 与触发并列；原始 reason 在详情“观察依据与操作 ID”展开，不逐行重复。技术 ID、固定跟进正文、来源元数据和审核就地展开。链管理在对象菜单，策略在局部 Sheet；保存策略本身就是明确确认，开页、切换和刷新都不写。缺少已确认 Web 原会话导航合同，因此没有虚构跳转。

暖纸、墨绿、1px 轮廓、少量台阶细节与原创正交图标延续像素方向。中文保持系统字体，正文 14px、辅助 12/13px；状态有文字与形状，未知不只换颜色。公共按钮/图标、Badge、Tab、选择/过滤、Modal 与 Tooltip 共用 token。提示属于共享 overlay，在滚动区之外测量、钳位，支持焦点与触摸长按；Modal 关闭背景提示并恢复焦点。

## 数据与命令边界

- `GET /api/session`：主机与 CSRF；`GET /api/recovery`：现行 RecoveryList，不从浏览器补造资格。
- `POST /api/recovery/continue`：用户确认固定 targets 与 operationId，正文使用现行 `CONTINUE_PROMPT`，不可编辑。accepted / not_sent / unknown 与 observed_* 分开。
- `POST /api/recovery/manage`：用户确认 cycleId + pause/resume/stop。
- `POST /api/recovery/configure`：完整策略及原 revision，浏览筛选与自动范围分离。
- 同源 JSON 写请求带 `X-Inspector-CSRF`。读取失败保留旧值、标为过期并禁止写；可见时刷新，隐藏/卸载释放。写请求不重试。
- 服务实际 `invalid_csrf` 拒绝后重新获取 session；再次写入仍需用户确认。POST 的传输中断、超时、不可解析回执及 HTTP 5xx 默认未知，不依赖错误 JSON 必须携带 uncertain。
- 发送前将原操作及目标保存到本标签页 sessionStorage（无凭据）；双击由同步锁阻止。操作 ID 使用 `crypto.getRandomValues`，适用于 HTTP 局域网。中断/超时保留未知屏障及原 ID，后续查询只能凭相同 operationId 和全部固定目标的真实尝试核实。GET 未发现尝试不能解除屏障。本地未决在列表、详情和过滤中显示“请求待核实”，不修改来源事实。策略/链管理传输未知也锁住写，不能通过刷新另造操作。sessionStorage 不可用时拒绝发起写入。
- 策略 projectIds/workspaceIds 各为空时不限；两者有值时取交集。浏览过滤永远不充当策略配置。策略拒绝保留本地草稿，版本冲突可显式载入当前版本。

资格只作保守的 UI 预检查：detected、无 blockedReason、Agent 无领域未决尝试、目标 revision 未变、查询新鲜且采集无错误；最终资格、授权与账本由插件守卫决定。未采集、空集合、旧值、连接失败与投递未知各自呈现。

## 源码与体系适配

| 所有者 | 入口与职责 |
| --- | --- |
| 公共 Web 资源 | `design-system/tokens.css`、`icons.tsx`、`kit.tsx`、`overlay.tsx`：语义 token、原创像素 SVG、通用控件和共享 DOM 浮层 |
| 恢复组合 | `recovery/screen.tsx`、`layouts/workspace.tsx`：工作区状态、命名区域及响应式滚动 |
| 恢复对象 | `queue.tsx`、`detail.tsx`、`dialogs.tsx`：列表、事实/尝试/审核、固定确认和策略草稿 |
| 数据与写边界 | `api.ts`、`use-recovery.ts`、`model.ts`：同源协议、刷新/过期、持久未知和展示投影 |

沿用 RN kit 的语义 token、控件命名、受控选择和共享 overlay 责任；Web 以 CSS variables、DOM 语义、viewport Portal、inert 与焦点恢复实现平台行为。没有复制 RN 页面状态或将 RN/宿主 UI 导入恢复入口。像素 SVG 从本人原创候选落到 Web 正式资源，部署不依赖历史画板；恢复提示和资格继续复用纯领域合同。

## 已完成的定向观察

隔离走查使用 [walkthrough.mjs](verification/walkthrough.mjs)，所有请求均在 `inspector.test` 截获，没有接触现场 API。测试资源不导入生产入口；runner 外置完整 Inspector 分支，仅进入同一 `?experience=recovery` 的恢复树。完整路径报告为 [walkthrough.json](verification/evidence/walkthrough.json)，最终变化另有 [unknown-presentation.json](verification/evidence/unknown-presentation.json) 和 [touch-gesture.json](verification/evidence/touch-gesture.json)。

- 固定勾选经过搜索和双击仍只有一个精确 targets 请求；accepted/not_sent/unknown 分开。502 无 uncertain 保留原身份；reload、换筛选、GET 未找到原尝试都不解锁；相同 operationId 接回 unknown 后继续由领域守卫阻止发送。
- 实际 invalid_csrf 后刷新 token，只有用户再次确认才写；sessionStorage 不可写时无 POST。GET 失败保留旧值并禁写；策略拒绝保留草稿，链管理传输未知刷新后仍锁写。
- 390×844、320×568 下无 body 横向溢出，主动作均为 44px 且处于可视区内。长标题和原始错误可读/滚动；确认、Escape、返回及筛选连续。长按后 pointercancel 不吞下一次普通点击；同次长按合成 click 被消费，后续新手势正常。

视觉观察针对“同一对象的事实和操作是否同屏、未知是否一致、历史是否紧凑、窄屏是否保持主动作”展开，已实际查看 [桌面队列](verification/evidence/desktop-queue.png)、[历史默认](verification/evidence/history-default.png)、[待核实](verification/evidence/desktop-unknown.png)、[320 详情](verification/evidence/mobile-320-detail.png)、[320 确认](verification/evidence/mobile-320-confirm.png)、[策略草稿](verification/evidence/policy-rejected-draft.png) 等图。暖纸/墨绿、细轮廓和正交图标承载层级，中文保留普通字体；最终待核实图的行徽标、详情徽标和禁用动作一致，紫色局部请求卡保留原 ID，旁边仍可查原始错误。历史两层行减少重复说明；手机底部动作保持可达，原文由主体滚动。截图只证明对应 fixture 与视口，不代表所有数据组合。

Web `typecheck` 已通过。本域没有写共享 dist、启动持久预览或重载插件；隔离浏览器和临时 bundle 已释放。集成者先前已完成真实 GET 的桌面/390/320 只读走查（`web/evidence/live-*.png`、`live-acceptance.json`），最终构建和入口接收由其完成。真实 provider 写入、原生设备手势和读屏未在本域验证。
