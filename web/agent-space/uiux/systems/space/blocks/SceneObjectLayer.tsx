/** @aiPurpose Accessible DOM anchors that consume existing scene positions and keep React identity stable. */
import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { FactButton } from "../components/FactButton";
/** @aiPurpose Group accessible world entries above the actual Canvas.
 * @aiUsage Render canonical anchors here; runtime updates only their position variables and never React state per frame.
 */
export function SceneObjectLayer({ children }: { children: ReactNode }) { return <div className="office-object-layer" role="group" aria-label="大厅人物与道具">{children}</div>; }
interface ActorProps extends ButtonHTMLAttributes<HTMLButtonElement> { facts: ReactNode; label: string; controller: boolean }
/** @aiPurpose Present a supplied exact Agent entry, label, role crown and contextual facts.
 * @aiUsage Pass exact data attributes, existing gesture handlers and runtime-owned position/ref from HallScene; the anchor creates no clock or selection state.
 */
export const ActorAnchor = forwardRef<HTMLButtonElement, ActorProps>(function ActorAnchor({ facts, label, controller, ...props }, ref) {
  return <FactButton {...props} ref={ref} facts={facts} dismissOnAction type="button" className="office-actor"><span className="office-actor-label">{controller && <svg className="office-controller-crown" viewBox="0 0 10 8" aria-hidden="true"><path d="M.5 1.5 3 3 5 .5 7 3 9.5 1.5 8.5 7.5h-7Z" /></svg>}<span className="office-actor-name">{label}</span></span></FactButton>;
});
