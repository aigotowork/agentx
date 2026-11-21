/** Decorative cat values, sampled by the existing scene clock. No mail or Agent state. */
import type { Point } from "./geometry";
import type { HallScene } from "./model";

export interface CatFrame {
  readonly at: Point;
  readonly activity: "rest" | "jump" | "patrol" | "yarn";
  readonly facing: "left" | "right";
  readonly pose: number;
  readonly lift: number;
  readonly yarn: Point | null;
}
const CYCLE = 22000;
function random(seed: number, cycle: number, salt: number): number {
  let value = (Math.floor(seed * 0xffffffff) ^ Math.imul(cycle + 1, 0x45d9f3b) ^ salt) >>> 0;
  value ^= value >>> 16; value = Math.imul(value, 0x45d9f3b); value ^= value >>> 16;
  return (value >>> 0) / 0x100000000;
}
/** All patrol feet and sprites remain in the open entrance strip, before the first room wall.
 * Rooms currently have south-only doors and no exterior connecting aisle, so a fictional indoor
 * shortcut would cross walls. Extend geometry/navigation before allowing room entry.
 */
export function sampleCat(scene: HallScene, elapsed: number, seed: number, still: boolean): CatFrame {
  const home = scene.human;
  const resting: CatFrame = { at: home, activity: "rest", facing: "left", pose: 0, lift: 0, yarn: null };
  if (still) return resting;
  const cycle = Math.floor(Math.max(0, elapsed) / CYCLE), time = Math.max(0, elapsed) % CYCLE;
  const delay = 8000 + random(seed, cycle, 1) * 5000;
  const duration = 4200 + random(seed, cycle, 2) * 2600;
  const progress = (time - delay) / duration;
  if (progress <= 0 || progress >= 1) return resting;
  const kind = Math.floor(random(seed, cycle, 3) * 3);
  const pose = Math.floor(time / 160) % 4;
  if (kind === 0) {
    const hop = Math.sin(progress * Math.PI * 3);
    return { ...resting, activity: "jump", pose, lift: Math.max(0, hop) * 24 };
  }
  const roomTop = Math.min(scene.height, ...scene.rooms.map(room => room.origin.y));
  const reach = Math.min(180, scene.width - home.x - 48);
  if (roomTop < home.y + 34 || reach < 36) return resting;
  if (kind === 1) {
    const travel = Math.sin(progress * Math.PI) ** 2;
    return { ...resting, activity: "patrol", at: { x: home.x + reach * travel, y: home.y + Math.sin(progress * Math.PI * 2) * 12 },
      facing: progress < .5 ? "right" : "left", pose, lift: pose % 2 ? 2 : 0 };
  }
  const reachYarn = Math.min(56, reach);
  const chase = Math.sin(progress * Math.PI) ** 2;
  return { ...resting, activity: "yarn", at: { x: home.x + reachYarn * chase, y: home.y },
    facing: progress < .5 ? "right" : "left", pose, lift: pose === 1 ? 3 : 0,
    yarn: { x: home.x + 26 + reachYarn * chase + Math.sin(progress * Math.PI * 8) * 5, y: home.y - 2 } };
}
