import { assertCloudIdentifiers } from './cloud-identifiers.js';
/** @aiPurpose Administer cloud namespaces and host operations over the native generated contract. */
import { createClient } from './generated/cloud-admin/client/index.js';
import * as wire from './generated/cloud-admin/sdk.gen.js';
import type { SchemaTypes } from './generated/cloud-admin/schema-types.gen.js';
import type * as W from './generated/cloud-admin/types.gen.js';
import { wireDocument } from './generated/cloud-admin/wire-schema.gen.js';
import { assertCloudRequestId, assertCloudToken, assertNamespaceId, cloudBaseUrl, cloudFetchConfig, cloudResponse, CloudHttpError, createCloudCodec, inventoryGeneration, isCloudRejection } from './cloud-transport.js';
import type { CloudClientOptions, CloudWireResult } from './cloud-transport.js';
import type { CallOptions } from './transport.js';

export { CloudHttpError as CloudAdminError };
export type CloudAdminOptions = CloudClientOptions;
export type CloudAdminTypes = SchemaTypes;
export type CloudNamespacePut = W.PutAdminNamespaceData['body'];
export type CloudNamespaceRotation = W.RotateAdminNamespaceTokenData['body'];
export type CloudControlInput = W.CreateAdminControlOperationData['body'];
export type CloudControlReceipt = SchemaTypes['ControlReceipt'];
export type CloudHostCapabilities = SchemaTypes['HostCapabilities'];
type Page = NonNullable<W.ListAdminNamespacesData['query']>;
type AgentPage = NonNullable<W.ListAdminNamespaceAgentsData['query']>;
type HostPage = NonNullable<W.ListAdminNamespaceHostsData['query']>;
type WorkspaceQuery = NonNullable<W.ListAdminWorkspacesData['query']>;
const codec = createCloudCodec(wireDocument);

/** @aiPurpose Distinguish an uncertain admin effect without exposing credentials. @aiUsage If retrying, reuse the original endpoint, namespace and complete request body. */
export class CloudAdminOutcomeUnknown extends Error {
  readonly code = 'OutcomeUnknown';
  constructor(readonly operation: 'putNamespace' | 'rotateNamespaceToken' | 'submitControlOperation' | 'sendHumanMessage', readonly namespaceId: string, readonly requestId: string) {
    super(operation === 'sendHumanMessage' ? 'Cloud message submission is unconfirmed; retry only with the original request identity and input' : operation === 'submitControlOperation'
      ? 'Cloud operation submission is unconfirmed; look up the original receipt before replaying its exact input'
      : 'Cloud namespace change outcome is unknown; retry only with the original request identity and input');
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
    || (receipt.status === 'queued' || receipt.status === 'running' ? receipt.outcome !== null : (receipt.outcome?.status === 'workspace_added' ? 'succeeded' : receipt.outcome?.status) !== receipt.status)
    || (receipt.status === 'queued' && receipt.claim_id !== null)
    || (receipt.status === 'running' && !receipt.claim_id)) {
    throw new TypeError('Cloud operation receipt identity or state mismatch');
  }
  if (receipt.outcome?.status === 'workspace_added' && (receipt.command.kind !== 'add_workspace' || !receipt.outcome.workspace_id)) {
    throw new TypeError('Cloud operation workspace result mismatch');
  }
  if (receipt.outcome?.status === 'succeeded' && (receipt.command.kind === 'add_workspace' || receipt.outcome.agent.host !== receipt.host
    || (receipt.command.name != null && receipt.command.name !== receipt.outcome.agent.name)
    || (receipt.command.kind === 'launch' && (receipt.outcome.agent.workspace.kind !== 'project'
      || receipt.command.workspace_id !== receipt.outcome.agent.workspace.workspace_id))
    || (receipt.command.kind === 'stop' && receipt.command.incarnation !== receipt.outcome.agent.incarnation))) {
    throw new TypeError('Cloud operation result target mismatch');
  }
  if (input) {
    const expected = input.command, actual = receipt.command;
    const sameCommand = expected.kind === 'launch' ? actual.kind === 'launch' && actual.workspace_id === expected.workspace_id
      && (actual.role ?? null) === (expected.role ?? null) && (actual.name ?? null) === (expected.name ?? null)
      && (actual.working_directory ?? null) === (expected.working_directory ?? null) && (actual.create_directory ?? null) === (expected.create_directory ?? null)
      && (actual.title ?? null) === (expected.title ?? null) && (actual.initial_prompt ?? null) === (expected.initial_prompt ?? null)
      : expected.kind === 'add_workspace' ? actual.kind === 'add_workspace' && actual.name === expected.name && actual.parent === expected.parent
        && actual.directory === expected.directory && actual.create_directory === expected.create_directory
      : actual.kind === 'stop' && actual.name === expected.name && actual.incarnation === expected.incarnation;
    if (receipt.generation !== input.generation || receipt.host !== input.host || receipt.installation_id !== input.installation_id || !sameCommand) {
      throw new TypeError('Cloud operation receipt differs from the original request');
    }
  }
  return receipt;
}

/** @aiPurpose Bind cloud administration to one deployment root and its admin bearer. @aiUsage Namespace communication uses a different client and bearer. Writes are never replayed implicitly. */
export function createCloudAdminClient(options: CloudAdminOptions) {
  const client = createClient(cloudFetchConfig(options, 'admin'));
  const secrets = [options.token];
  const read = <K extends keyof SchemaTypes>(schema: K, pending: Promise<CloudWireResult>) =>
    cloudResponse(pending, text => codec.parse(schema, text, true) as SchemaTypes[K], secrets);
  const readPage = <K extends 'HostPage' | 'AgentPage' | 'WorkspacePage'>(
    schema: K, query: { generation?: string; after?: string } | undefined, run: () => Promise<CloudWireResult>,
  ) => {
    const expected = inventoryGeneration(query);
    return cloudResponse(run(), text => codec.page(schema, text, expected) as SchemaTypes[K], secrets);
  };
  const withNamespace = (id: string, opts?: CallOptions) => {
    assertNamespaceId(id);
    return { client, path: { id }, signal: opts?.signal };
  };
  const decodeControl = (text: string, requestId: string, input?: CloudControlInput) =>
    validateControlReceipt(codec.parse('ControlReceipt', text) as CloudControlReceipt, requestId, input);
  function getControlOperation(id: string, requestId: string, opts?: CallOptions, original?: CloudControlInput) {
    assertControlRequestId(requestId);
    const namespace = withNamespace(id, opts);
    return cloudResponse(wire.getAdminControlOperation({ ...namespace, path: { id, request_id: requestId } }),
      text => decodeControl(text, requestId, original), secrets);
  }
  async function change(
    operation: CloudAdminOutcomeUnknown['operation'],
    id: string, requestId: string, token: string, receiptVersion: number, opts: CallOptions | undefined,
    run: () => Promise<CloudWireResult>,
  ): Promise<SchemaTypes['NamespaceReceipt']> {
    assertNamespaceId(id);
    assertCloudRequestId(requestId);
    assertCloudToken(token);
    opts?.signal?.throwIfAborted();
    try {
      return await cloudResponse(run(), text => {
        const { namespace_id: receiptId, namespace_url, generation, credential_version } = codec.parse('NamespaceReceipt', text) as SchemaTypes['NamespaceReceipt'];
        if (receiptId !== id || credential_version !== receiptVersion
          || !cloudBaseUrl(namespace_url, 'namespace').endsWith(`/namespaces/${id}/api/v1`)) throw new TypeError('Namespace receipt identity mismatch');
        return { namespace_id: id, namespace_url, generation, credential_version };
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
    listNamespaces: (query?: Page, opts?: CallOptions) => read('NamespacePage', wire.listAdminNamespaces({ client, query, signal: opts?.signal })),
    getNamespace: (id: string, opts?: CallOptions) => cloudResponse(wire.getAdminNamespace(withNamespace(id, opts)), text => {
      const namespace = codec.parse('NamespaceRow', text, true) as SchemaTypes['NamespaceRow'];
      if (namespace.namespace_id !== id || !namespace.generation) throw new TypeError('Namespace identity does not match the request');
      return namespace;
    }, secrets),
    listHosts: (id: string, query?: HostPage, opts?: CallOptions) =>
      readPage('HostPage', query, () => wire.listAdminNamespaceHosts({ ...withNamespace(id, opts), query })),
    listWorkspaces: (id: string, query?: WorkspaceQuery, opts?: CallOptions) =>
      readPage('WorkspacePage', query, () => wire.listAdminWorkspaces({ ...withNamespace(id, opts), query })),
    listAgents(id: string, query?: AgentPage, opts?: CallOptions) {
      assertCloudIdentifiers(query);
      return readPage('AgentPage', query, () => wire.listAdminNamespaceAgents({ ...withNamespace(id, opts), query }));
    },
    spaceChanges: (id: string, query: W.AdminSpaceChangesData['query'], opts?: CallOptions) =>
      read('SpaceChanges', wire.adminSpaceChanges({ ...withNamespace(id, opts), query })),
    listMessages(id: string, query: W.ListAdminNamespaceMessagesData['query'], opts?: CallOptions) {
      assertCloudIdentifiers(query);
      const same = (agent: SchemaTypes['AgentView']) => agent.host === query.host && agent.name === query.name && agent.incarnation === query.incarnation;
      return cloudResponse(wire.listAdminNamespaceMessages({ ...withNamespace(id, opts), query }), text => {
        const page = codec.parse('MessageHistoryPage', text, true) as SchemaTypes['MessageHistoryPage'];
        if (!page.items.every(message => same(message.from) || same(message.to))) throw new TypeError('Cloud history target mismatch');
        return page;
      }, secrets);
    },
    /** Read the bounded visible model records for one exact registration. */
    listTranscript(id: string, query: W.ListAdminNamespaceTranscriptData['query'], opts?: CallOptions) {
      assertCloudIdentifiers(query);
      const { host, name, incarnation } = query;
      return cloudResponse(wire.listAdminNamespaceTranscript({ ...withNamespace(id, opts), query }), text => {
        const page = codec.parse('TranscriptHistoryPage', text, true) as SchemaTypes['TranscriptHistoryPage'];
        if (page.agent.host !== host || page.agent.name !== name || page.agent.incarnation !== incarnation)
          throw new TypeError('Cloud transcript target mismatch');
        return page;
      }, secrets);
    },
    async sendHumanMessage(id: string, input: SchemaTypes['SendHumanMessage'], opts?: CallOptions) {
      const namespace = withNamespace(id, opts);
      const encoded = codec.stringify('SendHumanMessage', input);
      const body = JSON.parse(encoded) as SchemaTypes['SendHumanMessage'];
      assertControlRequestId(body.request_id);
      opts?.signal?.throwIfAborted();
      try {
        return await cloudResponse(wire.sendAdminNamespaceMessage({ ...namespace, body }), text => {
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
      const namespace = withNamespace(id, opts);
      if (!/^[a-z0-9][a-z0-9_.-]{0,252}$/.test(host)) throw new TypeError('Invalid cloud host name');
      return cloudResponse(wire.getAdminHostCapabilities({ ...namespace, path: { id, host } }), text => {
        const capabilities = codec.parse('HostCapabilities', text, true) as CloudHostCapabilities;
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
      const namespace = withNamespace(id, opts);
      // Encode into an owned snapshot before dispatch; no caller mutation can change the retained identity.
      const body = snapshotControlInput(input);
      opts?.signal?.throwIfAborted();
      try {
        return await cloudResponse(wire.createAdminControlOperation({ ...namespace, body }), text => decodeControl(text, body.request_id, body), secrets);
      } catch (error) {
        if (isCloudRejection(error) && ![408, 425, 429].includes(error.status!)) throw error;
        const publicId = body.request_id.includes(options.token) ? '[redacted]' : body.request_id;
        throw new CloudAdminOutcomeUnknown('submitControlOperation', id, publicId);
      }
    },
    putNamespace(id: string, input: CloudNamespacePut, opts?: CallOptions) {
      const { request_id, token } = input;
      return change('putNamespace', id, request_id, token, 1, opts,
        () => wire.putAdminNamespace({ ...withNamespace(id, opts), body: { request_id, token } }));
    },
    rotateNamespaceToken(id: string, input: CloudNamespaceRotation, opts?: CallOptions) {
      const { request_id, token, expected_version } = input;
      if (!Number.isInteger(expected_version) || expected_version < 1 || expected_version > 0xffff_ffff) {
        throw new TypeError('Expected credential version must be a positive 32-bit integer');
      }
      return change('rotateNamespaceToken', id, request_id, token, expected_version + 1, opts,
        () => wire.rotateAdminNamespaceToken({ ...withNamespace(id, opts), body: { request_id, token, expected_version } }));
    },
  };
}
