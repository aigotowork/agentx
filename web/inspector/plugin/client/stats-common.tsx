/** @aiPurpose Share Stats page structure and disclose source, retention and admission limits near the query. */
import { skinAttributes, skinStyle } from "../../uiux/systems/inspector/inspector.theme";
import { solidForeground } from "../../uiux/systems/inspector/inspector.native";
import { useTooltip } from "../../uiux/systems/inspector/inspector.native";
import type { DirectoryHealth } from "../shared/source-health";
import { currentLanguage } from "../shared/i18n";
import { t } from "../shared/text";
import type { ReactNode } from "react";
import { Pressable, Text, View } from "react-native";
import type { PluginTheme } from "../../host/contracts";
import { Icon } from "../../host/ui";
import { STATS_RANGES, vendorNames, type StatsBreakdownRow, type StatsRange, type StatsSnapshot } from "../shared/stats";
import { InfoButton, Banner, EmptyState, Segmented, Skeleton, Spacer, fontSize, gradient, hue, hueFor, isDarkTheme, radius, relativeTime, space, withAlpha, type Hue } from "../../uiux/systems/inspector/inspector.native";

import { localTimeZone, localUtcOffset } from "./use-stats";

/** @aiPurpose Provide rangeOptions presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const rangeOptions = STATS_RANGES.map(value => ({ value, get label() { return value === "7d" ? t("secondaryStatsCommon.days") : value === "30d" ? t("secondaryStatsCommon.days2") : t("secondaryStatsCommon.days3"); } }));
export interface StatsViewProps {
  theme: PluginTheme; compact: boolean; stats?: StatsSnapshot; loading: boolean; error?: string;
  range: StatsRange; onRange: (range: StatsRange) => void; onRefresh: () => void;
}

/** Vendor brand-ish colours; a vendor's second and third models take neighbouring hues. */
const vendorHues: Record<string, readonly Hue[]> = {
  anthropic: ["orange", "amber", "red"], openai: ["green", "teal", "lime"], google: ["blue", "sky", "cyan"], xai: ["violet", "fuchsia", "indigo"],
  moonshot: ["indigo", "violet", "blue"], deepseek: ["sky", "blue", "cyan"], bytedance: ["cyan", "teal", "sky"], alibaba: ["amber", "orange", "lime"],
  zhipu: ["teal", "green", "cyan"], other: ["pink", "fuchsia", "gray"],
};
/** @aiPurpose Provide vendorHue presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const vendorHue = (vendor: string | null | undefined): Hue => (vendorHues[vendor ?? "other"] ?? vendorHues.other!)[0]!;
/** Colour per breakdown row: vendor families for models / harnesses / vendors, the shared project hue for projects. */
/** @aiPurpose Provide rowHues presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function rowHues(rows: readonly StatsBreakdownRow[], kind: "model" | "provider" | "vendor" | "project"): Map<string, Hue> {
  const used = new Map<string, number>(), result = new Map<string, Hue>();
  for (const row of rows) {
    if (row.key === "__other__" || row.key === "unknown") { result.set(row.key, "gray"); continue; }
    if (kind === "project") { result.set(row.key, hueFor(row.label)); continue; }
    const family = vendorHues[row.vendor ?? "other"] ?? vendorHues.other!, index = used.get(row.vendor ?? "other") ?? 0;
    used.set(row.vendor ?? "other", index + 1);
    result.set(row.key, family[index % family.length]!);
  }
  return result;
}
/** @aiPurpose Provide vendorLabel presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const vendorLabel = (vendor: string | null | undefined) => vendorNames[vendor ?? "other"] ?? vendor ?? t("secondaryStatsCommon.other");

/** A small filled disk with a white icon — the dashboards' marker for a metric or record. */
/** @aiPurpose Compose IconDisk presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function IconDisk({ theme, icon, colors, size = 28 }: { theme: PluginTheme; icon: string; colors: readonly [Hue, Hue] | readonly [Hue]; size?: number }) {
  return <View style={[{ width: size, height: size, borderRadius: size / 2.6, alignItems: "center", justifyContent: "center" }, gradient(theme, colors.length > 1 ? colors : [colors[0], colors[0]], { angle: 135 }), skinStyle(theme, "badge")]}>
    <Icon name={icon} size={Math.round(size * 0.55)} color={solidForeground(theme, hue(theme, colors[0]))} />
  </View>;
}

/** Scope and directory freshness are separate from the time at which an aggregate was generated. */
function DirectoryStatus({ theme, health, logs }: { theme: PluginTheme; health?: DirectoryHealth; logs: boolean }) {
  const status = health?.status === "healthy" && !health.complete ? "unknown" : health?.status ?? "unknown";
  const label = logs ? {
    healthy: t("workbench.attributionHealthy"), stale: t("workbench.attributionStale"), unavailable: t("workbench.attributionUnavailable"), unknown: t("workbench.attributionUnknown"),
  }[status] : {
    healthy: t("workbench.directoryHealthy"), stale: t("workbench.directoryStale"), unavailable: t("workbench.directoryUnavailable"), unknown: t("workbench.directoryUnknown"),
  }[status];
  const tone: Hue = status === "healthy" ? "green" : status === "stale" ? "amber" : status === "unavailable" ? "red" : "gray";
  return <View style={{ flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: space.xs }}>
    <Icon name={status === "healthy" ? "CircleCheck" : status === "unavailable" ? "WifiOff" : status === "stale" ? "History" : "CircleHelp"} size={fontSize.meta} color={hue(theme, tone, "text")} />
    <Text style={{ color: hue(theme, tone, "text"), fontSize: fontSize.meta }}>{label}</Text>
    {status !== "healthy" && health?.lastSuccessAt ? <Text style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta }}>{t("workbench.lastSuccess", { time: relativeTime(health.lastSuccessAt) })}</Text> : null}
  </View>;
}

/** Range, actual host scope, aggregate generation time and source health for all statistics views. */
/** @aiPurpose Compose StatsHeader presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function StatsHeader({ theme, compact, stats, range, onRange, extra, generatedAt, info, infoTitle = t("secondaryStatsCommon.aboutStatistics"), directoryHealth, scope = "stats" }: {
  theme: PluginTheme; compact: boolean; stats?: StatsSnapshot; range: StatsRange; onRange: (range: StatsRange) => void; extra?: ReactNode;
  generatedAt?: string; info?: readonly string[]; infoTitle?: string; directoryHealth?: DirectoryHealth; scope?: "stats" | "logs";
}) {
  const updated = generatedAt ?? stats?.generatedAt, health = directoryHealth ?? stats?.directoryHealth;
  const errorLines = health?.errorCode ? {
    read_failed: t("workbench.healthReadFailed"), timeout: t("workbench.healthTimeout"), invalidated: t("workbench.healthInvalidated"), incomplete: t("workbench.healthIncomplete"),
  }[health.errorCode] : undefined;
  const details = [scope === "logs" ? t("workbench.logScopeInfo") : t("workbench.scopeInfo"),
    t("workbench.localTime", { zone: localTimeZone() ?? t("workbench.localOffset", { minutes: localUtcOffset() }) }),
    ...(!health || health.status === "unknown" ? [t("workbench.healthUnknownInfo")] : []),
    ...(health?.lastSuccessAt ? [t("workbench.lastSuccess", { time: new Date(health.lastSuccessAt).toLocaleString(currentLanguage()) })] : []),
    ...(health?.lastAttemptAt ? [t("workbench.lastAttempt", { time: new Date(health.lastAttemptAt).toLocaleString(currentLanguage()) })] : []),
    ...(errorLines ? [errorLines] : []),
    ...(info ?? [
      t("secondaryStatsCommon.sessionCountsIncludeAllPaseoSessions"),
      stats?.observedSince ? t("secondaryStatsCommon.thePluginHasTrackedTurnsDuration", { value1: new Date(stats.observedSince).toLocaleDateString(currentLanguage()) }) : t("secondaryStatsCommon.turnsDurationAndConcurrencyAccumulateFrom"),
      stats?.contentSince ? t("secondaryStatsCommon.messagesToolCallsAndLinesOf", { value1: new Date(stats.contentSince).toLocaleDateString(currentLanguage()) }) : t("secondaryStatsCommon.messageToolCallAndCodeLine"),
      t("secondaryStatsCommon.codeLineCountsAreEstimatedFrom"),
      t("secondaryStatsCommon.successRateCompletedCompletedFailedTurns"),
      t("secondaryStatsCommon.periodComparisonsUseTheImmediatelyPreceding"),
    ]), ...(scope === "stats" ? [
      t("statsBasis.notLifetime"), ...(stats ? [t("statsBasis.cumulativeSince", { at: stats.cumulative.since })] : []), t("statsBasis.bestEffort"), t("statsBasis.timestampGrain"), t("statsBasis.contentPartial"),
      ...(stats?.basis.lifecycle.retainedFrom ? [t("statsBasis.basisWindow", { at: stats.basis.lifecycle.retainedFrom })] : []),
      ...(stats?.basis.attribution.status !== "complete" ? [t("statsBasis.attributionPartial")] : []),
      ...(stats?.basis.attribution.capped || stats?.basis.toolNames.capped ? [t("statsBasis.caps")] : []),
    ] : [])];
  const basisLabel = stats?.basis.admission === "unavailable" ? t("statsBasis.storageUnavailable")
    : stats?.basis.admission === "limited" ? t("statsBasis.admissionLimited") : t("statsBasis.retained");
  return <View style={{ gap: space.sm }}>
    <View style={{ flexDirection: "row", alignItems: "center", gap: space.sm, flexWrap: "wrap" }}>
      <Segmented theme={theme} size="sm" value={range} onChange={onRange} accessibilityLabel={t("secondaryStatsCommon.statisticsRange")} options={rangeOptions} />
      {extra}<Spacer />
      {updated && !compact ? <Text style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta }}>{t("secondaryStatsCommon.updated", { value1: relativeTime(updated) })}</Text> : null}
      <InfoButton theme={theme} title={infoTitle} lines={details} />
    </View>
    <View style={{ flexDirection: "row", flexWrap: "wrap", alignItems: "center", columnGap: space.md, rowGap: space.xs }}>
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta }}>{t(scope === "logs" ? "workbench.logScope" : "workbench.hostScope")}</Text>
      <DirectoryStatus theme={theme} health={health} logs={scope === "logs"} />
      {scope === "stats" && stats ? <Text style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta }}>{basisLabel}</Text> : null}
      {scope === "stats" && stats && stats.basis.query.previous !== "complete" ? <Text style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta }}>{t("statsBasis.previousUnavailable")}</Text> : null}
    </View>
  </View>;
}

/** Loading / error / empty wrapper for statistics pages. */
/** @aiPurpose Compose StatsGate presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function StatsGate({ theme, stats, loading, error, onRefresh, children }: { theme: PluginTheme; stats?: StatsSnapshot; loading: boolean; error?: string; onRefresh: () => void; children: (stats: StatsSnapshot) => ReactNode }) {
  if (!stats) return loading ? <Skeleton theme={theme} rows={6} />
    : <EmptyState theme={theme} icon={error ? "WifiOff" : "ChartNoAxesCombined"} tone={error ? "danger" : "violet"} title={error ? t("secondaryStatsCommon.statisticsAreTemporarilyUnavailable") : t("secondaryStatsCommon.noStatisticsYet")} message={error ?? t("secondaryStatsCommon.sendOutAnAgentToStart")} action={{ label: t("secondaryStatsCommon.retry"), onPress: onRefresh, icon: "RefreshCw" }} />;
  return <>{error ? <Banner theme={theme} tone="warning" message={t("secondaryStatsCommon.statisticsSyncFailedShowingThePrevious", { value1: error })} action={{ label: t("secondaryStatsCommon.retry"), onPress: onRefresh }} /> : null}{children(stats)}</>;
}

/** Frosted pill used on gradient heroes. */
/** @aiPurpose Compose GlassPill presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function GlassPill({ theme, label, value, icon, onPress, mark }: { theme?: PluginTheme; label: string; value?: string | number; icon?: string; onPress?: () => void; mark?: ReactNode }) {
  const name = value === undefined ? label : `${label} ${value}`;
  const hint = useTooltip({ label: onPress ? name : false, onPress });
  const body = <>
    {mark ?? (icon ? <Icon name={icon} size={13} color="#ffffff" /> : null)}
    <Text style={{ color: "#ffffff", fontSize: fontSize.small, fontWeight: "600" }}>{label}</Text>
    {value !== undefined ? <Text style={{ color: "#ffffff", fontSize: fontSize.small, fontWeight: "800", fontVariant: ["tabular-nums"] }}>{value}</Text> : null}
  </>;
  const style = { flexDirection: "row" as const, alignItems: "center" as const, gap: 5, minHeight: 30, paddingHorizontal: 11, borderRadius: radius.full, backgroundColor: withAlpha("#ffffff", 0.18), borderWidth: 1, borderColor: withAlpha("#ffffff", 0.28), ...(theme ? skinStyle(theme, "chip") : {}) };
  return onPress ? <Pressable {...skinAttributes("chip")} {...hint} accessibilityRole="button" accessibilityLabel={name} hitSlop={{ top: 6, bottom: 6 }} style={state => [style, { opacity: state.pressed ? 0.75 : 1 }]}>{body}</Pressable> : <View style={style}>{body}</View>;
}

/** @aiPurpose Provide surfaceFor presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const surfaceFor = (theme: PluginTheme) => isDarkTheme(theme) ? theme.colors.surface1 : theme.colors.surface0;
/** @aiPurpose Provide trackFor presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const trackFor = (theme: PluginTheme) => isDarkTheme(theme) ? withAlpha("#ffffff", 0.08) : withAlpha("#000000", 0.06);
/** @aiPurpose Provide hueText presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const hueText = (theme: PluginTheme, name: Hue) => hue(theme, name, "text");
