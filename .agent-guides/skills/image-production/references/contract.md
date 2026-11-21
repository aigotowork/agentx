# 批量准备合同 v2

这是本地工具与知识目录之间的最小合同。目标是准备可以交给图片执行器的独立任务，不调用网络、不选择模型、不声称完成图片生成。来源、选择和验证仍遵循 本 Skill 的输入/生产边界。

## 目录真源

三个 UTF-8 JSON 文件都使用 `{ "version": 1, "items": [...] }`。目录 ID 为非空小写 ASCII kebab-case；所有解释、提示词片段可为中文。未知可选字段允许保留；必需字段缺失、重复 ID、无效引用拒绝。

- `catalog/styles.json`：每项必需 `id`, `name`, `aliases`（字符串数组，普通人用语）, `summary`, `suitable_for`（数组）, `avoid_for`（数组）, `axes`（对象：density/geometry/typography/color/material/motion 六个字符串）, `prompt`（字符串）, `asset_styles`（媒介 ID 数组）, `evidence`（字符串，明确种子/已验证）。至少 8 个具有结构差异的方向，必须含 `calm-editorial`, `precise-tech`, `warm-playful`，由目录 owner 定义其设计含义。
- `catalog/asset-styles.json`：必需 `id`, `name`, `aliases`, `summary`, `prompt`, `constraints`（数组）, `suitable_for`（数组）, `avoid_for`（数组）, `background`（`opaque` 或 `transparent`）, `evidence`。必须含 `monoline`, `ink-wash`；可选其他媒介。background 是该配方默认交付需求，不代表模型支持。
- `catalog/formats.json`：必需 `id`, `name`, `kind`（`system`/`screen`/`states`/`asset`）, `width`, `height`（正整数像素）, `prompt`, `checks`（数组）。必须含 `system-board`, `screen`, `component-states`, `spot-illustration`。这是目标画布；执行器另验证供应商尺寸支持。

## 输入

brief JSON 必需 `id`, `product`, `audience`, `intent`, `scenario`（非空字符串），`styles`（非空且不重复的有效 ID 数组）。可选 `constraints`（字符串数组）、`asset_subjects`（非空字符串数组，缺省使用“开始”“空状态”“完成”）、`asset_style`（指定有效媒介 ID，否则取风格的第一项）、`formats`（有效格式 ID 数组；缺省为上述四种）。用户普通语言先由 Agent 理解；工具的 `suggest` 仅做可解释词汇匹配，无匹配要明确说没有匹配，不能冒充审美理解。

以上是编排事实，不直接拼接进绘图提示词。v2 新增必需的 `render` 对象；缺少时拒绝且不产生输出，不回退到 v1 提示词。目录文件仍为 version 1；manifest 为 version 2。历史 `003/batch` 及其快照不迁移、不重写。

| render 字段 | 类型与含义 |
| --- | --- |
| `version` | 必需，整数 `1`，标识此 render 结构 |
| `formats` | 必需，对象，以有效 format ID 为键；每个本批所选格式必须有一项。允许预先写入其他有效格式，但只消费本批所选项 |
| `formats[ID].content` | UI 格式必需的非空字符串数组：此图明确可见的对象、准确文案、数值、选中态和状态标签。`kind: asset` 以及 `asset-family-board` 禁止此字段，绘制内容来自下方独立主题 |
| `formats[ID].constraints` | 必需字符串数组，可空；只写该图视觉要求、允许/禁止出现的内容。不得用批次目标、执行方法或验收责任替代 |
| `formats[ID].unknowns` | 必需字符串数组，可空；作者明确保留的未知决定，只进 manifest/review，不画成图中文字。未知的可见内容先补充或从本图范围中明确排除，工具不推断 |
| `asset_subjects` | 选择任一 `asset` 格式或 `asset-family-board` 时必需，非空字符串数组。与解析后的顶层 `asset_subjects` 等长且按位置一一对应；只写实际描绘的物体/构图，顶层仍保留用途与业务含义 |
| `asset_style_constraints` | 选择素材/家族图集时必需，以有效 style ID 为键的对象；每个选定风格有非空字符串数组，明确素材家族的配色/几何/材质等视觉约束。只消费当前风格项，不从完整 UI 配方或轴中猜测素材画法 |

Agent 将用户事实明确翻译为 render 白名单，保留顶层原意，不通过关键词过滤改写句子。内容必须覆盖实际要呈现的文案、数值和状态；例如“禁止重复触发”属于行为事实，“进行中：正在准备…”和忙碌标记属于可见状态。工具验证结构与引用，不自动证明这种翻译完整或排除了全部非视觉语义。未知字段原样快照但不自动解释。

## 操作和产物

Python 3.9+ 标准库 CLI，入口 `scripts/design_batch.py`。短命令：`catalog` 浏览；`suggest TEXT` 词汇检索；`prepare BRIEF --out DIRECTORY` 输出一个全新目录。只支持本地目录；ENV/外部 config 不参与。实际参数和帮助由工具 owner 确定并在 README 维护。

prepare 对每风格每非素材格式创建一项，对每风格每素材格式每 subject 创建独立素材项。每个 job 有稳定 `id`、`style_id`、`format_id`、`kind`、`prompt_file`、`width`、`height`、`status: prepared`、`depends_on`（job ID 数组）、`reference_roles`（说明后续要绑定的真实图）。同风格 screen/states 依赖 system-board（当 batch 选了 system-board）；素材可独立执行，所需风格锚点用 reference_roles 声明。不得把一个未生成的路径说成已绑定参考图。无系统板时明确其他格式只受文字配方约束。

根 `manifest.json` 保留完整 brief（含 render）、目录来源/摘要、选中目录记录、选择理由或显式选择事实、全部 jobs；`prompts/` 每项单独文件；`review.md` 保留完整 brief、选中目录记录（含 evidence/checks/媒介 constraints）、比较方式、依赖与未验证项。jobs 保留 `checks` 和实际 `render_content`；素材 job 的 `subject` 仍是原编排主题，`render_subject` 是明确描绘主题。可以产生静态可浏览预览，但必须标成目录/文字配方预览，不能冒充模型图像。验证输入及计划后才写；目标目录存在则拒绝，失败不得覆盖用户内容。不保存密钥或读取历史私有配置。

## 质量边界

单项 prompt 只消费所选格式的 render 内容/视觉约束及该格式 `prompt` 与画布。UI 项继承当前一个风格的 `prompt` 和静态轴 density/geometry/typography/color/material；motion 只留审阅。每个系统项明确只画一个完整家族，屏幕/状态按格式组织同一家族，不画其他风格方向。

独立素材只消费当前 `render.asset_subjects[index]`、当前风格的显式素材视觉约束、媒介 `prompt`/`constraints`/背景及格式 `prompt`，不复制顶层 UI 场景、完整 UI 风格配方或任何 UI 轴。`asset-family-board` 一次消费完整描绘主题列表（至多三项），保留单家族、中性实底的格式语义；超过三项拒绝，不截断。原始目录中的绘制配方字段按原文消费，其作者负责让字段适合绘图；工具不解析或清洗自由文本。

批次比较目标、产品/受众说明、原始场景/行为、原始约束、作者/来源证据、执行 prepared 状态、未绑定参考及生成后 checks 不拼入单项 prompt。执行器从 manifest 读取依赖和未来参考绑定要求；无系统板时 review 明确只受文字配方约束。UI 不烘焙装饰素材中的文字为真实交互。整批比较固定 render 内容与不同视觉结构，防止只换颜色。manifest 是准备结果；生成回执、视觉检查、选定规则、代码/真实消费验证需要另外取得，不能由 prepare 推断。
