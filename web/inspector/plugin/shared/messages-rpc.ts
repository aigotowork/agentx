import { defineRpc } from "../../host/contracts";
import { z } from "zod";
import { addDefaults, t } from "./i18n";
import { kitDeferred } from "./locales/kit/zh-CN.json";

addDefaults({ kitDeferred });

/** Longest name for the message in a confirmation (`what`: "message", "answer", a translated word such as "Eingabeaufforderung"). */
export const SEND_NOUN_LIMIT = 80;

export const MessageViewSchema = z.object({
  messageId: z.string(),
  agentId: z.string(),
  /** What makes two messages the same request: one per agent and key is on its way at a time. */
  key: z.string(),
  source: z.string(),
  preview: z.string(),
  createdAt: z.number(),
  at: z.number(),
  state: z.enum(["queued", "held", "sending", "sent", "maybe-sent", "canceled", "discarded", "expired"]),
  note: z.string(),
  
});
export type MessageView = z.infer<typeof MessageViewSchema>;

/** What the user agreed to in a confirmation (`consentFor` in send-plan.ts). */
export const ConsentSchema = z.object({ interrupt: z.boolean(), approvals: z.array(z.string().max(200)).max(100), turn: z.string().max(300).nullable().optional() });

export const MessageGuardSchema = z.record(z.string().min(1).max(40), z.string().max(200).nullable()).refine((value) => Object.keys(value).length <= 8, "at most 8 conditions");

export const MessageSendResultSchema = z.union([
  /** sent, still sending, held for the user (failed or unknown outcome), or the same content already on its way. */
  z.object({ status: z.enum(["sent", "sending", "held", "existing"]), view: MessageViewSchema }),
  /** What sending would do now needs the user's OK: ask `plan.confirm`, then send again with `consent`. */
  z.object({ status: z.literal("confirm"), plan: z.object({ label: z.string(), note: z.string(), confirm: z.string() }), consent: ConsentSchema }),
  /**
   * Nothing was sent: another send to the agent is going on (or the plugin could not check the message's `guard`), the agent
   * is archived or deleted, or the plugin's own condition for this message (`guard`) no longer holds.
   */
  z.object({ status: z.enum(["busy", "gone", "refused"]), message: z.string() }),
]);
export type MessageSendResult = z.infer<typeof MessageSendResultSchema>;

// Not `key`: a top-level `key` makes esbuild rename its interop helper's loop variable, and AgentX's Hermes patch
// (which rewrites the exact text `from[key]`) then misses it.
const sendKey = z.string().min(1).max(120);

export function immediateSendContracts(prefix: string) {
  return {
    /**
     * Send now. The server plans again from the agent as it is and sends only if `consent` covers what that would do;
     * otherwise it answers "confirm". Retry with the same `messageId` after a lost answer: it never sends twice. `group`:
     * messages answering one question (one per group goes out; the others answer with it, "existing"); `guard`: the plugin's
     * own condition, checked right before the send ("refused" once it no longer holds).
     */
    send: defineRpc({
      name: `${prefix}.messages.send`,
      input: z.object({
        group: sendKey.optional(), guard: MessageGuardSchema.optional(),
        agentId: z.string().min(1), text: z.string().min(1).max(200_000), messageId: z.string().min(8).max(100), source: z.string().max(200).optional(),
        key: sendKey.optional(), what: z.string().max(SEND_NOUN_LIMIT).optional(), consent: ConsentSchema.nullable().optional(),
      }),
      output: MessageSendResultSchema,
    }),
    /** Queued, held and sending messages, plus the last half hour's outcomes; newest first. */
    list: defineRpc({ name: `${prefix}.messages.list`, input: z.object({ agentId: z.string().optional() }), output: z.array(MessageViewSchema) }),
  };
}

/** Still on its way: queued, held for the user's OK, or sending. At most one per agent and key. */
export function isOnItsWay(view: Pick<MessageView, "state">): boolean {
  return view.state === "queued" || view.state === "held" || view.state === "sending";
}

/** The message with this content that is still on its way to this agent, if any. */
export function onItsWay(views: readonly MessageView[] | null | undefined, agentId: string, key: string): MessageView | null {
  return views?.find((view) => view.agentId === agentId && view.key === key && isOnItsWay(view)) ?? null;
}

/** Short status for a chip, and its tone (kit `Tone`): amber while waiting or sending, orange when it needs you, green once sent. */
export function messageBadge(view: Pick<MessageView, "state">): { label: string; tone: "warning" | "orange" | "success" | "neutral" | "danger" } {
  switch (view.state) {
    case "queued": return { label: t("kitDeferred.queued"), tone: "warning" };
    case "held": return { label: t("kitDeferred.held"), tone: "orange" };
    case "sending": return { label: t("kitDeferred.sending"), tone: "warning" };
    case "sent": return { label: t("kitDeferred.sent"), tone: "success" };
    case "maybe-sent": return { label: t("kitDeferred.maybeSent"), tone: "warning" };
    case "canceled": return { label: t("kitDeferred.canceled"), tone: "neutral" };
    case "discarded": return { label: t("kitDeferred.discarded"), tone: "danger" };
    case "expired": return { label: t("kitDeferred.expired"), tone: "danger" };
  }
}
