import { messageSourceLabel } from "./message-state";
import { t } from "../shared/text";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Text, View } from "react-native";
import type { PluginTheme } from "../../host/contracts";
import { useRpc } from "../../host/adapter";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { messageBadge, isOnItsWay, type MessageSendResult, type MessageView } from "../shared/messages-rpc";
import { observatoryMessages } from "../shared/steer";
import { messagePollInterval, visibleMessages } from "./message-state";
import { Badge, aria, fontSize, hairline, radius, relativeTime, space, toneColor, withAlpha } from "../../uiux/systems/inspector/inspector.native";
import { Breathe, Flash, useChangeCount } from "../../uiux/systems/inspector/inspector.native";
import type { Consent } from "./send-plan";
import type { LedgerInput } from "./send-check";
import { useForeground } from "../../uiux/systems/inspector/inspector.native";

/** @aiPurpose Provide messagesQueryKey presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const messagesQueryKey = (agentId: string) => ["agent-observatory", "messages", agentId] as const;

/** @aiPurpose Provide useMessages presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useMessages(agentId: string, enabled = true) {
  const list = useRpc(observatoryMessages.list), sendRpc = useRpc(observatoryMessages.send);
  const queryClient = useQueryClient(), foreground = useForeground();
  const key = useMemo(() => messagesQueryKey(agentId), [agentId]);
  const query = useQuery({ queryKey: key, queryFn: () => list({ agentId }), enabled, retry: false, staleTime: 2_000, refetchOnWindowFocus: false,
    refetchInterval: current => messagePollInterval(current.state.data, foreground) });
  const put = useCallback((view: MessageView | null) => {
    if (view) queryClient.setQueryData<MessageView[]>(key, (previous = []) => [view, ...previous.filter(entry => entry.messageId !== view.messageId)]);
    void queryClient.invalidateQueries({ queryKey: key });
  }, [key, queryClient]);
  // An outcome disappears on its own after a while: draw again when the next one is due.
  const [, setTick] = useState(0);
  const shown = visibleMessages(query.data ?? []);
  useEffect(() => {
    if (shown.until === null) return;
    const timer = setTimeout(() => setTick(value => value + 1), Math.max(0, shown.until - Date.now()) + 50);
    return () => clearTimeout(timer);
  }, [shown.until]);
  const send = useCallback(async (input: LedgerInput & { what: string; consent: Consent | null }): Promise<MessageSendResult> => {
    try { const result = await sendRpc({ agentId, ...input }); put("view" in result ? result.view : null); return result; }
    catch (error) { put(null); throw error; }
  }, [agentId, put, sendRpc]);
  return {
    /** What the entries draw: messages on their way and fresh outcomes, newest first. */
    views: shown.views,
    /** Everything the server listed for this Agent (the last half hour), for 在途 checks and receipts. */
    all: query.data ?? [],
    send,
  };
}
export type MessageLedger = ReturnType<typeof useMessages>;

const ICONS: Record<MessageView["state"], string> = { queued: "Hourglass", held: "CircleHelp", sending: "Send", sent: "CircleCheck", "maybe-sent": "CircleAlert", canceled: "Ban", discarded: "CircleX", expired: "TimerOff" };

/** @aiPurpose Compose MessageList presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function MessageList({ theme, queue, only, compact = false }: { theme: PluginTheme; queue: MessageLedger; only?: (view: MessageView) => boolean; compact?: boolean }) {
  const rows = only ? queue.views.filter(only) : queue.views;
  if (!rows.length) return null;
  return <View testID="message-receipts" style={{ gap: space.xs + 2 }}>
    {rows.map(view => <MessageRow key={view.messageId} theme={theme} view={view} compact={compact}
 />)}
  </View>;
}

/** @aiPurpose Compose MessageRow presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function MessageRow({ theme, view, compact, agentTitle }: { theme: PluginTheme; view: MessageView; compact: boolean; agentTitle?: string }) {
  const { colors } = theme;
  const badge = messageBadge(view), changed = useChangeCount(view.state), waiting = isOnItsWay(view);
  const mark = <Badge theme={theme} label={badge.label} tone={badge.tone} icon={ICONS[view.state]} />;
  const source = messageSourceLabel(view.source) || view.preview, name = agentTitle ? `${agentTitle} · ${source}` : source;
  // One being sent right now cannot be taken back: it has no buttons.
  // 已送达 says it all; a note is worth a line when it explains or asks.
  const note = view.note && view.note !== badge.label ? view.note : null;
  return <View accessibilityLabel={t("coreDeferred.message", { value1: badge.label, value2: name })} {...aria({ live: "polite" })}
    style={{ overflow: "hidden", gap: space.xs + 2, padding: compact ? space.sm : space.sm + 2, borderRadius: radius.md, borderWidth: hairline, borderColor: toneColor(theme, badge.tone, "line"), backgroundColor: toneColor(theme, badge.tone, "soft") }}>
    <Flash count={changed} color={withAlpha(toneColor(theme, badge.tone, "mark"), 0.35)} radius={radius.md} />
    <View style={{ flexDirection: "row", alignItems: "center", flexWrap: "wrap", gap: space.sm }}>
      {view.state === "sending" ? <Breathe>{mark}</Breathe> : mark}
      <Text numberOfLines={1} style={{ flexShrink: 1, color: colors.foreground, fontSize: fontSize.small, fontWeight: "500" }}>{name}</Text>
      <Text style={{ flexShrink: 0, color: colors.foregroundMuted, fontSize: fontSize.meta }}>{relativeTime(waiting ? view.createdAt : view.at)}</Text>
    </View>

    {note ? <Text style={{ color: colors.foreground, fontSize: fontSize.meta, lineHeight: 17 }}>{note}</Text> : null}
  </View>;
}
