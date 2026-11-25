/** Isolated, synthetic deployment wire boundary for browser tests and manual preview. */
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export const demoAdminToken = 'AdminTest123';
const prefix = '/api/v1/admin';

export function createDeploymentFixture(output) {
  const now = 1795334400;
  const hosts = Array.from({ length: 22 }, (_, index) => ({ name: `runner-${String(index + 1).padStart(2, '0')}`, online: index < 18, last_seen: now - index * 37 }));
  const agents = Array.from({ length: 21 }, (_, index) => ({ name: `worker-${String(index + 1).padStart(2, '0')}`, host: hosts[index].name, incarnation: `AgentRun${String(1600 + index)}`, workspace: { kind: 'unexposed' } }));
  const makeNamespace = (id, version = 1) => ({ namespace_id: id, namespace_url: `https://deployment.example/namespaces/${id}`, credential_version: version, generation: `fixture-generation-${id}`, created_at: now, hosts: 0, online_hosts: 0, agents: 0, available: true });
  const namespaces = new Map([
    ['atlas', { ...makeNamespace('atlas', 4), hosts: hosts.length, online_hosts: 18, agents: agents.length }],
    ['field-notes', makeNamespace('field-notes')],
    ['quarantine', { ...makeNamespace('quarantine'), available: false }],
    ...Array.from({ length: 20 }, (_, i) => [`stage-${String(i + 1).padStart(2, '0')}`, makeNamespace(`stage-${String(i + 1).padStart(2, '0')}`)]),
  ]);
  const writes = [], requests = [], receipts = new Map(), operations = new Map(), controlWrites = [];
  const state = { dropNextWrite: false, dropControlReply: false, failHosts: false, protocol: 2 };
  const server = createServer(async (request, response) => {
    const url = new URL(request.url, 'http://fixture');
    const send = (value, status = 200) => {
      response.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
      response.end(JSON.stringify(value));
    };
    const fail = (status, code) => send({ code, message: code }, status);
    const page = (items, key, generation) => {
      const after = url.searchParams.get('after');
      const size = Number(url.searchParams.get('limit') || 20);
      const remaining = after ? items.filter(item => item[key] > after) : items;
      const entries = remaining.slice(0, size);
      return send({ generation, items: entries, next: remaining.length > size ? entries.at(-1)[key] : null });
    };
    try {
      if (url.pathname === '/') { response.writeHead(302, { Location: '/deployment' }); response.end(); return; }
      if (!url.pathname.startsWith('/api/')) {
        const file = ['/deployment', '/deployment/'].includes(url.pathname) ? 'index.html' : url.pathname.slice(1);
        if (!/^[a-zA-Z0-9.-]+\.(?:html|js|css)$/.test(file)) { response.writeHead(404).end(); return; }
        const bytes = await readFile(join(output, file));
        response.writeHead(200, {
          'Content-Type': file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : 'text/html',
          'Cache-Control': 'no-store', 'Content-Security-Policy': "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'",
        });
        response.end(bytes);
        return;
      }
      requests.push({ path: url.pathname, method: request.method, after: url.searchParams.get('after'), host: url.searchParams.get('host'), generation: url.searchParams.get('generation'), authorized: request.headers.authorization === `Bearer ${demoAdminToken}` });
      if (!requests.at(-1).authorized) return fail(401, 'Unauthorized');
      if (url.pathname === `${prefix}/overview`) return send({ deployment_id: 'fixture-deployment', protocol_version: state.protocol, observed_at: now, namespaces: namespaces.size, hosts: hosts.length, online_hosts: 18, agents: agents.length, unavailable: 1 });
      if (url.pathname === `${prefix}/namespaces`) return page([...namespaces.values()].sort((a, b) => a.namespace_id.localeCompare(b.namespace_id)), 'namespace_id');
      const hostMatch = url.pathname.match(/^\/api\/v1\/admin\/namespaces\/atlas\/hosts\/([^/]+)\/capabilities$/);
      if (hostMatch) {
        const host = hosts.find(entry => entry.name === hostMatch[1]);
        if (!host) return fail(404, 'not_found');
        return send({ host: host.name, generation: namespaces.get('atlas').generation, installation_id: `installation-${host.name}`,
          online: host.online, observed_at: now, project_roots: ['default'], workspaces: [{ workspace_id: 'WSStudio0001', name: 'studio', default_available: true, default_reason: null,
            roles: [{ name: 'reviewer', available: true, reason: null }, { name: 'operator', available: false, reason: 'tool_unavailable' }] }] });
      }
      const operationMatch = url.pathname.match(/^\/api\/v1\/admin\/namespaces\/atlas\/operations(?:\/([^/]+))?$/);
      if (operationMatch) {
        if (request.method === 'GET') {
          const receipt = operations.get(decodeURIComponent(operationMatch[1]));
          return receipt ? send(receipt) : fail(404, 'not_found');
        }
        let raw = '';
        for await (const chunk of request) raw += chunk;
        const input = JSON.parse(raw);
        controlWrites.push(input);
        const receipt = { ...input, status: 'queued', claim_id: null, outcome: null };
        operations.set(input.request_id, receipt);
        if (state.dropControlReply) {
          state.dropControlReply = false;
          response.writeHead(200, { 'Content-Type': 'application/json', 'Content-Length': 1000 });
          response.write('{"request_id":');
          setTimeout(() => request.socket.destroy(), 20);
          return;
        }
        return send(receipt);
      }
      const match = url.pathname.match(/^\/api\/v1\/admin\/namespaces\/([^/]+)(?:\/(hosts|agents|token-rotations))?$/);
      if (!match) return fail(404, 'NotFound');
      const [, id, sub] = match;
      const namespace = namespaces.get(id);
      if ((request.method === 'PUT' && !sub) || (request.method === 'POST' && sub === 'token-rotations')) {
        let raw = '';
        for await (const chunk of request) raw += chunk;
        const body = JSON.parse(raw);
        writes.push({ id, method: request.method, body });
        const retained = receipts.get(body.request_id);
        if (retained) return retained.id === id && retained.raw === raw ? send(retained.receipt) : fail(409, 'RequestConflict');
        if (!sub && namespace) return fail(409, 'NamespaceExists');
        if (sub && namespace?.credential_version !== body.expected_version) return fail(409, 'VersionConflict');
        const version = sub ? namespace.credential_version + 1 : 1;
        const next = { ...(namespace ?? makeNamespace(id, version)), credential_version: version };
        namespaces.set(id, next);
        const { namespace_url, generation } = next;
        const receipt = { namespace_id: id, namespace_url, generation, credential_version: version };
        receipts.set(body.request_id, { id, raw, receipt });
        if (state.dropNextWrite) {
          state.dropNextWrite = false;
          response.writeHead(200, { 'Content-Type': 'application/json', 'Content-Length': 1000 });
          response.write('{"id":');
          setTimeout(() => request.socket.destroy(), 20);
          return;
        }
        return send(receipt);
      }
      if (!namespace) return fail(404, 'NotFound');
      if (['hosts', 'agents'].includes(sub) && url.searchParams.get('generation') !== namespace.generation) return fail(409, 'stale_generation');
      if (sub === 'hosts' && state.failHosts) return fail(503, 'Unavailable');
      if (sub === 'hosts') return page(id === 'atlas' ? hosts : [], 'name', namespace.generation);
      if (sub === 'agents') {
        const host = url.searchParams.get('host');
        return page(id === 'atlas' ? agents.filter(agent => !host || agent.host === host) : [], 'name', namespace.generation);
      }
      return send(namespace);
    } catch {
      if (!response.headersSent) fail(500, 'FixtureFailure'); else response.end();
    }
  });
  return { server, state, namespaces, writes, requests, operations, controlWrites };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [, , output, port = '18797'] = process.argv;
  if (!output) throw new Error('Pass a built Management directory or stable symlink');
  createDeploymentFixture(output).server.listen(Number(port), '127.0.0.1', () => {
    console.log(`Synthetic deployment: http://127.0.0.1:${port}/deployment (fixture admin token: ${demoAdminToken})`);
  });
}
