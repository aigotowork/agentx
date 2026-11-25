/** Scope/Workspace/Agent behavior through the production LiveSpace consumer and public SDK. */
import assert from 'node:assert/strict';
import { test } from 'node:test';
import './source-imports.mjs';
const { LiveSpace } = await import('../src/live/controller.ts');
const { parseManagementJson } = await import('@agentx/client');
globalThis.document = Object.assign(new EventTarget(), { hidden: false });

const listener = 'http://disposable.invalid';
const scope = '/disposable', generation = 'g-one', office = '/work/office';
const descriptor = { scope, generation, project: '/work', tools: [], profiles: [], supports_workspace_members: true, supports_message_history: true, max_page: 80, max_text: 65536 };
const current = { name: 'same', incarnation: '18446744073709551615' };
const stopped = { name: 'same', stopped_event: '20' };
const row = (agent, workspace = office) => ({
  agent, collaboration: { status: 'adopted', workspace, role: null }, directory: '/unrelated-cwd',
  tool: 'test', status: agent.stopped_event ? 'stopped' : 'listening', stored_status: 'listening',
  status_context: '', detail: '', can_send: !agent.stopped_event, can_stop: false, can_resume: false, has_transcript: false,
});
const message = (id, text) => ({ id, text, at: '2026-11-18T12:00:00Z', from: '@fixture', recipients: ['same'], attachments: [] });
const memberPage = (agents, next) => ({ generation, agents, next, current_complete: !next });
const latest = (messages, live_after, next_before = null) => ({ generation, messages, live_after, next_before });
const historyPage = (messages, next_after, has_more = false) => ({ generation, messages, next_after, has_more });
const activity = (observedGeneration = generation) => ({ generation: observedGeneration, next_after: '0', has_more: false, messages: [], created: [], human_messages: [], human_has_more: false, human_unhandled_count: 0 });
const refused = (code, message) => Response.json({ error: { code, message } }, { status: 409 });
const waitFor = async predicate => {
  const deadline = Date.now() + 1000;
  while (!predicate()) {
    if (Date.now() >= deadline) assert.fail('fixture transition not observed');
    await new Promise(resolve => setImmediate(resolve));
  }
};

test('Space uses the activity mailbox prefix and reads filtered history only when needed', async () => {
  const fetch = globalThis.fetch, document = globalThis.document;
  globalThis.document = { hidden: false };
  const mailboxCalls = [];
  const letter = (id, handled_at = null) => ({ id: String(id), text: `mail ${id}`, at: '2026-11-18T12:00:00Z', from: 'fixture', handled_at, sender: null });
  let activity = { generation, next_after: '25', has_more: false, messages: [], created: [], human_messages: [letter(25)], human_has_more: false, human_unhandled_count: 1 };
  globalThis.fetch = async request => {
    const url = new URL(request.url);
    switch (url.pathname) {
      case '/api/v1/management': return Response.json(descriptor);
      case '/api/v1/workspaces': return Response.json({ generation, workspaces: [], discovery: { scanned: 0, truncated: false } });
      case '/api/v1/agents': return Response.json(memberPage([], null));
      case '/api/v1/office-activity': return Response.json(activity);
      case '/api/v1/human-mailbox': {
        mailboxCalls.push([url.searchParams.get('pending'), url.searchParams.get('before')]);
        if (url.searchParams.has('before')) return Response.json({ generation, messages: [letter(1)], unhandled_count: 22 });
        return Response.json({ generation, messages: [letter(25), ...Array.from({ length: 19 }, (_, index) => letter(23-index))], next_before: '5', unhandled_count: 22 });
      }
      default: return Response.json({}, { status: 404 });
    }
  };
  const live = new LiveSpace(() => {}, { baseUrl: listener, expectedScope: scope });
  try {
    await live.refresh();
    assert.deepEqual(live.supply.human?.messages.map(message => message.id), ['25']);
    assert.deepEqual(mailboxCalls, []);
    activity = { ...activity, human_unhandled_count: 22, human_has_more: true, human_messages: [letter(25), letter(24, 'handled')] };
    await live.refresh();
    await live.refresh();
    assert.deepEqual(mailboxCalls, [['true', null]]);
    assert.equal(live.supply.human?.nextBefore, '5');
    live.supply.human.page(true);
    await waitFor(() => live.supply.human?.before === '5');
    await live.refresh();
    assert.deepEqual(mailboxCalls, [['true', null], ['true', '5']]);
    assert.deepEqual(live.supply.human?.messages.map(message => message.id), ['1']);
  } finally {
    clearTimeout(live.timer);
    globalThis.fetch = fetch; globalThis.document = document;
  }
});

test('a wrong trusted scope fails before discovery or any fallback request', async () => {
  const calls = [];
  const fetch = globalThis.fetch;
  globalThis.fetch = async request => { calls.push(request.url); return Response.json(descriptor); };
  const live = new LiveSpace(() => {}, { baseUrl: listener, expectedScope: '/wrong' });
  const dispose = live.start();
  try {
    await waitFor(() => live.connection === 'failed');
    assert.deepEqual(calls, [`${listener}/api/v1/management`]);
    assert.deepEqual(live.supply.workspaces, []);
    assert.equal(live.blocked('launch:new'), true);
  } finally { dispose(); globalThis.fetch = fetch; }
});

for (const missing of ['supports_workspace_members', 'supports_message_history']) {
  test(`Space localizes missing ${missing} and keeps exact Agent access`, async () => {
    const calls = [], originalFetch = globalThis.fetch;
    globalThis.fetch = async request => {
      const url = new URL(request.url), path = url.pathname;
      calls.push(path);
      switch (path) {
        case '/api/v1/management': return Response.json({ ...descriptor, [missing]: undefined });
        case '/api/v1/workspaces': return Response.json({ generation, workspaces: [{ path: office, sources: ['adopted'] }],
          coverage: { errors: [], agent_limit_reached: false, scan_limit_reached: false, agent_pages: 1, directories_examined: 1 } });
        case '/api/v1/agents':
        case '/api/v1/workspaces/members': return Response.json(memberPage([row(current)], null));
        case '/api/v1/office-activity': return Response.json(activity());
        case '/api/v1/roles': return Response.json({ generation, workspace: url.searchParams.get('workspace') === 'global' ? null : office, roles: [] });
        case '/api/v1/agents/same':
          assert.equal(url.searchParams.get('incarnation'), current.incarnation);
          return Response.json(row(current));
        case '/api/v1/agents/same/message-history': return Response.json(latest([message('1', 'retained history')], '1'));
        case '/api/v1/agents/same/messages': return Response.json(historyPage([], '1'));
        default: assert.fail(`unhandled fixture endpoint ${path}`);
      }
    };
    const live = new LiveSpace(() => {}, { baseUrl: listener, expectedScope: scope });
    const dispose = live.start();
    try {
      await waitFor(() => live.supply.agents.length === 1 && !live.refreshing);
      const [overview] = live.supply.agents;
      live.chooseOffice(office);
      const membersMissing = missing === 'supports_workspace_members';
      await waitFor(() => live.supply.roster?.phase === (membersMissing ? 'unavailable' : 'ready'));
      assert.equal(live.connection, 'pinned');
      assert.equal(live.supply.agents.length, membersMissing ? 0 : 1);
      if (membersMissing) assert.match(live.supply.roster.error, /不支持成员列表/);
      const exact = await live.openReference(current, descriptor, undefined, office);
      assert.equal(exact.id, overview.id);
      live.select(exact.id);
      await waitFor(() => live.messagePage(exact.id)?.phase === (membersMissing ? 'ready' : 'unavailable'));
      assert.equal(live.supply.records(exact.id, 'messages').status, membersMissing ? 'available' : 'unavailable');
      assert.equal(live.capabilities(exact.id).can_send, true);
      assert.equal(live.canSendTo(exact.id), true);
      live.refreshData();
      await waitFor(() => !live.refreshing);
      assert.equal(calls.filter(path => path === '/api/v1/management').length, 1);
      assert(!calls.includes(membersMissing ? '/api/v1/workspaces/members' : '/api/v1/agents/same/message-history'));
      live.chooseOffice('');
      await waitFor(() => !live.supply.roster && live.supply.agents.length === 1 && !live.refreshing);
    } finally { dispose(); globalThis.fetch = originalFetch; }
  });
}

test('hidden Space waits for foreground and refreshes again on return', async () => {
  const fetch = globalThis.fetch, document = globalThis.document;
  const visibility = Object.assign(new EventTarget(), { hidden: true });
  globalThis.document = visibility;
  let reads = 0;
  let activityReads = 0;
  globalThis.fetch = async request => {
    const pathname = new URL(request.url).pathname;
    if (pathname === '/api/v1/management') { reads++; return Response.json(descriptor); }
    if (pathname === '/api/v1/agents') return Response.json(memberPage([], null));
    if (pathname === '/api/v1/office-activity') { activityReads++; return Response.json(activity()); }
    return Response.json({ generation, workspaces: [], discovery: { scanned: 0, truncated: false } });
  };
  const live = new LiveSpace(() => {}, { baseUrl: listener, expectedScope: scope });
  const dispose = live.start();
  try {
    assert.equal(reads, 0);
    const setVisible = visible => { visibility.hidden = !visible; visibility.dispatchEvent(new Event('visibilitychange')); };
    setVisible(true);
    await waitFor(() => live.connection === 'pinned' && !live.refreshing);
    setVisible(false);
    assert.equal(reads, 1);
    assert.equal(activityReads, 1);
    setVisible(true);
    await waitFor(() => activityReads === 2 && !live.refreshing);
    assert.equal(reads, 1);
  } finally { dispose(); globalThis.fetch = fetch; globalThis.document = document; }
});

test('all offices and legacy use the same pinned exact reader; embedded upgrades retain matching unmarked receipts', async () => {
  const fetch = globalThis.fetch;
  const urls = [];
  const legacy = { ...row(current), collaboration: { status: 'legacy' } };
  globalThis.fetch = async request => {
    const url = new URL(request.url);
    urls.push(url);
    switch (url.pathname) {
      case '/api/v1/management': return Response.json(descriptor);
      case '/api/v1/agents': return Response.json(memberPage([legacy], null));
      case '/api/v1/agents/same': return Response.json(legacy);
      case '/api/v1/agents/same/message-history': return Response.json(latest([message('1', 'legacy text')], 'checkpoint'));
      case '/api/v1/agents/same/messages': return Response.json(historyPage([], 'checkpoint'));
      default: return Response.json({}, { status: 404 });
    }
  };
  const live = new LiveSpace(() => {}, { baseUrl: listener, expectedScope: scope, embedded: true });
  const dispose = live.start();
  try {
    await waitFor(() => live.supply.agents.length === 1);
    const [agent] = live.supply.agents;
    assert.equal(agent.membership.kind, 'legacy');
    live.chooseOffice('$legacy'); live.select(agent.id);
    await waitFor(() => live.supply.records(agent.id, 'messages').status === 'available');
    const exact = urls.filter(url => url.pathname.startsWith('/api/v1/agents/same'));
    assert.deepEqual(exact.slice(0, 2).map(url => url.pathname), ['/api/v1/agents/same', '/api/v1/agents/same/message-history']);
    assert(exact.slice(2).every(url => url.pathname.endsWith('/messages') && url.searchParams.get('after') === 'checkpoint'));
    const old = { scope, body: { generation } };
    assert(live.ownsReceipt(old));
    assert(!live.ownsReceipt({ ...old, listener: 'http://other.invalid' }));
    assert(!live.ownsReceipt({ ...old, body: { generation: 'older' } }));
  } finally { dispose(); globalThis.fetch = fetch; }
});

test('a sparse office and exact namesakes keep independent continuation and discard late reads', async () => {
  const calls = [];
  let rejectNext = true, release, pendingSignal, currentGeneration = generation;
  const fetch = globalThis.fetch;
  globalThis.fetch = async request => {
    assert.equal(request.method, 'GET');
    const url = new URL(request.url), params = url.searchParams;
    assert.equal(url.origin, listener);
    calls.push(url);
    switch (url.pathname) {
      case '/api/v1/management': return Response.json(descriptor);
      case '/api/v1/office-activity': return Response.json(activity(currentGeneration));
      case '/api/v1/workspaces': return Response.json({
        generation, workspaces: [{ path: office, sources: ['adopted'] }],
        coverage: { errors: [], agent_limit_reached: false, scan_limit_reached: false, agent_pages: 1, directories_examined: 2 },
      });
      case '/api/v1/workspaces/members': {
        assert.equal(params.get('workspace'), office);
        assert.equal(params.get('limit'), '1', 'focused owner does not run the broad list');
        const after = params.get('after');
        if (after === null) return Response.json(memberPage([], 'opaque:2'));
        if (after === 'opaque:2' && rejectNext) { rejectNext = false; throw new Error('temporary roster failure'); }
        if (after === 'opaque:2') return Response.json(memberPage([row(current)], 'opaque:3'));
        assert.equal(after, 'opaque:3'); return Response.json(memberPage([row(stopped)], null));
      }
      case '/api/v1/agents/same':
        return Response.json(row(params.has('stopped_event') ? stopped : current));
      case '/api/v1/agents/same/message-history':
        if (params.has('stopped_event')) return Response.json(latest([message('9', 'stopped text')], '20'));
        assert.equal(params.get('incarnation'), current.incarnation);
        if (!params.has('before')) return Response.json(latest([message('2', 'current one')], '10', '2'));
        assert.equal(params.get('before'), '2');
        pendingSignal = request.signal;
        return new Promise(resolve => { release = () => resolve(Response.json(latest([message('1', 'late older')], null))); });
      default:
        // Ancillary feeds are outside these reader assertions.
        return Response.json({}, { status: 404 });
    }
  };
  const live = new LiveSpace(() => {}, { baseUrl: listener, expectedScope: scope, pageLimit: '1' });
  live.chooseOffice(office);
  const dispose = live.start();
  try {
    await waitFor(() => live.supply.roster?.phase === 'ready' || live.supply.roster?.phase === 'error' || live.catalogError);
    assert.equal(live.supply.roster?.phase, 'ready', JSON.stringify({ error: live.catalogError || live.supply.roster?.error, calls }));
    assert.equal(live.supply.roster.more, true);
    assert.equal(live.supply.agents.length, 0);
    live.moreMembers();
    await waitFor(() => live.supply.roster?.phase === 'error');
    assert.equal(live.supply.roster.more, true);
    live.moreMembers();
    await waitFor(() => live.supply.agents.length === 1);
    live.moreMembers();
    await waitFor(() => live.supply.roster.phase === 'ready' && !live.supply.roster.more);
    assert.deepEqual(calls.filter(url => url.pathname === '/api/v1/workspaces/members').map(url => url.searchParams.get('after')), [null, 'opaque:2', 'opaque:2', 'opaque:3']);
    assert(!calls.some(url => url.pathname === '/api/v1/agents'), 'focused roster never scans the scope');
    const [a, b] = live.supply.agents;
    assert.notEqual(a.id, b.id);
    assert.equal(a.cwd, '/unrelated-cwd');
    live.select(a.id);
    await waitFor(() => live.messagePage(a.id)?.more || live.messagePage(a.id)?.error);
    assert.equal(live.messagePage(a.id)?.more, true, live.messagePage(a.id)?.error);
    live.select(a.id);
    assert.equal(calls.filter(url => url.pathname === '/api/v1/agents/same/message-history').length, 1);
    live.moreMessagesFor(a.id);
    await waitFor(() => release);
    live.select(b.id);
    assert.equal(pendingSignal.aborted, true);
    await waitFor(() => live.supply.records(b.id, 'messages').status === 'available');
    release();
    await new Promise(resolve => setImmediate(resolve));
    assert.deepEqual(live.supply.records(b.id, 'messages').items.map(item => item.text), ['stopped text']);
    assert.notEqual(live.supply.records(a.id, 'messages').status, 'available');
    assert.equal(live.capabilities(b.id).can_send, false);
    currentGeneration = 'g-two';
    await live.refresh();
    assert.equal(live.connection, 'reset');
    assert.equal(live.supply.scope.generation, generation);
    assert.equal(live.blocked(b.id), true);
    assert.equal(calls.filter(url => url.pathname === '/api/v1/management').length, 1);
    assert.equal(calls.filter(url => url.pathname === '/api/v1/workspaces').length, 1);
  } finally { dispose(); globalThis.fetch = fetch; }
});

const other = { name: 'peer', incarnation: '5' };

function readerFixture(t, references, respond) {
  const originalFetch = globalThis.fetch;
  let scopeGeneration = generation;
  globalThis.fetch = async request => {
    const url = new URL(request.url);
    assert.equal(url.origin, listener);
    switch (url.pathname) {
      case '/api/v1/management': return Response.json(descriptor);
      case '/api/v1/workspaces': return Response.json({ generation, workspaces: [{ path: office, sources: ['adopted'] }],
        coverage: { errors: [], agent_limit_reached: false, scan_limit_reached: false, agent_pages: 1, directories_examined: 1 } });
      case '/api/v1/agents':
      case '/api/v1/workspaces/members': return Response.json(memberPage(references.map(ref => row(ref)), null));
      case '/api/v1/office-activity': return scopeGeneration === generation ? Response.json(activity()) : refused('StaleGeneration', 'database generation changed');
      default: return respond(request, url) ?? Response.json({}, { status: 404 });
    }
  };
  const live = new LiveSpace(() => {}, { baseUrl: listener, expectedScope: scope });
  live.chooseOffice(office);
  const stop = live.start();
  t.after(() => { stop(); globalThis.fetch = originalFetch; });
  return { live, reset: () => { scopeGeneration = 'g-two'; } };
}

test('a replaced same-name Agent invalidates its exact reader without resetting the Scope', async t => {
  const successor = { ...current, incarnation: '12' }, references = [current, other];
  const { live } = readerFixture(t, references, (request, url) => {
    assert.equal(request.method, 'GET');
    const match = url.pathname.match(/^\/api\/v1\/agents\/([^/]+)(?:\/(.+))?$/);
    if (!match) return;
    const ref = references.find(agent => agent.name === match[1]);
    if (!ref || url.searchParams.get('incarnation') !== ref.incarnation) return refused('StaleReference', 'exact Agent changed');
    if (match[2] === 'message-history') return Response.json(latest([], '0'));
    if (match[2] === 'messages') return Response.json(historyPage([], '0'));
    return Response.json(row(ref));
  });
  await waitFor(() => live.supply.agents.length === 2 && !live.refreshing);
  const [old, unaffected] = live.supply.agents;
  live.selectReaders([old.id, unaffected.id], old.id);
  await waitFor(() => live.canSendTo(old.id) && live.canSendTo(unaffected.id));
  references[0] = successor;
  await live.refresh();
  await waitFor(() => live.messagePage(old.id)?.live?.phase === 'unavailable');
  assert.equal(live.connection, 'pinned');
  assert.equal(live.canSendTo(old.id), false);
  assert.equal(live.canSendTo(unaffected.id), true);
  await assert.rejects(live.target('send', old.id, { text: 'not for the successor' }), /精确会话/);
  live.refreshMembers();
  await waitFor(() => live.supply.agents.some(agent => agent.ref.incarnation === successor.incarnation));
  const adopted = live.supply.agents.find(agent => agent.ref.incarnation === successor.incarnation);
  assert.notEqual(adopted.id, old.id);
});

test('latest reads isolate older retries, concurrent arrivals and a stopped namesake', async t => {
  const calls = [];
  let failedOnce = false, releaseOlder, arriving = true;
  const backwards = (messages, next_before, live_after) => Response.json({ generation, messages, next_before, live_after });
  const { live } = readerFixture(t, [current, stopped], (request, url) => {
    const params = url.searchParams, archive = params.has('stopped_event');
    const ref = archive ? stopped : current;
    if (url.pathname === '/api/v1/agents/same') return Response.json(row(ref));
    const service = url.pathname.split('/').at(-1);
    if (!['messages', 'message-history'].includes(service)) return;
    calls.push({ service, archive, before: params.get('before'), after: params.get('after'), limit: params.get('limit') });
    if (service === 'messages') {
      assert(!archive, 'stopped namesake never polls forward');
      const after = params.get('after');
      if (!arriving) return Response.json(historyPage([], after));
      arriving = false;
      assert.equal(after, '17');
      return Response.json(historyPage([message('18', 'live')], '18'));
    }
    if (archive) return backwards([message('13', 'archive')], null, '20');
    if (!params.has('before')) return backwards([message('15', 'recent'), message('16', 'newest')], '15', '17');
    assert.equal(params.get('before'), '15');
    if (!failedOnce) {
      failedOnce = true;
      return Response.json({ error: { code: 'Unavailable', message: 'fixture failure' } }, { status: 503 });
    }
    return new Promise(resolve => { releaseOlder = () => resolve(backwards([message('12', 'older'), message('14', 'older2')], null, null)); });
  });
  await waitFor(() => live.supply.agents.length === 2 && !live.refreshing);
  const a = live.supply.agents.find(agent => agent.ref.record === 'current').id;
  const b = live.supply.agents.find(agent => agent.ref.record === 'stopped').id;
  const ids = id => live.supply.records(id, 'messages').items?.map(item => item.id);
  live.selectReaders([a, b], a);
  await waitFor(() => [a, b].every(id => live.messagePage(id)?.token));
  assert.deepEqual(ids(a), ['15', '16']); assert.deepEqual(ids(b), ['13']);
  const token = live.messagePage(a).token;
  live.moreMessagesFor(a);
  await waitFor(() => live.messagePage(a)?.older.phase === 'error');
  assert.equal(live.messagePage(a).phase, 'ready', 'an older read error does not remove current capabilities');
  live.moreMessagesFor(a);
  await waitFor(() => releaseOlder);
  await live.refresh();
  await waitFor(() => live.messagePage(a)?.arrivals === 1);
  releaseOlder();
  await waitFor(() => !live.messagePage(a)?.more);
  assert.deepEqual(ids(a), ['12', '14', '15', '16', '18']);
  assert.equal(live.messagePage(a).arrivals, 1);
  assert(live.messagePage(a).token === token);
  live.selectReaders([b], b);
  await live.refresh();
  assert.deepEqual(calls.filter(call => call.service === 'messages').map(call => call.after), ['17']);
  live.selectReaders([a, b], a);
  await waitFor(() => live.messagePage(a)?.token);
  assert(live.messagePage(a).token === token);
  assert.deepEqual(ids(a), ['12', '14', '15', '16', '18']);
  assert(calls.every(call => call.limit === '20'));
});

test('parked history survives view changes but not newly observed membership', async t => {
  let memberOffice = office;
  const requested = [];
  const { live } = readerFixture(t, [current], (request, url) => {
    if (url.pathname === '/api/v1/agents/same') return Response.json(row(current, memberOffice));
    if (!url.pathname.endsWith('/message-history')) return;
    requested.push(url.searchParams.get('before'));
    return Response.json(latest([message('1', `read ${requested.length}`)], '1'));
  });
  await waitFor(() => live.supply.agents.length === 1 && !live.refreshing);
  const [{ id }] = live.supply.agents;
  const ready = () => live.messagePage(id)?.phase === 'ready';
  const texts = () => live.supply.records(id, 'messages').items?.map(item => item.text);
  live.select(id);
  await waitFor(ready);
  live.select(null);
  live.chooseOffice('');
  await waitFor(() => live.supply.agents.length === 1 && !live.refreshing);
  live.select(id);
  await waitFor(ready);
  assert.deepEqual(texts(), ['read 1'], 'same verified Agent keeps its history across office views');
  live.select(null);
  memberOffice = '/new-office';
  live.select(id);
  await waitFor(ready);
  assert.deepEqual(texts(), ['read 2'], 'all offices permits the new membership but invalidates the old history');
  assert.deepEqual(requested, [null, null]);

  live.chooseOffice(office);
  memberOffice = office;
  await waitFor(() => live.supply.roster?.phase === 'ready');
  live.select(id);
  await waitFor(ready);
  live.select(null);
  memberOffice = '/new-office';
  live.select(id);
  await waitFor(() => live.messagePage(id)?.phase === 'error');
  assert.equal(texts(), undefined, 'mismatched office cannot display the parked history');
  assert.equal(live.capabilities(id), undefined);
  assert.deepEqual(requested, [null, null, null]);
});

test('the oldest parked history is evicted when the eight-reader bound is reached', async t => {
  const references = Array.from({ length: 9 }, (_, index) => ({ name: `peer-${index}`, incarnation: '5' }));
  const pages = [];
  const { live } = readerFixture(t, references, (request, url) => {
    const [, name, history] = url.pathname.match(/^\/api\/v1\/agents\/(peer-\d+)(\/message-history)?$/) ?? [];
    if (!name) return;
    if (!history) return Response.json(row(references.find(ref => ref.name === name)));
    pages.push(name);
    return Response.json(latest([message('1', name)], '1'));
  });
  await waitFor(() => live.supply.agents.length === references.length && !live.refreshing);
  const ids = live.supply.agents.map(agent => agent.id);
  const read = async id => { live.select(id); await waitFor(() => live.messagePage(id)?.phase === 'ready'); };
  for (const id of ids) await read(id);
  await read(ids[1]);
  assert.equal(pages.length, 9, 'the second-oldest entry is still available');
  await read(ids[0]);
  assert.deepEqual(pages, [...references.map(ref => ref.name), references[0].name]);
});

test('admission keeps the exact submitted pane across focus but rejects revoked leases and membership', async t => {
  const storage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const journal = new Map();
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: { getItem: key => journal.get(key), setItem: (key, value) => journal.set(key, value) } });
  t.after(() => { if (storage) Object.defineProperty(globalThis, 'localStorage', storage); else Reflect.deleteProperty(globalThis, 'localStorage'); });
  let hold, pending;
  const posts = [];
  const { live, reset } = readerFixture(t, [current, other], (request, url) => {
    if (request.method === 'POST') return request.text().then(raw => {
      assert.equal(url.pathname, '/api/v1/agents/same/messages');
      const body = parseManagementJson('SubmitMessage', raw);
      assert.equal(live.receipts[0].id, body.request_id);
      assert([...journal.values()].some(contents => contents.includes(body.request_id)), 'journal is durable before dispatch');
      posts.push(body);
      return Response.json({ generation, request_id: body.request_id, status: 'Succeeded', result: { Sent: { message_id: '1', recipients: ['same'] } } });
    });
    if (url.pathname.endsWith('/message-history')) return Response.json(latest([], '0'));
    if (url.pathname.endsWith('/messages')) return Response.json(historyPage([], '0'));
    if (url.pathname === '/api/v1/agents/peer') return Response.json(row(other));
    if (url.pathname !== '/api/v1/agents/same') return;
    if (!hold) return Response.json(row(current));
    hold = false;
    return new Promise(resolve => { pending = { signal: request.signal, finish: (workspace = office) => resolve(Response.json(row(current, workspace))) }; });
  });
  await waitFor(() => live.supply.agents.length === 2 && !live.refreshing);
  const [a, b] = live.supply.agents.map(agent => agent.id);
  const pairReady = () => [a, b].every(id => live.messagePage(id)?.phase === 'ready');
  live.selectReaders([a, b], a);
  await waitFor(pairReady);
  const begin = async () => {
    hold = true; pending = undefined;
    const result = live.target('send', a, { text: 'for exact current', intent: 'request' });
    // Attach immediately so transport-level abort cannot produce an unhandled rejection.
    result.catch(() => {});
    await waitFor(() => pending);
    return { result, ...pending };
  };
  const focused = await begin();
  live.selectReaders([b, a], b);
  assert.equal(focused.signal.aborted, false);
  focused.finish();
  assert.equal(await focused.result, true);
  assert.deepEqual(posts.map(post => [post.generation, post.agent.name, post.agent.incarnation, post.message.text]), [[generation, current.name, current.incarnation, 'for exact current']]);

  const moved = await begin();
  moved.finish('/other-office');
  await assert.rejects(moved.result, /精确目标或发送能力已失效/);

  const closed = await begin();
  live.selectReaders([b], b);
  live.selectReaders([a, b], a);
  await waitFor(pairReady);
  assert.equal(closed.signal.aborted, true);
  closed.finish();
  await assert.rejects(closed.result);

  const switched = await begin();
  live.chooseOffice('');
  assert.equal(live.messagePage(a), undefined);
  assert.equal(switched.signal.aborted, true);
  switched.finish();
  await assert.rejects(switched.result);
  await waitFor(() => live.supply.agents.length === 2 && !live.refreshing);
  live.selectReaders([a, b], a);
  await waitFor(pairReady);

  const obsolete = await begin();
  reset();
  await live.refresh();
  assert.equal(live.connection, 'reset');
  assert.equal(live.messagePage(a), undefined);
  assert.equal(obsolete.signal.aborted, true);
  obsolete.finish();
  await assert.rejects(obsolete.result);
  assert.equal(posts.length, 1);
  assert.equal(live.receipts.length, 1);
});
