/** Finite SDK request-count and selector checks; browser interaction belongs to the verifier. */
import assert from 'node:assert/strict';
import { directory, directoryAgent, directoryReference } from '../host/live/client';
const originalFetch = globalThis.fetch;
const calls: URL[] = [];
let response = { generation: 'g1', agents: [], current_complete: false, next: 'opaque-current' as string | null };
globalThis.fetch = async input => {
  calls.push(new URL(input instanceof Request ? input.url : String(input)));
  return Response.json(response);
};
try {
  assert.deepEqual(await directory('g1'), { agents: [], next: 'opaque-current', currentComplete: false });
  assert.equal(calls.length, 1, 'first page returns without collecting the next page');
  assert.equal(calls[0].searchParams.get('current_only'), 'true');
  assert.equal(calls[0].searchParams.get('limit'), '100');
  assert.equal(calls[0].searchParams.has('after'), false);
  response = { ...response, next: null };
  await directory('g1', undefined, true, 'opaque-current');
  assert.equal(calls.length, 2, 'explicit next page adds exactly one request');
  assert.equal(calls[1].searchParams.get('after'), 'opaque-current');
  await directory('g1', undefined, false);
  assert.equal(calls[2].searchParams.get('current_only'), 'false', 'history is a deliberate independent selector');
  assert.equal(calls[2].searchParams.has('after'), false, 'current cursor never leaks into history');
  response = { ...response, generation: 'g2' };
  await assert.rejects(directory('g1'), /代次/);
  response = { ...response, generation: 'g1', next: 'loop' };
  await assert.rejects(directory('g1', undefined, true, 'loop'), /游标重复/);
  const stopped = { name: 'same', incarnation: null, stopped_event: '9007199254740993' };
  assert.deepEqual(directoryReference(JSON.stringify(['same', null, stopped.stopped_event])), stopped);
  for (const key of ['invalid', '["same",null,null]', '["same","1","2"]', '["../same",null,"1"]']) assert.equal(directoryReference(key), null);
  const exact = { agent: stopped, tool: 'codex', status: 'stopped', stored_status: 'active', status_context: '', detail: 'Stopped', directory: '/fixture', session_id: null, can_send: false, can_stop: false, can_resume: false, has_transcript: false, collaboration: { status: 'legacy', workspace: null, role: null, reason: null }, request_error: null };
  globalThis.fetch = async input => {
    calls.push(new URL(input instanceof Request ? input.url : String(input)));
    return Response.json(exact);
  };
  assert.equal((await directoryAgent('g1', stopped)).agent.stopped_event, stopped.stopped_event);
  assert.equal(calls.at(-1)!.pathname, '/api/v1/agents/same');
  assert.equal(calls.at(-1)!.searchParams.get('stopped_event'), stopped.stopped_event);
  await assert.rejects(directoryAgent('g1', { name: 'same', incarnation: '1', stopped_event: null }), /引用/);
  console.log('directory paging scenarios passed');
} finally { globalThis.fetch = originalFetch; }
