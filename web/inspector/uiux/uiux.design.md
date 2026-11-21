# 本项目的UIUX接入

工具已独立到 [longwork/uiux](../../uiux/README.md)。[通用机制](../../uiux/.docs/architecture.md)、[资源标准](../../uiux/uiux/uiux.design.md)、[命令](../../uiux/.docs/usage.md)由工具项目维护；本文件只定义本产品的集成边界。

本地开发依赖@longwork/uiux使用file:../../uiux。产品组件仍从systems/inspector的native/web/theme/tokens/assets公开入口消费，不反向导入工作台。领域查询、授权、恢复发送与真实状态继续由plugin/web拥有。

uiux.config.tsx只登记inspector与recovery，工具自动附加自身workbench；SceneProps/SystemAdapter/Resource等从@longwork/uiux导入，展示用宿主控件从@longwork/uiux/workbench导入。uiux.vite.config.ts仅把RN和Paseo平台接口映射到本产品已有Web bridge；工具没有这些产品依赖。

声明资源仍使用同词干resource.json／design.md／scenes.tsx，产品组件与相邻说明/场景不挪到工具项目。管理场景只组合真实组件和显式固定设计数据，不调用业务API；主题不改变unknown、stopped、disabled或授权。

稳定workspaceId为agent-inspector；旧未分scope的方案首次读取时复制到新键，保留原记录。完整方案只保存声明参数、语义视觉设置和实例组合，私有hooks仍由当前实例拥有。跨端口迁移可通过JSON导出导入，不共享不同origin的localStorage。

开发入口4196由独立工具CLI创建；旧4192和四个Canvas页面消费同一静态构建镜像。tooling/canvas.build.mjs保留Canvas批注、参数和元数据，只替换生成HTML/资源。此适配属于本项目，不放进通用工具。
