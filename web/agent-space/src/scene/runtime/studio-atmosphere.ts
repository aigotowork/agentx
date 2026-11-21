/** Scene craft applied on the existing draw clock. No state, random input, or native effects. */
import { actorAccent, roomIsLit, roomLantern } from "../ambience";
import { STUDIO_WORLDS } from "../studio-appearance";
import type { SpaceGameWorld } from "../../../uiux/systems/space/components/presentation";
import type { HallRoom } from "../model";
import type { ActorFrame, SceneFrame } from "./contracts";
import type { StudioImage } from "../studio-art";
import type { StudioArtSet } from "../studio-assets";

export class StudioAtmosphere {
  private readonly grade;

  constructor(private readonly ctx: CanvasRenderingContext2D, world: SpaceGameWorld, private readonly art: StudioArtSet) {
    this.grade = STUDIO_WORLDS[world].ambience;
  }

  vista(width: number, height: number): void {
    const image = this.art.ambience.get("scene-vista");
    if (!image || !width || !height) return;
    const scale = Math.max(width / image.naturalWidth, height / image.naturalHeight);
    const w = image.naturalWidth * scale, h = image.naturalHeight * scale;
    this.ctx.drawImage(image, (width - w) / 2, (height - h) / 2, w, h);
  }

  floor(room: HallRoom, sample: SceneFrame): void {
    const { ctx, grade } = this, { x, y } = room.origin, { width, height } = room;
    const value = sample.still ? 0 : Math.sin(sample.elapsed / 730) * .05;
    const shade = ctx.createLinearGradient(x, y + 48, x + width * .78, y + height);
    shade.addColorStop(0, `${grade.shade}6e`);
    shade.addColorStop(.38, `${grade.shade}00`);
    shade.addColorStop(1, `${grade.shade}79`);
    ctx.fillStyle = shade;
    ctx.fillRect(x, y + 48, width, height - 48);
    this.halo(x + 84, y + 100, Math.min(220, width * .59), grade.daylight);
    if (width > 320) this.halo(x + width - 92, y + 104, 140, grade.daylight * .7);
    if (roomIsLit(sample.input.roomLights, room.key)) {
      const lamp = roomLantern(room);
      this.halo(lamp.x + 10, lamp.y - 13, 112, .56 + value, grade.flame);
    }
  }

  lantern(room: HallRoom, sample: SceneFrame): void {
    const { ctx, grade } = this, at = roomLantern(room);
    const image = this.art.ambience.get("scene-lantern");
    if (!image) return;
    const lit = roomIsLit(sample.input.roomLights, room.key);
    ctx.save();
    this.contact(at.x, at.y - 2, 18, .22);
    if (!lit) ctx.filter = "saturate(.5) brightness(.64)";
    ctx.drawImage(image, at.x - 32, at.y - 62, 64, 62);
    ctx.restore();
    if (!lit) return;
    this.halo(at.x, at.y - 30, 35, .58, grade.flame);
    if (sample.still) return;
    ctx.save();
    ctx.fillStyle = grade.spark;
    for (let index = 0; index < 3; index += 1) {
      const t = (sample.elapsed / 2200 + index * .33) % 1;
      ctx.globalAlpha = Math.sin(t * Math.PI) * .69;
      ctx.fillRect(at.x + Math.sin(t * 9 + index) * 11, at.y - 40 - t * 31, 1.6, 1.6);
    }
    ctx.restore();
  }

  contact(x: number, y: number, size: number, opacity = .27): void {
    const { ctx } = this;
    ctx.save();
    ctx.globalAlpha *= opacity;
    ctx.fillStyle = this.grade.shade;
    ctx.beginPath();
    ctx.ellipse(x + 4, y - 1, size, Math.max(3.5, size * .24), -.08, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
  }

  body(actor: ActorFrame, sample: SceneFrame, body: StudioImage, draw: (image: StudioImage) => void): void {
    const { ctx, grade } = this, { frame } = actor;
    const effect = actorAccent(actor.actor.key, frame.pose, sample.elapsed, sample.still);
    this.contact(frame.x, frame.y, frame.pose === "sit" ? 19 : 16, .42 - effect.rise * .025);
    if (effect.activity > .05) {
      const working = frame.pose === "sit";
      this.halo(frame.x + (working ? 17 : -10), frame.y - (working ? 33 : 3), working ? 22 : 12, effect.activity * .47);
      ctx.save();
      ctx.globalAlpha *= effect.activity * .84;
      ctx.fillStyle = grade.spark;
      ctx.fillRect(frame.x + (working ? 21 : -10), frame.y - (working ? 41 : 3), 2.1, 2.1);
      ctx.fillRect(frame.x + (working ? 15 : 1), frame.y - (working ? 47 : 6), 1.1, 1.1);
      ctx.restore();
    }
    ctx.save();
    ctx.translate(frame.x, frame.y - effect.rise);
    ctx.rotate(effect.sway);
    ctx.scale(1, effect.stretch);
    ctx.translate(-frame.x, -frame.y);
    draw(body);
    ctx.restore();
  }

  private halo(x: number, y: number, radius: number, opacity: number, tint: string = this.grade.light): void {
    const { ctx } = this;
    ctx.save();
    ctx.globalAlpha *= opacity;
    const light = ctx.createRadialGradient(x, y, 0, x, y, radius);
    light.addColorStop(0, `${tint}fd`);
    light.addColorStop(.38, `${tint}60`);
    light.addColorStop(1, `${tint}00`);
    ctx.fillStyle = light;
    ctx.fillRect(x - radius, y - radius, radius * 2, radius * 2);
    ctx.restore();
  }
}
