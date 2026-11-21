/** @aiPurpose Find bounded shortest indoor walks from the same room and furniture geometry used by scene art. */
import type { Point } from "./geometry";
import type { HallOfficeRect } from "./model";

/** @aiPurpose Describe one room's navigable feet coordinates and inflated furniture. @aiUsage Door is the only permitted boundary crossing; callers retain room-transfer semantics. */
export interface HallNavigation { bounds: HallOfficeRect; door: Point; obstacles: readonly HallOfficeRect[] }

function inside(point: Point, rect: HallOfficeRect): boolean { return point.x > rect.x && point.x < rect.x + rect.width && point.y > rect.y && point.y < rect.y + rect.height; }
function intersects(a: Point, b: Point, rect: HallOfficeRect): boolean {
  // Open rectangle: touching a visibility corner/edge is valid after clearance inflation.
  let low = 0, high = 1;
  for (const axis of ["x", "y"] as const) {
    const delta = b[axis] - a[axis], min = rect[axis], max = min + (axis === "x" ? rect.width : rect.height);
    if (Math.abs(delta) < 1e-9) { if (a[axis] <= min || a[axis] >= max) return false; }
    else { const first = (min - a[axis]) / delta, last = (max - a[axis]) / delta; low = Math.max(low, Math.min(first, last)); high = Math.min(high, Math.max(first, last)); }
  }
  return high - low > 1e-7 && high > 0 && low < 1;
}

/** @aiPurpose Prefer a direct visible segment, otherwise the shortest furniture-corner route inside one actual room. @aiUsage Supply projected geometry for production; absent geometry permits a direct walk for portable callers. */
export function indoorPath(from: Point, to: Point, navigation?: HallNavigation): Point[] {
  if (!navigation) return [from, to];
  const { bounds, obstacles, door } = navigation;
  const atDoor = (p: Point) => p.x === door.x && p.y === door.y;
  const innerDoor = { x: door.x, y: bounds.y + bounds.height };
  if (atDoor(from)) return [from, ...indoorPath(innerDoor, to, navigation)];
  if (atDoor(to)) return [...indoorPath(from, innerDoor, navigation), to];
  const within = (p: Point) => p.x >= bounds.x && p.x <= bounds.x + bounds.width && p.y >= bounds.y && p.y <= bounds.y + bounds.height;
  const clear = (a: Point, b: Point) => within(a) && within(b) && !obstacles.some(rect => intersects(a, b, rect));
  if (clear(from, to)) return [from, to];
  const points = [from, to, ...obstacles.flatMap(r => [{ x: r.x, y: r.y }, { x: r.x + r.width, y: r.y }, { x: r.x, y: r.y + r.height }, { x: r.x + r.width, y: r.y + r.height }]).filter(p => within(p) && !obstacles.some(r => inside(p, r)))];
  const distance = points.map(() => Infinity), previous = points.map(() => -1), visited = new Set<number>(); distance[0] = 0;
  for (;;) {
    let current = -1;
    for (let i = 0; i < points.length; i++) if (!visited.has(i) && (current < 0 || distance[i]! < distance[current]!)) current = i;
    if (current < 0 || !Number.isFinite(distance[current]!)) return [from]; // Unreachable must never tunnel through a wall.
    if (current === 1) break;
    visited.add(current);
    for (let next = 0; next < points.length; next++) {
      if (visited.has(next) || !clear(points[current]!, points[next]!)) continue;
      const candidate = distance[current]! + Math.hypot(points[next]!.x - points[current]!.x, points[next]!.y - points[current]!.y);
      if (candidate < distance[next]!) { distance[next] = candidate; previous[next] = current; }
    }
  }
  const route: Point[] = [];
  for (let index = 1; index >= 0; index = previous[index]!) route.unshift(points[index]!);
  return route;
}
