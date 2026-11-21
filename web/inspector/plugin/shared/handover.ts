import { t } from "./text";
/** The label a handed-over Agent carries: the id of the Agent it took over from (opus-B23). */
export const HANDOVER_LABEL = "observatory.handover-from";
/**
 * 交给另一个 Provider: the new Agent's first message. No transcript is pasted: it reads the original itself with the CLI, looks
 * at the workspace's changes and carries on; the original must not be resumed as well.
 */
export function handoverPrompt(source: { id: string; title?: string | null }): string {
  const title = source.title?.trim() || t("sharedResume.m046");
  return [
    t("sharedResume.m047", { title: title, value2: source.id }),
    "",
    t("sharedResume.m048"),
    `   ${source.id}`,
    t("sharedResume.m049"),
    t("sharedResume.m050"),
    "",
    t("sharedResume.m051"),
  ].join("\n");
}
/** One stop handed to one profile is one Agent, however often it is tapped or retried (`idempotencyKey`). */
export function handoverKey(agentId: string, stopAt: string, profileId: string): string {
  return `observatory-handover:${agentId}:${stopAt}:${profileId}`.slice(0, 512);
}
/** 「<原标题>（接手自 <provider>）」. */
export function handoverTitle(title: string | null | undefined, provider: string): string {
  return t("sharedResume.m052", { value1: (title?.trim() || t("sharedResume.m053")).slice(0, 50), value2: provider.slice(0, 24) });
}

