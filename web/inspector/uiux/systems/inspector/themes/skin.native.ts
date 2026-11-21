/** @aiPurpose Define opt-in presentation skins without changing host contracts, layout or domain state. */
import type { PluginTheme } from "../../../../host/contracts";
import { Platform, type ViewStyle } from "react-native";

export type VisualStyle = "original" | "pixel";
export type InspectorTheme = PluginTheme & { inspectorStyle?: VisualStyle };
export const isPixelTheme = (theme: PluginTheme): boolean => (theme as InspectorTheme).inspectorStyle === "pixel";

import { pixelShape } from "../foundations/foundation.tokens";
export { pixelColors, pixelHues, pixelShape, pixelMotion } from "../foundations/foundation.tokens";
export type SkinRole = "button" | "icon-button" | "chip" | "tab" | "badge" | "field" | "card" | "row" | "dot" | "hero" | "switch";

/** @aiPurpose Apply only the visual treatment of a component. @aiUsage Spread last in its style; original themes return an empty override. */
export function skinStyle(theme: PluginTheme, role: SkinRole, active = false, fill?: string, edge?: string): ViewStyle {
  if (!isPixelTheme(theme)) return {};
  const { colors } = theme;
  const border = edge ?? (active ? colors.accent : colors.border);
  const style: ViewStyle = { borderRadius: role === "dot" ? pixelShape.dotCorner : pixelShape.corner };
  if (role === "hero") style.backgroundColor = pixelShape.hero;
  const outlined = ["button", "icon-button", "field", "card", "hero"].includes(role) || active;
  if (Platform.OS === "web") {
    // RN Web preserves custom properties. CSS draws the two stepped layers behind the content,
    // leaving the element's hit area, focus outline and child overlays rectangular and unclipped.
    Object.assign(style, { "--inspector-frame-edge": border, "--inspector-frame-fill": fill ?? (role === "hero" ? pixelShape.hero : colors.surface0) });
    if (outlined) style.boxShadow = "none";
  }
  return style;
}

/** Decorative motion hooks never carry state or execute effects; non-Web hosts receive no attributes. */
export function motionAttributes(role: string): object {
  return Platform.OS === "web" ? { dataSet: { inspectorMotion: role } } : {};
}

/** @aiPurpose Expose a stable semantic styling hook on Web only. @aiUsage Use with the corresponding skinStyle; native receives no DOM attributes. */
export function skinAttributes(role: SkinRole): object {
  return Platform.OS === "web" ? { dataSet: { inspectorUi: role } } : {};
}

/** @aiPurpose Keep metric cards white with a saturated category edge in the pixel skin. @aiUsage Apply after its existing gradient; calculations and size stay with the metric owner. */
export function metricSkin(theme: PluginTheme, color: string): ViewStyle {
  if (!isPixelTheme(theme)) return {};
  return { ...skinStyle(theme, "card", false, theme.colors.surface0, color), backgroundColor: theme.colors.surface0 };
}
