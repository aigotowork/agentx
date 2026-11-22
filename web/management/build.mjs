/** Compile fresh management sources; publish the finite native asset manifest. */
import { spawnSync } from "node:child_process";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadVite } from "../vite.mjs";

const source = dirname(fileURLToPath(import.meta.url));
const root = resolve(source, "../..");
const target = process.argv[2];
if (process.argv.length !== 3 || !target || !isAbsolute(target) || target === root || target.startsWith(`${source}/`)) {
  throw new Error("Supply one new absolute disposable output directory outside the Web source tree");
}
await mkdir(target, { recursive: false });
const compiler = spawnSync(process.execPath, [join(root, "node_modules/typescript/bin/tsc"), "-p", join(source, "tsconfig.json"), "--noEmit"], { cwd: root, stdio: "inherit" });
if (compiler.error || compiler.status !== 0) throw new Error("Management typecheck failed; install the locked root dependencies before building");
const { build } = await loadVite();
await build({ configFile: join(source, "vite.config.ts"), build: { outDir: target } });
const manifest = { files: [] };
let total = 0;
for (const file of (await readdir(target)).sort()) {
  if (!/^[a-zA-Z0-9][a-zA-Z0-9.-]*\.(js|css|html)$/.test(file)) throw new Error(`Unsupported generated asset: ${file}`);
  const bytes = await readFile(join(target, file));
  total += bytes.length;
  if (bytes.length > 4 * 1024 * 1024) throw new Error("Management asset exceeds native limit");
  const content_type = file.endsWith(".js") ? "text/javascript; charset=utf-8" : file.endsWith(".css") ? "text/css; charset=utf-8" : "text/html; charset=utf-8";
  manifest.files.push({ url: file === "index.html" ? "/" : `/${file}`, file, content_type });
}
if (!manifest.files.some(file => file.url === "/") || manifest.files.length > 128 || total > 16 * 1024 * 1024) throw new Error("Invalid management bundle bounds");
await writeFile(join(target, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
