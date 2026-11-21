/** @aiPurpose Present a compact analysis basis with locally disclosed source, usage and review facts; never infer missing Token counters or verified original evidence. */
import { useState } from "react";
import { Pressable, Text, View } from "react-native";
import type { PluginTheme } from "../../host/contracts";
import { Icon } from "../../host/ui";
import { t } from "../shared/text";
import type { InsightJob } from "../shared/insights";
import { Badge, fontSize, hairline, radius, space } from "../../uiux/systems/inspector/inspector.native";
import { analysisFacts, type EvidenceAvailability } from "./analysis-evidence-model";

const exactNumber = (value: number | null) => value === null ? t("analysisEvidence.unknown") : String(value);
const timestamp = (value: string | null | undefined) => value && Number.isFinite(Date.parse(value)) ? new Date(value).toISOString() : t("analysisEvidence.unknown");

/** @aiPurpose Name only the material retained with this report. @aiUsage Use beside the saved excerpt and a separate request-to-locate action. */
/** @aiPurpose Provide evidenceAvailabilityLabel presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function evidenceAvailabilityLabel(availability: EvidenceAvailability): string {
  return availability === "retained" ? t("analysisEvidence.retainedSnippet")
    : availability === "empty" ? t("analysisEvidence.emptySnippet") : t("analysisEvidence.missingSnippet");
}

function BasisFact({ theme, children }: { theme: PluginTheme; children: string }) {
  return <Text selectable style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: 20, flexShrink: 1 }}>{children}</Text>;
}

/**
 * @aiPurpose Show the basis of a saved report without displacing its findings.
 * @aiUsage Mount by report id; opening details is local and never reads a source, starts a runner or changes a policy.
 * @aiBehavior Freshness, window truncation and usage remain independent; absent metadata stays unknown and exact zero stays visible.
 */
/** @aiPurpose Compose AnalysisBasis presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function AnalysisBasis({ theme, job }: { theme: PluginTheme; job: InsightJob }) {
  const [expanded, setExpanded] = useState(false);
  const { colors } = theme, facts = analysisFacts(job);
  const freshness = facts.freshness === "current" ? t("analysisEvidence.freshnessCurrent")
    : facts.freshness === "new_activity" ? t("analysisEvidence.freshnessChanged") : t("analysisEvidence.freshnessUnknown");
  const mode = facts.mode === "full" ? t("analysisEvidence.modeFull")
    : facts.mode === "incremental" ? t("analysisEvidence.modeIncremental") : t("analysisEvidence.modeUnknown");
  const coverage = job.coverage ? t("analysisEvidence.coverageSummary", { rows: job.coverage.rows,
    window: job.coverage.truncated ? t("analysisEvidence.windowTruncated") : t("analysisEvidence.windowUntruncated") }) : t("analysisEvidence.coverageUnknown");
  return <View testID="analysis-basis" style={{ gap: space.sm, padding: space.md, borderRadius: radius.md, borderWidth: hairline, borderColor: colors.border }}>
    <Pressable accessibilityRole="button" accessibilityLabel={expanded ? t("analysisEvidence.collapseBasis") : t("analysisEvidence.expandBasis")} accessibilityState={{ expanded }} onPress={() => setExpanded(!expanded)}
      style={{ flexDirection: "row", alignItems: "center", gap: space.sm, minHeight: 32 }}>
      <Icon name="Files" size={15} color={colors.foregroundMuted} />
      <Text style={{ flex: 1, color: colors.foreground, fontSize: fontSize.small, fontWeight: "600" }}>{t("analysisEvidence.title")}</Text>
      <Icon name={expanded ? "ChevronUp" : "ChevronDown"} size={14} color={colors.foregroundMuted} />
    </Pressable>
    <View style={{ flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: space.sm }}>
      <Badge theme={theme} label={freshness} tone={facts.freshness === "current" ? "neutral" : "warning"} />
    </View>
    <BasisFact theme={theme}>{`${mode} · ${coverage}`}</BasisFact>
    {expanded ? <View testID="analysis-basis-details" style={{ gap: space.sm, paddingTop: space.sm, borderTopWidth: hairline, borderTopColor: colors.border }}>
      {job.eligibility ? <BasisFact theme={theme}>{t("coreInsights.turnsToolCalls", { count: job.eligibility.turnCount, value2: job.eligibility.toolCount })}</BasisFact> : null}
      {job.coverage ? <>
        <BasisFact theme={theme}>{t("analysisEvidence.coverageWindow", { oldest: timestamp(job.coverage.oldestAt), newest: timestamp(job.coverage.newestAt) })}</BasisFact>
        <BasisFact theme={theme}>{t("analysisEvidence.inputCharacters", { value: job.coverage.characters })}</BasisFact>
      </> : null}
      {job.source ? <>
        <BasisFact theme={theme}>{t("analysisEvidence.sourceVersion", { epoch: job.source.epoch, seq: job.source.endSeq })}</BasisFact>
        <BasisFact theme={theme}>{t("analysisEvidence.captureTime", { value: timestamp(job.source.capturedAt) })}</BasisFact>
      </> : <BasisFact theme={theme}>{t("analysisEvidence.sourceUnknown")}</BasisFact>}
      {job.reviewContext?.mode === "incremental" ? <>
        <BasisFact theme={theme}>{t("analysisEvidence.baseReport", { id: job.reviewContext.baseJobId ?? t("analysisEvidence.unknown") })}</BasisFact>
        <BasisFact theme={theme}>{t("analysisEvidence.newRows", { count: job.reviewContext.newRows })}</BasisFact>
        <BasisFact theme={theme}>{t("analysisEvidence.unresolved", { count: job.reviewContext.unresolvedFindings })}</BasisFact>
      </> : null}
      {facts.freshness === "new_activity" ? <>
        <BasisFact theme={theme}>{t("analysisEvidence.latestActivity", { time: timestamp(job.freshness?.latestTurnAt) })}</BasisFact>
        {job.freshness?.newTurns !== null && job.freshness?.newTurns !== undefined ? <BasisFact theme={theme}>{t("analysisEvidence.newTurns", { count: job.freshness.newTurns })}</BasisFact> : null}
      </> : null}
      <BasisFact theme={theme}>{job.runner ? t("analysisEvidence.runnerRecorded", { label: job.runner.label }) : t("analysisEvidence.runnerUnknown")}</BasisFact>
      <BasisFact theme={theme}>{t("analysisEvidence.modelDetails", { model: job.model, effort: job.effort ?? t("analysisEvidence.unknown") })}</BasisFact>
      <BasisFact theme={theme}>{t("analysisEvidence.inputTokens", { value: exactNumber(facts.inputTokens) })}</BasisFact>
      <BasisFact theme={theme}>{t("analysisEvidence.outputTokens", { value: exactNumber(facts.outputTokens) })}</BasisFact>
      <BasisFact theme={theme}>{t("analysisEvidence.availabilitySummary", { retained: facts.retainedReferences, total: facts.totalReferences })}</BasisFact>
    </View> : null}
  </View>;
}
