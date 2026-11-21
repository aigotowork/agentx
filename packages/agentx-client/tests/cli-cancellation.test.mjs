import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { once } from 'node:events';

test('CLI SIGINT preserves observed receipt and original identity on stderr', { timeout: 10000 }, async () => {
  const identity = { generation: 'original-generation', request_id: 'original-request' };
  const receipt = { ...identity, status: 'Running', result: null, error: null };
  const requests = [];
  let child;
  const server = createServer((request, response) => {
    requests.push({ method: request.method, url: request.url });
    if (requests.length === 2) { child.kill('SIGINT'); return; }
    response.writeHead(200, { 'Content-Type': 'application/json' }); response.end(JSON.stringify(receipt));
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  let watchdog;
  try {
    child = spawn(process.execPath, [new URL('../dist/cli.js', import.meta.url).pathname,
      'operations', 'wait', identity.request_id, '--generation', identity.generation,
      '--management-url', `http://127.0.0.1:${server.address().port}`, '--poll-seconds', '0.01'],
      { stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, AGENTX_API_CONFIG: undefined } });
    let stdout = '', stderr = '';
    child.stdout.setEncoding('utf8').on('data', chunk => { stdout += chunk; });
    child.stderr.setEncoding('utf8').on('data', chunk => { stderr += chunk; });
    watchdog = setTimeout(() => child.kill('SIGKILL'), 5000);
    const [code, signal] = await once(child, 'close');
    assert.equal(signal, null); assert.equal(code, 130); assert.equal(stdout, '');
    const error = JSON.parse(stderr);
    assert.equal(error.code, 'Canceled'); assert.deepEqual(error.identity, identity); assert.deepEqual(error.lastOperation, receipt);
    assert.equal(requests.length, 2); assert.ok(requests.every(request => request.method === 'GET'));
  } finally {
    clearTimeout(watchdog);
    if (child && child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
    server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
  }
});
