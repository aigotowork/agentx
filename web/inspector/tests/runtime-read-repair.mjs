/** Bundle the real SDK consumer as ESM despite the root package's CommonJS default. */
import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';
const entries = ['./directory-paging.ts', '../../agent-space/tests/operation-failures.ts'];
for (const entry of entries) {
  const result = await build({ entryPoints: [fileURLToPath(new URL(entry, import.meta.url))], bundle: true, platform: 'node', format: 'esm', write: false });
  try { await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString('base64')}`); }
  catch (error) { console.error(`${entry}: ${error.name}: ${error.message}`); process.exitCode = 1; break; }
}
