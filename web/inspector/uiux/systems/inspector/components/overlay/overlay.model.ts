/** Pure geometry and ownership for one nonmodal tooltip layer. No SDK or UI dependencies. */
export interface OverlayRect { x: number; y: number; width: number; height: number }
export interface OverlaySize { width: number; height: number }
export interface TooltipGeometry { boundary: OverlayRect; anchor: OverlayRect }
export interface TooltipPlacement { left: number; top: number; width: number; height: number; side: "above" | "below" }
export interface TooltipMetrics { margin: number; gap: number; maxWidth: number; minAvailableWidth: number }

const validRect = (rect: OverlayRect) => [rect.x, rect.y, rect.width, rect.height].every(Number.isFinite) && rect.width > 0 && rect.height > 0;
export function tooltipWidthLimit(boundary: OverlayRect, metrics: TooltipMetrics): number {
  const available = Math.min(metrics.maxWidth, boundary.width - 2 * metrics.margin);
  return validRect(boundary) && available >= metrics.minAvailableWidth ? available : 0;
}
export function tooltipAnchorVisible({ boundary: b, anchor: a }: TooltipGeometry): boolean {
  return validRect(b) && validRect(a) && a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
}
/** Inputs share window coordinates; output belongs to the boundary's own coordinate system. */
export function placeTooltip(geometry: TooltipGeometry, size: OverlaySize, metrics: TooltipMetrics): TooltipPlacement | null {
  const { boundary: b, anchor: a } = geometry;
  const limit = tooltipWidthLimit(b, metrics), availableHeight = b.height - 2 * metrics.margin;
  if (!limit || !tooltipAnchorVisible(geometry) || !Number.isFinite(size.width) || !Number.isFinite(size.height) || size.width <= 0 || size.height <= 0 || availableHeight <= 0) return null;
  const width = Math.min(size.width, limit), height = Math.min(size.height, availableHeight);
  const leftEdge = a.x - b.x, topEdge = a.y - b.y;
  const below = b.height - metrics.margin - (topEdge + a.height + metrics.gap), above = topEdge - metrics.gap - metrics.margin;
  const side = below >= height || below >= above && above < height ? "below" : "above";
  const clamp = (value: number, low: number, high: number) => Math.max(low, Math.min(high, value));
  return {
    left: clamp(leftEdge + (a.width - width) / 2, metrics.margin, b.width - metrics.margin - width),
    top: clamp(side === "below" ? topEdge + a.height + metrics.gap : topEdge - metrics.gap - height, metrics.margin, b.height - metrics.margin - height),
    width, height, side,
  };
}

export interface TooltipSnapshot {
  owner: string; revision: number; label: string; geometry: TooltipGeometry | null; size: OverlaySize | null;
}
export type TooltipDismissReason = "leave" | "press" | "escape" | "scroll" | "layout" | "scope" | "replace" | "unmount" | "modal";
/**
 * A boundary owns at most one request. Every replacement/close invalidates pending measurements.
 * Old owners cannot dismiss a newer request. Suspension release is idempotent and never reopens one.
 */
export function createOverlaySession() {
  let snapshot: TooltipSnapshot | null = null, revision = 0, suspended = 0;
  let onDismiss: (() => void) | undefined;
  const listeners = new Set<() => void>();
  const emit = () => listeners.forEach(listener => listener());
  const current = (owner: string, expected: number) => snapshot?.owner === owner && snapshot.revision === expected;
  const dismiss = (_reason: TooltipDismissReason, owner?: string) => {
    if (!snapshot || owner !== undefined && snapshot.owner !== owner) return;
    revision++; snapshot = null;
    const callback = onDismiss; onDismiss = undefined;
    callback?.(); emit();
  };
  return {
    getSnapshot: () => snapshot,
    subscribe: (listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; },
    open(owner: string, label: string, dismissed: () => void): number | null {
      if (suspended || !label.trim()) return null;
      if (snapshot?.owner === owner && snapshot.label === label) return snapshot.revision;
      if (snapshot?.owner !== owner) dismiss("replace");
      snapshot = { owner, label, revision: ++revision, geometry: null, size: null };
      onDismiss = dismissed; emit(); return revision;
    },
    measure(owner: string, expected: number, geometry: TooltipGeometry): boolean {
      if (!current(owner, expected)) return false;
      if (!tooltipAnchorVisible(geometry)) { dismiss("layout", owner); return false; }
      snapshot = { ...snapshot!, geometry }; emit(); return true;
    },
    layout(owner: string, expected: number, size: OverlaySize): boolean {
      if (!current(owner, expected) || size.width <= 0 || size.height <= 0 || !Number.isFinite(size.width) || !Number.isFinite(size.height)) return false;
      if (snapshot!.size?.width === size.width && snapshot!.size?.height === size.height) return true;
      snapshot = { ...snapshot!, size }; emit(); return true;
    },
    dismiss,
    suspend(): () => void {
      suspended++; dismiss("modal");
      let released = false;
      return () => { if (!released) { released = true; suspended--; } };
    },
  };
}
export type OverlaySession = ReturnType<typeof createOverlaySession>;

export interface TooltipIntent { hovered: boolean; focused: boolean; held: boolean; suppressed: boolean }
export const initialTooltipIntent = (): TooltipIntent => ({ hovered: false, focused: false, held: false, suppressed: false });
export type TooltipIntentEvent = "hover-in" | "hover-out" | "focus" | "blur" | "hold" | "dismiss";
/** Hover/focus are independent. Dismissal requires a fresh entry before displaying again. */
export function tooltipIntent(state: TooltipIntent, event: TooltipIntentEvent): TooltipIntent {
  switch (event) {
    case "hover-in": return { ...state, hovered: true, suppressed: state.hovered ? state.suppressed : false };
    case "hover-out": return { ...state, hovered: false };
    case "focus": return { ...state, focused: true, suppressed: state.focused ? state.suppressed : false };
    case "blur": return { ...state, focused: false };
    case "hold": return { ...state, held: true, suppressed: false };
    case "dismiss": return { ...state, held: false, suppressed: true };
  }
}
export const tooltipWanted = (state: TooltipIntent) => !state.suppressed && (state.hovered || state.focused || state.held);
