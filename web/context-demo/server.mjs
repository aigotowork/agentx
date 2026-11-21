import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';

const publicRoot = fileURLToPath(new URL('./public/', import.meta.url));
const assets = new Map([
  ['/', ['index.html', 'text/html; charset=utf-8']],
  ['/app.js', ['app.js', 'text/javascript; charset=utf-8']],
  ['/bridge.js', ['bridge.js', 'text/javascript; charset=utf-8']],
  ['/scenario.js', ['scenario.js', 'text/javascript; charset=utf-8']],
  ['/style.css', ['style.css', 'text/css; charset=utf-8']],
  ['/layouts/workspace.css', ['layouts/workspace.css', 'text/css; charset=utf-8']],
]);

function serviceOrigin(value) {
  const url = new URL(value);
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.pathname !== '/' || url.search || url.hash) throw new Error('AGENTX_ORIGIN must be an HTTP(S) origin without credentials/path/query');
  return url.origin;
}

function json(response, status, value) {
  response.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8' });
  response.end(JSON.stringify(value));
}

/** Serves owned assets and one verified immutable package; native reads remain inside Space. */
export function createDemoServer(input = {}) {
  const config = { ...input, origin: serviceOrigin(input.origin ?? 'http://192.168.2.178:38023') };
  const extension = input.extension;
  if (extension && (!Buffer.isBuffer(extension.bytes) || createHash('sha256').update(extension.bytes).digest('hex') !== extension.sha256)) throw new Error('Extension package digest mismatch');
  const packageInfo = extension ? { filename: `agentx-browser-context-${extension.version}.zip`, version: extension.version, sha256: extension.sha256, bytes: extension.bytes.length } : null;
  return http.createServer(async (request, response) => {
    response.setHeader('Cache-Control', 'no-store');
    response.setHeader('X-Content-Type-Options', 'nosniff');
    response.setHeader('Referrer-Policy', 'strict-origin-when-cross-origin');
    try {
      if (request.method !== 'GET' && request.method !== 'HEAD') return json(response, 405, { error: '只支持 GET / HEAD' });
      const url = new URL(request.url, 'http://demo.invalid');
      if (url.search) return json(response, 400, { error: '此端点不接受查询参数' });
      if (url.pathname === '/api/config') return json(response, 200, { origin: config.origin, extension: packageInfo });
      if (url.pathname === '/downloads/agentx-browser-context.zip' && extension) {
        response.writeHead(200, { 'Content-Type': 'application/zip', 'Content-Disposition': `attachment; filename="${packageInfo.filename}"`, 'Content-Length': extension.bytes.length });
        return response.end(request.method === 'HEAD' ? undefined : extension.bytes);
      }
      const asset = assets.get(url.pathname);
      if (!asset) return json(response, 404, { error: '未找到页面' });
      const bytes = await readFile(resolve(publicRoot, asset[0]));
      response.writeHead(200, { 'Content-Type': asset[1], 'Content-Length': bytes.length });
      response.end(request.method === 'HEAD' ? undefined : bytes);
    } catch (error) {
      json(response, 502, { error: error.message || '服务暂不可用，请重试' });
    }
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const origin = process.env.AGENTX_ORIGIN ?? 'http://192.168.2.178:38023';
  const host = process.env.HOST ?? '0.0.0.0';
  const port = Number(process.env.PORT ?? 38024);
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('PORT must be between 1 and 65535');
  let extension;
  if (process.env.EXTENSION_ZIP && process.env.EXTENSION_META) {
    const meta = JSON.parse(await readFile(process.env.EXTENSION_META, 'utf8'));
    if (!/^\d+\.\d+\.\d+$/.test(meta.version) || !/^[a-f0-9]{64}$/.test(meta.sha256)) throw new Error('Invalid extension receipt');
    extension = { bytes: await readFile(process.env.EXTENSION_ZIP), sha256: meta.sha256, version: meta.version };
    if (extension.bytes.length !== meta.bytes) throw new Error('Extension package size mismatch');
  }
  const server = createDemoServer({ origin, extension });
  server.listen(port, host, () => console.log(JSON.stringify({ event: 'listening', pid: process.pid, host, port, origin, at: new Date().toISOString() })));
  server.on('error', error => { console.error(error.message); process.exitCode = 1; });
}
