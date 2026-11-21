import { build } from 'esbuild';
import { mkdir, copyFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
const root = dirname(dirname(fileURLToPath(import.meta.url)));
await mkdir(join(root, 'dist/assets'), { recursive: true });
await build({ absWorkingDir: root, entryPoints: ['web/client/main.tsx'], bundle: true, format: 'esm', splitting: true, target: ['es2022'], outdir: 'dist/assets', entryNames: 'app', minify: true, jsx: 'automatic', alias: { 'react-native': 'react-native-web' }, define: { 'process.env.NODE_ENV': '"production"', __DEV__: 'false' }, metafile: true, logLevel: 'info' }).then(async result => { const {writeFile} = await import('node:fs/promises'); await writeFile(join(root,'dist/metafile.json'),JSON.stringify(result.metafile,null,2)); });
await copyFile(join(root, 'index.html'), join(root, 'dist/index.html'));
