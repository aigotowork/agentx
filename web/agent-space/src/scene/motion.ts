/** @aiPurpose Sample ordinary-Agent walking and rest poses as platform-neutral data; animation never controls execution. */
import { type Leg } from "./art/walk";
import type { CharacterPose, Facing } from "./art/pixel/characters";
import type { AgentResource } from "../offline/model";
import type { HallActor } from "./model";
import type { Point } from "./geometry";
import { indoorPath, type HallNavigation } from "./navigation";
export interface HallMotion { from: Point; to: Point; started: number; legs: Leg[] }
export interface HallFrame extends Point { pose: CharacterPose; facing: Facing; carry?: "book" | "cup" }
/** @aiPurpose Project a resource's work/rest seat without altering the resource.
 * @aiUsage Keep the actor's layout from the full authorized roster, independent of visual filters.
 */
export type SceneBehavior = "working" | "listening" | "sleeping" | "blocked" | "request-error" | "error" | "launching" | "unknown";
export function sceneBehavior(resource: AgentResource): SceneBehavior {
  const status = resource.nativeStatus;
  if (status === "active" || status === "working" || !status && resource.state === "running") return "working";
  if (status === "listening" || status === "idle" || status === "ready" || !status && (resource.state === "idle" || resource.state === "ready")) return "listening";
  if (status === "inactive") return resource.statusContext === "launch_failed" ? "error" : "sleeping";
  if (status === "blocked") return "blocked";
  if (status === "error" || status === "failed" || !status && resource.state === "failed") {
    // Native effective context simplifies model:request to model; the session-fenced
    // request fact identifies its source, while context keeps other failures primary.
    const requestFailure = resource.requestError?.source === "model_request" && (resource.statusContext === "model" || resource.statusContext === "model:request");
    return requestFailure ? "request-error" : "error";
  }
  return status === "launching" || status === "pending" ? "launching" : "unknown";
}
export function hallActorGoal(actor: HallActor): Point { return sceneBehavior(actor.resource) === "listening" ? actor.rest : actor.home; }
function activitySeed(key: string): number { let hash = 0; for (const char of key) hash = (hash * 31 + char.charCodeAt(0)) >>> 0; return hash; }
function settledFrame(actor: HallActor, point: Point, now: number, still: boolean): HallFrame {
  const behavior = sceneBehavior(actor.resource);
  if (behavior === "working") return { ...point, pose: "sit", facing: "up" };
  if (behavior === "sleeping") return { ...point, pose: "doze", facing: "down" };
  if (behavior !== "listening") return { ...point, pose: "idle", facing: "down" };
  const seed = activitySeed(actor.key), period = 36000 + seed % 16000, duration = 4000 + seed % 2000;
  const cycle = Math.floor((now + seed % period) / period), phase = (now + seed % period) % period;
  const poses = ["drink", "stretch", "idle", "talk"] as const;
  const pose = still || phase > duration ? "idle" : poses[(seed + cycle) % poses.length]!;
  return { ...point, pose, facing: "down", ...(pose === "drink" ? { carry: "cup" as const } : {}) };
}
/** @aiPurpose Route through actual room geometry while retaining the existing Leg sampler and clock.
 * @aiUsage Supply the shared animation clock in milliseconds, paused when decoration is hidden.
 */
export function beginHallMotion(from: Point, to: Point, started: number, navigation?: HallNavigation): HallMotion {
  const path = indoorPath(from, to, navigation), legs: Leg[] = [];
  for (let index = 1; index < path.length; index++) {
    const a = path[index - 1]!, b = path[index]!, dx = b.x - a.x, dy = b.y - a.y;
    const length = Math.hypot(dx, dy); if (!length) continue;
    legs.push({ from: { x: a.x / 16, y: a.y / 16 }, to: { x: b.x / 16, y: b.y / 16 }, heading: Math.abs(dx) > Math.abs(dy) ? dx > 0 ? "right" : "left" : dy > 0 ? "down" : "up", durationMs: length / 16 * 260, row: Math.max(a.y, b.y) / 16 });
  }
  const duration = legs.reduce((sum, leg) => sum + leg.durationMs, 0);
  if (duration > 9000) for (const leg of legs) leg.durationMs *= 9000 / duration;
  return { from, to: path[path.length - 1] ?? from, started, legs };
}

/** @aiPurpose Sample a shared walk or a stable resource pose; reduced motion reaches the destination immediately.
 * @aiUsage Selected actors may be visually held at their current point while their object body is open.
 */
export function sampleHallMotion(actor: HallActor, motion: HallMotion, now: number, still: boolean): HallFrame {
  let elapsed = now - motion.started;
  if (!still) for (const leg of motion.legs) {
    if (elapsed < leg.durationMs) { const p = Math.max(0, elapsed / leg.durationMs); return { x: (leg.from.x + (leg.to.x - leg.from.x) * p) * 16, y: (leg.from.y + (leg.to.y - leg.from.y) * p) * 16, pose: "walk", facing: leg.heading }; }
    elapsed -= leg.durationMs;
  }
  return settledFrame(actor, motion.to, now, still);
}
