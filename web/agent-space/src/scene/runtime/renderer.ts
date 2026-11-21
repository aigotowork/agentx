/** Canvas-only presentation: backing size, fixed mount palette, bounded bitmap cache and draw order. */
import { prop, officePalette, emote, frameAt, loopOf, type Bitmap, type Palette } from "../art/art";
import { worldToScreen, type Point } from "../geometry";
import { sceneBehavior } from "../motion";
import { hallOfficeArt, hallOfficePalette, humanCatNest, humanYarnBall } from "../office-art";
import { HUMAN_ENDPOINT } from "../../offline/model";
import type { ActorFrame, SceneFrame } from "./contracts";
import { actorSprite, attentionGlyph, spriteBounds } from "./sprite";
import type { StudioArtSet } from "../studio-assets";
import { StudioPainter } from "./studio-render";

export class SceneRenderer {
  private readonly palette: Palette;
  private readonly cache = new Map<string, HTMLCanvasElement>();
  private readonly studio?: StudioPainter;

  constructor(root: HTMLElement, private readonly surface: HTMLCanvasElement, private readonly context: CanvasRenderingContext2D, studio?: StudioArtSet) {
    if (studio) this.studio = new StudioPainter(context, studio);
    const style = getComputedStyle(root);
    const read = (name: string) => style.getPropertyValue(`--office-art-${name}`).trim();
    this.palette = hallOfficePalette(officePalette({ colors: {
      surface0: read("surface0"), surface1: read("surface1"), surface2: read("surface2"),
      foreground: read("foreground"), foregroundMuted: read("muted"), accent: read("accent"),
      statusWarning: read("warning"), statusDanger: read("danger"), statusSuccess: read("success"),
    } }));
  }

  setStudioArt(art: StudioArtSet): void { this.studio?.setArt(art); }

  draw(sample: SceneFrame): void {
    const { camera, viewport, scene } = sample.input;
    const context = this.context;
    this.resize(sample);
    context.clearRect(0, 0, viewport.width, viewport.height);
    this.studio?.background(viewport.width, viewport.height);
    context.save();
    context.translate(camera.x, camera.y);
    context.scale(camera.zoom, camera.zoom);
    context.imageSmoothingEnabled = false;
    const left = -camera.x / camera.zoom - 40, top = -camera.y / camera.zoom - 40;
    const right = (viewport.width - camera.x) / camera.zoom + 40, bottom = (viewport.height - camera.y) / camera.zoom + 40;
    const visible = (point: Point) => point.x >= left && point.x <= right && point.y >= top && point.y <= bottom;

    if (this.studio) {
      this.studio.rooms(sample, visible);
      this.drawHumanLetters(sample);
    } else this.drawRooms(sample, visible);
    this.drawPendingLetters(sample, visible);
    this.drawFlights(sample);
    for (const actor of [...sample.actors].sort((a, b) => a.frame.y - b.frame.y)) {
      if (!visible(actor.frame)) continue;
      if (this.studio) this.studio.actor(actor, sample);
      else this.drawActor(actor, sample);
    }
    if (!this.studio) {
      const cat = sample.cat;
      this.sprite(humanCatNest("cat", cat.pose, cat.facing), cat.at.x, cat.at.y - cat.lift);
      if (cat.yarn) this.sprite(humanYarnBall(cat.pose), cat.yarn.x, cat.yarn.y);
      if (!scene.actors.length) {
        this.sprite(prop("coffee"), 568, 160);
        this.sprite(prop("sofa"), 600, 240);
      }
    }
    context.restore();
  }

  dispose(): void { this.cache.clear(); }

  private resize(sample: SceneFrame): void {
    const ratio = Math.min(2, devicePixelRatio || 1), { viewport } = sample.input;
    const width = Math.round(viewport.width * ratio), height = Math.round(viewport.height * ratio);
    if (this.surface.width !== width || this.surface.height !== height) {
      this.surface.width = width;
      this.surface.height = height;
    }
    this.context.setTransform(ratio, 0, 0, ratio, 0, 0);
  }

  private drawRooms(sample: SceneFrame, visible: (point: Point) => boolean): void {
    const { scene, props } = sample.input;
    this.sprite(humanCatNest("nest"), scene.human.x, scene.human.y);
    this.drawHumanLetters(sample);
    for (const room of scene.rooms) this.sprite(hallOfficeArt(room, 2), room.origin.x, room.origin.y);
    for (const item of props) {
      if (!visible(item)) continue;
      const kind = item.id === "cabinet" ? "archive" : item.id === "workspaces" ? "bookshelf" : "desk";
      this.sprite(prop(kind, { screen: "lit", frame: 0 }), item.x, item.y - 4);
    }
    for (const actor of scene.actors) {
      if (!visible(actor.home)) continue;
      this.sprite(prop("chair"), actor.home.x, actor.home.y + 16);
      const behavior = sceneBehavior(actor.resource);
      const screen = behavior === "working" ? "lit" : ["request-error", "error"].includes(behavior) ? "error" : "off";
      const frame = sample.still ? 0 : frameAt(loopOf(8, 160), sample.elapsed);
      this.sprite(prop("desk", { screen, frame }), actor.home.x, actor.home.y - 16);
    }
  }

  private drawHumanLetters(sample: SceneFrame): void {
    const { scene, humanMessages } = sample.input;
    for (let letter = 0; letter < (humanMessages > 3 ? 3 : Number(humanMessages)); letter++) {
      this.letter(scene.human.x + 22 + letter * 4, scene.human.y - 12 - letter * 3);
    }
    if (humanMessages > 0) {
      this.context.save(); this.context.font = "12px sans-serif"; this.context.fillStyle = "#302e2a";
      this.context.fillText(String(humanMessages), scene.human.x + 34, scene.human.y - 30); this.context.restore();
    }
  }

  private drawPendingLetters(sample: SceneFrame, visible: (point: Point) => boolean): void {
    const context = this.context;
    for (const actor of sample.input.scene.actors) {
      const count = actor.resource.pendingMessages;
      if (!count || !visible(actor.home)) continue;
      for (let letter = 0; letter < (count > 3 ? 3 : Number(count)); letter++) {
        this.letter(actor.home.x + 20 + letter * 4, actor.home.y - 12 - letter * 3);
      }
      context.save();
      context.font = "12px sans-serif";
      context.fillStyle = "#302e2a";
      context.fillText(String(count), actor.home.x + 34, actor.home.y - 30);
      context.restore();
    }
  }

  private drawFlights(sample: SceneFrame): void {
    const { scene, camera, viewport } = sample.input;
    const endpoint = (key: string) => key === HUMAN_ENDPOINT ? scene.human : sample.actors.find(item => item.actor.key === key && !item.dim)?.actor.home;
    const onScreen = (point: Point) => {
      const screen = worldToScreen(point, camera);
      return screen.x >= 0 && screen.y >= 0 && screen.x <= viewport.width && screen.y <= viewport.height;
    };
    for (const flight of sample.flights) {
      if (sample.elapsed < flight.started) continue;
      const from = endpoint(flight.event.from), to = endpoint(flight.event.to);
      if (!from || !to || !onScreen(from) || !onScreen(to)) continue;
      const progress = Math.min(1, Math.max(0, (sample.elapsed - flight.started) / 700));
      this.letter(from.x + (to.x - from.x) * progress, from.y - 20 + (to.y - from.y) * progress - Math.sin(progress * Math.PI) * 24);
    }
  }

  private letter(x: number, y: number): void {
    if (this.studio) this.studio.envelope(x, y);
    else this.sprite(prop("envelope"), x, y);
  }

  private drawActor({ actor, frame, dim }: ActorFrame, sample: SceneFrame): void {
    this.context.globalAlpha = dim ? .22 : 1;
    this.sprite(actorSprite(actor.key, frame, sample.elapsed, sample.still), frame.x, frame.y);
    this.sprite(emote(attentionGlyph(sceneBehavior(actor.resource)), 0), frame.x + 30, frame.y - 24);
    this.context.globalAlpha = 1;
  }

  private sprite(bitmap: Bitmap, x: number, y: number, scale = 2): void {
    let image = this.cache.get(bitmap.key);
    if (!image) {
      image = document.createElement("canvas");
      image.width = bitmap.width;
      image.height = bitmap.height;
      const context = image.getContext("2d");
      if (!context) return;
      const data = context.createImageData(bitmap.width, bitmap.height);
      bitmap.pixels.forEach((slot, index) => data.data.set(this.palette.colors[slot]!, index * 4));
      context.putImageData(data, 0, 0);
      if (this.cache.size >= 512) this.cache.delete(this.cache.keys().next().value!);
      this.cache.set(bitmap.key, image);
    }
    const bounds = spriteBounds(bitmap, x, y, scale);
    this.context.drawImage(image, bounds.x, bounds.y, bounds.width, bounds.height);
  }
}
