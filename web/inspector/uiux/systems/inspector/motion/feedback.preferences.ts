/** Browser-local expressive feedback; independent from domain preferences and appearance routing. */
export type ToolCursor = "system" | "hammer" | "shovel" | "chisel";
export type FeedbackPreferences = { motion: boolean; sound: boolean; cursor: ToolCursor; volume: number };
export const feedbackKey = "agent-inspector.pixel-feedback.v1";
export const feedbackDefaults: FeedbackPreferences = { motion: true, sound: false, cursor: "hammer", volume: 0.3 };
const tools: readonly unknown[] = ["system", "hammer", "shovel", "chisel"];

export function readFeedback(storage: Pick<Storage, "getItem">): FeedbackPreferences {
  try {
    const value = JSON.parse(storage.getItem(feedbackKey) ?? "null");
    if (!value || typeof value !== "object") return { ...feedbackDefaults };
    return {
      motion: typeof value.motion === "boolean" ? value.motion : feedbackDefaults.motion,
      sound: value.sound === true,
      cursor: tools.includes(value.cursor) ? value.cursor : feedbackDefaults.cursor,
      volume: typeof value.volume === "number" && Number.isFinite(value.volume) ? Math.max(0, Math.min(1, value.volume)) : feedbackDefaults.volume,
    };
  } catch { return { ...feedbackDefaults }; }
}

export function saveFeedback(value: FeedbackPreferences, storage: Pick<Storage, "setItem">): void {
  try { storage.setItem(feedbackKey, JSON.stringify(value)); } catch { /* Optional persistence. */ }
}
