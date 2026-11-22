/** Build the real finite browser bundle using the already-installed TypeScript compiler. */
import { spawnSync } from "node:child_process";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadVite } from '../vite.mjs';

const source = dirname(fileURLToPath(import.meta.url));
const repo = resolve(source, "../..");
const target = process.argv[2];
if (process.argv.length !== 3 || !target || !isAbsolute(target) || target === repo || target.startsWith(`${source}/`)) {
  throw new Error("Supply one new absolute disposable bundle directory outside the Web source tree");
}
// No recursive deletion or reuse: the executor owns this newly created disposable output.
await mkdir(target, { recursive: false });
const compiler = spawnSync(process.execPath, [join(repo, 'node_modules/typescript/bin/tsc'), '-p', join(source, 'tsconfig.json'), '--noEmit'], { cwd: repo, stdio: 'inherit' });
if (compiler.error || compiler.status !== 0) throw new Error('Observation typecheck failed; install the locked root dependencies before building');
const { build } = await loadVite();
await build({ configFile: join(source, 'vite.config.ts'), build: { outDir: target } });

const manifest = { files: [] };
for (const file of (await readdir(target)).sort()) {
  if (!/^[a-zA-Z0-9][a-zA-Z0-9.-]*\.(js|html|css)$/.test(file)) throw new Error(`Unexpected bundle asset ${relative(target, join(target, file))}`);
  const bytes = await readFile(join(target, file));
  if (bytes.length > 4 * 1024 * 1024) throw new Error("Asset exceeds adapter limit");
  const content_type = file.endsWith(".js") ? "text/javascript; charset=utf-8" : file.endsWith(".css") ? "text/css; charset=utf-8" : "text/html; charset=utf-8";
  manifest.files.push({ url: file === "index.html" ? "/" : `/${file}`, file, content_type });
}
if (manifest.files.length > 128) throw new Error("Bundle exceeds adapter manifest limit");
await writeFile(join(target, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
