/** @aiPurpose Expose complete operator Work workflows over the native finite Workbench HTTP contract. */
import { createClient } from './generated/workbench/client/index.js';
import * as wire from './generated/workbench/sdk.gen.js';
import type { SchemaTypes } from './generated/workbench/schema-types.gen.js';
import { parseWorkbenchJson, stringifyWorkbenchJson, workbenchWireValue } from './codec.js';
import type { Native } from './codec.js';
import { AdapterError, freezeInput, responseText, decodeResponse } from './transport.js';
import type { HttpOptions, CallOptions } from './transport.js';

/** @aiPurpose Supply native SDK values derived from actual marker-coded Workbench wire schemas. @aiUsage Integers are bigint, while identities and closed native variants retain their original meaning. */
export type WorkbenchTypes = { [K in keyof SchemaTypes]: Native<SchemaTypes[K]> };
type W = WorkbenchTypes;
type Read = W['Read'];
type Command = W['OperatorCommand'];
type Payload<T, K extends string> = Extract<T, Record<K, unknown>>[K];
type WriteContext = Omit<W['ApplyCall'], 'command'>;

/** @aiPurpose Retain original Work command intent after loss of its reply. @aiUsage Lookup input.command_id with input.generation; never rebuild a new command from current revision. */
export class WorkOutcomeUnknown extends Error {
  readonly code = 'OutcomeUnknown';
  constructor(readonly input: Readonly<W['ApplyCall']>, cause: unknown) {
    super('Work command outcome unknown; query original generation/command_id', { cause }); this.name = 'WorkOutcomeUnknown';
  }
}
/** @aiPurpose Expose closed operator calls and intentional work methods. @aiUsage Native Rejected remains a value, distinct from HTTP denial; no AgentReport method or actor impersonation exists. */
export interface WorkbenchClient {
  describe(options?: CallOptions): Promise<W['Descriptor']>;
  read(input: W['ReadCall'], options?: CallOptions): Promise<W['Response']>;
  apply(input: W['ApplyCall'], options?: CallOptions): Promise<W['Response']>;
  works: {
    list(generation: string, input: Payload<Read, 'List'>, options?: CallOptions): Promise<W['Response']>;
    get(generation: string, input: Payload<Read, 'Detail'>, options?: CallOptions): Promise<W['Response']>;
    candidates(generation: string, input: Payload<Read, 'Candidates'>, options?: CallOptions): Promise<W['Response']>;
    command(generation: string, command_id: string, options?: CallOptions): Promise<W['Response']>;
    thread(generation: string, input: Payload<Read, 'Thread'>, options?: CallOptions): Promise<W['Response']>;
    create(context: WriteContext, input: Payload<Command, 'CreateWork'>, options?: CallOptions): Promise<W['Response']>;
    updateBrief(context: WriteContext, input: Payload<Command, 'UpdateBrief'>, options?: CallOptions): Promise<W['Response']>;
    bind(context: WriteContext, input: Payload<Command, 'BindParticipant'>, options?: CallOptions): Promise<W['Response']>;
    dispatch(context: WriteContext, input: Payload<Command, 'DispatchAssignment'>, options?: CallOptions): Promise<W['Response']>;
    note(context: WriteContext, input: Payload<Command, 'SendNote'>, options?: CallOptions): Promise<W['Response']>;
    answer(context: WriteContext, input: Payload<Command, 'AnswerQuestion'>, options?: CallOptions): Promise<W['Response']>;
    requestCheck(context: WriteContext, input: Payload<Command, 'RequestCheck'>, options?: CallOptions): Promise<W['Response']>;
    accept(context: WriteContext, input: Payload<Command, 'AcceptResult'>, options?: CallOptions): Promise<W['Response']>;
    requestChanges(context: WriteContext, input: Payload<Command, 'RequestChanges'>, options?: CallOptions): Promise<W['Response']>;
  };
}
/** @aiPurpose Connect one explicit Workbench listener with exact marker codecs. @aiUsage Retain generation, command_id, expected_revision and full ApplyCall before applying; no retries or descriptor rebase occur. */
export function createWorkbenchClient(options: HttpOptions): WorkbenchClient {
  const client = createClient({ baseUrl: options.baseUrl.replace(/\/$/, ''), fetch: options.fetch, headers: options.headers, parseAs: 'text', credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer' });
  const api: WorkbenchClient = {
    async describe(opts) { const text = await responseText(wire.workbenchConnection({ client, body: {}, signal: opts?.signal })); return decodeResponse('Descriptor', text, raw => parseWorkbenchJson('Descriptor', raw)); },
    async read(input, opts) {
      const body = stringifyWorkbenchJson('ReadCall', input);
      const text = await responseText(wire.workbenchRead({ client, bodySerializer: () => body, body: workbenchWireValue('ReadCall', input), signal: opts?.signal }));
      return decodeResponse('Response', text, raw => parseWorkbenchJson('Response', raw));
    },
    async apply(input, opts) {
      const body = stringifyWorkbenchJson('ApplyCall', input);
      const retained = freezeInput(parseWorkbenchJson('ApplyCall', body));
      if (opts?.signal?.aborted) throw opts.signal.reason;
      try {
        const text = await responseText(wire.workbenchApply({ client, bodySerializer: () => body, body: workbenchWireValue('ApplyCall', retained), signal: opts?.signal }));
        const result = decodeResponse('Response', text, raw => parseWorkbenchJson('Response', raw));
        if ('Applied' in result) {
          const reference = result.Applied.reference;
          if (result.Applied.command_id !== retained.command_id || reference.generation !== retained.generation || retained.work && (reference.id !== retained.work.id || reference.scope !== retained.work.scope || reference.project !== retained.work.project)) throw new WorkOutcomeUnknown(retained, result);
        }
        if ('Rejected' in result && typeof result.Rejected === 'object' && 'OutcomeUnknown' in result.Rejected) throw new WorkOutcomeUnknown(retained, result);
        return result;
      } catch (error) {
        if (error instanceof AdapterError || error instanceof WorkOutcomeUnknown) throw error;
        throw new WorkOutcomeUnknown(retained, error);
      }
    },
    works: {
      list: (generation, input, opts) => api.read({ generation, read: { List: input } }, opts),
      get: (generation, input, opts) => api.read({ generation, read: { Detail: input } }, opts),
      candidates: (generation, input, opts) => api.read({ generation, read: { Candidates: input } }, opts),
      command: (generation, command_id, opts) => api.read({ generation, read: { Command: { command_id } } }, opts),
      thread: (generation, input, opts) => api.read({ generation, read: { Thread: input } }, opts),
      create: (context, input, opts) => api.apply({ ...context, command: { CreateWork: input } }, opts),
      updateBrief: (context, input, opts) => api.apply({ ...context, command: { UpdateBrief: input } }, opts),
      bind: (context, input, opts) => api.apply({ ...context, command: { BindParticipant: input } }, opts),
      dispatch: (context, input, opts) => api.apply({ ...context, command: { DispatchAssignment: input } }, opts),
      note: (context, input, opts) => api.apply({ ...context, command: { SendNote: input } }, opts),
      answer: (context, input, opts) => api.apply({ ...context, command: { AnswerQuestion: input } }, opts),
      requestCheck: (context, input, opts) => api.apply({ ...context, command: { RequestCheck: input } }, opts),
      accept: (context, input, opts) => api.apply({ ...context, command: { AcceptResult: input } }, opts),
      requestChanges: (context, input, opts) => api.apply({ ...context, command: { RequestChanges: input } }, opts),
    },
  };
  return api;
}

/** @aiPurpose Convert only a retained current management identity to the collaboration identity representation. @aiUsage This grants no membership/authority; selected native Workbench still validates its scope and exact participant. Historical stopped refs are refused. */
export function participantFromAgent(agent: import('./generated/management/types.gen.js').AgentRef): W['ParticipantRef'] {
  if (agent.stopped_event !== undefined && agent.stopped_event !== null || !agent.incarnation || !/^(0|[1-9][0-9]*)$/.test(agent.incarnation)) throw new TypeError('A current exact decimal incarnation is required; stopped references cannot become participants');
  const incarnation_bits = BigInt(agent.incarnation);
  if (incarnation_bits > 18446744073709551615n) throw new RangeError('Incarnation outside u64');
  return { canonical_name: agent.name, incarnation_bits };
}
