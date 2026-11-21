# Bots 管理观测预览

这里保留生产组件的合成数据截图；界面和SDK Adapter已经实现，不是另一套静态原型。当前未连接个人Bots服务，截图不代表真实业务运行。

- [桌面总览](overview-light.png)：四项摘要、会话构成、Team/角色负载与关注队列。
- [手机总览](overview-mobile.png)：320px下优先展示关注执行，六板块和同一对象详情可用。
- [自动化](automation-light.png)：按Bot审阅例行定义、调度器和保留的运行结算。
- [手机英文详情](detail-mobile-en.png)：对象身份、所属Team、关联执行与返回。

可操作预览由`plugin/preview/index.jsx`加载真实`client/bots/`页面；在`plugin/`运行`npm run preview:serve`后使用启动输出的局域网地址访问，例如`http://192.168.2.178:4177/?surface=bots&theme=light`。参数`botsCase=partial|unconfigured|empty|denied`切换覆盖场景；`language=en`和`theme=light`沿用公共预览参数。

设计、计算与缺失能力见[现行说明](../../.docs/bots-inspector.md)。交互验证见[实施记录](../../.controller/bots-observation/tasks/implementation.md)，使用固定模拟数据，不发送/审批/中断真实执行。桌面截图1440×1000，手机320×568；物理App和真实服务验收另列。
