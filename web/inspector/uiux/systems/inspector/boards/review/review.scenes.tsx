/** @aiPurpose Compose canonical components into local, explicitly modeled design-review scenes. */
import { useState } from "react";
import {
  Badge,
  Banner,
  Button,
  Chip,
  ChoiceGroup,
  IconButton,
  KeyValue,
  Panel,
  Row,
  SearchField,
  SectionHeader,
  Sheet,
  Switch,
  TextField,
} from "../../components/kit/kit.native";
import { StatusGlyph } from "../../../../../plugin/client/status";
import { PixelLoading } from "../../components/activity/activity.native";
import { TrendChart } from "../../blocks/charts/charts.native";
import { ToolPlayground } from "../../blocks/tool-playground/tool-playground.web";
import { FeedbackControls } from "../../motion/feedback.web";
import { Icon } from "../../platforms/host.web";
import { useDesignSystem } from "../../platforms/web/studio.context";
/** @aiPurpose Provide boardRegistry presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const boardRegistry = [
  {
    id: "frames",
    label: "基础组件",
    description: "生产 kit · 按钮 / 输入 / 选择",
    source: "uiux/systems/inspector/inspector.native.ts",
  },
  {
    id: "tools",
    label: "图标与反馈",
    description: "共享工具 / 动效 / 音效",
    source: "uiux/systems/inspector/",
  },
  {
    id: "signals",
    label: "状态与图表",
    description: "状态语义 / 加载 / 已有数据",
    source: "plugin/client/status.tsx · charts.tsx",
  },
  {
    id: "investigation",
    label: "恢复调查样例",
    description: "同组件组合 · 本地场景",
    source: "studio/stories.tsx",
  },
] as const;
export type BoardId = (typeof boardRegistry)[number]["id"];
/** @aiPurpose Exercise canonical primitive states. @aiUsage Keep mounted to retain local inputs across appearance changes. */
/** @aiPurpose Compose Foundations presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Foundations() {
  const { theme } = useDesignSystem();
  const [count, setCount] = useState(0),
    [query, setQuery] = useState(""),
    [draft, setDraft] = useState(""),
    [checked, setChecked] = useState(false),
    [filter, setFilter] = useState("all");
  return (
    <div className="story-grid">
      <Panel theme={theme} title="真实按钮 · 完整状态">
        <div className="story-row">
          <Button
            theme={theme}
            label="主操作"
            icon="Play"
            variant="primary"
            onPress={() => setCount((value) => value + 1)}
            testID="studio-primary"
          />
          <Button
            theme={theme}
            label="次操作"
            icon="RefreshCw"
            onPress={() => setCount(0)}
          />
          <Button
            theme={theme}
            label="危险操作"
            icon="Trash2"
            variant="danger"
            onPress={() => setCount((value) => value - 1)}
          />
        </div>
        <div className="story-row">
          <Button
            theme={theme}
            label="已禁用"
            disabled
            onPress={() => setCount(999)}
          />
          <Button
            theme={theme}
            label="处理中"
            busy
            onPress={() => setCount(999)}
          />
          <Button
            theme={theme}
            label="轻操作"
            variant="ghost"
            onPress={() => setCount((value) => value + 1)}
          />
        </div>
        <output className="story-note" aria-live="polite">
          本地操作次数：{count}
        </output>
      </Panel>
      <Panel theme={theme} title="输入与状态连续">
        <SearchField
          theme={theme}
          value={query}
          onChangeText={setQuery}
          placeholder="搜索示例"
          accessibilityLabel="工作台搜索"
          grow={false}
        />
        <TextField
          theme={theme}
          label="草稿"
          value={draft}
          onChangeText={setDraft}
          placeholder="切换整个风格，草稿仍保留"
        />
        <div className="story-row">
          <Switch
            theme={theme}
            label="仅看待核实"
            value={checked}
            onChange={setChecked}
          />
          <span>仅看待核实</span>
        </div>
      </Panel>
      <Panel theme={theme} title="选择与类别">
        <ChoiceGroup
          theme={theme}
          label="范围"
          layout="wrap"
          value={filter}
          onChange={setFilter}
          options={[
            { value: "all", label: "全部", count: 28 },
            { value: "pending", label: "待核实", count: 10, tone: "warning" },
            { value: "running", label: "运行中", count: 0, disabled: true },
          ]}
        />
        <div className="story-row">
          <Chip
            theme={theme}
            label="已选择"
            selected
            onPress={() => setChecked((value) => !value)}
          />
          <Badge theme={theme} label="已知事实" tone="success" />
          <Badge theme={theme} label="来源未知" tone="warning" />
        </div>
        <KeyValue theme={theme} label="选择值" value={filter} compact />
      </Panel>
      <Panel theme={theme} title="同一 Panel · 轮廓和阅读面">
        <div className="story-row">
          {["Search", "Folder", "Layers", "Settings2", "ArrowUpRight"].map(
            (name) => (
              <IconButton
                key={name}
                theme={theme}
                icon={name}
                label={`${name} 示例`}
                onPress={() => setCount((value) => value + 1)}
              />
            ),
          )}
        </div>
        <Banner
          theme={theme}
          tone="info"
          message="Panel、Button、IconButton 都从正式设计系统导入。"
        />
        <p className="story-note">悬停或键盘聚焦图标可看单次马赛克微动。</p>
      </Panel>
    </div>
  );
}
/** @aiPurpose Review shared icon, sound and tool feedback. @aiUsage Local samples have no business effects. */
/** @aiPurpose Compose Tools presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Tools() {
  const { theme, feedback, appearance } = useDesignSystem();
  return (
    <div className="story-grid">
      <Panel theme={theme} title="原创工具 · 真实共享组件">
        <ToolPlayground theme={theme} feedback={feedback} />
      </Panel>
      <Panel theme={theme} title="声音与动效偏好">
        <FeedbackControls theme={theme} feedback={feedback} />
      </Panel>
      <Panel theme={theme} title="共享图标集">
        <div className="story-icons">
          {Object.entries({
            Search: "搜索",
            Folder: "文件夹",
            Layers: "层级",
            Play: "运行",
            RotateCcw: "重置",
            Settings2: "设置",
            ChartColumn: "图表",
            TriangleAlert: "警告",
            Check: "完成",
            CircleHelp: "未知",
          }).map(([name, label]) => (
            <div key={name}>
              <IconButton
                theme={theme}
                icon={name}
                label={`${name} 图标`}
                onPress={() => {}}
              />
              <span>{label}</span>
            </div>
          ))}
        </div>
        <p className="story-note">
          资源映射：
          {appearance.style === "pixel" ? "整数网格 SVG" : "原版 Lucide SVG"}
          。组件契约保持一致。
        </p>
      </Panel>
      <Panel theme={theme} title="变量与资源的责任">
        <KeyValue
          theme={theme}
          label="按下"
          value="--ds-motion-press → 共享组件"
          compact
        />
        <KeyValue
          theme={theme}
          label="图标微动"
          value="--ds-motion-icon → 共享 Icon"
          compact
        />
        <KeyValue
          theme={theme}
          label="工具图像"
          value="inspector.assets.ts"
          compact
        />
        <KeyValue
          theme={theme}
          label="声音"
          value="assets/sound/sound.web.ts · 显式启用"
          compact
        />
      </Panel>
    </div>
  );
}
const chartPoints = [
  { label: "主会话", value: 68 },
  { label: "子 Agent", value: 420 },
];
/** @aiPurpose Compare domain-neutral statuses and fixed-data charts. @aiUsage Samples never infer missing facts. */
/** @aiPurpose Compose Signals presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Signals() {
  const { theme } = useDesignSystem();
  const [scene, setScene] = useState<"running" | "done" | "unknown">("running");
  return (
    <div className="story-grid">
      <Panel theme={theme} title="状态与加载分开">
        <div className="story-row">
          <StatusGlyph theme={theme} phase="running" />
          <Badge theme={theme} label="运行中" tone="info" />
        </div>
        <div className="story-row">
          <StatusGlyph theme={theme} phase="approval" />
          <Badge theme={theme} label="等待回复" tone="warning" />
        </div>
        <div className="story-row">
          <PixelLoading color={theme.colors.accent} />
          <span>读取中 · 未知进度</span>
        </div>
        <div className="story-row">
          <Icon name="Square" color={theme.colors.statusWarning} />
          <span>已停止 · 静态</span>
          <Icon name="CircleHelp" color={theme.colors.statusWarning} />
          <span>效果未知 · 静态</span>
        </div>
        <ChoiceGroup
          theme={theme}
          label="本地状态场景"
          layout="wrap"
          value={scene}
          onChange={setScene}
          options={[
            { value: "running", label: "运行" },
            { value: "done", label: "明确完成" },
            { value: "unknown", label: "效果未知" },
          ]}
        />
        <div className="story-row">
          {scene === "unknown" ? (
            <Icon name="CircleHelp" color={theme.colors.foregroundMuted} />
          ) : (
            <StatusGlyph
              theme={theme}
              phase={scene}
              animate={scene === "running"}
            />
          )}
          <span>
            {scene === "unknown"
              ? "未知不能变成成功"
              : scene === "done"
                ? "明确完成"
                : "执行中"}
          </span>
        </div>
      </Panel>
      <Panel theme={theme} title="真实图表组件 · 固定事实样例">
        <TrendChart
          theme={theme}
          points={chartPoints}
          color="blue"
          average={false}
          format={String}
        />
        <p className="story-note">
          来自已接受 v2：总量 488，子 Agent 420，主会话
          68。设计样例，不是实时主机统计。
        </p>
        <div className="story-missing">逐日事实未提供 · 不补零、不制造趋势</div>
      </Panel>
    </div>
  );
}
/** @aiPurpose Model recovery presentation scenes with canonical controls. @aiUsage Local preview only; no RPC or recovery delivery. */
/** @aiPurpose Compose Investigation presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Investigation() {
  const { theme } = useDesignSystem();
  const [scene, setScene] = useState("stopped"),
    [open, setOpen] = useState(true),
    [preview, setPreview] = useState(false),
    [query, setQuery] = useState("");
  const labels: Record<string, string> = {
    stopped: "明确停止",
    tool: "仅工具错误",
    unknown: "投递效果未知",
    accepted: "宿主已接受",
  };
  const visible = "Agent Inspector 工作台体验领域负责人 high demand"
    .toLowerCase()
    .includes(query.toLowerCase());
  return (
    <div className="story-grid">
      <Panel theme={theme} title="恢复调查 · 本地组合样例">
        <SearchField
          theme={theme}
          value={query}
          onChangeText={setQuery}
          accessibilityLabel="调查搜索"
          grow={false}
        />
        <SectionHeader
          theme={theme}
          title="Agent Inspector"
          icon="Folder"
          count={1}
          open={open}
          onToggle={() => setOpen((value) => !value)}
        />
        {open && visible && (
          <Row
            theme={theme}
            title="工作台体验领域负责人"
            subtitle="Project → Workspace → Agent · codex"
            trailing={
              <Badge theme={theme} label={labels[scene]} tone="warning" />
            }
          >
            <KeyValue
              compact
              theme={theme}
              label="原始错误"
              value="We’re currently experiencing high demand."
            />
            <KeyValue
              compact
              theme={theme}
              label="判断边界"
              value={
                scene === "tool"
                  ? "工具错误不构成停止证据"
                  : scene === "unknown"
                    ? "效果未知，保持屏障，不补发"
                    : scene === "accepted"
                      ? "宿主已接受不等于任务完成"
                      : "明确停止，允许展示核验动作"
              }
            />
            <div className="story-row">
              <Button
                theme={theme}
                label="预览恢复动作"
                icon="Play"
                variant="primary"
                disabled={scene !== "stopped"}
                onPress={() => setPreview(true)}
              />
              <Button
                theme={theme}
                label="原会话入口样例"
                icon="ArrowUpRight"
                onPress={() => {}}
              />
            </div>
          </Row>
        )}
        <p className="story-note">
          只组合真实组件与本地事实场景，不发送 follow-up。
        </p>
      </Panel>
      <Panel theme={theme} title="事实场景">
        <ChoiceGroup
          theme={theme}
          value={scene}
          onChange={setScene}
          label="调查事实场景"
          options={Object.entries(labels).map(([value, label]) => ({
            value,
            label,
          }))}
        />
        <Banner
          theme={theme}
          tone="warning"
          message="停止 / 工具错误 / 未知 / 接受回执分别保留语义。"
        />
      </Panel>
      <Sheet
        title="恢复动作视觉预览"
        open={preview}
        onClose={() => setPreview(false)}
      >
        <p>这是本地组件样例，没有发送恢复指令。</p>
        <Button
          theme={theme}
          label="关闭预览"
          onPress={() => setPreview(false)}
        />
      </Sheet>
    </div>
  );
}
