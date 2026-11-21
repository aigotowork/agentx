# 本地批量准备工具

用 `scripts/design_batch.py` 浏览文字配方、解释词汇命中，并把一份明确选择、明确绘制内容的 brief 编译成可审阅的独立图片任务。需要 Python 3.9+ 标准库，无需安装依赖；当前实测 Python 3.14。输入遵循 [合同 v2](contract.md)，必须包含 `render`，旧 brief 不自动兼容。工具仅访问显式本地文件和自身源码，不读取密钥、ENV 或历史私有配置，不调用模型或网络。

## 选择命令

以下命令在本 Skill 根目录执行。也可以从任意工作目录使用脚本的完整路径；默认目录始终相对脚本定位。

| 场景 | 命令 | 结果与副作用 |
| --- | --- | --- |
| 看有哪些风格、素材媒介与画布 | `python3 scripts/design_batch.py catalog` | stdout 列出 ID、描述、适用/避免和证据，格式列出 kind 与尺寸；不写文件 |
| 用普通人用语找方向 | `python3 scripts/design_batch.py suggest '清爽 安静 阅读'` | 返回命中的原词和风格 ID；无匹配明确报告；不写文件 |
| 同一产品比较三种方向 | `python3 scripts/design_batch.py prepare assets/fitness-brief.json --out /tmp/fitness-design-batch-01` | 新建输出目录，18 项独立提示词、manifest、review 和输入快照；不生成图片 |
| 消费指定版本的目录 | `python3 scripts/design_batch.py --catalog-dir ./catalog prepare assets/fitness-brief.json --out /tmp/fitness-design-batch-02` | 使用显式目录内三个 JSON；其他行为相同 |

`suggest` 只对 ID、名称和 `aliases` 做不区分大小写的完整词项子串匹配，按命中词项数降序、ID 升序排列。它不会解析否定、同义词或评估审美；例如“不要清爽”也会命中“清爽”。Agent 应结合真实情境与避免项解释候选，再明确写入 brief，不能把检索结果说成用户已选定的风格。

## 命令与输入模型

```text
design_batch.py [--catalog-dir DIRECTORY] COMMAND
  catalog
  suggest TEXT
  prepare BRIEF --out DIRECTORY
```

无子命令时显示根帮助并成功退出。根及每个子命令支持 `-h` / `--help`，不要求目录已存在。根选项放在子命令之前；`prepare --out` 放在子命令之后。

| 作用域 | 输入 | 类型 / 单位 / 限制 | 缺省值 | ENV / 外部配置 |
| --- | --- | --- | --- | --- |
| 根 | `--catalog-dir DIRECTORY` | 本地目录，包含 `styles.json`、`asset-styles.json`、`formats.json` | 脚本父级 `catalog/`；显式相对路径从当前工作目录解析 | 不支持 |
| `suggest` | `TEXT` | 必需、非空字符串；有空格时 shell 引号包裹 | 无，缺失时报错 | 不支持 |
| `prepare` | `BRIEF` | 必需、UTF-8 JSON 文件；相对当前工作目录 | 无，缺失时报错 | 不支持 |
| `prepare` | `--out DIRECTORY` | 必需、尚不存在的本地目录；父目录必须存在；相对当前工作目录 | 无，缺失时报错 | 不支持 |
| 各层 | `-h` / `--help` | 调用控制，显示帮助 | 不显示 | 不支持 |

只有内建默认值和显式 CLI 输入两层，显式值覆盖默认值。全局目录选择不是业务配置；brief 本身承载业务事实与选择。三个目录的全部条目都必须符合合同，包括未选中的条目、必需种子 ID、字段类型、有效外键、至少八种风格；未知可选字段原样保留。

brief 必需的 `id`、`product`、`audience`、`intent`、`scenario` 是非空字符串；`styles` 是非空且不重复的有效风格 ID 数组；`render` 是明确绘制范围的必需对象。顶层产品、受众、意图、场景和约束保留在 manifest/review，不发送给绘图模型。可选编排字段的解析如下：

| brief 字段 | 缺省值与约束 |
| --- | --- |
| `constraints` | `[]`；有值时每项为非空字符串 |
| `asset_subjects` | `开始`、`空状态`、`完成`；显式数组不能空，每项非空；相同主题允许重复，通过原始位置区分 |
| `asset_style` | 每种风格 `asset_styles` 的第一项；显式提供时必须是有效媒介 ID，统一覆盖本批所有风格的媒介选择；适配性仍由使用者判断 |
| `formats` | `system-board`、`screen`、`component-states`、`spot-illustration`；显式数组须非空、有效且不重复 |

格式项的宽高必须是正整数像素，布尔值不算整数。JSON 重复键、非有限数字、非法 UTF-8、空白必需值及无效引用均拒绝；错误指出文件/字段和原因。未知字段保存在 brief 和源快照中，不自动加入提示词；例如未定义的 `assumptions` 仍需 Agent 人工处理。提示词只消费下方明确 render 字段，工具不会从原始场景自动提取或补造产品数据。

## 明确每张图画什么

Agent 从用户原话逐项翻译出可见对象、准确文案、数值、选中态与视觉约束，写入 `render`。原话和行为事实仍保留在顶层，不用正则删句或把“比较三套”改名后继续发给模型。下面是只选 `screen` 时的 render 片段（其余顶层必需字段仍需填写）：

```json
{
  "version": 1,
  "formats": {
    "screen": {
      "content": ["今日训练；本周 2/3 次；20 分钟轻松跑", "主要动作：开始训练；仅今天导航选中"],
      "constraints": ["只画当前未开始的一屏，不画按钮四态网格"],
      "unknowns": ["屏幕外滚动内容未定义，不补入本图"]
    }
  }
}
```

这个对象放入 brief 的 `render` 字段。完整实际示例见 [fitness-brief.json](../assets/fitness-brief.json)，逐字段规则只由 [contract.md](contract.md#输入) 定义。

| render 输入 | 使用方法 |
| --- | --- |
| `version` | 必需整数 `1`；与合同 v2 和目录 version 1 是不同版本字段 |
| `formats[格式ID]` | 每个所选格式都有一项；可预填其他有效格式，但不为未知 ID 开后门 |
| UI 格式的 `content` | 非空数组，精确指定该图可见内容；系统板、单屏、状态板各自独立，不自动继承原场景 |
| 各格式的 `constraints` | 必需字符串数组，可空；只放本图视觉要求 |
| 各格式的 `unknowns` | 必需字符串数组，可空；保留未知决定供审阅，不作为画面标签，不自动补全 |
| `asset_subjects` | 有独立素材或家族图集时必需；每项只写描绘对象/构图，与顶层用途主题按位置一一对应，数量必须相同 |
| `asset_style_constraints[风格ID]` | 有素材时为每个选中风格提供非空视觉约束数组，明确素材配色、几何或材质，避免复用完整 UI 场景和轴 |

素材及 `asset-family-board` 的格式项不得填写 `content`；工具只取显式描绘主题，避免第二份内容偷偷混入。例：顶层主题可以写“空记录，用于尚无训练历史，不代表服务错误”，对应 render 主题只写“一本留白路线册，无文字”。“禁止重复触发”留在顶层行为记录，“进行中：正在准备…”和忙碌标记在要显示该状态的格式里明确写入。

工具验证类型、引用、数量与必需字段，不理解自然语言是否完整或只含视觉语义。作者仍需检查 render 是否遗漏用户事实、带入批次元数据、跨格式内容或未知结论；缺失信息须明确保留为 unknown 或缩小该图范围。合法显式内容原样消费，不按敏感词或句型静默过滤。

## 准备、阅读与后续执行

1. 编辑或复制 [fitness-brief.json](../assets/fitness-brief.json)，保留产品事实、场景、风格与约束，再为每个选中格式明确 render 内容。增加格式时也增加对应 render 项。
2. 运行 `prepare`，选择一个全新目录。成功消息链接到 `review.md`。
3. 阅读比较表与每个独立提示词，检查固定内容、结构差异、媒介和目标尺寸。
4. 将选定任务交给另外授权的图片执行器。先执行无依赖任务；依赖任务须等待实际系统板并完成参考绑定。工具不提交请求、不绑定图片、不写生成回执。

任务数量为 `风格数 ×（非素材格式数 + 素材格式数 × 主题数）`。默认三种风格、三个非素材格式和三个素材主题得到 18 项，其中 9 项 UI、9 项独立素材。扩展目录里的其他 `asset` 格式也按每主题展开。

`asset-family-board` 保留目录的 `system` 类型：每风格只产生一张参考图集，包含本 brief 的完整主题列表、媒介配方及约束。其目录限制为至多三主题；超过三项时工具拒绝，要求另写最多三主题的 brief 或移除该格式，不会悄悄截断。图集为统一中性实底，媒介对透明背景的要求用于后续独立素材；图集本身不算多份素材或 alpha 交付。`flow-board` 保留目录给出的至多三个节点及已定义数据要求，工具不自行补流程节点。

独立素材与家族图集的提示词完全不携带顶层产品场景、UI 文案、状态描述、完整 UI 风格配方或轴。它们只消费明确主题、当前风格的素材视觉约束、媒介画法/约束/背景和本格式构图要求。UI 格式只消费自己的可见内容、当前风格配方及五个静态轴（不含 motion）。每项明确一个完整家族，其他风格不进入同一图。原始格式 `prompt` 仍保留；`checks`、evidence、参考依赖、prepared 状态及 unknowns 放在 manifest/review。目录绘制字段原文消费，工具不会清洗其中混合的自然语言，目录作者仍须保证字段适合绘图。

```text
OUTPUT/
  manifest.json
  review.md
  prompts/job-<stable-digest>.txt
  sources/brief.json
  sources/catalog/styles.json
  sources/catalog/asset-styles.json
  sources/catalog/formats.json
  sources/design_batch.py
```

`manifest.json` 的 `version` 为 `2`，`brief` 保留原对象和 render；`resolved_brief` 列出实际默认值；`selection` 只记录显式风格选择和媒介选择依据。`selected_catalogue` 保留完整所选目录项，含 provenance evidence、动效轴、媒介约束和格式 checks；review 也附这些记录及完整原 brief，保证移出 prompt 的事实仍可审阅。`sources` 保存读取时的源路径、字节数、SHA-256 与快照路径，目录未知字段也保留在原始快照中。`jobs` 包含独立提示词路径及摘要、尺寸、格式、风格、`prepared` 状态、依赖、参考角色、checks 及实际 `render_content`；素材项另含原主题 `subject`、描绘主题 `render_subject`、位置、媒介和背景交付要求。

每种风格的 `screen` / `states` 项在选择 `system-board` 时依赖该风格系统板，`reference_roles` 声明未来需绑定的真实图，状态始终为 `unbound`。该声明和 `source_job_id` 不是图片路径，也不是已完成绑定，且不放入绘图提示词。素材始终可独立执行，风格锚图是可选参考；没有系统板时，审阅页明确说明只受文字配方约束。`jobs` 按输入风格、格式和主题顺序排列，执行器必须从 manifest 读取 `depends_on`，不能只取 prompt 或假定数组顺序就是可执行顺序。

job ID 由 brief ID、风格、格式及素材主题/位置生成，重跑相同输入稳定；修改风格文字可能保留逻辑 job ID，但会改变提示词摘要、来源摘要与 batch ID。batch ID 包含 brief、三份目录与编译器源码字节摘要，源文件仅改变空白也会改变它。输出目录不参与 ID，工具没有时间戳等随机业务内容。不同源路径会保留各自 provenance 路径，不能把文件名相同视为来源相同。

需要用冻结快照复现时，在已有批次目录运行：

```sh
python3 sources/design_batch.py --catalog-dir sources/catalog prepare sources/brief.json --out ../fitness-replay-01
```

这会使用原编译器与原输入字节，保留相同 job ID、prompt 摘要与 batch ID；新 manifest 的读取路径指向快照。新输出仍必须不存在。路径只用于溯源，不扫描原机器目录或读取其他配置。

冻结的 `003/batch` 是 v1 原始证据，保持其 brief、编译器、提示词与结果原样。新工具拒绝缺少 render 的旧 brief；需要新准备时，另取工作副本并明确翻译 render。不要修改旧批次来制造新的历史，也不要把旧快照重放当成已修正的 prompt。

## 错误、检查与边界

退出码 `0` 表示成功或诚实的检索无匹配；`2` 表示参数、输入、目录或文件系统错误，原因写 stderr。验证与完整计划都先在内存完成，再排他创建输出目录；已存在的目录、文件和悬空符号链接均拒绝，不合并、不覆盖。输入无效不会创建输出。可处理的写入异常会清理本次创建的文件和目录；进程被强制终止或系统故障可能留下不完整新目录，需检查后另选输出路径，不能把它当成成功批次。

当前发行的实际消费验证由 release Task 记录；上游测试和固定历史继续由原研究 owner 保留。
