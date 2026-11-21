/** @aiPurpose Provide compact accessible controls, local disclosure and bounded decisions. */
import { useEffect, useId, useRef, useState } from "react";
import type { ReactNode } from "react";

type IconName = "info" | "refresh" | "add" | "close" | "edit" | "link" | "source" | "message" | "more" | "check";
const paths: Record<IconName, string> = { info: "M8 7v5M8 4h.01M15 8A7 7 0 1 1 1 8a7 7 0 0 1 14 0", refresh: "M14 6a6 6 0 1 0 0 5M14 1v5H9", add: "M8 2v12M2 8h12", close: "m3 3 10 10M13 3 3 13", edit: "m2 11 9-9 3 3-9 9H2v-3", link: "m6 10 4-4M6 5l2-2a3 3 0 0 1 5 5l-2 2M10 11l-2 2a3 3 0 0 1-5-5l2-2", source: "M3 1h7l3 3v11H3V1m7 0v4h3M5 8h6M5 11h5", message: "M1 2h14v10H6l-4 3v-3H1V2m3 4h8M4 9h5", more: "M2 8h.1M8 8h.1M14 8h.1", check: "m2 8 4 4 8-9" };
interface IconActionProps { icon: IconName; label: string; onClick?: () => void; disabledReason?: string | null; expanded?: boolean }

/**
 * @aiPurpose Show one named compact action with keyboard/touch disclosure.
 * @aiUsage Supply a reason for a blocked action; click then only reveals that reason.
 * @aiBehavior Activation reveals the label/reason and invokes an available action once; Escape, blur or outside pointer dismisses.
 */
export function IconAction({ icon, label, onClick, disabledReason, expanded }: IconActionProps) {
  const [shown, setShown] = useState(false); const id = useId(); const host = useRef<HTMLSpanElement>(null);
  useEffect(() => { if (!shown) return; const dismiss = (event: PointerEvent) => { if (!host.current?.contains(event.target as Node)) setShown(false); }; document.addEventListener("pointerdown", dismiss); return () => document.removeEventListener("pointerdown", dismiss); }, [shown]);
  const activate = () => { setShown(true); if (!disabledReason) onClick?.(); };
  const tooltip = disabledReason || label;
  return <span className="icon-host" ref={host} onPointerEnter={event => { if (event.pointerType === "mouse") setShown(true); }} onPointerLeave={event => { if (event.pointerType === "mouse") setShown(false); }} onKeyDown={event => { if (event.key === "Escape") { setShown(false); event.stopPropagation(); } }}><button type="button" className="icon-action" aria-label={label} aria-disabled={!!disabledReason} aria-expanded={expanded} aria-describedby={shown ? id : undefined} onPointerDown={event => { if (event.pointerType !== "mouse") setShown(true); }} onClick={activate} onFocus={() => setShown(true)} onBlur={() => setShown(false)}><svg viewBox="0 0 16 16" aria-hidden="true"><path d={paths[icon]} /></svg></button>{shown && <span id={id} role="tooltip" className="control-tooltip">{tooltip}</span>}</span>;
}

/**
 * @aiPurpose Give one semantic action a visible unavailable reason.
 * @aiUsage Pending and unknown command guards are supplied by the owning Page.
 */
export function Action({ children, onClick, reason, primary = false, type = "button" }: { children: ReactNode; onClick?: () => void; reason?: string | null; primary?: boolean; type?: "button" | "submit" }) {
  return <span className="action-with-reason"><button type={type} className={primary ? "action primary" : "action"} disabled={!!reason} onClick={onClick}>{children}</button>{reason && <span className="action-reason">{reason}</span>}</span>;
}

/**
 * @aiPurpose Pair status meaning with text and a supporting glyph.
 * @aiUsage Do not rely on color to distinguish current, historical or unknown.
 */
export function StatusLabel({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "good" | "warning" | "danger" | "info" }) { const glyph = { neutral: "·", good: "✓", warning: "!", danger: "×", info: "↗" }[tone]; return <span className={`status-label ${tone}`}><span aria-hidden="true">{glyph}</span>{children}</span>; }

/**
 * @aiPurpose Associate labels and validation with a local editor field.
 * @aiUsage Pass an input whose id matches the field id.
 */
export function DenseField({ id, label, error, children }: { id: string; label: string; error?: string; children: ReactNode }) { return <div className="dense-field"><label htmlFor={id}>{label}</label>{children}{error && <span id={`${id}-error`} className="field-error">{error}</span>}</div>; }

/**
 * @aiPurpose Reveal a complete saved reference and explicit external navigation.
 * @aiUsage Local paths are copied only, never fetched.
 */
export function ReferenceLink({ value, label, revision }: { value: string; label?: string; revision?: string }) {
  const [copy, setCopy] = useState<string | null>(null); const safe = /^https?:\/\//i.test(value);
  const copyReference = async () => { try { await navigator.clipboard.writeText(value); setCopy("已复制"); } catch { setCopy(value); } };
  return <details className="reference-disclosure"><summary>{label ?? value}</summary><div className="reference-content"><code>{value}</code>{revision && <code>{revision}</code>}<div className="action-row"><Action onClick={() => void copyReference()}>复制引用</Action>{safe && <a href={value} target="_blank" rel="noopener noreferrer">打开外部引用 ↗</a>}</div>{copy && <span className="copy-result" role="status">{copy}</span>}</div></details>;
}

/**
 * @aiPurpose Render a finite modal decision with native focus confinement.
 * @aiUsage Children own the consequence and explicit confirm/cancel intents.
 */
export function DecisionDialog({ title, children, onClose }: { title: string; children: ReactNode; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null); const id = useId();
  useEffect(() => { const invoker = document.activeElement as HTMLElement | null; ref.current?.showModal(); return () => { ref.current?.close(); invoker?.focus(); }; }, []);
  return <dialog className="decision-dialog" ref={ref} aria-labelledby={id} onCancel={event => { event.preventDefault(); onClose(); }}><header className="panel-heading"><h2 id={id}>{title}</h2><IconAction icon="close" label="关闭并返回" onClick={onClose} /></header>{children}</dialog>;
}

/**
 * @aiPurpose Keep an empty/error/loading message local to the affected collection.
 * @aiUsage Existing data remains visible during refresh failures.
 */
export function LocalNotice({ children, danger = false }: { children: ReactNode; danger?: boolean }) { return <div className={danger ? "local-notice danger" : "local-notice"} role={danger ? "alert" : "status"}>{children}</div>; }
