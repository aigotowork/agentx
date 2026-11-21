/** Original operation recovery and correspondence checks over the Management journal. */
import { ReceiptError, waitForReceipt } from '../client.js';
import { parseManagementJson, stringifyManagementJson } from '../codec.js';
import type { AgentRef, LabelsSnapshot, Operation, SubmitTarget } from '../generated/management/types.gen.js';
import type { ManagementClient } from '../management.js';
import type { ManagementTypes } from '../management.js';
import { freezeInput } from '../transport.js';
import type { CallOptions } from '../transport.js';
import { DomainReferenceError, readCurrentReference, readReference, requestId, requireAgent, requireGeneration, requireScope, wireReference } from './address.js';
import type { AgentAddress, AgentLabels, AgentReference, CurrentReference, LaunchResult, OperationAddress, OperationKind, OperationResults, PreparedMessage, PreparedOperation, ScopeAddress, Transcript, TypedOperationHandle, TypedOperationReceipt } from './contracts.js';

export function retainInput<K extends keyof ManagementTypes>(schema: K, value: ManagementTypes[K]): ManagementTypes[K] {
  return freezeInput(parseManagementJson(schema, stringifyManagementJson(schema, value)));
}

export function operationAddress(scope: ScopeAddress, id: string): OperationAddress {
  return Object.freeze({ scope, requestId: requestId(id) });
}

export function commandIdentity(address: OperationAddress, scope: ScopeAddress) {
  requireScope(address.scope, scope);
  return { generation: scope.generation, request_id: requestId(address.requestId) };
}

export function targetInput(prepared: { operation: OperationAddress; target: AgentAddress }, target: AgentAddress) {
  wireReference(prepared.target.reference);
  requireAgent(prepared.target, target);
  return { ...commandIdentity(prepared.operation, target.scope), agent: wireReference(target.reference) };
}

export function prepareTarget<K extends OperationKind, R extends AgentReference>(kind: K, target: AgentAddress<R>, id: string) {
  return Object.freeze({ kind, operation: operationAddress(target.scope, id), target });
}

function mismatch(receipt: Operation, reason: string): never {
  throw new ReceiptError('ProtocolError', freezeInput(receipt), reason);
}

function matchAgent(receipt: Operation, ref: AgentRef, expected: AgentAddress) {
  try { requireAgent({ scope: expected.scope, reference: readReference(ref) }, expected); }
  catch (cause) {
    if (!(cause instanceof DomainReferenceError)) throw cause;
    mismatch(receipt, 'Receipt does not match the original exact Agent');
  }
}

type ResultProjection<K extends OperationKind> = (receipt: Operation) => OperationResults[K];

export async function submitTarget<K extends OperationKind>(kind: K, target: AgentAddress, prepared: { kind: K; operation: OperationAddress; target: AgentAddress }, options: CallOptions | undefined, submit: (input: SubmitTarget, opts?: CallOptions) => Promise<Operation>, project: ResultProjection<K>) {
  if (prepared.kind !== kind) throw new TypeError(`Expected prepared ${kind} request`);
  const input = targetInput(prepared, target);
  const original = operationAddress(target.scope, input.request_id);
  return checkedReceipt(kind, original, await submit(input, options), project);
}

export function transcriptResult(target: AgentAddress): ResultProjection<'transcript'> {
  return receipt => {
    const { result } = receipt;
    if (!result || !('Transcript' in result)) mismatch(receipt, 'Succeeded Transcript request requires a Transcript result');
    const { agent, available, reason, exchanges } = result.Transcript;
    matchAgent(receipt, agent, target);
    const base = { agent: target, reason: reason ?? null };
    if (available) return { ...base, available: true, exchanges } satisfies Transcript;
    if (exchanges.length) mismatch(receipt, 'Unavailable Transcript cannot contain exchanges');
    return { ...base, available: false, exchanges: [] } satisfies Transcript;
  };
}

function sendResult(target: AgentAddress): ResultProjection<'send'> {
  return receipt => {
    const { result } = receipt;
    if (!result || !('Sent' in result) || result.Sent.recipients.length !== 1 || result.Sent.recipients[0] !== target.reference.name) {
      mismatch(receipt, 'Succeeded send requires a matching recipient in the Sent result');
    }
    return result.Sent;
  };
}

function stoppedResult(receipt: Operation, target: AgentAddress) {
  const { result } = receipt;
  if (!result || !('Stopped' in result) || result.Stopped.name !== target.reference.name) mismatch(receipt, 'Stopped result does not match the original target');
  return result.Stopped;
}

export const killResult = (target: AgentAddress): ResultProjection<'kill'> => receipt => {
  const { name, outcome } = stoppedResult(receipt, target);
  if (outcome !== 'sent' && outcome !== 'already_dead') mismatch(receipt, 'Incompatible kill result');
  return { name, outcome };
};

export const stopParticipationResult = (target: AgentAddress): ResultProjection<'stop-participation'> => receipt => {
  const { name, outcome } = stoppedResult(receipt, target);
  if (outcome !== 'participation_retained' && outcome !== 'participation_stopped') mismatch(receipt, 'Incompatible stop participation result');
  return { name, outcome };
};

export const launchResult = (scope: ScopeAddress) => (receipt: Operation): LaunchResult => {
  const { result } = receipt;
  if (!result || !('Launched' in result)) mismatch(receipt, 'Succeeded launch, resume or fork requires a Launched result');
  const { agents, failed, errors, agent_refs } = result.Launched;
  if (agent_refs == null) return { agents, failed, errors, agentAddresses: null };
  if (agent_refs.length === 0 || agent_refs.length !== agents.length) mismatch(receipt, 'Exact launch references do not match the retained launch results');
  const agentAddresses = agent_refs.map((ref, index) => {
    if (ref.name !== agents[index]) mismatch(receipt, 'Exact launch reference does not match the retained launch name');
    try { return currentAddress(scope, ref); }
    catch (cause) {
      if (!(cause instanceof DomainReferenceError)) throw cause;
      mismatch(receipt, cause.message);
    }
  });
  return { agents, failed, errors, agentAddresses };
};

function currentAddress(scope: ScopeAddress, ref: AgentRef): AgentAddress<CurrentReference> {
  return { scope, reference: readCurrentReference(ref) };
}

export function projectLabels(target: AgentAddress, snapshot: LabelsSnapshot): AgentLabels {
  requireGeneration(snapshot.generation, target.scope);
  const agent = currentAddress(target.scope, snapshot.agent);
  try { requireAgent(agent, target); }
  catch (cause) {
    if (!(cause instanceof DomainReferenceError)) throw cause;
    throw new DomainReferenceError('InconsistentResponse', 'Labels snapshot does not match the original exact Agent');
  }
  return freezeInput({ agent, labels: snapshot.labels, revision: snapshot.revision });
}

export const labelsResult = (target: AgentAddress): ResultProjection<'patch-labels'> => receipt => {
  const { result } = receipt;
  if (!result || !('LabelsPatched' in result)) mismatch(receipt, 'Succeeded labels patch requires a LabelsPatched result');
  try { return { changed: result.LabelsPatched.changed, snapshot: projectLabels(target, result.LabelsPatched.snapshot) }; }
  catch (cause) {
    if (!(cause instanceof DomainReferenceError)) throw cause;
    mismatch(receipt, cause.message);
  }
};

export const directorySendResult = (scope: ScopeAddress): ResultProjection<'directory-send'> => receipt => {
  const { result } = receipt;
  if (!result || !('DirectorySent' in result)) mismatch(receipt, 'Succeeded directory send requires a DirectorySent result');
  const { entry, recipient, sent, created } = result.DirectorySent;
  if (entry.generation !== scope.generation || entry.endpoint.status !== 'bound' || !entry.endpoint.participant || sent.recipients.length !== 1 || sent.recipients[0] !== recipient.name) {
    mismatch(receipt, 'Inconsistent accepted directory route');
  }
  let target: AgentAddress<CurrentReference>;
  try { target = currentAddress(scope, recipient); }
  catch (cause) {
    if (!(cause instanceof DomainReferenceError)) throw cause;
    mismatch(receipt, cause.message);
  }
  matchAgent(receipt, entry.endpoint.participant, target);
  return { entry, recipient: target, sent, created };
};

export function checkedReceipt<K extends OperationKind>(kind: K, address: OperationAddress, receipt: Operation, project: ResultProjection<K>): TypedOperationReceipt<K> {
  if (receipt.generation !== address.scope.generation || receipt.request_id !== address.requestId) mismatch(receipt, 'Receipt identity does not match the original request');
  const { status, result, error } = receipt;
  const base = { kind, address, receipt };
  if (status === 'Succeeded') {
    if (!result || Object.keys(result).length !== 1 || error != null) mismatch(receipt, 'Succeeded receipt requires one result and no error');
    return freezeInput({ ...base, status, result: project(receipt), error: null });
  }
  if (result != null) mismatch(receipt, 'Only a Succeeded receipt may contain a result');
  if (status === 'Accepted' || status === 'Running') {
    if (error != null) mismatch(receipt, 'Accepted and Running receipts cannot contain an error');
    return freezeInput({ ...base, status, result: null, error: null });
  }
  return freezeInput({ ...base, status, result: null, error: error ?? null });
}

function bindTyped<K extends OperationKind>(client: ManagementClient, scope: ScopeAddress, kind: K, operation: OperationAddress, project: ResultProjection<K>): TypedOperationHandle<K> {
  const original = commandIdentity(operation, scope);
  const address = operationAddress(scope, original.request_id);
  const decode = (receipt: Operation) => checkedReceipt(kind, address, receipt, project);
  return Object.freeze({
    address,
    observe: async (opts?: Parameters<typeof client.getOperation>[1]) => decode(await client.getOperation(original, opts)),
    wait: (opts?: Parameters<typeof client.waitForOperation>[1]) => waitForReceipt(original, opts, call => client.getOperation(original, call), decode),
  });
}

function checkedTarget(scope: ScopeAddress, prepared: { target: AgentAddress }, currentOnly = false): AgentAddress {
  requireScope(prepared.target.scope, scope);
  const reference = readReference(wireReference(prepared.target.reference));
  if (currentOnly && reference.kind !== 'current') throw new DomainReferenceError('InvalidReference', 'Operation requires an exact current Agent');
  return Object.freeze({ scope, reference });
}

export function bindPrepared(client: ManagementClient, scope: ScopeAddress, prepared: PreparedOperation | PreparedMessage) {
  const { operation } = prepared;
  if (!('kind' in prepared) || prepared.kind === 'send') return bindTyped(client, scope, 'send', operation, sendResult(checkedTarget(scope, prepared, true)));
  switch (prepared.kind) {
    case 'transcript': return bindTyped(client, scope, 'transcript', operation, transcriptResult(checkedTarget(scope, prepared)));
    case 'kill': return bindTyped(client, scope, 'kill', operation, killResult(checkedTarget(scope, prepared, true)));
    case 'stop-participation': return bindTyped(client, scope, 'stop-participation', operation, stopParticipationResult(checkedTarget(scope, prepared, true)));
    case 'patch-labels': return bindTyped(client, scope, 'patch-labels', operation, labelsResult(checkedTarget(scope, prepared, true)));
    case 'resume': case 'fork':
      checkedTarget(scope, prepared);
      return bindTyped(client, scope, prepared.kind, operation, launchResult(scope));
    case 'launch': return bindTyped(client, scope, 'launch', operation, launchResult(scope));
    case 'directory-send': return bindTyped(client, scope, 'directory-send', operation, directorySendResult(scope));
    default: throw new TypeError('Unknown prepared operation kind');
  }
}
