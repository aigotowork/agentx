/** @aiPurpose Exercise canonical production components in both visual skins without business commands. */
import { useState, type ReactNode } from "react";
import type { PluginTheme } from "../../../../../host/contracts";
import { Badge, Banner, Button, Chip, ChoiceGroup, EmptyState, IconButton, Row, SearchField, Segmented, Sheet, Switch, Tabs, TextField } from "../../components/kit/kit.native";
import { OverlayBoundary } from "../../components/overlay/overlay.native";
import { ViewTabs } from "../../components/navigation/navigation.native";
import { KeyValue } from "../../components/display/display.native";
import { StatusGlyph } from "../../../../../plugin/client/status";
import { TrendChart } from "../../blocks/charts/charts.native";
import { Pop, Sweep } from "../../motion/motion.native";
import { ToolPlayground } from "../../blocks/tool-playground/tool-playground.web";
import { FeedbackControls, type PixelFeedback } from "../../motion/feedback.web";

function PreviewBlock({ title, children }: { title: string; children: ReactNode }) {
  return <section className="component-preview-block"><h2>{title}</h2>{children}</section>;
}

function Actions({ theme }: { theme: PluginTheme }) {
  const [count, setCount] = useState(0);
  return <PreviewBlock title="按钮与反馈">
    <div className="component-preview-row">
      <Button theme={theme} label="主操作" icon="Play" variant="primary" onPress={() => setCount(value => value + 1)} />
      <Button theme={theme} label="次操作" icon="RefreshCw" onPress={() => setCount(0)} />
      <Button theme={theme} label="危险操作" icon="Trash2" variant="danger" onPress={() => setCount(value => value - 1)} />
    </div>
    <div className="component-preview-row">
      <Button theme={theme} label="已禁用" disabled onPress={() => setCount(999)} />
      <Button theme={theme} label="处理中" busy onPress={() => setCount(999)} />
      <IconButton theme={theme} label="筛选示例" icon="Filter" onPress={() => setCount(value => value + 1)} />
      <IconButton theme={theme} label="设置示例" icon="Settings2" onPress={() => setCount(value => value + 1)} />
    </div>
    <p className="component-preview-status" role="status">本地操作次数：{count}</p>
  </PreviewBlock>;
}

function Fields({ theme }: { theme: PluginTheme }) {
  const [search, setSearch] = useState("");
  const [draft, setDraft] = useState("");
  const [enabled, setEnabled] = useState(false);
  return <PreviewBlock title="搜索、输入与开关">
    <SearchField theme={theme} value={search} onChangeText={setSearch} accessibilityLabel="组件预览搜索" placeholder="搜索示例" grow={false} />
    <TextField theme={theme} label="本地草稿" value={draft} onChangeText={setDraft} placeholder="切换风格后草稿仍保留" />
    <div className="component-preview-row"><Switch theme={theme} label="本地开关" value={enabled} onChange={setEnabled} /><span>本地开关 · {enabled ? "开启" : "关闭"}</span></div>
  </PreviewBlock>;
}

function Selection({ theme }: { theme: PluginTheme }) {
  const [filter, setFilter] = useState(false), [range, setRange] = useState("7d"), [choice, setChoice] = useState<string | null>(null);
  return <PreviewBlock title="筛选与受控选择">
    <div className="component-preview-row">
      <Chip theme={theme} label="待核实" count={10} tone="danger" selected={filter} onPress={() => setFilter(value => !value)} tooltip="切换示例筛选" />
      <Chip theme={theme} label="已恢复" count={24} tone="success" disabled onPress={() => {}} />
    </div>
    <Segmented theme={theme} value={range} onChange={setRange} options={[{ value: "7d", label: "7 天" }, { value: "30d", label: "30 天" }, { value: "90d", label: "90 天" }]} />
    <ChoiceGroup theme={theme} value={choice} onChange={setChoice} label="示例范围" options={[{ value: null, label: "全部范围" }, { value: "project", label: "单个项目", hint: "仅用于组件交互预览" }]} />
  </PreviewBlock>;
}

function Navigation({ theme }: { theme: PluginTheme }) {
  const [tab, setTab] = useState("sessions"), [detail, setDetail] = useState("result");
  return <PreviewBlock title="主导航与详情页签">
    <ViewTabs theme={theme} value={tab} onChange={setTab} accessibilityLabel="示例主导航" tabs={[{ value: "sessions", label: "会话", icon: "MessagesSquare", colors: ["blue", "cyan"] }, { value: "recovery", label: "恢复", icon: "RotateCcw", colors: ["orange", "amber"] }, { value: "stats", label: "看板", icon: "LayoutGrid", colors: ["pink", "fuchsia"] }]} />
    <Tabs theme={theme} value={detail} onChange={setDetail} options={[{ value: "result", label: "结果" }, { value: "timeline", label: "时间线" }, { value: "children", label: "子任务", count: 2 }]} />
    <Row theme={theme} title="示例 Agent" subtitle="项目 / Workspace · codex" selected onPress={() => {}} trailing={<Badge theme={theme} label="执行中" tone="info" />} />
  </PreviewBlock>;
}

function States({ theme }: { theme: PluginTheme }) {
  return <PreviewBlock title="状态与提示">
    <div className="component-preview-row">{([['等待重试', 'warning'], ['核验中', 'info'], ['已恢复', 'success'], ['待核实', 'danger'], ['投递未知', 'violet']] as const).map(([label,tone]) => <Badge key={tone} theme={theme} label={label} tone={tone} />)}</div>
    <div className="component-preview-row">{(['blue','green','amber','red','violet','pink'] as const).map(tone => <Badge key={tone} theme={theme} label={tone} tone={tone} solid />)}</div>
    <Banner theme={theme} tone="warning" icon="TriangleAlert" message="观察信息暂未确认" />
    <KeyValue theme={theme} label="显示依据" value="保留范围内的事实" compact />
  </PreviewBlock>;
}

function Disclosure({ theme }: { theme: PluginTheme }) {
  const [open, setOpen] = useState(false), [draft, setDraft] = useState("");
  return <PreviewBlock title="空态与局部面板">
    <EmptyState theme={theme} icon="PanelLeft" title="选择一个会话" message="结果、时间线和子任务会显示在这里" />
    <Button theme={theme} label="打开本地面板" onPress={() => setOpen(true)} />
    <Sheet title="组件面板示例" open={open} onClose={() => setOpen(false)}>
      <TextField theme={theme} label="面板草稿" value={draft} onChangeText={setDraft} placeholder="关闭再打开也会保留" />
      <Button theme={theme} label="完成" variant="primary" onPress={() => setOpen(false)} />
    </Sheet>
  </PreviewBlock>;
}

function MotionScenes({ theme }: { theme: PluginTheme }) {
  const [result, setResult] = useState<"running" | "done" | "unknown">("running"), [revision, setRevision] = useState(0);
  const phase = result === "unknown" ? "error" : result;
  return <PreviewBlock title="状态节奏与结果反馈">
    <div className="component-preview-row"><StatusGlyph theme={theme} phase="running" /><Badge theme={theme} label="运行 · 三档跳动" tone="info" /></div>
    <div className="component-preview-row"><StatusGlyph theme={theme} phase="approval" /><Badge theme={theme} label="等待回复 · 阶梯轻节奏" tone="warning" /></div>
    <Sweep color={theme.colors.accent} track={theme.colors.surface2} height={4} />
    <p className="component-preview-status">加载扫描与运行状态分开；文字始终保持可读。</p>
    <div className="component-preview-row">
      <Pop size={18} count={result === "done" ? revision : 0}><StatusGlyph theme={theme} phase={phase} animate={result === "running"} size={18} /></Pop>
      <Badge theme={theme} label={result === "running" ? "本地样例运行中" : result === "done" ? "已明确完成 · 一次落定" : "结果未知 · 静止待核实"} tone={result === "done" ? "success" : result === "unknown" ? "warning" : "info"} />
    </div>
    <div className="component-preview-row">
      <Button theme={theme} label="模拟明确完成" onPress={() => { setResult("done");setRevision(value => value + 1); }} />
      <Button theme={theme} label="模拟结果未知" onPress={() => setResult("unknown")} />
      <Button theme={theme} label="重新运行样例" variant="ghost" onPress={() => setResult("running")} />
    </div>
  </PreviewBlock>;
}

const chartPoints = [18, 34, 23, 48, 29, 41, 37].map((value, index) => ({ label: `11/${index + 1}`, value }));
function ChartMotion({ theme }: { theme: PluginTheme }) {
  const [replay, setReplay] = useState(0);
  return <PreviewBlock title="图表显露与选点">
    <p className="component-preview-status">固定数据样例 · 点击柱形查看数值，只有显露过程使用动效。</p>
    <TrendChart key={replay} theme={theme} points={chartPoints} color="blue" average={false} format={String} />
    <Button theme={theme} label="重播图表显露" onPress={() => setReplay(value => value + 1)} />
  </PreviewBlock>;
}

/** @aiPurpose Compose real kit examples with locally controlled state. @aiUsage The host supplies theme; examples never invoke RPC or modify product policy. */
/** @aiPurpose Compose ComponentPreview presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ComponentPreview({ theme, feedback, onBack }: { theme: PluginTheme; feedback: PixelFeedback; onBack(): void }) {
  return <OverlayBoundary theme={theme} scopeKey="component-preview"><main className="component-preview"><div className="component-preview-inner">
    <header className="component-preview-heading"><h1>基础组件预览</h1><Button theme={theme} label="返回 Inspector" onPress={onBack} /></header>
    <p className="component-preview-note">本地组件样例。右上角可切换原版 / 像素风及明亮 / 深色，输入、选择与草稿保持不变。</p>
    <div className="component-preview-grid"><Actions theme={theme} /><Fields theme={theme} /><Selection theme={theme} /><Navigation theme={theme} /><States theme={theme} /><Disclosure theme={theme} /><PreviewBlock title="工具光标、声音与动效"><FeedbackControls theme={theme} feedback={feedback} /></PreviewBlock><PreviewBlock title="原创工具试敲"><ToolPlayground theme={theme} feedback={feedback} /></PreviewBlock><MotionScenes theme={theme} /><ChartMotion theme={theme} /></div>
  </div></main></OverlayBoundary>;
}
