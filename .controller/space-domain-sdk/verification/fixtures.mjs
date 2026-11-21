/** Native-shaped wire data for the independent Space browser run. No live listener is contacted. */
export const listener = 'http://127.0.0.1:19741';
export const scope = '/fixture/agentx-space-sdk/.agentx';
export const generation = 'space-sdk-fixture-generation';
export const office = '/fixture/agentx-space-sdk/studio';
export const otherOffice = '/fixture/agentx-space-sdk/archive';
const descriptor = { scope, generation, project: '/fixture/agentx-space-sdk', tools: [], profiles: [], max_page: 100, max_text: 65536 };
const role = { name: 'illustrator', path: '/fixture/roles/illustrator.toml', namespace: 'global:fixture', profile: 'fixture:illustrator', description: '场景插画', default: false };
const agent = (name, workspace, incarnation, stoppedEvent = null) => ({
  agent: { name, incarnation, stopped_event: stoppedEvent }, title: stoppedEvent ? '上一班次' : '场景插画',
  status: stoppedEvent ? 'stopped' : 'listening', stored_status: stoppedEvent ? 'stopped' : 'listening', status_context: '',
  collaboration: { status: 'adopted', workspace, role }, directory: '/fixture/cwd/independent-of-office',
  tool: 'fixture', detail: '', session_id: 'fixture-session', can_send: !stoppedEvent, can_stop: !stoppedEvent,
  can_resume: false, has_transcript: false, pending_messages: 0,
});
const current = agent('luna', office, '9007199254740999');
const stopped = agent('luna', office, null, '170');
const external = agent('mora', otherOffice, '701');
export const currentId = JSON.stringify([scope, generation, current.agent.name, current.agent.incarnation, null, 'current']);
export const stoppedId = JSON.stringify([scope, generation, stopped.agent.name, null, stopped.agent.stopped_event, 'stopped']);
export const messageText = ['本班次：木作台已整理好。', '本班次：我把线稿放在白板上了。', '上一班次：前一版草图已经归档。'];
const message = (index) => ({ id: String(100 + index), at: new Date(1730000000000 + index * 60_000).toISOString(), from: 'bigboss', recipients: ['luna'], text: messageText[index] });

/** Record every call, enforce native request discriminators, and make every important page distinguishable. */
export function fixture({ describedScope = scope } = {}) {
  const calls = [], violations = [];
  let servedGeneration = generation;
  let allowReplacement = false;
  function reject(reason) {
    violations.push(reason);
    return { status: 400, json: { error: { code: 'invalid_input', message: reason } } };
  }
  function respond(request) {
    const url = new URL(request.url());
    const method = request.method(), path = url.pathname, query = Object.fromEntries(url.searchParams);
    calls.push({ path, method, query });
    if (method !== 'GET' || url.origin !== listener) return reject(`Unexpected request: ${method} ${url}`);
    if (path === '/api/v1/management') return { json: { ...descriptor, scope: describedScope, generation: servedGeneration } };
    if (query.generation !== generation && !(allowReplacement && query.generation === servedGeneration)) return reject(`Request was rebound or omitted the pinned generation: ${url}`);
    const after = query.after;
    if (path === '/api/v1/workspaces') {
      return { json: { generation: servedGeneration, workspaces: [office, otherOffice].map(path => ({ path, sources: ['adopted'] })), coverage: { errors: [], scan_limit_reached: false, agent_limit_reached: false, agent_pages: 1, directories_examined: 2 } } };
    }
    if (path === '/api/v1/agents') {
      if (allowReplacement && query.generation === servedGeneration && servedGeneration !== generation) {
        return { json: { generation: servedGeneration, agents: [], next: null, current_complete: true } };
      }
      if (query.limit !== '1' || query.current_only || query.label) return reject(`Roster must request one bounded unfiltered page: ${url}`);
      if (!after) return { json: { generation: servedGeneration, agents: [external], next: 'after-archive', current_complete: false } };
      if (after === 'after-archive') return { json: { generation: servedGeneration, agents: [current], next: 'after-current', current_complete: true } };
      if (after === 'after-current') return { json: { generation: servedGeneration, agents: [stopped], next: null, current_complete: true } };
      return reject(`Unexpected membership cursor: ${url}`);
    }
    if (path === '/api/v1/roles') {
      const workspace = query.workspace && query.workspace !== 'global' ? query.workspace : null;
      return { json: { generation: servedGeneration, workspace, roles: workspace ? [role] : [] } };
    }
    if (path === '/api/v1/office-activity') {
      return { json: { generation: servedGeneration, next_after: '1730000000000', messages: [], created: [], human_messages: [], human_unhandled_count: 0, human_has_more: false, has_more: false } };
    }
    if (path.startsWith('/api/v1/human')) {
      return { json: { generation: servedGeneration, messages: [], next_before: null, unhandled_count: 0 } };
    }
    if (path === '/api/v1/agents/luna' || path === '/api/v1/agents/luna/messages' || path === '/api/v1/agents/luna/launch-relations') {
      const historical = query.stopped_event === stopped.agent.stopped_event && query.incarnation === undefined;
      const active = query.incarnation === current.agent.incarnation && query.stopped_event === undefined;
      if (!historical && !active) return reject(`Exact live or historical discriminator required: ${url}`);
      if (path.endsWith('/launch-relations')) return { json: { generation: servedGeneration, origin: 'unknown', upstream: null, downstream: [], next: null } };
      if (path === '/api/v1/agents/luna') return { json: historical ? stopped : current };
      if (!after) return { json: { generation: servedGeneration, messages: [message(historical ? 2 : 0)], has_more: active, next_after: historical ? 'checkpoint-history' : 'checkpoint-live-1' } };
      if (active && after === 'checkpoint-live-1') return { json: { generation: servedGeneration, messages: [message(1)], has_more: false, next_after: 'checkpoint-live-2' } };
      if (active && after === 'checkpoint-live-2') return { json: { generation: servedGeneration, messages: [], has_more: false, next_after: after } };
      return reject(`Wrong message checkpoint: ${url}`);
    }
    return reject(`Unmodeled GET: ${url}`);
  }
  return { calls, violations, respond, changeGeneration: () => { servedGeneration = 'successor-generation'; }, allowReconnect: () => { allowReplacement = true; } };
}
