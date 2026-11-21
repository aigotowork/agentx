/** Refresh only the explicitly owned, self-contained browser distribution. Normal builds need no sibling checkout. */
import {readFile, writeFile, mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {describeDancingAIAsset, stripDancingAIInlineMap} from './production-assets.mjs';
const checkout = process.argv[2];
if (!checkout) throw Error('Usage: node web/agent-space/scripts/vendor-dancingai.mjs /absolute/dancingai-checkout');
const source = resolve(checkout, 'packages/page/dist/browser/dancingai.global.js');
const bytes = await readFile(source);
const runtime = stripDancingAIInlineMap(bytes);
const manifest = JSON.parse(await readFile(resolve(checkout, 'packages/page/package.json'), 'utf8'));
const target = fileURLToPath(new URL('../public/tools/dancingai/', import.meta.url));
await mkdir(target, {recursive: true});
await writeFile(resolve(target, 'dancingai.global.js'), runtime);
await writeFile(resolve(target, 'distribution.json'), JSON.stringify({package: manifest.name, version: manifest.version,
  entry: '@dancingai/inspection-page/browser/global', ...describeDancingAIAsset(runtime),
  upstream: describeDancingAIAsset(bytes),
  update: 'Build the matching Page package, then run scripts/vendor-dancingai.mjs with its checkout. The import removes only the final inline source map; runtime code and legal notices are retained.'}, null, 2) + '\n');
console.log(`DancingAI browser runtime imported (${runtime.length} bytes).`);
