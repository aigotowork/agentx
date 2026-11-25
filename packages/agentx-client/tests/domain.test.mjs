import assert from 'node:assert/strict';
import test from 'node:test';
import { DomainReferenceError, MutationOutcomeUnknown, ObservationStopped, OutcomeUnknown, ProtocolError, ReceiptError, selectAgentXScope } from '@agentx/client';

const baseUrl = 'http://management.invalid';
const expectedScope = '/data/isolated';
const generation = 'generation-1';
const reference = { name: 'mira', incarnation: '18446744073709551615' };
const requestId = '28e0dc4a-da88-49ee-af5d-b56eddb102d8';
const workspaceKey = '/work/target';
const respond = body => Response.json(body);
const descriptor = { scope: expectedScope, generation, project: '/work', tools: [], profiles: [], supports_workspace_members: true, supports_message_history: true, max_page: 100, max_text: 65536 };
const view = (agent, workspace) => ({
  agent, collaboration: { status: 'adopted', workspace, role: null }, directory: '/cwd/unrelated',
  tool: 'test', status: 'listening', stored_status: 'listening', status_context: '',
  detail: '', can_send: true, can_stop: true, can_resume: false, has_transcript: false,
});

// Scripted HTTP responses exercise the public package's projections and
// continuation; native membership, admission and effect count are separate evidence.
test('detached command survives response loss and recovery observes only its original receipt', async () => {
  let posted;
  let queries = 0;
  const scope = await selectAgentXScope({ baseUrl, expectedScope, async fetch(request) {
    const url = new URL(request.url);
    if (url.pathname === '/api/v1/management') return respond(descriptor);
    if (request.method === 'POST') {
      assert.equal(url.pathname, '/api/v1/agents/mira/messages');
      assert.equal(posted, undefined);
      posted = await request.json();
      throw new TypeError('simulated lost response');
    }
    assert.equal(url.pathname, `/api/v1/operations/${requestId}`);
    assert.equal(url.searchParams.get('generation'), generation);
    queries++;
    return respond({ generation, request_id: requestId, status: 'Succeeded', result: { Sent: { message_id: '27', recipients: ['mira'] } } });
  } });
  const agent = scope.agent({ scope: scope.address, reference: { ...reference, kind: 'current' } });
  const draft = { text: '', intent: 'ack', attachments: ['asset'], thread: '' };
  const prepared = agent.prepareMessage(draft, requestId);
  draft.text = 'changed'; draft.attachments.push('another');
  const restored = JSON.parse(JSON.stringify(prepared));
  const successor = scope.agent({ scope: scope.address, reference: { ...agent.address.reference, incarnation: '2' } });
  await assert.rejects(successor.send(restored), { code: 'ReferenceMismatch' });
  assert.throws(() => scope.operation({ ...restored.operation, scope: { ...scope.address, baseUrl: 'http://other.invalid' } }), { code: 'ScopeMismatch' });
  await assert.rejects(agent.send(restored), error => error instanceof OutcomeUnknown && error.input.request_id === requestId);
  assert.deepEqual(posted, { generation, agent: reference, request_id: requestId, message: { text: '', intent: 'ack', attachments: ['asset'], thread: '' } });
  assert.equal(queries, 0);
  const receipt = await scope.operation(restored.operation).wait();
  assert.equal(receipt.result.Sent.message_id, '27');
  assert.equal(queries, 1);
});

const op = (status, result = null, error = null) => ({ generation, request_id: requestId, status, result, error });
const stoppedRef = { name: 'mira', stopped_event: '17' };
const current = (scope, incarnation = reference.incarnation) => scope.agent({ scope: scope.address, reference: { kind: 'current', name: reference.name, incarnation } });
const stopped = scope => scope.agent({ scope: scope.address, reference: { ...stoppedRef, kind: 'stopped' } });
const transcript = (agent, available = true, exchanges = [], reason = null) => ({ Transcript: { agent, available, exchanges, reason } });

function scripted(steps, selectedDescriptor = descriptor) {
  return selectAgentXScope({ baseUrl, expectedScope, async fetch(request) {
    const url = new URL(request.url);
    if (url.pathname === '/api/v1/management') return respond(selectedDescriptor);
    assert.equal(request.headers.get('x-agentx-expected-scope'), encodeURIComponent(selectedDescriptor.scope));
    assert.ok(steps.length, `Unexpected ${request.method} ${url.pathname}`);
    return steps.shift()(request, url);
  } });
}

test('stopped Transcript response loss uses one POST and read-only recovery by original ID', async () => {
  const steps = [async (request, url) => {
    assert.equal(request.method, 'POST');
    assert.equal(url.pathname, '/api/v1/agents/mira/transcript');
    assert.deepEqual(await request.json(), { generation, request_id: requestId, agent: stoppedRef });
    throw new TypeError('lost response');
  }, (request, url) => {
    assert.equal(request.method, 'GET');
    assert.equal(url.pathname, `/api/v1/operations/${requestId}`);
    assert.equal(url.searchParams.get('generation'), generation);
    return respond(op('Succeeded', transcript(stoppedRef, false, [], 'No source')));
  }];
  const scope = await scripted(steps);
  const agent = stopped(scope);
  const original = JSON.parse(JSON.stringify(agent.prepareTranscript(requestId)));
  assert.equal(steps.length, 2);
  await assert.rejects(current(scope, '2').requestTranscript(original), { code: 'ReferenceMismatch' });
  await assert.rejects(agent.requestTranscript(original), error => error instanceof MutationOutcomeUnknown && error.input.request_id === requestId);
  const receipt = await scope.operation(original).observe();
  assert.equal(receipt.status, 'Succeeded');
  assert.deepEqual(receipt.result, { agent: agent.address, available: false, exchanges: [], reason: 'No source' });
  assert.equal(receipt.receipt.request_id, requestId);
  assert.equal(steps.length, 0);
  const reset = await scripted([], { ...descriptor, generation: 'reset' });
  assert.throws(() => reset.operation(original), { code: 'ScopeMismatch' });
  const stale = await scripted([(request, url) => {
    assert.equal(request.method, 'GET');
    assert.equal(url.searchParams.get('generation'), generation);
    return Response.json({ error: { code: 'StaleReference', message: 'reset' } }, { status: 409 });
  }]);
  await assert.rejects(stale.operation(original).observe(), { code: 'StaleReference' });
});

test('typed receipts distinguish acceptance, unknown and failure, and reject incompatible evidence', async () => {
  const exchange = { position: '1', at: '', user: 'hello', assistant: '', action: '', files: [] };
  const steps = [
    () => respond(op('Accepted')),
    () => respond(op('Running')),
    () => respond(op('Unknown', null, 'maybe completed')),
    () => respond(op('Failed', null, 'provider refused')),
    () => respond(op('Succeeded', transcript(reference))),
    () => respond(op('Running', transcript(reference))),
    () => respond(op('Succeeded', transcript({ ...reference, incarnation: '2' }))),
    () => respond(op('Succeeded', { Sent: { recipients: ['mira'], message_id: '1' } })),
    () => respond(op('Succeeded', transcript(reference, false, [exchange]))),
    () => respond({ ...op('Succeeded', transcript(reference)), request_id: 'wrong' }),
    () => respond(op('Succeeded', transcript(reference, 'not a boolean'))),
  ];
  const scope = await scripted(steps);
  const agent = current(scope);
  const prepared = agent.prepareTranscript(requestId);
  const admitted = await agent.requestTranscript(prepared);
  assert.deepEqual([admitted.status, admitted.result, admitted.error], ['Accepted', null, null]);
  const handle = scope.operation(prepared);
  const unknown = await handle.wait({ pollIntervalMs: 1 });
  assert.deepEqual([unknown.status, unknown.result, unknown.error], ['Unknown', null, 'maybe completed']);
  const failed = await handle.observe();
  assert.deepEqual([failed.status, failed.result, failed.error], ['Failed', null, 'provider refused']);
  const succeeded = await handle.observe();
  assert.equal(succeeded.result.available, true);
  assert.deepEqual(succeeded.result.exchanges, []);
  const incompatible = error => error instanceof ReceiptError && error.code === 'ProtocolError';
  await assert.rejects(handle.wait(), incompatible); // Invalid Running result must not be skipped while waiting.
  for (let i = 0; i < 4; i++) await assert.rejects(handle.observe(), incompatible);
  await assert.rejects(handle.observe(), error => error instanceof ProtocolError && error.rawText.includes('not a boolean'));
  assert.equal(steps.length, 0);
  await assert.rejects(handle.wait({ signal: AbortSignal.abort() }), error =>
    error instanceof ObservationStopped && error.code === 'Canceled' && error.identity.request_id === requestId);
});

test('direct creation provenance has an exact cursor and never navigates from a creation name', async () => {
  const next = 'fe4699fa-ef44-413e-b4b4-4b1cebe35697';
  const upstream = { name: 'sopa', incarnation: '1', availability: 'unavailable', launch_failure: 'not retained' };
  const child = { name: 'telo', incarnation: '2', availability: 'available', agent: view({ name: 'telo', stopped_event: '8' }, null) };
  const response = { generation, origin: 'agent', upstream, downstream: [child], next, fork_source: { name: 'source', stopped_event: '5' } };
  const steps = [(request, url) => {
    assert.equal(request.method, 'GET');
    assert.equal(url.pathname, '/api/v1/agents/mira/launch-relations');
    assert.equal(url.searchParams.get('stopped_event'), stoppedRef.stopped_event);
    return respond(response);
  }, (_, url) => {
    assert.equal(url.searchParams.get('after'), next);
    return respond({ ...response, next: null, downstream: [{ ...child, agent: view({ name: 'telo', incarnation: '3' }, null) }] });
  }];
  const scope = await scripted(steps);
  const agent = stopped(scope);
  const page = await agent.launchRelations({ limit: 1 });
  assert.equal(page.origin, 'agent');
  assert.deepEqual(page.forkSource, { kind: 'stopped', name: 'source', stopped_event: '5' });
  assert.deepEqual(page.upstream, { creation: { name: 'sopa', incarnation: '1' }, launchFailure: 'not retained', availability: 'unavailable', agent: null });
  const related = page.downstream[0];
  assert.equal(related.creation.incarnation, '2');
  assert.deepEqual(scope.agent(related.agent.address).address.reference, { kind: 'stopped', name: 'telo', stopped_event: '8' });
  await assert.rejects(current(scope, '2').launchRelations({ after: page.next }), { code: 'ReferenceMismatch' });
  await assert.rejects(agent.messages({ after: page.next }), { code: 'InvalidReference' });
  await assert.rejects(agent.launchRelations({ after: page.next }), { code: 'InconsistentResponse' });
  assert.equal(steps.length, 0);
});

test('exact lifecycle, scoped launch and label CAS retain separate command semantics', async () => {
  const steps = [async (request, url) => {
    assert.equal(url.pathname, '/api/v1/agents/mira/resume');
    assert.deepEqual((await request.json()).agent, stoppedRef);
    return respond(op('Succeeded', { Launched: { agents: ['mira'], failed: 0, errors: [] } }));
  }, async (request, url) => {
    assert.equal(url.pathname, '/api/v1/agents/mira/stop-participation');
    assert.deepEqual((await request.json()).agent, reference);
    return respond(op('Succeeded', { Stopped: { name: 'mira', outcome: 'participation_retained' } }));
  }, async (request, url) => {
    assert.equal(url.pathname, '/api/v1/agents/mira/labels');
    assert.deepEqual(await request.json(), { generation, request_id: requestId, agent: reference, set: {}, remove: ['focus'], if_revision: 'rev-old' });
    return respond(op('Succeeded', { LabelsPatched: { changed: true, snapshot: { generation, agent: reference, labels: {}, revision: 'rev-new' } } }));
  }, async (request, url) => {
    assert.equal(url.pathname, '/api/v1/agents');
    const { launch } = await request.json();
    assert.deepEqual(launch, { workspace: 'global', directory: '/work', role: 'coding', labels: { kind: 'saved' } });
    return respond(op('Accepted'));
  }];
  const scope = await scripted(steps);
  const retired = stopped(scope), active = current(scope);
  assert.equal(retired.kill, undefined);
  const resume = await retired.resume(retired.prepareResume(requestId));
  assert.deepEqual(resume.result, { agents: ['mira'], failed: 0, errors: [], agentAddresses: null });
  const participation = active.prepareStopParticipation(requestId);
  await assert.rejects(active.kill(participation), TypeError);
  assert.equal((await active.stopParticipation(participation)).result.outcome, 'participation_retained');
  const patch = { set: {}, remove: ['focus'], if_revision: 'rev-old' };
  const labels = active.prepareLabelsPatch(patch, requestId);
  patch.remove.push('other');
  assert.deepEqual((await active.patchLabels(labels)).result.snapshot, { agent: active.address, labels: {}, revision: 'rev-new' });
  const workspace = scope.workspace('global');
  const draft = { directory: '/work', role: 'coding', labels: { kind: 'saved' } };
  const launch = workspace.prepareLaunch(draft, requestId);
  draft.labels.kind = 'changed';
  await assert.rejects(scope.workspace('/work').launch(launch), { code: 'ReferenceMismatch' });
  assert.equal((await workspace.launch(launch)).status, 'Accepted');
  assert.equal(steps.length, 0);
});

test('attachments, human inbox, activity and directory use separate scope resources', async () => {
  const letter = { at: '', from: 'mira', id: '7', sender: reference, text: 'hello' };
  const role = { name: 'coding', path: '/roles/coding', namespace: 'project', profile: 'coding', default: true };
  const route = { generation, directory: '/work/route', role, endpoint: { status: 'bound', participant: reference, candidates: 1 } };
  const steps = [async (request, url) => {
    assert.equal(url.pathname, '/api/v1/attachments');
    assert.equal(url.searchParams.get('generation'), generation);
    assert.equal(await request.text(), 'hi');
    return respond({ id: 'asset', name: 'hi.txt', media_type: 'text/plain', size: 2, url: '/api/v1/attachments/asset' });
  }, (request, url) => {
    assert.equal(request.method, 'GET');
    assert.equal(url.pathname, '/api/v1/attachments/asset');
    return new Response('hi', { headers: { 'Content-Type': 'text/plain' } });
  }, (request, url) => {
    assert.equal(url.pathname, '/api/v1/human-mailbox');
    assert.equal(request.method, 'GET');
    assert.equal(url.searchParams.get('pending'), 'true');
    return respond({ generation, messages: [letter], next_before: '7', unhandled_count: 2 });
  }, async (request, url) => {
    assert.equal(url.pathname, '/api/v1/human-mailbox/handle');
    assert.equal(request.method, 'POST');
    assert.deepEqual(await request.json(), { generation, ids: ['7'] });
    return respond({ generation, messages: [{ ...letter, handled_at: 'now' }], unhandled_count: 1 });
  }, (request, url) => {
    assert.equal(url.pathname, '/api/v1/office-activity');
    assert.equal(request.method, 'GET');
    return respond({ generation, messages: [], created: [], has_more: false, next_after: '7', human_messages: [letter], human_has_more: false, human_unhandled_count: 1 });
  }, (request, url) => {
    assert.equal(request.method, 'GET');
    assert.equal(url.searchParams.get('directory'), '/work/route');
    return respond(route);
  }, async (request, url) => {
    assert.equal(url.pathname, '/api/v1/directory-messages');
    assert.equal(request.method, 'POST');
    assert.deepEqual(await request.json(), { generation, request_id: requestId, directory: '/work/route', message: { text: 'hello' } });
    return respond(op('Succeeded', { DirectorySent: { entry: route, recipient: reference, sent: { message_id: '9', recipients: ['mira'] }, created: false } }));
  }];
  const scope = await scripted(steps);
  const attachment = await scope.attachments.upload({ name: 'hi.txt', bytes: new Blob(['hi']), mediaType: 'text/plain' });
  assert.equal(attachment.size, 2n);
  await assert.rejects(scope.attachments.download({ ...attachment.address, scope: { ...scope.address, generation: 'other' } }), { code: 'ScopeMismatch' });
  assert.equal(await (await scope.attachments.download(attachment.address)).text(), 'hi');
  const pending = await scope.inbox.list({ pending: true });
  assert.equal(pending.unhandledCount, 2n);
  await assert.rejects(scope.inbox.list({ before: pending.next }), { code: 'ReferenceMismatch' });
  assert.equal((await scope.inbox.handle(['7'])).messages[0].handled_at, 'now');
  const activity = await scope.officeActivity();
  assert.equal(activity.checkpoint.position, '7');
  await assert.rejects(scope.officeActivity({ after: pending.next }), { code: 'InvalidReference' });
  const preview = await scope.directory.inspect('/work/route');
  assert.throws(() => scope.directory.prepareSend(preview.directory, { text: 'hello', attachments: ['asset'] }, requestId), TypeError);
  const sent = await scope.directory.send(scope.directory.prepareSend(preview.directory, { text: 'hello' }, requestId));
  assert.deepEqual(sent.result.recipient, current(scope).address);
  assert.equal(steps.length, 0);
});

test('explicit listener verification observes a reset without rebinding scope addresses', async () => {
  let described = descriptor;
  const scope = await selectAgentXScope({ baseUrl, expectedScope, async fetch(request) {
    assert.equal(request.method, 'GET');
    assert.equal(new URL(request.url).pathname, '/api/v1/management');
    return respond(described);
  } });
  described = { ...descriptor, max_page: 5 };
  assert.equal((await scope.verify()).max_page, 5);
  assert.equal(scope.descriptor.max_page, descriptor.max_page);
  described = { ...descriptor, generation: 'new-generation' };
  await assert.rejects(scope.verify(), { code: 'InconsistentResponse' });
  assert.equal(scope.address.generation, generation);
  described = { ...descriptor, scope: '/wrong' };
  await assert.rejects(scope.verify(), { code: 'ScopeMismatch' });
});

test('member pages bind the exact Workspace and selector', async () => {
  const nativeCursor = 'workspace-members:fixture:0:mira';
  const steps = [(_, url) => {
    assert.equal(url.pathname, '/api/v1/workspaces/members');
    assert.equal(url.searchParams.get('generation'), generation);
    assert.equal(url.searchParams.get('workspace'), 'global');
    assert.equal(url.searchParams.get('current_only'), 'true');
    assert.equal(url.searchParams.get('limit'), '1');
    return respond({ generation, agents: [view(reference, null)], current_complete: false, next: nativeCursor });
  }, (_, url) => {
    assert.equal(url.searchParams.get('after'), nativeCursor);
    return respond({ generation, agents: [view({ name: 'elsewhere', incarnation: '2' }, '/work/other')], current_complete: true, next: null });
  }];
  const scope = await scripted(steps, { ...descriptor, supports_workspace_members: true });
  const workspace = scope.workspace('global');
  const page = await workspace.memberPage({ currentOnly: true, limit: 1 });
  assert.equal(page.coverage.kind, 'native-workspace-members');
  assert.equal(page.coverage.next.position, nativeCursor);
  assert.deepEqual(page.members[0].snapshot.membership, { kind: 'adopted', workspace: 'global', role: null });
  assert.equal(page.members[0].snapshot.facts.directory, '/cwd/unrelated');
  const after = page.coverage.next;
  await assert.rejects(scope.workspace('/work/other').memberPage({ currentOnly: true, after }), { code: 'ReferenceMismatch' });
  await assert.rejects(workspace.memberPage({ currentOnly: false, after }), { code: 'ReferenceMismatch' });
  await assert.rejects(workspace.memberPage({ currentOnly: true, after: { workspace: workspace.address, position: '0:global' } }), { code: 'InvalidReference' });
  await assert.rejects(workspace.memberPage({ currentOnly: true, after }), { code: 'InconsistentResponse' });
  assert.equal(steps.length, 0);
});

test('launch recovery carries the original exact address and refuses a namesake or incompatible references', async () => {
  const launched = { agents: ['mira'], failed: 0, errors: [] };
  const steps = [
    () => respond(op('Accepted')),
    (_, url) => {
      assert.equal(url.pathname, `/api/v1/operations/${requestId}`);
      return respond(op('Succeeded', { Launched: { ...launched, agent_refs: [reference] } }));
    },
    (_, url) => {
      assert.equal(url.pathname, '/api/v1/agents/mira');
      assert.equal(url.searchParams.get('incarnation'), reference.incarnation);
      return respond(view({ ...reference, incarnation: '2' }, null));
    },
    () => respond(op('Succeeded', { Launched: { ...launched, agent_refs: [] } })),
    () => respond(op('Succeeded', { Launched: { ...launched, agent_refs: [stoppedRef] } })),
    () => respond(op('Succeeded', { Launched: { ...launched, agent_refs: [{ ...reference, name: 'other' }] } })),
  ];
  const scope = await scripted(steps);
  const prepared = scope.prepareLaunch({ role: 'coding', workspace: 'global' }, requestId);
  assert.equal((await scope.launch(prepared)).status, 'Accepted');
  const original = scope.operation(JSON.parse(JSON.stringify(prepared)));
  const recovered = await original.observe();
  const [exact] = recovered.result.agentAddresses;
  assert.deepEqual(exact, current(scope).address);
  await assert.rejects(scope.agent(exact).snapshot(), { code: 'InconsistentResponse' });
  for (let i = 0; i < 3; i++) await assert.rejects(original.observe(), error => error instanceof ReceiptError && error.code === 'ProtocolError');
  assert.equal(steps.length, 0);
});

test('latest and older message history keep distinct exact cursors and a forward watermark', async () => {
  const item = id => ({ id, at: '2026-11-21T00:00:00.000000+00:00', from: 'mira', text: id, recipients: [], attachments: [] });
  const boundary = '9007199254740997';
  const watermark = '9007199254741005';
  const latest = { generation, messages: [item(boundary), item('9007199254740999')], next_before: boundary, live_after: watermark };
  const steps = [
    (request, url) => {
      assert.equal(request.method, 'GET');
      assert.equal(url.pathname, '/api/v1/agents/mira/message-history');
      assert.equal(url.searchParams.get('generation'), generation);
      assert.equal(url.searchParams.get('incarnation'), reference.incarnation);
      assert.equal(url.searchParams.get('before'), null);
      assert.equal(url.searchParams.get('limit'), '2');
      return respond(latest);
    },
    (_, url) => {
      assert.equal(url.pathname, '/api/v1/agents/mira/message-history');
      assert.equal(url.searchParams.get('before'), boundary);
      assert.equal(url.searchParams.get('after'), null);
      return respond({ generation, messages: [item('9007199254740996')], next_before: null, live_after: null });
    },
    (_, url) => {
      assert.equal(url.pathname, '/api/v1/agents/mira/messages');
      assert.equal(url.searchParams.get('after'), watermark);
      return respond({ generation, messages: [item('9007199254741006')], next_after: '9007199254741006', has_more: false });
    },
    (_, url) => {
      assert.equal(url.searchParams.get('stopped_event'), stoppedRef.stopped_event);
      assert.equal(url.searchParams.get('incarnation'), null);
      return respond({ ...latest, generation: 'mismatched-generation' });
    },
  ];
  const scope = await scripted(steps, { ...descriptor, supports_message_history: true });
  assert.equal(scope.descriptor.supports_message_history, true);
  const agent = current(scope);
  const page = await agent.messageHistory({ limit: 2 });
  assert.deepEqual(page.messages.map(message => message.id), [boundary, '9007199254740999']);
  assert.deepEqual(page.nextBefore, { kind: 'message-history', agent: agent.address, position: boundary });
  assert.deepEqual(page.liveAfter, { agent: agent.address, position: watermark });
  assert.ok(Object.isFrozen(page.nextBefore));

  await assert.rejects(stopped(scope).messageHistory({ before: page.nextBefore }), { code: 'ReferenceMismatch' });
  await assert.rejects(current(scope, '1').messageHistory({ before: page.nextBefore }), { code: 'ReferenceMismatch' });
  const wrongGeneration = { ...page.nextBefore, agent: { ...page.agent, scope: { ...scope.address, generation: 'retired-generation' } } };
  await assert.rejects(agent.messageHistory({ before: wrongGeneration }), { code: 'ScopeMismatch' });
  await assert.rejects(agent.messages({ after: page.nextBefore }), { code: 'InvalidReference' });
  await assert.rejects(agent.messageHistory({ before: page.liveAfter }), { code: 'InvalidReference' });

  const older = await agent.messageHistory({ before: page.nextBefore });
  assert.equal(older.liveAfter, null);
  assert.equal(older.nextBefore, null);
  assert.equal(older.messages[0].id, '9007199254740996');
  assert.equal((await agent.messages({ after: page.liveAfter })).messages[0].id, '9007199254741006');
  await assert.rejects(stopped(scope).messageHistory(), { code: 'InconsistentResponse' });
  assert.equal(steps.length, 0);
});

test('a missing optional read only disables its own method', async () => {
  const workspacePage = { generation, agents: [view(reference, workspaceKey)], next: null, current_complete: true };
  const historyPage = { generation, messages: [], live_after: '7', next_before: null };
  for (const [missing, capability] of [['supports_workspace_members', 'workspace-members'], ['supports_message_history', 'message-history']]) {
    const steps = [
      (_, url) => { assert.equal(url.pathname, '/api/v1/agents'); return respond(workspacePage); },
      (_, url) => { assert.equal(url.pathname, '/api/v1/agents/mira'); return respond(view(reference, workspaceKey)); },
      (_, url) => { assert.equal(url.pathname, '/api/v1/agents/mira/messages'); return respond({ generation, messages: [], next_after: '7', has_more: false }); },
      (_, url) => {
        assert.equal(url.pathname, capability === 'workspace-members' ? '/api/v1/agents/mira/message-history' : '/api/v1/workspaces/members');
        return respond(capability === 'workspace-members' ? historyPage : workspacePage);
      },
    ];
    const scope = await scripted(steps, { ...descriptor, [missing]: undefined });
    const workspace = scope.workspace(workspaceKey), agent = current(scope);
    const unavailable = capability === 'workspace-members' ? workspace.memberPage() : agent.messageHistory();
    await assert.rejects(unavailable, { code: 'UnsupportedCapability', capability });
    assert.equal((await scope.agents()).agents.length, 1);
    assert.equal((await agent.snapshot()).facts.can_send, true);
    assert.equal((await agent.messages()).checkpoint.position, '7');
    if (capability === 'workspace-members') assert.equal((await agent.messageHistory()).liveAfter.position, '7');
    else {
      const { members } = await workspace.memberPage();
      await assert.rejects(members[0].agent.messageHistory(), { code: 'UnsupportedCapability', capability });
    }
    assert.equal(steps.length, 0);
  }
});

test('explicit verification keeps the pinned capabilities and a server refusal is local to the read', async () => {
  let observed = descriptor;
  const scope = await selectAgentXScope({ baseUrl, expectedScope, async fetch(request) {
    const path = new URL(request.url).pathname;
    if (path === '/api/v1/management') return respond(observed);
    assert.equal(path, '/api/v1/agents/mira/message-history');
    return Response.json({ error: { code: 'Unsupported', message: 'operation unavailable' } }, { status: 422 });
  } });
  observed = { ...descriptor, supports_message_history: false };
  assert.equal((await scope.verify()).supports_message_history, false);
  assert.equal(scope.descriptor.supports_message_history, true);
  await assert.rejects(current(scope).messageHistory(), { code: 'UnsupportedCapability', capability: 'message-history' });
});
