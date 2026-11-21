/** Canonical native presentation values. Host semantic colors remain in PluginTheme. */
import { Platform, StyleSheet } from "react-native";

import { space, fontSize, radius, control } from "./foundation.tokens";
export { space, fontSize, radius, control } from "./foundation.tokens";
export const monoFont = Platform.OS === "ios" ? "Menlo" : "monospace";
export const hairline = Platform.OS === "web" ? 1 : StyleSheet.hairlineWidth;
/** Extend compact controls' touch area without changing their layout. */
export const touchSlop = { top: 8, bottom: 8, left: 6, right: 6 } as const;
export const tooltipTokens = {
  margin: space.sm, gap: space.xs, maxWidth: 280, minAvailableWidth: space.xxl,
  paddingHorizontal: space.sm, paddingVertical: space.xs, radius: radius.sm,
  fontSize: fontSize.meta, lineHeight: 18, layer: 1, longPressMs: 500,
} as const;
