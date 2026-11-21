/** @aiPurpose Compose the original Lobby artwork over the shared office projection without inventing furniture actions. */
import { door, prop, tile, type Bitmap, type Palette } from "./art/art";
import { blit, drawn } from "./art/pixel/grid";
import { parseColor } from "./art/pixel/color";
import { SHADOW_OF, slotIndex, type Slot } from "./art/pixel/palette";
import type { HallRoom } from "./model";

const ART = { wall: 16, rim: 3, doorway: 16, mat: 16, pantry: 48 } as const;
const OFFICE_ENVIRONMENT = {
  floor: "#EEEADF", floorAlt: "#ECE8DD", floorSeam: "#E6E2D8",
  wall: "#CDD2BE", wallShade: "#BBC4AD", wallTrim: "#A8B297",
  partition: "#BBC4AD", partitionShade: "#A8B297", partitionLight: "#CDD2BE",
} as const;
const environments = new WeakMap<Palette, Palette>();

/** @aiPurpose Give only the office architecture a quiet warm ground. @aiUsage Character, furniture and state slots keep the host palette; dark rooms keep its existing dark surfaces. */
export function hallOfficePalette(base: Palette): Palette {
  if (base.dark) return base;
  const cached = environments.get(base); if (cached) return cached;
  const colors = [...base.colors], changed = new Set<Slot>();
  for (const [name, value] of Object.entries(OFFICE_ENVIRONMENT)) { const slot = name as Slot; colors[slotIndex(slot)] = parseColor(value); changed.add(slot); }
  for (const [slot, shade] of SHADOW_OF) if (changed.has(slot) || changed.has(shade)) colors[slotIndex(`${slot}Shadow`)] = colors[slotIndex(shade)]!;
  const result = { ...base, colors, key: `${base.key}:office:${JSON.stringify(OFFICE_ENVIRONMENT)}` };
  environments.set(base, result); return result;
}

/** @aiPurpose Compose a furnished office using geometry-only cache keys. @aiUsage Scale maps original art pixels to shared world coordinates; polls, filters and the clock never enter the key. */
export function hallOfficeArt(room: HallRoom, scale: number): Bitmap {
  const width = Math.ceil(room.width / scale), height = Math.ceil(room.height / scale);
  const local = (at: { x: number; y: number }) => ({ x: Math.round((at.x - room.origin.x) / scale), y: Math.round((at.y - room.origin.y) / scale) });
  const office = room.office, lounge = office ? { ...local(office.lounge), width: Math.round(office.lounge.width / scale), height: Math.round(office.lounge.height / scale) } : null;
  const fixtures = office?.fixtures.map((fixture) => ({ prop: fixture.prop, ...local(fixture.at) })) ?? [];
  const entry = office ? local(office.door) : { x: ART.doorway * 2, y: height };
  return drawn(`hall-office-v1:${JSON.stringify([width, height, lounge, entry, fixtures])}`, width, height, (out) => {
    const floor = tile("floor"), carpet = tile("carpet"), kitchen = tile("kitchen"), wall = tile("wall"), cap = tile("wallCap");
    for (let y = 0; y < height; y += floor.height) for (let x = 0; x < width; x += floor.width) blit(out, floor, x, y);
    // Zone texture follows the very same lounge rect that owns the characters' rest slots.
    if (lounge?.height) for (let y = lounge.y; y < lounge.y + lounge.height; y++) for (let x = lounge.x; x < lounge.x + lounge.width; x++) {
      if (x < 0 || y < 0 || x >= width || y >= height) continue;
      const source = x >= lounge.x + lounge.width - ART.pantry && y < lounge.y + ART.pantry ? kitchen : carpet;
      out.pixels[y * width + x] = source.pixels[((y - lounge.y) % source.height) * source.width + (x - lounge.x) % source.width]!;
    }
    // A continuous wall face and its cap replace the former thin rectangle over a white grid.
    for (let x = 0; x < width; x += wall.width) blit(out, wall, x, 0);
    for (let y = 0; y < ART.rim; y++) for (let x = 0; x < width; x++) out.pixels[y * width + x] = cap.pixels[y * cap.width + x % cap.width]!;
    const border = tile("partition");
    for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
      const bottom = y >= height - ART.rim && Math.abs(x - entry.x) > ART.doorway / 2;
      if (x < ART.rim || x >= width - ART.rim || bottom) out.pixels[y * width + x] = border.pixels[(y % border.height) * border.width + border.width / 2]!;
    }
    blit(out, tile("doormat"), entry.x - ART.mat / 2, height - ART.mat * 2);
    blit(out, door(), entry.x - ART.doorway / 2, height - ART.doorway);
    for (const item of fixtures) { const bitmap = prop(item.prop); blit(out, bitmap, item.x - bitmap.anchor.x, item.y - bitmap.anchor.y); }
  });
}

/** @aiPurpose Draw the scope-human endpoint as a curled ginger cat in a woven nest. @aiUsage Place this bottom-centred bitmap at scene.human; it has no Agent pose or lifecycle. */
export function humanCatNest(part: "both" | "nest" | "cat" = "both", pose = 0, facing: "left" | "right" = "left"): Bitmap {
  const frame = Math.abs(Math.floor(pose)) % 4;
  const bitmap = drawn(`human-cat-nest-v2:${part}:${frame}:${facing}`, 44, 34, out => {
    const dot = (x: number, y: number, slot: Slot) => { out.pixels[y * out.width + x] = slotIndex(slot); };
    const box = (x: number, y: number, width: number, height: number, slot: Slot) => { for (let yy = y; yy < y + height; yy++) for (let xx = x; xx < x + width; xx++) dot(xx, yy, slot); };
    // Broad elliptical wicker rim and a soft, recessed cushion.
    if (part !== "cat") for (let y = 18; y < 33; y++) for (let x = 1; x < 43; x++) {
      const radius = ((x - 22) / 21) ** 2 + ((y - 25) / 7.5) ** 2;
      if (radius <= 1) dot(x, y, radius > .75 ? "woodShade" : y < 26 ? "fabricShade" : (x + y) % 4 === 0 ? "woodLight" : "wood");
    }
    if (part === "nest") return;
    // Low horizontal body, four tucked paws, triangular ears and a curling tail.
    box(16, 14, 18, 11, "outline"); box(18, 12, 13, 14, "hair3"); box(15, 16, 20, 7, "hair3");
    box(13, 22, 7, 4, "white"); box(24, 23, 7, 3, "white");
    box(33, 16, 3, 9, "hair2"); box(29, 24, 7, 3, "hair2"); box(27, 22, 3, 4, "hair2");
    box(8, 9, 14, 12, "outline"); box(9, 8, 12, 13, "hair3");
    for (let i = 0; i < 6; i++) { box(8, 3 + i, 1 + i, 1, "hair3"); box(21 - i, 3 + i, 1 + i, 1, "hair3"); }
    box(10, 6, 2, 3, "earInner"); box(18, 6, 2, 3, "earInner");
    box(13, 9, 2, 3, "hair2"); box(17, 9, 2, 2, "hair2");
    box(10, 13, 3, 2, "outline"); box(17, 13, 3, 2, "outline");
    box(13, 16, 5, 4, "white"); dot(15, 16, "earInner"); dot(15, 18, "outline");
    box(5, 16, 5, 1, "white"); box(20, 16, 5, 1, "white");
    box(23, 14, 2, 4, "hair2"); box(28, 15, 2, 4, "hair2");
    if (frame) {
      box(12 + frame % 2 * 2, 25, 5, 3, "white"); box(27 - frame % 2 * 2, 25, 5, 3, "white");
      box(10, 13, 2, 1, "white"); box(18, 13, 2, 1, "white");
      box(34, 12 - frame, 2, 8, "hair2");
    }
    if (facing === "right") for (let y = 0; y < out.height; y++) for (let x = 0; x < out.width / 2; x++) {
      const a = y * out.width + x, b = y * out.width + out.width - x - 1;
      const pixel = out.pixels[a]!; out.pixels[a] = out.pixels[b]!; out.pixels[b] = pixel;
    }
  });
  return { ...bitmap, anchor: { x: 22, y: 34 } };
}

/** @aiPurpose Draw the cat's small decorative yarn ball in the existing palette. @aiUsage Finite roll frames only; no mail or actor meaning. */
export function humanYarnBall(pose: number): Bitmap {
  const frame = Math.abs(Math.floor(pose)) % 4;
  const bitmap = drawn(`human-yarn-v1:${frame}`, 12, 12, out => {
    for (let y = 1; y < 11; y++) for (let x = 1; x < 11; x++) {
      if ((x - 5.5) ** 2 + (y - 5.5) ** 2 > 25) continue;
      out.pixels[y * out.width + x] = slotIndex((x + y + frame) % 4 === 0 ? "white" : "earInner");
    }
    out.pixels[11 * out.width + 10] = slotIndex("earInner");
  });
  return { ...bitmap, anchor: { x: 6, y: 12 } };
}
