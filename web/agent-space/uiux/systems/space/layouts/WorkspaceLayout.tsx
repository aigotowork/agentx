import type { ReactNode } from "react";
import { Button } from "../components/Controls";
import { useConversationScroll, type ConversationScroll } from "./useConversationScroll";
export type { ConversationScroll } from "./useConversationScroll";

/** Layout owns scrolling and narrow-screen disclosure; work survives in the page owner. */
/** @aiPurpose Arrange original IM navigation beside its active conversation.
 * @aiUsage Own only layout; pass application state and handlers from the page.
 */
export function WorkspaceLayout(props: {
  header: ReactNode;
  navigation: ReactNode;
  children: ReactNode;
  navigationOpen: boolean;
  onDismissNavigation(): void;
}) {
  return <div className="workspace" data-navigation-open={props.navigationOpen}>
    <header className="workspace-header">{props.header}</header>
    {props.navigationOpen && <button className="navigation-scrim" aria-label="关闭导航" onClick={props.onDismissNavigation} />}
    <aside className="workspace-navigation" aria-label="联系人与会话">{props.navigation}</aside>
    <main className="workspace-main" id="main-content">{props.children}</main>
  </div>;
}

/** @aiPurpose Keep identity and primary input persistent while records and secondary context share body scrolling.
 * @aiUsage Supply controlled content; the body owns record/context scrolling and the intrinsic footer never shrinks below its primary controls.
 */
export function ConversationLayout(props: { header: ReactNode; history: ReactNode; composer?: ReactNode; context?: ReactNode; feedback?: ReactNode; historyScroll?: ConversationScroll; liveIssue?: ReactNode; olderStatus?: ReactNode }) {
  const { scroll, unread, jumpToLatest, scrolled, demand, touchStart } = useConversationScroll(props.historyScroll);
  const controlledHistory = !!props.historyScroll?.latest;
  const newLabel = `${unread} 条新消息 · 回到最新`;
  return <section className="conversation-layout" aria-label="当前会话">
    <header className="conversation-heading">{props.header}</header>
    <div className="conversation-feedback">{props.feedback}</div>
    <div className="conversation-history" aria-label="会话历史" tabIndex={0} data-controlled-history={controlledHistory || undefined} ref={scroll}
        onScroll={event => scrolled(event.currentTarget)}
        onWheel={event => demand(event.currentTarget, event.deltaY)}
        onKeyDown={event => {
          if (event.target !== event.currentTarget) return;
          if (['Home', 'PageUp', 'ArrowUp'].includes(event.key) || event.key === ' ' && event.shiftKey) demand(event.currentTarget, -1);
        }}
        onTouchStart={event => { touchStart.current = event.touches.length === 1 ? event.touches[0].clientY : undefined; }}
        onTouchCancel={() => { touchStart.current = undefined; }}
        onTouchEnd={event => {
          const distance = touchStart.current === undefined ? 0 : touchStart.current - event.changedTouches[0]?.clientY;
          if (Math.abs(distance) > 24) demand(event.currentTarget, distance);
          touchStart.current = undefined;
        }}>{props.history}{props.context && <div className="conversation-context">{props.context}</div>}</div>
    <div className="conversation-history-status" data-edge="start" aria-live="polite">
      {props.olderStatus && <div className="conversation-older-status">{props.olderStatus}</div>}
    </div>
    <div className="conversation-history-status" aria-live="polite">
      {props.liveIssue}
      {unread > 0 && <Button aria-label={newLabel} onClick={jumpToLatest}>{newLabel}</Button>}
    </div>
    {props.composer && <footer className="conversation-composer">{props.composer}</footer>}
  </section>;
}
