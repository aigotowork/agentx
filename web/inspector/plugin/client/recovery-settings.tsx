import { t } from "../shared/text";
/** @aiPurpose Edit daemon automation policy separately from temporary screen filters. */
import { useState } from "react";
import { Text, View } from "react-native";
import { ScrollView, SettingsSelect, SettingsSwitch } from "../../host/ui";
import type { Policy } from "../shared/automation";
import type { Incident } from "../shared/recovery";
import { Check, IconAction, toggle } from "./recovery-controls";
import type { RecoveryStyles } from "./recovery.styles";
import { Sheet } from "../../uiux/systems/inspector/inspector.native";

export interface ScopeOption { id: string; name: string }
/** @aiPurpose Render a bounded multi-select scope list. @aiUsage Empty selection means all; include selected IDs missing from the current catalog. */
/** @aiPurpose Compose ScopePicker presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ScopePicker({ title, options, values, onChange, disabled, styles: s }: {
  title: string; options: ScopeOption[]; values: string[]; onChange(values: string[]): void; disabled?: boolean; styles: RecoveryStyles;
}) {
  const known = new Set(options.map((o) => o.id));
  const all = [...options, ...values.filter((id) => !known.has(id)).map((id) => ({ id, name: id }))];
  return <View style={s.stack}>
    <View style={s.header}><Text style={s.rowTitle}>{title} · {values.length || t("personalRecovery.m080")}</Text>
      {!!values.length && <IconAction label={t("personalRecovery.m081", { p0: title })} icon="X" onPress={() => onChange([])} disabled={disabled} styles={s} />}</View>
    <ScrollView style={s.filterList}>{all.map((option) => <View key={option.id} style={s.filterRow}>
      <Check label={option.name} checked={values.includes(option.id)} disabled={disabled} onPress={() => onChange(toggle(values, option.id))} styles={s} />
      <Text style={[s.text, s.grow]}>{option.name}</Text>
    </View>)}</ScrollView>
  </View>;
}
/** @aiPurpose Provide scopeOptions presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function scopeOptions(incidents: Incident[]) {
  return {
    projects: [...new Map(incidents.map((i) => [i.projectId ?? "ungrouped", { id: i.projectId ?? "ungrouped", name: i.projectName }])).values()],
    workspaces: [...new Map(incidents.map((i) => [i.workspaceId ?? "ungrouped", { id: i.workspaceId ?? "ungrouped", name: `${i.projectName} / ${i.workspaceName}` }])).values()],
  };
}
/** @aiPurpose Edit a revision-checked automatic recovery policy. @aiUsage Mount afresh when opened; report rejected saves without silently replacing the draft. */
/** @aiPurpose Compose AutomationSettings presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function AutomationSettings({ policy, options, saving, error, onSave, onClose, styles: s }: {
  policy: Policy; options: { projects: ScopeOption[]; workspaces: ScopeOption[] }; saving: boolean; error: string;
  onSave(policy: Policy): void; onClose(): void; styles: RecoveryStyles;
}) {
  const [draft, setDraft] = useState(policy);
  return <Sheet title={t("personalRecovery.m082")} open onClose={() => { if (!saving) onClose(); }}>
    <View style={s.modal}>
      <SettingsSwitch label={t("personalRecovery.m082")} value={draft.enabled} disabled={saving} onValueChange={(enabled) => setDraft({ ...draft, enabled })} />
      <SettingsSelect label={t("personalRecovery.m083")} value={String(draft.scanIntervalSeconds)} disabled={saving}
        options={[30, 60, 120, 300].map((seconds) => ({ value: String(seconds), label: t("personalRecovery.m084", { p0: seconds }) }))}
        onValueChange={(value) => setDraft({ ...draft, scanIntervalSeconds: Number(value) })} />
      <Text style={s.muted}>{t("personalRecovery.m085")}</Text>
      <Text style={s.rowTitle}>{t("personalRecovery.m086")}</Text>
      <Text style={s.muted}>{t("personalRecovery.m087")}</Text>
      <ScopePicker title={t("personalRecovery.m075")} options={options.projects} values={draft.projectIds} disabled={saving} onChange={(projectIds) => setDraft({ ...draft, projectIds })} styles={s} />
      <ScopePicker title="Workspace" options={options.workspaces} values={draft.workspaceIds} disabled={saving} onChange={(workspaceIds) => setDraft({ ...draft, workspaceIds })} styles={s} />
      {!!error && <Text accessibilityRole="alert" style={s.danger}>{error}</Text>}
      <View style={s.actions}><IconAction label={t("personalRecovery.m088")} icon="Check" value={t("personalRecovery.m089")} primary disabled={saving} onPress={() => onSave(draft)} styles={s} />
        <IconAction label={t("personalRecovery.m090")} icon="X" disabled={saving} onPress={onClose} styles={s} /></View>
    </View>
  </Sheet>;
}
