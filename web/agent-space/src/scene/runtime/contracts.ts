/** Private scene presentation contracts: milliseconds and world/screen coordinates stay explicit. */
import type { SceneMessage } from "../../offline/model";
import type { Camera, Point, Viewport } from "../geometry";
import type { HallActor, HallScene } from "../model";
import type { HallFrame } from "../motion";
import type { CatFrame } from "../cat";
import type { RoomLights } from "../ambience";

export type HallProp = "roster" | "workspaces" | "cabinet" | "information";
export interface HallPropAnchor extends Point { id: HallProp; title: string; roomKey?: string }
export interface SceneInput {
  scene: HallScene;
  camera: Camera;
  viewport: Viewport;
  visibleKeys: ReadonlySet<string>;
  paused: boolean;
  selected?: { key: string; point: Point };
  props: readonly HallPropAnchor[];
  messages?: readonly SceneMessage[];
  humanMessages: number | bigint;
  humanOpen?: boolean;
  roomLights?: RoomLights;
}
export interface ActorFrame {
  readonly actor: HallActor;
  readonly frame: HallFrame;
  readonly screen: Point;
  readonly dim: boolean;
  readonly hidden: boolean;
}
export interface MessageFlight { readonly event: SceneMessage; readonly started: number }
export interface SceneFrame {
  readonly cat: CatFrame;
  readonly input: SceneInput;
  readonly actors: readonly ActorFrame[];
  readonly flights: readonly MessageFlight[];
  readonly elapsed: number;
  readonly still: boolean;
}

/** Only the clock and environment signals needed by this scene; deterministic hosts can drive cases. */
export interface SceneEnvironment {
  requestFrame(callback: (time: number) => void): number;
  cancelFrame(handle: number): void;
  wallNow(): number;
  hidden(): boolean;
  reducedMotion(): boolean;
  subscribe(changed: (reason: "visibility" | "motion") => void): () => void;
}
