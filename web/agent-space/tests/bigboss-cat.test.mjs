/** Finite decorative time/geometry invariants; no claim of native or browser execution. */
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { existsSync } from 'node:fs';
import { registerHooks } from 'node:module';
registerHooks({ resolve(specifier, context, next) {
  if (specifier.startsWith('.') && context.parentURL) {
    const url = new URL(specifier, context.parentURL);
    if (!existsSync(url) && existsSync(new URL(`${url.href}.ts`))) return next(`${url.href}.ts`, context);
  }
  return next(specifier, context);
} });
const { sampleCat } = await import('../src/scene/cat.ts');
const { humanCatNest, humanYarnBall } = await import('../src/scene/office-art.ts');
const { projectHall } = await import('../src/scene/model.ts');

test('random repertoire returns home, stays before real walls, retains fixed endpoint and finite artwork', () => {
  const activities = new Set(), cache = new Set();
  for (const width of [192, 390, 1440]) {
    const scene = projectHall([], new Map([['/office', 'Office']]), width);
    const home = { ...scene.human }, wall = Math.min(...scene.rooms.map(room => room.origin.y));
    for (const seed of [.1, .4, .9]) for (let t = 0; t < 220000; t += 80) {
      const cat = sampleCat(scene, t, seed, false);
      activities.add(cat.activity);
      assert.ok(Number.isFinite(cat.at.x) && Number.isFinite(cat.at.y));
      assert.ok(cat.at.x >= 44 && cat.at.x <= scene.width - 44);
      assert.ok(cat.at.y <= wall - 20, 'feet/sprite never cross a room wall');
      assert.ok(cat.at.y - cat.lift >= 44, 'jump retains full cat within foyer');
      assert.ok(cat.lift >= 0 && cat.lift <= 24);
      assert.deepEqual(scene.human, home, 'mail and accessible entry never chase the cat');
      cache.add(humanCatNest('cat', cat.pose, cat.facing).key);
      if (cat.yarn) cache.add(humanYarnBall(cat.pose).key);
      assert.equal(sampleCat(scene, t, seed, true).activity, 'rest');
      assert.deepEqual(sampleCat(scene, t, seed, true).at, home);
    }
    for (let cycle = 0; cycle < 10; cycle++) assert.deepEqual(sampleCat(scene, cycle * 22000, .5, false).at, home);
  }
  assert.deepEqual([...activities].sort(), ['jump', 'patrol', 'rest', 'yarn']);
  assert.ok(cache.size <= 12);
});

test('insufficient room clearance suppresses travel rather than crossing an obstacle', () => {
  const scene = { human: { x: 80, y: 84 }, width: 192, height: 192, rooms: [{ origin: { x: 0, y: 100 } }] };
  for (let t = 0; t < 100000; t += 100) {
    const cat = sampleCat(scene, t, .1, false);
    assert.ok(['rest', 'jump'].includes(cat.activity));
    assert.deepEqual(cat.at, scene.human);
  }
});
