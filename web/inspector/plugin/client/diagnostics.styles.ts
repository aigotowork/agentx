/** @aiPurpose Keep compact diagnostic layout roles on the established native kit tokens. */
import type { PluginTheme } from "../../host/contracts";
import { StyleSheet } from "react-native";
import { fontSize, hairline, space } from "../../uiux/systems/inspector/inspector.native";

/** @aiPurpose Provide semantic styles for the diagnostic workspace. @aiUsage Memoize for the current theme and compact layout. */
export function diagnosticsStyles(theme: PluginTheme, compact: boolean) {
  return StyleSheet.create({
    screen: { flex: 1, backgroundColor: theme.colors.surface0 },
    content: { padding: compact ? space.sm : space.lg, gap: space.md },
    heading: { flex: 1, minWidth: 0, color: theme.colors.foreground, fontSize: fontSize.title, fontWeight: "600" },
    context: { gap: space.xs },
    section: { gap: space.sm, paddingBottom: space.sm, borderBottomWidth: hairline, borderBottomColor: theme.colors.border },
    sectionHeading: { flexDirection: "row", alignItems: "center", gap: space.sm },
    sectionTitle: { color: theme.colors.foreground, fontSize: fontSize.body, fontWeight: "600", flex: 1, minWidth: 0 },
    fact: { flexDirection: "row", flexWrap: "wrap", alignItems: "baseline", gap: space.xs },
    label: { color: theme.colors.foregroundMuted, fontSize: fontSize.meta },
    value: { color: theme.colors.foreground, fontSize: fontSize.small, flexShrink: 1 },
    muted: { color: theme.colors.foregroundMuted, fontSize: fontSize.meta },
    detail: { gap: space.xs },
    history: { gap: space.sm },
  });
}
