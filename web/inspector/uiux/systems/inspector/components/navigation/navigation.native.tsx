/** @aiPurpose Keep navigation and tooltips stable while switching presentation skins. */
import * as React from "react";
import { Pressable, Text, View } from "react-native";
import type { PluginTheme } from "../../../../../host/contracts";
import { Icon, ScrollView } from "../../../../../host/ui";
import { fontSize, glow, gradient, hue, isDarkTheme, radius, type Hue } from "../kit/kit.native";
import { skinAttributes, skinStyle } from "../../themes/skin.native";
import { solidForeground } from "../../foundations/palette.native";
import { useTooltip } from "../overlay/overlay.native";

export interface ViewTab<Value extends string> { value: Value; label: string; icon: string; colors: readonly [Hue, Hue]; count?: number }
type Interaction = { pressed: boolean; hovered?: boolean };

/**
 * Main navigation: the selected view is a gradient pill with its own colours; the others show a coloured icon.
 * `iconOnly` (phones) hides the labels of unselected views — their names stay in the accessibility label — and the
 * row scrolls sideways instead of wrapping when it still does not fit.
 */
/** @aiPurpose Compose ViewTabs presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ViewTabs<Value extends string>({ theme, value, tabs, onChange, iconOnly = false, accessibilityLabel }: {
  theme: PluginTheme; value: Value; tabs: readonly ViewTab<Value>[]; onChange: (value: Value) => void; iconOnly?: boolean; accessibilityLabel?: string;
}) {
  return <ScrollView horizontal showsHorizontalScrollIndicator={false} style={{ flexGrow: 0, flexShrink: 1 }} contentContainerStyle={{ alignItems: "center", paddingVertical: 3, paddingHorizontal: 2 }}>
    <View accessibilityRole="tablist" accessibilityLabel={accessibilityLabel} style={{ flexDirection: "row", alignItems: "center", gap: iconOnly ? 2 : 4 }}>
      {tabs.map(tab => <ViewTabItem key={tab.value} theme={theme} tab={tab} selected={tab.value === value} iconOnly={iconOnly} onChange={onChange} />)}
    </View>
  </ScrollView>;
}

/** Stable item identity keeps the shared tooltip lifecycle independent of sibling navigation. */
function ViewTabItem<Value extends string>({ theme, tab, selected, iconOnly, onChange }: {
  theme: PluginTheme; tab: ViewTab<Value>; selected: boolean; iconOnly: boolean; onChange: (value: Value) => void;
}) {
  const { colors } = theme;
  const selectedForeground = solidForeground(theme, hue(theme, tab.colors[0]));
  const bare = iconOnly && !selected;
  const name = tab.count === undefined ? tab.label : `${tab.label} ${tab.count}`;
  const hint = useTooltip({ label: name, onPress: () => onChange(tab.value) });
  return <Pressable {...skinAttributes("tab")} {...hint} accessibilityRole="tab" accessibilityLabel={name} accessibilityState={{ selected }} aria-selected={selected} hitSlop={{ top: 6, bottom: 6 }}
    style={state => {
      const { hovered, pressed } = state as Interaction;
      return [{ flexDirection: "row", alignItems: "center", justifyContent: "center", gap: 5, height: 30, minWidth: bare ? 30 : undefined, paddingHorizontal: bare ? 0 : 11, borderRadius: radius.full, opacity: pressed ? 0.8 : 1 },
        selected ? [gradient(theme, tab.colors, { angle: 120 }), glow(theme, hue(theme, tab.colors[1]), 12)] : { backgroundColor: hovered ? hue(theme, tab.colors[0], "soft") : "transparent" }, skinStyle(theme, "tab", selected, selected ? hue(theme, tab.colors[0]) : hovered ? hue(theme, tab.colors[0], "soft") : colors.surface0, selected ? hue(theme, tab.colors[0]) : undefined)];
    }}>
    <Icon name={tab.icon} size={15} color={selected ? selectedForeground : hue(theme, tab.colors[0], isDarkTheme(theme) ? "solid" : "text")} />
    {bare ? null : <Text numberOfLines={1} style={{ color: selected ? selectedForeground : colors.foregroundMuted, fontSize: fontSize.small, fontWeight: selected ? "800" : "500" }}>{tab.label}</Text>}
    {!bare && tab.count !== undefined ? <Text style={{ color: selected ? selectedForeground : colors.foregroundMuted, opacity: selected ? 0.85 : 1, fontSize: fontSize.meta, fontWeight: "700", fontVariant: ["tabular-nums"] }}>{tab.count}</Text> : null}
  </Pressable>;
}
