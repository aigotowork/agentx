/** One mounted scene's clock, sampled frame and lifecycle. No React or drawing state lives here. */
import type { Point } from "../geometry";
import { ActorAnimation } from "./actors";
import { MessageEffects } from "./messages";
import { sampleCat } from "../cat";
import type { SceneEnvironment, SceneFrame, SceneInput } from "./contracts";

export class SceneRuntime {
  private readonly actors: ActorAnimation;
  private readonly messages: MessageEffects;
  private state: "created" | "started" | "disposed" = "created";
  private pending: number | null = null;
  private unsubscribe: (() => void) | null = null;
  private previous: number | null = null;
  private elapsed = 0;
  private lastDraw = 0;
  private readonly catSeed = Math.random();

  constructor(private input: SceneInput, private readonly environment: SceneEnvironment, private readonly present: (frame: SceneFrame) => void) {
    this.actors = new ActorAnimation(input);
    this.messages = new MessageEffects(input);
  }

  start(): void {
    if (this.state !== "created") return;
    this.state = "started";
    this.unsubscribe = this.environment.subscribe(reason => {
      if (this.state !== "started") return;
      if (reason === "visibility") {
        this.messages.restore(this.input.messages ?? []);
        this.actors.restore(this.elapsed);
      }
      this.wake();
    });
    this.wake();
  }

  update(input: SceneInput): void {
    if (this.state === "disposed") return;
    this.input = input;
    if (this.state === "started") this.wake();
  }

  point(key: string): Point | undefined { return this.actors.point(key); }

  dispose(): void {
    if (this.state === "disposed") return;
    this.state = "disposed";
    this.cancelPending();
    this.unsubscribe?.();
    this.unsubscribe = null;
    this.previous = null;
    this.actors.dispose();
    this.messages.dispose();
  }

  private cancelPending(): void {
    if (this.pending !== null) this.environment.cancelFrame(this.pending);
    this.pending = null;
  }

  private wake(): void {
    this.cancelPending();
    this.previous = null;
    this.pending = this.environment.requestFrame(this.tick);
  }

  private readonly tick = (time: number): void => {
    this.pending = null;
    if (this.state !== "started" || this.environment.hidden()) { this.previous = null; return; }
    const still = this.environment.reducedMotion() || this.input.paused;
    if (this.previous !== null && !still) this.elapsed += Math.min(100, time - this.previous);
    this.previous = time;
    if (time - this.lastDraw >= 40 || still) {
      this.lastDraw = time;
      const actors = this.actors.sample(this.input, this.elapsed, still);
      const flights = this.messages.sample(this.input, this.elapsed, this.environment.wallNow(), still);
      const cat = sampleCat(this.input.scene, this.elapsed, this.catSeed, still || !!this.input.humanOpen);
      this.present({ input: this.input, actors, flights, cat, elapsed: this.elapsed, still });
    }
    if (!still && this.state === "started") this.pending = this.environment.requestFrame(this.tick);
  };
}

/** Browser handles are acquired only at composition; subscribe returns the exact listener cleanup. */
export function browserSceneEnvironment(): SceneEnvironment {
  const preference = matchMedia("(prefers-reduced-motion: reduce)");
  return {
    requestFrame: callback => requestAnimationFrame(callback),
    cancelFrame: handle => cancelAnimationFrame(handle),
    wallNow: () => Date.now(),
    hidden: () => document.hidden,
    reducedMotion: () => preference.matches,
    subscribe(changed) {
      const motion = () => changed("motion"), visibility = () => changed("visibility");
      preference.addEventListener("change", motion);
      document.addEventListener("visibilitychange", visibility);
      return () => {
        preference.removeEventListener("change", motion);
        document.removeEventListener("visibilitychange", visibility);
      };
    },
  };
}
