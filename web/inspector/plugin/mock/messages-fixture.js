

import { consentFor, coversConsent, permissionIds, sendPlan } from "../client/send-plan";

const NOTES = {
  sent: "已送达",
  gone: "这个 Agent 已归档或不存在，没有发送。",
};
const RECENT_SAME_MS = 10_000;
const LISTED_MS = 30 * 60_000;
const onWay = message => message.state === "queued" || message.state === "held" || message.state === "sending";
const answerFor = view => view.state === "sent" ? "sent" : view.state === "sending" ? "sending" : view.state === "held" ? "held" : "existing";

/** Page-local immediate-send ledger: confirm current state, deduplicate message IDs, retain unknown outcomes. No background execution. */
export function createFixtureLedger({ agent, deliver, capabilities, guard: check = () => null }) {
  const messages = new Map();
  const view = message => ({ messageId: message.messageId, agentId: message.agentId, key: message.key, source: message.source, preview: message.text.replace(/\s+/g, " ").slice(0, 160),
    createdAt: message.createdAt, at: message.at, state: message.state, note: message.note });
  const settle = (message, state, note) => Object.assign(message, { state, note, at: Date.now() });
  const sameContent = (agentId, key) => [...messages.values()].find(message => message.agentId === agentId && message.key === key
    && (onWay(message) || (message.state === "sent" || message.state === "maybe-sent") && Date.now() - message.at < RECENT_SAME_MS)) ?? null;
  // One reply per question (a quick reply's turn): one on its way, or one that went out or may have, answers it for good.
  const sameGroup = (agentId, group) => group ? [...messages.values()].find(message => message.agentId === agentId && message.group === group
    && (onWay(message) || message.state === "sent" || message.state === "maybe-sent")) ?? null : null;
  const refused = reason => `${reason}，没有发送。`;
  const unknownNote = error => `没能确认这条消息还适用（${error}），没有发送。`;
  /** The plugin's check of a guarded message: `reason` (null when it fits), or `unknown` when it threw. */
  const verdict = async (message, current) => {
    if (!message.guard) return { reason: null };
    try { return { reason: await check(message.agentId, message.guard, current) }; } catch (error) { return { unknown: error instanceof Error ? error.message : String(error) }; }
  };
  async function go(message) {
    settle(message, "sending", "正在发送");
    try { await deliver(message.agentId, message.text, { activeTurnBehavior: message.behavior, messageId: message.messageId }); settle(message, "sent", NOTES.sent); }
    catch (error) { settle(message, "maybe-sent", "发送结果未知；请查看会话记录。不会自动重发。"); }
  }
  return {
    async send(input) {
      const known = messages.get(input.messageId);
      if (known) return { status: answerFor(known), view: view(known) };
      const other = sameContent(input.agentId, input.key) ?? sameGroup(input.agentId, input.group);
      if (other) return { status: "existing", view: view(other) };
      const current = agent(input.agentId);
      if (!current || current.archivedAt) return { status: "gone", message: NOTES.gone };
      const found = await verdict(input, current);
      if ("unknown" in found) return { status: "busy", message: `${unknownNote(found.unknown)}请稍后再试。` };
      if (found.reason) return { status: "refused", message: refused(found.reason) };
      const plan = sendPlan({ ...current, pendingPermissions: current.pendingPermissions?.length ?? 0 }, { ...capabilities, what: input.what, text: input.text });
      if (plan.kind === "unavailable") return { status: "gone", message: NOTES.gone };
      const needed = consentFor(plan, permissionIds(current.pendingPermissions), current.activeTurn);
      if (needed && !coversConsent(input.consent, needed)) return { status: "confirm", plan: { label: plan.label, note: plan.note ?? "", confirm: plan.confirm ?? plan.note ?? "" }, consent: needed };
      const message = { ...input, createdAt: Date.now(), at: Date.now(), state: "sending", note: "正在发送", behavior: plan.options?.activeTurnBehavior ?? "interrupt" };
      messages.set(input.messageId, message);
      await go(message);
      return { status: answerFor(message), view: view(message) };
    },
    list({ agentId }) {
      return [...messages.values()].filter(message => (!agentId || message.agentId === agentId) && (onWay(message) || Date.now() - message.at < LISTED_MS))
        .sort((a, b) => b.createdAt - a.createdAt).map(view);
    },
  };
}
