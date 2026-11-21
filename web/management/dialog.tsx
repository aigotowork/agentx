import { useId, useLayoutEffect, useRef, type ReactNode } from 'react';
import { Icon, type Glyph } from './presentation';

interface DialogProps {
  open: boolean;
  variant: 'launch' | 'stop';
  icon?: Glyph;
  title: string;
  description: string;
  closeLabel: string;
  onDismiss: () => void;
  children: ReactNode;
  footer: ReactNode;
}

/**
 * @aiPurpose Provide the common modal anatomy, native focus trap and animated top-layer exit.
 * @aiUsage Control open from the decision owner; mark a safe control with data-dialog-focus or use the title.
 * @aiBehavior Escape, close, and an uninterrupted backdrop press dismiss; focus returns to the invoker or its focus fallback.
 */
export function InteractionDialog({ open, variant, icon, title, description, closeLabel, onDismiss, children, footer }: DialogProps) {
  const dialog = useRef<HTMLDialogElement>(null);
  const opener = useRef<HTMLElement>(null);
  const pointerOutside = useRef(false);
  const id = useId();
  useLayoutEffect(() => {
    const node = dialog.current;
    if (!node) return;
    if (open && !node.open) {
      opener.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      node.showModal();
      node.querySelector<HTMLElement>('[data-dialog-focus]')?.focus({ preventScroll: true });
    } else if (!open && node.open) {
      node.close();
      const previous = opener.current;
      if (previous?.isConnected && previous.getClientRects().length && !previous.matches(':disabled')) {
        previous.focus({ preventScroll: true });
      } else {
        const fallback = previous?.closest('.agent-detail') ?? document.querySelector<HTMLElement>('.console-main');
        if (fallback instanceof HTMLElement) fallback.focus({ preventScroll: true });
      }
    }
  }, [open]);
  return <dialog ref={dialog} className={`interaction-dialog ${variant}-dialog`} aria-labelledby={`${id}-title`} aria-describedby={`${id}-description`}
    onCancel={event => { event.preventDefault(); onDismiss(); }}
    onPointerDown={event => { pointerOutside.current = event.target === event.currentTarget; }}
    onClick={event => { if (pointerOutside.current && event.target === event.currentTarget) onDismiss(); pointerOutside.current = false; }}>
    <div className="dialog-surface">
      <header className="dialog-heading">
        <div className="dialog-heading-copy">
          <h2 id={`${id}-title`} tabIndex={-1} data-dialog-focus={variant === 'launch' ? true : undefined}>{icon && <Icon name={icon} />}{title}</h2>
          <p id={`${id}-description`}>{description}</p>
        </div>
        <button className="quiet icon-control dialog-close" type="button" onClick={onDismiss} aria-label={closeLabel} title={closeLabel}><Icon name="close" /></button>
      </header>
      <div className="dialog-body">{children}</div>
      <footer className="dialog-actions">{footer}</footer>
    </div>
  </dialog>;
}
