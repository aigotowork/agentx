import { messageSourceLabel } from "./message-state";
import { TextParts } from "./core-text";
import { t } from "../shared/text";
import { useEffect, useRef, useState } from "react";
import { Text, View } from "react-native";
import type { PluginTheme } from "../../host/contracts";
import { useAgentX, useRpc } from "../../host/adapter";
import { Icon, useToast } from "../../host/ui";
import { useQueryClient } from "@tanstack/react-query";
import { QUICK_REPLIES, quickReplyText, type QuickReply, type ReplyNudge } from "../shared/reply";
import { aiReply } from "../shared/recap";
import { markCompletionRead } from "../shared/read-state";
import { getTelemetry } from "../shared/telemetry";
import { BUILTIN_SEND } from "../shared/steer";
import { isOnItsWay, type MessageView } from "../shared/messages-rpc";
import { Banner, Button, fontSize, hue, space } from "../../uiux/systems/inspector/inspector.native";
import { Enter, Pop } from "../../uiux/systems/inspector/inspector.native";
import { useSteerableProviders } from "./insight-actions";
import { MessageList, useMessages } from "./messages";
import { ledgerNotice, messageRecord, onItsWayLabel, quickReplyGroup, quickReplyGuard, quickReplyKey, rememberUnanswered, requestId, settleRequest, unansweredId } from "./message-state";
import { answering, checkFits, clearingAttention, isSendRefused, ledgerInput, outgoingMessage, sameRound, sendChecked, type LedgerAnswer, type OutgoingMessage, type SendConfirmation } from "./send-check";
import type { Consent, SendPlan } from "./send-plan";
import { readReceiptQueryKey } from "./use-read-receipts";

/** A message that certainly never went out: the reply is free to be sent again, under a new id. */
const NEVER_SENT: ReadonlySet<MessageView["state"]> = new Set(["canceled", "discarded", "expired"]);
type Problem = { reply: QuickReply; message: string; tone: "danger" | "warning"; retry: Consent | null | false };

// Catalog migration preserves the original question distinction: nudge === "go-ahead" ? "它问：" : "它说接下来："
/** @aiPurpose Compose QuickReplies presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function QuickReplies({ theme, agentId, agentTitle, nudge, suggestion, expectPrompt, expectRound, ask, compact = false, onSent }: {
  theme: PluginTheme; agentId: string; agentTitle: string; nudge: ReplyNudge | null;
  /** The recap's suggested next message for this turn (AI 建议), if any. */ suggestion?: string | null;
  /** The prompt of the turn they answer (its `lastUserMessageAt`). */ expectPrompt: string | null;
   expectRound?: string | null;
  /** The sentence they answer, quoted above them where the reply itself is not on screen (the popover, P2-1). */ ask?: string | null;
  compact?: boolean; onSent?: (reply: QuickReply) => void;
}) {
  const agentx = useAgentX(), toast = useToast(), queryClient = useQueryClient(), markRead = useRpc(markCompletionRead), telemetry = useRpc(getTelemetry), { colors } = theme;
  // Without the host's lists only the built-in providers steer; a custom one is asked about first.
  const capabilities = useSteerableProviders(true), caps = capabilities.data ?? (capabilities.isError ? BUILTIN_SEND : undefined);
  const deferred = useMessages(agentId);
  // The list as last drawn, for a tap on buttons drawn before it (the four plugins' r5-F1).
  const listed = useRef(deferred.all);
  listed.current = deferred.all;
  const rules = nudge ? QUICK_REPLIES[nudge] : [], ai = aiReply(suggestion, rules), replies = ai ? [...rules, ai] : rules;
  const keyOf = (reply: QuickReply) => quickReplyKey(quickReplyText(reply), expectPrompt, expectRound);
  const group = quickReplyGroup(expectPrompt, expectRound), guard = quickReplyGuard(expectPrompt, expectRound);
  const keys = new Set(replies.map(keyOf));
  const messages = useRef(new Map<string, OutgoingMessage>()), inFlight = useRef(false), mounted = useRef(true);
  useEffect(() => () => { mounted.current = false; }, []);
  const [busy, setBusy] = useState<string | null>(null), [sent, setSent] = useState<QuickReply | null>(null);
  const [confirming, setConfirming] = useState<{ reply: QuickReply; confirmation: SendConfirmation } | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null), [refused, setRefused] = useState<string | null>(null);

  const clearAttention = clearingAttention(input => markRead(input).then(() => queryClient.invalidateQueries({ queryKey: readReceiptQueryKey })), agentId, expectRound);
  const expect = answering(expectPrompt, agentTitle);
  // The turn start the plugin saw last, read at the tap: a later one than the round they answer is another round.
  const precheck = sameRound(async () => { const answer = await telemetry({ agentId }); return "agents" in answer ? answer.agents.find(item => item.agentId === agentId)?.turnStartedAt : null; }, expectRound, agentTitle);

  // What the ledger did with these replies (any screen or device): one on its way locks them, one it delivered settles them.
  const records = replies.map(reply => ({ reply, view: messageRecord(deferred.all, agentId, keyOf(reply)) }));
  const delivered = sent ?? records.find(record => record.view?.state === "sent")?.reply ?? null;
  const uncertain = records.find(record => record.view?.state === "maybe-sent") ?? null;
  const locked = records.some(record => !!record.view && isOnItsWay(record.view));

  // Settled or on its way, the confirmation shown before goes first; a lost try the list now shows is done with after it.
  useEffect(() => {
    for (const { view } of records) {
      if (!view || view.state === "canceled" || view.state === "discarded" || view.state === "expired") continue;
      setConfirming(null);
      if (view.state !== "sent" && view.state !== "maybe-sent") continue;
      setProblem(null);
      if (unansweredId(agentId, view.key) === view.messageId) settleRequest(agentId, view.key);
    }
  }, [deferred.all, agentId, expectPrompt, expectRound, nudge, suggestion]);
  /** Whether the list drawn last (not the one this tap's buttons were drawn with) shows these replies settled or on their way. */
  const taken = () => replies.some(reply => {
    const view = messageRecord(listed.current, agentId, keyOf(reply));
    return !!view && !NEVER_SENT.has(view.state);
  });

  // answer was lost is asked about again under its id.
  const messageFor = (reply: QuickReply) => {
    const key = keyOf(reply);
    let message = messages.current.get(key);
    const record = message && listed.current.find(view => view.messageId === message!.messageId);
    if (!message || record && NEVER_SENT.has(record.state)) {
      message = outgoingMessage(agentId, quickReplyText(reply), { what: t("coreQuickReplies.reply"), source: reply.ai ? t("recap.aiReplySource", { value1: reply.text }) : t("coreQuickReplies.quickReply", { value1: reply.text }), key, messageId: requestId(agentId, key, listed.current), group, guard });
      messages.current.set(key, message);
    }
    return message;
  };
  const lost = (reply: QuickReply, message: OutgoingMessage, error: unknown, retry: Consent | null) => {
    if (message.tried) rememberUnanswered(agentId, message.key, message.messageId);
    const reason = (error instanceof Error ? error.message : String(error)).replace(/[。.]$/, "");
    setProblem({ reply, tone: "danger", retry, message: message.tried
      ? t("coreQuickReplies.noSendResultReceivedForIfThe", { value1: reply.text, value2: reason })
      : t("coreQuickReplies.wasNotSent", { value1: reply.text, value2: reason }) });
  };
  /**
   * Another message answers this turn (codex-obs-attn-R3): a tap here, on another screen or device, of this button or another.
   * It is named for what it is, and this button is never shown as sent for it: the list shows the reply that went.
   */
  const answeredElsewhere = (view: MessageView) => {
    const other = replies.find(item => keyOf(item) === view.key), named = other ? t("coreQuickReplies.quotedReply", { reply: other.text }) : messageSourceLabel(view.source) || t("coreQuickReplies.anotherReply");
    toast.show(view.state === "sent" ? t("coreQuickReplies.thisTurnWasAlreadyAnsweredWithNothing", { value1: named }) : t("coreQuickReplies.theReplyForThisTurnSeeBelow", { value1: named, value2: onItsWayLabel(view) }), { variant: view.state === "maybe-sent" ? "warning" : "success" });
  };
  /** The ledger's answer for this reply's own message, by its state (codex-obs-send-ui-R2): only a queued one waits for the turn. */
  const answeredOwn = (reply: QuickReply, view: MessageView, sentWords: string) => {
    const notice = ledgerNotice(view, agentTitle, reply.text);
    if (notice.kind === "sent") {
      // The new turn may already have taken the reply's row away (the Agent is running): the toast still says it went.
      toast.show(sentWords, { variant: "success" });
      onSent?.(reply);
      if (mounted.current) setSent(reply);
    } else if (view.state === "discarded") {
      // Dropped by the server: the turn it answers has moved on (or the Agent is gone), as a refusal here says it.
      if (mounted.current) { setProblem(null); setRefused(notice.text); }
    } else toast.show(notice.text, { variant: notice.tone === "danger" ? "error" : notice.tone }); // held: its row below has the reason, 仍然发送 and 取消
  };
  const answered = (reply: QuickReply, message: OutgoingMessage, answer: LedgerAnswer, plan: SendPlan, consent: Consent | null) => {
    if ("message" in answer) { setProblem({ reply, tone: answer.status === "busy" ? "warning" : "danger", message: answer.message, retry: answer.status === "busy" ? consent : false }); return; }
    const { view } = answer;
    if (view.messageId !== message.messageId) { answeredElsewhere(view); return; }
    answeredOwn(reply, view, plan.kind === "steer" ? t("coreQuickReplies.joinedSCurrentTurn", { value1: agentTitle, value2: reply.text }) : t("coreQuickReplies.repliedTo", { value1: agentTitle, value2: reply.text }));
  };
  const send = async (reply: QuickReply, consent: Consent | null = null) => {
    if (!caps || inFlight.current || refused) return;
    if (taken()) { setConfirming(null); return; }
    inFlight.current = true;
    const message = messageFor(reply);
    setBusy(reply.key); setProblem(null);
    try {
      const result = await sendChecked(agentx, message, caps, deferred.send, consent, { expect, precheck, ...(reply.rearm ? { beforeSend: clearAttention } : {}) });
      if ("confirm" in result) { if (mounted.current) setConfirming({ reply, confirmation: result.confirm }); return; }
      settleRequest(agentId, message.key);
      if (mounted.current) setConfirming(null);
      answered(reply, message, result.answered, result.plan, consent);
    } catch (error) {
      if (!mounted.current) return;
      setConfirming(null);
      // The Agent moved on: these no longer answer anything, so nothing to retry (the next turn's reply brings its own).
      if (isSendRefused(error)) { setProblem(null); setRefused(error.message); return; }
      lost(reply, message, error, consent);
    } finally { inFlight.current = false; if (mounted.current) setBusy(null); }
  };


  
  if (delivered) return <Enter active><View testID="quick-reply-sent" style={{ flexDirection: "row", alignItems: "center", gap: space.sm, minHeight: 28 }}>
    <Pop count={1} size={16}><Icon name="CircleCheck" size={16} color={hue(theme, "green")} /></Pop>
    <Text style={{ flexShrink: 1, color: hue(theme, "green", "text"), fontSize: fontSize.small, fontWeight: "600" }}>{t("coreQuickReplies.sent", { value1: delivered.text })}</Text>
  </View></Enter>;
  if (refused) return <Enter active><View testID="quick-replies-stale"><Banner theme={theme} tone="info" icon="CircleSlash" message={refused} /></View></Enter>;
  const shown = confirming && !locked && !uncertain ? confirming : null, plan = shown?.confirmation.check.plan;
  return <View testID="quick-replies" style={{ gap: space.sm }}>
    {rules.length ? <View style={{ flexDirection: "row", alignItems: "center", gap: space.xs + 2 }}>
      <Icon name="Zap" size={13} color={hue(theme, "violet")} />
      <Text numberOfLines={2} style={{ flexShrink: 1, color: colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: 17 }}>{t("coreQuickReplies.quickRepliesTapToSendTheWords")}</Text>
    </View> : null}
    {/* What a tap answers, where the reply is not on screen: its closing question or the step it announced. */}
    {ask && nudge ? <View testID="quick-reply-ask" style={{ paddingLeft: space.sm + 2, borderLeftWidth: 3, borderLeftColor: hue(theme, "violet") }}>
      <Text numberOfLines={3} style={{ flexShrink: 1, color: colors.foreground, fontSize: fontSize.meta, lineHeight: 18 }}>
        <TextParts id="coreQuickReplies.questionContext" values={{ lead: <Text style={{ color: hue(theme, "violet", "text"), fontWeight: "700" }}>{nudge === "go-ahead" ? t("coreQuickReplies.itAsks") : t("coreQuickReplies.itSaysNext")}</Text>, question: ask }} />
      </Text>
    </View> : null}
    {shown && plan ? <View style={{ gap: space.sm }}>
      <Banner theme={theme} tone="warning" message={plan.confirm ?? plan.note ?? t("coreQuickReplies.theAgentSStateChangedConfirmBefore")} />
      
      <View style={{ flexDirection: "row", flexWrap: "wrap", gap: space.md }}>
        <Button theme={theme} variant="danger" label={plan.label} grow={compact} busy={busy === shown.reply.key} disabled={!!busy}
          accessibilityLabel={t("coreQuickReplies.sendTo", { value1: plan.label, value2: agentTitle, value3: shown.reply.text })} onPress={() => void send(shown.reply, shown.confirmation.consent)} />

        <Button theme={theme} variant="ghost" label={t("coreQuickReplies.doNotSend")} disabled={!!busy} onPress={() => setConfirming(null)} />
      </View>
    </View> : <View style={{ gap: space.sm }}>
      <View style={{ flexDirection: "row", flexWrap: "wrap", gap: compact ? space.md : space.sm }}>
        {rules.map(reply => <Button key={reply.key} theme={theme} label={reply.text} icon={reply.icon} tone={reply.tone} size={compact ? "md" : "sm"} grow={compact}
          busy={busy === reply.key} disabled={!caps || !!busy || locked || !!uncertain} accessibilityLabel={t("coreQuickReplies.sendTo2", { value1: agentTitle, value2: reply.text })} testID={`quick-reply-${reply.key}`}
          onPress={() => void send(reply)} />)}
        {}
        
      </View>
      {/* 建议回复: the recap's guess at the next message, marked as the model's, sent only on a tap. */}
      {ai ? <View testID="quick-reply-ai-block" style={{ gap: space.xs + 2 }}>
        <View style={{ flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: space.xs + 2 }}>
          <Text style={{ color: hue(theme, "fuchsia"), fontSize: 13, fontWeight: "800" }}>✦</Text>
          <Text style={{ color: hue(theme, "fuchsia", "text"), fontSize: fontSize.meta, fontWeight: "800" }}>{t("recap.aiSuggestion")}</Text>
          <Text numberOfLines={2} style={{ flexShrink: 1, color: colors.foregroundMuted, fontSize: fontSize.meta, lineHeight: 17 }}>{t("recap.aiSuggestionHint")}</Text>
        </View>
        <View style={{ flexDirection: "row", flexWrap: "wrap" }}>
          <Button theme={theme} label={ai.text} icon={ai.icon} tone={ai.tone} size={compact ? "md" : "sm"} grow={compact}
            busy={busy === ai.key} disabled={!caps || !!busy || locked || !!uncertain} accessibilityLabel={t("recap.sendAiReply", { agent: agentTitle, text: ai.text })} testID="quick-reply-ai"
            onPress={() => void send(ai)} />
        </View>
      </View> : null}
    </View>}
    
    {uncertain ? <Banner theme={theme} tone="warning" message={t("coreQuickReplies.mayAlreadyHaveArrivedOpenTheSession", { value1: uncertain.reply.text })} /> : null}
    {problem && !uncertain ? <Banner theme={theme} tone={problem.tone} message={problem.message}
      action={problem.retry !== false && !locked ? { label: t("coreQuickReplies.retry"), onPress: () => void send(problem.reply, problem.retry || null) } : undefined} /> : null}
    
    <MessageList theme={theme} queue={deferred} only={view => keys.has(view.key)} compact={compact} />
  </View>;
}
