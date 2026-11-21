/** @aiPurpose Share platform-free scene coordinates and readable object anchoring between DOM and RN. */
export interface Point { x: number; y: number }
export interface Camera { x: number; y: number; zoom: number }
export interface Viewport { width: number; height: number }
export const VIEW = { edge: 12, desktop: 760, bubbleWidth: 520, bubbleHeight: 620, compactHeight: 420, expandedWidth: 920, minimumZoom: .75, maximumZoom: 4 } as const;
export function worldToScreen(point: Point, camera: Camera): Point { return { x: point.x * camera.zoom + camera.x, y: point.y * camera.zoom + camera.y }; }
export function overview(world: { width: number; height: number }, viewport: Viewport): Camera {
  const zoom = Math.max(VIEW.minimumZoom, Math.min(2, (viewport.width - VIEW.edge * 2) / world.width, (viewport.height - VIEW.edge * 2) / world.height));
  return { zoom, x: (viewport.width - world.width * zoom) / 2, y: VIEW.edge };
}
/** Move the selected person beside its reader without changing the user's world scale. */
export function focusCamera(point: Point, viewport: Viewport, camera: Camera): Camera {
  const zoom = camera.zoom;
  return viewport.width < VIEW.desktop ? { zoom, x: viewport.width / 2 - point.x * zoom, y: 80 - point.y * zoom }
    : { zoom, x: Math.max(96, (viewport.width - Math.min(VIEW.bubbleWidth, viewport.width * .55)) / 2) - point.x * zoom, y: viewport.height * .46 - point.y * zoom };
}

/** @aiPurpose Retain a selected object's readable anchor or an unselected view's world center across viewport changes. @aiUsage Resize both the displayed camera and its saved return view; never change route or draft identity. */
export function resizeCamera(camera: Camera, before: Viewport, after: Viewport, selected?: Point): Camera {
  return selected ? focusCamera(selected, after, camera) : { ...camera, x: camera.x + (after.width - before.width) / 2, y: camera.y + (after.height - before.height) / 2 };
}
/** Preserve the return zoom; reframe only axes where the selected landmark is clipped after a resize. */
export function recoverReturnAnchor(camera: Camera, point: Point, viewport: Viewport): Camera {
  const screen = worldToScreen(point, camera);
  const side = Math.min(40, viewport.width / 4);
  const top = Math.min(64 * camera.zoom, viewport.height / 3);
  const bottom = Math.min(76, viewport.height / 3);
  return { ...camera,
    x: screen.x >= side && screen.x <= viewport.width - side ? camera.x : camera.x + viewport.width / 2 - screen.x,
    y: screen.y >= top && screen.y <= viewport.height - bottom ? camera.y : camera.y + viewport.height / 2 - screen.y,
  };
}
export function zoomCamera(camera: Camera, factor: number, center: Point): Camera {
  const zoom = Math.max(VIEW.minimumZoom, Math.min(VIEW.maximumZoom, camera.zoom * factor));
  const ratio = zoom / camera.zoom;
  return { zoom, x: center.x - (center.x - camera.x) * ratio, y: center.y - (center.y - camera.y) * ratio };
}
export function bubblePosition(anchor: Point, viewport: Viewport, expanded: boolean): { x: number; y: number; width: number; height: number } {
  const small = viewport.width < VIEW.desktop;
  const width = Math.max(0, Math.min(viewport.width - VIEW.edge * 2, expanded ? VIEW.expandedWidth : VIEW.bubbleWidth));
  // Leave room below the character's feet for its name and two-line status badge.
  const y = small ? Math.min(Math.max(VIEW.edge, viewport.height - VIEW.edge), Math.max(76, anchor.y + VIEW.edge * 6)) : VIEW.edge;
  const height = Math.max(0, Math.min(viewport.height - y - VIEW.edge, expanded ? viewport.height : VIEW.bubbleHeight));
  const x = small || expanded ? (viewport.width - width) / 2 : Math.max(VIEW.edge, Math.min(viewport.width - width - VIEW.edge, anchor.x + 42));
  return { x, y, width, height };
}
