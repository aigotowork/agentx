/** @aiPurpose Export trusted source-first examples as inert JSON without invoking native business code. */
import { writeFileSync } from 'node:fs';
import { dirname, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { compile } from '../index.ts';
import type { WorkbenchArtifact } from '../contracts.ts';
import { createOrderModel } from './order-stock.ts';
import { createTechnicalModel } from './technical.ts';

/** @aiPurpose Compile a selected pure example without running any native scenario.
 * @aiUsage Call from acceptance/build tooling; errors retain diagnostic codes/paths and cannot be published as accepted JSON.
 */
export function exportArtifact(example: 'order-stock' | 'technical' = 'order-stock'): WorkbenchArtifact {
  const checked = compile(example === 'technical' ? createTechnicalModel() : createOrderModel());
  if (!checked.ok) throw new Error(checked.diagnostics.map(item => `${item.code} ${JSON.stringify(item.path)}: ${item.message}`).join('\n'));
  return checked.value;
}
function main(args: readonly string[]): void {
  if (args.includes('--help')) { process.stdout.write('Usage: node --experimental-strip-types tools/flow-atlas/domain/examples/export.ts [order-stock|technical] [new-output.json|-]\nInspection/export invokes zero native business commands. Existing output is never overwritten.\n'); return; }
  const [example = 'order-stock', output = '-'] = args;
  if (!['order-stock', 'technical'].includes(example) || args.length > 2) throw new Error('Expected example order-stock|technical and optional new JSON output path.');
  const artifact = exportArtifact(example as 'order-stock' | 'technical');
  const json = `${JSON.stringify(artifact, null, 2)}\n`;
  if (output === '-') process.stdout.write(json);
  else {
    const destination = resolve(output);
    const toolRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
    const protectedRoots = ['domain', 'design', 'authoring', 'workbench', 'viewer', 'spatial', 'inspector'].map(name => resolve(toolRoot, name));
    if (!destination.endsWith('.json') || protectedRoots.some(root => destination === root || destination.startsWith(`${root}${sep}`))) throw new Error('Use a new .json output outside source directories.');
    writeFileSync(destination, json, { flag: 'wx' });
    process.stderr.write(`Exported ${example}: ${artifact.scenarios.length} scenarios; native business calls: 0.\n`);
  }
}
if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try { main(process.argv.slice(2)); } catch (error) { process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`); process.exitCode = 1; }
}
