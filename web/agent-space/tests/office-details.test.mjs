/** Developer cases for population invariants and the existing platform-neutral motion contract. */
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { existsSync } from 'node:fs';
import { registerHooks } from 'node:module';
registerHooks({ resolve(specifier, context, next) {
  if (specifier.startsWith('.') && context.parentURL) {
    const exact = new URL(specifier, context.parentURL);
    if (!existsSync(exact) && existsSync(new URL(`${exact.href}.ts`))) return next(`${exact.href}.ts`, context);
  }
  return next(specifier, context);
} });
const { projectHall } = await import('../src/scene/model.ts');
const { populationCapacity } = await import('../src/scene/population.ts');
const { sceneBehavior, hallActorGoal, beginHallMotion, sampleHallMotion } = await import('../src/scene/motion.ts');
function member(index, status = 'listening', office = '/project') {
  return { id: `scope:gen:agent${index}:incarnation1`, name: `agent${index}`, title: `@agent${index}`, state: status === 'active' ? 'running' : 'idle', nativeStatus: status, attention: false, description: '', ref: { name: `agent${index}`, incarnation: '1', record: 'current' }, membership: { kind: 'directory', path: office }, role: { kind: 'none' }, createdAt: '' };
}
const names = new Map([['/project', 'Project'], ['/global', 'Global']]);
function room(resources, memory = new Map(), revision = 0, complete = true, width = 1440) { return projectHall(resources, names, width, memory, { revision, complete }, 'scope:gen'); }

test('one/few/many capacity and real independent room dimensions', () => {
  assert.deepEqual([0, 1, 2, 3, 4, 5, 8, 9, 12, 16, 17].map(populationCapacity), [0, 2, 2, 4, 4, 6, 8, 12, 12, 16, 20]);
  const scene = room([member(0, 'listening', '/global'), ...Array.from({ length: 14 }, (_, i) => member(i + 1))]);
  const small = scene.rooms.find(room => room.key === '/global'), large = scene.rooms.find(room => room.key === '/project');
  assert.deepEqual([small.width, small.height], [312, 392]);
  assert.deepEqual([large.width, large.height], [1008, 616]);
  assert.equal(scene.actors.length, 15);
  assert.ok(scene.human.y < small.origin.y);
  const narrow = room([member(0)], new Map(), 0, true, 390).rooms.find(room => room.key === '/project');
  assert.ok(narrow.width <= (390 - 24) / .75);
});

test('status and poll order preserve seats, both reserved zones and room dimensions', () => {
  const memory = new Map(), members = Array.from({ length: 8 }, (_, i) => member(i));
  const first = room(members, memory, 1).rooms.find(room => room.key === '/project');
  const working = room([...members].reverse().map(person => ({ ...person, state: 'running', nativeStatus: 'active' })), memory, 2).rooms.find(room => room.key === '/project');
  assert.deepEqual([first.width, first.height, first.origin], [working.width, working.height, working.origin]);
  for (const actor of first.actors) {
    const next = working.actors.find(person => person.key === actor.key);
    assert.deepEqual([actor.home, actor.rest], [next.home, next.rest]);
  }
  const withHistory = room([...members, { ...member(99), ref: { record: 'stopped' } }], memory, 3).rooms.find(room => room.key === '/project');
  assert.deepEqual([withHistory.width, withHistory.height], [first.width, first.height]);
});

test('spare seats grow only on overflow; partial/error snapshots cannot shrink', () => {
  const memory = new Map();
  const dims = (count, revision, complete = true) => { const result = room(Array.from({ length: count }, (_, i) => member(i)), memory, revision, complete).rooms.find(room => room.key === '/project'); return [result.width, result.height]; };
  const one = dims(1, 1), two = dims(2, 2), three = dims(3, 3), four = dims(4, 4);
  assert.deepEqual(one, two);
  assert.notDeepEqual(three, two);
  assert.deepEqual(three, four);
  const full = dims(16, 5);
  assert.deepEqual(dims(1, 6, false), full);
  assert.deepEqual(dims(1, 7), full);
  assert.deepEqual(dims(1, 8, false), full);
  assert.deepEqual(dims(1, 9), full);
  assert.deepEqual(dims(1, 10), [312, 392]);
});

test('effective status owns explicit work/listen/sleep/error mapping and pause settles motion', () => {
  const behavior = status => sceneBehavior(member(0, status));
  assert.equal(behavior('active'), 'working'); assert.equal(behavior('listening'), 'listening');
  assert.equal(behavior('inactive'), 'sleeping'); assert.equal(behavior('blocked'), 'blocked');
  assert.equal(behavior('error'), 'error'); assert.equal(behavior('launching'), 'launching');
  const error = { ...member(0, 'error'), statusContext: 'model', requestError: { source: 'model_request', reason: 'failed' } };
  assert.equal(sceneBehavior(error), 'request-error');
  assert.equal(sceneBehavior({ ...error, statusContext: 'model:request' }), 'request-error');
  assert.equal(sceneBehavior({ ...error, requestError: null }), 'error');
  assert.equal(sceneBehavior({ ...error, requestError: { source: 'other', reason: 'model request failed' } }), 'error');
  assert.equal(sceneBehavior({ ...error, statusContext: 'failure:tool' }), 'error');
  assert.equal(sceneBehavior({ ...error, nativeStatus: 'active' }), 'working');
  assert.equal(sceneBehavior({ ...error, nativeStatus: 'listening' }), 'listening');
  assert.equal(sceneBehavior({ ...error, nativeStatus: 'inactive' }), 'sleeping');
  assert.equal(sceneBehavior({ ...error, nativeStatus: 'blocked' }), 'blocked');
  const actor = room([member(0, 'inactive')]).actors[0];
  assert.deepEqual(hallActorGoal(actor), actor.home);
  const motion = beginHallMotion(actor.rest, actor.home, 0), moving = sampleHallMotion(actor, motion, 50, false);
  assert.equal(moving.pose, 'walk');
  const settled = sampleHallMotion(actor, motion, 50, true);
  assert.equal(settled.pose, 'doze'); assert.deepEqual([settled.x, settled.y], [actor.home.x, actor.home.y]);
  const listening = { ...actor, resource: member(0) };
  assert.deepEqual(hallActorGoal(listening), actor.rest);
  assert.deepEqual(sampleHallMotion(listening, beginHallMotion(actor.rest, actor.rest, 0), 1000, false), sampleHallMotion(listening, beginHallMotion(actor.rest, actor.rest, 0), 1000, false));
  const rerouted = beginHallMotion(moving, actor.rest, 50);
  assert.deepEqual(rerouted.from, moving);
});

test('indoor routes stay in the real room, avoid furniture and retarget without the old x=32 detour', async () => {
  const { indoorPath } = await import('../src/scene/navigation.ts');
  for (const width of [390, 1440]) for (const count of [1, 3, 14, 30]) {
    const scene = room(Array.from({ length: count }, (_, i) => member(i)), new Map(), 0, true, width);
    for (const actor of scene.actors) {
      const nav = actor.navigation;
      const assertRoute = (from, to) => {
        const route = indoorPath(from, to, nav);
        assert.deepEqual(route.at(-1), to, `unreachable ${width}/${count}/${actor.key}`);
        for (let leg = 1; leg < route.length; leg++) for (let step = 1; step < 100; step++) {
          const point = { x: route[leg - 1].x + (route[leg].x - route[leg - 1].x) * step / 100, y: route[leg - 1].y + (route[leg].y - route[leg - 1].y) * step / 100 };
          const doorway = from === nav.door && leg === 1;
          if (!doorway) {
            assert.ok(point.x >= nav.bounds.x - 1e-6 && point.x <= nav.bounds.x + nav.bounds.width + 1e-6);
            assert.ok(point.y >= nav.bounds.y - 1e-6 && point.y <= nav.bounds.y + nav.bounds.height + 1e-6);
          }
          for (const obstacle of nav.obstacles) assert.ok(!(point.x > obstacle.x + 1e-6 && point.x < obstacle.x + obstacle.width - 1e-6 && point.y > obstacle.y + 1e-6 && point.y < obstacle.y + obstacle.height - 1e-6), 'route intersects furniture');
        }
        return route;
      };
      assertRoute(actor.home, actor.rest); assertRoute(actor.rest, actor.home); assertRoute(nav.door, actor.home);
      const motion = beginHallMotion(actor.home, actor.rest, 0, nav), middle = sampleHallMotion(actor, motion, motion.legs.reduce((n, leg) => n + leg.durationMs, 0) * .43, false);
      assertRoute(middle, actor.home);
      const interrupted = beginHallMotion(middle, actor.home, 100, nav);
      assert.deepEqual(interrupted.from, middle);
      assert.deepEqual(sampleHallMotion(actor, interrupted, 20000, false).x, actor.home.x);
    }
  }
  const nav = { bounds: { x: 400, y: 100, width: 400, height: 300 }, door: { x: 464, y: 412 }, obstacles: [] };
  const a = { x: 500, y: 200 }, b = { x: 650, y: 260 };
  assert.deepEqual(indoorPath(a, b, nav), [a, b], 'unobstructed walk must be direct, not an invented aisle');
  const obstacle = { x: 550, y: 150, width: 50, height: 100 };
  const route = indoorPath(a, b, { ...nav, obstacles: [obstacle] });
  assert.ok(route.length > 2);
  assert.ok(route.every(point => point.x >= a.x), 'no wall-directed backtracking');
  assert.deepEqual(indoorPath(a, b, { ...nav, obstacles: [{ x: 540, y: 99, width: 100, height: 302 }] }), [a], 'unreachable route must hold rather than cross wall');
});

test('lounge is stable, compact standing groups with safe spacing and only shared facilities', () => {
  for (const count of [1, 3, 14, 30]) {
    const members = Array.from({ length: count }, (_, i) => member(i)), memory = new Map();
    const first = room(members, memory, 1), next = room([...members].reverse().map(m => ({ ...m, nativeStatus: 'active' })), memory, 2);
    for (const actor of first.actors) {
      const other = next.actors.find(item => item.key === actor.key);
      assert.deepEqual(actor.rest, other.rest);
      assert.equal(sampleHallMotion(actor, beginHallMotion(actor.rest, actor.rest, 0), 0, true).pose, 'idle');
      for (const rect of actor.navigation.obstacles) assert.ok(!(actor.rest.x > rect.x && actor.rest.x < rect.x + rect.width && actor.rest.y > rect.y && actor.rest.y < rect.y + rect.height));
      const peers = first.actors.filter(item => item.key !== actor.key);
      for (const peer of peers) assert.ok(Math.hypot(actor.rest.x - peer.rest.x, actor.rest.y - peer.rest.y) >= 64, 'independent 44px hit targets at .75 scale');
      if (peers.length) assert.ok(Math.min(...peers.map(peer => Math.hypot(actor.rest.x - peer.rest.x, actor.rest.y - peer.rest.y))) <= 110, 'nearby conversation partner');
    }
    const office = first.rooms.find(item => item.key === '/project').office;
    assert.equal(office.fixtures.filter(item => item.prop === 'coffee').length, 1);
    assert.ok(office.fixtures.length <= 2);
  }
});
