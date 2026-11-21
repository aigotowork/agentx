/** Canvas-only vector rendering; consumes existing world geometry and the one presented actor frame. */
import type { Point } from "../geometry";
import type { HallRoom } from "../model";
import { STUDIO_IMAGE, STUDIO_TRIM, studioActorArt, studioDeskAt, studioImage, studioPropArt, studioRoomArt, studioStoolAt, type StudioImage } from "../studio-art";
import type { StudioArtSet } from "../studio-assets";
import { spaceGameWorld } from "../../../uiux/systems/space/components/presentation";
import type { ActorFrame, SceneFrame } from "./contracts";
import { StudioAtmosphere } from "./studio-atmosphere";

import { STUDIO_CANVAS, STUDIO_WORLDS } from "../studio-appearance";

export class StudioPainter {
  private atmosphere?: StudioAtmosphere;

  constructor(private readonly context: CanvasRenderingContext2D, private art: StudioArtSet) { this.setArt(art); }

  setArt(art: StudioArtSet): void {
    this.art = art;
    const game = spaceGameWorld(art.world);
    this.atmosphere = game ? new StudioAtmosphere(this.context, game, art) : undefined;
  }

  private get palette() { return this.art.world ? STUDIO_WORLDS[this.art.world] : STUDIO_CANVAS; }

  background(width: number, height: number): void {
    this.context.fillStyle = this.palette.courtyard;
    this.context.fillRect(0, 0, width, height);
    this.atmosphere?.vista(width, height);
  }

  rooms(sample: SceneFrame, visible: (point: Point) => boolean): void {
    const { scene, props } = sample.input;
    const { context } = this;
    context.imageSmoothingEnabled = true;
    for (const room of scene.rooms) this.room(room, sample);
    this.image(studioImage("human-mailbox", scene.human));
    for (const item of props) if (visible(item)) this.image(studioPropArt(item));
    for (const actor of scene.actors) {
      if (!visible(actor.home)) continue;
      this.image(studioImage("desk", studioDeskAt(actor.home)));
      this.image(studioImage("stool", studioStoolAt(actor.home)));
    }
  }

  actor(sampled: ActorFrame, sample: SceneFrame): void {
    const { actor, frame, dim } = sampled;
    const art = studioActorArt(actor.resource, frame, sample.elapsed, sample.still, sample.input.selected?.key === actor.key);
    this.context.imageSmoothingEnabled = true;
    this.context.globalAlpha = dim ? .22 : 1;
    if (art.selection) this.image(art.selection);
    if (this.atmosphere) this.atmosphere.body(sampled, sample, art.body, placement => this.image(placement));
    else this.image(art.body);
    if (art.signal) this.image(art.signal);
    this.context.globalAlpha = 1;
  }

  envelope(x: number, y: number): void { this.image(studioImage("envelope", { x, y })); }

  private room(room: HallRoom, sample: SceneFrame): void {
    const { context } = this, { x, y } = room.origin, { width, height } = room;
    const { assets } = this.art, palette = this.palette;
    const art = studioRoomArt(room);
    context.fillStyle = palette.shadow;
    context.fillRect(x + STUDIO_TRIM, y + STUDIO_TRIM, width, height);
    context.save();
    context.beginPath();
    context.rect(x, y, width, height);
    context.clip();
    const floor = assets.get("floor-oak")!, [tileWidth, tileHeight] = STUDIO_IMAGE["floor-oak"];
    for (let row = 0; row < height; row += tileHeight) {
      for (let column = 0; column < width; column += tileWidth) context.drawImage(floor, x + column, y + row, tileWidth, tileHeight);
    }
    if (art.rug) this.image(art.rug);
    this.atmosphere?.floor(room, sample);
    const wall = assets.get("wall-plaster")!, [wallWidth, wallHeight] = STUDIO_IMAGE["wall-plaster"];
    for (let column = 0; column < width; column += wallWidth) context.drawImage(wall, x + column, y, wallWidth, wallHeight);
    for (const detail of art.wall) this.image(detail);
    const open = art.door.bounds;
    context.fillStyle = palette.shell;
    context.fillRect(x, y, STUDIO_TRIM, height);
    context.fillRect(x + width - STUDIO_TRIM, y, STUDIO_TRIM, height);
    context.fillRect(x, y + height - STUDIO_TRIM, open.x - x, STUDIO_TRIM);
    context.fillRect(open.x + open.width, y + height - STUDIO_TRIM, x + width - open.x - open.width, STUDIO_TRIM);
    context.fillStyle = palette.edge;
    context.fillRect(x + 1, y, 2, height - STUDIO_TRIM);
    context.fillRect(x + width - STUDIO_TRIM + 1, y, 2, height - STUDIO_TRIM);
    this.image(art.door);
    for (const fixture of art.fixtures) this.image(fixture.image);
    this.atmosphere?.lantern(room, sample);
    context.restore();
  }

  private image(placement: StudioImage): void {
    const { context } = this, image = this.art.assets.get(placement.asset)!;
    const { x, y, width, height } = placement.bounds;
    if (placement.mirror) {
      context.save();
      context.translate(x + width, y);
      context.scale(-1, 1);
      context.drawImage(image, 0, 0, width, height);
      context.restore();
    } else context.drawImage(image, x, y, width, height);
  }
}
