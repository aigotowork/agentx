/** Exact-identity actor movement and arrival memory; consumes time, never schedules it. */
import { worldToScreen, type Point } from "../geometry";
import { beginHallMotion, hallActorGoal, sampleHallMotion, type HallFrame, type HallMotion } from "../motion";
import type { HallActor } from "../model";
import type { ActorFrame, SceneInput } from "./contracts";

interface ActorMotion { seats: string; motion: HallMotion; frame: HallFrame }
export class ActorAnimation {
  private readonly motions = new Map<string, ActorMotion>();
  private readonly seenArrivals: Set<string>;

  constructor(input: SceneInput) {
    this.seenArrivals = new Set(input.scene.actors.flatMap(actor => actor.resource.arrival ? [actor.resource.arrival] : []));
  }

  sample(input: SceneInput, elapsed: number, still: boolean): readonly ActorFrame[] {
    const keys = new Set(input.scene.actors.map(actor => actor.key));
    for (const key of this.motions.keys()) if (!keys.has(key)) this.motions.delete(key);
    return input.scene.actors.map(actor => this.sampleActor(actor, input, elapsed, still));
  }

  /** Visibility restoration settles paths without advancing hidden decorative time. */
  restore(elapsed: number): void {
    for (const entry of this.motions.values()) entry.motion = beginHallMotion(entry.motion.to, entry.motion.to, elapsed);
  }

  point(key: string): Point | undefined { return this.motions.get(key)?.frame; }
  dispose(): void { this.motions.clear(); this.seenArrivals.clear(); }

  private sampleActor(actor: HallActor, input: SceneInput, elapsed: number, still: boolean): ActorFrame {
    const goal = hallActorGoal(actor);
    const seats = JSON.stringify([actor.home.x, actor.home.y, actor.rest.x, actor.rest.y]);
    let entry = this.motions.get(actor.key);
    // Reflow changes coordinates, not identity or office arrival semantics.
    if (!entry || entry.seats !== seats) {
      entry = this.initialMotion(actor, input, seats, !entry, elapsed, still);
      this.motions.set(actor.key, entry);
    }
    if (entry.motion.to.x !== goal.x || entry.motion.to.y !== goal.y) {
      entry.motion = beginHallMotion(entry.frame, goal, elapsed, actor.navigation);
    }
    if (still) entry.motion = beginHallMotion(goal, goal, elapsed);
    entry.frame = sampleHallMotion(actor, entry.motion, elapsed, still);
    if (input.selected?.key === actor.key) {
      entry.frame = { ...entry.frame, ...input.selected.point, pose: "idle", facing: "down" };
      entry.motion = beginHallMotion(input.selected.point, goal, elapsed, actor.navigation);
    }
    const screen = worldToScreen(entry.frame, input.camera), dim = !input.visibleKeys.has(actor.key);
    const hidden = dim || screen.x < -100 || screen.x > input.viewport.width + 100 || screen.y < -80 || screen.y > input.viewport.height + 80;
    return { actor, frame: entry.frame, screen, dim, hidden };
  }

  private initialMotion(actor: HallActor, input: SceneInput, seats: string, newActor: boolean, elapsed: number, still: boolean): ActorMotion {
    const goal = hallActorGoal(actor), arrival = actor.resource.arrival;
    const room = input.scene.rooms.find(room => room.actors.some(person => person.key === actor.key));
    const door = room?.office?.door;
    const inView = (point: Point) => {
      const screen = worldToScreen(point, input.camera);
      return screen.x >= 0 && screen.y >= 0 && screen.x <= input.viewport.width && screen.y <= input.viewport.height;
    };
    const enter = newActor && arrival && !this.seenArrivals.has(arrival) && input.visibleKeys.has(actor.key) && !still && door && inView(door) && inView(goal);
    if (arrival) this.seenArrivals.add(arrival);
    const motion = beginHallMotion(enter ? door : goal, goal, elapsed, actor.navigation);
    if (enter) {
      const duration = motion.legs.reduce((sum, leg) => sum + leg.durationMs, 0);
      if (duration > 1600) for (const leg of motion.legs) leg.durationMs *= 1600 / duration;
    }
    return { seats, motion, frame: sampleHallMotion(actor, motion, elapsed, still) };
  }
}
