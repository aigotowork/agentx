/** @aiPurpose Provide named public management calls and preserve caller knowledge across transport uncertainty. */
import { createClient } from './generated/management/client/index.js';
import * as wire from './generated/management/sdk.gen.js';
import type { AgentRef, Descriptor, AgentPage, AgentView, Operation, SubmitMessage, SendOutcome, PublicError, ManagementListAgentsData } from './generated/management/types.gen.js';
import type { SchemaTypes } from './generated/management/schema-types.gen.js';
import { decodeResponse } from './transport.js';
import { parseManagementJson, stringifyManagementJson, stringifyJson } from './codec.js';

/** @aiPurpose Retain a structured server refusal. @aiUsage Branch on code; status and detail are diagnostics, not message parsing APIs. */
export class ManagementError extends Error {
  readonly code: PublicError['code'];
  constructor(readonly failure: PublicError, readonly status: number) {
    super(failure.message); this.name = 'ManagementError'; this.code = failure.code;
  }
}
/** @aiPurpose Preserve submission identity when admission or completion cannot be learned. @aiUsage Query input.generation/input.request_id; never silently replace the request or resend with a new ID. */
export class OutcomeUnknown extends Error {
  readonly code = 'OutcomeUnknown';
  constructor(readonly input: Readonly<SubmitMessage>, cause: unknown) {
    super('Message submission outcome is unknown; query the original generation/request_id', { cause }); this.name = 'OutcomeUnknown';
  }
}
/** @aiPurpose Retain an unexpected or inconsistent receipt. @aiUsage Inspect operation and reconcile; never interpret this failure as confirmed nonexecution. */
export class ReceiptError extends Error {
  constructor(readonly code: 'ProtocolError' | 'Failed' | 'OutcomeUnknown', readonly operation: Operation, message: string) {
    super(message); this.name = 'ReceiptError';
  }
}
/** @aiPurpose Represent observation ending without canceling server work. @aiUsage Keep identity and lastOperation for later observation. */
export class ObservationStopped extends Error {
  constructor(readonly code: 'Canceled' | 'DeadlineExceeded', readonly identity: OperationIdentity, readonly lastOperation?: Operation) {
    super(`${code}: local receipt observation stopped; server work is unchanged`); this.name = 'ObservationStopped';
  }
}
/** @aiPurpose Select the listener and optional transport. @aiUsage Supply an explicit management base URL; SDK imports need no Node environment. */
export interface ManagementClientOptions { baseUrl: string; fetch?: typeof fetch; headers?: HeadersInit }
/** @aiPurpose Address an original durable receipt. @aiUsage Retain both fields before dispatch and across reloads. */
export type OperationIdentity = Pick<Operation, 'generation' | 'request_id'>;
/** @aiPurpose Control local observation lifetime. @aiUsage timeoutMs defaults to 30000 and pollIntervalMs to 250; AbortSignal stops only local observation. */
export interface WaitOptions { signal?: AbortSignal; timeoutMs?: number; pollIntervalMs?: number }
/** @aiPurpose Supply native list filters. @aiUsage Generation comes from describe, after from the original page; label arrays are AND and require current_only. */
export type ListAgentsInput = NonNullable<ManagementListAgentsData['query']>;
/** @aiPurpose Observe five management operations and a checked send result. @aiUsage Persist SubmitMessage before send; helper cancellation never requests native cancellation. */
export interface ManagementClient {
  describe(options?: { signal?: AbortSignal }): Promise<Descriptor>;
  listAgents(input: ListAgentsInput, options?: { signal?: AbortSignal }): Promise<AgentPage>;
  getAgent(input: { generation: string; agent: AgentRef }, options?: { signal?: AbortSignal }): Promise<AgentView>;
  send(input: SubmitMessage, options?: { signal?: AbortSignal }): Promise<Operation>;
  getOperation(input: OperationIdentity, options?: { signal?: AbortSignal }): Promise<Operation>;
  waitForOperation(input: OperationIdentity, options?: WaitOptions): Promise<Operation>;
  waitForSent(input: OperationIdentity, options?: WaitOptions): Promise<SendOutcome>;
}

type WireResult = { data?: unknown; error?: unknown; response?: Response };
export async function decode<K extends keyof SchemaTypes>(pending: Promise<WireResult>, schema: K): Promise<SchemaTypes[K]> {
  const result = await pending;
  if (result.error !== undefined || !result.response?.ok) {
    if (result.response && !result.response.ok) {
      const envelope = parseManagementJson('ErrorEnvelope', stringifyJson(result.error));
      throw new ManagementError(envelope.error, result.response.status);
    }
    throw result.error instanceof Error ? result.error : new Error('Management transport failed', { cause: result.error });
  }
  if (typeof result.data !== 'string') throw new TypeError('Expected unparsed management response text');
  return decodeResponse(schema, result.data, text => parseManagementJson(schema, text));
}
export function checkIdentity(operation: Operation, input: OperationIdentity): Operation {
  if (operation.generation !== input.generation || operation.request_id !== input.request_id) {
    throw new ReceiptError('ProtocolError', operation, 'Receipt generation/request_id does not match the retained request');
  }
  return operation;
}
function freezeInput<T>(value: T): Readonly<T> {
  if (value && typeof value === 'object') { Object.freeze(value); for (const item of Object.values(value)) freezeInput(item); }
  return value;
}
function delay(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const done = () => { clearTimeout(timer); signal.removeEventListener('abort', abort); };
    const abort = () => { done(); reject(signal.reason); };
    const timer = setTimeout(() => { done(); resolve(); }, ms);
    signal.addEventListener('abort', abort, { once: true });
    if (signal.aborted) abort();
  });
}

/** Internal shared polling lifetime; the projection checks each observed receipt. */
export async function waitForReceipt<T>(input: OperationIdentity, opts: WaitOptions = {}, read: (options: { signal: AbortSignal }) => Promise<Operation>, project: (receipt: Operation) => T): Promise<T> {
  const timeout = opts.timeoutMs ?? 30000;
  const interval = opts.pollIntervalMs ?? 250;
  if (!Number.isFinite(timeout) || timeout < 0 || !Number.isFinite(interval) || interval <= 0) throw new TypeError('Invalid observation timeout/poll interval');
  let lastOperation: Operation | undefined;
  const lifetime = new AbortController();
  const stop = () => lifetime.abort('Canceled');
  opts.signal?.addEventListener('abort', stop, { once: true });
  if (opts.signal?.aborted) stop();
  const deadline = setTimeout(() => lifetime.abort('DeadlineExceeded'), timeout);
  try {
    for (;;) {
      if (lifetime.signal.aborted) throw lifetime.signal.reason;
      lastOperation = await read({ signal: lifetime.signal });
      const value = project(lastOperation);
      if (lastOperation.status !== 'Accepted' && lastOperation.status !== 'Running') return value;
      await delay(interval, lifetime.signal);
    }
  } catch (error) {
    if (lifetime.signal.aborted) throw new ObservationStopped(lifetime.signal.reason === 'Canceled' ? 'Canceled' : 'DeadlineExceeded', { ...input }, lastOperation);
    throw error;
  } finally {
    clearTimeout(deadline); opts.signal?.removeEventListener('abort', stop);
  }
}

/** @aiPurpose Connect generated HTTP operations to lossless codecs and receipt recovery. @aiUsage Use one instance per management listener; no implicit describe refresh or mutation retry occurs. */
export function createManagementClient(options: ManagementClientOptions): ManagementClient {
  const endpoint = new URL(options.baseUrl);
  if (!['http:', 'https:'].includes(endpoint.protocol)) throw new TypeError('Management URL must use http or https');
  const client = createClient({ baseUrl: options.baseUrl.replace(/\/$/, ''), fetch: options.fetch, headers: options.headers, parseAs: 'text', credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer' });
  const api: ManagementClient = {
    describe: (opts) => decode(wire.managementDescribe({ client, signal: opts?.signal }), 'Descriptor'),
    listAgents: (input, opts) => decode(wire.managementListAgents({ client, query: input, signal: opts?.signal }), 'AgentPage'),
    getAgent: (input, opts) => decode(wire.managementGetAgent({ client, path: { name: input.agent.name }, query: {
      generation: input.generation, incarnation: input.agent.incarnation ?? undefined, stopped_event: input.agent.stopped_event ?? undefined,
    }, signal: opts?.signal }), 'AgentView'),
    async send(input, opts) {
      // Validate and snapshot before dispatch; caller mutation cannot change recovery identity or payload.
      const body = stringifyManagementJson('SubmitMessage', input);
      const retained = freezeInput(parseManagementJson('SubmitMessage', body));
      if (opts?.signal?.aborted) throw new ObservationStopped('Canceled', retained);
      try {
        const operation = await decode(wire.managementSubmitMessage({ client, path: { name: retained.agent.name }, body: retained,
          bodySerializer: () => body, signal: opts?.signal }), 'Operation');
        return checkIdentity(operation, retained);
      } catch (error) {
        if (error instanceof ManagementError || error instanceof ReceiptError) throw error;
        throw new OutcomeUnknown(retained, error);
      }
    },
    async getOperation(input, opts) {
      return checkIdentity(await decode(wire.managementGetOperation({ client, path: { request_id: input.request_id }, query: { generation: input.generation }, signal: opts?.signal }), 'Operation'), input);
    },
    waitForOperation: (input, opts) => waitForReceipt(input, opts, call => api.getOperation(input, call), receipt => receipt),
    async waitForSent(input, opts) {
      const operation = await api.waitForOperation(input, opts);
      if (operation.status === 'Unknown') throw new ReceiptError('OutcomeUnknown', operation, 'Native operation outcome remains unknown');
      if (operation.status === 'Failed') throw new ReceiptError('Failed', operation, operation.error ?? 'Native message operation failed');
      if (operation.status !== 'Succeeded' || !operation.result || !('Sent' in operation.result) || Object.keys(operation.result).length !== 1 || operation.error !== null && operation.error !== undefined) {
        throw new ReceiptError('ProtocolError', operation, 'Succeeded message operation requires a Sent result and no error');
      }
      return operation.result.Sent;
    },
  };
  return api;
}
