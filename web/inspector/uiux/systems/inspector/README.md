# Inspector 设计系统入口

迁移后的当前入口是 [模块 README](../../../README.md)、[设计契约](inspector.design.md)和[迁移模型](../../../docs/migration.md)。在模块静态服务中打开 `/?surface=components` 可查看真实生产控件；`style=original|pixel` 与 `theme=light|dark` 独立。组件预览只拥有本地展示状态，不发送业务命令。

| 层次 | 正式所有者与契约 |
| --- | --- |
| 基础值 | `foundations/foundation.tokens.ts`：色彩、间距、字号、形状、动效 |
| 语义主题 | `themes/theme.resolve.ts`：appearance → PluginTheme、CSS 变量和资源角色 |
| 展示适配 | `themes/skin.native.ts`：role → RN 样式与 Web 语义标记 |
| 公共控件 | `inspector.native.ts`、`components/`：受控值、回调、busy/disabled 和尺寸 |
| Web 资源 | `assets/`、`motion/`、`platforms/web/`：图标、背景、工具指针、声音和偏好 |
| 原始设计资源 | [UIUX 原始设计](../../uiux.design.md)、[历史提案](../../../design/README.md)；不代表当前 authoring 服务 |

风格切换改变主题输入，不改变组件 key；选择、表单草稿、权限和未知屏障由上层持有。颜色与几何使用 token；SVG、音效与背景保留原资源。公共提示使用 `OverlayBoundary`，不在按钮中另造提示定位逻辑。共享 native 组件不导入 DOM、WebAudio 或浏览器偏好。

Web host 使用本模块 `host/contracts.ts` 的展示主题类型及 React Native Web 适配。原生 App 与独立 UIUX workbench 的运行时没有迁入；`.scenes.tsx` 和 workbench adapter 仅作为设计源保留。原始设计文档中的旧部署地址和源码相对链接属于历史背景，不是当前服务。
