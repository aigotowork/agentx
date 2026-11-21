/** Check AgentX source correspondence, exact domain routes and generated files. */
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve, dirname, sep } from 'node:path';
import { runInNewContext } from 'node:vm';
import { prepare, publish } from '../../../tools/contract-canvas/build.mjs';

const directory = dirname(fileURLToPath(import.meta.url));
const root = resolve(directory, '../../..');
const read = name => readFileSync(resolve(directory, name), 'utf8');
const model = JSON.parse(read('contract-model.json'));
const view = JSON.parse(read('../view.json'));
const fixture = JSON.parse(read('fixture.json'));
const page = prepare(model, view, { viewerPath: 'viewer', dataPath: 'model/graph.js' });
const {nodes, anchors, sources, ownerNode, refs, hasEdge} = page.index;
const sdkStates = new Set(['implemented', 'projection', 'external', 'deferred']);
const nativeStates = new Set(['implemented', 'mapped', 'not-applicable', 'deferred']);

for (const source of sources.values()) {
  const path = resolve(root, source.path);
  assert(path.startsWith(root + sep), `source outside repository: ${source.path}`);
  assert(existsSync(path), `missing source ${source.path}`);
  const line = readFileSync(path, 'utf8').split('\n')[source.line - 1];
  assert(line?.includes(source.symbol), `stale source anchor ${source.path}:${source.line} ${source.symbol}`);
}
for (const item of [...model.layers, ...model.nodes, ...model.edges, ...model.scenarios]) {
  assert(item.sourceRefs.length > 0, `${item.id}: missing source evidence`);
}
for (const item of [...model.nodes, ...model.edges]) {
  const proposal = item.provenance === 'proposed';
  assert(item.sourceRefs.some(id => (sources.get(id).authority === 'proposal') === proposal), `${item.id}: evidence does not support provenance`);
}
for (const node of model.nodes) {
  assert(sdkStates.has(node.support?.sdk), `${node.id}.support.sdk`);
  assert(nativeStates.has(node.support?.native), `${node.id}.support.native`);
  assert(node.support.note, `${node.id}: support note is required`);
  if (node.provenance === 'proposed') {
    assert.equal(node.support.sdk, 'deferred', `${node.id}: proposal cannot claim implemented SDK`);
    assert.equal(node.support.native, 'not-applicable', `${node.id}: application proposal is not native support`);
  }
}
refs(fixture.requiredNodes, 'fixture requiredNodes');
for (const route of fixture.routes) {
  refs(route, 'fixture route', anchors);
  for (let i = 1; i < route.length; i++) assert(hasEdge(route[i - 1], route[i]), `broken fixture route ${route[i - 1]} -> ${route[i]}`);
}
for (const port of fixture.forbiddenPorts) assert(!nodes.has(port), `must not expose ${port}`);
const field = (id, name) => anchors.get(`${id}.${name}`);
assert.equal(nodes.get('NativeWorkspaceMembershipPaging').support.native, 'implemented');
assert.equal(nodes.get('BoundedWorkspaceMemberProjection').support.sdk, 'implemented');
assert.equal(field('Management.Descriptor', 'supports_workspace_members').optional, true);
for (const [method, request, coverage, cursor] of [
  ['Workspace.members', 'PageRequest<MemberCursor>', 'MemberCoverage', 'MemberCursor'],
  ['Workspace.memberPage', 'NativeMemberRequest', 'NativeMemberCoverage', 'NativeMemberCursor'],
]) {
  assert.deepEqual(nodes.get(method).inputs[0].typeIds, [request]);
  assert.deepEqual(field(request, 'after').typeIds, [cursor]);
  assert.equal(field(request, 'after').optional, true);
  assert.deepEqual(field(coverage, 'next').typeIds, [cursor]);
  assert.equal(field(coverage, 'next').nullable, true);
}
assert.equal(field('NativeMemberRequest', 'currentOnly').optional, true);
assert.equal(field('NativeMemberCursor', 'currentOnly').type, 'boolean');
assert(!hasEdge('Workspace.memberPage', 'Management.agents.list', 'calls'), 'native membership cannot implicitly use the legacy projection');
assert(!nodes.get('Transcript').fields.some(f => /cursor|next|checkpoint/i.test(f.name)), 'Provider transcript has no continuation');
assert.deepEqual(field('LaunchResult', 'agentAddresses').typeIds, ['AgentAddress<CurrentReference>']);
assert.equal(field('LaunchResult', 'agentAddresses').nullable, true, 'older launch receipts may omit exact references');
assert.equal(field('RelatedConversation', 'agent').nullable, true, 'an unavailable creation record does not supply navigation');
assert.equal(nodes.get('AgentXScope.operation').effect, 'local');
assert.equal(nodes.get('TypedOperationHandle.observe').effect, 'observe');
assert.equal(nodes.get('TypedOperationHandle.wait').effect, 'observe');
assert.deepEqual(nodes.get('OperationHandle.wait').outcomes, ['OperationStatus.Succeeded', 'OperationStatus.Failed', 'OperationStatus.Unknown']);
assert.deepEqual(nodes.get('TypedOperationHandle.wait').outcomes, nodes.get('OperationHandle.wait').outcomes);
assert.equal(nodes.get('Agent').kind, 'union');
assert(!model.edges.some(e => e.kind === 'extends' && (e.from === 'Agent' || e.to === 'Agent')), 'SDK Agent is not a class base');
assert.equal(nodes.get('CurrentActor').provenance, 'proposed');
for (const id of ['AgentSnapshot', 'AgentFacts', 'Workspace', 'OperationReceipt']) assert.equal(nodes.get(id).layerId, 'sdk', `${id}: domain meaning owns layer, not the cited DTO`);
assert(hasEdge('CurrentActor', 'AgentActor', 'extends'));
assert(hasEdge('AgentActor', 'Actor', 'implements'));
assert(hasEdge('AgentActor.agent', 'Agent', 'wraps'));
assert(hasEdge('CurrentActor.send', 'CurrentAgent.send', 'delegates'));
assert(!nodes.has('AgentActor.send'), 'read-only proposed actor must not gain sender capability');
const conversation = model.scenarios.find(scenario => scenario.id === 'office-conversation-draft');
assert.equal(conversation?.provenance, 'proposed');
assert.equal(conversation.steps.length, 5);
for (const step of conversation.steps) for (const id of step.nodeIds) {
  const node = ownerNode(id);
  assert.equal(node.layerId, 'application', `${id}: default scenario must stay in its application layer`);
  assert.notEqual(node.visibility, 'implementation', `${id}: private dependency leaked into scenario step`);
  assert.notEqual(anchors.get(id).visibility, 'implementation', `${id}: private anchor leaked into scenario step`);
  assert.notEqual(node.effect, 'submit', `${id}: reading/draft path must not submit`);
}
for (const id of ['Office.workspace', 'AgentActor.agent', 'CurrentActor.agent', 'OfficeRole.role', 'OfficeSession.drafts', 'OfficeSession.viewRevision', 'ConversationSource']) {
  assert.equal(anchors.get(id).visibility, 'implementation', `${id}: keep receiver/state behind implementation disclosure`);
}
for (const port of ['OfficeSession.selectActor', 'OfficeSession.openConversation', 'OfficeSession.closeConversation', 'ActorConversation.editDraft', 'ActorConversation.sendAvailability']) {
  const node = nodes.get(port);
  assert.equal(node.effect, 'local', `${port}: no I/O`);
  const publicTypes = [...node.inputs.flatMap(input => input.typeIds), ...node.output.typeIds];
  assert(publicTypes.every(id => nodes.get(id).layerId === 'application' && nodes.get(id).visibility !== 'implementation'), `${port}: public contract leaked implementation type`);
}
assert.equal(nodes.get('ActorConversation.readMessages').effect, 'read');
assert.equal(nodes.get('ActorConversation.readMessages').output.typeIds[0], 'ConversationRead');
assert(hasEdge('ActorConversation.editDraft', 'OfficeSession.drafts', 'references'));
assert(hasEdge('OfficeSession.closeConversation', 'OfficeSession.drafts', 'references'));
assert(hasEdge('OfficeSession.openConversation', 'MessageDraft', 'references'));
assert.equal(anchors.get('ActorKey.discriminator').type, 'string', 'exact actor keys must retain decimal text');
publish(page, {
  index: resolve(directory, '../index.html'),
  data: resolve(directory, 'graph.js'),
  write: process.argv.includes('--write-wrapper'),
});
assert.equal(read('../index.html'), page.html, 'offline entry differs from the shared template binding');
assert.equal(read('graph.js'), page.script, 'offline data differs from prepared content');
const sandbox = {};
runInNewContext(read('graph.js'), sandbox, { timeout: 2000 });
assert.deepEqual(JSON.parse(JSON.stringify(sandbox.CONTRACT_MODEL)), model, 'offline script differs from canonical JSON');
assert.deepEqual(JSON.parse(JSON.stringify(sandbox.CONTRACT_VIEW)), view, 'offline script differs from canonical view');
const api = sandbox.CONTRACT_LAYERS;
assert.deepEqual(JSON.parse(JSON.stringify(api.layers.flatMap(layer => layer.nodeIds))).sort(), [...nodes.keys()].sort());
assert.equal(api.layerForNode('Office.workspace').id, 'application');
assert.equal(api.layerForNode('Workspace.members').id, 'sdk');
assert.equal(api.layerForNode('Management.AgentView').id, 'transport');
assert.equal(api.layerForNode('AbortSignal').id, 'platform');
assert.equal(api.layer('missing'), undefined);
assert.equal(api.layerForNode('missing'), undefined);
assert.equal(api.layerMappings('missing').length, 0);
for (const layer of model.layers) {
  const expected = model.edges.filter(edge => ownerNode(edge.from).layerId !== ownerNode(edge.to).layerId && [ownerNode(edge.from).layerId, ownerNode(edge.to).layerId].includes(layer.id));
  assert.deepEqual(Array.from(api.layerMappings(layer.id), edge => edge.id), expected.map(edge => edge.id), `${layer.id}: flattened or missing boundary relationships`);
}
console.log(`Contract model OK: ${model.layers.length} layers, ${model.nodes.length} nodes, ${anchors.size - nodes.size} fields, ${model.edges.length} edges, ${model.sources.length} source anchors, ${model.scenarios.length} scenarios; ${fixture.routes.length} navigation routes; layer API and offline wrapper identical.`);
