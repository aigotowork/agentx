/** Start or switch only the Space synthetic LAN preview. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const unit = 'agentx-space-conversation-lan-preview.service';
const host = '192.168.2.178', port = '18983', base = 'http://' + host + ':' + port;
const artifacts = resolve(root, '.artifacts/space-conversation-workbench');
const baseline = {
  build: resolve(root, '.artifacts/space-canvas-workspace/experience-r3'),
  fixture: resolve(artifacts, 'rollback/workspace.fixture.mjs'),
  scenario: 'long',
};
const previousFixture = resolve(root, 'web/agent-space/tests/workspace.fixture.mjs');

const [mode, path] = process.argv.slice(2);
assert(['promote', 'restore', 'rollback'].includes(mode) && (mode === 'rollback' || path), 'usage: node preview.mjs (promote|restore) ARTIFACT_DIR | rollback');
const candidate = path && { build: resolve(path, 'space'), fixture: resolve(path, 'workspace.fixture.mjs'), scenario: 'workbench' };
const selected = mode === 'rollback' ? baseline : candidate;
const run = (command, args) => execFileSync(command, args, { cwd: root, encoding: 'utf8', timeout: 10000 }).trim();
const config = () => run('systemctl', ['--user', 'show', unit, '-p', 'ExecStart', '--value']);
const stop = () => run('systemctl', ['--user', 'stop', unit]);
const start = item => run('systemd-run', [
  '--user', '--collect', '--unit', unit, '--working-directory', root, '--description',
  'AgentX Space conversation preview (synthetic)', process.execPath, item.fixture, item.build, port, host, item.scenario,
]);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function embeddedAssets(html) {
  return [...html.matchAll(/(?:src|href)="([^"]+\.(?:js|css))"/g)]
    .map(match => new URL(match[1], base + '/space/'))
    .filter(address => address.origin === base && address.pathname.startsWith('/space/assets/'))
    .map(address => address.pathname);
}
async function assets(item) {
  const html = await readFile(resolve(item.build, 'index.html'), 'utf8');
  const paths = embeddedAssets(html);
  assert(paths.some(path => path.endsWith('.js')) && paths.some(path => path.endsWith('.css')), 'expected a /space/ product build');
  return Object.fromEntries(await Promise.all(paths.map(async path => [path, sha(await readFile(resolve(item.build, path.slice('/space/'.length))))])));
}
async function fetchOk(path) {
  const response = await fetch(base + path, { signal: AbortSignal.timeout(1500) });
  assert(response.ok, path + ' returned ' + response.status);
  return response;
}
async function verify(item, wanted) {
  let last;
  for (let attempt = 0; attempt < 16; attempt += 1) {
    try {
      const descriptor = await (await fetchOk('/api/v1/management')).json();
      assert.equal(descriptor.scope, '/fixture/space-workspace');
      assert.equal(descriptor.supports_message_history === true, item.scenario === 'workbench');
      const served = embeddedAssets(await (await fetchOk('/space/')).text());
      for (const [path, expected] of Object.entries(wanted)) {
        assert(served.includes(path), 'running HTML does not name ' + path);
        assert.equal(sha(Buffer.from(await (await fetchOk(path)).arrayBuffer())), expected);
      }
      return { url: base + '/space/', assets: wanted, scenario: item.scenario, unit };
    } catch (error) {
      last = error;
      await delay(250);
    }
  }
  throw last;
}
const active = config();
if (mode === 'promote') {
  assert(active.includes(baseline.build) && [baseline.fixture, previousFixture].some(fixture => active.includes(fixture)), 'the expected baseline is not the running unit');
} else if (mode === 'restore') {
  assert(!active, 'the preview unit already exists; restore only starts a missing preview');
} else {
  assert(active.includes(artifacts + '/') && active.includes('workbench'), 'the running unit is not the owned workbench preview');
}
const wanted = await assets(selected);
const rollbackAssets = mode === 'promote' ? await assets(baseline) : null;
if (mode === 'restore') {
  const recorded = JSON.parse(await readFile(resolve(artifacts, 'lan-adoption.json'), 'utf8'));
  assert.deepEqual(wanted, recorded.assets, 'restore must use the accepted r5 app assets');
}
assert((await readFile(selected.fixture, 'utf8')).includes('serveWorkspace'));
if (mode !== 'restore') stop();
try {
  start(selected);
  console.log(JSON.stringify(await verify(selected, wanted), null, 2));
} catch (cause) {
  if (mode === 'rollback') throw cause;
  if (config()) stop();
  if (mode === 'restore') throw cause;
  start(baseline);
  console.error('Restored the previous synthetic preview.');
  await verify(baseline, rollbackAssets);
  throw cause;
}
