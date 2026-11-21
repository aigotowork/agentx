/** @aiPurpose Controlled object reader with shared overlay dismissal, focus and a product-supplied geometry boundary. */
import { useEffect, useRef, type CSSProperties, type ReactNode } from "react";
import { useOverlay } from "./SpaceInteractions";
import { Button } from "./Controls";
import { SurfaceHeader } from "./SurfaceHeader";
interface Props { title: string; kind: string; expanded: boolean; visible: boolean; restoreInvoker?: boolean; geometry: CSSProperties; onExpand(): void; onClose(): void; children: ReactNode }
/** @aiPurpose Keep the anchored reading surface mounted while selection visibility changes.
 * @aiUsage Supply validated window geometry from the scene adapter; callbacks express size/close intent and do not change drafts.
 * @aiBehavior Shares reader priority 20; Escape closes only the foremost layer and restores its invoker.
 */
export function ObjectWindow({ title, kind, expanded, visible, restoreInvoker = true, geometry, onExpand, onClose, children }: Props) {
  const root = useRef<HTMLElement>(null);
  const close = useOverlay(visible, onClose, 20, root, false, restoreInvoker ? undefined : false);
  useEffect(() => { if (visible) root.current?.focus({ preventScroll: true }); }, [visible, title]);
  const expand = <Button className="text-button" aria-label={expanded ? "收起阅读尺寸" : "放大阅读"} onClick={onExpand}>{expanded ? "收起" : "放大"}</Button>;
  const heading = <h2><span className="object-window-title">{title}</span>{kind === "conversation" && <span className="object-window-kicker"><svg viewBox="0 0 20 20" aria-hidden="true"><rect x="2.5" y="4" width="15" height="12" rx="2" /><path d="m3 5 7 6 7-6" /></svg>会话</span>}</h2>;
  return <section ref={root} hidden={!visible} tabIndex={-1} role="region" aria-label={title} className="office-expanded-object" data-kind={kind} data-expanded={expanded} style={geometry}><SurfaceHeader className="office-object-heading" title={heading} actions={expand} closeLabel="关闭并返回办公室" onClose={close} /><div className="office-object-content">{children}</div></section>;
}
