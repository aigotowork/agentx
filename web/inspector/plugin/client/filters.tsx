import { t } from "../shared/text";
import { useState } from "react";
import { Text, View } from "react-native";
import type { PluginTheme } from "../../host/contracts";
import { AGENT_STATUSES, type AgentStatus } from "../shared/model";
import type { SavedView } from "../shared/preferences";
import { rangeLabels, type TimeRange } from "../shared/time-range";
import { ChoiceGroup, Button, Chip, Divider, IconButton, MetaLine, ProjectMark, Sheet, TextField, fontSize, hue, space, type Tone } from "../../uiux/systems/inspector/inspector.native";
import { Icon } from "../../host/ui";
import { statusNames } from "./session-ui";

export type SessionScope = "all" | "current" | "history";
/** @aiPurpose Provide sessionScopeNames presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const sessionScopeNames: Record<SessionScope, string> = { get all() { return t("coreFilters.allSessions"); }, get current() { return t("coreFilters.currentSessions"); }, get history() { return t("coreFilters.pastSessions"); } };
/** @aiPurpose Provide readFilterNames presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const readFilterNames = { get all() { return t("coreFilters.allResults"); }, get unread() { return t("coreFilters.completedAndUnread"); }, get read() { return t("coreFilters.completedAndRead"); } } as const;

/** Status chips carry the same colours as the list: running blue, waiting amber, error red, done green. */
const statusTones: Partial<Record<AgentStatus, Tone>> = { running: "info", initializing: "info", waiting: "warning", error: "danger", completed: "success" };

export interface FilterState {
  status: AgentStatus | "all"; readFilter: "all" | "unread" | "read"; sessionScope: SessionScope;
  checkout: string; workspace: string; provider: string;
}
/** Everything that is not a quick scope: status, read state, checkout, workspace, provider, and named views. */
/** @aiPurpose Compose FilterSheet presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function FilterSheet({ theme, open, onClose, archiveMode, state, checkouts, workspaces, providers, showWorkspace, onChange, onReset, savedViews, canSave, onSaveView, onApplyView, onDeleteView }: {
  theme: PluginTheme; open: boolean; onClose: () => void; archiveMode: boolean; state: FilterState;
  checkouts: readonly (readonly [string, string])[]; workspaces: readonly (readonly [string, string])[]; providers: readonly string[]; showWorkspace: boolean;
  onChange: (patch: Partial<FilterState>) => void; onReset: () => void;
  savedViews: readonly SavedView[]; canSave: boolean; onSaveView: (name: string) => Promise<void>; onApplyView: (view: SavedView) => void; onDeleteView: (view: SavedView) => void;
}) {
  const [name, setName] = useState("");
  const [saving, setSaving] = useState(false);
  const save = () => { const trimmed = name.trim(); if (!trimmed || saving) return; setSaving(true); void onSaveView(trimmed).then(() => setName("")).catch(() => {}).finally(() => setSaving(false)); };
  return <Sheet title={t("coreFilters.filters")} open={open} onClose={onClose}>
    {!archiveMode ? <ChoiceGroup layout="wrap" theme={theme} label={t("coreFilters.sessions")} options={(["all", "current", "history"] as const).map(value => ({ value, label: sessionScopeNames[value] }))} value={state.sessionScope} onChange={sessionScope => onChange({ sessionScope, readFilter: "all" })} /> : null}
    <ChoiceGroup layout="wrap" theme={theme} label={t("coreFilters.status")} options={[{ value: "all", label: t("coreFilters.allStatuses") }, ...AGENT_STATUSES.map(value => ({ value, label: statusNames[value], tone: statusTones[value] }))]} value={state.status} onChange={status => onChange({ status })} />
    {!archiveMode ? <ChoiceGroup layout="wrap" theme={theme} label={t("coreFilters.results")} options={(["all", "unread", "read"] as const).map(value => ({ value, label: readFilterNames[value], tone: value === "unread" ? "success" as const : undefined }))} value={state.readFilter} onChange={readFilter => onChange({ readFilter, status: "all", sessionScope: "all" })} /> : null}
    <ChoiceGroup layout="wrap" theme={theme} label={t("coreFilters.worktreeCheckout")} options={[{ value: "all", label: t("coreFilters.allCheckouts") }, ...checkouts.map(([value, label]) => ({ value, label }))]} value={state.checkout} onChange={checkout => onChange({ checkout })} />
    {showWorkspace ? <ChoiceGroup layout="wrap" theme={theme} label={t("coreFilters.workspace")} options={[{ value: "all", label: t("coreFilters.allWorkspaces") }, ...workspaces.map(([value, label]) => ({ value, label }))]} value={state.workspace} onChange={workspace => onChange({ workspace, checkout: "all" })} /> : null}
    <ChoiceGroup layout="wrap" theme={theme} label={t("coreFilters.provider")} options={[{ value: "all", label: t("coreFilters.allProviders") }, ...providers.map(value => ({ value, label: value }))]} value={state.provider} onChange={provider => onChange({ provider })} />
    <Divider theme={theme} />
    <View style={{ gap: space.sm }}>
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: fontSize.meta, fontWeight: "500" }}>{t("coreFilters.savedViews")}</Text>
      {savedViews.map(view => <View key={view.id} style={{ flexDirection: "row", alignItems: "center", gap: space.sm }}>
        <View style={{ flex: 1, minWidth: 0, alignItems: "flex-start" }}><Chip theme={theme} label={view.name} icon="Bookmark" onPress={() => { onApplyView(view); onClose(); }} /></View>
        <IconButton theme={theme} icon="Trash2" label={t("coreFilters.deleteView", { value1: view.name })} size="sm" onPress={() => onDeleteView(view)} />
      </View>)}
      <View style={{ flexDirection: "row", alignItems: "flex-end", gap: space.sm }}>
        <View style={{ flex: 1 }}><TextField theme={theme} label={t("coreFilters.viewName")} value={name} onChangeText={setName} placeholder={t("coreFilters.forExampleUnreadPaymentProjectResults")} /></View>
        <Button theme={theme} label={t("coreFilters.saveView")} icon="Bookmark" onPress={save} disabled={!name.trim() || !canSave} busy={saving} />
      </View>
    </View>
    <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: space.sm, paddingTop: space.xs }}>
      <Button theme={theme} label={t("coreFilters.reset")} variant="ghost" onPress={onReset} />
      <Button theme={theme} label={t("coreFilters.done")} variant="primary" onPress={onClose} />
    </View>
  </Sheet>;
}

/** @aiPurpose Compose ProjectSheet presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ProjectSheet({ theme, open, onClose, projects, value, total, onChange }: {
  theme: PluginTheme; open: boolean; onClose: () => void; projects: readonly { id: string; name: string; count: number }[]; value: string; total: number; onChange: (id: string) => void;
}) {
  return <Sheet title={t("coreFilters.projects")} open={open} onClose={onClose}>
    <ChoiceGroup theme={theme} value={value} onChange={id => { onChange(id); onClose(); }} options={[{ value: "all", label: t("coreFilters.allProjects"), count: total, leading: <Icon name="Layers" size={18} color={hue(theme, "blue")} /> },
      ...projects.map(project => ({ value: project.id, label: project.name, count: project.count, leading: <ProjectMark theme={theme} name={project.name} size={20} /> }))]} />
  </Sheet>;
}

/** @aiPurpose Provide rangeOptions presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const rangeOptions: readonly TimeRange[] = ["24h", "7d", "30d", "all"];
/** @aiPurpose Compose RangeSheet presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function RangeSheet({ theme, open, onClose, value, onChange }: { theme: PluginTheme; open: boolean; onClose: () => void; value: TimeRange; onChange: (range: TimeRange) => void }) {
  return <Sheet title={t("coreFilters.timeRange")} open={open} onClose={onClose}>
    <ChoiceGroup theme={theme} value={value} onChange={range => { onChange(range); onClose(); }} options={rangeOptions.map(range => ({ value: range, label: rangeLabels[range] }))} />
    <MetaLine theme={theme} numberOfLines={2} parts={[t("coreFilters.runningSessionsPendingApprovalsAndUnreadResults")]} />
  </Sheet>;
}
