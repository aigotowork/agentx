/** Run the final public package against the same finite native scope as the Space browser check. */
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { selectAgentXScope } from '@agentx/client';

const configPath = process.argv[2];
const config = JSON.parse(await readFile(configPath, 'utf8'));
const requests = [];
const scope = await selectAgentXScope({
  baseUrl: config.nativeUrl, expectedScope: config.scope,
  fetch: request => {
    requests.push({ method: request.method, path: new URL(request.url).pathname });
    return fetch(request);
  },
});
assert.equal(scope.supportsWorkspaceMembers, true);
const member = await scope.workspace(config.workspace).memberPage({ limit: 1, currentOnly: true });
const absent = await scope.workspace(config.empty).memberPage({ limit: 1, currentOnly: true });
assert.equal(member.coverage.kind, 'native-workspace-members');
assert.equal(member.coverage.currentComplete, true);
assert.equal(member.members.length, 1);
assert.equal(absent.members.length, 0);
const [{ agent, snapshot }] = member.members;
assert.equal(snapshot.membership.workspace, config.workspace);
assert.notEqual(snapshot.facts.directory, config.workspace, 'cwd has different ownership from adopted membership');
const reference = { kind: 'current', name: config.reference.name, incarnation: config.reference.incarnation };
assert.deepEqual(agent.address.reference, reference);
const operation = { scope: scope.address, requestId: config.requestId };
const original = await scope.operation({ kind: 'launch', operation, launch: { workspace: config.workspace } }).observe();
assert.equal(original.status, 'Succeeded');
assert.deepEqual(original.result.agentAddresses, [agent.address]);
assert.deepEqual(original.result.agents, [reference.name]);
const provenance = await agent.launchRelations({ limit: 1 });
assert.deepEqual(provenance.agent, agent.address);
assert.deepEqual(provenance.downstream, []);
assert.ok(requests.every(({ method }) => method === 'GET'), 'recovery never repeats launch');
await writeFile(join(dirname(configPath), 'domain-native-report.json'), JSON.stringify({ status: 'pass', reference, requests }, null, 2));
