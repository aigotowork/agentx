/** Disposable synthetic labels supply for the actual management UI, never native acceptance. */
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export function createLabelsFixture(output) {
  const generation = 'labels-synthetic-zulo';
  const agents = [
    { name: 'alfa', incarnation: '18446744073709551001', stopped_event: null },
    { name: 'beta', incarnation: '18446744073709551002', stopped_event: null },
    { name: 'gama', incarnation: '18446744073709551003', stopped_event: null },
    { name: 'past', incarnation: null, stopped_event: '18446744073709551004' },
  ];
  const labels = new Map([['alfa', { project: 'atlas', 'plugin/state': 'pending', note: '<img src=x onerror=alert(1)>' }], ['beta', { project: 'comet' }], ['gama', {}]]);
  const revisions = new Map(agents.map(agent => [agent.name, 'r1']));
  const operations = new Map();
  const posts = [];
  const queries = [];
  const state = { mode: 'normal', supported: true };
  const view = agent => ({ agent, labels: labels.get(agent.name) ?? null, labels_revision: labels.has(agent.name) ? revisions.get(agent.name) : null, tool: 'codex', status: agent.incarnation ? 'listening' : 'stopped', status_context: '', stored_status: agent.incarnation ? 'listening' : 'stopped', detail: 'Synthetic labels preview', directory: '/synthetic/labels', title: null, session_id: null, file_bindings: [], collaboration: { status: 'legacy', workspace: null, role: null, reason: null }, can_send: false, can_stop: false, can_resume: false, has_transcript: false });
  const snapshot = agent => ({ agent, generation, labels: labels.get(agent.name), revision: revisions.get(agent.name) });
  const server = createServer(async (request, response) => {
    const url = new URL(request.url, 'http://fixture');
    const send = (body, status = 200) => { response.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' }); response.end(JSON.stringify(body)); };
    try {
      if (!url.pathname.startsWith('/api/')) {
        const file = url.pathname === '/' ? 'index.html' : url.pathname.slice(1);
        if (!/^[a-zA-Z0-9.-]+$/.test(file)) { response.writeHead(404); return response.end(); }
        const bytes = await readFile(join(output, file));
        response.writeHead(200, { 'Cache-Control': 'no-store', 'Content-Type': file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : 'text/html' }); return response.end(bytes);
      }
      if (url.pathname === '/api/v1/management') return send({ generation, scope: '/synthetic', project: 'SYNTHETIC PREVIEW · labels', tools: [], profiles: [], max_page: 2, max_text: 65536, ...(state.supported ? { supports_labels: true } : {}) });
      if (request.method === 'POST') {
        let raw = ''; for await (const chunk of request) raw += chunk;
        const body = JSON.parse(raw); posts.push({ body, raw });
        assert.equal(body.generation, generation);
        assert.deepEqual(body.agent, agents.find(agent => agent.name === body.agent.name));
        if (operations.has(body.request_id)) return send(operations.get(body.request_id), 202);
        if (state.mode === 'drop-before') { request.socket.destroy(); return; }
        const current = labels.get(body.agent.name);
        assert(current && state.supported);
        let operation;
        if (body.if_revision !== revisions.get(body.agent.name)) return send({ error: { code: 'Conflict', message: 'revision_conflict' } }, 409);
        else {
          const next = { ...current, ...body.set }; body.remove.forEach(key => delete next[key]);
          labels.set(body.agent.name, next); revisions.set(body.agent.name, `r${Number(revisions.get(body.agent.name).slice(1)) + 1}`);
          operation = { request_id: body.request_id, generation, status: state.mode === 'unknown' ? 'Unknown' : 'Succeeded', result: state.mode === 'unknown' ? null : { LabelsPatched: { snapshot: snapshot(body.agent), changed: true } }, error: null };
        }
        operations.set(body.request_id, operation);
        if (state.mode === 'drop-after') { state.mode = 'normal'; request.socket.destroy(); return; }
        return send(operation, 202);
      }
      assert.equal(url.searchParams.get('generation'), generation);
      if (url.pathname.startsWith('/api/v1/operations/')) { const operation = operations.get(url.pathname.split('/').at(-1)); if (!operation && state.mode === 'drop-before') state.mode = 'normal'; return operation ? send(operation) : send({ error: { code: 'NotFound', message: 'Operation not found' } }, 404); }
      if (url.pathname === '/api/v1/workspaces') return send({ generation, workspaces: [], coverage: { agent_pages: 1, agent_limit_reached: false, directories_examined: 0, scan_limit_reached: false, errors: [] } });
      if (url.pathname === '/api/v1/roles') return send({ generation, workspace: null, roles: [] });
      if (url.pathname === '/api/v1/agents') {
        queries.push(url);
        const selectors = url.searchParams.getAll('label');
        let rows = agents.filter(agent => !selectors.length || agent.incarnation);
        for (const selector of selectors) {
          const match = selector.match(/^([^!=]+)(!=|=)(.*)$/);
          rows = rows.filter(agent => { const map = labels.get(agent.name) ?? {}; if (match) return match[2] === '=' ? map[match[1]] === match[3] : map[match[1]] !== match[3]; return selector.startsWith('!') ? !Object.hasOwn(map, selector.slice(1)) : Object.hasOwn(map, selector); });
        }
        const offset = Number(url.searchParams.get('after') ?? 0);
        return send({ generation, current_complete: offset + 2 >= rows.length, agents: rows.slice(offset, offset + 2).map(view), next: offset + 2 < rows.length ? String(offset + 2) : null });
      }
      const agent = agents.find(agent => url.pathname.split('/')[4] === agent.name);
      if (!agent) return send({ error: { code: 'NotFound', message: 'Synthetic resource not found' } }, 404);
      assert.equal(url.searchParams.get(agent.incarnation ? 'incarnation' : 'stopped_event'), agent.incarnation ?? agent.stopped_event);
      if (url.pathname.endsWith('/labels')) return send(snapshot(agent));
      if (url.pathname.endsWith('/messages')) return send({ generation, messages: [], next_after: '0', has_more: false });
      return send(view(agent));
    } catch (error) { send({ error: { code: 'Internal', message: error.message } }, 500); }
  });
  return { server, state, labels, revisions, operations, posts, queries, agents };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const fixture = createLabelsFixture(process.argv[2]);
  fixture.server.listen(Number(process.argv[3] ?? 18973), '0.0.0.0', () => process.stdout.write(`Synthetic management labels preview http://0.0.0.0:${fixture.server.address().port}\n`));
}
