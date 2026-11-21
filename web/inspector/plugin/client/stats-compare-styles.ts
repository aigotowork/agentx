/** @aiPurpose Own native comparison layout and named treatments using the existing kit's theme and sizing tokens. */
import type { PluginTheme } from "../../host/contracts";
import { StyleSheet } from "react-native";
import { control, fontSize, hue, radius, space } from "../../uiux/systems/inspector/inspector.native";

/** @aiPurpose Provide cross-platform styles for the metric comparison surface. @aiUsage Memoize by theme and compact layout; no DOM or CSS dependencies. */
export function comparisonStyles(theme: PluginTheme, compact: boolean) {
  const { colors } = theme;
  return StyleSheet.create({
    viewport: { flex: 1, minHeight: 0 },
    scroll: { paddingHorizontal: compact ? space.md : space.xl, paddingTop: space.md, paddingBottom: space.xxl },
    page: { width: "100%", maxWidth: 1080, alignSelf: "center", gap: space.lg },
    pair: { flexDirection: "row", gap: space.sm, alignItems: "stretch" },
    side: { flex: 1, minWidth: 0, borderWidth: StyleSheet.hairlineWidth, borderColor: colors.border, borderRadius: radius.lg, padding: space.md, gap: space.xs, backgroundColor: colors.surface1 },
    sideHeading: { flexDirection: "row", alignItems: "center", gap: space.xs },
    sideTitle: { flex: 1, color: colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: "600" },
    sideName: { color: colors.foreground, fontSize: fontSize.body, fontWeight: "700" },
    identity: { color: colors.foregroundMuted, fontSize: fontSize.meta, flexShrink: 1 },
    sideA: { borderTopColor: hue(theme, "violet"), borderTopWidth: space.xs },
    sideB: { borderTopColor: hue(theme, "cyan"), borderTopWidth: space.xs },
    toolbar: { flexDirection: "row", alignItems: "center", flexWrap: "wrap", gap: space.sm },
    title: { color: colors.foreground, fontSize: fontSize.title, fontWeight: "700", flexGrow: 1 },
    muted: { color: colors.foregroundMuted, fontSize: fontSize.meta },
    body: { color: colors.foreground, fontSize: fontSize.body },
    basisBlock: { gap: space.sm, minWidth: 0 },
    basisSide: { gap: space.xs, padding: space.md, backgroundColor: colors.surface1, borderRadius: radius.md },
    table: { borderRadius: radius.lg, borderWidth: StyleSheet.hairlineWidth, borderColor: colors.border, backgroundColor: colors.surface1, overflow: "hidden" },
    columnHeader: { flexDirection: "row", alignItems: "center", paddingHorizontal: space.md, paddingVertical: space.sm, backgroundColor: colors.surface2 },
    group: { paddingHorizontal: space.md, paddingVertical: space.sm, backgroundColor: colors.surface2 },
    groupTitle: { color: colors.foreground, fontSize: fontSize.small, fontWeight: "700" },
    row: { paddingHorizontal: space.md, paddingVertical: space.sm, gap: space.xs, borderTopWidth: StyleSheet.hairlineWidth, borderTopColor: colors.border },
    wideRow: { flexDirection: "row", alignItems: "center" },
    metricLabel: { flexDirection: "row", alignItems: "center", minWidth: 0, gap: space.xs, minHeight: control.sm },
    wideLabel: { flex: 1 },
    metricText: { color: colors.foreground, fontSize: fontSize.small, flexShrink: 1 },
    values: { flexDirection: "row", gap: space.xs },
    wideValues: { flex: 2 },
    value: { flex: 1, minWidth: 0, textAlign: "right", color: colors.foreground, fontSize: fontSize.small, fontWeight: "600", fontVariant: ["tabular-nums"] },
    headerValue: { flex: 1, textAlign: "right", color: colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: "600" },
    headerSpacer: { flex: 1 },
    headerValues: { flex: 1, flexDirection: "row", gap: space.xs },
    wideHeaderValues: { flex: 2 },
    option: { minHeight: control.md + space.md, paddingVertical: space.sm, paddingHorizontal: space.sm, borderRadius: radius.md, gap: space.xs },
    selectedOption: { backgroundColor: colors.surface2 },
    pressed: { backgroundColor: colors.surface2 },
    empty: { padding: space.lg, gap: space.sm, borderRadius: radius.lg, backgroundColor: colors.surface1 },
  });
}
