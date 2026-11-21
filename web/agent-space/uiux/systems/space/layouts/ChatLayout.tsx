import { useEffect, useId, useRef, useState, type ReactNode } from "react";
/** @aiPurpose Preserve the source IM roster/reader split with local narrow-screen return.
 * @aiUsage Parent retains selection/drafts while reading toggles the narrow-screen region.
 */
export function ChatLayout({ directory, reader, workspace, hasWindows, reading, onReturnFallback }: { directory: ReactNode; reader: ReactNode; workspace?: ReactNode; hasWindows?: boolean; reading: boolean; onReturnFallback?(): void }) {
  const root = useRef<HTMLDivElement>(null), wasReading = useRef(false);
  const [rosterOpen, setRosterOpen] = useState(false);
  const rosterId = useId();
  const rosterTrigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (workspace) return;
    const node = root.current;
    if (reading) node?.querySelector<HTMLElement>('.space-chat-reader')?.focus({ preventScroll: true });
    else if (wasReading.current) {
      const row = node?.querySelector<HTMLElement>('.hall-directory-person[aria-current]');
      if (row) { row.scrollIntoView({ block: 'nearest' }); row.focus({ preventScroll: true }); }
      else onReturnFallback?.();
    }
    wasReading.current = reading;
  }, [reading, !!workspace]);
  function dismissRoster(focusTrigger: boolean) {
    setRosterOpen(false);
    if (focusTrigger) rosterTrigger.current?.focus({ preventScroll: true });
  }
  return <div ref={root} className="space-chat-layout" data-reading={reading} data-window-workspace={!!workspace || undefined} data-has-windows={hasWindows || undefined} data-roster-open={rosterOpen || undefined}>
    {workspace && reading && <button type="button" ref={rosterTrigger} className="space-chat-roster-trigger" aria-controls={rosterId} aria-expanded={rosterOpen} onClick={() => setRosterOpen(value => !value)}>{rosterOpen ? "收起名册" : "名册"}</button>}
    <aside id={rosterId} className="space-chat-roster" aria-label="会话名单" onKeyDown={event => {
      if (rosterOpen && event.key === "Escape") { event.stopPropagation(); dismissRoster(true); }
    }} onClick={event => { if (rosterOpen && event.target instanceof Element && event.target.closest("[data-agent-key]")) dismissRoster(true); }}>{directory}</aside>
    <section tabIndex={-1} className="space-chat-reader" aria-label="会话阅读">{hasWindows ? workspace : reader}</section>
  </div>;
}
