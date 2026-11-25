import { assertCloudIdentifiers } from './cloud-identifiers.js';
/** @aiPurpose Compose the native generated communication routes at one authenticated cloud namespace. */
import { createClient } from './generated/cloud-namespace/client/index.js';
import * as wire from './generated/cloud-namespace/sdk.gen.js';
import type { SchemaTypes } from './generated/cloud-namespace/schema-types.gen.js';
import type * as W from './generated/cloud-namespace/types.gen.js';
import { wireDocument } from './generated/cloud-namespace/wire-schema.gen.js';
import { cloudFetchConfig, cloudResponse, CloudHttpError, createCloudCodec, inventoryGeneration, isCloudRejection } from './cloud-transport.js';
import type { CloudClientOptions, CloudWireResult } from './cloud-transport.js';
import type { CallOptions } from './transport.js';

export { CloudHttpError as CloudNamespaceError };
export type CloudNamespaceOptions = CloudClientOptions;
export type CloudNamespaceTypes = SchemaTypes;
export type CloudNamespaceClient = ReturnType<typeof createCloudNamespaceClient>;
type Query<T extends { query?: unknown }> = NonNullable<T['query']>;
type RequestSchema = 'HostRegistration' | 'AgentRegistration' | 'SendMessage' | 'ClaimInbox' | 'ConfirmInbox' | 'RecoverInbox';
type Mutation = 'renewHost' | 'registerAgent' | 'retireAgent' | 'send' | 'claimInbox' | 'confirmInbox' | 'recoverInbox';
const codec = createCloudCodec(wireDocument);

function sameWorkspace(a: SchemaTypes['AgentWorkspace'], b: SchemaTypes['AgentWorkspace']): boolean {
  return a.kind === b.kind && (a.kind !== 'project' || b.kind === 'project' && a.workspace_id === b.workspace_id);
}

/** @aiPurpose Mark ambiguous namespace mutations without disclosing caller payloads. @aiUsage Keep the original request or claim ID and generation; reconcile with getOperation or recoverInbox as applicable. */
export class CloudNamespaceOutcomeUnknown extends Error {
  readonly code = 'OutcomeUnknown';
  constructor(readonly operation: Mutation) {
    super('Cloud mutation outcome is unknown; reconcile using the original request and generation');
    this.name = 'CloudNamespaceOutcomeUnknown';
  }
}

function requirePathSegment(value: string): void {
  if (!value || value === '.' || value === '..') throw new TypeError('Expected a nonempty identifier usable as a URL path segment');
}

/** @aiPurpose Bind the namespace communication API to its own origin and bearer. @aiUsage Accepts a NamespaceReceipt namespace_url or the full namespace API base. Pass the original generation and exact Agent identity for each write; the client never enrolls, refreshes generation or retries on its own. */
export function createCloudNamespaceClient(options: CloudNamespaceOptions) {
  const config = cloudFetchConfig(options, 'namespace');
  const client = createClient(config);
  const namespaceId = config.baseUrl.split('/').at(-3);
  const secrets = [options.token];
  const read = <R extends keyof SchemaTypes>(
    schema: R, pending: Promise<CloudWireResult>, check?: (value: SchemaTypes[R]) => boolean, resource = true,
  ) => cloudResponse(pending, text => {
    const value = codec.parse(schema, text, resource) as SchemaTypes[R];
    if (check && !check(value)) throw new TypeError('Response identity does not match the caller request');
    return value;
  }, secrets);
  const readPage = <K extends 'HostPage' | 'AgentPage' | 'WorkspacePage'>(
    schema: K, query: { generation?: string; after?: string } | undefined, run: () => Promise<CloudWireResult>,
  ) => {
    const expected = inventoryGeneration(query);
    return cloudResponse(run(), text => codec.page(schema, text, expected) as SchemaTypes[K], secrets);
  };
  async function write<K extends RequestSchema, R extends keyof SchemaTypes>(
    operation: Mutation, inputSchema: K, input: SchemaTypes[K], outputSchema: R,
    opts: CallOptions | undefined,
    run: (bodySerializer: () => string) => Promise<CloudWireResult>,
    check?: (value: SchemaTypes[R]) => boolean,
  ): Promise<SchemaTypes[R]> {
    opts?.signal?.throwIfAborted();
    const body = codec.stringify(inputSchema, input);
    try { return await read(outputSchema, run(() => body), check, false); }
    catch (error) {
      if (isCloudRejection(error)) throw error;
      throw new CloudNamespaceOutcomeUnknown(operation);
    }
  }
  return {
    spaceChanges: (query: Query<W.NamespaceSpaceChangesData>, opts?: CallOptions) =>
      read('SpaceChanges', wire.namespaceSpaceChanges({ client, query, signal: opts?.signal })),
    describe: (opts?: CallOptions) => cloudResponse(wire.namespaceDescribe({ client, signal: opts?.signal }), text => {
      const descriptor = codec.parse('Descriptor', text) as SchemaTypes['Descriptor'];
      if (descriptor.namespace_id !== namespaceId || !descriptor.generation) throw new TypeError('Descriptor identity does not match the caller request');
      return descriptor;
    }, secrets),
    listHosts: (query?: Query<W.NamespaceListHostsData>, opts?: CallOptions) =>
      readPage('HostPage', query, () => wire.namespaceListHosts({ client, query, signal: opts?.signal })),
    listWorkspaces: (query?: Query<W.NamespaceListWorkspacesData>, opts?: CallOptions) =>
      readPage('WorkspacePage', query, () => wire.namespaceListWorkspaces({ client, query, signal: opts?.signal })),
    listAgents(query?: Query<W.NamespaceListAgentsData>, opts?: CallOptions) {
      assertCloudIdentifiers(query);
      return readPage('AgentPage', query, () => wire.namespaceListAgents({ client, query, signal: opts?.signal }));
    },
    /** Read accepted cloud messages for one exact registration with an exclusive older-page cursor. */
    listMessages(query: Query<W.NamespaceListMessagesData>, opts?: CallOptions) {
      assertCloudIdentifiers(query);
      const { host, name, incarnation } = query;
      const isAgent = (agent: SchemaTypes['AgentView']) => agent.host === host && agent.name === name && agent.incarnation === incarnation;
      return read('MessageHistoryPage', wire.namespaceListMessages({ client, query, signal: opts?.signal }),
        result => result.items.every(message => isAgent(message.from) || isAgent(message.to)));
    },
    /** Read the bounded visible model records for one exact registration. */
    listTranscript(query: Query<W.NamespaceListTranscriptData>, opts?: CallOptions) {
      assertCloudIdentifiers(query);
      const { host, name, incarnation } = query;
      return read('TranscriptHistoryPage', wire.namespaceListTranscript({ client, query, signal: opts?.signal }),
        ({ agent }) => agent.host === host && agent.name === name && agent.incarnation === incarnation);
    },
    renewHost(host: string, body: SchemaTypes['HostRegistration'], opts?: CallOptions) {
      requirePathSegment(host);
      return write('renewHost', 'HostRegistration', body, 'HostView', opts,
        bodySerializer => wire.namespaceRenewHost({ client, path: { host }, body, bodySerializer, signal: opts?.signal }),
        result => result.name === host);
    },
    registerAgent(name: string, body: SchemaTypes['AgentRegistration'], opts?: CallOptions) {
      requirePathSegment(name);
      const { host, incarnation } = body;
      return write('registerAgent', 'AgentRegistration', body, 'AgentView', opts,
        bodySerializer => wire.namespaceRegisterAgent({ client, path: { name }, body, bodySerializer, signal: opts?.signal }),
        result => result.name === name && result.host === host && result.incarnation === incarnation && sameWorkspace(result.workspace, body.workspace));
    },
    retireAgent(name: string, body: SchemaTypes['AgentRegistration'], opts?: CallOptions) {
      requirePathSegment(name);
      const { host, incarnation } = body;
      return write('retireAgent', 'AgentRegistration', body, 'AgentView', opts,
        bodySerializer => wire.namespaceRetireAgent({ client, path: { name }, body, bodySerializer, signal: opts?.signal }),
        result => result.name === name && result.host === host && result.incarnation === incarnation && sameWorkspace(result.workspace, body.workspace));
    },
    send(body: SchemaTypes['SendMessage'], opts?: CallOptions) {
      requirePathSegment(body.request_id);
      return write('send', 'SendMessage', body, 'MessageView', opts,
        bodySerializer => wire.namespaceSendMessage({ client, body, bodySerializer, signal: opts?.signal }));
    },
    getOperation(request_id: string, opts?: CallOptions) {
      requirePathSegment(request_id);
      return read('MessageOperation', wire.namespaceGetOperation({ client, path: { request_id }, signal: opts?.signal }),
        result => result.request_id === request_id, false);
    },
    claimInbox(body: SchemaTypes['ClaimInbox'], opts?: CallOptions) {
      const { claim_id } = body;
      return write('claimInbox', 'ClaimInbox', body, 'InboxBatch', opts,
        bodySerializer => wire.namespaceClaimInbox({ client, body, bodySerializer, signal: opts?.signal }),
        result => result.claim_id === claim_id);
    },
    confirmInbox(body: SchemaTypes['ConfirmInbox'], opts?: CallOptions) {
      const ids = [...body.message_ids];
      return write('confirmInbox', 'ConfirmInbox', body, 'InboxConfirmation', opts,
        bodySerializer => wire.namespaceConfirmInbox({ client, body, bodySerializer, signal: opts?.signal }),
        result => result.confirmed.length === ids.length && result.confirmed.every((id, index) => id === ids[index]));
    },
    recoverInbox(body: SchemaTypes['RecoverInbox'], opts?: CallOptions) {
      const { claim_id } = body;
      return write('recoverInbox', 'RecoverInbox', body, 'InboxBatch', opts,
        bodySerializer => wire.namespaceRecoverInbox({ client, body, bodySerializer, signal: opts?.signal }),
        result => result.claim_id === claim_id);
    },
  };
}
