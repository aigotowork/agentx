# Agent Inspector 设计系统

本系统拥有基础值、语义主题、正式组件、图表、素材、动效和评审板。Paseo 原生组件与完整 Web 使用相同契约；sibling recovery 是既有独立 DOM 系统，不虚构为已合并 API。

Foundation 定义色彩尺度、间距、字级与时长；theme 一次解析产生原生可计算的颜色与 CSS 语义变量。Components 拥有输入、选择、名称、busy／disabled、焦点与浮层；Blocks 组合事实展示。查询、权限、恢复屏障、发送与页面状态继续由产品领域拥有。

对外入口：inspector.native.ts 提供 kit／overlay／navigation／motion／charts；inspector.tokens.ts 提供纯基础值；inspector.theme.ts 提供主题／皮肤；inspector.web.ts 提供浏览器表达；inspector.assets.ts 提供语义素材选择。内部对象相邻维护 design/resource/scenes，多个资源可消费同一 kit 实现，不复制组件。

图标、背景 SVG、主题纹理、工具矢量与声音是资源。调用方通过 resolveAsset('background.workspace') 等语义角色选择，资源可替换，调用处不散落具体路径。保留来源和许可。未来位图与字体按同一资源契约登记，未提供资源时不声明其已支持。

横竖阶梯空心轮廓与 inset 填充同 cut，2px 网格，不裁点击盒／焦点。按压 90ms／2px；图标局部错开 1px／240ms；斜锤下敲 200ms、碎屑 360ms。声音显式开启；减少动态、停止／未知和禁用优先。动效不会暗示业务成功。

旧 SVG／HTML 是历史候选；迁移源码与浏览器证据不等于真机、人工听感或读屏验收。没有新增 App 风格入口，也不 reload／restart 产品服务。进度见 uiux.migration.md。

