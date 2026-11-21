/** @aiPurpose Investigate retained per-Agent analysis jobs with passive reads and explicit single-job cancellation outcomes. */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Pressable, Text, View, type FlatList as NativeFlatList } from "react-native";
import { useRpc, type PluginScreenProps } from "../../host/adapter";
import { FlatList } from "../../host/ui";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getInsightJobsSnapshot, type InsightJobsSnapshot } from "../shared/insight-jobs";
import { cancelInsight, type InsightJobSummary } from "../shared/insights";
import { t, type Params } from "../shared/i18n";
import { Badge, Banner, Button, IconButton, MetaLine, ChoiceGroup, SearchControls, Sheet, Skeleton, Toolbar, aria, type Tone } from "../../uiux/systems/inspector/inspector.native";
import { canCancelInsightJob, completeInsightJobsSource, filterInsightJobs, initialInsightJobsView, insightCancellation, insightJobAgents,
  type InsightCancellation, type InsightJobsFilters, type InsightJobsView } from "./insight-jobs-model";
import { insightJobsStyles } from "./insight-jobs.styles";
import { useForeground } from "../../uiux/systems/inspector/inspector.native";

type JobsProps = PluginScreenProps & { onBack?: () => void; onOpenAnalysis?: () => void; onOpenReport?: (jobId: string) => void };
type Theme = PluginScreenProps["theme"];
type Styles = ReturnType<typeof insightJobsStyles>;
type Feedback = Record<string, InsightCancellation>;
const messageIds = {
  all: "insightJobs.all",
  qualityAvailable: "insightJobs.qualityAvailable",
  qualityPartial: "insightJobs.qualityPartial",
  generated: "insightJobs.generated",
  revision: "insightJobs.revision",
  sourceCode: "insightJobs.sourceCode",
  empty: "insightJobs.empty",
  noMatches: "insightJobs.noMatches",
  readFailed: "insightJobs.readFailed",
  refreshFailed: "insightJobs.refreshFailed",
  details: "insightJobs.details",
  hideDetails: "insightJobs.hideDetails",
  recordId: "insightJobs.recordId",
  agentId: "insightJobs.agentId",
  projectName: "insightJobs.projectName",
  rawState: "insightJobs.rawState",
  created: "insightJobs.created",
  updated: "insightJobs.updated",
  stage: "insightJobs.stage",
  error: "insightJobs.error",
  runner: "insightJobs.runner",
  runnerAgent: "insightJobs.runnerAgent",
  model: "insightJobs.model",
  effort: "insightJobs.effort",
  mode: "insightJobs.mode",
  coverage: "insightJobs.coverage",
  rows: "insightJobs.rows",
  characters: "insightJobs.characters",
  oldest: "insightJobs.oldest",
  newest: "insightJobs.newest",
  coverageTruncated: "insightJobs.coverageTruncated",
  coverageComplete: "insightJobs.coverageComplete",
  inputTokens: "insightJobs.inputTokens",
  outputTokens: "insightJobs.outputTokens",
  epoch: "insightJobs.epoch",
  endSeq: "insightJobs.endSeq",
  freshness: "insightJobs.freshness",
  current: "insightJobs.current",
  new_activity: "insightJobs.new_activity",
  reviewMode: "insightJobs.reviewMode",
  full: "insightJobs.full",
  incremental: "insightJobs.incremental",
  baseJob: "insightJobs.baseJob",
  unknown: "insightJobs.unknown",
} as const;
type MessageKey = keyof typeof messageIds;
const msg = (key: MessageKey, params?: Params) => t(messageIds[key], params);
const labels = { queued: "insightJobs.queued", collecting: "insightJobs.collecting", analyzing: "insightJobs.analyzing", completed: "insightJobs.completed",
  failed: "insightJobs.failed", canceled: "insightJobs.canceled", skipped: "insightJobs.skipped", manual: "insightJobs.manual", auto: "insightJobs.auto" } as const;
const feedbackLabels: Record<InsightCancellation, string> = { pending: "insightJobs.cancelPending", canceled: "insightJobs.cancelCanceled",
  completed: "insightJobs.cancelCompleted", failed: "insightJobs.cancelFailed", skipped: "insightJobs.cancelSkipped", unknown: "insightJobs.cancelUnknown" };
const statusTones: Record<InsightJobSummary["status"], Tone> = { queued: "warning", collecting: "info", analyzing: "info", completed: "success", failed: "danger", canceled: "neutral", skipped: "neutral" };
const stamp = (at: string | null | undefined) => at && Number.isFinite(Date.parse(at)) ? new Date(at).toLocaleString() : t("insightJobs.unknown");
const value = (fact: string | number | null | undefined) => fact === null || fact === undefined || fact === "" ? t("insightJobs.unknown") : String(fact);
const snapshotKey = (host: string) => ["observatory", "insight-jobs", host, "snapshot"] as const;

/**
 * @aiPurpose Manage the selected host's retained analysis jobs without discovering or launching a runner.
 * @aiUsage Register as insight-jobs; onOpenReport hands a saved job ID to the existing report path. Workspace params do not establish identity scope.
 * @aiBehavior Host-scoped cached view and command outcomes survive navigation; unknown cancellation stays locked across refreshes. Background queries stop.
 * @aiLayout A fixed compact toolbar/search precedes one virtualized list; detail and actions remain local to each job.
 */
/** @aiPurpose Compose InsightJobsScreen presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function InsightJobsScreen(props: JobsProps) {
  return <InsightJobsHost key={props.host.id} {...props} />;
}

function useJobsWorkspace(host: string) {
  const client = useQueryClient();
  const viewKey = useMemo(() => ["observatory", "insight-jobs", host, "view"] as const, [host]);
  const feedbackKey = useMemo(() => ["observatory", "insight-jobs", host, "cancellation"] as const, [host]);
  const [view, setView] = useState(() => client.getQueryData<InsightJobsView>(viewKey) ?? initialInsightJobsView());
  const feedback = useQuery<Feedback>({ queryKey: feedbackKey, queryFn: async () => ({}), enabled: false, initialData: {}, staleTime: Infinity, gcTime: Infinity }).data;
  const latest = useRef(view), offset = useRef(view.offset);
  latest.current = view;
  useEffect(() => () => { client.setQueryData(viewKey, { ...latest.current, offset: offset.current }); }, [client, viewKey]);
  const saveView = (next: InsightJobsView) => { latest.current = next; offset.current = next.offset; client.setQueryData(viewKey, next); setView(next); };
  const saveFeedback = (id: string, outcome: InsightCancellation) => client.setQueryData<Feedback>(feedbackKey, old => ({ ...old, [id]: outcome }));
  const hasFeedback = (id: string) => Boolean(client.getQueryData<Feedback>(feedbackKey)?.[id]);
  return { view, saveView, feedback, saveFeedback, hasFeedback, offset };
}

function InsightJobsHost({ theme, layout, host, navigation, onBack, onOpenAnalysis, onOpenReport }: JobsProps) {
  const styles = useMemo(() => insightJobsStyles(theme, layout.compact), [theme, layout.compact]);
  const foreground = useForeground();
  const read = useRpc(getInsightJobsSnapshot), cancelJob = useRpc(cancelInsight);
  const query = useQuery({ queryKey: snapshotKey(host.id), queryFn: () => read({}), retry: false, enabled: foreground,
    staleTime: 5_000, refetchInterval: foreground ? 15_000 : false, refetchIntervalInBackground: false, refetchOnWindowFocus: false });
  const workspace = useJobsWorkspace(host.id), { view, saveView } = workspace;
  const [filtersOpen, setFiltersOpen] = useState(false), [sourceOpen, setSourceOpen] = useState(false);
  const jobs = query.data?.jobs ?? [];
  const filtered = useMemo(() => filterInsightJobs(jobs, view.filters), [jobs, view.filters]);
  const changeFilters = (filters: InsightJobsFilters) => saveView({ ...view, filters, offset: 0 });
  const clear = () => changeFilters(initialInsightJobsView().filters);
  const hasFilters = Boolean(view.filters.search || view.filters.status || view.filters.trigger || view.filters.agentId !== null);
  const toggle = (id: string) => saveView({ ...view, offset: workspace.offset.current, expanded: view.expanded.includes(id) ? view.expanded.filter(entry => entry !== id) : [...view.expanded, id] });
  const cancel = async (job: InsightJobSummary) => {
    if (!canCancelInsightJob(job) || workspace.hasFeedback(job.id) || query.isError || query.isFetching || !foreground) return;
    workspace.saveFeedback(job.id, "pending");
    try { workspace.saveFeedback(job.id, insightCancellation(job.id, await cancelJob({ jobId: job.id }))); }
    catch { workspace.saveFeedback(job.id, "unknown"); }
    void query.refetch();
  };
  const incomplete = query.data ? !completeInsightJobsSource(query.data.source) : false;
  const header = query.isError || incomplete || query.isPending ? <View style={styles.empty}>
    {query.isError ? <Banner theme={theme} tone="danger" message={msg(query.data ? "refreshFailed" : "readFailed")} /> : null}
    {incomplete ? <Banner theme={theme} tone="warning" message={t("insightJobs.incomplete")} /> : null}
    {query.data?.source.truncated ? <Banner theme={theme} tone="warning" message={t("insightJobs.truncated", { loaded: jobs.length, total: query.data.source.total })} /> : null}
    {query.isPending ? <Skeleton theme={theme} rows={3} /> : null}
  </View> : null;
  const empty = !query.isPending && !query.isError && !incomplete ? <Text style={styles.value}>{msg(hasFilters ? "noMatches" : "empty")}</Text> : null;
  return <View style={styles.screen} testID="insight-jobs-screen">
    <Toolbar theme={theme}>
      {onBack ? <IconButton theme={theme} icon="ArrowLeft" label={t("insightJobs.back")} onPress={onBack} /> : null}
      <Text style={styles.heading}>{t("insightJobs.title")}</Text>
      {onOpenAnalysis ? <IconButton theme={theme} icon="Sparkles" label={t("insightJobs.analysis")} onPress={onOpenAnalysis} /> : null}
      <IconButton theme={theme} icon="RefreshCw" label={t("insightJobs.refresh")} disabled={query.isFetching || !foreground} onPress={() => void query.refetch()} />
    </Toolbar>
    <View style={styles.controls}>
      <Text style={styles.muted}>{t("insightJobs.scope", { host: host.label })}</Text>
      <SearchControls theme={theme} value={view.filters.search} onChangeText={search => changeFilters({ ...view.filters, search })} label={t("insightJobs.search")}
        filtered={hasFilters} filterLabel={t("insightJobs.filters")} clearLabel={t("insightJobs.clear")} onOpenFilters={() => setFiltersOpen(true)} onClear={clear} />
      {query.data ? <View style={styles.sources} testID="insight-jobs-source">
        <Badge theme={theme} tone={incomplete ? "warning" : "neutral"} label={msg(incomplete ? "qualityPartial" : "qualityAvailable")} />
        <Text style={styles.muted}>{t("insightJobs.loaded", { count: jobs.length, visible: filtered.length })}</Text>
        <IconButton theme={theme} icon="Info" label={t("insightJobs.sourceDetails")} onPress={() => setSourceOpen(true)} />
      </View> : null}
    </View>
    <JobsList styles={styles} jobs={filtered} view={view} header={header} empty={empty}
      renderJob={job => <JobRecord theme={theme} styles={styles} job={job} open={view.expanded.includes(job.id)} feedback={workspace.feedback[job.id]}
        cancelDisabled={query.isError || query.isFetching || !foreground} onToggle={() => toggle(job.id)} onCancel={() => void cancel(job)}
        onOpenAgent={navigation ? () => navigation.openAgent({ agentId: job.agentId, serverId: host.id }) : undefined} onOpenReport={onOpenReport ? () => onOpenReport(job.id) : undefined} />}
      onScroll={at => { workspace.offset.current = at; }} />
    <JobsFilters theme={theme} styles={styles} open={filtersOpen} onClose={() => setFiltersOpen(false)} jobs={jobs} filters={view.filters} onChange={changeFilters} onClear={clear} />
    <Sheet title={t("insightJobs.sourceDetails")} open={sourceOpen} onClose={() => setSourceOpen(false)}>
      {query.data ? <SourceScope styles={styles} snapshot={query.data} /> : null}
    </Sheet>
  </View>;
}

function JobsList({ styles, jobs, view, header, empty, renderJob, onScroll }: {
  styles: Styles; jobs: InsightJobSummary[]; view: InsightJobsView; header: React.ReactElement | null; empty: React.ReactElement | null; renderJob: (job: InsightJobSummary) => React.ReactElement; onScroll: (at: number) => void;
}) {
  const list = useRef<NativeFlatList<InsightJobSummary>>(null), restored = useRef(false), initialOffset = useRef(view.offset), previousFilters = useRef(view.filters);
  useEffect(() => {
    if (previousFilters.current !== view.filters) { list.current?.scrollToOffset({ offset: 0, animated: false }); restored.current = true; }
    previousFilters.current = view.filters;
  }, [view.filters]);
  const onSize = useCallback(() => {
    if (!restored.current && jobs.length) { restored.current = true; if (initialOffset.current > 0) list.current?.scrollToOffset({ offset: initialOffset.current, animated: false }); }
  }, [jobs.length]);
  return <FlatList ref={list} testID="insight-jobs-list" style={styles.list} contentContainerStyle={styles.content} data={jobs} keyExtractor={job => job.id}
    renderItem={({ item }) => renderJob(item)} ListHeaderComponent={header} ListEmptyComponent={empty} onContentSizeChange={onSize}
    onScroll={event => onScroll(event.nativeEvent.contentOffset.y)} scrollEventThrottle={64} />;
}

function JobRecord({ theme, styles, job, open, feedback, cancelDisabled, onToggle, onCancel, onOpenAgent, onOpenReport }: {
  theme: Theme; styles: Styles; job: InsightJobSummary; open: boolean; feedback?: InsightCancellation; cancelDisabled: boolean;
  onToggle: () => void; onCancel: () => void; onOpenAgent?: () => void; onOpenReport?: () => void;
}) {
  const agentIdentity = job.agentId.length <= 20 ? job.agentId : `${job.agentId.slice(0, 8)}…${job.agentId.slice(-6)}`;
  return <View style={styles.row} testID={`insight-job-${job.id}`}>
    <View style={styles.rowHeading}>
      <Text style={styles.title} numberOfLines={2}>{job.agentTitle || job.agentId}</Text>
      <Badge theme={theme} label={t(labels[job.status])} tone={statusTones[job.status]} />
      <Badge theme={theme} label={t(labels[job.trigger])} />
    </View>
    <MetaLine theme={theme} numberOfLines={2} parts={[agentIdentity, job.runner?.label ?? job.model, stamp(job.updatedAt)]} />
    <View style={styles.actions}>
      <IconButton theme={theme} icon={open ? "ChevronUp" : "ChevronDown"} label={msg(open ? "hideDetails" : "details")} active={open} onPress={onToggle} />
      {onOpenAgent ? <IconButton theme={theme} icon="ExternalLink" label={t("insightJobs.openAgent")} onPress={onOpenAgent} /> : null}
      {job.reportAvailable && onOpenReport ? <IconButton theme={theme} icon="FileText" label={t("insightJobs.openReport")} onPress={onOpenReport} /> : null}
      {canCancelInsightJob(job) ? <Button theme={theme} size="sm" icon="X" label={t("insightJobs.cancel")} disabled={cancelDisabled || Boolean(feedback)} busy={feedback === "pending"} onPress={onCancel} /> : null}
    </View>
    {feedback ? <View testID={`insight-job-feedback-${job.id}`} {...aria({ live: "polite" })}>
      <Banner theme={theme} tone={feedback === "canceled" ? "success" : feedback === "pending" ? "info" : "warning"} message={t(feedbackLabels[feedback])} />
    </View> : null}
    {open ? <JobDetails styles={styles} job={job} /> : null}
  </View>;
}

function Fact({ styles, label, fact }: { styles: Styles; label: MessageKey; fact: string | number | null | undefined }) {
  return <View style={styles.fact}><Text style={styles.muted}>{msg(label)}</Text><Text style={styles.value} selectable>{value(fact)}</Text></View>;
}
function JobDetails({ styles, job }: { styles: Styles; job: InsightJobSummary }) {
  const facts: Array<[MessageKey, string | number | null | undefined]> = [
    ["recordId", job.id], ["agentId", job.agentId], ["projectName", job.projectName], ["rawState", job.status], ["created", stamp(job.createdAt)], ["updated", stamp(job.updatedAt)],
    ["stage", job.stage], ["runner", job.runner?.label], ["model", job.model], ["effort", job.effort], ["mode", job.runner?.modeId],
    ["inputTokens", job.usage?.inputTokens], ["outputTokens", job.usage?.outputTokens], 
  ];
  if (job.error) facts.push(["error", job.error]);
  if (job.runner?.sessionAgentId) facts.push(["runnerAgent", job.runner.sessionAgentId]);
  if (job.coverage) facts.push(["coverage", msg(job.coverage.truncated ? "coverageTruncated" : "coverageComplete")], ["rows", job.coverage.rows], ["characters", job.coverage.characters], ["oldest", stamp(job.coverage.oldestAt)], ["newest", stamp(job.coverage.newestAt)]);
  else facts.push(["coverage", null]);
  if (job.source) facts.push(["epoch", job.source.epoch], ["endSeq", job.source.endSeq]);
  if (job.freshness) facts.push(["freshness", msg(job.freshness.state)]);
  if (job.reviewContext) facts.push(["reviewMode", msg(job.reviewContext.mode)], ["baseJob", job.reviewContext.baseJobId]);
  return <View style={styles.detail} testID={`insight-job-detail-${job.id}`}>
    {job.summary ? <><Text style={styles.muted}>{t("insightJobs.summary")}</Text><Text style={styles.value} selectable>{job.summary}</Text></> : null}
    {facts.map(([label, fact]) => <Fact key={label} styles={styles} label={label} fact={fact} />)}
    <Text style={styles.muted}>{t("insightJobs.reportOnly")}</Text>
  </View>;
}
function SourceScope({ styles, snapshot }: { styles: Styles; snapshot: InsightJobsSnapshot }) {
  return <View style={styles.filterSection}>
    <Text style={styles.value}>{t("insightJobs.retention", { reports: snapshot.retention.reportLimit, statuses: snapshot.retention.statusLimit, bytes: snapshot.retention.reportBytesLimit })}</Text>
    <Text style={styles.value}>{t("insightJobs.retainedCount", { total: snapshot.source.total })}</Text>
    <Fact styles={styles} label="generated" fact={stamp(snapshot.generatedAt)} />
    <Fact styles={styles} label="revision" fact={snapshot.revision} />
    <Fact styles={styles} label="sourceCode" fact={snapshot.source.errorCode ?? t("insightJobs.none")} />
  </View>;
}

function JobsFilters({ theme, styles, open, onClose, jobs, filters, onChange, onClear }: {
  theme: Theme; styles: Styles; open: boolean; onClose: () => void; jobs: InsightJobSummary[]; filters: InsightJobsFilters;
  onChange: (filters: InsightJobsFilters) => void; onClear: () => void;
}) {
  const agents = insightJobAgents(jobs);
  if (filters.agentId !== null && !agents.some(agent => agent.id === filters.agentId)) agents.push({ id: filters.agentId, title: msg("unknown") });
  return <Sheet title={t("insightJobs.filters")} open={open} onClose={onClose}>
    <View testID="insight-jobs-filters" style={styles.filterSection}>
      <ChoiceGroup layout="wrap" theme={theme} label={t("insightJobs.statusFilter")} value={filters.status}
        options={[{ value: null, label: msg("all") }, ...Object.keys(statusTones).map(id => ({ value: id, label: t(labels[id as InsightJobSummary["status"]]) }))]}
        onChange={status => onChange({ ...filters, status: status as InsightJobsFilters["status"] })} />
      <ChoiceGroup layout="wrap" theme={theme} label={t("insightJobs.triggerFilter")} value={filters.trigger}
        options={[{ value: null, label: msg("all") }, { value: "manual", label: t(labels.manual) }, { value: "auto", label: t(labels.auto) }]}
        onChange={trigger => onChange({ ...filters, trigger: trigger as InsightJobsFilters["trigger"] })} />
      <ChoiceGroup layout="wrap" theme={theme} label={t("insightJobs.agentFilter")} value={filters.agentId}
        options={[{ value: null, label: msg("all") }, ...agents.map(agent => ({ value: agent.id, label: `${agent.title} · ${agent.id}` }))]}
        onChange={agentId => onChange({ ...filters, agentId })} />
      <Button theme={theme} size="sm" icon="FilterX" label={t("insightJobs.clear")} onPress={onClear} />
    </View>
  </Sheet>;
}
