/** @aiPurpose Give long object forms readable room while retaining their scene anchor and return view. */
import { bubblePosition, VIEW, type Point, type Viewport } from "./geometry";

export function objectPosition(anchor: Point, viewport: Viewport, expanded: boolean, utility: boolean) {
  if (!utility) {
    const box = bubblePosition(anchor, viewport, expanded);
    // In a short stage, reserve the viewport for reading/input instead of keeping the character preview above it.
    const compact = expanded || viewport.height < VIEW.compactHeight;
    return compact ? { ...box, y: VIEW.edge, height: Math.max(0, viewport.height - VIEW.edge * 2) } : box;
  }
  const width = Math.max(0, Math.min(viewport.width - VIEW.edge * 2, expanded ? VIEW.expandedWidth : 720));
  const small = viewport.width < VIEW.desktop;
  const y = small ? (expanded || viewport.height < VIEW.compactHeight ? VIEW.edge : 80) : VIEW.edge;
  const height = Math.max(0, Math.min(viewport.height - y - VIEW.edge, expanded ? viewport.height : 720));
  return {
    width, height,
    x: small || expanded ? (viewport.width - width) / 2 : Math.max(VIEW.edge, Math.min(viewport.width - width - VIEW.edge, anchor.x + 42)),
    y: small || expanded ? y : Math.max(VIEW.edge, Math.min(viewport.height - height - VIEW.edge, anchor.y - height / 2)),
  };
}
