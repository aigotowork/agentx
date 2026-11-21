/** Exercise Space's real browser journal and exact Transcript recovery without a daemon. */
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { selectAgentXScope } from '@agentx/client';
import './source-imports.mjs';

const { LiveSpace } = await import('../src/live/controller.ts');
const { DomainOfficeReads } = await import('../src/live/domain-office.ts');
const { domainAgentKey } = await import('../src/live/projection.ts');

test('lost Transcript submission restores the original exact request, without reissuing it', async () => {
  const originalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const memory = new Map();
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: {
    getItem: key => memory.get(key) ?? null,
    setItem: (key, value) => memory.set(key, value),
  } });
  const descriptor = { scope: '/isolated', generation: 'generation-a', project: '/workspace', tools: [], profiles: [], max_page: 100, max_text: 65536 };
  const calls = [];
  let original;
  let correctTarget = false;
  try {
    const scope = await selectAgentXScope({
      baseUrl: 'http://transcript-fixture.invalid', expectedScope: descriptor.scope,
      async fetch(request) {
        const { pathname } = new URL(request.url);
        if (pathname === '/api/v1/management') return Response.json(descriptor);
        if (request.method === 'POST') {
          assert.equal(pathname, '/api/v1/agents/mavu/transcript');
          original = await request.json();
          calls.push(['POST', original.request_id]);
          const [saved] = JSON.parse(memory.get('agentx.space.operations.v1'));
          assert.equal(saved.id, original.request_id, 'must save the original request before the POST');
          throw new TypeError('submit response lost');
        }
        assert.equal(request.method, 'GET');
        const requestId = pathname.split('/').at(-1);
        assert.equal(pathname, `/api/v1/operations/${original.request_id}`);
        calls.push(['GET', requestId]);
        return Response.json({
          request_id: requestId, generation: descriptor.generation, status: 'Succeeded', error: null, result: {
            Transcript: {
              agent: { name: 'mavu', incarnation: correctTarget ? '1' : '2' },
              available: true, reason: null, exchanges: [{ position: '1', at: '', user: 'exact original conversation', assistant: '', action: '', files: [] }],
            },
          },
        });
      },
    });
    const address = incarnation => ({ scope: scope.address, reference: { kind: 'current', name: 'mavu', incarnation } });
    const id = domainAgentKey(address('1'));
    const successor = domainAgentKey(address('2'));
    const bind = () => {
      const live = new LiveSpace(() => {});
      live.descriptor = scope.descriptor;
      live.domain = new DomainOfficeReads(scope, 20, () => {});
      for (const [key, incarnation] of [[id, '1'], [successor, '2']]) {
        live.domain.subjects.set(key, { address: address(incarnation), membership: { kind: 'legacy' }, facts: { has_transcript: true } });
      }
      return live;
    };
    const first = bind();
    assert.equal(await first.target('transcript', id), false);
    assert.equal(original.agent.incarnation, '1');
    assert.deepEqual(calls, [['POST', original.request_id]]);

    const restored = bind();
    const [receipt] = restored.receipts;
    assert.equal(receipt.id, original.request_id);
    assert.equal(restored.blocked(id, 'transcript'), true);
    await restored.lookup(receipt);
    assert.equal(receipt.operation, undefined, 'a different incarnation cannot settle the saved command');
    assert.match(receipt.issue, /original exact Agent/);
    assert.equal(restored.supply.records(id, 'model').status, 'unavailable');
    assert.equal(restored.supply.records(successor, 'model').status, 'unavailable');

    correctTarget = true;
    await restored.lookup(receipt);
    assert.equal(receipt.operation?.status, 'Succeeded');
    assert.equal(restored.supply.records(id, 'model').items[0].text, 'exact original conversation');
    assert.equal(restored.supply.records(successor, 'model').status, 'unavailable');
    assert.deepEqual(calls, [['POST', original.request_id], ['GET', original.request_id], ['GET', original.request_id]]);
  } finally {
    if (originalStorage) Object.defineProperty(globalThis, 'localStorage', originalStorage);
    else Reflect.deleteProperty(globalThis, 'localStorage');
  }
});
