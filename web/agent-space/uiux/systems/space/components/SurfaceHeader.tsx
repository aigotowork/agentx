import type { ReactNode } from "react";
import { Button } from "./Controls";

/** @aiPurpose Give a framed surface a title, optional neighboring actions and a named close control.
 * @aiUsage The caller owns focus, dismissal and what closing means; title content must provide the visible hierarchy.
 */
export function SurfaceHeader({ className, title, actions, closeLabel, onClose }: {
  className?: string; title: ReactNode; actions?: ReactNode; closeLabel: string; onClose(): void;
}) {
  return <header className={className}>{title}<div className="button-row surface-header-actions">{actions}<Button variant="icon" className="surface-close" aria-label={closeLabel} title={closeLabel} onClick={onClose}><svg viewBox="0 0 20 20" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15" /></svg></Button></div></header>;
}
