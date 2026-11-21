/** Studio asset placements shared by Canvas and inspection. World dimensions and anchors follow art/studio/AGENTS.md. */
import type { AgentResource } from "../offline/model";
import type { Camera, Point, Viewport } from "./geometry";
import type { HallOfficeRect, HallRoom, HallScene } from "./model";
import { sceneBehavior, type HallFrame } from "./motion";
import type { HallPropAnchor } from "./runtime/contracts";

const ACTOR = [64, 72, .5, 1] as const;
const GLYPH = [24, 24, .5, .5] as const;
export const STUDIO_IMAGE = {
  "floor-oak": [128, 128, 0, 0],
  "wall-plaster": [160, 64, 0, 0],
  "wall-window": [120, 48, 0, 0],
  "wall-print": [54, 48, 0, 0],
  "door-threshold": [64, 32, .5, 1],
  rug: [192, 112, 0, 0],
  desk: [86, 68, .5, 1],
  stool: [38, 36, .5, 1],
  coffee: [54, 62, .5, 1],
  sofa: [108, 60, .5, 1],
  bookshelf: [56, 64, .5, 1],
  cabinet: [68, 64, .5, 1],
  plant: [32, 46, .5, 1],
  envelope: [24, 16, .5, .5],
  "human-mailbox": [62, 62, .5, 1],
  "actor-maker": ACTOR, "actor-designer": ACTOR, "actor-keeper": ACTOR, "actor-scout": ACTOR,
  "glyph-work": GLYPH, "glyph-available": GLYPH, "glyph-attention": GLYPH,
  "glyph-select": [64, 24, .5, .5],
} as const;
export const STUDIO_ACTOR_SIZE = { width: ACTOR[0], height: ACTOR[1] } as const;
export const STUDIO_TRIM = 6;
export type StudioAsset = keyof typeof STUDIO_IMAGE;
export interface StudioImage { asset: StudioAsset; at: Point; bounds: HallOfficeRect; mirror: boolean }

export function studioImage(asset: StudioAsset, at: Point, scaleX = 1, scaleY = scaleX, mirror = false): StudioImage {
  const [w, h, anchorX, anchorY] = STUDIO_IMAGE[asset];
  const width = w * scaleX, height = h * scaleY;
  return { asset, at, bounds: { x: at.x - width * anchorX, y: at.y - height * anchorY, width, height }, mirror };
}

export function studioCast(resource: AgentResource): "actor-maker" | "actor-designer" | "actor-keeper" | "actor-scout" {
  const role = resource.role.kind === "adopted" ? resource.role.name : "";
  return role === "designer" ? "actor-designer" : role === "keeper" ? "actor-keeper" : role === "scout" ? "actor-scout" : "actor-maker";
}
export type StudioSignal = "work" | "available" | "attention";
const signalLabels: Record<StudioSignal, string> = { work: "工作中", available: "可交谈", attention: "待关注" };
export function studioSignal(resource: AgentResource): StudioSignal | null {
  const behavior = sceneBehavior(resource);
  if (resource.attention || behavior === "blocked" || behavior === "request-error" || behavior === "error") return "attention";
  if (behavior === "working") return "work";
  return behavior === "listening" ? "available" : null;
}
export function studioSignalLabel(resource: AgentResource) {
  const signal = studioSignal(resource);
  return signal ? signalLabels[signal] : null;
}

export function studioActorArt(resource: AgentResource, frame: HallFrame, elapsed: number, still: boolean, selected: boolean) {
  const at = { x: frame.x, y: frame.y };
  const walking = frame.pose === "walk", mirrored = walking && frame.facing === "left";
  const compression = frame.pose === "sit" ? .87 : frame.pose === "doze" ? .82 : 1;
  const phase = walking && !still ? Math.floor(elapsed / 140) % 4 : 0;
  const lift = [0, .035, 0, -.025][phase]!;
  const signal = studioSignal(resource);
  return {
    body: studioImage(studioCast(resource), at, 1, compression + lift, mirrored),
    signal: signal ? studioImage(`glyph-${signal}`, { x: at.x + 32, y: at.y - 43 }) : null,
    selection: selected ? studioImage("glyph-select", at) : null,
    animation: { pose: frame.pose, facing: frame.facing, carry: frame.carry ?? null, phase, mirrored, paused: still },
  };
}

export const studioDeskAt = (home: Point): Point => ({ x: home.x, y: home.y - 10 });
export const studioStoolAt = (home: Point): Point => ({ x: home.x, y: home.y + 16 });
export function studioFixtureArt(prop: string, at: Point): StudioImage | null {
  return prop === "coffee" || prop === "sofa" || prop === "bookshelf" || prop === "plant" ? studioImage(prop, at) : null;
}
/** Use contact-depth obstacles, not the height of the drawn facade, for the actor's feet. */
export function studioFootprint(art: StudioImage, depth: number, clearance = 4): HallOfficeRect {
  return { x: art.bounds.x - clearance, y: art.at.y - depth, width: art.bounds.width + 2 * clearance, height: depth + clearance };
}
export function studioPropArt(item: HallPropAnchor): StudioImage {
  const kind = item.id === "cabinet" ? "cabinet" : item.id === "workspaces" ? "bookshelf" : "desk";
  return studioImage(kind, { x: item.x, y: item.y });
}
export function studioRoomArt(room: HallRoom): { door: StudioImage; rug: StudioImage | null; wall: StudioImage[]; fixtures: { id: string; prop: string; image: StudioImage }[] } {
  const { origin, width, height, office } = room;
  const artDoor = studioImage("door-threshold", office?.door ?? { x: origin.x + 64, y: origin.y + height });
  const lounge = office?.lounge;
  const rug = lounge?.width && lounge.height ? studioImage("rug", { x: lounge.x + 8, y: lounge.y + 80 }, Math.min(1, (lounge.width - 16) / STUDIO_IMAGE.rug[0])) : null;
  const wall = [
    studioImage("wall-window", { x: origin.x + (width > 320 ? 27 : 17), y: origin.y + 1 }),
    ...(width > 320 ? [
      studioImage("wall-print", { x: origin.x + width / 2 - 27, y: origin.y + 1 }),
      studioImage("wall-window", { x: origin.x + width - 150, y: origin.y + 1 }),
    ] : []),
  ];
  const fixtures = (office?.fixtures ?? []).flatMap(fixture => {
    const image = studioFixtureArt(fixture.prop, fixture.at);
    return image ? [{ id: fixture.id, prop: fixture.prop, image }] : [];
  });
  return { door: artDoor, rug, wall, fixtures };
}

/** Tight entry for the focused office. Keep the same readable zoom floor and existing camera owner. */
export function studioEntryCamera(scene: HallScene, viewport: Viewport): Camera {
  const bounds = scene.rooms.length === 1 ? scene.rooms[0]! : null;
  const margin = 18, controls = 58, width = bounds?.width ?? scene.width;
  const height = bounds ? bounds.origin.y + bounds.height : scene.height;
  const zoom = Math.max(.75, Math.min(1.12, (viewport.width - 2 * margin) / width, (viewport.height - controls - margin) / height));
  return { zoom, x: (viewport.width - width * zoom) / 2 - (bounds?.origin.x ?? 0) * zoom, y: 12 };
}
