/** @aiPurpose Compare historical model, harness and vendor observations through exact values and inspectable collection evidence. */
import { useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";
import type { PluginTheme } from "../../host/contracts";
import { Icon, ScrollView } from "../../host/ui";
import { currentLanguage } from "../shared/i18n";
import { t } from "../shared/text";
import type { StatsBasis, StatsBreakdownRow, StatsSnapshot } from "../shared/stats";
import { IconButton, Segmented, Sheet } from "../../uiux/systems/inspector/inspector.native";
import { StatsGate, StatsHeader, vendorLabel, type StatsViewProps } from "./stats-common";
import { comparisonDifference, comparisonRows, formatComparisonNumber, hasRecordedContent, metricGroups, readComparisonWithBasis, resolveComparisonPair, type CompareDimension, type ComparisonPair, type MetricId, type MetricReading } from "./stats-compare-model";
import { comparisonStyles } from "./stats-compare-styles";

export type { CompareDimension } from "./stats-compare-model";
type Styles = ReturnType<typeof comparisonStyles>;
interface MetricText { label: string; basis: string }

function metricText(): Record<MetricId, MetricText> {
  return {
    sessions: { label: t("metricComparison.sessions"), basis: t("metricComparison.sessionsBasis") },
    turns: { label: t("metricComparison.turns"), basis: t("metricComparison.turnsBasis") },
    activeDays: { label: t("metricComparison.activeDays"), basis: t("metricComparison.activeDaysBasis") },
    completed: { label: t("metricComparison.completed"), basis: t("metricComparison.completedBasis") },
    failed: { label: t("metricComparison.failed"), basis: t("metricComparison.failedBasis") },
    canceled: { label: t("metricComparison.canceled"), basis: t("metricComparison.canceledBasis") },
    completionRate: { label: t("metricComparison.completionRate"), basis: t("metricComparison.completionRateBasis") },
    runMs: { label: t("metricComparison.runMs"), basis: t("metricComparison.runMsBasis") },
    averageTurnMs: { label: t("metricComparison.averageTurnMs"), basis: t("metricComparison.averageTurnMsBasis") },
    approvals: { label: t("metricComparison.approvals"), basis: t("metricComparison.approvalsBasis") },
    waitMs: { label: t("metricComparison.waitMs"), basis: t("metricComparison.waitMsBasis") },
    linesAdded: { label: t("metricComparison.linesAdded"), basis: t("metricComparison.linesAddedBasis") },
    linesRemoved: { label: t("metricComparison.linesRemoved"), basis: t("metricComparison.linesRemovedBasis") },
    filesChanged: { label: t("metricComparison.filesChanged"), basis: t("metricComparison.filesChangedBasis") },
    toolCalls: { label: t("metricComparison.toolCalls"), basis: t("metricComparison.toolCallsBasis") },
    toolFailureRate: { label: t("metricComparison.toolFailureRate"), basis: t("metricComparison.toolFailureRateBasis") },
    userMessages: { label: t("metricComparison.userMessages"), basis: t("metricComparison.userMessagesBasis") },
    assistantMessages: { label: t("metricComparison.assistantMessages"), basis: t("metricComparison.assistantMessagesBasis") },
    errors: { label: t("metricComparison.errors"), basis: t("metricComparison.errorsBasis") },
  };
}
function groupText() {
  return { activity: t("metricComparison.activity"), outcomes: t("metricComparison.outcomes"), time: t("metricComparison.time"), content: t("metricComparison.content") };
}
function formattedValue(reading: MetricReading, delta = false): string {
  const { value, unit } = reading;
  if (value === null) return "—";
  const number = formatComparisonNumber(unit === "rate" ? value * 100 : value, currentLanguage(), delta);
  return unit === "rate" ? delta ? t("metricComparison.percentagePoints", { value: number }) : `${number}%`
    : unit === "milliseconds" ? t("metricComparison.milliseconds", { value: number }) : number;
}

function ObjectSelector({ theme, styles, row, side, onPress }: { theme: PluginTheme; styles: Styles; row: StatsBreakdownRow; side: "a" | "b"; onPress: () => void }) {
  const label = side === "a" ? t("metricComparison.selectCurrentA", { label: row.label }) : t("metricComparison.selectCurrentB", { label: row.label });
  return <Pressable testID={`comparison-side-${side}`} accessibilityRole="button" accessibilityLabel={label} onPress={onPress}
    style={state => [styles.side, side === "a" ? styles.sideA : styles.sideB, state.pressed && styles.pressed]}>
    <View style={styles.sideHeading}>
      <Text style={styles.sideTitle}>{side.toUpperCase()} · {vendorLabel(row.vendor)}</Text>
      <Icon name="ChevronsUpDown" size={16} color={theme.colors.foregroundMuted} />
    </View>
    <Text style={styles.sideName} numberOfLines={2}>{row.label}</Text>
    <Text style={styles.identity} numberOfLines={2}>{row.key}</Text>
    <Text style={styles.muted}>{t("metricComparison.objectSamples", { sessions: formatComparisonNumber(row.sessions, currentLanguage()), turns: formatComparisonNumber(row.turns, currentLanguage()) })}</Text>
  </Pressable>;
}

function ObjectPicker({ styles, open, side, rows, selected, onSelect, onClose }: { styles: Styles; open: boolean; side: "a" | "b" | null; rows: readonly StatsBreakdownRow[]; selected: string; onSelect: (key: string) => void; onClose: () => void }) {
  return <Sheet title={side === "a" ? t("metricComparison.selectA") : t("metricComparison.selectB")} open={open} onClose={onClose}>
    {rows.map(row => <Pressable key={row.key} testID={`comparison-option-${row.key}`} accessibilityRole="button" accessibilityLabel={`${row.label} · ${row.key}`}
      accessibilityState={{ selected: selected === row.key }} onPress={() => onSelect(row.key)} style={state => [styles.option, selected === row.key && styles.selectedOption, state.pressed && styles.pressed]}>
      <Text style={styles.body}>{row.label}</Text><Text style={styles.identity}>{row.key}</Text>
      <Text style={styles.muted}>{t("metricComparison.objectSamples", { sessions: formatComparisonNumber(row.sessions, currentLanguage()), turns: formatComparisonNumber(row.turns, currentLanguage()) })}</Text>
    </Pressable>)}
  </Sheet>;
}

function MetricRow({ theme, styles, id, text, a, b, basis, wide, onBasis }: { theme: PluginTheme; styles: Styles; id: MetricId; text: MetricText; a: StatsBreakdownRow; b: StatsBreakdownRow; basis: StatsBasis; wide: boolean; onBasis: () => void }) {
  const left = readComparisonWithBasis(a, id, basis), right = readComparisonWithBasis(b, id, basis);
  const difference = { ...left, value: comparisonDifference(left, right) };
  return <View testID={`comparison-row-${id}`} style={[styles.row, wide && styles.wideRow]}>
    <View style={[styles.metricLabel, wide && styles.wideLabel]}>
      <Text style={styles.metricText}>{text.label}</Text>
      <IconButton theme={theme} size="sm" icon="Info" label={t("metricComparison.metricBasis", { metric: text.label })} onPress={onBasis} />
    </View>
    <View style={[styles.values, wide && styles.wideValues]}>
      <Text testID={`comparison-value-${id}-a`} style={styles.value}>{formattedValue(left)}</Text>
      <Text testID={`comparison-value-${id}-b`} style={styles.value}>{formattedValue(right)}</Text>
      <Text testID={`comparison-value-${id}-delta`} style={styles.value}>{formattedValue(difference, true)}</Text>
    </View>
  </View>;
}

function ComparisonTable({ theme, styles, a, b, basis, wide, texts, onBasis }: { theme: PluginTheme; styles: Styles; a: StatsBreakdownRow; b: StatsBreakdownRow; basis: StatsBasis; wide: boolean; texts: Record<MetricId, MetricText>; onBasis: (id: MetricId) => void }) {
  const titles = groupText();
  return <View style={styles.table}>
    <View style={styles.columnHeader}>
      {wide ? <View style={styles.headerSpacer} /> : null}
      <View style={[styles.headerValues, wide && styles.wideHeaderValues]}>
        <Text style={styles.headerValue}>A</Text><Text style={styles.headerValue}>B</Text><Text style={styles.headerValue}>{t("metricComparison.difference")}</Text>
      </View>
    </View>
    {metricGroups.map(group => <View key={group.id}>
      <View style={styles.group}><Text style={styles.groupTitle}>{titles[group.id]}</Text></View>
      {group.metrics.map(id => <MetricRow key={id} theme={theme} styles={styles} id={id} text={texts[id]} a={a} b={b} basis={basis} wide={wide} onBasis={() => onBasis(id)} />)}
    </View>)}
  </View>;
}

function RowEvidence({ styles, row, side, metric, basis }: { styles: Styles; row: StatsBreakdownRow; side: "A" | "B"; metric?: MetricId; basis: StatsBasis }) {
  const reading = metric ? readComparisonWithBasis(row, metric, basis) : null;
  return <View style={styles.basisSide}>
    <Text style={styles.body}>{side} · {row.label}</Text><Text style={styles.identity}>{row.key}</Text>
    {reading ? <Text style={styles.body}>{formattedValue(reading)}{reading.denominator !== null ? ` (${reading.numerator === null ? "—" : formatComparisonNumber(reading.numerator, currentLanguage())} / ${formatComparisonNumber(reading.denominator, currentLanguage())})` : ""}</Text> : null}
    <Text style={styles.muted}>{t("metricComparison.outcomeSamples", { completed: row.completed, failed: row.failed, canceled: row.canceled, turns: row.turns })}</Text>
    <Text style={styles.muted}>{t("metricComparison.durationSamples", { samples: row.runSamples })}</Text>
    <Text style={styles.muted}>{t("statsBasis.contentSamples", { samples: row.contentSamples, complete: row.contentComplete, partial: row.contentPartial, failed: row.contentFailed })}</Text>
    <Text style={styles.muted}>{hasRecordedContent(row) ? t("metricComparison.contentRecorded") : t("metricComparison.contentUnknown")}</Text>
  </View>;
}

function CollectionBasis({ styles, stats, a, b, open, onClose }: { styles: Styles; stats: StatsSnapshot; a: StatsBreakdownRow; b: StatsBreakdownRow; open: boolean; onClose: () => void }) {
  return <Sheet title={t("metricComparison.collectionBasis")} open={open} onClose={onClose}>
    <View testID="comparison-basis" style={styles.basisBlock}>
      <Text style={styles.body}>{t("metricComparison.window", { from: stats.from, to: stats.to, zone: stats.timeZone ?? `UTC${stats.utcOffsetMinutes >= 0 ? "+" : ""}${stats.utcOffsetMinutes / 60}` })}</Text>
      <Text style={styles.muted}>{t("metricComparison.generatedAt", { at: stats.generatedAt })}</Text>
      <Text style={styles.muted}>{t("metricComparison.observedSince", { at: stats.observedSince })}</Text>
      <Text style={styles.muted}>{t("metricComparison.contentSince", { at: stats.contentSince })}</Text>
      <Text style={styles.body}>{t("statsBasis.sampleBasis")}</Text>
      <Text style={styles.body}>{t("metricComparison.directorySeparate")}</Text>
      {stats.directoryTruncated ? <Text style={styles.body}>{t("metricComparison.directoryTruncated")}</Text> : null}
      <Text style={styles.body}>{t("metricComparison.collectionLimits")}</Text>
      <Text style={styles.body}>{t("metricComparison.precision")}</Text>
      <Text style={styles.body}>{t("statsBasis.notLifetime")}</Text>
      <Text style={styles.body}>{t("statsBasis.bestEffort")}</Text>
      <Text style={styles.body}>{t("statsBasis.timestampGrain")}</Text>
      {stats.basis.attribution.status !== "complete" ? <Text style={styles.body}>{t("statsBasis.attributionPartial")}</Text> : null}
      {stats.basis.attribution.capped || stats.basis.toolNames.capped ? <Text style={styles.body}>{t("statsBasis.caps")}</Text> : null}
      <RowEvidence styles={styles} row={a} side="A" basis={stats.basis} /><RowEvidence styles={styles} row={b} side="B" basis={stats.basis} />
    </View>
  </Sheet>;
}

function Comparison({ theme, styles, stats, dimension, wide, preferred, onPairChange }: { theme: PluginTheme; styles: Styles; stats: StatsSnapshot; dimension: CompareDimension; wide: boolean; preferred?: ComparisonPair; onPairChange: (pair: ComparisonPair) => void }) {
  const rows = useMemo(() => comparisonRows(stats, dimension), [stats, dimension]);
  const [picking, setPicking] = useState<"a" | "b" | null>(null);
  const [metric, setMetric] = useState<MetricId | null>(null);
  const [collectionOpen, setCollectionOpen] = useState(false);
  const pair = resolveComparisonPair(rows, preferred);
  const texts = metricText();
  if (!pair) return <View style={styles.empty}><Text style={styles.title}>{t("metricComparison.exactMetrics")}</Text><Text style={styles.body}>{t("metricComparison.needTwo")}</Text></View>;
  const a = rows.find(row => row.key === pair[0])!, b = rows.find(row => row.key === pair[1])!;
  const choose = (key: string) => { onPairChange(picking === "a" ? [key, pair[1]] : [pair[0], key]); setPicking(null); };
  const changed = preferred && (preferred[0] !== pair[0] || preferred[1] !== pair[1]);
  return <>
    <View style={styles.pair}>
      <ObjectSelector theme={theme} styles={styles} row={a} side="a" onPress={() => setPicking("a")} />
      <ObjectSelector theme={theme} styles={styles} row={b} side="b" onPress={() => setPicking("b")} />
    </View>
    <View style={styles.toolbar}>
      <Text style={styles.title}>{t("metricComparison.exactMetrics")}</Text>
      <IconButton theme={theme} icon="ArrowLeftRight" label={t("metricComparison.swap")} onPress={() => onPairChange([pair[1], pair[0]])} />
      <IconButton theme={theme} icon="Info" label={t("metricComparison.collectionBasis")} onPress={() => setCollectionOpen(true)} />
    </View>
    <Text style={styles.muted}>{t("metricComparison.unpaired")}</Text>
    {stats.basis.attribution.status !== "complete" ? <Text style={styles.muted}>{t("statsBasis.attributionPartial")}</Text> : null}
    {changed ? <Text style={styles.muted}>{t("metricComparison.selectionChanged")}</Text> : null}
    <ComparisonTable theme={theme} styles={styles} a={a} b={b} basis={stats.basis} wide={wide} texts={texts} onBasis={setMetric} />
    <ObjectPicker styles={styles} open={picking !== null} side={picking} rows={rows.filter(row => row.key !== pair[picking === "a" ? 1 : 0])} selected={pair[picking === "a" ? 0 : 1]} onSelect={choose} onClose={() => setPicking(null)} />
    <CollectionBasis styles={styles} stats={stats} a={a} b={b} open={collectionOpen} onClose={() => setCollectionOpen(false)} />
    <Sheet title={t("metricComparison.metricBasis", { metric: metric ? texts[metric].label : "" })} open={metric !== null} onClose={() => setMetric(null)}>
      {metric ? <View style={styles.basisBlock}>
        <Text style={styles.body}>{texts[metric].basis}</Text>
        <RowEvidence styles={styles} row={a} side="A" metric={metric} basis={stats.basis} /><RowEvidence styles={styles} row={b} side="B" metric={metric} basis={stats.basis} />
      </View> : null}
    </Sheet>
  </>;
}

/** @aiPurpose Present descriptive A/B statistics in the existing host-wide range. @aiUsage Pass the unchanged StatsViewProps; range/refresh remain caller-owned. @aiBehavior Selection is owned per dimension and preserved when available across snapshot refreshes; reads never trigger business commands. */
/** @aiPurpose Compose StatsCompare presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function StatsCompare({ theme, compact, stats, loading, error, range, onRange, onRefresh }: StatsViewProps) {
  const [dimension, setDimension] = useState<CompareDimension>("models");
  // Range queries can temporarily unmount StatsGate's content; selection belongs to the surviving page.
  const [pairs, setPairs] = useState<Partial<Record<CompareDimension, ComparisonPair>>>({});
  const [width, setWidth] = useState(0);
  const styles = useMemo(() => comparisonStyles(theme, compact), [theme, compact]);
  const changePair = (pair: ComparisonPair) => setPairs(previous => ({ ...previous, [dimension]: pair }));
  const dimensions = <Segmented theme={theme} size="sm" value={dimension} onChange={setDimension} accessibilityLabel={t("metricComparison.dimension")}
    options={[{ value: "models", label: t("metricComparison.models") }, { value: "providers", label: t("metricComparison.providers") }, { value: "vendors", label: t("metricComparison.vendors") }]} />;
  return <ScrollView testID="metric-comparison" style={styles.viewport} contentContainerStyle={styles.scroll}>
    <View style={styles.page} onLayout={event => { const next = event.nativeEvent.layout.width; if (next > 0) setWidth(next); }}>
      <StatsHeader theme={theme} compact={compact} stats={stats} range={range} onRange={onRange} extra={dimensions} />
      <StatsGate theme={theme} stats={stats} loading={loading} error={error} onRefresh={onRefresh}>
        {snapshot => <Comparison theme={theme} styles={styles} stats={snapshot} dimension={dimension} wide={width >= 760} preferred={pairs[dimension]} onPairChange={changePair} />}
      </StatsGate>
    </View>
  </ScrollView>;
}
