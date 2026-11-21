---
scope: Reusable AgentX launch configuration exploration
status: discussion-draft
owner: AgentX launch design
last_updated: 2026-11-14
related: ["./agentx-cli-panorama.md", "../.docs/model/README.md", "../src/runtime/launcher.rs", "../src/integrations/launch_args/overrides.rs"]
---

# 启动 Profile：讨论草案

用户希望为同一种工具保存多套参数、模型等配置，选定默认配置，并允许启动时临时覆盖。用户已认可本文的整体概念和设计方向；进一步的实现建议、规则细化和剩余工程问题由 [启动 Profile 提案](../.docs/proposals/launch-profiles.md) 维护。本文保留初始讨论，不代表功能已经实现。

## 建议模型

| 对象 | 职责 |
| --- | --- |
| Tool / Provider adapter | 定义接入能力、支持的配置及原生参数解释 |
| LaunchProfile | 具名、可复用的单一工具启动配置；拥有工具标识与配置内容 |
| LaunchRequest | 指定 profile 或工具、本次覆盖，以及目录、提示、数量等执行上下文 |
| ResolvedLaunchConfiguration | 解析后的本次配置及来源；与可编辑 profile 分离 |
| Agent / execution | 运行身份和执行记录；关联实际采用的配置快照 |

建议使用 `LaunchProfile` 作为领域名，对用户使用 `profile`。模型、推理配置、审批策略等可进入工具拥有的配置；支持的结构化字段由对应工具契约定义，不能假定所有工具能力相同。原生 argv 仍需保留，并沿用工具适配器的覆盖语义。不要为同一设置同时维护互不一致的结构化值和原生参数值。

## 建议解析规则

1. 显式 profile 决定工具；同时指定不匹配的工具应报错。
2. 只指定工具时，选择该工具配置的默认 profile；完全不指定时，选择作用域的默认 profile。未配置默认时的回退行为尚待决定。
3. 默认 profile 是省略选择时的指向，不自动成为所有具名 profile 的父配置。
4. 在选中的 profile 上应用显式覆盖，再进行工具能力和跨字段校验。临时覆盖不写回保存的 profile。
5. 运行记录保存本次实际解析的配置和 profile 来源；修改或删除 profile 不改变既有运行记录。

可复用的工具配置与调用上下文应保持边界。数量、输出格式、等待和本次任务提示仍属于请求；是否支持保存目录、运行承载方式或系统提示默认值，应由明确用例决定。

## 未决问题与设计风险

| 问题 | 当前建议或后续动作 |
| --- | --- |
| 旧 `<tool>_args` 与 profile 双重默认 | 明确收敛策略，避免长期保留两套隐式参数底座；尚未选择具体迁移方式 |
| 结构化参数与原生 argv 冲突 | 工具拥有统一解析和覆盖语义；已知单值、可重复选项和未知参数不能套用同一种深合并规则 |
| 配置作用域 | 优先沿用 AgentX 现有作用域；项目与用户层覆盖关系需在正式设计中确定 |
| resume / fork | 建议默认沿用历史已解析配置，不静默读取已修改的 profile；显式换 profile 需工具兼容性检查 |
| 远端启动 | 明确在哪台设备选择和解析 profile，避免两端同名配置含义不同；不得直接改变既有远端默认配置规则 |
| 契约表达 | 领域语义先确定，再映射原生契约及需要的 Zod/oRPC 投影；本次未验证具体 TS 服务承载位置 |

第一版建议聚焦保存、查看、选择、设置默认、临时覆盖和配置来源查询；继承链、角色模板和跨设备同步暂不引入。
