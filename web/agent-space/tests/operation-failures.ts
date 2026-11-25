/** Disposable SDK replies exercise uncertain launch handling and original receipt reconciliation. */
import assert from 'node:assert/strict';
import { LiveSpace, operationIssue } from '../src/live/controller';
import { selectAgentXScope } from '@agentx/client';
import { DomainOfficeReads } from '../src/live/domain-office';
import type { Descriptor, Operation } from '../../management/api/native';
const originalFetch = globalThis.fetch;
const originalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
const memory = new Map<string, string>();
Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: {
  getItem: (key: string) => memory.get(key) ?? null,
  setItem: (key: string, value: string) => memory.set(key, value),
} });
const descriptor: Descriptor = { scope: '/disposable', generation: 'g1', project: '/test', tools: [], profiles: [], supports_workspace_members: true, supports_message_history: true, max_page: 100, max_text: 65536 };
const calls: { method: string; id: string | null }[] = [];
let operation: Operation;
let outcome: Operation['status'] = 'Unknown';
globalThis.fetch = async input => {
  const request = input as Request;
  if (new URL(request.url).pathname === '/api/v1/management') return Response.json(descriptor);
  if (request.method === 'POST') {
    const body = await request.json();
    assert.ok(memory.size, 'original receipt is durable before submission');
    operation = { request_id: body.request_id, generation: body.generation, status: outcome, error: 'worker executable unavailable', result: null };
    calls.push({ method: request.method, id: body.request_id });
  } else calls.push({ method: request.method, id: new URL(request.url).pathname.split('/').at(-1)! });
  return Response.json(operation);
};
const launch = { directory: '/test', role: null, tool: null, profile: null, workspace: null, prompt: null, title: null };
try {
  const scope = await selectAgentXScope({ baseUrl: 'http://disposable.invalid', expectedScope: descriptor.scope });
  function bound() {
    const live = new LiveSpace(() => {});
    live.descriptor = scope.descriptor;
    Reflect.set(live, 'domain', new DomainOfficeReads(scope, 20, () => {}));
    return live;
  }
  const live = bound();
  await assert.rejects(live.launchNew(launch, descriptor), /结果未确定.*worker executable unavailable/);
  assert.equal(live.blocked('launch:new'), true, 'Unknown must block another launch');
  const receipt = live.latestLaunch('launch:new')!;
  assert.match(operationIssue(receipt)!, /结果未确定/);
  await assert.rejects(live.launchNew(launch, descriptor), /原回执/);
  assert.equal(calls.length, 1, 'blocked launch never replays');
  const restored = bound();
  assert.match(operationIssue(restored.latestLaunch('launch:new')!)!, /worker executable unavailable/, 'restored errors remain visible');
  operation = { ...operation!, status: 'Failed' };
  await live.lookup(receipt);
  assert.equal(calls[1].method, 'GET');
  assert.equal(calls[1].id, receipt.id, 'reconciliation queries the original request');
  assert.match(operationIssue(receipt)!, /操作失败.*worker executable unavailable/);
  assert.equal(live.blocked('launch:new'), false);
  live.descriptor = { ...descriptor, generation: 'g2' };
  assert.equal(live.latestLaunch('launch:new'), undefined, 'old dataset feedback is fenced');
  outcome = 'Failed';
  live.descriptor = descriptor;
  await assert.rejects(live.launchNew(launch, descriptor), /操作失败.*worker executable unavailable/);
  console.log('Space operation failure scenarios passed');
} finally {
  globalThis.fetch = originalFetch;
  if (originalStorage) Object.defineProperty(globalThis, 'localStorage', originalStorage);
  else Reflect.deleteProperty(globalThis, 'localStorage');
}
