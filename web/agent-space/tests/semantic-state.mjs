/** Slice-1 semantic cases: transpile only pure frontend owners into a disposable ESM directory. */
import assert from 'node:assert/strict';
import { stripTypeScriptTypes } from 'node:module';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
const temp = await mkdtemp(join(tmpdir(), 'agent-space-state-'));
const results = [];
try {
  for (const name of ['model', 'fixture', 'controller', 'projection']) {
    const source = await readFile(new URL(`../src/offline/${name}.ts`, import.meta.url), 'utf8');
    const compiled = stripTypeScriptTypes(source, { mode: 'transform' }).replace(/from "\.\/(\w+)"/g, 'from "./$1.mjs"');
    await writeFile(join(temp, `${name}.mjs`), compiled);
  }
  const load = name => import(pathToFileURL(join(temp, `${name}.mjs`)));
  const [{ createOfflineSupply }, { SpaceController }, projection, model] = await Promise.all(['fixture', 'controller', 'projection', 'model'].map(load));
  const supply = createOfflineSupply(), controller = new SpaceController(supply);
  const current = name => supply.agents.find(agent => agent.name === name && agent.ref.record === 'current');
  const stopped = supply.agents.find(agent => agent.ref.record === 'stopped');
  const web = '/projects/atlas/web', atlas = '/projects/atlas';
  function check(name, run) { run(); results.push({ name, status: 'passed' }); }
  check('S1 exact directory/global/unknown and empty offices', () => {
    assert.equal(supply.workspaces.length, 3);
    assert.equal(projection.currentMembers(projection.officeMembers(supply, web)).length, 5);
    assert.deepEqual(projection.officeMembers(supply, atlas).map(a => a.name), ['tobi']);
    assert.equal(projection.officeMembers(supply, '/projects/empty').length, 0);
    assert.deepEqual(projection.officeMembers(supply, model.GLOBAL_OFFICE).map(a => a.name), ['gavo']);
    assert.deepEqual(projection.officeMembers(supply, model.UNKNOWN_OFFICE).map(a => a.name), ['keni']);
    assert.equal(current('tobi').cwd, web);
  });
  check('S2 adopted provenance matches two actual current members; vacant post has none', () => {
    const fronter = supply.posts.find(post => post.office === web && post.name === 'fronter');
    assert.deepEqual(projection.matchingPostMembers(supply, fronter).map(a => a.name), ['luna', 'mora']);
    assert.equal(fronter.origin, '继承'); assert.notEqual(fronter.revision, current('luna').role.revision);
    assert.notEqual(fronter.profile, current('luna').role.profile);
    assert.equal(projection.matchingPostMembers(supply, supply.posts.find(post => post.office === web && post.name === 'controller')).length, 0);
    assert.equal(model.membershipKey(current('sira')), web); assert.equal(current('sira').role.source.startsWith(supply.scope.id), true);
  });
  check('S4 confirmed no Role differs from unknown and cwd cannot establish membership', () => {
    assert.equal(model.adoptedRoleLabel(current('nora').role), '未采用 Role');
    assert.equal(model.adoptedRoleLabel(current('riku').role), '岗位未知');
    assert.equal(current('keni').membership.kind, 'unknown'); assert.equal(current('keni').cwd, web);
  });
  check('S5 stopped namesake and failed/idle current members have separate exact keys', () => {
    assert.notEqual(stopped.id, current('mora').id);
    assert.equal(new Set(supply.agents.map(a => a.id)).size, supply.agents.length);
    for (const agent of supply.agents) assert.equal(agent.id, model.exactAgentKey(agent.ref));
    for (const field of ['scope', 'generation', 'incarnation', 'record']) assert.notEqual(model.exactAgentKey({ ...current('mora').ref, [field]: 'other' }), current('mora').id);
    controller.office(web);
    assert.equal(projection.visibleMembers(supply, controller.getSnapshot()).length, 5);
    controller.view('history'); assert.deepEqual(projection.visibleMembers(supply, controller.getSnapshot()).map(a => a.id), [stopped.id]);
    controller.view('members'); assert(projection.visibleMembers(supply, controller.getSnapshot()).some(a => a.name === 'riku'));
  });
  check('S5/S7 exact drafts survive office/view/tab/filter and stopped-name changes', () => {
    const mora = current('mora'); controller.select(mora.id); controller.writeDraft(mora.id, 'current draft');
    controller.channel(mora.id, 'model'); controller.presentation('chat'); controller.office(atlas); controller.view('posts');
    controller.select(stopped.id); controller.writeDraft(stopped.id, 'old draft'); controller.filter({ query: 'missing' });
    controller.office(web); controller.select(mora.id);
    assert.equal(controller.getSnapshot().drafts[mora.id], 'current draft');
    assert.equal(controller.getSnapshot().drafts[stopped.id], 'old draft'); assert.equal(controller.getSnapshot().channels[mora.id], 'model');
  });
  check('S6 authored message envelopes, model empty/unavailable/error and actual supplied blocks', () => {
    const messages = supply.records(current('luna').id, 'messages');
    assert.equal(messages.items[0].author, '@tobi'); assert.deepEqual(messages.items[0].recipients, ['@luna']);
    assert.equal(messages.items[1].author, '@luna'); assert.deepEqual(messages.items[1].recipients, ['@tobi']);
    assert.equal(messages.items[1].replyTo, messages.items[0].id); assert.equal(messages.items.some(r => r.kind !== 'message'), false);
    assert.equal(supply.records(current('sira').id, 'model').items.length, 0);
    assert.equal(supply.records(current('mora').id, 'model').status, 'unavailable');
    assert.equal(supply.records(current('riku').id, 'model').status, 'error');
    assert(supply.records(current('luna').id, 'model').items.some(r => r.kind === 'tool'));
  });
} finally { await rm(temp, { recursive: true, force: true }); }
console.log(JSON.stringify(results, null, 2));
