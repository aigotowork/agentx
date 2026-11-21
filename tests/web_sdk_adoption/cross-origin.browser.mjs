/**
 * Exercise actual browser CORS enforcement from a separate local origin.
 * Config is an evidence-owned JSON file; requests must use disposable fixture data.
 */
import assert from 'node:assert/strict';
import {mkdir, writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';

const configPath = process.argv[2];
const evidence = process.argv[3];
assert(configPath && evidence, 'usage: node cross-origin.browser.mjs CONFIG.json EVIDENCE_DIR');
const config = JSON.parse(await readFile(configPath, 'utf8'));
assert(Array.isArray(config.requests) && config.requests.length > 0, 'requests must be a non-empty array');
await mkdir(evidence, {recursive: false});

const driverPath = process.env.AGENTX_PLAYWRIGHT_PACKAGE ?? '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright';
const {chromium} = createRequire(import.meta.url)(driverPath);
const pageServer = createServer((request, response) => {
  response.writeHead(200, {'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store'});
  response.end('<!doctype html><meta charset="utf-8"><title>AgentX cross-origin acceptance</title><main id="result">ready</main>');
});
await new Promise((resolve, reject) => pageServer.listen(0, '0.0.0.0', resolve).once('error', reject));
const pagePort = pageServer.address().port;
const origin = `http://localhost:${pagePort}`;
const targets = config.requests.map(request => new URL(request.url));
assert(targets.every(target => target.origin !== origin), 'every API URL must be cross-origin from the browser page');

const browser = await chromium.launch({executablePath: process.env.AGENTX_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',
  headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-proxy-server']});
const context = await browser.newContext();
const page = await context.newPage();
const devtools = await context.newCDPSession(page);
await devtools.send('Network.enable');
const exchanges = [];
const responseCaptures = [];
const pageErrors = [];
devtools.on('Network.requestWillBeSent', event => {
  if (event.request.method === 'OPTIONS' && targets.some(target => target.origin === new URL(event.request.url).origin)) {
    exchanges.push({kind: 'request', method: event.request.method, url: event.request.url, headers: event.request.headers});
  }
});
devtools.on('Network.responseReceived', event => {
  if (event.requestId && targets.some(target => target.origin === new URL(event.response.url).origin) && event.response.status >= 200 && event.response.status < 300) {
    const request = exchanges.find(item => item.kind === 'request' && item.method === 'OPTIONS' && item.url === event.response.url);
    if (request) exchanges.push({kind: 'response', method: 'OPTIONS', url: event.response.url, status: event.response.status, headers: event.response.headers});
  }
});
page.on('pageerror', error => pageErrors.push(error.message));
page.on('request', request => {
  const url = new URL(request.url());
  if (targets.some(target => target.origin === url.origin)) {
    exchanges.push({kind: 'request', method: request.method(), url: request.url(), headers: request.headers()});
  }
});
page.on('response', response => {
  const url = new URL(response.url());
  if (targets.some(target => target.origin === url.origin)) {
    responseCaptures.push(response.allHeaders().then(headers => exchanges.push({kind: 'response',
      method: response.request().method(), url: response.url(), status: response.status(), headers})));
  }
});

let failure;
try {
  await page.goto(origin, {waitUntil: 'load'});
  const results = await page.evaluate(async requests => {
    const observed = [];
    for (const item of requests) {
      try {
        const response = await fetch(item.url, {method: item.method ?? 'GET', headers: item.headers ?? {},
          body: item.body ?? undefined, credentials: 'omit', redirect: 'error'});
        const body = await response.text();
        observed.push({name: item.name, ok: response.ok, status: response.status,
          contentType: response.headers.get('content-type'), body: body.slice(0, 4096)});
      } catch (error) {
        observed.push({name: item.name, error: String(error)});
      }
    }
    return observed;
  }, config.requests);
  await Promise.all(responseCaptures);

  assert.equal(results.length, config.requests.length);
  for (let index = 0; index < results.length; index++) {
    const expected = config.requests[index];
    const result = results[index];
    assert.equal(result.error, undefined, `${expected.name}: browser rejected cross-origin fetch: ${result.error}`);
    assert((expected.expectedStatuses ?? [200]).includes(result.status), `${expected.name}: unexpected status ${result.status}: ${result.body}`);
    if (expected.expectedContentType) assert.equal(result.contentType, expected.expectedContentType, expected.name);
    const response = exchanges.find(item => item.kind === 'response' && item.method === (expected.method ?? 'GET').toUpperCase() &&
      new URL(item.url).href === new URL(expected.url).href);
    assert(response, `${expected.name}: browser response was not captured`);
    const allowOrigin = response.headers['access-control-allow-origin'];
    assert(allowOrigin === '*' || allowOrigin === origin, `${expected.name}: invalid response allow-origin ${allowOrigin}`);
  }

  for (const expected of config.requests) {
    const target = new URL(expected.url);
    const method = (expected.method ?? 'GET').toUpperCase();
    const hasPreflight = method !== 'GET' && method !== 'HEAD' && method !== 'POST' ||
      Object.entries(expected.headers ?? {}).some(([name, value]) => name.toLowerCase() !== 'content-type' || !['text/plain', 'application/x-www-form-urlencoded', 'multipart/form-data'].includes(String(value).toLowerCase()));
    if (!hasPreflight) continue;
    const preflight = exchanges.find(item => item.kind === 'response' && item.method === 'OPTIONS' && new URL(item.url).origin === target.origin);
    assert(preflight, `${expected.name}: browser did not receive an OPTIONS preflight response`);
    assert(preflight.status >= 200 && preflight.status < 300, `${expected.name}: preflight status ${preflight.status}`);
    const headers = preflight.headers;
    assert(headers['access-control-allow-origin'] === '*' || headers['access-control-allow-origin'] === origin,
      `${expected.name}: invalid preflight allow-origin ${headers['access-control-allow-origin']}`);
    const allowedMethods = (headers['access-control-allow-methods'] ?? '').toUpperCase().split(/\s*,\s*/);
    assert(allowedMethods.includes(method) || allowedMethods.includes('*'), `${expected.name}: preflight does not allow ${method}`);
    const requested = Object.entries(expected.headers ?? {}).map(([name, value]) => [name.toLowerCase(), String(value).toLowerCase()])
      .filter(([name, value]) => !['accept', 'accept-language', 'content-language'].includes(name) &&
        (name !== 'content-type' || !['text/plain', 'application/x-www-form-urlencoded', 'multipart/form-data'].includes(value)))
      .map(([name]) => name);
    const allowedHeaders = (headers['access-control-allow-headers'] ?? '').toLowerCase().split(/\s*,\s*/);
    for (const name of requested) assert(allowedHeaders.includes(name) || allowedHeaders.includes('*'), `${expected.name}: preflight does not allow header ${name}`);
  }
  assert.deepEqual(pageErrors, []);
  await writeFile(`${evidence}/browser-result.json`, JSON.stringify({status: 'passed', browser: browser.version(), origin,
    requests: config.requests, results, exchanges, pageErrors,
    limits: 'Direct browser Fetch and CORS/preflight only; application UI/SDK integration is evidenced separately.'}, null, 2) + '\n');
} catch (error) {
  failure = {message: error.message, stack: error.stack};
  throw error;
} finally {
  await writeFile(`${evidence}/browser-result.json`, JSON.stringify({status: failure ? 'failed' : 'passed', browser: browser.version(),
    origin, requests: config.requests, exchanges, pageErrors, failure,
    limits: 'Direct browser Fetch and CORS/preflight only; application UI/SDK integration is evidenced separately.'}, null, 2) + '\n');
  await Promise.allSettled([context.close(), browser.close(), new Promise(resolve => pageServer.close(resolve))]);
}
