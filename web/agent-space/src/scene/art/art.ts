/** @aiPurpose Publish original Lobby bitmap art as data for DOM canvas and RN Image renderers. */
import { Lru } from "./pixel/lru";
import { pngDataUri } from "./pixel/png";
import type { Bitmap } from "./pixel/grid";
import type { Palette } from "./pixel/palette";
export { character, CHARACTER_FRAMES, type CharacterSpec, type CharacterPose } from "./pixel/characters";
export { prop, tile, wallTopAt, door } from "./pixel/props";
export { emote } from "./pixel/emotes";
export { officePalette, lookFor, paletteColor, type ArtTheme, type Palette, type Slot } from "./pixel/palette";
export type { Bitmap } from "./pixel/grid";
export { frameAt, loopOf, walkPath } from "./walk";

const images = new Lru<string>(512, 6_000_000, (uri) => uri.length);
/** @aiPurpose Encode the same palette-indexed sprite for an RN Image or a DOM image without a canvas.
 * @aiUsage Supply an integer pixel scale. Cache lifetime is bounded and carries no business state.
 */
export function hallSpriteUri(bitmap: Bitmap, palette: Palette, pixelScale = 1): string {
  const scale = Math.max(1, Math.min(4, Math.round(pixelScale)));
  const key = `${bitmap.key}:${palette.key}:${scale}`;
  const cached = images.get(key); if (cached) return cached;
  const uri = pngDataUri(bitmap.pixels, bitmap.width, bitmap.height, palette.colors, scale);
  images.set(key, uri); return uri;
}

export const POSE_FRAME_MS = { idle: 900, walk: 140, sit: 220, sleep: 1200, drink: 600, talk: 320, stretch: 700, doze: 1200, lie: 1500 } as const;
