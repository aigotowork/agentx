# Agent Inspector 设计候选

原生 Paseo Design Canvas 读取本目录的 `canvas.json` 与各 slug 的 `design.json`。

工作台交互重设计：[组装式工作台三板](uiux-workbench/README.md)，整体／局部选择／窄屏 Layout 检查器；正式实现由 [uiux](../uiux/README.md) 拥有，并能读取自身设计系统。

原动效四板：[Inspector 设计系统 · inspector-pixel-motion](inspector-pixel-motion/README.md)。Vite + React 四板直接消费生产 kit、状态、图表和 Web 反馈，支持原版／像素、明暗主题和实时语义变量；局域网 [192.168.2.178:4192](http://192.168.2.178:4192/)。Canvas 读取相同 React 构建，历史 HTML/SVG 留作参考；采用与提案、矩阵和证据见[工作台说明](inspector-pixel-motion/README.md)。当前产品边界由[设计系统模型](../.docs/design-system.md)拥有。

已接受的静态视觉：[鲜彩像素换肤 · inspector-pixel-skin-v2](inspector-pixel-skin-v2/index.html)。依据用户反馈，保持完整 Inspector 的原版结构、功能和操作位置，仅比较饱和配色与像素细节。包含恢复明暗版、会话、看板、手机和控件共 6 张纯 SVG 画板，见[设计说明](inspector-pixel-skin-v2/README.md)。

历史探索：[像素案头 · inspector-pixel-v1](inspector-pixel-v1/index.html)。包含 6 张可操作中文画板、29 个原创 SVG 图标和 6 张可编辑页面 SVG；其低饱和配色和导航重组不作为本轮方向，详见[原方案说明](inspector-pixel-v1/README.md)。

所有画板均为离线样例，只修改当前文档内的模拟状态。用户批注、移动、文字编辑、`comments.json`、`tweaks.json` 与 `tweaks.css` 归用户所有；重新生成前必须先合并用户改动。
