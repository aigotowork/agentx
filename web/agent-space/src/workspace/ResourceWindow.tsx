import { useEffect, useId, useRef, type ReactNode } from "react";
import { useOverlay } from "../interactions/SpaceInteractions";

/** @aiPurpose Show office posts or skills beside the preserved conversation workspace.
 * @aiUsage Mount for the current resource view and return to its previous People view on dismissal.
 */
export function ResourceWindow({ title, onClose, children, compact }: { title: string; onClose(): void; children: ReactNode; compact?: boolean }) {
  const root = useRef<HTMLDialogElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const close = useOverlay(true, onClose, 70, root);
  useEffect(() => {
    const dialog = root.current;
    dialog?.showModal();
    closeButton.current?.focus({ preventScroll: true });
    return () => dialog?.close();
  }, []);
  return <dialog ref={root} className="space-resource-window" data-compact={compact || undefined} aria-labelledby={titleId} onCancel={event => { event.preventDefault(); close(); }}>
    <header className="space-resource-window-heading">
      <h2 id={titleId}>{title}</h2>
      <button ref={closeButton} type="button" aria-label="关闭办公室资源" onClick={close}>×</button>
    </header>
    <div className="space-resource-window-body">{children}</div>
  </dialog>;
}
