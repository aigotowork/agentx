# 契约设计 Inspector

同一个 TS 定义经 `authoring.inspect` 输出 `flow-atlas/inspector-1` 数据，再生成离线 HTML。模块视图展示所属对象、公开方法及允许的模块依赖；流程视图展示声明的结果分支和步骤绑定。点击对象／方法／步骤查看原始 Schema、职责、设计文档和源码引用；检查面板保留 lint 位置与检查范围。

```sh
node --experimental-strip-types tools/flow-atlas/authoring/examples/order-inventory.ts /tmp/design.json
python3 tools/flow-atlas/inspector/build.py /tmp/design.json --output /tmp/design.html
```

这是声明设计的投影。图中的流程线不表示运行已发生，Schema 与 metadata 不在这里另写。生成器不运行 TS，不请求接口，不下载文档。数据契约和检查语义由[authoring](../authoring/README.md)拥有；设计与代码的对应边界见[来源设计](../design/source-binding.md)。已有[有限轨迹查看器](../README.md)继续负责事件播放、对象状态和布局编辑。

源码与设计文档的相对路径保留为可选择文字；单文件页面没有仓库文件服务，不把这些路径伪装成可访问的网址。HTTP(S)文档链接可主动打开。若需要托管文档，应由消费者提供真实可访问的文档URL；本片不读取或发布仓库文件。

## 画布与检查维护

画布填满容器，可在内容范围外继续平移。空白拖动／中键拖动平移，滚轮缩放，触屏单指平移／双指缩放；左下角可恢复100%或适应全部。窄屏初次保留至少85%比例，避免把文字缩成难读的总览。选择对象保留相机，各视图／流程独立保存本页视角。这里只移动观察位置；对象位置编辑仍由行为轨迹查看器负责。共享实现与键盘操作见[视口契约](../contracts.md#shared-spatial-viewport)。

“检查”消费authoring导出的同一份diagnostics；依赖、依赖环、流程分支、实现绑定等规则与维护入口见[lint索引](../authoring/lint.md)。Inspector不另写规则，也不会扫描任意TS源码。
