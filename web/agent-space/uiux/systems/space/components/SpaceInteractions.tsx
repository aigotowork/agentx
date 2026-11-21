/** @aiPurpose Shared overlay stack, Escape disclosure and page-supplied shortcut binding. */
import { createContext, useContext, useEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { OverlayStack, editingTarget } from "./overlay";

interface Shortcuts { newAgent(): void; killAgent(): void; search(): void }
interface Interactions { overlays: OverlayStack; shortcuts?: Shortcuts }
const Context = createContext<Interactions | null>(null);

/** @aiPurpose Own Space-wide top-layer Escape and safe, explicit keyboard shortcuts.
 * @aiUsage Wrap the complete Space once, including portals; children register actual overlays.
 * @aiBehavior Escape closes one interactive layer plus passive hints; shortcuts never act while editing/IME/modal.
 */
export function SpaceInteractions({ children }: { children: ReactNode }) {
  const boundary = useRef<HTMLDivElement>(null);
  const [owner] = useState<Interactions>(() => ({ overlays: new OverlayStack() }));
  useEffect(() => {
    function keyDown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.isComposing || event.keyCode === 229) return;
      // A pointer preview can open without moving focus, and pagination can remove its button.
      const bodyEscape = event.key === "Escape" && event.target === document.body && !!owner.overlays.active;
      if (event.target instanceof Node && !boundary.current?.contains(event.target) && !bodyEscape) return;
      if (event.key === "Escape") {
        // Native select menus keep their own Escape; IME composition is handled above.
        if (event.target instanceof HTMLSelectElement) return;
        const active = owner.overlays.active;
        const disclosures = [...boundary.current!.querySelectorAll<HTMLDetailsElement>('details[open]')].filter(node => node.getClientRects().length && (!active || active.contains?.(node)));
        const focused = [...disclosures].reverse().find(node => node.contains(document.activeElement));
        const disclosure = focused ?? disclosures.at(-1);
        if (disclosure) { disclosure.open = false; disclosure.querySelector('summary')?.focus(); }
        else if (!owner.overlays.dismiss()) return;
        event.preventDefault(); event.stopImmediatePropagation(); return;
      }
      if (event.repeat || editingTarget(event.target) || owner.overlays.active) return;
      let command: (() => void) | undefined;
      if ((event.ctrlKey || event.metaKey) && event.shiftKey && !event.altKey) {
        if (event.key.toLowerCase() === "n") command = owner.shortcuts?.newAgent;
        if (event.key.toLowerCase() === "k") command = owner.shortcuts?.killAgent;
      } else if (event.key === "/" && !event.ctrlKey && !event.metaKey && !event.altKey) command = owner.shortcuts?.search;
      if (command) { event.preventDefault(); command(); }
    }
    document.addEventListener("keydown", keyDown, true);
    return () => document.removeEventListener("keydown", keyDown, true);
  }, [owner]);
  return <Context.Provider value={owner}><div ref={boundary} className="space-interactions">{children}</div></Context.Provider>;
}

/** @aiPurpose Register mounted disclosure lifetime and restore its reachable invoker.
 * @aiUsage Supply the controlled visibility, close callback, shared priority and optional root; passive facts never take the interactive slot.
 */
export function useOverlay(open: boolean, onClose: () => void, priority: number, root?: RefObject<HTMLElement | null>, passive = false, returnFocus?: HTMLElement | null | false): () => void {
  const owner = useContext(Context), latest = useRef(onClose), opener = useRef<HTMLElement | null>(null);
  latest.current = onClose;
  const close = useRef(() => {
    latest.current();
    const node = opener.current;
    if (!passive) requestAnimationFrame(() => { if (node?.isConnected) node.focus({ preventScroll: true }); });
  }).current;
  useEffect(() => {
    if (!open) return;
    opener.current = returnFocus === false ? null : returnFocus ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    return owner?.overlays.add({ priority, passive, close, contains: node => !!node && !!root?.current?.contains(node) });
  }, [owner, open, priority, passive, root, close, returnFocus]);
  return close;
}

/** @aiPurpose Bind page-owned command requests without acquiring capabilities.
 * @aiUsage Call under SpaceInteractions; supplied handlers must retain their own capability guards.
 */
export function useSpaceShortcuts(shortcuts: Shortcuts): void {
  const owner = useContext(Context), current = useRef(shortcuts); current.current = shortcuts;
  useEffect(() => {
    if (!owner) return;
    const binding = { newAgent: () => current.current.newAgent(), killAgent: () => current.current.killAgent(), search: () => current.current.search() };
    owner.shortcuts = binding;
    return () => { if (owner.shortcuts === binding) owner.shortcuts = undefined; };
  }, [owner]);
}
