/** Serve an isolated Space build with long conversations and deterministic synthetic SDK receipts. */
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { pathToFileURL } from 'node:url';

const scope = '/fixture/space-workspace';
const generation = 'space-workspace-fixture';
const project = '/projects/workspace';
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.webp': 'image/webp', '.json': 'application/json' };
const profiles = [{ name: 'review-fixture', tool: 'fixture', description: 'Synthetic review profile' }];
const sampleAttachmentText = 'Space 会话工作台样例：核对当前进展、范围与验收条目。\n';
const sampleAttachment = {
  id: 'fixture-notes', name: '验收笔记.txt', media_type: 'text/plain',
  size: Buffer.byteLength(sampleAttachmentText), url: '/api/v1/attachments/fixture-notes',
};
const initial = [
  ['luna', '101', '体验设计 · 对齐画布内工作流'],
  ['mora', '102', '前端实现 · 对照两套阅读布局'],
  ['sira', '103', '质量复核 · 窄屏与键盘'],
  ['tobi', '104', '交付协调 · 整理验收证据'],
];
const names = new Map(initial.map(([name, incarnation, title]) => [name, makeAgent(name, incarnation, title)]));

function makeAgent(name, incarnation, title) {
  return {
    agent: { name, incarnation, stopped_event: null }, title, status: 'listening', stored_status: 'listening',
    status_context: '', detail: '', request_error: null, pending_messages: 0,
    directory: project, tool: 'fixture', session_id: 'fixture-' + name,
    collaboration: { status: 'adopted', workspace: project, role: null, reason: null },
    can_send: true, can_stop: true, can_resume: false, has_transcript: false,
  };
}
function history(name, longHistory, workbench) {
  const count = longHistory ? ({ luna: 120, mora: 84, sira: 52, tobi: 5 }[name] ?? 18) : 18;
  return Array.from({ length: count }, (_, index) => ({
    id: String(index + 1), at: new Date(Date.UTC(2026, 10, 20, 12, index)).toISOString(),
    from: index % 2 ? '@' + name : 'bigboss', recipients: index % 2 ? ['bigboss'] : [name],
    attachments: workbench && index === count - 2 ? [sampleAttachment] : [],
    text: `[${name} / ${index + 1}] ${['比较阅读窗口的密度', '复核交互和精确会话', '整理从体验到交付的记录'][index % 3]}。\n\n` +
      '我们需要在同一个工作区里并排核对上下游的实际记录，标出结论、尚未处理的问题和下一条指令。' +
      (index % 3 === 0 ? '\n\n我复核了两个设计方向：紧凑工作台适合反复核对，安静阅读室适合读长讨论。两者使用同一组会话、草稿和阅读位置。接下来只需确认收起行为以及返回时的精确标识。' : '') +
      (longHistory && index % 5 === 2 ? '\n\n- 对照上次回执确认同一个任务\n- 记录边界状态、当前假设和下一步\n- 接续时保留相关的 Agent 与原稿' : '') +
      (longHistory && index % 7 === 3 ? '\n\n```text\nworkspace: product/space\ncheck: page-conversation-boundary\nrequest: compare_message_context_before_revision\n```' : ''),
  }));
}
function relation(agent) {
  return { name: agent.agent.name, incarnation: agent.agent.incarnation, availability: 'available', agent };
}

export async function serveWorkspace(build, port = 0, host = '127.0.0.1', { longHistory = false, workbench = false } = {}) {
  const root = resolve(build);
  const agents = new Map([...names].map(([name, agent]) => [name, workbench ? { ...agent, can_fork: name === 'luna' || name === 'mora' } : agent]));
  const calls = [], receipts = new Map(), forkSources = new Map();
  const messages = new Map([...agents.keys()].map(name => [name, history(name, longHistory || workbench, workbench)]));
  const server = createServer(async (request, response) => {
    const url = new URL(request.url, 'http://127.0.0.1');
    const json = (result, status = 200) => response.writeHead(status, { 'content-type': 'application/json', 'cache-control': 'no-store' }).end(JSON.stringify(result));
    const path = url.pathname;
    try {
      if (path.startsWith('/api/')) {
        const method = request.method;
        const call = { method, path, query: Object.fromEntries(url.searchParams) };
        calls.push(call);
        if (method === 'POST') {
          const input = [];
          for await (const part of request) input.push(part);
          const body = call.body = JSON.parse(Buffer.concat(input));
          if (body.generation !== generation) return json({ error: { code: 'StaleReference', message: '样例连接已变化' } }, 409);
          const target = path.match(/^\/api\/v1\/agents\/([^/]+)\/(messages|fork)$/);
          const source = target && agents.get(target[1]);
          let result;
          if (path === '/api/v1/agents') {
            const name = 'demo' + receipts.size, agent = makeAgent(name, String(200 + receipts.size), body.launch?.title || '新建样例 Agent');
            agents.set(name, agent);
            messages.set(name, []);
            result = { Launched: { agents: [name], agent_refs: [agent.agent], errors: [], failed: 0 } };
          } else if (target?.[2] === 'messages' && source && body.agent?.name === target[1] && body.agent.incarnation === source.agent.incarnation) {
            const items = messages.get(target[1]);
            const id = String(items.length + 1);
            items.push({ id, at: new Date().toISOString(), from: 'bigboss', recipients: [target[1]], attachments: [], text: body.message.text });
            result = { Sent: { message_id: id, recipients: [target[1]] } };
          } else if (workbench && target?.[2] === 'fork' && source?.can_fork && body.agent?.name === target[1] && body.agent.incarnation === source.agent.incarnation) {
            const name = 'fork' + receipts.size, agent = makeAgent(name, String(300 + receipts.size), '分叉复核 · ' + source.title);
            agents.set(name, agent);
            messages.set(name, []);
            forkSources.set(name, source.agent);
            result = { Launched: { agents: [name], agent_refs: [agent.agent], errors: [], failed: 0 } };
          } else return json({}, 404);
          const receipt = { generation, request_id: body.request_id, status: 'Succeeded', result };
          receipts.set(body.request_id, receipt);
          return json(receipt);
        }
        if (method !== 'GET') return json({}, 405);
        if (path === '/api/v1/management') return json({ scope, generation, project, profiles, tools: ['fixture'], max_page: 100, max_text: 65536, ...(workbench && { supports_message_history: true }) });
        if (workbench && path === sampleAttachment.url) {
          return response.writeHead(200, { 'content-type': 'text/plain; charset=utf-8', 'content-disposition': 'attachment; filename="review-notes.txt"' }).end(sampleAttachmentText);
        }
        if (path === '/api/v1/workspaces') return json({ generation, workspaces: [{ path: project, sources: ['adopted'] }], coverage: { errors: [], agent_limit_reached: false, scan_limit_reached: false, agent_pages: 1, directories_examined: 1 } });
        if (path === '/api/v1/roles') return json({ generation, workspace: url.searchParams.get('workspace') === 'global' ? null : url.searchParams.get('workspace'), roles: [] });
        if (path === '/api/v1/office-activity') return json({ generation, next_after: '0', has_more: false, messages: [], created: [], human_messages: [], human_has_more: false, human_unhandled_count: 0 });
        if (path === '/api/v1/agents') return json({ generation, agents: [...agents.values()], next: null, current_complete: true });
        const operation = path.match(/^\/api\/v1\/operations\/([^/]+)$/);
        if (operation) return receipts.has(operation[1]) ? json(receipts.get(operation[1])) : json({ error: { code: 'NotFound', message: '未观察到原操作回执' } }, 404);
        const exact = path.match(/^\/api\/v1\/agents\/([^/]+)(?:\/(messages|message-history|launch-relations))?$/);
        const agent = exact && agents.get(exact[1]);
        if (!agent || url.searchParams.get('incarnation') !== agent.agent.incarnation) return json({}, 404);
        if (exact[2] === 'messages') {
          const list = messages.get(exact[1]), after = Number(url.searchParams.get('after') ?? 0);
          const limit = workbench ? Number(url.searchParams.get('limit') ?? 20) : 12;
          const page = list.filter(item => Number(item.id) > after).slice(0, limit);
          return json({ generation, messages: page, has_more: list.length > after + page.length, next_after: page.at(-1)?.id ?? String(after) });
        }
        if (exact[2] === 'message-history') {
          if (!workbench) return json({}, 404);
          if (url.searchParams.get('generation') !== generation) return json({ error: { code: 'StaleReference', message: '样例连接已变化' } }, 409);
          const list = messages.get(exact[1]), before = url.searchParams.get('before');
          const limit = Number(url.searchParams.get('limit') ?? 20);
          if (!Number.isInteger(limit) || limit < 1 || limit > 100 || (before !== null && !/^[1-9]\d*$/.test(before))) return json({ error: { code: 'InvalidInput', message: '样例读取边界无效' } }, 400);
          const eligible = before === null ? list : list.filter(item => Number(item.id) < Number(before));
          const page = eligible.slice(-limit);
          return json({ generation, messages: page, next_before: eligible.length > page.length ? page[0].id : null, live_after: before === null ? list.at(-1)?.id ?? '0' : null });
        }
        if (exact[2] === 'launch-relations') {
          const upstream = exact[1] === 'mora' ? relation(agents.get('luna')) : null;
          const downstream = exact[1] === 'luna' ? [relation(agents.get('mora'))] : [];
          if (workbench && exact[1] === 'luna') {
            for (const [index, name] of ['nevo', 'lito', 'davu', 'sora', 'meru', 'zila'].entries()) {
              downstream.push({ name, incarnation: String(600 + index), availability: 'unavailable', launch_failure: index === 0 ? 'cancelled_before_ready' : null, agent: null });
            }
          }
          const after = Number(url.searchParams.get('after') ?? 0);
          const limit = Number(url.searchParams.get('limit') ?? 20);
          const forkSource = forkSources.get(exact[1]);
          return json({ generation, origin: upstream ? 'agent' : 'external', upstream,
            ...(forkSource && { fork_source: forkSource }),
            downstream: downstream.slice(after, after + limit), next: after + limit < downstream.length ? String(after + limit) : null });
        }
        return json(agent);
      }
      const relative = decodeURIComponent(path.replace(/^\/space\//, '') || 'index.html');
      const file = resolve(root, relative);
      if (!file.startsWith(root + '/')) { response.writeHead(404).end(); return; }
      let contents = await readFile(file);
      if (relative === 'index.html') contents = contents.toString().replace('name="agentx-space-scope"', `name="agentx-space-scope" content="${scope}"`);
      response.writeHead(200, { 'content-type': mime[extname(file)] ?? 'application/octet-stream' }).end(contents);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise((ok, fail) => { server.once('error', fail); server.listen(port, host, ok); });
  const { address, family, port: actualPort } = server.address();
  const authority = family === 'IPv6' ? `[${address}]` : address;
  return { base: `http://${authority}:${actualPort}/space/`, calls, close: () => new Promise(ok => server.close(ok)) };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [build, port, host, scenario] = process.argv.slice(2);
  if (!build || !port || (scenario && !['long', 'workbench'].includes(scenario))) throw new Error('Usage: node workspace.fixture.mjs BUILD PORT [BIND_ADDRESS] [long|workbench]');
  const fixture = await serveWorkspace(build, Number(port), host, { longHistory: scenario === 'long', workbench: scenario === 'workbench' });
  console.log(`Space workspace fixture (synthetic data only): ${fixture.base}`);
}
