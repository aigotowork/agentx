/** Viewport-constrained facts must remain outside the complete invoking hit rectangle. */
export interface FactAnchor { left: number; top: number; right: number; bottom: number }
export interface FactPlacement { x: number; y: number; maxHeight: number }
const EDGE = 8, GAP = 8;
/** Prefer above, then below/side; constrain height within a free region rather than overlapping the trigger. */
export function placeFacts(anchor: FactAnchor, size: { width: number; height: number }, viewport: { width: number; height: number }): FactPlacement | null {
  const width = Math.min(size.width, Math.max(0, viewport.width - EDGE * 2));
  const height = Math.min(size.height, Math.max(0, viewport.height - EDGE * 2));
  const x = Math.max(EDGE, Math.min(viewport.width - EDGE - width, (anchor.left + anchor.right - width) / 2));
  const above = Math.max(0, anchor.top - EDGE - GAP), below = Math.max(0, viewport.height - EDGE - anchor.bottom - GAP);
  if (above >= height) return { x, y: anchor.top - GAP - height, maxHeight: height };
  if (below >= height) return { x, y: anchor.bottom + GAP, maxHeight: height };
  const y = Math.max(EDGE, Math.min(viewport.height - EDGE - height, anchor.top));
  if (anchor.right + GAP + width <= viewport.width - EDGE) return { x: anchor.right + GAP, y, maxHeight: height };
  if (anchor.left - GAP - width >= EDGE) return { x: anchor.left - GAP - width, y, maxHeight: height };
  const space = Math.max(above, below);
  if (space < 1 || width < 1) return null;
  return above >= below ? { x, y: EDGE, maxHeight: above } : { x, y: anchor.bottom + GAP, maxHeight: below };
}
