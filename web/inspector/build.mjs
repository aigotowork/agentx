/** Integration-only fresh artifact. Existing development dist and services are never touched. */
import { build } from 'esbuild';
import { mkdir, copyFile, access, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, isAbsolute, join } from 'node:path';
const root = dirname(fileURLToPath(import.meta.url));
const output = process.argv[2];
if (!output || !isAbsolute(output)) throw new Error('Usage: node build.mjs /absolute/nonexistent/output');
let exists = false;
try { await access(output); exists = true; } catch (error) { if (error.code !== 'ENOENT') throw error; }
if (exists) throw new Error('Output must not already exist');
await mkdir(output, { recursive: true });
await build({ absWorkingDir: root, entryPoints: ['web/client/main.tsx'], bundle: true, format: 'esm', splitting: true, target: ['es2022'], outdir: join(output, 'assets'), entryNames: 'app', minify: true, jsx: 'automatic', alias: { 'react-native': 'react-native-web' }, define: { 'process.env.NODE_ENV': '"production"', __DEV__: 'false' }, logLevel: 'info' });
const html = '<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"><meta name="color-scheme" content="light dark"><title>Agent Inspector</title><link rel="stylesheet" href="/inspector/assets/app.css"></head><body><div id="root"></div><script type="module" src="/inspector/assets/app.js"></script></body></html>\n';
await writeFile(join(output, 'index.html'), html);
for (const file of ['LICENSE', 'NOTICE']) await copyFile(join(root, file), join(output, file));
