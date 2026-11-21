/** @aiPurpose Present passive source/session evidence and safe sharing in one compact host-scoped workspace. */
import { useMemo, useState } from "react";
import { Text, View } from "react-native";
import { useQuery } from "@tanstack/react-query";
import { useRpc, type PluginScreenProps } from "../../host/adapter";
import { ScrollView, copyText } from "../../host/ui";
import { BRIDGE_OPERATIONS, type BridgeRequestDiagnostics, type DiagnosticsSnapshot } from "../shared/domain/diagnostics";
import { formatDiagnosticsSummary, getDiagnostics } from "../shared/diagnostics";
import { t } from "../shared/i18n";
import { Badge, Banner, IconButton, MetaLine, Row, SectionHeader, Skeleton, Toolbar, type Tone } from "../../uiux/systems/inspector/inspector.native";
import { diagnosticsStyles } from "./diagnostics.styles";

type ScreenProps = PluginScreenProps & { onBack?: () => void };
type Styles = ReturnType<typeof diagnosticsStyles>;
type Theme = PluginScreenProps["theme"];
const stamp = (value: string | null | undefined) => value ? new Date(value).toLocaleString() : t("diagnostics.notObserved");
const directoryTones: Record<NonNullable<DiagnosticsSnapshot["directory"]>["status"], Tone> = { healthy: "success", stale: "warning", unavailable: "danger", unknown: "neutral" };
const sessionTones: Record<NonNullable<DiagnosticsSnapshot["bridge"]>["session"]["state"], Tone> = { open: "info", closed: "danger", disposed: "warning", unknown: "neutral" };

/**
 * @aiPurpose Show a host's cached diagnostics without native probes or business commands.
 * @aiUsage Register as diagnostics with standard screen props; onBack requests the parent to restore its original context.
 * @aiBehavior Switching host resets local feedback/disclosure; refresh only re-reads memory. Copy is disabled during failed or pending reads.
 */
/** @aiPurpose Compose DiagnosticsScreen presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function DiagnosticsScreen(props: ScreenProps) {
  return <DiagnosticsHostScreen key={props.host.id} {...props} />;
}

function DiagnosticsHostScreen({ theme, layout, host, onBack }: ScreenProps) {
  const styles = useMemo(() => diagnosticsStyles(theme, layout.compact), [theme, layout.compact]);
  const read = useRpc(getDiagnostics);
  const query = useQuery({ queryKey: ["observatory", "diagnostics", host.id], queryFn: () => read({}), retry: false,
    staleTime: 0, refetchOnWindowFocus: false });
  const [feedback, setFeedback] = useState<"copied" | "copyFailed" | null>(null);
  const [copying, setCopying] = useState(false);
  const data = query.data;
  const copy = async () => {
    if (!data || query.isError || query.isFetching || copying) return;
    setCopying(true); setFeedback(null);
    try { await copyText(formatDiagnosticsSummary(data)); setFeedback("copied"); }
    catch { setFeedback("copyFailed"); }
    finally { setCopying(false); }
  };
  const refresh = () => { setFeedback(null); void query.refetch(); };
  return <View style={styles.screen} testID="diagnostics-screen">
    <Toolbar theme={theme}>
      {onBack ? <IconButton theme={theme} icon="ArrowLeft" label={t("diagnostics.back")} onPress={onBack} /> : null}
      <Text style={styles.heading}>{t("diagnostics.title")}</Text>
      <IconButton theme={theme} icon="Copy" label={t("diagnostics.copy")} onPress={() => void copy()} disabled={!data || query.isError || query.isFetching || copying} />
      <IconButton theme={theme} icon="RefreshCw" label={t("diagnostics.refresh")} onPress={refresh} disabled={query.isFetching} />
    </Toolbar>
    <ScrollView contentContainerStyle={styles.content}>
      <View style={styles.context}>
        <Text style={styles.value}>{host.label}</Text>
        <Text style={styles.muted}>{t("diagnostics.scope")}</Text>
        {data ? <Fact styles={styles} label={t("diagnostics.capturedAt")} value={stamp(data.capturedAt)} /> : null}
      </View>
      {query.isError ? <Banner theme={theme} tone="danger" message={t(data ? "diagnostics.refreshFailed" : "diagnostics.readFailed")} /> : null}
      {feedback ? <Banner theme={theme} tone={feedback === "copied" ? "success" : "warning"} message={t(`diagnostics.${feedback}`)} /> : null}
      {query.isPending ? <Skeleton theme={theme} rows={3} /> : null}
      {data ? <>
        <DirectoryEvidence theme={theme} data={data.directory} styles={styles} />
        <SessionEvidence theme={theme} data={data.bridge} styles={styles} />
        <RequestHistory theme={theme} data={data.bridge} styles={styles} />
        <ArtifactEvidence theme={theme} data={data} styles={styles} />
      </> : null}
    </ScrollView>
  </View>;
}

function Fact({ styles, label, value }: { styles: Styles; label: string; value: string }) {
  return <View style={styles.fact}><Text style={styles.label}>{label}</Text><Text selectable style={styles.value}>{value}</Text></View>;
}
function DirectoryEvidence({ theme, data, styles }: { theme: Theme; data: DiagnosticsSnapshot["directory"]; styles: Styles }) {
  const status = data?.status ?? "unknown";
  return <View style={styles.section} testID="diagnostics-directory">
    <View style={styles.sectionHeading}>
      <Text style={styles.sectionTitle}>{t("diagnostics.directory")}</Text>
      <Badge theme={theme} label={t(`diagnostics.directory_${status}`)} tone={directoryTones[status]} />
    </View>
    <Fact styles={styles} label={t("diagnostics.lastAttempt")} value={stamp(data?.lastAttemptAt)} />
    <Fact styles={styles} label={t("diagnostics.lastSuccess")} value={stamp(data?.lastSuccessAt)} />
    {data?.errorCode ? <Text style={styles.value}>{t(`diagnostics.directoryError_${data.errorCode}`)}</Text> : null}
    <Text style={styles.muted}>{t("diagnostics.directoryScope")}</Text>
  </View>;
}
function SessionEvidence({ theme, data, styles }: { theme: Theme; data: DiagnosticsSnapshot["bridge"]; styles: Styles }) {
  const state = data?.session.state ?? "unknown";
  return <View style={styles.section} testID="diagnostics-session">
    <View style={styles.sectionHeading}>
      <Text style={styles.sectionTitle}>{t("diagnostics.session")}</Text>
      <Badge theme={theme} label={t(`diagnostics.session_${state}`)} tone={sessionTones[state]} />
    </View>
    <Fact styles={styles} label={t("diagnostics.lastFrame")} value={stamp(data?.session.lastFrameAt)} />
    <Fact styles={styles} label={t("diagnostics.lastClosed")} value={stamp(data?.session.lastClosedAt)} />
    {data ? <MetaLine theme={theme} numberOfLines={2} parts={[
      t("diagnostics.sessionEpoch", { epoch: data.session.epoch }), t("diagnostics.pending", { pending: data.pending, queued: data.queued }),
    ]} /> : null}
    <Text style={styles.muted}>{t("diagnostics.sessionScope")}</Text>
  </View>;
}
function RequestHistory({ theme, data, styles }: { theme: Theme; data: DiagnosticsSnapshot["bridge"]; styles: Styles }) {
  const [open, setOpen] = useState(false);
  const observed = data?.operations.filter((entry) => entry.attempts > 0) ?? [];
  const failures = observed.reduce((sum, entry) => sum + entry.failures, 0);
  return <View style={styles.history} testID="diagnostics-history">
    <SectionHeader theme={theme} title={t("diagnostics.history")} open={open} onToggle={() => setOpen(!open)} />
    <MetaLine theme={theme} numberOfLines={2} parts={[
      t("diagnostics.observedOperations", { count: observed.length, total: BRIDGE_OPERATIONS.length }),
      t("diagnostics.failures", { count: failures }),
    ]} />
    <Text style={styles.muted}>{t("diagnostics.historyScope")}</Text>
    {open ? observed.length ? observed.map((entry) => <RequestEvidence key={entry.operation} theme={theme} entry={entry} styles={styles} currentEpoch={data!.session.epoch} />)
      : <Text style={styles.value}>{t("diagnostics.noHistory")}</Text> : null}
  </View>;
}
function evidenceEpoch(epoch: number | null, currentEpoch: number) {
  return epoch === null ? t("diagnostics.notSent") : t(epoch === currentEpoch ? "diagnostics.currentEpoch" : "diagnostics.previousEpoch", { epoch });
}
function RequestEvidence({ theme, entry, styles, currentEpoch }: { theme: Theme; entry: BridgeRequestDiagnostics; styles: Styles; currentEpoch: number }) {
  const failure = entry.lastFailureCode ? t(`diagnostics.failure_${entry.lastFailureCode}`) : t("diagnostics.failureUnknown");
  const latest = entry.lastOutcome === "failure" ? failure : t(entry.lastOutcome === "success" ? "diagnostics.responseReceived" : "diagnostics.responsePending");
  return <Row theme={theme} title={t(`diagnostics.operation_${entry.operation}`)} trailing={<Badge theme={theme} label={latest} tone={entry.lastOutcome === "failure" ? "warning" : "neutral"} />}>
    <MetaLine theme={theme} numberOfLines={2} parts={[t("diagnostics.requestCounts", { attempts: entry.attempts, successes: entry.successes, failures: entry.failures })]} />
    <Fact styles={styles} label={t("diagnostics.lastAttempt")} value={stamp(entry.lastAttemptAt)} />
    {entry.lastSuccessAt ? <Fact styles={styles} label={t("diagnostics.lastResponse")} value={`${stamp(entry.lastSuccessAt)} · ${evidenceEpoch(entry.lastSuccessEpoch, currentEpoch)}`} /> : null}
    {entry.lastFailureAt ? <Fact styles={styles} label={t("diagnostics.lastFailure")} value={`${stamp(entry.lastFailureAt)} · ${failure} · ${evidenceEpoch(entry.lastFailureEpoch, currentEpoch)}`} /> : null}
  </Row>;
}
function ArtifactEvidence({ theme, data, styles }: { theme: Theme; data: DiagnosticsSnapshot; styles: Styles }) {
  const [open, setOpen] = useState(false);
  return <View style={styles.detail}>
    <SectionHeader theme={theme} title={t("diagnostics.artifact")} open={open} onToggle={() => setOpen(!open)} />
    <MetaLine theme={theme} numberOfLines={2} parts={[`SDK ${data.target.sdk}`, `UI ${data.target.clientChunk.hash}`]} />
    {open ? <>
      <Fact styles={styles} label={t("diagnostics.apiBytes")} value={`${data.target.clientChunk.api} / ${data.target.clientChunk.bytes.toLocaleString()}`} />
      <Fact styles={styles} label={t("diagnostics.observedSince")} value={stamp(data.bridge?.startedAt)} />
      <Text style={styles.muted}>{t("diagnostics.targetScope")}</Text>
    </> : null}
  </View>;
}
