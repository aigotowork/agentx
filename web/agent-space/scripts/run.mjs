/** Isolated module runner; all dependencies resolve from the existing root lockfile. */
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { resolve, isAbsolute } from "node:path";
import { loadVite } from "../../vite.mjs";
const source = resolve(fileURLToPath(new URL("../", import.meta.url)));
const root = resolve(source, "../..");
const [mode, target] = process.argv.slice(2);
if (!["dev", "typecheck", "build"].includes(mode) || (mode === "build" && (!target || !isAbsolute(target) || target === root || target === source || source.startsWith(`${target}/`) || target.startsWith(`${source}/`)))) {
  throw new Error("Usage: node web/agent-space/scripts/run.mjs dev|typecheck|build [new absolute output directory outside source]");
}
if (mode === "typecheck" || mode === "build") {
  const check = spawnSync(process.execPath, [resolve(root, "node_modules/typescript/bin/tsc"), "-p", resolve(source, "tsconfig.json"), "--noEmit"], { cwd: root, stdio: "inherit" });
  if (check.error || check.status !== 0) process.exit(check.status || 1);
}
if (mode !== "typecheck") {
  const vite = await loadVite();
  const configFile = resolve(source, "vite.config.ts");
  if (mode === "dev") { const server = await vite.createServer({ configFile }); await server.listen(); server.printUrls(); }
  else {
    const { mkdir, cp } = await import("node:fs/promises");
    await mkdir(target, { recursive: false });
    await vite.build({ configFile, build: { outDir: target, emptyOutDir: false } });
    const { prepareProductionAssets } = await import('./production-assets.mjs');
    await prepareProductionAssets(target);
    // Vite copies public notices; keep a stable root attribution entry in the artifact.
    await cp(resolve(source, "NOTICE.md"), resolve(target, "NOTICE.md"));
  }
}
