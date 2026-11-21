import { copyFile, rm } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
await rm(`${root}/dist`, { recursive: true, force: true });
execFileSync(process.execPath, [`${root}/node_modules/typescript/bin/tsc`, '-p', `${root}/tsconfig.json`], { stdio: 'inherit' });
await copyFile(new URL('../../../LICENSE', import.meta.url), `${root}/dist/LICENSE`);
