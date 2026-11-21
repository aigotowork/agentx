/** @aiPurpose Compose the native generated communication routes at one authenticated cloud scope. */
import { createClient } from './generated/cloud-scope/client/index.js';
import * as wire from './generated/cloud-scope/sdk.gen.js';
import type { SchemaTypes } from './generated/cloud-scope/schema-types.gen.js';
import type * as W from './generated/cloud-scope/types.gen.js';
import { wireDocument } from './generated/cloud-scope/wire-schema.gen.js';
import { createSchemaCodec } from './schema-codec.js';
import { cloudFetchConfig, cloudResponse, CloudHttpError, isCloudRejection } from './cloud-transport.js';
import type { CloudClientOptions, CloudWireResult } from './cloud-transport.js';
import type { CallOptions } from './transport.js';

export { CloudHttpError as CloudScopeError };
export type CloudScopeOptions = CloudClientOptions;
export type CloudScopeTypes = SchemaTypes;
export type CloudScopeClient = ReturnType<typeof createCloudScopeClient>;
type Query<T extends { query?: unknown }> = NonNullable<T['query']>;
type RequestSchema = 'HostRegistration' | 'AgentRegistration' | 'SendMessage' | 'ClaimInbox' | 'ConfirmInbox' | 'RecoverInbox';
type Mutation = 'renewHost' | 'registerAgent' | 'retireAgent' | 'send' | 'claimInbox' | 'confirmInbox' | 'recoverInbox';
const codec = createSchemaCodec(wireDocument);

/** @aiPurpose Mark ambiguous scope mutations without disclosing caller payloads. @aiUsage Keep the original request or claim ID and generation; reconcile with getOperation or recoverInbox as applicable. */
export class CloudScopeOutcomeUnknown extends Error {
  readonly code = 'OutcomeUnknown';
  constructor(readonly operation: Mutation) {
    super('Cloud mutation outcome is unknown; reconcile using the original request and generation');
    this.name = 'CloudScopeOutcomeUnknown';
  }
}

function requirePathSegment(value: string): void {
  if (!value || value === '.' || value === '..') throw new TypeError('Expected a nonempty identifier usable as a URL path segment');
}

/** @aiPurpose Bind the scope communication API to its own origin and bearer. @aiUsage Accepts a ScopeReceipt scope_url or the full scope API base. Pass the original generation and exact Agent identity for each write; the client never enrolls, refreshes generation or retries on its own. */
export function createCloudScopeClient(options: CloudScopeOptions) {
  const config = cloudFetchConfig(options, 'scope');
  const client = createClient(config);
  const scopeId = config.baseUrl.split('/').at(-3);
  const secrets = [options.token];
  const read = <R extends keyof SchemaTypes>(
    schema: R, pending: Promise<CloudWireResult>, check?: (value: SchemaTypes[R]) => boolean,
  ) => cloudResponse(pending, text => {
    const value = codec.parse(schema, text) as SchemaTypes[R];
    if (check && !check(value)) throw new TypeError('Response identity does not match the caller request');
    return value;
  }, secrets);
  async function write<K extends RequestSchema, R extends keyof SchemaTypes>(
    operation: Mutation, inputSchema: K, input: SchemaTypes[K], outputSchema: R,
    opts: CallOptions | undefined,
    run: (bodySerializer: () => string) => Promise<CloudWireResult>,
    check?: (value: SchemaTypes[R]) => boolean,
  ): Promise<SchemaTypes[R]> {
    opts?.signal?.throwIfAborted();
    const body = codec.stringify(inputSchema, input);
    try { return await read(outputSchema, run(() => body), check); }
    catch (error) {
      if (isCloudRejection(error)) throw error;
      throw new CloudScopeOutcomeUnknown(operation);
    }
  }
  return {
    describe: (opts?: CallOptions) => read('Descriptor', wire.scopeDescribe({ client, signal: opts?.signal }), result => result.scope_id === scopeId),
    listHosts: (query?: Query<W.ScopeListHostsData>, opts?: CallOptions) =>
      read('HostPage', wire.scopeListHosts({ client, query, signal: opts?.signal })),
    listAgents: (query?: Query<W.ScopeListAgentsData>, opts?: CallOptions) =>
      read('AgentPage', wire.scopeListAgents({ client, query, signal: opts?.signal })),
    renewHost(host: string, body: SchemaTypes['HostRegistration'], opts?: CallOptions) {
      requirePathSegment(host);
      return write('renewHost', 'HostRegistration', body, 'HostView', opts,
        bodySerializer => wire.scopeRenewHost({ client, path: { host }, body, bodySerializer, signal: opts?.signal }),
        result => result.name === host);
    },
    registerAgent(name: string, body: SchemaTypes['AgentRegistration'], opts?: CallOptions) {
      requirePathSegment(name);
      const { host, incarnation } = body;
      return write('registerAgent', 'AgentRegistration', body, 'AgentView', opts,
        bodySerializer => wire.scopeRegisterAgent({ client, path: { name }, body, bodySerializer, signal: opts?.signal }),
        result => result.name === name && result.host === host && result.incarnation === incarnation);
    },
    retireAgent(name: string, body: SchemaTypes['AgentRegistration'], opts?: CallOptions) {
      requirePathSegment(name);
      const { host, incarnation } = body;
      return write('retireAgent', 'AgentRegistration', body, 'AgentView', opts,
        bodySerializer => wire.scopeRetireAgent({ client, path: { name }, body, bodySerializer, signal: opts?.signal }),
        result => result.name === name && result.host === host && result.incarnation === incarnation);
    },
    send(body: SchemaTypes['SendMessage'], opts?: CallOptions) {
      requirePathSegment(body.request_id);
      return write('send', 'SendMessage', body, 'MessageView', opts,
        bodySerializer => wire.scopeSendMessage({ client, body, bodySerializer, signal: opts?.signal }));
    },
    getOperation(request_id: string, opts?: CallOptions) {
      requirePathSegment(request_id);
      return read('MessageOperation', wire.scopeGetOperation({ client, path: { request_id }, signal: opts?.signal }),
        result => result.request_id === request_id);
    },
    claimInbox(body: SchemaTypes['ClaimInbox'], opts?: CallOptions) {
      const { claim_id } = body;
      return write('claimInbox', 'ClaimInbox', body, 'InboxBatch', opts,
        bodySerializer => wire.scopeClaimInbox({ client, body, bodySerializer, signal: opts?.signal }),
        result => result.claim_id === claim_id);
    },
    confirmInbox(body: SchemaTypes['ConfirmInbox'], opts?: CallOptions) {
      const ids = [...body.message_ids];
      return write('confirmInbox', 'ConfirmInbox', body, 'InboxConfirmation', opts,
        bodySerializer => wire.scopeConfirmInbox({ client, body, bodySerializer, signal: opts?.signal }),
        result => result.confirmed.length === ids.length && result.confirmed.every((id, index) => id === ids[index]));
    },
    recoverInbox(body: SchemaTypes['RecoverInbox'], opts?: CallOptions) {
      const { claim_id } = body;
      return write('recoverInbox', 'RecoverInbox', body, 'InboxBatch', opts,
        bodySerializer => wire.scopeRecoverInbox({ client, body, bodySerializer, signal: opts?.signal }),
        result => result.claim_id === claim_id);
    },
  };
}
