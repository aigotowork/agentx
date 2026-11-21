---
schema: document
description: 管理产品设计方法的原始出处、局部采用方式、适用边界与内部专业 owner
---

# 参考方法及采用范围

本领域把实际工作台经验与已有专业方法组合成可执行流程。下面是进一步理解判断依据的入口；执行正文不要求联网、安装 Skill 或阅读全部原文。本次没有调用外部 Skill，也不把参考文章当作执行工具。

## 外部方法

以下原始页面于 2026-10-07 实际访问并核对相关内容。这里概述采用范围并链接原文，未复制整篇方法；网页可能更新。引用不意味着本方法获得来源方认证，也不能代替自身产品的使用证据。

| 来源 | 采用的判断及正文位置 | 边界 |
| --- | --- | --- |
| Jakob Nielsen / NN/g，[Progressive Disclosure](https://www.nngroup.com/articles/progressive-disclosure/) | [页面信息层级](page-design.md#3-安排信息层级与空间)：常用且重要的信息先出现，低频细节按需展开，保留清楚入口 | 不隐藏当前决策必要前提、异常和后果；不把所有内容都折叠 |
| Jakob Nielsen / NN/g，[10 Usability Heuristics for User Interface Design](https://www.nngroup.com/articles/ten-usability-heuristics/) | [核心理念](README.md#核心理念)和[页面方法](page-design.md)：状态可见、贴合用户概念、识别胜于记忆、用户控制、一致性、错误恢复 | 这是评估视角，不能单独推导领域模型、固定导航或证明用户测试通过；“简洁”不等于删掉必要帮助 |
| Design Council，[Framework for Innovation / Double Diamond](https://www.designcouncil.org.uk/our-resources/framework-for-innovation/) | [设计流程](workflow.md)：先理解/界定问题，再探索/验证解决方式；原型与发现可以往返 | 不照搬为四阶段瀑布，不要求每项小改动重做调研或等待新批准；本领域的九步是局部操作方法，不冒充原文规定 |
| W3C WAI，[Understanding SC 1.4.10: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html) | [视觉与适配](page-design.md#7-形成一致可操作的视觉语言)：缩放/窄宽度下保留信息和功能，按任务处理需要二维布局的区域 | 理解页说明 Level AA 的 320 CSS px 宽度/256 CSS px 高度条件和二维内容例外；一次手机截图或无溢出检查不建立 WCAG 合规 |
| W3C WAI，[Understanding SC 2.4.7: Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html) | [视觉与适配](page-design.md#7-形成一致可操作的视觉语言)：用户能看见键盘焦点，并在实际交互中检查 | 可见焦点只是一个要求；不能代替完整键盘操作、焦点顺序和其他可访问性评估 |

对象、任务与关系的推导在此作为实际设计方法表达，不声称正式执行了某个另行认证的方法体系。采用外部方法时，说明它改变哪项判断、成立条件是什么；不要附一串未使用的名词或要求安装未知 Skill。

## 内部专业方法

| 需要深入的问题 | 继续阅读 |
| --- | --- |
| 需求、场景、概念与实现如何连接 | [产品设计工作循环](../../scenarios/product-design.md)，以及[专业方法入口](../product-design/AGENTS.md) |
| 如何持续维护产品知识与当前/目标差异 | [设计管理](../design-management/AGENTS.md) |
| 工具页的模型、图表、信息密度与依据 | [工具界面](../webdevelopment/tool-interfaces.md) |
| 页面、地址、覆盖层、返回及草稿寿命 | [导航与恢复](../webdevelopment/navigation-and-recovery.md) |
| 数据来源、动作、状态与组件协议 | [交互与状态](../webdevelopment/interaction-and-state.md)，[组件设计](../webdevelopment/component-design.md) |
| 视觉规则及跨页面一致性 | [设计系统](../webdevelopment/design-system.md) |
| 原型、用户反馈及实际验证如何选 | [原型](../product-design/prototyping.md)，[评估](../product-design/evaluation.md)，[前端评审](../webdevelopment/review.md) |

UI Preview 等成熟内部产品可作为消费项目自己的参考案例。采用本方法不依赖它们的源码或界面；参考研究的可复用部分是[观察与转化步骤](workflow.md#4-研究参考产品确定页面结构)。本次来源与历史证据的具体路径留在作者记录中，不成为指导包的运行前提。
