/** Implementer cases for URL grammar, lifecycle and preservation; independent real App evidence is separate. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import './source-imports.mjs';
const { parseRoute, routeSearch, routeFromState } = await import('../src/navigation/route.ts');
const { installNavigation } = await import('../src/navigation/browser.ts');
const { SpaceController } = await import('../src/offline/controller.ts');
const agent = { id: 'exact-current', name: 'mora', membership: { kind: 'directory', path: '/office' }, ref: { scope: 's', generation: 'g', incarnation: '123', stopped_event: null, record: 'current' } };
const stopped = { ...agent, id: 'exact-stopped', ref: { ...agent.ref, incarnation: null, stopped_event: '99', record: 'stopped' } };
const supply = (agents = [agent, stopped]) => ({ mode: 'live', scope: { id: 's', generation: 'g' }, agents, posts: [{ id: 'post:2', office: '/office' }], workspaces: [{ id: '/office' }] });
function browser(search = '') {
  const listeners = new Map(), entries = [new URL(`http://example/space/${search}#host`)], calls = [];
  let index = 0;
  const set = () => { globalThis.location = entries[index]; };
  set();
  globalThis.window = { dispatchEvent: event => listeners.get(event.type)?.(event), addEventListener: (type, fn) => listeners.set(type, fn), removeEventListener: type => listeners.delete(type), history: Object.fromEntries(['pushState', 'replaceState'].map(type => [type, (_state, _title, path) => {
    const url = new URL(path, entries[index]); calls.push(type);
    if (type === 'pushState') { entries.splice(index + 1); entries.push(url); index++; } else entries[index] = url;
    set();
  }])) };
  return { calls, listeners, back: () => { index--; set(); listeners.get('popstate')(); }, forward: () => { index++; set(); listeners.get('popstate')(); }, count: () => entries.length };
}
test('real state round trips exact current/stopped, channels, resources and preserved hints', () => {
  const controller = new SpaceController(supply());
  controller.openExact(stopped.id); controller.presentation('chat'); controller.channel(stopped.id, 'model'); controller.detail(true);
  const route = routeFromState(controller.getSnapshot(), controller.supply);
  const search = routeSearch('?supply=demo&layout=sidebar&host=yes', route);
  assert.deepEqual(parseRoute(search).route, { ...route, inferOffice: false });
  assert.match(search, /host=yes/); assert.match(search, /layout=sidebar/); assert.equal(parseRoute(search).issue, '');
  controller.office(''); controller.select(agent.id);
  const all = routeSearch('', routeFromState(controller.getSnapshot(), controller.supply));
  assert.match(all, /office=/); assert.equal(parseRoute(all).route.workspaceId, '');
  for (const query of ['?view=bad', '?tab=posts&post=a&post=b', '?agent=mora', '?scope=s&generation=g&agent=mora&incarnation=123&stopped_event=99', '?panel=directory&view=chat', '?channel=wrong', '?tab=skills&panel=mailbox', '?detail=agent']) assert.ok(parseRoute(query).issue, query);
  assert.equal(parseRoute('?channel=notes').route.channel, 'messages');
  assert.equal(parseRoute('?scope=s&generation=g&agent=mora&stopped_event=99').route.officeView, 'history');
});
test('UI navigation pushes, filter typing replaces, history hydrates without draft or attachment loss', () => {
  const env = browser('?layout=sidebar&host=yes');
  const controller = new SpaceController(supply()); let issue;
  const cleanup = installNavigation(controller, value => { issue = value; });
  controller.office('/office'); controller.presentation('chat'); controller.select(agent.id);
  controller.writeDraft(agent.id, 'private draft'); controller.restoreNavigation({ attachments: { [agent.id]: [{ id: 'file' }] } });
  const count = env.count();
  controller.replaceSupply(supply()); controller.writeDraft(agent.id, 'edited private draft');
  assert.equal(env.count(), count); assert.doesNotMatch(location.search, /private|file/);
  controller.channel(agent.id, 'model'); controller.close(); env.back();
  assert.equal(controller.getSnapshot().reading, true); assert.equal(controller.getSnapshot().channels[agent.id], 'model');
  env.back(); assert.equal(controller.getSnapshot().channels[agent.id], 'messages');
  env.forward(); assert.equal(controller.getSnapshot().channels[agent.id], 'model');
  assert.equal(controller.getSnapshot().drafts[agent.id], 'edited private draft');
  assert.equal(controller.getSnapshot().attachments[agent.id][0].id, 'file');
  controller.close(); controller.filter({ query: 'm' }); const filteredCount = env.count(); controller.filter({ query: 'mora' });
  assert.equal(env.count(), filteredCount); assert.equal(env.calls.at(-1), 'replaceState');
  assert.match(location.search, /host=yes/); assert.equal(location.hash, '#host'); assert.equal(issue, '');
  cleanup(); assert.equal(env.listeners.size, 0);
});
test('delayed deep link retains intent through drafts/refresh; exact lookup and successor fences', async () => {
  const query = '?office=%2Foffice&scope=s&generation=g&agent=mora&incarnation=123&record=current&view=chat&channel=model';
  const env = browser(query), controller = new SpaceController(supply([])); let issue = '', lookups = 0;
  const cleanup = installNavigation(controller, value => { issue = value; }, async target => { lookups++; assert.equal(target.incarnation, '123'); });
  assert.match(issue, /精确/); assert.equal(controller.getSnapshot().reading, false);
  await Promise.resolve();
  controller.replaceSupply(supply([{ ...agent, id: 'successor', ref: { ...agent.ref, incarnation: '456' } }]));
  controller.writeDraft('successor', 'other'); assert.equal(controller.getSnapshot().reading, false);
  assert.equal(location.search, query); assert.equal(env.calls.length, 0); assert.equal(lookups, 1);
  controller.replaceSupply(supply()); assert.equal(controller.getSnapshot().selectedId, agent.id); assert.equal(controller.getSnapshot().channels[agent.id], 'model'); assert.equal(issue, '');
  controller.replaceSupply({ ...supply(), scope: { id: 's', generation: 'next' } });
  assert.equal(controller.getSnapshot().reading, false); assert.match(issue, /generation/); assert.equal(location.search, query);
  controller.office(''); controller.replaceSupply(supply()); assert.equal(controller.getSnapshot().reading, false); assert.equal(issue, '');
  cleanup();
});
test('direct post unavailable never falls back; panel/mail/facts restore and user cancels pending route', () => {
  browser('?office=%2Foffice&tab=posts&post=missing'); const controller = new SpaceController(supply()); let issue;
  const cleanup = installNavigation(controller, value => { issue = value; });
  assert.equal(controller.getSnapshot().postId, 'missing'); assert.match(issue, /岗位/);
  controller.view('members'); controller.panel('mailbox'); controller.mailbox('all', 'cursor-20');
  const route = parseRoute(location.search); assert.equal(route.route.panel, 'mailbox'); assert.equal(route.route.mail, 'all'); assert.equal(route.route.before, 'cursor-20');
  controller.openExact(agent.id); controller.panel(''); assert.equal(controller.getSnapshot().reading, true);
  controller.detail(true); assert.match(location.search, /detail=agent/);
  cleanup();
});
