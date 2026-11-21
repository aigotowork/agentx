/** Presentation-only game office geometry and pose samples. The scene clock is their only time source. */
import type { HallOfficeRect, HallRoom } from "./model";
import type { HallFrame } from "./motion";
import type { Point } from "./geometry";

export const AMBIENCE_IMAGES = { "scene-vista": [800, 450], "scene-lantern": [64, 62] } as const;
export type AmbienceImage = keyof typeof AMBIENCE_IMAGES;
export type RoomLights = Readonly<Record<string, boolean>>;

export function roomLantern(room: HallRoom) {
  return { x: room.origin.x + 24, y: room.origin.y + 123 };
}

export function lanternInViewport(point: Point, viewport: { width: number; height: number }) {
  return point.x >= 0 && point.x <= viewport.width && point.y >= 24 && point.y <= viewport.height;
}

export function roomIsLit(lights: RoomLights | undefined, roomKey: string) {
  return lights?.[roomKey] !== false;
}

function phaseFor(key: string): number {
  let value = 0;
  for (const char of key) value = (value * 31 + char.charCodeAt(0)) % 997;
  return value / 997 * Math.PI * 2;
}

/** Material movement is subtle and foot anchored; pose still comes from the Agent frame. */
export function actorAccent(key: string, pose: HallFrame["pose"], elapsed: number, still: boolean) {
  if (still) return { sway: 0, rise: 0, stretch: 1, activity: 0 };
  const seed = phaseFor(key), idle = elapsed / 600 + seed, step = elapsed / 85 + seed;
  if (pose === "walk") return { sway: Math.sin(step) * .075, rise: Math.abs(Math.sin(step)) * 3.8, stretch: 1, activity: Math.max(0, Math.sin(step)) };
  if (pose === "sit") {
    const work = elapsed / 115 + seed;
    return { sway: Math.sin(work) * .019, rise: 0, stretch: 1 + Math.sin(work) * .021, activity: Math.max(0, Math.sin(work) - .48) * 1.8 };
  }
  const expressive = pose === "drink" || pose === "talk" || pose === "stretch";
  return { sway: Math.sin(idle) * (expressive ? .05 : .023), rise: expressive ? (1 + Math.sin(idle * 2)) * 2 : 0, stretch: 1 + Math.sin(idle) * .024, activity: 0 };
}

/** Inspector AABB for the same foot-anchored transform the painter applies. */
export function accentedBounds(bounds: HallOfficeRect, at: Point, sample: ReturnType<typeof actorAccent>): HallOfficeRect {
  const cos = Math.cos(sample.sway), sin = Math.sin(sample.sway);
  const corners = [0, bounds.width].flatMap(dx => [0, bounds.height].map(dy => {
    const x = bounds.x + dx - at.x, y = (bounds.y + dy - at.y) * sample.stretch;
    return { x: at.x + x * cos - y * sin, y: at.y - sample.rise + x * sin + y * cos };
  }));
  const xs = corners.map(point => point.x), ys = corners.map(point => point.y);
  const x = Math.min(...xs), y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}
