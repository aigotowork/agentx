import { useEffect, useId, useRef, type FormEvent, type ReactNode } from "react";
import { Button } from "./Controls";
import { useOverlay } from "./SpaceInteractions";
import { SurfaceHeader } from "./SurfaceHeader";

/** @aiPurpose Frame a deliberate confirmation or input in a native modal with a stable footer.
 * @aiUsage Mount when open; pass the exact invoker and controlled content. The consumer owns validation, native commands and the successful-close decision.
 * @aiBehavior Escape and Cancel share the overlay owner and restore focus; destructive commands start on Cancel and submit explicitly.
 */
export function CommandSurface({ title, opener, onClose, onSubmit, submitLabel, disabled, danger, children }: {
  title: string; opener: HTMLElement | null; onClose(): void; onSubmit(event: FormEvent<HTMLFormElement>, close: () => void): void;
  submitLabel: string; disabled?: boolean; danger?: boolean; children: ReactNode;
}) {
  const root = useRef<HTMLDialogElement>(null), cancel = useRef<HTMLButtonElement>(null), titleId = useId();
  const close = useOverlay(true, onClose, 100, root, false, opener);
  useEffect(() => {
    const node = root.current;
    node?.showModal();
    if (danger) cancel.current?.focus({ preventScroll: true });
    return () => node?.close();
  }, [danger]);
  return <dialog ref={root} className="agent-command-dialog" data-danger={danger || undefined} aria-labelledby={titleId} onCancel={event => { event.preventDefault(); close(); }}>
    <form onSubmit={event => onSubmit(event, close)}>
      <SurfaceHeader className="command-heading" title={<h2 id={titleId}>{title}</h2>} closeLabel="关闭操作对话框" onClose={close} />
      <div className="command-fields">{children}</div>
      <footer className="command-footer"><Button ref={cancel} onClick={close}>取消</Button><Button variant="primary" type="submit" disabled={disabled}>{submitLabel}</Button></footer>
    </form>
  </dialog>;
}
