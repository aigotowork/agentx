/** @aiPurpose Compose all selected management capabilities on their actual generated HTTP operations. */
import { createClient } from './generated/management/client/index.js';
import * as wire from './generated/management/sdk.gen.js';
import type * as M from './generated/management/types.gen.js';
import type { SchemaTypes } from './generated/management/schema-types.gen.js';
import { createManagementClient as createPilotClient, decode, checkIdentity, ManagementError, ReceiptError } from './client.js';
import type { ManagementClient as PilotClient, ManagementClientOptions } from './client.js';
import { parseManagementJson, stringifyManagementJson } from './codec.js';
import { freezeInput, MutationOutcomeUnknown } from './transport.js';
import type { CallOptions } from './transport.js';

type Exact = { generation: string; agent: M.AgentRef };
type ExactPage = Exact & { after?: string; limit?: number };
type ExactHistory = Exact & { before?: string; limit?: number };
type Query<T extends { query?: unknown }> = NonNullable<T['query']>;
type Submit = M.SubmitLaunch | M.SubmitDirectoryMessage | M.SubmitTarget | M.SubmitLabels;
type SubmitSchema = 'SubmitLaunch' | 'SubmitDirectoryMessage' | 'SubmitTarget' | 'SubmitLabels';
type WireResult = { data?: unknown; error?: unknown; response?: Response };
/** @aiPurpose Expose actual management schema types without a second DTO catalog. @aiUsage Index by emitted native component name; plain JSON long integers use bigint. */
export type ManagementTypes = SchemaTypes;
/** @aiPurpose Expose complete management methods beside the original pilot conveniences. @aiUsage Journal operations return original Operation; Inspector results are their own Snapshot/Job/Attempt values. */
export interface ManagementClient extends PilotClient {
  workspaces: {
    list(generation: string, options?: CallOptions): Promise<M.WorkspaceCatalog>;
    members(input: Query<M.ManagementListWorkspaceMembersData>, options?: CallOptions): Promise<M.AgentPage>;
  };
  roles: { list(input: { generation: string; workspace: string }, options?: CallOptions): Promise<M.RoleCatalog> };
  directory: {
    inspect(input: { generation: string; directory: string }, options?: CallOptions): Promise<M.DirectoryEntry>;
    send(input: M.SubmitDirectoryMessage, options?: CallOptions): Promise<M.Operation>;
  };
  agents: {
    list: PilotClient['listAgents']; get: PilotClient['getAgent'];
    launch(input: M.SubmitLaunch, options?: CallOptions): Promise<M.Operation>;
    kill(input: M.SubmitTarget, options?: CallOptions): Promise<M.Operation>;
    stopParticipation(input: M.SubmitTarget, options?: CallOptions): Promise<M.Operation>;
    resume(input: M.SubmitTarget, options?: CallOptions): Promise<M.Operation>;
    fork(input: M.SubmitTarget, options?: CallOptions): Promise<M.Operation>;
    transcript(input: M.SubmitTarget, options?: CallOptions): Promise<M.Operation>;
    relations(input: ExactPage, options?: CallOptions): Promise<M.LaunchRelations>;
    messages: {
      list(input: ExactPage, options?: CallOptions): Promise<M.MessagePage>;
      history(input: ExactHistory, options?: CallOptions): Promise<M.MessageHistoryPage>;
      send: PilotClient['send'];
    };
    labels: { get(input: Exact, options?: CallOptions): Promise<M.LabelsSnapshot>; patch(input: M.SubmitLabels, options?: CallOptions): Promise<M.Operation> };
  };
  attachments: {
    upload(input: { generation: string; name: string; media_type?: string; bytes: Blob }, options?: CallOptions): Promise<M.Attachment>;
    download(input: { generation: string; id: string }, options?: CallOptions): Promise<Blob>;
  };
  humanMailbox: {
    list(input: Query<M.ManagementListHumanMailboxData>, options?: CallOptions): Promise<M.HumanMailbox>;
    handle(input: M.HandleHumanMessages, options?: CallOptions): Promise<M.HumanMailbox>;
  };
  officeActivity: { list(input: Query<M.ManagementListOfficeActivityData>, options?: CallOptions): Promise<M.OfficeActivity> };
  inspection: {
    get(generation: string, options?: CallOptions): Promise<M.Snapshot>;
    configure(input: M.ConfigureInput, options?: CallOptions): Promise<M.Snapshot>;
    analyze(input: M.AnalyzeInput, options?: CallOptions): Promise<M.AnalysisJob>;
    cancelAnalysis(input: M.CancelAnalysisInput, options?: CallOptions): Promise<M.AnalysisJob>;
    requestContinuation(input: M.RequestContinuationInput, options?: CallOptions): Promise<M.RecoveryAttempt>;
  };
}
const query = (input: ExactPage) => ({ generation: input.generation, incarnation: input.agent.incarnation ?? undefined, stopped_event: input.agent.stopped_event ?? undefined, after: input.after, limit: input.limit });

/** @aiPurpose Bind the complete management listener with original identity and effect knowledge. @aiUsage No automatic mutation retry, identity fallback, descriptor refresh or upload replay occurs. */
export function createManagementClient(options: ManagementClientOptions): ManagementClient {
  const pilot = createPilotClient(options);
  const client = createClient({ baseUrl: options.baseUrl.replace(/\/$/, ''), fetch: options.fetch, headers: options.headers, parseAs: 'text', credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer' });
  async function journal<K extends SubmitSchema>(schema: K, input: SchemaTypes[K], operation: string, opts: CallOptions | undefined, run: (retained: SchemaTypes[K], body: string) => Promise<WireResult>): Promise<M.Operation> {
    if (opts?.signal?.aborted) throw opts.signal.reason;
    const body = stringifyManagementJson(schema, input);
    const retained = freezeInput(parseManagementJson(schema, body));
    try { return checkIdentity(await decode(run(retained, body), 'Operation'), retained); }
    catch (error) {
      if (error instanceof ManagementError || error instanceof ReceiptError) throw error;
      throw new MutationOutcomeUnknown<SchemaTypes[K]>(operation, retained, error);
    }
  }
  async function mutation<K extends keyof SchemaTypes, R extends keyof SchemaTypes>(schema: K, input: SchemaTypes[K], output: R, operation: string, opts: CallOptions | undefined, run: (retained: SchemaTypes[K], body: string) => Promise<WireResult>): Promise<SchemaTypes[R]> {
    if (opts?.signal?.aborted) throw opts.signal.reason;
    const body = stringifyManagementJson(schema, input);
    const retained = freezeInput(parseManagementJson(schema, body));
    try { return await decode(run(retained, body), output); }
    catch (error) { if (error instanceof ManagementError) throw error; throw new MutationOutcomeUnknown<SchemaTypes[K]>(operation, retained, error); }
  }
  const target = (input: M.SubmitTarget, opts: CallOptions | undefined, operation: 'kill' | 'stopParticipation' | 'resume' | 'fork' | 'transcript') => journal('SubmitTarget', input, operation, opts, (body, encoded) => {
    const args = { client, path: { name: body.agent.name }, body, bodySerializer: () => encoded, signal: opts?.signal };
    switch (operation) {
      case 'kill': return wire.managementKillAgent(args);
      case 'stopParticipation': return wire.managementStopParticipation(args);
      case 'resume': return wire.managementResumeAgent(args);
      case 'fork': return wire.managementForkAgent(args);
      case 'transcript': return wire.managementRequestTranscript(args);
    }
  });
  return Object.assign(pilot, {
    workspaces: {
      list: (generation: string, opts?: CallOptions) => decode(wire.managementListWorkspaces({ client, query: { generation }, signal: opts?.signal }), 'WorkspaceCatalog'),
      members: (input: Query<M.ManagementListWorkspaceMembersData>, opts?: CallOptions) => decode(wire.managementListWorkspaceMembers({ client, query: input, signal: opts?.signal }), 'AgentPage'),
    },
    roles: { list: (input: { generation: string; workspace: string }, opts?: CallOptions) => decode(wire.managementListRoles({ client, query: input, signal: opts?.signal }), 'RoleCatalog') },
    directory: {
      inspect: (input: { generation: string; directory: string }, opts?: CallOptions) => decode(wire.managementInspectDirectory({ client, query: input, signal: opts?.signal }), 'DirectoryEntry'),
      send: (input: M.SubmitDirectoryMessage, opts?: CallOptions) => journal('SubmitDirectoryMessage', input, 'directory.send', opts, (body, encoded) => wire.managementSubmitDirectoryMessage({ client, body, bodySerializer: () => encoded, signal: opts?.signal })),
    },
    agents: {
      list: pilot.listAgents, get: pilot.getAgent,
      launch: (input: M.SubmitLaunch, opts?: CallOptions) => journal('SubmitLaunch', input, 'agents.launch', opts, (body, encoded) => wire.managementSubmitLaunch({ client, body, bodySerializer: () => encoded, signal: opts?.signal })),
      kill: (input: M.SubmitTarget, opts?: CallOptions) => target(input, opts, 'kill'),
      stopParticipation: (input: M.SubmitTarget, opts?: CallOptions) => target(input, opts, 'stopParticipation'),
      resume: (input: M.SubmitTarget, opts?: CallOptions) => target(input, opts, 'resume'),
      fork: (input: M.SubmitTarget, opts?: CallOptions) => target(input, opts, 'fork'),
      transcript: (input: M.SubmitTarget, opts?: CallOptions) => target(input, opts, 'transcript'),
      relations: (input: ExactPage, opts?: CallOptions) => decode(wire.managementListLaunchRelations({ client, path: { name: input.agent.name }, query: query(input), signal: opts?.signal }), 'LaunchRelations'),
      messages: {
        send: pilot.send,
        list: (input: ExactPage, opts?: CallOptions) => decode(wire.managementListMessages({ client, path: { name: input.agent.name }, query: query(input), signal: opts?.signal }), 'MessagePage'),
        history: (input: ExactHistory, opts?: CallOptions) => decode(wire.managementMessageHistory({
          client, path: { name: input.agent.name }, query: { ...query(input), before: input.before }, signal: opts?.signal,
        }), 'MessageHistoryPage'),
      },
      labels: {
        get: (input: Exact, opts?: CallOptions) => decode(wire.managementGetLabels({ client, path: { name: input.agent.name }, query: query(input), signal: opts?.signal }), 'LabelsSnapshot'),
        patch: (input: M.SubmitLabels, opts?: CallOptions) => journal('SubmitLabels', input, 'agents.labels.patch', opts, (body, encoded) => wire.managementPatchLabels({ client, path: { name: body.agent.name }, body, bodySerializer: () => encoded, signal: opts?.signal })),
      },
    },
    attachments: {
      async upload(input: { generation: string; name: string; media_type?: string; bytes: Blob }, opts?: CallOptions) {
        if (opts?.signal?.aborted) throw opts.signal.reason;
        const retained = Object.freeze({ ...input });
        try { return await decode(wire.managementUploadAttachment({ client, query: { generation: input.generation, name: input.name, media_type: input.media_type }, body: input.bytes, bodySerializer: body => body, signal: opts?.signal }), 'Attachment'); }
        catch (error) { if (error instanceof ManagementError) throw error; throw new MutationOutcomeUnknown('attachments.upload', retained, error); }
      },
      async download(input: { generation: string; id: string }, opts?: CallOptions) {
        const result = await wire.managementDownloadAttachment({ client, path: { id: input.id }, query: { generation: input.generation }, parseAs: 'blob', signal: opts?.signal });
        if (result.response?.ok && result.data instanceof Blob) return result.data;
        await decode(Promise.resolve(result), 'ErrorEnvelope'); throw new TypeError('Expected binary attachment response');
      },
    },
    humanMailbox: {
      list: (input: Query<M.ManagementListHumanMailboxData>, opts?: CallOptions) => decode(wire.managementListHumanMailbox({ client, query: input, signal: opts?.signal }), 'HumanMailbox'),
      handle: (input: M.HandleHumanMessages, opts?: CallOptions) => mutation('HandleHumanMessages', input, 'HumanMailbox', 'humanMailbox.handle', opts, (body, encoded) => wire.managementHandleHumanMessages({ client, body, bodySerializer: () => encoded, signal: opts?.signal })),
    },
    officeActivity: { list: (input: Query<M.ManagementListOfficeActivityData>, opts?: CallOptions) => decode(wire.managementListOfficeActivity({ client, query: input, signal: opts?.signal }), 'OfficeActivity') },
    inspection: {
      get: (generation: string, opts?: CallOptions) => decode(wire.managementInspectionSnapshot({ client, query: { generation }, signal: opts?.signal }), 'Snapshot'),
      configure: (input: M.ConfigureInput, opts?: CallOptions) => mutation('ConfigureInput', input, 'Snapshot', 'inspection.configure', opts, body => wire.managementConfigureInspection({ client, body: { Configure: body }, bodySerializer: () => `{"Configure":${stringifyManagementJson('ConfigureInput', body)}}`, signal: opts?.signal })),
      analyze: (input: M.AnalyzeInput, opts?: CallOptions) => mutation('AnalyzeInput', input, 'AnalysisJob', 'inspection.analyze', opts, body => wire.managementAnalyzeInspection({ client, body: { Analyze: body }, bodySerializer: () => `{"Analyze":${stringifyManagementJson('AnalyzeInput', body)}}`, signal: opts?.signal })),
      cancelAnalysis: (input: M.CancelAnalysisInput, opts?: CallOptions) => mutation('CancelAnalysisInput', input, 'AnalysisJob', 'inspection.cancelAnalysis', opts, body => wire.managementCancelInspectionAnalysis({ client, body: { CancelAnalysis: body }, bodySerializer: () => `{"CancelAnalysis":${stringifyManagementJson('CancelAnalysisInput', body)}}`, signal: opts?.signal })),
      requestContinuation: (input: M.RequestContinuationInput, opts?: CallOptions) => mutation('RequestContinuationInput', input, 'RecoveryAttempt', 'inspection.requestContinuation', opts, body => wire.managementRequestContinuation({ client, body: { RequestContinuation: body }, bodySerializer: () => `{"RequestContinuation":${stringifyManagementJson('RequestContinuationInput', body)}}`, signal: opts?.signal })),
    },
  });
}
