/** Contextual facts share one portal treatment across people and available posts. */
import { forwardRef, useEffect, useId, useRef, useState, type ButtonHTMLAttributes, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useOverlay } from './SpaceInteractions';
import { placeFacts } from './FactButton.geometry';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> { facts: ReactNode; dismissOnAction?: boolean }
/** @aiPurpose Reveal noninteractive local facts on hover/focus without changing selection.
 * @aiUsage Supply factual content and ordinary button behavior; clicks retain their original action.
 */
export const FactButton = forwardRef<HTMLButtonElement, Props>(function FactButton({ facts, children, dismissOnAction, ...props }, forwarded) {
  const anchor = useRef<HTMLButtonElement | null>(null), bubble = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false), id = useId();
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  function cancel() { clearTimeout(timer.current); }
  function show() { cancel(); timer.current = setTimeout(() => setOpen(true), 180); }
  function hide() { cancel(); timer.current = setTimeout(() => setOpen(false), 120); }
  useOverlay(open, () => { cancel(); setOpen(false); }, 0, bubble, true);
  useEffect(() => () => cancel(), []);
  useEffect(() => {
    if (!open) return;
    let frame = 0;
    function position() {
      const target = anchor.current, tip = bubble.current;
      if (!target || !tip || target.hidden || !target.isConnected) { setOpen(false); return; }
      const rect = target.getBoundingClientRect();
      if (rect.bottom < 0 || rect.top > innerHeight || rect.right < 0 || rect.left > innerWidth) { setOpen(false); return; }
      const label = target.querySelector(".office-actor-label, .office-human > span")?.getBoundingClientRect();
      const hit = label ? { left: Math.min(rect.left, label.left), right: Math.max(rect.right, label.right), top: Math.min(rect.top, label.top), bottom: Math.max(rect.bottom, label.bottom) } : rect;
      tip.style.maxHeight = `${Math.max(0, innerHeight - 16)}px`;
      const placement = placeFacts(hit, tip.getBoundingClientRect(), { width: innerWidth, height: innerHeight });
      if (!placement) { setOpen(false); return; }
      tip.style.left = `${placement.x}px`; tip.style.top = `${placement.y}px`; tip.style.maxHeight = `${placement.maxHeight}px`;
      frame = requestAnimationFrame(position);
    }
    frame = requestAnimationFrame(position);
    return () => { cancelAnimationFrame(frame); };
  }, [open]);
  return <><button {...props} ref={node => { anchor.current = node; if (typeof forwarded === 'function') forwarded(node); else if (forwarded) forwarded.current = node; }} aria-describedby={[props['aria-describedby'], open ? id : null].filter(Boolean).join(' ') || undefined} onPointerEnter={event => { props.onPointerEnter?.(event); if (!event.defaultPrevented) show(); }} onPointerLeave={event => { props.onPointerLeave?.(event); hide(); }} onFocus={event => { props.onFocus?.(event); cancel(); setOpen(true); }} onBlur={event => { props.onBlur?.(event); hide(); }} onClick={event => { props.onClick?.(event); if (dismissOnAction) { cancel(); setOpen(false); } }}>{children}</button>{open && createPortal(<div ref={bubble} id={id} className="space-fact-tooltip" role="tooltip" onPointerEnter={cancel} onPointerLeave={hide}>{facts}</div>, anchor.current?.closest(".space-system") ?? document.body)}</>;
});
