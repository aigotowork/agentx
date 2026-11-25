/** Domain addresses and observations over the existing management contracts. */
import type { ManagementClientOptions, WaitOptions } from '../client.js';
import type { CallOptions } from '../transport.js';
import type { AgentRef, AgentView, Attachment as NativeAttachment, Descriptor, DirectoryEntry, DirectorySendOutcome, DiscoveryCoverage, ExchangeView, HumanMailbox, LabelsOutcome, LabelsSnapshot, LaunchInput, LaunchOutcome, MessageInput, MessageView, OfficeActivity, Operation, RoleDefinitionView, RoleView, SendOutcome, SubmitLabels, WorkspaceView } from '../generated/management/types.gen.js';

export type ReadonlyValue<T> = T extends object ? { readonly [K in keyof T]: ReadonlyValue<T[K]> } : T;
export type ScopeSelection = ManagementClientOptions & { readonly expectedScope: Descriptor['scope'] };
export type ScopeAddress = Readonly<Pick<Descriptor, 'scope' | 'generation'> & { baseUrl: string }>;
/** Canonical native path, or the explicit literal `global`. */
export type WorkspaceKey = WorkspaceView['path'];
export type WorkspaceAddress = Readonly<{ scope: ScopeAddress; key: WorkspaceKey }>;

type ExactReference<K extends string, F extends keyof AgentRef> =
  Readonly<Pick<AgentRef, 'name'> & Required<{ [P in F]: NonNullable<AgentRef[P]> }> & { kind: K }>;
export type CurrentReference = ExactReference<'current', 'incarnation'>;
export type StoppedReference = ExactReference<'stopped', 'stopped_event'>;
export type AgentReference = CurrentReference | StoppedReference;
export type AgentAddress<R extends AgentReference = AgentReference> = Readonly<{ scope: ScopeAddress; reference: R }>;
export type OperationAddress = Readonly<{ scope: ScopeAddress; requestId: Operation['request_id'] }>;

export type Membership =
  | Readonly<{ kind: 'adopted'; workspace: WorkspaceKey; role: AdoptedRole | null }>
  | Readonly<{ kind: 'legacy' }>
  | Readonly<{ kind: 'unavailable'; reason: string | null }>;
export type AgentSnapshot<R extends AgentReference = AgentReference> = Readonly<{
  address: AgentAddress<R>;
  membership: Membership;
  facts: ReadonlyValue<Omit<AgentView, 'agent' | 'collaboration'>>;
}>;
/** A current catalog definition. Adopted Role facts live on Membership. */
export type RoleDefinition = ReadonlyValue<RoleDefinitionView>;
/** Launch-time adoption, including the concrete profile selected by native resolution. */
export type AdoptedRole = ReadonlyValue<RoleView>;
export type Role = Readonly<{ definition: RoleDefinition }>;
export type RoleCatalog = Readonly<{ workspace: Workspace; roles: readonly Role[] }>;
export type WorkspaceCatalog = Readonly<{
  workspaces: readonly Readonly<{ workspace: Workspace; sources: ReadonlyValue<WorkspaceView['sources']> }>[];
  coverage: ReadonlyValue<DiscoveryCoverage>;
}>;

/** Retain opaque native position with the object it came from. */
export type AgentCursor = Readonly<{ scope: ScopeAddress; position: string }>;
export type MessageCursor = Readonly<{ agent: AgentAddress; position: string; kind?: never }>;
export type MessageHistoryCursor = Readonly<{ kind: 'message-history'; agent: AgentAddress; position: string }>;
export type MessageHistoryRequest = Readonly<{ before?: MessageHistoryCursor; limit?: number }>;
export type PageRequest<C> = Readonly<{ after?: C; limit?: number }>;
export type ScopeAgentPage = Readonly<{
  agents: readonly AgentSnapshot[];
  currentComplete: boolean;
  next: AgentCursor | null;
}>;
export type NativeMemberCursor = Readonly<{ kind: 'workspace-members'; workspace: WorkspaceAddress; currentOnly: boolean; position: string }>;
export type NativeMemberRequest = PageRequest<NativeMemberCursor> & Readonly<{ currentOnly?: boolean }>;
export type NativeMemberPage = Readonly<{
  members: readonly Readonly<{ agent: Agent; snapshot: AgentSnapshot }>[];
  coverage: Readonly<{
    kind: 'native-workspace-members';
    currentOnly: boolean;
    currentComplete: boolean;
    next: NativeMemberCursor | null;
  }>;
}>;
export type Message = ReadonlyValue<MessageView>;
export type MessagePage = Readonly<{
  agent: AgentAddress;
  messages: readonly Message[];
  checkpoint: MessageCursor;
  hasMore: boolean;
}>;

export type MessageHistoryPage = Readonly<{
  agent: AgentAddress;
  messages: readonly Message[];
  nextBefore: MessageHistoryCursor | null;
  /** Forward message checkpoint from the latest read; null on older pages. */
  liveAfter: MessageCursor | null;
}>;

/** One bounded Provider observation with no public continuation. Empty available is distinct from unavailable. */
export type Transcript = Readonly<{ agent: AgentAddress; reason: string | null }> & (
  | Readonly<{ available: true; exchanges: readonly ReadonlyValue<ExchangeView>[] }>
  | Readonly<{ available: false; exchanges: readonly [] }>
);
/** Creation identity stays the same when the navigable observation moves from current to stopped. */
export type RelatedConversation = Readonly<{
  creation: Pick<CurrentReference, 'name' | 'incarnation'>;
  launchFailure: string | null;
}> & (
  | Readonly<{ availability: 'available'; agent: AgentSnapshot }>
  | Readonly<{ availability: 'unavailable'; agent: null }>
);
export type LaunchRelationCursor = Readonly<{ kind: 'launch-relations'; agent: AgentAddress; position: string }>;
export type LaunchRelationsPage = Readonly<{
  agent: AgentAddress;
  origin: 'agent' | 'external' | 'unknown';
  upstream: RelatedConversation | null;
  downstream: readonly RelatedConversation[];
  /** Original fork input, which may no longer be navigable. */
  forkSource?: AgentReference | null;
  next: LaunchRelationCursor | null;
}>;

/** Caller-retainable original command. Send never changes these addressing facts. */
export type PreparedMessage = Readonly<{
  target: AgentAddress<CurrentReference>;
  operation: OperationAddress;
  message: ReadonlyValue<MessageInput>;
}>;
export type LaunchDraft = ReadonlyValue<LaunchInput>;
/** The workspace supplies the selector; `role` is resolved again by the native launch. */
export type WorkspaceLaunchDraft = ReadonlyValue<Omit<LaunchInput, 'workspace'>>;
export type LabelsPatch = ReadonlyValue<Omit<SubmitLabels, 'generation' | 'request_id' | 'agent'>>;
export type AgentLabels = ReadonlyValue<Omit<LabelsSnapshot, 'generation' | 'agent'>> & Readonly<{ agent: AgentAddress<CurrentReference> }>;
type TargetCommand<K extends string, R extends AgentReference = AgentReference> = Readonly<{ kind: K; operation: OperationAddress; target: AgentAddress<R> }>;
type PreparedCommands = {
  transcript: TargetCommand<'transcript'>;
  send: PreparedMessage & Readonly<{ kind: 'send' }>;
  kill: TargetCommand<'kill', CurrentReference>;
  'stop-participation': TargetCommand<'stop-participation', CurrentReference>;
  resume: TargetCommand<'resume'>;
  fork: TargetCommand<'fork'>;
  'patch-labels': TargetCommand<'patch-labels', CurrentReference> & Readonly<{ patch: LabelsPatch }>;
  launch: Readonly<{ kind: 'launch'; operation: OperationAddress; launch: LaunchDraft }>;
  'directory-send': Readonly<{ kind: 'directory-send'; operation: OperationAddress; directory: string; message: ReadonlyValue<MessageInput> }>;
};
export type OperationKind = keyof PreparedCommands;
/** Inert original input. Retain before dispatch and restore only against the original scope. */
export type PreparedOperation<K extends OperationKind = OperationKind> = PreparedCommands[K];
/** Names are diagnostic. Only exact addresses enable navigation, including later receipt recovery. */
export type LaunchResult = ReadonlyValue<Pick<LaunchOutcome, 'agents' | 'failed' | 'errors'>> & Readonly<{ agentAddresses: readonly AgentAddress<CurrentReference>[] | null }>;
export type DirectoryPreview = ReadonlyValue<DirectoryEntry>;
export type DirectorySendResult = ReadonlyValue<Omit<DirectorySendOutcome, 'recipient'>> & Readonly<{ recipient: AgentAddress<CurrentReference> }>;
export type OperationResults = {
  transcript: Transcript;
  send: ReadonlyValue<SendOutcome>;
  kill: Readonly<{ name: string; outcome: 'sent' | 'already_dead' }>;
  'stop-participation': Readonly<{ name: string; outcome: 'participation_retained' | 'participation_stopped' }>;
  resume: LaunchResult;
  fork: LaunchResult;
  launch: LaunchResult;
  'patch-labels': ReadonlyValue<Pick<LabelsOutcome, 'changed'>> & Readonly<{ snapshot: AgentLabels }>;
  'directory-send': DirectorySendResult;
};
export type OperationReceipt = ReadonlyValue<Operation>;
/** The receipt is retained for presentation/diagnostics, including exact original request identity. */
export type TypedOperationReceipt<K extends OperationKind = OperationKind> = Readonly<{
  address: OperationAddress;
  kind: K;
  receipt: OperationReceipt;
}> & (
  | Readonly<{ status: 'Accepted' | 'Running'; result: null; error: null }>
  | Readonly<{ status: 'Succeeded'; result: OperationResults[K]; error: null }>
  | Readonly<{ status: 'Failed' | 'Unknown'; result: null; error: string | null }>
);
export interface TypedOperationHandle<K extends OperationKind> {
  readonly address: OperationAddress;
  observe(options?: CallOptions): Promise<TypedOperationReceipt<K>>;
  /** Ends at the first non-pollable receipt, or stops local observation by signal/deadline. */
  wait(options?: WaitOptions): Promise<TypedOperationReceipt<K>>;
}
export interface OperationHandle {
  readonly address: OperationAddress;
  observe(options?: CallOptions): Promise<OperationReceipt>;
  /** Stop at a native terminal receipt, or stop only local polling by signal/deadline. */
  wait(options?: WaitOptions): Promise<OperationReceipt>;
}

interface AgentReader<R extends AgentReference> {
  readonly kind: R['kind'];
  readonly address: AgentAddress<R>;
  snapshot(options?: CallOptions): Promise<AgentSnapshot<R>>;
  messages(page?: PageRequest<MessageCursor>, options?: CallOptions): Promise<MessagePage>;
  /** Read latest or older history; unavailable history rejects with UnsupportedCapability. */
  messageHistory(page?: MessageHistoryRequest, options?: CallOptions): Promise<MessageHistoryPage>;
  launchRelations(page?: PageRequest<LaunchRelationCursor>, options?: CallOptions): Promise<LaunchRelationsPage>;
  prepareTranscript(requestId: OperationAddress['requestId']): PreparedOperation<'transcript'>;
  /** Explicitly admit one Provider read. Observe the prepared request after acceptance or response loss. */
  requestTranscript(prepared: PreparedOperation<'transcript'>, options?: CallOptions): Promise<TypedOperationReceipt<'transcript'>>;
  prepareResume(requestId: OperationAddress['requestId']): PreparedOperation<'resume'>;
  resume(prepared: PreparedOperation<'resume'>, options?: CallOptions): Promise<TypedOperationReceipt<'resume'>>;
  prepareFork(requestId: OperationAddress['requestId']): PreparedOperation<'fork'>;
  fork(prepared: PreparedOperation<'fork'>, options?: CallOptions): Promise<TypedOperationReceipt<'fork'>>;
}
export interface CurrentAgent extends AgentReader<CurrentReference> {
  prepareMessage(message: ReadonlyValue<MessageInput>, requestId: OperationAddress['requestId']): PreparedMessage;
  /** Dispatch exactly this command once per call. On response loss, observe prepared.operation. */
  send(prepared: PreparedMessage, options?: CallOptions): Promise<OperationReceipt>;
  prepareKill(requestId: OperationAddress['requestId']): PreparedOperation<'kill'>;
  kill(prepared: PreparedOperation<'kill'>, options?: CallOptions): Promise<TypedOperationReceipt<'kill'>>;
  prepareStopParticipation(requestId: OperationAddress['requestId']): PreparedOperation<'stop-participation'>;
  stopParticipation(prepared: PreparedOperation<'stop-participation'>, options?: CallOptions): Promise<TypedOperationReceipt<'stop-participation'>>;
  labels(options?: CallOptions): Promise<AgentLabels>;
  prepareLabelsPatch(patch: LabelsPatch, requestId: OperationAddress['requestId']): PreparedOperation<'patch-labels'>;
  patchLabels(prepared: PreparedOperation<'patch-labels'>, options?: CallOptions): Promise<TypedOperationReceipt<'patch-labels'>>;
}
export interface StoppedAgent extends AgentReader<StoppedReference> {}
export type Agent = CurrentAgent | StoppedAgent;

export interface Workspace {
  readonly address: WorkspaceAddress;
  roles(options?: CallOptions): Promise<RoleCatalog>;
  /** Read native adopted members; unavailable member paging rejects with UnsupportedCapability. */
  memberPage(page?: NativeMemberRequest, options?: CallOptions): Promise<NativeMemberPage>;
  prepareLaunch(launch: WorkspaceLaunchDraft, requestId: OperationAddress['requestId']): PreparedOperation<'launch'>;
  launch(prepared: PreparedOperation<'launch'>, options?: CallOptions): Promise<TypedOperationReceipt<'launch'>>;
}

export type AttachmentAddress = Readonly<{ scope: ScopeAddress; id: NativeAttachment['id'] }>;
export type Attachment = ReadonlyValue<Omit<NativeAttachment, 'id'>> & Readonly<{ address: AttachmentAddress }>;
export interface Attachments {
  /** Register bytes and metadata. Registration does not send a message. */
  upload(input: Readonly<{ name: string; mediaType?: string; bytes: Blob }>, options?: CallOptions): Promise<Attachment>;
  download(address: AttachmentAddress, options?: CallOptions): Promise<Blob>;
}
export type InboxCursor = Readonly<{ kind: 'human-inbox'; scope: ScopeAddress; pending: boolean; position: string }>;
export type InboxRequest = Readonly<{ before?: InboxCursor; limit?: number; pending?: boolean }>;
export type InboxPage = Readonly<{
  scope: ScopeAddress;
  messages: ReadonlyValue<HumanMailbox['messages']>;
  unhandledCount: HumanMailbox['unhandled_count'];
  next: InboxCursor | null;
}>;
export interface HumanInbox {
  list(page?: InboxRequest, options?: CallOptions): Promise<InboxPage>;
  /** Monotonic, idempotent handling of these explicitly selected message IDs. */
  handle(ids: readonly string[], options?: CallOptions): Promise<InboxPage>;
}
export interface DirectoryRoutes {
  /** Inspect the route without adopting or creating an Agent. */
  inspect(directory: string, options?: CallOptions): Promise<DirectoryPreview>;
  prepareSend(directory: string, message: ReadonlyValue<MessageInput>, requestId: OperationAddress['requestId']): PreparedOperation<'directory-send'>;
  send(prepared: PreparedOperation<'directory-send'>, options?: CallOptions): Promise<TypedOperationReceipt<'directory-send'>>;
}
export type OfficeActivityCursor = Readonly<{ kind: 'office-activity'; scope: ScopeAddress; position: string }>;
export type OfficeActivityPage = ReadonlyValue<Omit<OfficeActivity, 'next_after'>> & Readonly<{ checkpoint: OfficeActivityCursor }>;
export interface AgentXScope {
  readonly address: ScopeAddress;
  readonly descriptor: ReadonlyValue<Descriptor>;
  /** Check the pinned listener's current scope and generation without rebinding this selection. */
  verify(options?: CallOptions): Promise<ReadonlyValue<Descriptor>>;
  workspaces(options?: CallOptions): Promise<WorkspaceCatalog>;
  workspace(key: WorkspaceKey): Workspace;
  /** One scope-wide page, retaining unassigned and historical records. */
  agents(page?: PageRequest<AgentCursor>, options?: CallOptions): Promise<ScopeAgentPage>;
  agent(address: AgentAddress<CurrentReference>): CurrentAgent;
  agent(address: AgentAddress<StoppedReference>): StoppedAgent;
  agent(address: AgentAddress): Agent;
  operation<K extends OperationKind>(prepared: PreparedOperation<K> & { readonly kind: K }): TypedOperationHandle<K>;
  operation(prepared: PreparedMessage): TypedOperationHandle<'send'>;
  operation(prepared: PreparedOperation | PreparedMessage): TypedOperationHandle<OperationKind>;
  operation(address: OperationAddress): OperationHandle;
  prepareLaunch(launch: LaunchDraft, requestId: OperationAddress['requestId']): PreparedOperation<'launch'>;
  launch(prepared: PreparedOperation<'launch'>, options?: CallOptions): Promise<TypedOperationReceipt<'launch'>>;
  readonly attachments: Attachments;
  readonly inbox: HumanInbox;
  readonly directory: DirectoryRoutes;
  /** First read observes a watermark; subsequent pages are accepted addressing, not delivery. */
  officeActivity(page?: PageRequest<OfficeActivityCursor>, options?: CallOptions): Promise<OfficeActivityPage>;
}
export type SelectAgentXScope = (selection: ScopeSelection, options?: CallOptions) => Promise<AgentXScope>;
