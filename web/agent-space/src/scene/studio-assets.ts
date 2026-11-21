/** Vite bundles the editable SVG masters locally. One set of decoded images is reused by remounts and portraits. */
import { STUDIO_IMAGE, type StudioAsset } from "./studio-art";
import { spaceGameWorld, type SpaceWorld } from "../../uiux/systems/space/components/presentation";
import { AMBIENCE_IMAGES, type AmbienceImage } from "./ambience";

const urls = import.meta.glob<string>("./art/studio/*.svg", { eager: true, import: "default", query: "?url" });
const finish = import.meta.glob<string>("./art/studio/finish/*.svg", { eager: true, import: "default", query: "?url" });
const worlds = import.meta.glob<string>("./art/studio/worlds/*/*.svg", { eager: true, import: "default", query: "?url&no-inline" });
export type StudioAssets = ReadonlyMap<StudioAsset, HTMLImageElement>;
export type StudioArtSet = { world?: SpaceWorld; assets: StudioAssets; ambience: ReadonlyMap<AmbienceImage, HTMLImageElement> };
const loading = new Map<SpaceWorld | undefined, Promise<StudioArtSet>>();

export function studioAssetUrl(asset: StudioAsset, world?: SpaceWorld): string {
  if (world) return worldAssetUrl(world, asset);
  const url = finish[`./art/studio/finish/${asset}.svg`] ?? urls[`./art/studio/${asset}.svg`];
  if (!url) throw new Error(`Missing Studio art: ${asset}`);
  return url;
}

function worldAssetUrl(world: SpaceWorld, asset: StudioAsset | AmbienceImage): string {
  const url = worlds[`./art/studio/worlds/${world}/${asset}.svg`];
  if (!url) throw new Error(`Missing ${world} art: ${asset}`);
  return url;
}

async function decode<T extends string>(id: T, url: string, [width, height]: readonly [number, number, ...unknown[]]) {
  const image = new Image();
  image.src = url;
  await image.decode();
  if (image.naturalWidth !== width || image.naturalHeight !== height) throw new Error(`Unexpected Studio art dimensions: ${id}`);
  return [id, image] as const;
}

export function loadStudioAssets(world?: SpaceWorld): Promise<StudioArtSet> {
  const cached = loading.get(world);
  if (cached) return cached;
  const game = spaceGameWorld(world);
  const next = Promise.resolve().then(async () => {
    const [sprites, extras] = await Promise.all([
      Promise.all((Object.keys(STUDIO_IMAGE) as StudioAsset[]).map(id => decode(id, studioAssetUrl(id, world), STUDIO_IMAGE[id]))),
      game ? Promise.all((Object.keys(AMBIENCE_IMAGES) as AmbienceImage[]).map(id => decode(id, worldAssetUrl(game, id), AMBIENCE_IMAGES[id]))) : [],
    ]);
    return { world, assets: new Map(sprites), ambience: new Map(extras) };
  }).catch(error => { loading.delete(world); throw error; });
  loading.set(world, next);
  return next;
}
