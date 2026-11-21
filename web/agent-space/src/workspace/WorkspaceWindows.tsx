/** @aiPurpose Lay out exact conversation windows over the scene or in the chat reader region. */
import { useRef, type FocusEvent, type ReactNode } from "react";
import "./workspace.css";

interface Pane {
  id: string;
  title: string;
  minimized: boolean;
  visible: boolean;
  reader?: ReactNode | ((windowControls: ReactNode) => ReactNode);
}

interface Props {
  panes: readonly Pane[];
  activeId: string | null;
  focused: boolean;
  overlay: boolean;
  motion?: { id: string; event: "open" | "minimize" | "restore" };
  onActivate(id: string): void;
  onFocus(id: string): void;
  onMinimize(id: string): void;
  onClose(id: string): void;
}

const icons = {
  focus: "M6 10V6h4m4 0h4v4m0 4v4h-4m-4 0H6v-4",
  restore: "M6 10h4V6m4 0v4h4m0 4h-4v4m-4 0v-4H6",
  minimize: "M6 15h12",
  close: "m7 7 10 10M17 7 7 17",
};

function WindowIcon({ name }: { name: keyof typeof icons }) {
  return <svg className="space-window-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={icons[name]} /></svg>;
}

function WindowPane({ pane, active, focused, motion, onActivate, onFocus, onMinimize, onClose }: {
  pane: Pane;
  active: boolean;
  focused: boolean;
  motion?: "open" | "restore";
  onActivate(id: string): void;
  onFocus(id: string): void;
  onMinimize(id: string): void;
  onClose(id: string): void;
}) {
  const focusLabel = focused ? "恢复并排" : "聚焦会话";
  const controls = <div className="space-window-actions" data-window-motion-header="">
    <button type="button" className="space-window-action" data-window-action="focus" aria-label={`${focusLabel}：${pane.title}`} title={focusLabel} onClick={() => onFocus(pane.id)}><WindowIcon name={focused ? "restore" : "focus"} /></button>
    <button type="button" className="space-window-action" data-window-action="minimize" aria-label={`最小化会话：${pane.title}`} title="最小化会话" onClick={() => onMinimize(pane.id)}><WindowIcon name="minimize" /></button>
    <button type="button" className="space-window-action" data-window-action="close" aria-label={`关闭会话：${pane.title}`} title="关闭会话" onClick={() => onClose(pane.id)}><WindowIcon name="close" /></button>
  </div>;
  const content = pane.reader;
  const embedded = typeof content === "function";
  const reader = embedded ? content(controls) : content;

  function activateReader(event: FocusEvent<HTMLElement>) {
    if (active || event.currentTarget.contains(event.relatedTarget)) return;
    // Window actions already identify their own target, including when a peer is closed or parked directly.
    if (!event.target.closest("[data-window-action]")) onActivate(pane.id);
  }

  return <section className="space-window-pane" aria-label={pane.title} data-pane-id={pane.id} data-active={active} data-focused={focused} data-window-motion={motion} data-embedded-controls={embedded} onFocusCapture={activateReader}>
    {!embedded && <header className="space-window-header"><span className="space-window-caption" title={pane.title}>{pane.title}</span>{controls}</header>}
    <div className="space-window-reader">{reader}</div>
  </section>;
}

/** @aiPurpose Keep the open conversation shelf and visible reading panes in the same working region.
 * @aiUsage Pass ordered exact IDs, recognizable Agent names and at most two visible readers; set overlay inside a positioned scene, or fill the existing chat reader slot.
 * @aiBehavior The caller owns visibility, focus mode, the optional motion marker and last-window return; local focus follows the shelf when a pane is minimized or closed.
 * @aiLayout The deck follows the caller's reader leases; a reader render function places its supplied window controls in an existing strip.
 */
export function WorkspaceWindows({ panes, activeId, focused, overlay, motion, onActivate, onFocus, onMinimize, onClose }: Props) {
  const shelf = useRef<HTMLElement>(null);

  function focusShelf(id: string, closing: boolean) {
    const buttons = Array.from(shelf.current?.querySelectorAll<HTMLButtonElement>("[data-shelf-id]") ?? []);
    const index = buttons.findIndex(button => button.dataset.shelfId === id);
    if (index < 0) return;
    const next = closing ? buttons[index + 1] ?? buttons[index - 1] : buttons[index];
    next?.focus();
  }

  function minimize(id: string) { focusShelf(id, false); onMinimize(id); }
  function close(id: string) { focusShelf(id, true); onClose(id); }

  const visiblePanes = panes.filter(pane => pane.visible && !pane.minimized);
  const shelfMotionId = motion?.event === "minimize" ? motion.id : undefined;
  const paneMotion = motion?.event === "open" || motion?.event === "restore" ? motion.event : undefined;
  if (!panes.length) return null;

  return <section className="space-window-region" aria-label="会话工作台" data-overlay={overlay} data-focused={focused}>
    <nav ref={shelf} className="space-window-shelf" aria-label="已打开的会话">
      <ul className="space-window-shelf-list">
        {panes.map(pane => <li key={pane.id}>
          <button type="button" className="space-window-shelf-button" data-shelf-id={pane.id} data-minimized={pane.minimized} data-window-motion={pane.minimized && pane.id === shelfMotionId ? "minimize" : undefined} aria-current={pane.id === activeId ? "true" : undefined} aria-label={pane.minimized ? `${pane.title}（已最小化）` : undefined} title={pane.title} onClick={() => onActivate(pane.id)}>
            {pane.minimized && <WindowIcon name="minimize" />}
            <span className="space-window-shelf-title">{pane.title}</span>
          </button>
        </li>)}
      </ul>
    </nav>
    <div className="space-window-deck" data-tiled={visiblePanes.length === 2}>
      {visiblePanes.map(pane => <WindowPane key={pane.id} pane={pane} active={pane.id === activeId} focused={focused && pane.id === activeId} motion={motion?.id === pane.id ? paneMotion : undefined} onActivate={onActivate} onFocus={onFocus} onMinimize={minimize} onClose={close} />)}
    </div>
  </section>;
}
