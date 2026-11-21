/** @aiPurpose Provide shared accessible Inspector components with optional visual skins. */
import * as React from "react";
import type { PluginTheme } from "../../../../../host/contracts";
import { Icon, Modal, ScrollView, TextInput } from "../../../../../host/ui";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { ActivityIndicator, Pressable, Text, View, type LayoutChangeEvent, type StyleProp, type TextInput as NativeTextInput, type ViewStyle } from "react-native";
import { addDefaults, t } from "../../../../../plugin/shared/i18n";
import { kitTime, kitUi } from "../../../../../plugin/shared/locales/kit/zh-CN.json";
import { control, fontSize, hairline, radius, space, touchSlop } from "../../foundations/native.tokens";
import { OverlayBoundary, useTooltip } from "../overlay/overlay.native";
export { control, fontSize, hairline, monoFont, radius, space } from "../../foundations/native.tokens";
import { isPixelTheme, skinAttributes, skinStyle } from "../../themes/skin.native";
import { PixelLoading } from "../activity/activity.native";
import { hue, hueFor, solidForeground, toneColor, type Tone, type Hue } from "../../foundations/palette.native";
export { solidForeground, contrast, isDarkTheme, withAlpha, hue, gradient, glow, hueFor, toneHues, toneColor, type Tone, type Hue } from "../../foundations/palette.native";
import { StatusDot } from "../display/display.native";
export { KeyValue, StatRow, PaneLabel, Switch, StatusDot, StateDot, type Stat } from "../display/display.native";

/**
 * Agent Inspector's AgentX-native kit, derived from Agent Observatory 2.18.1.
 * This maintained copy owns local component behavior; numeric foundations live in ./tokens.
 * Preserve upstream attribution in LICENSE/NOTICE when evolving it.
 * Scales mirror the host app: 4-pt spacing, 12/13/14/16 type, 8 px control radius, 28/32 px controls,
 * regular-weight button text, one primary action per view. Surfaces, text and the primary button come from the
 * host theme; states and categories use the vivid palette policy (`hue`, `toneColor`, `hueFor`).
 * Words come from `../shared/i18n` (`kitUi`, `kitTime`): the plugin's language once it binds one, else Chinese.
 */
addDefaults({ kitUi, kitTime });

/**
 * react-native-web ignores `accessibilityState`, so selected / pressed / expanded / busy never reach the DOM.
 * Pass the matching `aria-*` attribute next to it (React Native's types lack some of them, hence the cast).
 */
/** @aiPurpose Provide aria presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const aria = (attributes: Record<string, boolean | string | undefined>): object =>
  Object.fromEntries(Object.entries(attributes).filter(([, value]) => value !== undefined).map(([key, value]) => [`aria-${key}`, value]));

type Interaction = { pressed: boolean; hovered?: boolean };
const hoverable = (state: unknown) => state as Interaction;

/** Compact layout below 760 px of available width or whenever the host says so. */
/** @aiPurpose Provide useCompactLayout presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useCompactLayout(hostCompact: boolean, breakpoint = 760): { compact: boolean; width: number; onLayout: (event: LayoutChangeEvent) => void } {
  const [width, setWidth] = useState(0);
  return { compact: hostCompact || (width > 0 && width < breakpoint), width, onLayout: event => setWidth(event.nativeEvent.layout.width) };
}

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
/** `grow` fills the free space of a row from the label's own width; a basis of 0 would collapse it inside a column on native. */
/** @aiPurpose Compose Button presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Button({ theme, label, icon, onPress, variant = "secondary", size = "md", disabled = false, busy = false, grow = false, tone, accessibilityLabel, testID, tooltip }: {
  theme: PluginTheme; label: string; icon?: string; onPress: () => void; variant?: ButtonVariant; size?: "sm" | "md";
  disabled?: boolean; busy?: boolean; grow?: boolean; accessibilityLabel?: string; testID?: string; tooltip?: string | false;
  /** Colors a secondary/ghost button (tinted ground, colored label) — e.g. 跳过评审 amber, 合并 green. */ tone?: Tone;
}) {
  const { colors } = theme;
  const tint = variant === "danger" ? "danger" as const : tone && tone !== "neutral" && variant !== "primary" ? tone : null;
  const background = variant === "primary" ? colors.accent : tint ? toneColor(theme, tint, "soft") : variant === "secondary" ? colors.surface2 : "transparent";
  const foreground = variant === "primary" ? colors.accentForeground : tint ? toneColor(theme, tint, "text") : variant === "ghost" ? colors.foregroundMuted : colors.foreground;
  const inactive = disabled || busy;
  const hint = useTooltip({ label: tooltip, onPress });
  return <Pressable {...skinAttributes("button")} {...hint} testID={testID} accessibilityRole="button" accessibilityLabel={accessibilityLabel ?? label} accessibilityState={{ disabled: inactive, busy }} {...aria({ busy: busy || undefined })} disabled={inactive} hitSlop={touchSlop}
    style={state => {
      const { pressed, hovered } = hoverable(state);
      return [{ flexDirection: "row", alignItems: "center", justifyContent: "center", gap: space.xs + 2, minHeight: size === "sm" ? control.sm : control.md, paddingHorizontal: size === "sm" ? 10 : space.md, borderRadius: radius.md, backgroundColor: background, borderWidth: tint ? hairline : 0, borderColor: tint ? toneColor(theme, tint, "line") : colors.border, opacity: disabled ? 0.45 : pressed ? 0.8 : 1 },
        hovered && !inactive && variant !== "primary" && !tint ? { backgroundColor: variant === "secondary" ? colors.surface2 : colors.surface1 } : null, grow ? { flexGrow: 1, flexShrink: 1, minWidth: 0 } : null, skinStyle(theme, "button", variant === "primary", hovered && !inactive && variant === "ghost" && !tint ? colors.surface1 : background === "transparent" ? colors.surface0 : background, tint ? toneColor(theme, tint, "mark") : undefined)];
    }}>
    {busy ? isPixelTheme(theme) ? <PixelLoading color={foreground} /> : <ActivityIndicator size="small" color={foreground} /> : icon ? <Icon name={icon} size={size === "sm" ? 14 : 16} color={foreground} /> : null}
    <Text numberOfLines={1} style={{ color: foreground, fontSize: size === "sm" ? fontSize.small : fontSize.body, fontWeight: variant === "primary" ? "500" : "400" }}>{label}</Text>
  </Pressable>;
}

/** Square icon-only button; `label` is required for screen readers and tooltips. Passing `active` makes it a toggle. */
/** @aiPurpose Compose IconButton presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function IconButton({ theme, icon, label, onPress, active, disabled = false, tone, size = "md", badge, testID, tooltip }: {
  theme: PluginTheme; icon: string; label: string; onPress: () => void; active?: boolean; disabled?: boolean; tone?: Tone; size?: "sm" | "md"; badge?: boolean; testID?: string; tooltip?: string | false;
}) {
  const { colors } = theme, edge = size === "sm" ? control.sm : control.md, on = active ?? false;
  const color = tone ? toneColor(theme, tone) : on ? colors.foreground : colors.foregroundMuted;
  const hint = useTooltip({ label: tooltip ?? label, onPress });
  return <Pressable {...skinAttributes("icon-button")} {...hint} testID={testID} accessibilityRole="button" accessibilityLabel={label} accessibilityState={{ disabled, selected: on }} {...aria({ pressed: active })} disabled={disabled} hitSlop={touchSlop}
    style={state => {
      const { pressed, hovered } = hoverable(state);
      return { width: edge, height: edge, alignItems: "center", justifyContent: "center", borderRadius: radius.md, backgroundColor: on ? colors.surface2 : hovered || pressed ? colors.surface1 : "transparent", opacity: disabled ? 0.4 : pressed ? 0.8 : 1, ...skinStyle(theme, "icon-button", on, on ? colors.surface2 : hovered || pressed ? colors.surface1 : colors.surface0) };
    }}>
    <Icon name={icon} size={size === "sm" ? 14 : 16} color={color} />
    {badge ? <View style={{ position: "absolute", top: 5, right: 5, width: 7, height: 7, borderRadius: 4, backgroundColor: colors.accent, borderWidth: 1.5, borderColor: colors.surface0 }} /> : null}
  </Pressable>;
}

/** The single top bar of a surface or pane. Never repeat the host title here; put context and actions in it. */
/** @aiPurpose Compose Toolbar presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Toolbar({ theme, children, border = true, style }: { theme: PluginTheme; children: ReactNode; border?: boolean; style?: StyleProp<ViewStyle> }) {
  return <View style={[{ flexDirection: "row", alignItems: "center", gap: space.sm, minHeight: 48, paddingHorizontal: space.md, paddingVertical: space.sm, borderBottomWidth: border ? hairline : 0, borderBottomColor: theme.colors.border }, style]}>{children}</View>;
}
/** @aiPurpose Compose Spacer presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const Spacer = () => <View style={{ flex: 1 }} />;

/** Horizontally scrolling row of chips (filters, scopes); never wraps into several lines on phones. */
/** @aiPurpose Compose ChipBar presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ChipBar({ children, border = false, theme }: { children: ReactNode; border?: boolean; theme?: PluginTheme }) {
  return <ScrollView horizontal showsHorizontalScrollIndicator={false} keyboardShouldPersistTaps="handled" style={{ flexGrow: 0, flexShrink: 0, ...(border && theme ? { borderBottomWidth: hairline, borderBottomColor: theme.colors.border } : {}) }}
    contentContainerStyle={{ flexDirection: "row", alignItems: "center", gap: space.sm, paddingHorizontal: space.md, paddingVertical: space.sm }}>{children}</ScrollView>;
}

export type Option<Value extends string> = { value: Value; label: string; count?: number; icon?: string; tone?: Tone };
/** 2–4 mutually exclusive views of the same content. */
/** @aiPurpose Compose Segmented presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Segmented<Value extends string>({ theme, value, options, onChange, size = "md", accessibilityLabel }: {
  theme: PluginTheme; value: Value; options: readonly Option<Value>[]; onChange: (value: Value) => void; size?: "sm" | "md"; accessibilityLabel?: string;
}) {
  const { colors } = theme;
  return <View accessibilityRole="tablist" accessibilityLabel={accessibilityLabel} style={{ flexDirection: "row", alignSelf: "flex-start", padding: space.xxs, gap: space.xxs, borderRadius: radius.md, backgroundColor: colors.surface2 }}>
    {options.map(option => {
      const selected = option.value === value;
      return <Pressable {...skinAttributes("tab")} key={option.value} accessibilityRole="tab" accessibilityLabel={option.count === undefined ? option.label : `${option.label} ${option.count}`} accessibilityState={{ selected }} aria-selected={selected} onPress={() => onChange(option.value)} hitSlop={{ top: 6, bottom: 6 }}
        style={state => ({ flexDirection: "row", alignItems: "center", gap: space.xs + 1, minHeight: size === "sm" ? 24 : control.md - 2 * space.xxs, paddingHorizontal: size === "sm" ? space.sm : 10, borderRadius: radius.sm, backgroundColor: selected ? colors.surface0 : hoverable(state).hovered ? colors.surface1 : "transparent", borderWidth: selected ? hairline : 0, borderColor: colors.border, ...skinStyle(theme, "tab", selected, selected ? colors.surface0 : colors.surface1) })}>
        {option.icon ? <Icon name={option.icon} size={14} color={option.tone ? toneColor(theme, option.tone, "mark") : selected ? colors.foreground : colors.foregroundMuted} /> : null}
        <Text numberOfLines={1} style={{ color: selected ? colors.foreground : colors.foregroundMuted, fontSize: fontSize.small, fontWeight: selected ? "500" : "400" }}>{option.label}</Text>
        {option.count !== undefined ? <Text style={{ color: option.tone && option.count ? toneColor(theme, option.tone) : colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: option.tone && option.count ? "600" : "400", fontVariant: ["tabular-nums"] }}>{option.count}</Text> : null}
      </Pressable>;
    })}
  </View>;
}

/** Section tabs inside a detail pane (讨论 / 文件 / 检查 …). */
/** @aiPurpose Compose Tabs presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Tabs<Value extends string>({ theme, value, options, onChange }: { theme: PluginTheme; value: Value; options: readonly Option<Value>[]; onChange: (value: Value) => void }) {
  const { colors } = theme;
  return <View accessibilityRole="tablist" style={{ flexDirection: "row", gap: space.lg, borderBottomWidth: hairline, borderBottomColor: colors.border }}>
    {options.map(option => {
      const selected = option.value === value;
      return <Pressable {...skinAttributes("tab")} key={option.value} accessibilityRole="tab" accessibilityLabel={option.count === undefined ? option.label : `${option.label} ${option.count}`} accessibilityState={{ selected }} aria-selected={selected} onPress={() => onChange(option.value)}
        style={{ flexDirection: "row", alignItems: "center", gap: space.xs + 1, minHeight: 40, marginBottom: -hairline, borderBottomWidth: 2, borderBottomColor: selected ? option.tone ? toneColor(theme, option.tone, "mark") : colors.foreground : "transparent" }}>
        {option.icon ? <Icon name={option.icon} size={14} color={option.tone ? toneColor(theme, option.tone, "mark") : selected ? colors.foreground : colors.foregroundMuted} /> : null}
        <Text style={{ color: selected ? colors.foreground : colors.foregroundMuted, fontSize: fontSize.body, fontWeight: selected ? "500" : "400" }}>{option.label}</Text>
        {option.count !== undefined ? <Text style={{ color: option.tone && option.count ? toneColor(theme, option.tone) : colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: option.tone && option.count ? "600" : "400", fontVariant: ["tabular-nums"] }}>{option.count}</Text> : null}
      </Pressable>;
    })}
  </View>;
}

/** Filter / quick-scope pill. With `onRemove` it becomes an applied-filter token. */
/** @aiPurpose Compose Chip presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Chip({ theme, label, icon, count, tone, selected = false, onPress, onRemove, testID, accessibilityLabel, disabled = false, tooltip }: {
  theme: PluginTheme; label: string; icon?: string; count?: number; tone?: Tone; selected?: boolean; onPress?: () => void; onRemove?: () => void; testID?: string; accessibilityLabel?: string;
  disabled?: boolean; tooltip?: string | false;
}) {
  const { colors } = theme, color = tone ? toneColor(theme, tone, "mark") : selected ? colors.foreground : colors.foregroundMuted;
  const hint = useTooltip({ label: tooltip, onPress });
  const body = <>
    {tone && !icon ? <StatusDot theme={theme} tone={tone} size={7} /> : icon ? <Icon name={icon} size={13} color={color} /> : null}
    <Text numberOfLines={1} style={{ flexShrink: 1, minWidth: 0, color: selected || tone ? colors.foreground : colors.foregroundMuted, fontSize: fontSize.small, fontWeight: selected ? "500" : "400" }}>{label}</Text>
    {count !== undefined ? <Text style={{ color: tone && tone !== "neutral" && count ? toneColor(theme, tone) : colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: tone && count ? "600" : "400", fontVariant: ["tabular-nums"] }}>{count}</Text> : null}
  </>;
  const remove = onRemove ? <Pressable accessibilityRole="button" accessibilityLabel={t("kitUi.remove", { label })} accessibilityState={{ disabled }} disabled={disabled} onPress={onRemove} hitSlop={8}><Icon name="X" size={12} color={colors.foregroundMuted} /></Pressable> : null;
  const tinted = tone && tone !== "neutral";
  const style = (hovered = false): ViewStyle => ({ flexDirection: "row", alignItems: "center", maxWidth: "100%", minWidth: 0, gap: space.xs + 1, minHeight: control.sm, paddingHorizontal: 10, borderRadius: radius.full, borderWidth: hairline,
    borderColor: selected ? tinted ? toneColor(theme, tone, "mark") : colors.foregroundMuted : tinted ? toneColor(theme, tone, "line") : colors.border,
    backgroundColor: selected ? tinted ? toneColor(theme, tone, "soft") : colors.surface2 : hovered && !disabled ? colors.surface1 : "transparent", opacity: disabled ? 0.45 : 1, ...skinStyle(theme, "chip", selected, selected ? tinted ? toneColor(theme, tone, "soft") : colors.surface2 : hovered && !disabled ? colors.surface1 : colors.surface0, tinted ? toneColor(theme, tone, "mark") : undefined) });
  const pressLabel = accessibilityLabel ?? (count === undefined ? label : `${label} ${count}`);
  if (!onPress) return <View {...skinAttributes("chip")} testID={testID} style={style()}>{body}{remove}</View>;
  if (!remove) return <Pressable {...skinAttributes("chip")} {...hint} testID={testID} accessibilityRole="button" accessibilityLabel={pressLabel} accessibilityState={{ selected, disabled }} {...aria({ pressed: selected })} disabled={disabled} hitSlop={{ top: 6, bottom: 6 }} style={state => [style(hoverable(state).hovered), { opacity: disabled ? 0.45 : hoverable(state).pressed ? 0.8 : 1 }]}>{body}</Pressable>;
  // Both actions: two sibling buttons inside one pill; disabled preserves the same information and identity.
  return <View {...skinAttributes("chip")} testID={testID} style={[style(), { paddingHorizontal: 0, gap: 0 }]}>
    <Pressable {...hint} accessibilityRole="button" accessibilityLabel={pressLabel} accessibilityState={{ selected, disabled }} {...aria({ pressed: selected })} disabled={disabled} hitSlop={{ top: 6, bottom: 6 }}
      style={state => ({ flexDirection: "row", alignItems: "center", flexShrink: 1, minWidth: 0, gap: space.xs + 1, minHeight: control.sm, paddingLeft: 10, paddingRight: space.xs, opacity: hoverable(state).pressed ? 0.8 : 1 })}>{body}</Pressable>
    <View style={{ paddingRight: 10 }}>{remove}</View>
  </View>;
}

export interface ChoiceOption<Value extends string | null> {
  value: Value; label: string; count?: number; tone?: Tone; disabled?: boolean;
  /** A secondary line in rows; included in the tooltip in either layout. */
  hint?: string;
  /** Row decoration, such as a project mark. Wrapped choices use the tone mark. */
  leading?: ReactNode;
}

/**
 * @aiPurpose Controlled single selection for filter groups and sheet pickers.
 * @aiUsage Keep scope resets and unavailable selected options in the domain owner. Values must be unique.
 * No internal selection state; null and the empty string retain their distinct domain meanings.
 */
/** @aiPurpose Compose ChoiceGroup presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ChoiceGroup<Value extends string | null>({ theme, options, value, onChange, layout = "rows", label }: {
  theme: PluginTheme; options: readonly ChoiceOption<Value>[]; value: Value; onChange: (value: Value) => void;
  layout?: "rows" | "wrap"; label?: string;
}) {
  return <View style={{ gap: space.sm }}>
    {label ? <Text style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: "500" }}>{label}</Text> : null}
    <View accessibilityLabel={label} style={layout === "wrap" ? { flexDirection: "row", flexWrap: "wrap", gap: space.sm } : { marginHorizontal: -space.sm }}>
      {options.map(option => <ChoiceItem key={JSON.stringify(option.value)} theme={theme} option={option} selected={option.value === value} onChange={onChange} layout={layout} groupLabel={label} />)}
    </View>
  </View>;
}

function ChoiceItem<Value extends string | null>({ theme, option, selected, onChange, layout, groupLabel }: {
  theme: PluginTheme; option: ChoiceOption<Value>; selected: boolean; onChange: (value: Value) => void;
  layout: "rows" | "wrap"; groupLabel?: string;
}) {
  const optionName = layout === "wrap" && option.count !== undefined ? `${option.label} ${option.count}` : option.label;
  const name = groupLabel ? `${groupLabel}: ${optionName}` : optionName;
  const description = option.hint ? `${name} · ${option.hint}` : name;
  const choose = () => { if (!option.disabled) onChange(option.value); };
  if (layout === "wrap") return <Chip theme={theme} label={option.label} count={option.count} tone={option.tone} selected={selected} disabled={option.disabled}
    accessibilityLabel={groupLabel ? name : undefined} tooltip={description} onPress={choose} />;
  return <ChoiceRow theme={theme} option={option} selected={selected} name={name} description={description} onPress={choose} />;
}

function ChoiceRow({ theme, option, selected, name, description, onPress }: {
  theme: PluginTheme; option: ChoiceOption<string | null>; selected: boolean; name: string; description: string; onPress: () => void;
}) {
  const { colors } = theme;
  const hint = useTooltip({ label: description, onPress });
  return <Pressable {...hint} accessibilityRole="button" accessibilityLabel={name} accessibilityHint={option.hint} accessibilityState={{ selected, disabled: option.disabled ?? false }}
    {...aria({ pressed: selected })} disabled={option.disabled} hitSlop={touchSlop}
    style={state => ({ flexDirection: "row", alignItems: "center", gap: space.md, minHeight: 44, paddingHorizontal: space.sm, borderRadius: radius.md,
      opacity: option.disabled ? 0.45 : state.pressed ? 0.8 : 1, backgroundColor: selected ? colors.surface2 : hoverable(state).hovered ? colors.surface1 : "transparent", ...skinStyle(theme, "row", selected) })}>
    {option.leading ?? null}
    <View style={{ flex: 1, minWidth: 0, gap: space.xxs }}>
      <Text numberOfLines={1} style={{ color: colors.foreground, fontSize: fontSize.body, fontWeight: selected ? "500" : "400" }}>{option.label}</Text>
      {option.hint ? <Text numberOfLines={1} style={{ color: colors.foregroundMuted, fontSize: fontSize.meta }}>{option.hint}</Text> : null}
    </View>
    {option.count !== undefined ? <Text style={{ color: colors.foregroundMuted, fontSize: fontSize.meta, fontVariant: ["tabular-nums"] }}>{option.count}</Text> : null}
    <View style={{ width: 16 }}>{selected ? <Icon name="Check" size={16} color={colors.foreground} /> : null}</View>
  </Pressable>;
}

/** Small label: state, count or category, on a ground tinted with its tone (`solid` for the loudest ones). */
/** @aiPurpose Compose Badge presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Badge({ theme, label, tone = "neutral", icon, solid = false }: { theme: PluginTheme; label: string; tone?: Tone; icon?: string; solid?: boolean }) {
  const loud = solid && tone !== "neutral", color = loud ? solidForeground(theme, toneColor(theme, tone, "mark")) : toneColor(theme, tone);
  return <View {...skinAttributes("badge")} style={{ flexDirection: "row", alignItems: "center", gap: space.xs, alignSelf: "flex-start", minHeight: 20, paddingHorizontal: space.xs + 2, borderRadius: radius.sm - 2, backgroundColor: loud ? toneColor(theme, tone, "mark") : toneColor(theme, tone, "soft"), ...skinStyle(theme, "badge") }}>
    {icon ? <Icon name={icon} size={12} color={loud ? color : toneColor(theme, tone, "mark")} /> : null}
    <Text numberOfLines={1} style={{ color: tone === "neutral" ? theme.colors.foregroundMuted : color, fontSize: fontSize.meta, fontWeight: "500" }}>{label}</Text>
  </View>;
}

/** @aiPurpose Compose HueDot presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function HueDot({ theme, hue: name, size = 8 }: { theme: PluginTheme; hue: Hue; size?: number }) {
  return <View accessibilityElementsHidden importantForAccessibility="no" style={{ width: size, height: size, borderRadius: size / 2, ...skinStyle(theme, "dot"), backgroundColor: hue(theme, name) }} />;
}
/** Colored initial for a project / repository / workspace; the color is stable per name (`hueFor`). */
/** @aiPurpose Compose ProjectMark presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ProjectMark({ theme, name, size = 18 }: { theme: PluginTheme; name: string; size?: number }) {
  const initial = Array.from(name.trim().replace(/^[^\p{L}\p{N}]+/u, ""))[0]?.toUpperCase() ?? "#";
  return <View accessibilityElementsHidden importantForAccessibility="no" style={{ width: size, height: size, borderRadius: Math.round(size * 0.3), alignItems: "center", justifyContent: "center", backgroundColor: hue(theme, hueFor(name)), ...skinStyle(theme, "badge") }}>
    <Text style={{ color: solidForeground(theme, hue(theme, hueFor(name))), fontSize: Math.round(size * 0.6), lineHeight: size, fontWeight: "600", textAlign: "center" }}>{initial}</Text>
  </View>;
}

/** "a · b · c" secondary line; falsy parts are skipped. */
/** @aiPurpose Compose MetaLine presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function MetaLine({ theme, parts, numberOfLines = 1, selectable = false }: { theme: PluginTheme; parts: readonly (string | number | null | undefined | false)[]; numberOfLines?: number; selectable?: boolean }) {
  const text = parts.filter(part => part !== null && part !== undefined && part !== false && part !== "").join(t("kitUi.metaSeparator"));
  return text ? <Text selectable={selectable} numberOfLines={numberOfLines} style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: 17 }}>{text}</Text> : null;
}

/** The list primitive: whole row pressable, title first, one muted meta line, optional trailing time/badge. */
/** @aiPurpose Compose Row presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Row({ theme, title, subtitle, leading, trailing, onPress, selected = false, titleLines = 2, children, accessibilityLabel, testID }: {
  theme: PluginTheme; title: string; subtitle?: ReactNode; leading?: ReactNode; trailing?: ReactNode; onPress?: () => void; selected?: boolean; titleLines?: number;
  children?: ReactNode; accessibilityLabel?: string; testID?: string;
}) {
  const { colors } = theme;
  const content = <>
    {leading ? <View style={{ minHeight: 20, justifyContent: "center" }}>{leading}</View> : null}
    <View style={{ flex: 1, minWidth: 0, gap: 3 }}>
      <View style={{ flexDirection: "row", alignItems: "flex-start", gap: space.sm }}>
        <Text numberOfLines={titleLines} style={{ flex: 1, color: colors.foreground, fontSize: fontSize.body, lineHeight: 20, fontWeight: "500" }}>{title}</Text>
        {trailing ? <View style={{ minHeight: 20, justifyContent: "center", alignItems: "flex-end" }}>{trailing}</View> : null}
      </View>
      {typeof subtitle === "string" ? <Text numberOfLines={1} style={{ color: colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: 17 }}>{subtitle}</Text> : subtitle}
      {children}
    </View>
  </>;
  const frame: ViewStyle = { flexDirection: "row", alignItems: "flex-start", gap: 10, paddingVertical: 10, paddingHorizontal: space.md, borderRadius: radius.md, ...skinStyle(theme, "row", selected) };
  if (!onPress) return <View testID={testID} style={frame}>{content}</View>;
  return <Pressable testID={testID} accessibilityRole="button" accessibilityLabel={accessibilityLabel ?? title} accessibilityState={{ selected }} {...aria({ current: selected ? "true" : undefined })} onPress={onPress}
    style={state => { const { pressed, hovered } = hoverable(state); return [frame, { backgroundColor: selected ? colors.surface2 : pressed || hovered ? colors.surface1 : "transparent" }]; }}>{content}</Pressable>;
}

/** Group header inside a list. Collapsible when `open`/`onToggle` are given. */
/** @aiPurpose Compose SectionHeader presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SectionHeader({ theme, title, count, action, open, onToggle, icon, tone }: { theme: PluginTheme; title: string; count?: number; action?: ReactNode; open?: boolean; onToggle?: () => void; icon?: string; tone?: Tone }) {
  const { colors } = theme;
  const tinted = tone && tone !== "neutral" ? toneColor(theme, tone) : null;
  const label = <View style={{ flexDirection: "row", alignItems: "center", gap: space.xs + 2, flex: 1 }}>
    {onToggle ? <Icon name={open ? "ChevronDown" : "ChevronRight"} size={14} color={colors.foregroundMuted} /> : null}
    {icon ? <Icon name={icon} size={13} color={tone ? toneColor(theme, tone, "mark") : colors.foregroundMuted} /> : null}
    <Text style={{ color: tinted ?? colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: tinted ? "600" : "500" }}>{title}</Text>
    {count !== undefined ? <Text style={{ color: colors.foregroundMuted, fontSize: fontSize.meta, fontVariant: ["tabular-nums"], opacity: 0.8 }}>{count}</Text> : null}
  </View>;
  return <View style={{ flexDirection: "row", alignItems: "center", gap: space.sm, minHeight: 32, paddingHorizontal: space.md, paddingTop: space.sm }}>
    {onToggle ? <Pressable accessibilityRole="button" accessibilityLabel={t(open ? "kitUi.collapse" : "kitUi.expand", { title })} accessibilityState={{ expanded: open }} {...aria({ expanded: open })} onPress={onToggle} style={{ flex: 1 }}>{label}</Pressable> : label}
    {action}
  </View>;
}

/** @aiPurpose Compose Divider presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Divider({ theme, inset = 0 }: { theme: PluginTheme; inset?: number }) {
  return <View style={{ height: hairline, marginLeft: inset, backgroundColor: theme.colors.border }} />;
}

/** One-line inline message. Use for errors and states that need a decision; prefer toasts for confirmations. */
/** @aiPurpose Compose Banner presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Banner({ theme, tone = "neutral", message, icon, action, onDismiss }: {
  theme: PluginTheme; tone?: Tone; message: string; icon?: string; action?: { label: string; onPress: () => void }; onDismiss?: () => void;
}) {
  const { colors } = theme, color = toneColor(theme, tone);
  // Message text stays foreground: red text on a red-tinted ground falls under 4.5:1 on dark themes.
  const glyph = icon ?? (tone === "danger" ? "CircleAlert" : tone === "warning" ? "TriangleAlert" : tone === "success" ? "CircleCheck" : "Info");
  const tinted = tone !== "neutral";
  return <View accessibilityRole={tone === "danger" ? "alert" : undefined} style={{ flexDirection: "row", alignItems: "center", gap: space.sm, paddingVertical: space.sm, paddingLeft: space.md, paddingRight: action || onDismiss ? space.xs : space.md, borderRadius: radius.md, backgroundColor: tinted ? toneColor(theme, tone, "soft") : colors.surface1, borderWidth: hairline, borderColor: tinted ? toneColor(theme, tone, "line") : colors.border, ...skinStyle(theme, "card") }}>
    <Icon name={glyph} size={16} color={tinted ? toneColor(theme, tone, "mark") : color} />
    <Text selectable style={{ flex: 1, color: colors.foreground, fontSize: fontSize.small, lineHeight: 19, fontWeight: tone === "danger" ? "500" : "400" }}>{message}</Text>
    {action ? <Button theme={theme} label={action.label} size="sm" variant="ghost" onPress={action.onPress} /> : null}
    {onDismiss ? <IconButton theme={theme} icon="X" label={t("kitUi.dismiss")} size="sm" onPress={onDismiss} /> : null}
  </View>;
}

/** @aiPurpose Compose EmptyState presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function EmptyState({ theme, icon, title, message, action, tone = "info" }: { theme: PluginTheme; icon: string; title: string; message?: string; action?: { label: string; onPress: () => void; icon?: string }; tone?: Tone }) {
  const { colors } = theme;
  return <View style={{ alignItems: "center", justifyContent: "center", gap: space.sm, paddingHorizontal: space.xl, paddingVertical: 40 }}>
    <View style={{ width: 44, height: 44, borderRadius: radius.lg, alignItems: "center", justifyContent: "center", backgroundColor: toneColor(theme, tone, "soft"), ...skinStyle(theme, "badge"), marginBottom: space.xs }}><Icon name={icon} size={22} color={tone === "neutral" ? colors.foregroundMuted : toneColor(theme, tone, "mark")} /></View>
    <Text style={{ color: colors.foreground, fontSize: fontSize.body, fontWeight: "500", textAlign: "center" }}>{title}</Text>
    {message ? <Text style={{ color: colors.foregroundMuted, fontSize: fontSize.small, lineHeight: 19, textAlign: "center", maxWidth: 300 }}>{message}</Text> : null}
    {action ? <View style={{ marginTop: space.xs }}><Button theme={theme} label={action.label} icon={action.icon} size="sm" onPress={action.onPress} /></View> : null}
  </View>;
}

/** First-load placeholder shaped like `Row`s. Background refreshes must not show it. */
/** @aiPurpose Compose Skeleton presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Skeleton({ theme, rows = 4 }: { theme: PluginTheme; rows?: number }) {
  const bar = (width: `${number}%`, height: number) => <View style={{ width, height, borderRadius: 4, backgroundColor: theme.colors.surface2 }} />;
  return <View accessibilityRole="progressbar" accessibilityLabel={t("kitUi.loading")} style={{ gap: space.xs, paddingVertical: space.xs }}>
    {Array.from({ length: rows }, (_, index) => <View key={index} style={{ flexDirection: "row", gap: 10, paddingVertical: 10, paddingHorizontal: space.md, opacity: 1 - index * 0.15 }}>
      <View style={{ width: 8, height: 8, marginTop: 6, borderRadius: 4, backgroundColor: theme.colors.surface2 }} />
      <View style={{ flex: 1, gap: 7 }}>{bar(`${88 - (index % 3) * 14}%`, 12)}{bar(`${46 + (index % 2) * 12}%`, 9)}</View>
    </View>)}
  </View>;
}

/** Quiet sync state for toolbars: a dot plus optional short label ("同步中", "3 分钟前"). */
/** @aiPurpose Compose SyncStatus presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SyncStatus({ theme, state, label }: { theme: PluginTheme; state: "live" | "syncing" | "stale" | "error"; label?: string }) {
  const tone: Tone = state === "live" ? "success" : state === "error" ? "danger" : state === "stale" ? "warning" : "neutral";
  const spoken = label ?? t(state === "live" ? "kitUi.syncLive" : state === "syncing" ? "kitUi.syncSyncing" : state === "stale" ? "kitUi.syncStale" : "kitUi.syncError");
  return <View accessibilityLabel={spoken} style={{ flexDirection: "row", alignItems: "center", gap: space.xs + 2 }}>
    {state === "syncing" ? <ActivityIndicator size="small" color={theme.colors.foregroundMuted} /> : <StatusDot theme={theme} tone={tone} size={6} />}
    {label ? <Text numberOfLines={1} style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta }}>{label}</Text> : null}
  </View>;
}

/**
 * Autofocus once the host sheet has finished opening (about 300 ms): focusing during the opening animation makes
 * web scroll the half-laid-out form, and on phones the keyboard pushes a sheet that is still moving.
 */
/** @aiPurpose Provide useSettledFocus presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useSettledFocus(enabled: boolean, delay = 350) {
  const ref = useRef<NativeTextInput>(null);
  useEffect(() => {
    if (!enabled) return;
    const timer = setTimeout(() => ref.current?.focus(), delay);
    return () => clearTimeout(timer);
  }, [enabled, delay]);
  return ref;
}

/** `grow` fills a row. It keeps the content-sized basis: `flex: 1` (basis 0) collapses the field's parent on native, so whatever follows overlaps it. */
/** @aiPurpose Compose SearchField presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SearchField({ theme, value, onChangeText, placeholder = t("kitUi.search"), onSubmit, autoFocus = false, accessibilityLabel, grow = true }: {
  theme: PluginTheme; value: string; onChangeText: (value: string) => void; placeholder?: string; onSubmit?: () => void; autoFocus?: boolean; accessibilityLabel?: string; grow?: boolean;
}) {
  const { colors } = theme, input = useSettledFocus(autoFocus);
  return <View {...skinAttributes("field")} style={{ flexDirection: "row", alignItems: "center", gap: space.xs + 2, minHeight: control.md, paddingLeft: 10, paddingRight: value ? space.xxs : 10, borderRadius: radius.md, backgroundColor: colors.surface2, ...(grow ? { flexGrow: 1, flexShrink: 1, minWidth: 0 } : {}), ...skinStyle(theme, "field", false, colors.surface2) }}>
    <Icon name="Search" size={14} color={colors.foregroundMuted} />
    <TextInput ref={input} accessibilityLabel={accessibilityLabel ?? placeholder} value={value} onChangeText={onChangeText} onSubmitEditing={onSubmit} placeholder={placeholder} placeholderTextColor={colors.foregroundMuted}
      autoCapitalize="none" autoCorrect={false} returnKeyType="search" style={{ flex: 1, minWidth: 0, color: colors.foreground, fontSize: fontSize.body, paddingVertical: 6 }} />
    {value ? <IconButton theme={theme} icon="X" label={t("kitUi.clearSearch")} size="sm" onPress={() => onChangeText("")} /> : null}
  </View>;
}

/** @aiPurpose Compose TextField presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function TextField({ theme, label, value, onChangeText, placeholder, multiline = false, minHeight, autoFocus = false, editable = true, hint }: {
  theme: PluginTheme; label: string; value: string; onChangeText: (value: string) => void; placeholder?: string; multiline?: boolean; minHeight?: number; autoFocus?: boolean; editable?: boolean; hint?: string;
}) {
  const { colors } = theme, input = useSettledFocus(autoFocus && editable);
  return <View style={{ gap: space.xs + 2 }}>
    <Text style={{ color: colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: "500" }}>{label}</Text>
    <View {...skinAttributes("field")} style={{ borderRadius: radius.md, backgroundColor: colors.surface2, opacity: editable ? 1 : 0.6, ...skinStyle(theme, "field", false, colors.surface2) }}>
      <TextInput ref={input} accessibilityLabel={label} value={value} onChangeText={onChangeText} placeholder={placeholder} placeholderTextColor={colors.foregroundMuted} multiline={multiline} editable={editable}
        textAlignVertical={multiline ? "top" : "center"} style={{ color: colors.foreground, borderRadius: radius.md, minHeight: minHeight ?? (multiline ? 120 : 36), paddingHorizontal: space.md, paddingVertical: space.sm, fontSize: fontSize.body, lineHeight: 20 }} />
    </View>
    {hint ? <Text style={{ color: colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: 17 }}>{hint}</Text> : null}
  </View>;
}

/** Host-styled modal: a bottom sheet on phones, a dialog on desktop. */
/** @aiPurpose Compose Sheet presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Sheet({ title, open, onClose, children, scrollable = true }: { title: string; open: boolean; onClose: () => void; children: ReactNode; scrollable?: boolean }) {
  return <Modal title={title} open={open} onOpenChange={next => { if (!next) onClose(); }}>
    <OverlayBoundary mode={scrollable ? "content" : "surface"} modal active={open}>
      <Modal.Content scrollable={scrollable} contentContainerStyle={{ padding: space.lg, gap: space.md }}>{children}</Modal.Content>
    </OverlayBoundary>
  </Modal>;
}

export type MenuAction = { key: string; label: string; icon?: string; onPress: () => void; tone?: Tone; disabled?: boolean; hint?: string };
/** Overflow ("···") actions. Keep frequent actions out of here — they belong in the toolbar. */
/** @aiPurpose Compose ActionMenu presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ActionMenu({ theme, title, open, onClose, actions }: { theme: PluginTheme; title: string; open: boolean; onClose: () => void; actions: readonly MenuAction[] }) {
  const { colors } = theme;
  return <Modal title={title} open={open} onOpenChange={next => { if (!next) onClose(); }}>
    <OverlayBoundary mode="content" modal active={open}>
    <Modal.Content contentContainerStyle={{ paddingHorizontal: space.sm, paddingVertical: space.sm, gap: 0 }}>
      {actions.map(action => {
        const color = action.tone === "danger" ? toneColor(theme, "danger") : colors.foreground;
        return <Pressable key={action.key} accessibilityRole="button" accessibilityLabel={action.label} accessibilityState={{ disabled: !!action.disabled }} disabled={action.disabled} onPress={() => { onClose(); action.onPress(); }}
          style={state => ({ flexDirection: "row", alignItems: "center", gap: space.md, minHeight: 44, paddingHorizontal: space.md, borderRadius: radius.md, backgroundColor: hoverable(state).pressed || hoverable(state).hovered ? colors.surface1 : "transparent", opacity: action.disabled ? 0.45 : 1 })}>
          {action.icon ? <Icon name={action.icon} size={16} color={action.tone && action.tone !== "neutral" ? toneColor(theme, action.tone, "mark") : colors.foregroundMuted} /> : null}
          <View style={{ flex: 1, gap: 2 }}>
            <Text style={{ color, fontSize: fontSize.body }}>{action.label}</Text>
            {action.hint ? <Text numberOfLines={2} style={{ color: colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: 17 }}>{action.hint}</Text> : null}
          </View>
        </Pressable>;
      })}
    </Modal.Content>
    </OverlayBoundary>
  </Modal>;
}

/** Thumb-reach action bar pinned to the bottom of compact detail views. */
/** @aiPurpose Compose BottomBar presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function BottomBar({ theme, children }: { theme: PluginTheme; children: ReactNode }) {
  return <View style={{ flexDirection: "row", alignItems: "center", gap: space.sm, paddingHorizontal: space.md, paddingTop: space.sm, paddingBottom: space.md, borderTopWidth: hairline, borderTopColor: theme.colors.border, backgroundColor: theme.colors.surface0 }}>{children}</View>;
}

/** List | detail on wide layouts; one pane at a time on compact ones. */
/** @aiPurpose Compose SplitView presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SplitView({ theme, compact, showDetail, list, detail, listWidth = 360 }: { theme: PluginTheme; compact: boolean; showDetail: boolean; list: ReactNode; detail: ReactNode; listWidth?: number }) {
  if (compact) return <View style={{ flex: 1, minHeight: 0 }}>{showDetail ? detail : list}</View>;
  return <View style={{ flex: 1, minHeight: 0, flexDirection: "row" }}>
    <View style={{ width: listWidth, minWidth: 280, minHeight: 0, borderRightWidth: hairline, borderRightColor: theme.colors.border }}>{list}</View>
    <View style={{ flex: 1, minWidth: 0, minHeight: 0 }}>{detail}</View>
  </View>;
}

/** just now / 5 minutes ago / 3 hours ago / yesterday 14:20 / Sep 20 / Sep 20, 2025 (刚刚 / 5 分钟前 / … in Chinese) */
/** @aiPurpose Provide relativeTime presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function relativeTime(value: string | number | Date | null | undefined, now = Date.now()): string {
  if (value === null || value === undefined || value === "") return "";
  const date = value instanceof Date ? value : new Date(value);
  const time = date.getTime();
  if (!Number.isFinite(time)) return "";
  const seconds = Math.round((now - time) / 1000);
  if (seconds < 45) return t("kitTime.justNow");
  if (seconds < 3600) return t("kitTime.minutesAgo", { count: Math.max(1, Math.round(seconds / 60)) });
  const today = new Date(now), sameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();
  const clock = `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
  if (sameDay(date, today)) return seconds < 6 * 3600 ? t("kitTime.hoursAgo", { count: Math.round(seconds / 3600) }) : t("kitTime.todayAt", { time: clock });
  const yesterday = new Date(now - 86_400_000);
  if (sameDay(date, yesterday)) return t("kitTime.yesterdayAt", { time: clock });
  const month = t("kitTime.months").split("|")[date.getMonth()] ?? String(date.getMonth() + 1);
  return date.getFullYear() === today.getFullYear()
    ? t("kitTime.monthDay", { month, day: date.getDate() })
    : t("kitTime.yearMonthDay", { year: date.getFullYear(), month, day: date.getDate() });
}

/** 42s / 3m 20s / 2h 5m (42 秒 / 3 分 20 秒 / 2 小时 5 分 in Chinese) */
/** @aiPurpose Provide formatDuration presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function formatDuration(milliseconds: number): string {
  if (!Number.isFinite(milliseconds) || milliseconds < 0) return "";
  const seconds = Math.round(milliseconds / 1000);
  if (seconds < 60) return t("kitTime.seconds", { count: seconds });
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return seconds % 60 ? t("kitTime.minutesSeconds", { minutes, seconds: seconds % 60 }) : t("kitTime.minutes", { count: minutes });
  const hours = Math.floor(minutes / 60);
  return minutes % 60 ? t("kitTime.hoursMinutes", { hours, minutes: minutes % 60 }) : t("kitTime.hours", { count: hours });
}

/** 980 / 12.3k / 4.5M */
/** @aiPurpose Provide compactNumber presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function compactNumber(value: number): string {
  if (!Number.isFinite(value)) return "";
  const abs = Math.abs(value);
  if (abs < 1000) return String(Math.round(value));
  if (abs < 1_000_000) return `${(value / 1000).toFixed(abs < 10_000 ? 1 : 0).replace(/\.0$/, "")}k`;
  return `${(value / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`;
}

/**
 * @aiPurpose Shared query/filter/clear arrangement in management lists.
 * @aiUsage The page owns query state, filter sheets, host context and list restoration.
 */
/** @aiPurpose Compose SearchControls presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SearchControls({ theme, value, onChangeText, label, filtered, onOpenFilters, onClear, filterLabel, clearLabel }: {
  theme: PluginTheme; value: string; onChangeText: (value: string) => void; label: string; filtered: boolean;
  onOpenFilters: () => void; onClear: () => void; filterLabel: string; clearLabel: string;
}) {
  return <View style={{ flexDirection: "row", alignItems: "center", gap: space.sm, minWidth: 0 }}>
    <SearchField theme={theme} value={value} onChangeText={onChangeText} placeholder={label} accessibilityLabel={label} />
    <IconButton theme={theme} icon="ListFilter" label={filterLabel} active={filtered} onPress={onOpenFilters} />
    {filtered ? <IconButton theme={theme} icon="FilterX" label={clearLabel} onPress={onClear} /> : null}
  </View>;
}

/** @aiPurpose Explicitly disclose longer caveats in a Sheet. @aiUsage Provide a title and plain text lines; use Tooltip for brief control names. */
/** @aiPurpose Compose InfoButton presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function InfoButton({ theme, title, lines }: { theme: PluginTheme; title: string; lines: readonly string[] }) {
  const [open, setOpen] = useState(false);
  return <>
    <IconButton theme={theme} icon="Info" label={title} size="sm" onPress={() => setOpen(true)} />
    <Sheet title={title} open={open} onClose={() => setOpen(false)}>
      {lines.map(line => <Text key={line} style={{ color: theme.colors.foreground, fontSize: fontSize.body, lineHeight: 22 }}>{line}</Text>)}
    </Sheet>
  </>;
}


export { Panel } from '../panel/panel.native';
