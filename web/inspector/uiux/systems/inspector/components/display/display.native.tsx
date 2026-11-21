/** @aiPurpose Present diagnostic values and controlled status components through the shared kit. */
import * as React from "react";
import type { ReactNode } from "react";
import type { PluginTheme } from "../../../../../host/contracts";
import { Icon } from "../../../../../host/ui";
import { Pressable, Text, View } from "react-native";
import { fontSize, hairline, monoFont, space } from "../../foundations/native.tokens";
import { skinAttributes, skinStyle } from "../../themes/skin.native";
import { toneColor, type Tone } from "../../foundations/palette.native";

/** @aiPurpose Diagnostic label/value pair with selectable, wrapping values. @aiUsage The owner supplies the responsive compact layout. */
/** @aiPurpose Compose KeyValue presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function KeyValue({ theme, label, value, mono = false, compact }: { theme: PluginTheme; label: string; value: string; mono?: boolean; compact: boolean }) {
  return <View style={{ flexDirection: compact ? "column" : "row", gap: compact ? 2 : space.md, paddingVertical: space.sm, borderBottomWidth: hairline, borderBottomColor: theme.colors.border }}>
    <Text style={{ width: compact ? undefined : 132, color: theme.colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: 20 }}>{label}</Text>
    <Text selectable style={{ flex: compact ? undefined : 1, minWidth: 0, color: theme.colors.foreground, fontSize: mono ? fontSize.meta : fontSize.small, lineHeight: 20, fontFamily: mono ? monoFont : undefined }}>{value}</Text>
  </View>;
}

export interface Stat { label: string; value: string; tone?: Tone }
/** @aiPurpose Values above muted labels. @aiUsage Supply columns for an even narrow grid; values are already formatted by the owner. */
/** @aiPurpose Compose StatRow presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function StatRow({ theme, stats, size = "md", columns }: { theme: PluginTheme; stats: readonly Stat[]; size?: "md" | "lg"; columns?: number }) {
  return <View style={{ flexDirection: "row", flexWrap: "wrap", columnGap: columns ? 0 : space.xl, rowGap: space.md }}>
    {stats.map(stat => <View key={stat.label} style={{ gap: 2, minWidth: 64, ...(columns ? { width: `${100 / columns}%` as const, paddingRight: space.sm } : {}) }}>
      <Text numberOfLines={1} style={{ color: stat.tone ? toneColor(theme, stat.tone) : theme.colors.foreground, fontSize: size === "lg" ? fontSize.heading : fontSize.title, fontWeight: "500" }}>{stat.value}</Text>
      <Text numberOfLines={1} style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta }}>{stat.label}</Text>
    </View>)}
  </View>;
}

/** @aiPurpose Quiet section label inside detail panes. @aiUsage Use SectionHeader for expandable list groups. */
/** @aiPurpose Compose PaneLabel presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function PaneLabel({ theme, children, trailing, icon, tone }: { theme: PluginTheme; children: string; trailing?: ReactNode; icon?: string; tone?: Tone }) {
  return <View style={{ flexDirection: "row", alignItems: "center", gap: space.sm, minHeight: 28 }}>
    {icon ? <Icon name={icon} size={13} color={tone ? toneColor(theme, tone, "mark") : theme.colors.foregroundMuted} /> : null}
    <Text style={{ flex: 1, color: tone ? toneColor(theme, tone) : theme.colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: tone ? "600" : "500" }}>{children}</Text>
    {trailing}
  </View>;
}

/** @aiPurpose Controlled boolean switch with native and Web checked semantics. @aiUsage The owner keeps the value and persists changes. */
/** @aiPurpose Compose Switch presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Switch({ theme, label, value, onChange, disabled = false }: { theme: PluginTheme; label: string; value: boolean; onChange: (value: boolean) => void; disabled?: boolean }) {
  const { colors } = theme;
  return <Pressable {...skinAttributes("switch")} accessibilityRole="switch" accessibilityLabel={label} accessibilityState={{ checked: value, disabled }} aria-checked={value} aria-disabled={disabled} disabled={disabled} onPress={() => onChange(!value)} hitSlop={8}
    style={state => ({ opacity: disabled ? 0.45 : state.pressed ? 0.8 : 1 })}>
    <View style={{ width: 36, height: 20, borderRadius: 10, padding: 2, alignItems: value ? "flex-end" : "flex-start", justifyContent: "center", backgroundColor: value ? colors.accent : colors.surface2, borderWidth: value ? 0 : hairline, borderColor: colors.border, ...skinStyle(theme, "chip") }}>
      <View style={{ width: 16, height: 16, borderRadius: 8, backgroundColor: value ? colors.accentForeground : colors.foregroundMuted, ...skinStyle(theme, "dot") }} />
    </View>
  </Pressable>;
}

/** @aiPurpose Noninteractive semantic status mark. @aiUsage Use a tone without coupling to a domain record. */
/** @aiPurpose Compose StatusDot presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function StatusDot({ theme, tone, size = 8 }: { theme: PluginTheme; tone: Tone; size?: number }) {
  return <View accessibilityElementsHidden importantForAccessibility="no" style={{ width: size, height: size, borderRadius: size / 2, ...skinStyle(theme, "dot"), backgroundColor: tone === "neutral" ? theme.colors.foregroundMuted : toneColor(theme, tone, "mark"), opacity: tone === "neutral" ? 0.6 : 1 }} />;
}

/** A dot in any palette hue (legends, categories, project colors). */
/** @aiPurpose Accent work mark uses a ring so it differs from success in host light themes. @aiUsage Use for semantic work phases, not arbitrary categories. */
/** @aiPurpose Compose StateDot presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function StateDot({ theme, tone, size = 8 }: { theme: PluginTheme; tone: Tone; size?: number }) {
  if (tone !== "accent") return <StatusDot theme={theme} tone={tone} size={size} />;
  return <View accessibilityElementsHidden importantForAccessibility="no" style={{ width: size, height: size, borderRadius: size / 2, ...skinStyle(theme, "dot"), borderWidth: 2, borderColor: toneColor(theme, "accent", "mark") }} />;
}

