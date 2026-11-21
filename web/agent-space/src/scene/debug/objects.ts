/** On-demand semantic projection of a presented frame, including furniture baked into room bitmaps. */
import { prop, emote } from "../art/art";
import { worldToScreen, type Point } from "../geometry";
import { hallActorGoal, sceneBehavior } from "../motion";
import { humanCatNest, humanYarnBall } from "../office-art";
import { actorAnimation, actorSprite, attentionGlyph, spriteBounds } from "../runtime/sprite";
import { studioActorArt, studioDeskAt, studioImage, studioPropArt, studioRoomArt, studioStoolAt, type StudioImage } from "../studio-art";
import { HUMAN_ENDPOINT } from "../../offline/model";
import { accentedBounds, actorAccent, lanternInViewport, roomIsLit, roomLantern } from "../ambience";
import type { SpaceGameWorld } from "../../../uiux/systems/space/components/presentation";
import type { SceneFrame } from "../runtime/contracts";
import type { CanvasMetrics, DebugObject, Geometry, Json, ObjectKind, Rect } from "./contracts";

export function objectId(kind: ObjectKind, ...keys: string[]): string { return JSON.stringify([kind, ...keys]); }
function intersect(a: Rect, b: Rect): Rect {
  const x = Math.max(a.x, b.x), y = Math.max(a.y, b.y);
  return { x, y, width: Math.max(0, Math.min(a.x + a.width, b.x + b.width) - x), height: Math.max(0, Math.min(a.y + a.height, b.y + b.height) - y) };
}
export function sceneObjects(sample: SceneFrame, metrics: CanvasMetrics, dom: (key: string) => HTMLElement | undefined, game?: SpaceGameWorld): DebugObject[] {
  const { scene, camera, viewport, visibleKeys } = sample.input;
  const studio = scene.art === "studio";
  const svgBounds = "SVG image box including transparent padding";
  const objects: DebugObject[] = [];
  const roomFor = (key: string) => scene.rooms.find(room => room.actors.some(actor => actor.key === key));
  const inDrawMargin = (point: Point) => {
    const screen = worldToScreen(point, camera);
    return screen.x >= -40 * camera.zoom && screen.x <= viewport.width + 40 * camera.zoom && screen.y >= -40 * camera.zoom && screen.y <= viewport.height + 40 * camera.zoom;
  };
  const inCanvas = (point: Point) => {
    const screen = worldToScreen(point, camera);
    return screen.x >= 0 && screen.y >= 0 && screen.x <= viewport.width && screen.y <= viewport.height;
  };
  function geometry(at: Point | null, bounds: Rect | null, meaning: string, domKey?: string): Geometry {
    const canvas = at ? worldToScreen(at, camera) : null;
    const toClient = (p: Point) => ({ x: metrics.client.x + p.x * metrics.canvasToClient.x, y: metrics.client.y + p.y * metrics.canvasToClient.y });
    const toPage = (p: Point) => ({ x: metrics.page.x + p.x * metrics.canvasToClient.x, y: metrics.page.y + p.y * metrics.canvasToClient.y });
    const toBacking = (p: Point) => ({ x: p.x * metrics.cssToBacking.x, y: p.y * metrics.cssToBacking.y });
    const canvasBounds = bounds ? { ...worldToScreen(bounds, camera), width: bounds.width * camera.zoom, height: bounds.height * camera.zoom } : null;
    const node = domKey ? dom(domKey) : undefined, box = node?.getBoundingClientRect();
    const offset = { x: metrics.page.x - metrics.client.x, y: metrics.page.y - metrics.client.y };
    return {
      anchor: at && canvas ? { world: { x: at.x, y: at.y }, canvas, client: toClient(canvas), page: toPage(canvas), backing: toBacking(canvas) } : null,
      bounds: canvasBounds && bounds ? {
        world: bounds, canvas: canvasBounds,
        client: { ...toClient(canvasBounds), width: canvasBounds.width * metrics.canvasToClient.x, height: canvasBounds.height * metrics.canvasToClient.y },
        page: { ...toPage(canvasBounds), width: canvasBounds.width * metrics.canvasToClient.x, height: canvasBounds.height * metrics.canvasToClient.y },
        backing: { ...toBacking(canvasBounds), width: canvasBounds.width * metrics.cssToBacking.x, height: canvasBounds.height * metrics.cssToBacking.y },
      } : null,
      boundsMeaning: meaning,
      dom: box ? { client: { x: box.x, y: box.y, width: box.width, height: box.height }, page: { x: box.x + offset.x, y: box.y + offset.y, width: box.width, height: box.height } } : null,
    };
  }
  function add(kind: ObjectKind, id: string, label: string, parentId: string | null, at: Point | null, bounds: Rect | null, details: Record<string, Json>, options: {
    rendering?: DebugObject["rendering"]; drawn?: boolean; filtered?: boolean; reason?: string; domKey?: string; meaning?: string; actions?: string[]; related?: string[];
  } = {}) {
    const placement = geometry(at, bounds, options.meaning ?? "Sprite image box including transparent padding", options.domKey);
    const box = placement.bounds?.canvas;
    const outside = box && (box.width === 0 || box.height === 0 || box.x + box.width <= 0 || box.y + box.height <= 0 || box.x >= viewport.width || box.y >= viewport.height);
    const inside = box && box.x >= 0 && box.y >= 0 && box.x + box.width <= viewport.width && box.y + box.height <= viewport.height;
    objects.push({
      id, kind, label, parentId, relatedIds: options.related ?? [], geometry: placement,
      visibility: { filtered: options.filtered ?? false, drawn: options.drawn ?? true, canvasIntersection: !box ? "unknown" : outside ? "outside" : inside ? "inside" : "partial", reason: options.reason ?? null },
      rendering: options.rendering ?? "sprite", actionIds: options.actions ?? [], details,
    });
  }
  for (const room of scene.rooms) {
    const id = objectId("room", room.key), bounds = { ...room.origin, width: room.width, height: room.height };
    add("room", id, room.title, null, room.origin, bounds, { key: room.key, ...(studio ? { art: "studio" } : {}) }, studio
      ? { rendering: "room-composite", meaning: "Room model bounds and clip of the tiled SVG scene, excluding exterior shadow" }
      : { rendering: "room-bitmap", meaning: "Room model bounds; bitmap dimensions round up to whole art pixels" });
    if (game) {
      const at = roomLantern(room), key = objectId("prop", room.key, "lantern");
      const domTargetHidden = !lanternInViewport(worldToScreen(at, camera), viewport);
      add("prop", key, "场景灯", id, at, intersect({ x: at.x - 32, y: at.y - 62, width: 64, height: 62 }, bounds),
        { assetId: "scene-lantern", roomKey: room.key, lit: roomIsLit(sample.input.roomLights, room.key), domTargetHidden },
        { domKey: key, meaning: "Lamp SVG clipped to room; radiance is not a target", actions: ["object.activate"] });
    }
    if (room.office) {
      for (const zone of ["work", "lounge"] as const) {
        const rect = room.office[zone];
        add("zone", objectId("zone", room.key, zone), zone, id, rect, rect, { zone }, { rendering: "model-only", drawn: false, reason: studio ? "Geometric zone within the room composition" : "Geometric zone; textures are part of the room bitmap", meaning: "Model geometry" });
      }
      if (studio) {
        const art = studioRoomArt(room);
        const artPart = (kind: "furniture" | "effect", key: string, label: string, image: StudioImage, details: Record<string, Json>) => {
          add(kind, objectId(kind, room.key, key), label, id, image.at, intersect(image.bounds, bounds), { assetId: image.asset, ...details }, { meaning: "SVG image box clipped to the room" });
        };
        artPart("effect", "door", "Door threshold", art.door, { decorative: true, use: "modeled doorway" });
        if (art.rug) artPart("effect", "rug", "Lounge rug", art.rug, { decorative: true });
        for (const fixture of art.fixtures) artPart("furniture", fixture.id, fixture.prop, fixture.image, { prop: fixture.prop, fixtureId: fixture.id });
      } else for (const fixture of room.office.fixtures) {
        const at = { x: room.origin.x + Math.round((fixture.at.x - room.origin.x) / 2) * 2, y: room.origin.y + Math.round((fixture.at.y - room.origin.y) / 2) * 2 };
        const bitmapBounds = { ...room.origin, width: Math.ceil(room.width / 2) * 2, height: Math.ceil(room.height / 2) * 2 };
        const box = intersect(spriteBounds(prop(fixture.prop), at.x, at.y), bitmapBounds);
        add("furniture", objectId("furniture", room.key, fixture.id), fixture.prop, id, at, box, { prop: fixture.prop, fixtureId: fixture.id }, { rendering: "room-bitmap", meaning: "Image box after room-local rounding and composite clipping" });
      }
    }
  }
  add("human", objectId("human", HUMAN_ENDPOINT), "@bigboss", null, scene.human, studio ? studioImage("human-mailbox", scene.human).bounds : spriteBounds(humanCatNest("nest"), scene.human.x, scene.human.y), { endpoint: HUMAN_ENDPOINT, unhandledMessages: typeof sample.input.humanMessages === "bigint" ? String(sample.input.humanMessages) : sample.input.humanMessages, deliveryAcknowledgement: null, activity:'human-mailbox',open:sample.input.humanOpen??false,
    ...(studio ? { assetId: "human-mailbox" } : { animation:{activity:sample.cat.activity,pose:sample.cat.pose,facing:sample.cat.facing,lift:sample.cat.lift,feet:{...sample.cat.at},yarn:sample.cat.yarn?{...sample.cat.yarn}:null,paused:sample.still,decorative:true,meaning:'Human endpoint uses a static nest and a separate decorative cat'} }),
  }, { domKey: objectId("human", HUMAN_ENDPOINT), actions: ["object.activate"], ...(studio ? { meaning: svgBounds } : { related: [objectId("effect", HUMAN_ENDPOINT, "cat")] }) });
  if (sample.input.humanMessages) add("effect", objectId("effect", HUMAN_ENDPOINT, "unhandled-letters"), "Human unhandled-message letters", objectId("human", HUMAN_ENDPOINT), scene.human, null, { unhandledMessages: typeof sample.input.humanMessages === "bigint" ? String(sample.input.humanMessages) : sample.input.humanMessages, individualPixelBounds: null }, { rendering: "aggregate", meaning: "Individual letter image bounds not exposed" });
  if (!studio) {
    const cat = sample.cat;
    const catPoint = { x: cat.at.x, y: cat.at.y - cat.lift };
    add("effect", objectId("effect", HUMAN_ENDPOINT, "cat"), "Decorative cat", objectId("human", HUMAN_ENDPOINT), catPoint,
      spriteBounds(humanCatNest("cat", cat.pose, cat.facing), catPoint.x, catPoint.y),
      { activity: cat.activity, pose: cat.pose, paused:sample.still, facing: cat.facing, lift: cat.lift, feet: { x: cat.at.x, y: cat.at.y }, decorative: true, endpoint: { x: scene.human.x, y: scene.human.y } });
    if (cat.yarn) add("effect", objectId("effect", HUMAN_ENDPOINT, "yarn"), "Yarn ball", objectId("human", HUMAN_ENDPOINT), cat.yarn,
      spriteBounds(humanYarnBall(cat.pose), cat.yarn.x, cat.yarn.y), { decorative: true, pose: cat.pose });
  }
  for (const actor of scene.actors) {
    const parent = roomFor(actor.key), parentId = parent ? objectId("room", parent.key) : null;
    const drawn = inDrawMargin(actor.home);
    for (const kind of ["chair", "desk"] as const) {
      const asset = kind === "chair" ? "stool" : "desk";
      const at = kind === "chair" ? studioStoolAt(actor.home) : studio ? studioDeskAt(actor.home) : { ...actor.home, y: actor.home.y - 16 };
      const box = studio ? studioImage(asset, at).bounds : spriteBounds(prop(kind), at.x, at.y);
      add("furniture", objectId("furniture", actor.key, kind), studio ? asset : kind, parentId, at, box,
        { prop: studio ? asset : kind, actorKey: actor.key, ...(studio ? { assetId: asset } : {}) }, { drawn, related: [objectId("actor", actor.key)], meaning: studio ? svgBounds : undefined });
    }
    if (actor.resource.pendingMessages) add("effect", objectId("effect", actor.key, "pending-letters"), "Pending Agent letters", objectId("actor", actor.key), actor.home, null, { pendingMessages: typeof actor.resource.pendingMessages === "bigint" ? String(actor.resource.pendingMessages) : actor.resource.pendingMessages, individualPixelBounds: null }, { rendering: "aggregate", drawn, meaning: "Individual letter image bounds and count text bounds not exposed" });
  }
  for (const { actor, frame, dim, hidden } of sample.actors) {
    const behavior = sceneBehavior(actor.resource), drawn = inDrawMargin(frame), id = objectId("actor", actor.key), parent = roomFor(actor.key);
    const selected = sample.input.selected?.key === actor.key;
    const art = studio ? studioActorArt(actor.resource, frame, sample.elapsed, sample.still, selected) : null;
    const accent = game ? actorAccent(actor.key, frame.pose, sample.elapsed, sample.still) : null;
    const actorBounds = art && accent ? accentedBounds(art.body.bounds, frame, accent) : art?.body.bounds ?? spriteBounds(actorSprite(actor.key, frame, sample.elapsed, sample.still), frame.x, frame.y);
    add("actor", id, actor.resource.title, parent ? objectId("room", parent.key) : null, frame, actorBounds, {
      key: actor.key, reference: actor.resource.ref ? { ...actor.resource.ref } : null,
      behavior, animation: art?.animation ?? actorAnimation(frame,sample.elapsed,sample.still), status: actor.resource.nativeStatus ?? null, home: { ...actor.home }, rest: { ...actor.rest }, goal: { ...hallActorGoal(actor) }, sampled: { ...frame },
      selected, domTargetHidden: hidden,
      ...(art ? { assetId: art.body.asset, signal: art.signal?.asset ?? null } : {}),
      ...(accent ? { presentationAccent: accent } : {}),
    }, { drawn, filtered: dim, reason: dim ? "Filtered actors remain drawn at reduced opacity" : !drawn ? "Anchor is outside renderer draw margin" : undefined, domKey: id, actions: ["object.activate"], related: [objectId("furniture", actor.key, "desk"), objectId("furniture", actor.key, "chair")], meaning: studio ? svgBounds : undefined });
    if (art) {
      for (const [kind, placement] of [["attention", art.signal], ["selection", art.selection]] as const) {
        if (placement) add("effect", objectId("effect", actor.key, kind), placement.asset, id, placement.at, placement.bounds, { assetId: placement.asset }, { drawn, filtered: dim, meaning: svgBounds });
      }
    } else {
      const glyph = attentionGlyph(behavior), at = { x: frame.x + 30, y: frame.y - 24 };
      add("effect", objectId("effect", actor.key, "attention"), glyph, id, at, spriteBounds(emote(glyph, 0), at.x, at.y), { glyph }, { drawn, filtered: dim });
    }
  }
  for (const item of sample.input.props) {
    const at = { x: item.x, y: item.y - 4 }, kind = item.id === "cabinet" ? "archive" : item.id === "workspaces" ? "bookshelf" : "desk";
    const room = scene.rooms.find(room => room.cabinet.at.x === item.x && room.cabinet.at.y === item.y);
    const art = studio ? studioPropArt(item) : null;
    add("prop", objectId("prop", item.id, String(item.x), String(item.y)), item.title, room ? objectId("room", room.key) : null, art?.at ?? at, art?.bounds ?? spriteBounds(prop(kind), at.x, at.y), { anchorId: item.id, anchor: { x: item.x, y: item.y }, ...(art ? { assetId: art.asset } : {}) }, { drawn: inDrawMargin(item), domKey: objectId("prop", item.id, String(item.x), String(item.y)), actions: ["object.activate"], meaning: studio ? svgBounds : undefined });
  }
  for (const flight of sample.flights) {
    const endpoint = (key: string) => key === HUMAN_ENDPOINT ? scene.human : sample.actors.find(item => item.actor.key === key && !item.dim)?.actor.home;
    const from = endpoint(flight.event.from), to = endpoint(flight.event.to);
    const started = sample.elapsed >= flight.started;
    const drawn = started && !!from && !!to && inCanvas(from) && inCanvas(to);
    const progress = Math.min(1, Math.max(0, (sample.elapsed - flight.started) / 700));
    const at = from && to ? { x: from.x + (to.x - from.x) * progress, y: from.y - 20 + (to.y - from.y) * progress - Math.sin(progress * Math.PI) * 24 } : null;
    const endpointId = (key: string) => objectId(key === HUMAN_ENDPOINT ? "human" : "actor", key);
    add("effect", objectId("effect", flight.event.id), "Message flight", null, at, at ? (studio ? studioImage("envelope", at).bounds : spriteBounds(prop("envelope"), at.x, at.y)) : null, { eventId: flight.event.id, from: flight.event.from, to: flight.event.to, started: flight.started, progress, anchorAvailable: !!from && !!to, ...(studio ? { assetId: "envelope" } : {}) }, { drawn, reason: drawn ? undefined : "Queued or endpoint filtered/offscreen/unavailable", related: [endpointId(flight.event.from), endpointId(flight.event.to)], meaning: studio ? svgBounds : undefined });
  }
  if (!studio && !scene.actors.length) {
    for (const [kind, at] of [["coffee", { x: 568, y: 160 }], ["sofa", { x: 600, y: 240 }]] as const) add("furniture", objectId("furniture", "empty-scene", kind), kind, null, at, spriteBounds(prop(kind), at.x, at.y), { prop: kind });
  }
  return objects;
}
