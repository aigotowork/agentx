import type { InboxPage, OfficeActivityPage } from '@agentx/client';

export const humanPageSize = 20;

/** Reuse the unfiltered newest page only when it proves the exact requested page. */
export function mailboxHead(activity: OfficeActivityPage, pending: boolean): InboxPage | undefined {
  const messages = pending ? activity.human_messages.filter(message => !message.handled_at) : activity.human_messages;
  const hasMore = pending ? activity.human_unhandled_count > messages.length : activity.human_has_more;
  if (pending && hasMore && messages.length < humanPageSize) return undefined;
  const scope = activity.checkpoint.scope, position = hasMore ? messages.at(-1)?.id : undefined;
  return { scope, messages, unhandledCount: activity.human_unhandled_count, next: position === undefined ? null : { scope, pending, position, kind: 'human-inbox' } };
}
