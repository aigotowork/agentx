import { useState } from "react";
import { HandoverSheet, handoverOf } from "./handover-sheet";
import { t } from "../shared/text";
import type { ReactNode } from "react";
import { Text, View } from "react-native";
import type { PluginTheme } from "../../host/contracts";
import { Icon } from "../../host/ui";
import type { AgentRecord } from "../shared/model";
import { clockLabel } from "../shared/attention";
import { Button, MetaLine, Spacer, fontSize, hairline, radius, space } from "../../uiux/systems/inspector/inspector.native";
import { PhasePill, hueColor, tint, tintAlpha, type Hue } from "./status";
import { Breathe } from "../../uiux/systems/inspector/inspector.native";
import { useClock } from "./use-clock";

/** @aiPurpose Show why a turn stopped, including model quota errors, without scheduled continuation.
 * @aiUsage Mount beside an Agent result; opening the conversation is an explicit local navigation. */
export function StopCard({ theme, agent, onOpenConversation, onHandedOver }: { theme: PluginTheme; agent: AgentRecord; onOpenConversation?: () => void; onHandedOver?: (id: string) => void }) {
  const [handing, setHanding] = useState(false);
  const now = useClock(30_000), stop = agent.stop;
  if (!stop || agent.archivedAt || agent.status === "running" || agent.status === "initializing") return null;
  const silent = stop.kind === "silent", limited = stop.kind === "quota", hue: Hue = limited ? "orange" : "amber";
  const title = limited ? t("secondaryStopCard.quotaExhausted") : t(silent ? "secondaryStopCard.emptyTurn" : "secondaryStopCard.possiblyUnfinished");
  const text = limited ? t("secondaryStopCard.aQuotaOrRateLimitStopped3") : t(silent ? "secondaryStopCard.thisTurnEndedWithoutTextTool" : agent.stop?.closedMidTurn ? "secondaryStopCard.thisTurnWasStillRunningWhen" : "secondaryStopCard.thisTurnStoppedAfterATool");
  const handed = handoverOf(agent.id, stop.at);
  return <><Card theme={theme} hue={hue} icon={limited ? "BatteryWarning" : "CircleDashed"} title={title} text={text}
    pill={<PhasePill theme={theme} hue={hue} label={clockLabel(stop.at, now)} />}
    actions={<>{limited ? <Button theme={theme} size="sm" tone="violet" icon="ArrowRightLeft" label={t("secondaryStopCard.handOverToAnotherProvider")} onPress={() => setHanding(true)} /> : null}{handed && onHandedOver ? <Button theme={theme} size="sm" icon="ArrowUpRight" label={handed.profile} onPress={() => onHandedOver(handed.agentId)} /> : null}{onOpenConversation ? <Button theme={theme} size="sm" tone={hue} icon="ArrowUpRight" label={t("secondaryStopCard.openSession")} onPress={onOpenConversation} /> : null}</>} /><HandoverSheet theme={theme} open={handing} onClose={() => setHanding(false)} agent={agent} stopAt={stop.at} onHandedOver={onHandedOver} /></>;
}

/** @aiPurpose Compose Card presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Card({ theme, hue, icon, title, pill, text, note, actions, pulse = false, children }: { theme: PluginTheme; hue: Hue; icon: string; title: string; pill?: ReactNode; text: string; note?: string | null;
  actions?: ReactNode; pulse?: boolean; children?: ReactNode }) {
  const color = hueColor(theme, hue);
  return <View testID="stop-card" style={{ gap: space.sm, padding: space.md, borderRadius: radius.lg, borderWidth: hairline, borderColor: tint(color, 0.5), backgroundColor: tint(color, tintAlpha(theme) * 0.7) }}>
    <View style={{ flexDirection: "row", alignItems: "center", gap: space.sm }}>
      <Breathe active={pulse}><Icon name={icon} size={16} color={color} /></Breathe>
      <Text style={{ color: hueColor(theme, hue, "text"), fontSize: fontSize.body, fontWeight: "800" }}>{title}</Text>
      <Spacer />
      {pill}
    </View>
    <Text style={{ color: theme.colors.foreground, fontSize: fontSize.small, lineHeight: 20 }}>{text}</Text>
    {children}
    {note ? <MetaLine theme={theme} numberOfLines={3} parts={[note]} /> : null}
    {actions ? <View style={{ flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: space.sm }}>{actions}</View> : null}
  </View>;
}

