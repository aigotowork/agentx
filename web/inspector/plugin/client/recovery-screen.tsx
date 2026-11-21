import { t } from "../shared/text";
/** @aiPurpose Coordinate grouped recovery, audit history and automatic backoff policy in a compact AgentX-native screen. */
import { useMemo, useRef, useState } from "react";
import { Text, View } from "react-native";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRpc, type PluginScreenProps, type PluginSurfaceProps } from "../../host/adapter";
import { Loading, TextInput } from "../../host/ui";
import { configureAutomation, cycleLabels, manageAutomation, type Policy, type RecoveryCycle } from "../shared/automation";
import { categoryLabels, continueRecovery, CONTINUE_PROMPT, listRecovery, type Category, type Incident, type Target } from "../shared/recovery";
import { Check, IconAction, Status, shortTime, toggle } from "./recovery-controls";
import { categoryIcons, matches, recoveryOutcomeCounts, recoveryRecords, statusIcons, statusTones, type Filters } from "./recovery-model";
import { IncidentGroups, RecordGroups } from "./recovery-list";
import { AutomationSettings, ScopePicker, scopeOptions } from "./recovery-settings";
import { recoveryStyles } from "./recovery.styles";
import { Sheet } from "../../uiux/systems/inspector/inspector.native";

/** @aiPurpose Reset local selection and filters on host changes. @aiUsage Register as the recovery Screen; all operations stay scoped to its host. */
/** @aiPurpose Compose RecoveryScreen presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function RecoveryScreen(props: PluginScreenProps) {
  return <RecoveryHostScreen key={props.host.id} {...props} />;
}
/** @aiPurpose Embed recovery inside the observatory's existing host and workbench. @aiUsage Workspace panels constrain the initial view; automatic policy scope remains explicitly independent. */
/** @aiPurpose Compose RecoveryPanel presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function RecoveryPanel(props: PluginSurfaceProps & { workspaceId?: string }) {
  return <RecoveryHostScreen key={`${props.host.id}:${props.workspaceId ?? "all"}`} {...props} params={props.workspaceId ? { workspaceId: props.workspaceId } : {}} embedded />;
}
function RecoveryHostScreen({ theme, layout, host, navigation, params, embedded = false }: PluginScreenProps & { embedded?: boolean }) {
  const s = useMemo(() => recoveryStyles(theme, layout.compact), [theme, layout.compact]);
  const list = useRpc(listRecovery), send = useRpc(continueRecovery), configure = useRpc(configureAutomation), manage = useRpc(manageAutomation);
  const cache = useQueryClient(), queryKey = ["agentx-retry", host.id];
  const query = useQuery({ queryKey, queryFn: () => list({}), refetchInterval: 3000, retry: false,
    refetchIntervalInBackground: false, staleTime: 2000 });
  const [filters, setFilters] = useState<Filters>({ projects: [], workspaces: params.workspaceId ? [params.workspaceId] : [], categories: [], search: "" });
  const [selected, setSelected] = useState<string[]>([]), [tab, setTab] = useState<"agents" | "records">("agents");
  const [statusFilter, setStatusFilter] = useState<RecoveryCycle["status"] | null>(null);
  const [filterOpen, setFilterOpen] = useState(false), [settingsOpen, setSettingsOpen] = useState(false), [promptOpen, setPromptOpen] = useState(false);
  const [feedback, setFeedback] = useState("");
  const sending = useRef(false);
  const refresh = () => cache.invalidateQueries({ queryKey });
  const mutation = useMutation({
    mutationFn: (input: { operationId: string; targets: Target[] }) => send(input), retry: false,
    onSuccess: ({ attempts }) => {
      setFeedback(t("recoveryDelivery.batchResult", recoveryOutcomeCounts(attempts))); setSelected([]);
    },
    onError: () => setFeedback(t("personalRecovery.m039")),
    onSettled: async () => { await refresh(); sending.current = false; },
  });
  const policyMutation = useMutation({
    mutationFn: (policy: Policy) => configure(policy), retry: false,
    onSuccess: ({ policy }) => { setSettingsOpen(false); setFeedback(policy.enabled ? t("personalRecovery.m040") : t("personalRecovery.m022")); },
    onError: (error) => setFeedback(error instanceof Error ? error.message : t("personalRecovery.m041")), onSettled: refresh,
  });
  const manageMutation = useMutation({
    mutationFn: (input: { cycleId: string; action: "pause" | "resume" | "stop" }) => manage(input), retry: false,
    onError: (error) => setFeedback(error instanceof Error ? error.message : t("personalRecovery.m042")), onSettled: refresh,
  });
  const data = query.data, automation = data?.automation, policy = automation?.policy, incidents = data?.incidents ?? [];
  const cycles = automation?.cycles ?? [], cycleMap = new Map(cycles.map((cycle) => [cycle.agentId, cycle]));
  const currentCycles = [...cycleMap.values()], attempts = data?.attempts ?? [];
  const stale = query.isError || !data || Date.now() - Date.parse(data.scannedAt) > 15000;
  const disabled = stale || mutation.isPending || manageMutation.isPending || policyMutation.isPending;
  const visible = incidents.filter((item) => {
    const cycle = cycleMap.get(item.agentId);
    const outstanding = item.state !== "superseded" || (cycle?.incidentId === item.id && ["waiting", "checking", "blocked", "paused"].includes(cycle.status));
    return matches(item, filters) && (statusFilter ? cycle?.status === statusFilter : outstanding);
  });
  const eligible = visible.filter((i) => i.state === "detected" && !i.blockedReason);
  const chosen = eligible.filter((i) => selected.includes(i.id)), targets = chosen.length ? chosen : eligible;
  const filtered = filters.projects.length + filters.workspaces.length + filters.categories.length > 0 || !!filters.search.trim() || !!statusFilter;
  const records = data ? recoveryRecords(data).filter((record) => matches(record, filters)) : [];
  const options = automation?.catalog ?? scopeOptions(incidents);
  const submit = (items: Incident[]) => {
    if (disabled || sending.current || !items.length || items.length > 500) return;
    sending.current = true; setFeedback(t("personalRecovery.m043", { p0: items.length }));
    mutation.mutate({ operationId: `retry-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      targets: items.map(({ id, revision }) => ({ incidentId: id, revision })) });
  };
  const openAgent = navigation ? (agentId: string) => navigation.openAgent({ agentId, serverId: host.id }) : undefined;
  const scopeCount = (policy?.projectIds.length ?? 0) + (policy?.workspaceIds.length ?? 0);
  const scanError = query.isError || !!automation?.monitor.error;

  return <View style={s.screen}>
    <View style={s.toolbar}>
      <View style={s.header}>
        <View style={s.line}>{!embedded && <Text style={s.title}>{t("personalRecovery.m044")}</Text>}<Text style={s.muted}>{policy?.enabled ? t("personalRecovery.m045") : t("personalRecovery.m046")}</Text></View>
        <View style={s.actions}>
          <IconAction label={query.isFetching ? t("personalRecovery.m047") : t("personalRecovery.m048")} icon="ScanLine" disabled={query.isFetching || mutation.isPending} onPress={() => void query.refetch()} styles={s} />
          <IconAction label={t("personalRecovery.m049", { p0: policy?.enabled ? t("personalRecovery.m024") : t("personalRecovery.m050"), p1: scopeCount ? t("personalRecovery.m051") : t("personalRecovery.m052") })} icon={policy?.enabled ? "Zap" : "ZapOff"}
            value={policy?.enabled ? t("personalRecovery.m053") : t("personalRecovery.m054")} tone={policy?.enabled ? "success" : "muted"} selected={!!policy?.enabled} disabled={disabled || !policy}
            onPress={() => { if (policy) policyMutation.mutate({ ...policy, enabled: !policy.enabled }); }} styles={s} />
          <IconAction label={t("personalRecovery.m055")} icon="Settings2" disabled={!policy || policyMutation.isPending} onPress={() => { policyMutation.reset(); setSettingsOpen(true); }} styles={s} />
        </View>
      </View>
      <View style={s.header}>
        <View style={s.actions}>
          <IconAction label={t("personalRecovery.m056")} icon="Layers" value={visible.length} selected={tab === "agents"} onPress={() => setTab("agents")} styles={s} />
          <IconAction label={t("personalRecovery.m057")} icon="History" value={records.length} selected={tab === "records"} onPress={() => { setTab("records"); setStatusFilter(null); }} styles={s} />
          <IconAction label={t("personalRecovery.m058")} icon="ListFilter" selected={!!filtered} onPress={() => setFilterOpen(true)} styles={s} />
          {filtered && <IconAction label={t("personalRecovery.m059")} icon="FilterX" onPress={() => { setFilters({ projects: [], workspaces: [], categories: [], search: "" }); setStatusFilter(null); setSelected([]); }} styles={s} />}
        </View>
        {tab === "agents" && <View style={s.actions}>
          <Check label={t("personalRecovery.m060")} checked={eligible.length > 0 && chosen.length === eligible.length}
            disabled={disabled || !eligible.length} onPress={() => setSelected(chosen.length === eligible.length ? [] : eligible.map((i) => i.id))} styles={s} />
          <IconAction label={t("personalRecovery.m061", { p0: chosen.length ? t("personalRecovery.m062") : filtered ? t("personalRecovery.m063") : t("personalRecovery.m064"), p1: targets.length })} icon="Play" value={targets.length}
            primary disabled={disabled || !targets.length || targets.length > 500} onPress={() => submit(targets)} styles={s} />
        </View>}
      </View>
      <View style={s.actions}>{(["waiting", "checking", "recovered", "blocked"] as const).map((status) => {
        const count = currentCycles.filter((c) => c.status === status && incidents.some((i) => i.agentId === c.agentId && matches(i, filters))).length;
        return <IconAction key={status} label={`${cycleLabels[status]} ${count}`} icon={statusIcons[status]} value={count} tone={statusTones[status]}
          selected={statusFilter === status} onPress={() => { setStatusFilter(statusFilter === status ? null : status); setTab("agents"); }} styles={s} />;
      })}<Text style={s.muted}>5m → 20m → 1h</Text></View>
      <View style={s.line}>
        <TextInput accessibilityLabel={t("personalRecovery.m065")} value={filters.search} onChangeText={(search) => setFilters({ ...filters, search })}
          placeholder={t("personalRecovery.m066")} placeholderTextColor={theme.colors.foregroundMuted} style={s.search} />
        <IconAction label={t("personalRecovery.m067")} icon="Info" onPress={() => setPromptOpen(true)} styles={s} />
      </View>
      {scanError && <Text accessibilityRole="alert" style={s.danger}>{t("personalRecovery.m068")}</Text>}
      {!scanError && stale && data && <Text style={s.warning}>{t("personalRecovery.m069")}</Text>}
    </View>
    {query.isPending ? <Loading /> : tab === "records" ? <RecordGroups records={records} onOpen={openAgent} styles={s} />
      : <IncidentGroups items={visible} cycles={cycles} attempts={attempts} selected={selected} disabled={disabled} autoEnabled={!!policy?.enabled} now={Date.now()}
        onToggle={(id) => setSelected((values) => toggle(values, id))} onContinue={submit} onManage={(cycleId, action) => manageMutation.mutate({ cycleId, action })} onOpen={openAgent} styles={s} />}
    <View style={s.footer}>
      {!!feedback && <Text accessibilityLiveRegion="polite" style={s.muted}>{feedback}</Text>}
      <View style={s.header}><Status icon={scanError ? "WifiOff" : "CircleCheck"} tone={scanError ? "danger" : "muted"}
        label={data ? t("personalRecovery.m070", { p0: data.scannedAgents, p1: data.scannedAt }) : t("personalRecovery.m071")} value={data ? `${data.scannedAgents} · ${shortTime(data.scannedAt)}` : "…"} styles={s} />
        <Text style={s.muted}>{policy?.scanIntervalSeconds ?? 60}s · {scopeCount ? t("personalRecovery.m072", { p0: scopeCount }) : t("personalRecovery.m073")}</Text></View>
    </View>
    <Sheet title={t("personalRecovery.m074")} open={filterOpen} onClose={() => setFilterOpen(false)}><View style={s.modal}>
      <ScopePicker title={t("personalRecovery.m075")} options={options.projects} values={filters.projects} onChange={(projects) => { setFilters({ ...filters, projects }); setSelected([]); }} styles={s} />
      <ScopePicker title="Workspace" options={options.workspaces} values={filters.workspaces} onChange={(workspaces) => { setFilters({ ...filters, workspaces }); setSelected([]); }} styles={s} />
      <Text style={s.rowTitle}>{t("personalRecovery.m076")}</Text><View style={s.actions}>{(Object.keys(categoryLabels) as Category[]).map((category) => <IconAction key={category}
        label={categoryLabels[category]} icon={categoryIcons[category]} value={incidents.filter((i) => i.category === category).length} selected={filters.categories.includes(category)}
        onPress={() => { setFilters({ ...filters, categories: toggle(filters.categories, category) }); setSelected([]); }} styles={s} />)}</View>
      <IconAction label={t("personalRecovery.m077")} icon="Check" value={t("personalRecovery.m078")} primary onPress={() => setFilterOpen(false)} styles={s} />
    </View></Sheet>
    {settingsOpen && policy && <AutomationSettings policy={policy} options={options} saving={policyMutation.isPending}
      error={policyMutation.error instanceof Error ? policyMutation.error.message : ""} onSave={(draft) => policyMutation.mutate(draft)} onClose={() => setSettingsOpen(false)} styles={s} />}
    <Sheet title={t("personalRecovery.m079")} open={promptOpen} onClose={() => setPromptOpen(false)}><Text selectable style={s.text}>{CONTINUE_PROMPT}</Text></Sheet>
  </View>;
}
