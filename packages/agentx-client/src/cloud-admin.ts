/** @aiPurpose Administer cloud scopes and host operations over the native generated contract. */
import { createClient } from './generated/cloud-admin/client/index.js';
import * as wire from './generated/cloud-admin/sdk.gen.js';
import type { SchemaTypes } from './generated/cloud-admin/schema-types.gen.js';
import type * as W from './generated/cloud-admin/types.gen.js';
import { wireDocument } from './generated/cloud-admin/wire-schema.gen.js';
import { createSchemaCodec } from './schema-codec.js';
import { assertCloudRequestId, assertCloudToken, assertScopeId, cloudBaseUrl, cloudFetchConfig, cloudResponse, CloudHttpError, isCloudRejection } from './cloud-transport.js';
import type { CloudClientOptions, CloudWireResult } from './cloud-transport.js';
import type { CallOptions } from './transport.js';

export { CloudHttpError as CloudAdminError };
export type CloudAdminOptions = CloudClientOptions;
export type CloudAdminTypes = SchemaTypes;
export type CloudScopePut = W.PutAdminScopeData['body'];
export type CloudScopeRotation = W.RotateAdminScopeTokenData['body'];
export type CloudControlInput = W.CreateAdminControlOperationData['body'];
export type CloudControlReceipt = SchemaTypes['ControlReceipt'];
export type CloudHostCapabilities = SchemaTypes['HostCapabilities'];
type Page = NonNullable<W.ListAdminScopesData['query']>;
type AgentPage = NonNullable<W.ListAdminScopeAgentsData['query']>;
const codec = createSchemaCodec(wireDocument);

/** @aiPurpose Distinguish an uncertain admin effect without exposing credentials. @aiUsage If retrying, reuse the original endpoint, scope and complete request body. */
export class CloudAdminOutcomeUnknown extends Error {
  readonly code = 'OutcomeUnknown';
  constructor(readonly operation: 'putScope' | 'rotateScopeToken' | 'submitControlOperation' | 'sendHumanMessage', readonly scopeId: string, readonly requestId: string) {
    super(operation === 'sendHumanMessage' ? 'Cloud message submission is unconfirmed; retry only with the original request identity and input' : operation === 'submitControlOperation'
      ? 'Cloud operation submission is unconfirmed; look up the original receipt before replaying its exact input'
      : 'Cloud scope change outcome is unknown; retry only with the original request identity and input');
    this.name = 'CloudAdminOutcomeUnknown';
  }
}

export type CloudAdminClient = ReturnType<typeof createCloudAdminClient>;

function assertControlRequestId(value: string): void {
  if (typeof value !== 'string' || !/^[A-Za-z0-9_.:-]{1,128}$/.test(value) || value === '.' || value === '..') {
    throw new TypeError('Control request ID must be 1–128 ASCII letters, digits, dots, colons, hyphens or underscores and cannot be . or ..');
  }
}

/** Read or snapshot the native request contract without keeping references to caller data. */
export function snapshotControlInput(value: unknown): CloudControlInput {
  const body = JSON.parse(codec.stringify('CreateControlOperation', value)) as CloudControlInput;
  assertControlRequestId(body.request_id);
  return body;
}

function validateControlReceipt(receipt: CloudControlReceipt, requestId: string, input?: CloudControlInput): CloudControlReceipt {
  if (receipt.request_id !== requestId
    || (receipt.status === 'queued' || receipt.status === 'running' ? receipt.outcome !== null : receipt.outcome?.status !== receipt.status)
    || (receipt.status === 'queued' && receipt.claim_id !== null)
    || (receipt.status === 'running' && !receipt.claim_id)) {
    throw new TypeError('Cloud operation receipt identity or state mismatch');
  }
  if (receipt.outcome?.status === 'succeeded' && (receipt.outcome.agent.host !== receipt.host
    || (receipt.command.name != null && receipt.command.name !== receipt.outcome.agent.name)
    || (receipt.command.kind === 'stop' && receipt.command.incarnation !== receipt.outcome.agent.incarnation))) {
    throw new TypeError('Cloud operation result target mismatch');
  }
  if (input) {
    const expected = input.command, actual = receipt.command;
    const sameCommand = expected.kind === 'launch' ? actual.kind === 'launch' && actual.workspace === expected.workspace
      && (actual.role ?? null) === (expected.role ?? null) && (actual.name ?? null) === (expected.name ?? null)
      && (actual.working_directory ?? null) === (expected.working_directory ?? null) && (actual.create_directory ?? null) === (expected.create_directory ?? null)
      && (actual.title ?? null) === (expected.title ?? null) && (actual.initial_prompt ?? null) === (expected.initial_prompt ?? null)
      : actual.kind === 'stop' && actual.name === expected.name && actual.incarnation === expected.incarnation;
    if (receipt.generation !== input.generation || receipt.host !== input.host || receipt.installation_id !== input.installation_id || !sameCommand) {
      throw new TypeError('Cloud operation receipt differs from the original request');
    }
  }
  return receipt;
}

/** @aiPurpose Bind cloud administration to one deployment root and its admin bearer. @aiUsage Scope communication uses a different client and bearer. Writes are never replayed implicitly. */
export function createCloudAdminClient(options: CloudAdminOptions) {
  const client = createClient(cloudFetchConfig(options, 'admin'));
  const secrets = [options.token];
  const read = <K extends keyof SchemaTypes>(schema: K, pending: Promise<CloudWireResult>) =>
    cloudResponse(pending, text => codec.parse(schema, text) as SchemaTypes[K], secrets);
  const withScope = (id: string, opts?: CallOptions) => {
    assertScopeId(id);
    return { client, path: { id }, signal: opts?.signal };
  };
  const decodeControl = (text: string, requestId: string, input?: CloudControlInput) =>
    validateControlReceipt(codec.parse('ControlReceipt', text) as CloudControlReceipt, requestId, input);
  function getControlOperation(id: string, requestId: string, opts?: CallOptions, original?: CloudControlInput) {
    assertControlRequestId(requestId);
    const scope = withScope(id, opts);
    return cloudResponse(wire.getAdminControlOperation({ ...scope, path: { id, request_id: requestId } }),
      text => decodeControl(text, requestId, original), secrets);
  }
  async function change(
    operation: CloudAdminOutcomeUnknown['operation'],
    id: string, requestId: string, token: string, receiptVersion: number, opts: CallOptions | undefined,
    run: () => Promise<CloudWireResult>,
  ): Promise<SchemaTypes['ScopeReceipt']> {
    assertScopeId(id);
    assertCloudRequestId(requestId);
    assertCloudToken(token);
    opts?.signal?.throwIfAborted();
    try {
      return await cloudResponse(run(), text => {
        const { id: receiptId, scope_url, generation, credential_version } = codec.parse('ScopeReceipt', text) as SchemaTypes['ScopeReceipt'];
        if (receiptId !== id || credential_version !== receiptVersion
          || !cloudBaseUrl(scope_url, 'scope').endsWith(`/scopes/${id}/api/v1`)) throw new TypeError('Scope receipt identity mismatch');
        return { id, scope_url, generation, credential_version };
      }, [...secrets, token]);
    } catch (error) {
      if (isCloudRejection(error)) throw error;
      // A request ID chosen by the caller may itself be a secret; never echo it in that case.
      const publicId = [...secrets, token].some(secret => requestId.includes(secret)) ? '[redacted]' : requestId;
      throw new CloudAdminOutcomeUnknown(operation, id, publicId);
    }
  }
  return {
    overview: (opts?: CallOptions) => read('AdminOverview', wire.getAdminOverview({ client, signal: opts?.signal })),
    listScopes: (query?: Page, opts?: CallOptions) => read('ScopePage', wire.listAdminScopes({ client, query, signal: opts?.signal })),
    getScope: (id: string, opts?: CallOptions) => read('ScopeRow', wire.getAdminScope(withScope(id, opts))),
    listHosts: (id: string, query?: Page, opts?: CallOptions) => read('HostPage', wire.listAdminScopeHosts({ ...withScope(id, opts), query })),
    listAgents: (id: string, query?: AgentPage, opts?: CallOptions) => read('AgentPage', wire.listAdminScopeAgents({ ...withScope(id, opts), query })),
    spaceChanges: (id: string, query: W.AdminSpaceChangesData['query'], opts?: CallOptions) =>
      read('SpaceChanges', wire.adminSpaceChanges({ ...withScope(id, opts), query })),
    listMessages(id: string, query: W.ListAdminScopeMessagesData['query'], opts?: CallOptions) {
      const same = (agent: SchemaTypes['AgentView']) => agent.host === query.host && agent.name === query.name && agent.incarnation === query.incarnation;
      return cloudResponse(wire.listAdminScopeMessages({ ...withScope(id, opts), query }), text => {
        const page = codec.parse('MessageHistoryPage', text) as SchemaTypes['MessageHistoryPage'];
        if (!page.items.every(message => same(message.from) || same(message.to))) throw new TypeError('Cloud history target mismatch');
        return page;
      }, secrets);
    },
    /** Read the bounded visible model records for one exact registration. */
    listTranscript(id: string, query: W.ListAdminScopeTranscriptData['query'], opts?: CallOptions) {
      const { host, name, incarnation } = query;
      return cloudResponse(wire.listAdminScopeTranscript({ ...withScope(id, opts), query }), text => {
        const page = codec.parse('TranscriptHistoryPage', text) as SchemaTypes['TranscriptHistoryPage'];
        if (page.agent.host !== host || page.agent.name !== name || page.agent.incarnation !== incarnation)
          throw new TypeError('Cloud transcript target mismatch');
        return page;
      }, secrets);
    },
    async sendHumanMessage(id: string, input: SchemaTypes['SendHumanMessage'], opts?: CallOptions) {
      const scope = withScope(id, opts);
      const encoded = codec.stringify('SendHumanMessage', input);
      const body = JSON.parse(encoded) as SchemaTypes['SendHumanMessage'];
      assertControlRequestId(body.request_id);
      opts?.signal?.throwIfAborted();
      try {
        return await cloudResponse(wire.sendAdminScopeMessage({ ...scope, body }), text => {
          const message = codec.parse('MessageView', text) as SchemaTypes['MessageView'];
          if (message.to.host !== body.to.host || message.to.name !== body.to.name || message.to.incarnation !== body.to.incarnation
            || message.body !== body.body) throw new TypeError('Cloud message receipt mismatch');
          return message;
        }, secrets);
      } catch (error) {
        if (isCloudRejection(error) && ![408, 425, 429].includes(error.status!)) throw error;
        const publicId = body.request_id.includes(options.token) ? '[redacted]' : body.request_id;
        throw new CloudAdminOutcomeUnknown('sendHumanMessage', id, publicId);
      }
    },
    getHostCapabilities(id: string, host: string, opts?: CallOptions) {
      const scope = withScope(id, opts);
      if (!/^[a-z0-9][a-z0-9_.-]{0,252}$/.test(host)) throw new TypeError('Invalid cloud host name');
      return cloudResponse(wire.getAdminHostCapabilities({ ...scope, path: { id, host } }), text => {
        const capabilities = codec.parse('HostCapabilities', text) as CloudHostCapabilities;
        if (capabilities.host !== host) throw new TypeError('Cloud capability host mismatch');
        return capabilities;
      }, secrets);
    },
    getControlOperation(id: string, requestId: string, opts?: CallOptions) {
      return getControlOperation(id, requestId, opts);
    },
    /** Recover by the full original identity before offering a replay. */
    recoverControlOperation(id: string, original: CloudControlInput, opts?: CallOptions) {
      const snapshot = snapshotControlInput(original);
      return getControlOperation(id, snapshot.request_id, opts, snapshot);
    },
    async submitControlOperation(id: string, input: CloudControlInput, opts?: CallOptions): Promise<CloudControlReceipt> {
      const scope = withScope(id, opts);
      // Encode into an owned snapshot before dispatch; no caller mutation can change the retained identity.
      const body = snapshotControlInput(input);
      opts?.signal?.throwIfAborted();
      try {
        return await cloudResponse(wire.createAdminControlOperation({ ...scope, body }), text => decodeControl(text, body.request_id, body), secrets);
      } catch (error) {
        if (isCloudRejection(error) && ![408, 425, 429].includes(error.status!)) throw error;
        const publicId = body.request_id.includes(options.token) ? '[redacted]' : body.request_id;
        throw new CloudAdminOutcomeUnknown('submitControlOperation', id, publicId);
      }
    },
    putScope(id: string, input: CloudScopePut, opts?: CallOptions) {
      const { request_id, token } = input;
      return change('putScope', id, request_id, token, 1, opts,
        () => wire.putAdminScope({ ...withScope(id, opts), body: { request_id, token } }));
    },
    rotateScopeToken(id: string, input: CloudScopeRotation, opts?: CallOptions) {
      const { request_id, token, expected_version } = input;
      if (!Number.isInteger(expected_version) || expected_version < 1 || expected_version > 0xffff_ffff) {
        throw new TypeError('Expected credential version must be a positive 32-bit integer');
      }
      return change('rotateScopeToken', id, request_id, token, expected_version + 1, opts,
        () => wire.rotateAdminScopeToken({ ...withScope(id, opts), body: { request_id, token, expected_version } }));
    },
  };
}
