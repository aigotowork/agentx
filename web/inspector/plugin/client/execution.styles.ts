/** @aiPurpose Define semantic native layout roles for a compact execution management workspace using the shared kit tokens. */
import type { PluginTheme } from "../../host/contracts";
import { StyleSheet } from "react-native";
import { fontSize, hairline, radius, space } from "../../uiux/systems/inspector/inspector.native";

/** @aiPurpose Keep navigation, context and the investigated record together across device sizes. @aiUsage Memoize for the active theme and compact layout. */
export function executionStyles(theme: PluginTheme, compact: boolean) {
  return StyleSheet.create({
    screen: { flex: 1, minHeight: 0, backgroundColor: theme.colors.surface0 },
    heading: { flex: 1, minWidth: 0, fontSize: fontSize.title, color: theme.colors.foreground, fontWeight: "600" },
    controls: { paddingHorizontal: compact ? space.sm : space.lg, paddingVertical: space.sm, gap: space.sm, borderBottomWidth: hairline, borderBottomColor: theme.colors.border },
    context: { flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: space.xs },
    navigation: { flexDirection: "row", alignItems: "center", flexWrap: "wrap", gap: space.sm },
    sources: { flexDirection: "row", alignItems: "center", flexWrap: "wrap", gap: space.sm },
    source: { flexDirection: "row", alignItems: "center", flexWrap: "wrap", gap: space.xs },
    list: { flex: 1, minHeight: 0 },
    content: { paddingHorizontal: compact ? space.sm : space.lg, paddingBottom: space.lg },
    group: { flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: space.sm, paddingTop: space.md, paddingBottom: space.xs },
    groupTitle: { flexShrink: 1, fontSize: fontSize.small, fontWeight: "600", color: theme.colors.foregroundMuted },
    row: { paddingVertical: space.sm, gap: space.xs, borderBottomWidth: hairline, borderBottomColor: theme.colors.border },
    rowHeading: { flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: space.xs },
    title: { flexGrow: 1, flexShrink: 1, minWidth: 0, color: theme.colors.foreground, fontSize: fontSize.body, fontWeight: "500" },
    preview: { fontSize: fontSize.small, color: theme.colors.foreground },
    actions: { flexDirection: "row", alignItems: "center", flexWrap: "wrap", gap: space.sm },
    detail: { backgroundColor: theme.colors.surface1, borderRadius: radius.md, padding: space.sm, gap: space.xs },
    fact: { flexDirection: "row", flexWrap: "wrap", alignItems: "baseline", gap: space.xs },
    value: { color: theme.colors.foreground, fontSize: fontSize.small, flexShrink: 1 },
    muted: { color: theme.colors.foregroundMuted, fontSize: fontSize.meta, flexShrink: 1 },
    filterSection: { gap: space.sm },
    empty: { paddingVertical: space.lg, gap: space.sm },
  });
}
