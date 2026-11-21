/** Own the import and build policy for the pinned DancingAI browser script. */
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';

/** Strip only a final inline source-map comment. Leave a map-free input unchanged. */
export function stripDancingAIInlineMap(source) {
  const text = source.toString('utf8');
  const map = /(?:\r?\n)\/\/# sourceMappingURL=data:application\/json(?:;charset=[^;,\s]+)?;base64,[A-Za-z0-9+/=]+\s*$/.exec(text);
  return map ? Buffer.from(text.slice(0, map.index) + '\n') : source;
}

export function describeDancingAIAsset(bytes) {
  return { bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') };
}

/** Verify Vite copied the pinned runtime unchanged and record the bundled bytes. */
export async function prepareProductionAssets(output) {
  const directory = join(output, 'tools/dancingai');
  const file = 'dancingai.global.js';
  const [runtime, manifest] = await Promise.all([
    readFile(join(directory, file)),
    readFile(join(directory, 'distribution.json'), 'utf8').then(JSON.parse),
  ]);
  const artifact = { file, ...describeDancingAIAsset(runtime) };
  if (artifact.bytes !== manifest.bytes || artifact.sha256 !== manifest.sha256 ||
      stripDancingAIInlineMap(runtime) !== runtime) {
    throw new Error('DancingAI browser runtime must match the pinned map-free distribution');
  }
  await writeFile(join(directory, 'production.json'), JSON.stringify({
    source: artifact,
    output: artifact,
    transform: 'Copy the pinned browser runtime unchanged. Any trailing inline source map is removed on import.',
  }, null, 2) + '\n');
}
