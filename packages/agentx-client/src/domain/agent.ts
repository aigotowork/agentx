/** Exact Agent handles, immutable observations and a caller-retainable message command. */
import type { AgentView, MessageInput, RelatedAgent, SubmitLabels, SubmitMessage, SubmitTarget } from '../generated/management/types.gen.js';
import type { ManagementClient } from '../management.js';
import { freezeInput } from '../transport.js';
import type { CallOptions } from '../transport.js';
import { DomainReferenceError, readReference, requestId, requireAgent, requireGeneration, requireScope, wireReference } from './address.js';
import type { Agent, AgentAddress, AgentReference, AgentSnapshot, CurrentReference, LabelsPatch, LaunchRelationCursor, LaunchRelationsPage, Membership, MessageCursor, MessageHistoryPage, MessageHistoryRequest, PageRequest, PreparedMessage, PreparedOperation, ReadonlyValue, RelatedConversation, ScopeAddress } from './contracts.js';
import { checkedReceipt, killResult, labelsResult, launchResult, operationAddress, prepareTarget, projectLabels, retainInput, stopParticipationResult, submitTarget, targetInput, transcriptResult } from './operation.js';
import { readWhenSupported, type ReadCapabilities } from './read-capabilities.js';

export function projectSnapshot(scope: ScopeAddress, view: AgentView): AgentSnapshot {
  const { agent, collaboration, ...facts } = view;
  let membership: Membership;
  if (collaboration.status === 'adopted' && (collaboration.workspace === null || collaboration.workspace && collaboration.workspace !== 'global')) {
    membership = { kind: 'adopted', workspace: collaboration.workspace ?? 'global', role: collaboration.role ?? null };
  } else if (collaboration.status === 'legacy') {
    membership = { kind: 'legacy' };
  } else {
    membership = { kind: 'unavailable', reason: collaboration.reason ?? null };
  }
  return freezeInput({ address: { scope, reference: readReference(agent) }, membership, facts });
}

function projectRelated(scope: ScopeAddress, row: RelatedAgent): RelatedConversation {
  const { name, incarnation } = row;
  readReference({ name, incarnation });
  const base = { creation: { name, incarnation }, launchFailure: row.launch_failure ?? null };
  if (row.availability === 'unavailable' && row.agent == null) return { ...base, availability: 'unavailable', agent: null };
  if (row.availability === 'available' && row.agent != null) {
    const agent = projectSnapshot(scope, row.agent);
    const reference = agent.address.reference;
    // Native resolves the stopped event by its recorded creation. The stopped view doesn't repeat the incarnation.
    if (reference.name === name && (reference.kind === 'stopped' || reference.incarnation === incarnation)) {
      return { ...base, availability: 'available', agent };
    }
  }
  throw new DomainReferenceError('InconsistentResponse', 'Launch relation does not match its exact creation and availability');
}

function messageInput(target: AgentAddress<CurrentReference>, message: ReadonlyValue<MessageInput>, id: string): SubmitMessage {
  const { attachments, ...body } = message;
  if (attachments !== undefined && !Array.isArray(attachments)) throw new TypeError('Attachments must be an array of IDs');
  return {
    generation: target.scope.generation,
    request_id: requestId(id),
    agent: wireReference(target.reference),
    message: attachments === undefined ? body : { ...body, attachments: [...attachments] },
  };
}

function labelsInput(target: SubmitTarget, patch: LabelsPatch): SubmitLabels {
  return { ...target, set: patch.set, remove: patch.remove === undefined ? undefined : [...patch.remove], if_revision: patch.if_revision };
}

export function bindAgent(client: ManagementClient, scope: ScopeAddress, capabilities: ReadCapabilities, reference: AgentReference): Agent {
  const exact = wireReference(reference);
  const ownReference = readReference(exact);
  const exactInput = { generation: scope.generation, agent: exact };

  function reader<R extends AgentReference>(ref: R) {
    const address = Object.freeze({ scope, reference: ref });
    return {
      address,
      async snapshot(options?: CallOptions) {
        const snapshot = projectSnapshot(scope, await client.agents.get(exactInput, options));
        try {
          requireAgent(snapshot.address, address);
        } catch (cause) {
          if (!(cause instanceof DomainReferenceError)) throw cause;
          throw new DomainReferenceError('InconsistentResponse', 'Snapshot does not match the requested exact Agent');
        }
        return Object.freeze({ ...snapshot, address });
      },
      async messages(page: PageRequest<MessageCursor> = {}, options?: CallOptions) {
        if (page.after) {
          if (page.after.kind !== undefined) throw new DomainReferenceError('InvalidReference', 'Expected an AgentX message cursor');
          requireAgent(page.after.agent, address);
        }
        const result = await client.agents.messages.list({ ...exactInput, after: page.after?.position, limit: page.limit }, options);
        requireGeneration(result.generation, scope);
        return freezeInput({
          agent: address,
          messages: result.messages,
          checkpoint: { agent: address, position: result.next_after },
          hasMore: result.has_more,
        });
      },
      async messageHistory(page: MessageHistoryRequest = {}, options?: CallOptions): Promise<MessageHistoryPage> {
        if (page.before) {
          if (page.before.kind !== 'message-history') throw new DomainReferenceError('InvalidReference', 'Expected an AgentX message history cursor');
          requireAgent(page.before.agent, address);
        }
        const result = await readWhenSupported(capabilities, 'message-history',
          () => client.agents.messages.history({ ...exactInput, before: page.before?.position, limit: page.limit }, options));
        requireGeneration(result.generation, scope);
        if ((result.live_after === null) !== (page.before !== undefined)) {
          throw new DomainReferenceError('InconsistentResponse', 'Unexpected live checkpoint for this message history page');
        }
        return freezeInput({
          agent: address,
          messages: result.messages,
          nextBefore: result.next_before === null ? null : { kind: 'message-history', agent: address, position: result.next_before },
          liveAfter: result.live_after === null ? null : { agent: address, position: result.live_after },
        });
      },
      async launchRelations(page: PageRequest<LaunchRelationCursor> = {}, options?: CallOptions): Promise<LaunchRelationsPage> {
        if (page.after) {
          if (page.after.kind !== 'launch-relations') throw new DomainReferenceError('InvalidReference', 'Expected a launch relation cursor');
          requireAgent(page.after.agent, address);
        }
        const result = await client.agents.relations({ ...exactInput, after: page.after?.position, limit: page.limit }, options);
        requireGeneration(result.generation, scope);
        const { origin, upstream, downstream, next, fork_source } = result;
        if ((origin !== 'agent' && origin !== 'external' && origin !== 'unknown') || (origin === 'agent') !== (upstream != null)) {
          throw new DomainReferenceError('InconsistentResponse', 'Unexpected launch relation origin or upstream');
        }
        return freezeInput({ agent: address, origin, upstream: upstream == null ? null : projectRelated(scope, upstream),
          downstream: downstream.map(row => projectRelated(scope, row)), forkSource: fork_source == null ? null : readReference(fork_source), next: next == null ? null : { kind: 'launch-relations', agent: address, position: next } });
      },
      prepareTranscript: (id: string) => prepareTarget('transcript', address, id),
      requestTranscript: (prepared: PreparedOperation<'transcript'>, options?: CallOptions) => submitTarget('transcript', address, prepared, options, client.agents.transcript, transcriptResult(address)),
      prepareResume: (id: string) => prepareTarget('resume', address, id),
      resume: (prepared: PreparedOperation<'resume'>, options?: CallOptions) => submitTarget('resume', address, prepared, options, client.agents.resume, launchResult(scope)),
      prepareFork: (id: string) => prepareTarget('fork', address, id),
      fork: (prepared: PreparedOperation<'fork'>, options?: CallOptions) => submitTarget('fork', address, prepared, options, client.agents.fork, launchResult(scope)),
    };
  }

  if (ownReference.kind === 'stopped') {
    return Object.freeze({ kind: 'stopped', ...reader(ownReference) });
  }
  const current = reader(ownReference);
  return Object.freeze({
    kind: 'current',
    ...current,
    prepareMessage(message: ReadonlyValue<MessageInput>, id: string) {
      const input = messageInput(current.address, message, id);
      // Check and detach the retainable payload with the existing schema codec.
      const retained = retainInput('SubmitMessage', input);
      return freezeInput({
        target: current.address,
        operation: { scope, requestId: retained.request_id },
        message: retained.message,
      });
    },
    async send(prepared: PreparedMessage, options?: CallOptions) {
      requireAgent(prepared.target, current.address);
      requireScope(prepared.operation.scope, scope);
      const id = prepared.operation.requestId;
      return freezeInput(await client.agents.messages.send(messageInput(current.address, prepared.message, id), options));
    },
    prepareKill: (id: string) => prepareTarget('kill', current.address, id),
    kill: (prepared: PreparedOperation<'kill'>, options?: CallOptions) => submitTarget('kill', current.address, prepared, options, client.agents.kill, killResult(current.address)),
    prepareStopParticipation: (id: string) => prepareTarget('stop-participation', current.address, id),
    stopParticipation: (prepared: PreparedOperation<'stop-participation'>, options?: CallOptions) => submitTarget('stop-participation', current.address, prepared, options, client.agents.stopParticipation, stopParticipationResult(current.address)),
    labels: async (options?: CallOptions) => projectLabels(current.address, await client.agents.labels.get(exactInput, options)),
    prepareLabelsPatch(patch: LabelsPatch, id: string) {
      const prepared = prepareTarget('patch-labels', current.address, id);
      const { set, remove, if_revision } = retainInput('SubmitLabels', labelsInput({ ...exactInput, request_id: prepared.operation.requestId }, patch));
      return freezeInput({ ...prepared, patch: { set, remove, if_revision } });
    },
    async patchLabels(prepared: PreparedOperation<'patch-labels'>, options?: CallOptions) {
      if (prepared.kind !== 'patch-labels') throw new TypeError('Expected a prepared labels patch');
      const input = targetInput(prepared, current.address);
      return checkedReceipt('patch-labels', operationAddress(scope, input.request_id), await client.agents.labels.patch(labelsInput(input, prepared.patch), options), labelsResult(current.address));
    },
  });
}
