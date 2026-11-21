import { StyleSheet } from 'react-native';
import type { PluginTheme } from '../../../host/contracts';
import { space, fontSize, radius, hairline, control, monoFont } from '../../../uiux/systems/inspector/inspector.native';
export const botsStyles = (theme: PluginTheme, compact: boolean) => StyleSheet.create({
  root: { flex: 1, minHeight: 0, backgroundColor: theme.colors.surface0 },
  fill: { flex: 1, minWidth: 0 },
  heading: { color: theme.colors.foreground, fontSize: fontSize.title, fontWeight: '600' },
  text: { color: theme.colors.foreground, fontSize: fontSize.body },
  muted: { color: theme.colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: fontSize.heading },
  mono: { color: theme.colors.foregroundMuted, fontSize: fontSize.meta, fontFamily: monoFont },
  tabRegion: { flexGrow: 0, flexShrink: 0 },
  tabs: { paddingHorizontal: space.md },
  content: { padding: compact ? space.sm : space.lg, gap: space.lg },
  summary: { padding: space.md, borderWidth: hairline, borderColor: theme.colors.border, borderRadius: radius.md, backgroundColor: theme.colors.surface1, gap: space.md },
  grid: { flexDirection: compact ? 'column' : 'row', alignItems: 'stretch', gap: space.lg },
  primary: { flex: compact ? undefined : 2, minWidth: 0, gap: space.md },
  secondary: { flex: compact ? undefined : 1, minWidth: 0, gap: space.md },
  section: { gap: space.sm, minWidth: 0 },
  panel: { borderWidth: hairline, borderColor: theme.colors.border, borderRadius: radius.md, overflow: 'hidden' },
  team: { padding: space.md, gap: space.sm, borderBottomWidth: hairline, borderColor: theme.colors.border },
  line: { flexDirection: 'row', gap: space.sm, alignItems: 'center', minWidth: 0 },
  wrap: { flexDirection: 'row', flexWrap: 'wrap', gap: space.sm, alignItems: 'center' },
  member: { flexDirection: 'row', alignItems: 'center', gap: space.sm, paddingVertical: space.xs, minHeight: control.md },
  memberIdentity: { flex: 1, minWidth: 0, alignItems: 'flex-start' },
  memberName: { flex: 1, minWidth: 0, color: theme.colors.foreground, fontSize: fontSize.small },
  track: { height: space.xs, width: space.xxl * 2, backgroundColor: theme.colors.surface2, borderRadius: radius.sm, overflow: 'hidden' },
  distribution: { flexDirection: 'row', height: space.sm, overflow: 'hidden', borderRadius: radius.sm, backgroundColor: theme.colors.surface2 },
  segment: { height: '100%' },
  count: { color: theme.colors.foreground, fontSize: fontSize.small, fontVariant: ['tabular-nums'] },
  detail: { gap: space.md, padding: space.md },
});
export type BotsStyles = ReturnType<typeof botsStyles>;
