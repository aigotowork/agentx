/** @aiPurpose Place readable object content beside its world anchor and preserve its mounted conversation. */
import { type CSSProperties, type ReactNode } from "react";
import { ObjectWindow } from "../../uiux/systems/space/space.web";
import { VIEW, type Point, type Viewport } from "./geometry";
import { objectPosition } from "./object-position";

/** @aiPurpose Preserve an expandable conversation window beside its scene object.
 * @aiUsage Parent controls visibility and return; children retains the page-owned draft.
 */
export function ExpandedObject({ title, kind, point, viewport, expanded, visible, onExpand, onClose, children }: { title: string; kind: string; point: Point; viewport: Viewport; expanded: boolean; visible: boolean; onExpand(): void; onClose(): void; children: ReactNode }) {
  const box = objectPosition(point, viewport, expanded, ["management", "library", "agent"].includes(kind));
  const arrow = Math.max(VIEW.edge * 2, Math.min(box.height - VIEW.edge * 2, point.y - box.y));
  const style = { "--office-object-left": `${box.x}px`, "--office-object-top": `${box.y}px`, "--office-object-width": `${box.width}px`, "--office-object-height": `${box.height}px`, "--office-object-anchor-y": `${arrow}px` } as CSSProperties;
  return <ObjectWindow title={title} kind={kind} expanded={expanded} visible={visible} restoreInvoker={kind !== "conversation"} geometry={style} onExpand={onExpand} onClose={onClose}>{children}</ObjectWindow>;
}
