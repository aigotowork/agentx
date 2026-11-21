/** Isolated Management API fixture shared by the real entry preview and browser scenarios. */
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export function createManagementFixture(output, { preview = false, crossOrigin = false, sameNameHistory = false } = {}) {
  let generation = 'generation-fixture';
  const live = { name: 'fava', incarnation: '18446744073709551601', stopped_event: null };
  const stopped = { name: 'luna', incarnation: null, stopped_event: '18446744073709551602' };
  const earlier = { name: live.name, incarnation: null, stopped_event: '18446744073709551610' };
  const previewAgents = preview ? [
    ['miko', 'listening', '核对接口文档', '/workspaces/atlas'],
    ['nora', 'active', '运行发布检查', '/workspaces/atlas'],
    ['tavi', 'blocked', '等待测试样本', '/workspaces/studio'],
    ['zena', 'error', '诊断模型请求', '/workspaces/studio'],
    ['lemo', 'launching', '准备回归环境', '/workspaces/atlas'],
  ].map(([name, status, title, workspace], i) => ({ agent: { name, incarnation: String(200 + i), stopped_event: null }, status, title, workspace })) : [];
  const role = { name: 'builder', path: '/workspace/agentx/.agentx/roles/builder.yaml', namespace: '/workspace/agentx', profile: 'builder', description: 'Fixture post', default: true };
  const entry = { generation, directory: '/workspace/agentx/sub', requested_workspace: '/workspace/agentx/sub', resolved_workspace: '/workspace/agentx', role, endpoint: { status: 'adoptable', participant: live, candidates: 1, reason: 'One compatible candidate' } };
  const view = (agent, isStopped = false) => ({ agent, tool: 'codex', status: isStopped ? 'stopped' : 'working', status_context: '', stored_status: isStopped ? 'stopped' : 'working', detail: isStopped ? 'Finished' : 'Building console', directory: '/workspace/agentx', session_id: 'fixture-session', file_bindings: [], collaboration: isStopped ? { status: 'legacy', workspace: null, role: null, reason: null } : { status: 'adopted', workspace: '/workspace/office', role, reason: null }, can_send: !isStopped, can_stop: !isStopped, can_resume: isStopped, has_transcript: true });
  const describeAgent = agent => {
    const row = view(agent, agent.stopped_event !== null || (agent === live && stoppedLive));
    const sample = previewAgents.find(item => item.agent === agent);
    if (sample) return { ...row, status: sample.status, title: sample.title, directory: sample.workspace, collaboration: { ...row.collaboration, workspace: sample.workspace } };
    return preview ? { ...row, title: agent === live ? '管理台迭代 · 交付核对' : '上一轮界面走查' } : row;
  };
  const operations = new Map();
  const posts = [];
  const state = { mode: 'normal', addedMessages: [] };
  let stoppedLive = false;
  const requests = [];
  const server = createServer(async (request, response) => {
    const url = new URL(request.url, 'http://fixture');
    const send = (body, status = 200) => { response.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store', ...(crossOrigin ? { 'Access-Control-Allow-Origin': '*' } : {}) }); response.end(JSON.stringify(body)); };
    requests.push(url);
    try {
      if (!url.pathname.startsWith('/api/')) {
        const file = url.pathname === '/' ? 'index.html' : url.pathname.slice(1);
        if (!/^[a-zA-Z0-9.-]+$/.test(file)) return response.end();
        const bytes = await readFile(join(output, file));
        response.writeHead(200, { 'Cache-Control': 'no-store', 'Content-Type': file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : 'text/html' }); response.end(bytes); return;
      }
      const knownAgents = [live, ...previewAgents.map(row => row.agent), stopped, ...(sameNameHistory ? [earlier] : [])];
      if (url.pathname === '/api/v1/management') return send({ generation, scope: '/fixture', project: preview ? '演示数据 · AgentX' : '/workspace/agentx', tools: ['codex'], profiles: [{ name: 'builder', tool: 'codex', description: preview ? '代码实现与检查' : 'Fixture builder' }], max_page: 100, max_text: 65536 });
      if (request.method === 'POST') {
        let raw = ''; for await (const chunk of request) raw += chunk;
        const body = JSON.parse(raw); posts.push({ path: url.pathname, raw, body });
        assert.equal(body.generation, generation);
        if (body.launch?.role) assert.match(body.launch.role, /^(?:(?:local|global):)?[a-zA-Z0-9][a-zA-Z0-9_-]*$/, 'Launch consumes native Role selectors, never source paths');
        if (operations.has(body.request_id)) return send(operations.get(body.request_id), 202);
        if (body.agent) assert(knownAgents.some(agent => agent.name === body.agent.name && agent.incarnation === body.agent.incarnation && agent.stopped_event === body.agent.stopped_event), 'Request must keep full execution identity');
        if (state.mode === 'drop-before') { request.socket.destroy(); return; }
        let result;
        if (url.pathname.endsWith('/transcript')) result = { Transcript: { agent: body.agent, available: true, reason: null, exchanges: [{ position: '18446744073709551603', at: '2026-11-15T10:00:00Z', user: 'Inspect fixture', assistant: 'Fixture transcript response', action: 'Read source', files: ['src/management/contract.rs'] }] } };
        else if (url.pathname === '/api/v1/directory-messages') result = { DirectorySent: { entry: { ...entry, generation, directory: body.directory }, recipient: live, sent: { message_id: '18446744073709551606', recipients: ['fava'] }, created: false } };
        else if (url.pathname.endsWith('/messages')) result = { Sent: { message_id: '18446744073709551604', recipients: [body.agent.name] } };
        else if (/\/(stop|kill)$/.test(url.pathname)) { if (body.agent.name === live.name) stoppedLive = true; result = { Stopped: { name: body.agent.name, outcome: 'sent' } }; }
        else result = { Launched: { agents: ['newa'], failed: 0, errors: [] } };
        const operation = { request_id: body.request_id, generation, status: state.mode === 'unknown' ? 'Unknown' : 'Succeeded', result: state.mode === 'unknown' ? null : result, error: null };
        operations.set(body.request_id, operation);
        if (state.mode === 'drop-after') { state.mode = 'normal'; request.socket.destroy(); return; }
        state.mode = 'normal'; return send(operation, 202);
      }
      assert.equal(url.searchParams.get('generation'), generation);
      if (url.pathname === '/api/v1/workspaces') return send({ generation, workspaces: [{ path: '/workspace/office', sources: ['adopted'] }, { path: '/workspace/empty', sources: ['marker'] }, ...[...new Set(previewAgents.map(row => row.workspace))].map(path => ({ path, sources: ['adopted', 'marker'] }))], coverage: { agent_pages: 1, agent_limit_reached: false, directories_examined: 8, scan_limit_reached: true, errors: ['Fixture bounded scan'] } });
      if (url.pathname === '/api/v1/roles') { const workspace = url.searchParams.get('workspace'); return workspace === '/workspace/broken' ? send({ error: { code: 'InvalidInput', message: 'Fixture invalid Role catalog' } }, 400) : send({ generation, workspace: workspace === 'global' ? null : workspace, roles: workspace === '/workspace/empty' ? [] : [role, ...(preview ? [{ ...role, name: 'reviewer', path: role.path.replace('builder', 'reviewer'), description: '复核交付与证据', profile: null, default: false }] : [])] }); }
      if (url.pathname === '/api/v1/directory-entry') return send({ ...entry, generation, directory: url.searchParams.get('directory') });
      if (url.pathname === '/api/v1/agents') return send({ generation, agents: knownAgents.map(describeAgent), current_complete: true, next: null });
      if (url.pathname.startsWith('/api/v1/operations/')) { const operation = operations.get(url.pathname.split('/').at(-1)); if (!operation && state.mode === 'drop-before') state.mode = 'normal'; return operation ? send(operation) : send({ error: { code: 'NotFound', message: 'Operation not found' } }, 404); }
      const agent = knownAgents.find(agent => url.pathname.split('/')[4] === agent.name && url.searchParams.get(agent.incarnation ? 'incarnation' : 'stopped_event') === (agent.incarnation ?? agent.stopped_event));
      if (!agent) return send({ error: { code: 'NotFound', message: 'Synthetic Agent not found' } }, 404);
      assert.equal(url.searchParams.get(agent.incarnation ? 'incarnation' : 'stopped_event'), agent.incarnation ?? agent.stopped_event);
      if (url.pathname.endsWith('/messages')) {
        const first = { id: '18446744073709551605', at: '2026-11-15T10:00:00Z', from: 'bigboss', text: 'Implement management console with recoverable operations. '.repeat(5), recipients: [agent.name], intent: 'request', reply_to: null, thread: null };
        const messages = preview && agent === live ? [
          { ...first, text: '请核对本次管理台交付：Agents 的消息草稿能否跨页面保留，以及工作区能否独立查看空目录的岗位。', thread: '管理台交付' },
          { ...first, id: '18446744073709551607', at: '2026-11-15T10:04:00Z', from: 'fava', recipients: ['bigboss'], text: '工作区已独立成页。正在核对断线恢复：请求 ID 保持不变，回执可从原页面打开。', intent: 'inform', reply_to: first.id, thread: '管理台交付' },
          { ...first, id: '18446744073709551609', at: '2026-11-15T10:12:00Z', text: '收到。请在手机尺寸再走一遍：选择 Agent → 回复 → 查看操作回执 → 返回草稿。', thread: '管理台交付' },
        ] : [first];
        if (agent === live) messages.push(...state.addedMessages);
        const after = url.searchParams.get('after');
        const offset = after ? messages.findIndex(message => message.id === after) + 1 : 0;
        return send({ generation, messages: messages.slice(offset), next_after: messages.at(-1).id, has_more: false });
      }
      return send(describeAgent(agent));
    } catch (error) { send({ error: { code: 'Internal', message: error.message } }, 500); }
  });
  return { server, live, stopped, role, requests, posts, state, setGeneration: value => { generation = value; } };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [output, port = '18792'] = process.argv.slice(2);
  if (!output) throw new Error('Pass the built Management directory or a stable symlink to it');
  createManagementFixture(output, { preview: true }).server.listen(Number(port), '0.0.0.0', () => {
    process.stdout.write(`Synthetic AgentX Management preview on port ${port}\n`);
  });
}
