/** Studio's projected geometry, Canvas inspection and exact local identities. */
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { test } from 'node:test';
registerHooks({ resolve(specifier, context, next) {
  if (specifier.startsWith('.') && context.parentURL) {
    const exact = new URL(specifier, context.parentURL);
    if (!existsSync(exact) && existsSync(new URL(`${exact.href}.ts`))) return next(`${exact.href}.ts`, context);
  }
  return next(specifier, context);
} });
const { createStudioSupply } = await import('../src/offline/studio.ts');
const { projectHall } = await import('../src/scene/model.ts');
const { indoorPath } = await import('../src/scene/navigation.ts');
const { ActorAnimation } = await import('../src/scene/runtime/actors.ts');
const { sceneObjects } = await import('../src/scene/debug/objects.ts');
const { studioEntryCamera } = await import('../src/scene/studio-art.ts');
const { SpaceController } = await import('../src/offline/controller.ts');
const { parseRoute, routeFromState, routeSearch } = await import('../src/navigation/route.ts');

const supply = createStudioSupply();
const [catalog, annex] = supply.workspaces;
const roomNames = office => new Map([[office.id, office.title]]);
const project = (width, office = catalog) => projectHall(office === catalog ? supply.agents : [], roomNames(office), width, new Map(), supply.population, supply.scope.id, 'studio');

test('working and rest seats share a walkable modeled doorway at wide and narrow sizes', () => {
  for (const width of [1216, 390, 300]) {
    const scene = project(width);
    assert.equal(scene.actors.length, 6);
    const { office, cabinet } = scene.rooms[0];
    assert.equal(cabinet.resources.length, 1);
    assert.equal(width > 700, office.work.y === office.lounge.y);
    for (const actor of scene.actors) {
      for (const [from, to] of [[office.door, actor.home], [actor.home, actor.rest], [actor.rest, office.door]]) {
        const path = indoorPath(from, to, actor.navigation);
        assert.deepEqual(path.at(-1), to, `reachable in ${width}px for ${actor.resource.title}`);
      }
    }
  }
  const scene = project(390, annex);
  assert.equal(scene.actors.length, 0);
  assert.deepEqual(scene.rooms[0].office.fixtures.map(item => item.id), ['bookshelf', 'plant']);
});

test('the inspected frame exposes actual vector boxes for actors and decorative assets without giving decor actions', () => {
  const viewport = { width: 1216, height: 708 }, scene = project(viewport.width);
  const input = { scene, camera: studioEntryCamera(scene, viewport), viewport, props: [], humanMessages: 0, paused: true, visibleKeys: new Set(scene.actors.map(actor => actor.key)) };
  const actors = new ActorAnimation(input).sample(input, 0, true);
  const surface = { x: 0, y: 0, ...viewport };
  const metrics = { backing: viewport, client: surface, page: surface, cssToBacking: { x: 1, y: 1 }, canvasToClient: { x: 1, y: 1 } };
  const objects = sceneObjects({ input, actors, still: true, elapsed: 0, flights: [] }, metrics, () => undefined);
  const cast = objects.filter(object => object.kind === 'actor');
  assert.equal(new Set(cast.map(object => object.details.assetId)).size, 4);
  assert.deepEqual(new Set(cast.map(object => object.details.signal)), new Set(['glyph-work', 'glyph-available', 'glyph-attention']));
  for (const actor of cast) {
    const feet = actor.geometry.anchor.world, box = actor.geometry.bounds.world;
    assert.equal(box.x + box.width / 2, feet.x);
    assert.equal(box.y + box.height, feet.y);
  }
  const threshold = objects.find(object => object.details.assetId === 'door-threshold');
  assert.deepEqual(threshold.geometry.anchor.world, scene.rooms[0].office.door);
  assert.deepEqual(threshold.actionIds, []);
  assert(!objects.some(object => object.label === 'Decorative cat'), 'the Studio mailbox has its own single image');
});

test('synthetic current and stopped namesakes keep separate route and draft identities', () => {
  const [current, stopped] = supply.agents.filter(agent => agent.name === 'mora');
  assert.notEqual(current.id, stopped.id);
  const controller = new SpaceController(supply);
  controller.office(catalog.id);
  assert(controller.openConversation(current.id, 'actor'));
  controller.writeDraft(current.id, 'current-only');
  assert(!controller.openConversation(stopped.id, 'actor'));
  controller.openExact(stopped.id);
  const parsed = parseRoute(routeSearch('?supply=studio&style=studio', routeFromState(controller.getSnapshot(), supply)));
  assert.equal(parsed.issue, '');
  assert.equal(parsed.route.target.stopped_event, stopped.ref.stopped_event);
  assert.equal(controller.getSnapshot().drafts[stopped.id], undefined);
  assert.notEqual(supply.records(current.id, 'messages').items[0].text, supply.records(stopped.id, 'messages').items[0].text);
});
