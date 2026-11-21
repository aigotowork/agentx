import { t } from "../shared/text";
/** @aiPurpose Render dense grouped agents and audit rows, disclosing detailed errors and recovery management locally. */
import { useState } from "react";
import { Text, View } from "react-native";
import { FlatList, Icon } from "../../host/ui";
import type { RecoveryCycle } from "../shared/automation";
import { categoryLabels, phaseLabels, type Attempt, type Incident } from "../shared/recovery";
import { Check, IconAction, Status, shortTime, stamp, toggle } from "./recovery-controls";
import { categoryIcons, deliveryText, groupRows, rowStatus, type RecordRow, type GroupRow } from "./recovery-model";
import type { RecoveryStyles } from "./recovery.styles";

function GroupHeader<T>({ row, collapsed, onToggle, onContinue, styles: s }: {
  row: Exclude<GroupRow<T>, { kind: "item" }>; collapsed: boolean; onToggle(): void; onContinue?: () => void; styles: RecoveryStyles;
}) {
  return <View style={row.kind === "project" ? s.project : s.workspace}><View style={s.header}>
    <View style={[s.line, s.grow]}>
      <IconAction icon={collapsed ? "ChevronRight" : "ChevronDown"} label={`${collapsed ? t("personalRecovery.m002") : t("personalRecovery.m003")}${row.name}`} onPress={onToggle} styles={s} />
      <Icon name={row.kind === "project" ? "Folder" : "Layers"} color={s.muted.color} size={14} />
      <Text numberOfLines={1} style={row.kind === "project" ? s.rowTitle : s.muted}>{row.name}</Text><Text style={s.muted}>{row.count}</Text>
    </View>
    {onContinue && <IconAction icon="Play" label={t("personalRecovery.m004", { p0: row.name })} onPress={onContinue} styles={s} />}
  </View></View>;
}

function IncidentRow({ item, cycle, attempts, selected, disabled, autoEnabled, now, onToggle, onContinue, onManage, onOpen, styles: s }: {
  item: Incident; cycle?: RecoveryCycle; attempts: Attempt[]; selected: boolean; disabled: boolean; autoEnabled: boolean; now: number;
  onToggle(): void; onContinue(): void; onManage(action: "pause" | "resume" | "stop"): void; onOpen?: () => void; styles: RecoveryStyles;
}) {
  const [expanded, setExpanded] = useState(false);
  const eligible = item.state === "detected" && !item.blockedReason;
  const status = rowStatus(item, cycle, attempts, now, autoEnabled);
  const showCycle = cycle?.incidentId === item.id ? cycle : undefined;
  const latestAttempt = attempts.findLast(a => a.incidentId === item.id && a.phase !== "skipped");
  const reevaluate = latestAttempt?.phase === "not_sent";
  return <View style={s.row}>
    <View style={s.rowTop}>
      <Check label={t("personalRecovery.m005", { p0: item.title })} checked={selected} disabled={disabled || !eligible} onPress={onToggle} styles={s} />
      <View style={s.grow}><Text numberOfLines={1} style={s.rowTitle}>{item.title}</Text>
        <Text style={s.muted}>{item.provider} · {shortTime(item.observedAt)}</Text></View>
      <Status icon={status.icon} label={status.label} value={status.text} tone={status.tone} styles={s} />
      <IconAction icon={expanded ? "ChevronUp" : "ChevronDown"} label={t("personalRecovery.m006", { p0: expanded ? t("personalRecovery.m007") : t("personalRecovery.m002"), p1: item.title })} onPress={() => setExpanded(!expanded)} styles={s} />
    </View>
    <View style={s.header}>
      <View style={[s.line, s.grow]}>
        <Icon name={categoryIcons[item.category]} size={14} color={s.warning.color} />
        <Text numberOfLines={1} style={[s.muted, s.grow]}>{item.message}</Text>
      </View>
      <View style={s.actions}>
        {showCycle && <Text accessibilityLabel={t("personalRecovery.m008", { p0: showCycle.attemptCount })} style={s.muted}>#{showCycle.attemptCount}</Text>}
        {showCycle?.status === "waiting" && <IconAction icon="Pause" label={t("personalRecovery.m009", { p0: item.title })} disabled={disabled} onPress={() => onManage("pause")} styles={s} />}
        {showCycle?.status === "paused" && <IconAction icon="TimerReset" label={t("personalRecovery.m010", { p0: item.title })} disabled={disabled} onPress={() => onManage("resume")} styles={s} />}
        {onOpen && <IconAction icon="ArrowUpRight" label={t("personalRecovery.m011", { p0: item.title })} onPress={onOpen} styles={s} />}
        {item.state === "detected" && <IconAction icon="Play" label={reevaluate ? t("recoveryDelivery.reevaluate", { title: item.title }) : t("personalRecovery.m012", { p0: item.title })} disabled={disabled || !eligible} onPress={onContinue} styles={s} />}
      </View>
    </View>
    {expanded && <View style={s.details}>
      <View style={s.header}><Text style={s.warning}>{categoryLabels[item.category]}</Text><Text style={s.muted}>{stamp(item.observedAt)}</Text></View>
      <Text selectable style={s.text}>{item.message}</Text>
      {item.blockedReason && <Text style={s.warning}>{item.blockedReason}</Text>}
      {reevaluate && <Text style={s.warning}>{t("recoveryDelivery.manualReview")}</Text>}
      {latestAttempt?.phase === "unknown" && <Text style={s.warning}>{t("recoveryDelivery.checkConversation")}</Text>}
      {showCycle && <View style={s.header}>
        <View style={s.grow}><Text style={s.text}>{showCycle.reason}</Text>
          {showCycle.nextAt && <Text style={s.muted}>{t("personalRecovery.m013")}{stamp(showCycle.nextAt)}</Text>}</View>
        {!["stopped", "recovered"].includes(showCycle.status) && <IconAction icon="CircleStop" label={t("personalRecovery.m014", { p0: item.title })} disabled={disabled} onPress={() => onManage("stop")} styles={s} />}
      </View>}
      <Text selectable style={s.muted}>{item.source === "hook" ? t("personalRecovery.m015") : t("personalRecovery.m016")} · {item.agentId}{item.code ? ` · ${item.code}` : ""}</Text>
      {attempts.slice(-20).reverse().map((attempt) => <View key={attempt.id} style={s.detailLine}>
        <Icon name={attempt.trigger === "automatic" ? "Zap" : "Hand"} size={13} color={s.muted.color} />
        <Text style={s.muted}>{shortTime(attempt.updatedAt)}</Text>
        <View style={s.grow}><Text style={attempt.phase === "unknown" ? s.danger : attempt.phase === "not_sent" ? s.warning : s.text}>{phaseLabels[attempt.phase]}</Text>
          <Text style={s.muted}>{attempt.reason}</Text><Text style={s.muted}>{deliveryText(attempt)}</Text>
          {attempt.delivery?.status === "unknown" && attempt.delivery.relatedMessageId && <Text selectable style={s.muted}>{t("recoveryDelivery.relatedMessage", { id: attempt.delivery.relatedMessageId })}</Text>}
        </View>
      </View>)}
    </View>}
  </View>;
}

/** @aiPurpose Group visible agents by project and Workspace with bounded row disclosure. @aiUsage Pass server-authoritative eligibility and callbacks; filtering remains in the screen. */
/** @aiPurpose Compose IncidentGroups presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function IncidentGroups({ items, cycles, attempts, selected, disabled, autoEnabled, now, onToggle, onContinue, onManage, onOpen, styles: s }: {
  items: Incident[]; cycles: RecoveryCycle[]; attempts: Attempt[]; selected: string[]; disabled: boolean; autoEnabled: boolean; now: number;
  onToggle(id: string): void; onContinue(items: Incident[]): void; onManage(id: string, action: "pause" | "resume" | "stop"): void;
  onOpen?: (id: string) => void; styles: RecoveryStyles;
}) {
  const [collapsed, setCollapsed] = useState<string[]>([]);
  const rows = groupRows(items, collapsed), cycleMap = new Map(cycles.map((c) => [c.agentId, c]));
  return <FlatList data={rows} keyExtractor={(row) => row.key} contentContainerStyle={s.list}
    renderItem={({ item: row }) => row.kind !== "item" ? <GroupHeader row={row} collapsed={collapsed.includes(row.key)} onToggle={() => setCollapsed(toggle(collapsed, row.key))}
      onContinue={!disabled && row.items.some((i) => i.state === "detected" && !i.blockedReason) ? () => onContinue(row.items.filter((i) => i.state === "detected" && !i.blockedReason)) : undefined} styles={s} />
      : <IncidentRow item={row.item} cycle={cycleMap.get(row.item.agentId)} attempts={attempts.filter((a) => a.agentId === row.item.agentId)}
        selected={selected.includes(row.item.id)} disabled={disabled} autoEnabled={autoEnabled} now={now} onToggle={() => onToggle(row.item.id)}
        onContinue={() => onContinue([row.item])} onManage={(action) => { const cycle = cycleMap.get(row.item.agentId); if (cycle) onManage(cycle.id, action); }}
        onOpen={onOpen ? () => onOpen(row.item.agentId) : undefined} styles={s} />}
    ListEmptyComponent={<View style={s.empty}><Icon name="CircleCheck" size={22} color={s.success.color} /><Text style={s.muted}>{t("personalRecovery.m017")}</Text></View>} />;
}

function AuditRow({ item, onOpen, styles: s }: { item: RecordRow; onOpen?: () => void; styles: RecoveryStyles }) {
  const [expanded, setExpanded] = useState(false);
  return <View style={s.row}>
    <View style={s.rowTop}>
      <Icon name={item.icon} size={14} color={s[item.tone].color} /><View style={s.grow}>
        <Text style={s.rowTitle}>{item.title}</Text><Text numberOfLines={expanded ? undefined : 1} style={s[item.tone]}>{item.message}</Text>
      </View>
      <Text style={s.muted}>{shortTime(item.at)}</Text>
      <IconAction label={t("personalRecovery.m018")} icon={expanded ? "ChevronUp" : "ChevronDown"} onPress={() => setExpanded(!expanded)} styles={s} />
      {onOpen && <IconAction label={t("personalRecovery.m019")} icon="ArrowUpRight" onPress={onOpen} styles={s} />}
    </View>
    {expanded && <Text selectable style={s.muted}>{stamp(item.at)}{item.agentId ? ` · ${item.agentId}` : ""}</Text>}
  </View>;
}
/** @aiPurpose Present chronological audit facts under project and Workspace groups. @aiUsage Records contain actual attempts and automation transitions, not inferred task completion. */
/** @aiPurpose Compose RecordGroups presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function RecordGroups({ records, onOpen, styles: s }: { records: RecordRow[]; onOpen?: (id: string) => void; styles: RecoveryStyles }) {
  const [collapsed, setCollapsed] = useState<string[]>([]), rows = groupRows(records, collapsed);
  return <FlatList data={rows} keyExtractor={(row) => row.key} contentContainerStyle={s.list}
    renderItem={({ item: row }) => row.kind !== "item" ? <GroupHeader row={row} collapsed={collapsed.includes(row.key)} onToggle={() => setCollapsed(toggle(collapsed, row.key))} styles={s} />
      : <AuditRow item={row.item} onOpen={onOpen && row.item.agentId ? () => onOpen(row.item.agentId!) : undefined} styles={s} />}
    ListEmptyComponent={<View style={s.empty}><Text style={s.muted}>{t("personalRecovery.m020")}</Text></View>} />;
}
