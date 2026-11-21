/** Build only fresh /space/ assets; execution and embedding belong to integration. */
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const target = process.argv[2];
const runner = fileURLToPath(new URL('./scripts/run.mjs', import.meta.url));
const result = spawnSync(process.execPath, [runner, 'build', target ?? ''], { stdio: 'inherit', env: { ...process.env, AGENT_SPACE_BASE: '/space/' } });
if (result.error) throw result.error;
process.exit(result.status ?? 1);
