/** Address invariants shared by scope, cursor, exact Agent and Operation construction. */
import type { AgentRef } from '../generated/management/types.gen.js';
import type { AgentAddress, AgentReference, CurrentReference, ScopeAddress, WorkspaceAddress } from './contracts.js';

type ReferenceFailure = 'InvalidReference' | 'ScopeMismatch' | 'ReferenceMismatch' | 'InconsistentResponse' | 'UnsupportedCapability';
/** A local address or correspondence failure. Native refusals still use ManagementError. */
export class DomainReferenceError extends Error {
  constructor(readonly code: ReferenceFailure, message: string) {
    super(message);
    this.name = 'DomainReferenceError';
  }
}

/** A response was observed for a different database generation than the selected Scope. */
export class DomainGenerationMismatch extends DomainReferenceError {
  constructor() {
    super('InconsistentResponse', 'Response does not match the selected generation');
    this.name = 'DomainGenerationMismatch';
  }
}

export function normalizeListener(baseUrl: string): string {
  const url = new URL(baseUrl);
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) {
    throw new TypeError('Select an HTTP(S) listener base URL without credentials, query or fragment');
  }
  return url.href.replace(/\/+$/, '');
}

export function requireScope(actual: ScopeAddress, expected: ScopeAddress): void {
  if (actual.baseUrl !== expected.baseUrl || actual.scope !== expected.scope || actual.generation !== expected.generation) {
    throw new DomainReferenceError('ScopeMismatch', 'Address belongs to a different listener, scope or generation');
  }
}

export function requireGeneration(generation: string, scope: ScopeAddress): void {
  if (generation !== scope.generation) {
    throw new DomainGenerationMismatch();
  }
}

export function requireAgent(actual: AgentAddress, expected: AgentAddress): void {
  requireScope(actual.scope, expected.scope);
  const a = actual.reference;
  const b = expected.reference;
  if (a.name !== b.name || (a.kind === 'current'
    ? b.kind !== 'current' || a.incarnation !== b.incarnation
    : a.kind !== 'stopped' || b.kind !== 'stopped' || a.stopped_event !== b.stopped_event)) {
    throw new DomainReferenceError('ReferenceMismatch', 'Address belongs to a different exact Agent');
  }
}

export function requireWorkspace(actual: WorkspaceAddress, expected: WorkspaceAddress): void {
  requireScope(actual.scope, expected.scope);
  if (actual.key !== expected.key) {
    throw new DomainReferenceError('ReferenceMismatch', 'Cursor belongs to a different Workspace projection');
  }
}

export function requestId(value: string): string {
  if (typeof value !== 'string' || !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)) {
    throw new DomainReferenceError('InvalidReference', 'Supply a UUID in dashed form as the original request ID');
  }
  return value;
}

function validName(value: string): boolean {
  return typeof value === 'string' && /^[a-z0-9_-]{1,128}$/i.test(value);
}

function isCurrent(value: unknown): value is string {
  return typeof value === 'string' && /^(0|[1-9][0-9]{0,19})$/.test(value) && (value.length < 20 || value <= '18446744073709551615');
}

function isStopped(value: unknown): value is string {
  return typeof value === 'string' && /^[1-9][0-9]{0,18}$/.test(value) && (value.length < 19 || value <= '9223372036854775807');
}

export function readCurrentReference(ref: AgentRef): CurrentReference {
  const reference = readReference(ref);
  if (reference.kind !== 'current') throw new DomainReferenceError('InconsistentResponse', 'Expected an exact current Agent reference');
  return reference;
}

export function readReference(ref: AgentRef): AgentReference {
  if (validName(ref.name)) {
    if (isCurrent(ref.incarnation) && ref.stopped_event == null) {
      return Object.freeze({ kind: 'current', name: ref.name, incarnation: ref.incarnation });
    }
    if (isStopped(ref.stopped_event) && ref.incarnation == null) {
      return Object.freeze({ kind: 'stopped', name: ref.name, stopped_event: ref.stopped_event });
    }
  }
  throw new DomainReferenceError('InconsistentResponse', 'Management returned a nonexact Agent reference');
}

export function wireReference(reference: AgentReference): AgentRef {
  if (validName(reference.name)) {
    if (reference.kind === 'current' && isCurrent(reference.incarnation) && !('stopped_event' in reference)) {
      return { name: reference.name, incarnation: reference.incarnation };
    }
    if (reference.kind === 'stopped' && isStopped(reference.stopped_event) && !('incarnation' in reference)) {
      return { name: reference.name, stopped_event: reference.stopped_event };
    }
  }
  throw new DomainReferenceError('InvalidReference', 'Supply an exact current incarnation or stopped event');
}
