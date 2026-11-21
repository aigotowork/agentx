/** @aiPurpose Map AgentX semantic colors into one compact, responsive recovery management surface. */
import { skinStyle } from "../../uiux/systems/inspector/inspector.theme";
import type { PluginTheme } from "../../host/contracts";
import { space, fontSize, radius, control as kitControl, hairline } from "../../uiux/systems/inspector/inspector.native";
import { StyleSheet } from "react-native";

// AgentX 0.15.3 supplies colors; local tokens supply density, sizing and typography.
const tokens = { gap: space.xs, sectionGap: space.sm, rowPadding: space.sm, pagePadding: space.md, compactPadding: space.sm,
  radius: radius.sm, control: kitControl.md, touchControl: kitControl.md, titleSize: fontSize.heading, bodySize: fontSize.small, metaSize: fontSize.meta, lineHeight: 18, border: hairline,
  filterHeight: 200, projectTop: space.md, workspaceIndent: space.lg };
/** @aiPurpose Centralize recovery screen styles. @aiUsage Memoize per host theme and compact layout; assembly consumes named roles. */
export function recoveryStyles(theme: PluginTheme, compact: boolean) {
  const c = theme.colors, t = tokens, control = compact ? t.touchControl : t.control;
  return { theme, ...StyleSheet.create({
    screen: { flex: 1, backgroundColor: c.surface0 },
    toolbar: { padding: compact ? t.compactPadding : t.pagePadding, gap: t.sectionGap, borderBottomWidth: t.border, borderBottomColor: c.border },
    header: { flexDirection: "row", alignItems: "center", justifyContent: "space-between", gap: t.gap, flexWrap: "wrap" },
    line: { flexDirection: "row", alignItems: "center", gap: t.gap },
    title: { color: c.foreground, fontSize: t.titleSize, fontWeight: "600" },
    text: { color: c.foreground, fontSize: t.bodySize, lineHeight: t.lineHeight },
    muted: { color: c.foregroundMuted, fontSize: t.metaSize, lineHeight: t.lineHeight },
    danger: { color: c.statusDanger, fontSize: t.metaSize, lineHeight: t.lineHeight },
    warning: { color: c.statusWarning, fontSize: t.metaSize, lineHeight: t.lineHeight },
    success: { color: c.statusSuccess, fontSize: t.metaSize, lineHeight: t.lineHeight },
    accent: { color: c.accent, fontSize: t.metaSize, lineHeight: t.lineHeight },
    actions: { flexDirection: "row", alignItems: "center", gap: t.gap, flexWrap: "wrap" },
    grow: { flex: 1, minWidth: 0 }, stack: { gap: t.gap },
    button: { minWidth: control, minHeight: control, paddingHorizontal: t.gap, borderRadius: t.radius,
      justifyContent: "center", alignItems: "center", flexDirection: "row", gap: t.gap,
      borderWidth: t.border, borderColor: c.border, backgroundColor: c.surface0, ...skinStyle(theme, "icon-button") },
    primary: { backgroundColor: c.accent, borderColor: c.accent },
    primaryText: { color: c.accentForeground, fontSize: t.bodySize, fontWeight: "600" },
    selected: { borderColor: c.accent, backgroundColor: c.surface1 },
    disabled: { opacity: 0.4 }, pressed: { opacity: 0.65 },
    search: { flex: 1, minWidth: 0, color: c.foreground, fontSize: t.bodySize, minHeight: control,
      paddingHorizontal: t.rowPadding, borderWidth: t.border, borderColor: c.border, borderRadius: t.radius, backgroundColor: c.surface1, ...skinStyle(theme, "field") },
    list: { paddingHorizontal: compact ? t.compactPadding : t.pagePadding, paddingBottom: t.pagePadding },
    project: { paddingTop: t.projectTop, paddingBottom: t.gap, gap: t.gap, backgroundColor: c.surface0 },
    workspace: { paddingVertical: t.gap, paddingLeft: t.workspaceIndent, gap: t.gap, backgroundColor: c.surface0 },
    row: { paddingVertical: t.rowPadding, gap: t.gap, borderBottomWidth: t.border, borderBottomColor: c.border },
    rowTop: { flexDirection: "row", gap: t.gap, alignItems: "center" },
    rowTitle: { color: c.foreground, fontSize: t.bodySize, fontWeight: "600", lineHeight: t.lineHeight, flexShrink: 1 },
    details: { padding: t.rowPadding, gap: t.sectionGap, borderRadius: t.radius, backgroundColor: c.surface1, ...skinStyle(theme, "card") },
    detailLine: { flexDirection: "row", alignItems: "flex-start", gap: t.gap },
    empty: { paddingVertical: t.pagePadding, gap: t.gap, alignItems: "center" },
    footer: { paddingHorizontal: compact ? t.compactPadding : t.pagePadding, paddingVertical: t.gap,
      gap: t.gap, borderTopWidth: t.border, borderTopColor: c.border },
    modal: { gap: t.sectionGap }, filterList: { maxHeight: t.filterHeight },
    filterRow: { paddingVertical: t.gap, flexDirection: "row", alignItems: "center", gap: t.gap },
  }) };
}
export type RecoveryStyles = ReturnType<typeof recoveryStyles>;
