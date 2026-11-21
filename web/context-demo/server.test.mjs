import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { createHash } from 'node:crypto';
import { createDemoServer } from './server.mjs';

async function listen(server) {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  return `http://127.0.0.1:${server.address().port}`;
}
async function close(server) { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }

test('host exposes configuration and full-Space shell without native discovery or mutation forwarding', async t => {
  let upstreamReads = 0;
  const native = http.createServer((request, response) => { upstreamReads++; response.end('{}'); });
  const origin = await listen(native);
  const demo = createDemoServer({ origin });
  const base = await listen(demo);
  t.after(async () => { await close(demo); await close(native); });
  assert.equal((await (await fetch(base + '/api/config')).json()).origin, origin);
  for (const [path, init, expected] of [
    ['/api/discovery', {}, 404],
    ['/api/discovery?origin=http://untrusted.invalid', {}, 400],
    ['/api/config', { method: 'POST' }, 405],
    ['/api/v1/agents', {}, 404],
    ['/.git/config', {}, 404],
    ['/README.md', {}, 404],
    ['/downloads/other.zip', {}, 404],
  ]) assert.equal((await fetch(base + path, init)).status, expected);
  assert.equal(upstreamReads, 0);
  const html = await (await fetch(base)).text();
  assert.match(html, /id="launcher"/);
  assert.match(html, /id="assistant"[^>]*hidden/);
  assert.doesNotMatch(html, /id="(?:target|attachments|add-files|refresh-targets)"/);
});

test('does not expose a package or repository files', async t => {
  const demo = createDemoServer({ origin: 'http://127.0.0.1:9' });
  const base = await listen(demo);
  t.after(async () => { await close(demo); });
  const config = await (await fetch(base + '/api/config')).json();
  assert.equal(config.extension, null);
  assert.equal((await fetch(base + '/downloads/agentx-browser-context.zip')).status, 404);
  assert.equal((await fetch(base + '/downloads/')).status, 404);
  assert.equal((await fetch(base + '/package.json')).status, 404);
  assert.throws(() => createDemoServer({ origin: 'http://example.com/path' }), /origin/);
});

test('only the verified package bytes are served with exact receipt metadata', async t => {
  const bytes = Buffer.from('PK verified package fixture');
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  const extension = { bytes, sha256, version: '1.2.1' };
  assert.throws(() => createDemoServer({ extension: { ...extension, sha256: '0'.repeat(64) } }), /digest mismatch/);
  const demo = createDemoServer({ extension });
  const base = await listen(demo);
  t.after(async () => { await close(demo); });
  const config = await (await fetch(base + '/api/config')).json();
  assert.deepEqual(config.extension, { filename: 'agentx-browser-context-1.2.1.zip', version: '1.2.1', sha256, bytes: bytes.length });
  const download = await fetch(base + '/downloads/agentx-browser-context.zip');
  assert.equal(download.status, 200);
  assert.match(download.headers.get('content-disposition'), /attachment; filename="agentx-browser-context-1.2.1.zip"/);
  assert.deepEqual(Buffer.from(await download.arrayBuffer()), bytes);
  assert.equal((await fetch(base + '/downloads/agentx-browser-context-1.1.0.zip')).status, 404);
  assert.equal((await fetch(base + '/downloads/')).status, 404);
});
