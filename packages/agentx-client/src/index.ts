/** @aiPurpose Browser-safe public AgentX SDK for independent native HTTP listeners and cloud endpoints. */
import { createManagementClient } from './management.js';
import { createWorkbenchClient } from './workbench.js';
import { createObservationClient } from './observation.js';
export { createManagementClient } from './management.js';
export type { ManagementClient, ManagementTypes } from './management.js';
export { ManagementError, OutcomeUnknown, ReceiptError, ObservationStopped } from './client.js';
export type { ManagementClientOptions, OperationIdentity, ListAgentsInput, WaitOptions } from './client.js';
export { createWorkbenchClient, WorkOutcomeUnknown, participantFromAgent } from './workbench.js';
export type { WorkbenchClient, WorkbenchTypes } from './workbench.js';
export { createObservationClient } from './observation.js';
export type { ObservationClient, ObservationTypes, ObservationOptions } from './observation.js';
export { createCloudAdminClient, CloudAdminError, CloudAdminOutcomeUnknown } from './cloud-admin.js';
export type { CloudAdminClient, CloudAdminOptions, CloudAdminTypes, CloudControlInput, CloudControlReceipt, CloudHostCapabilities, CloudScopePut, CloudScopeRotation } from './cloud-admin.js';
export { createCloudScopeClient, CloudScopeError, CloudScopeOutcomeUnknown } from './cloud-scope.js';
export type { CloudScopeClient, CloudScopeTypes, CloudScopeOptions } from './cloud-scope.js';
export { AdapterError, AdapterDisconnected, MutationOutcomeUnknown, ProtocolError } from './transport.js';
export type { HttpOptions, CallOptions } from './transport.js';
export { parseManagementJson, stringifyManagementJson, parseWorkbenchJson, stringifyWorkbenchJson, parseObservationJson, stringifyObservationJson, stringifyJson, parseJson } from './codec.js';
export type { Native } from './codec.js';
export type { AgentRef, AgentView, AgentPage, Descriptor, MessageInput, SubmitMessage, Operation, OperationStatus, CommandResult, SendOutcome, PublicError, ErrorCode } from './generated/management/types.gen.js';

/** @aiPurpose Select independent endpoints without assuming a shared scope or listener. @aiUsage Supply only the URLs you intend to consume; omitted domains remain undefined. */
export interface AgentXClientOptions {
  managementUrl?: string; workbenchUrl?: string; observationUrl?: string;
  fetch?: typeof fetch; headers?: HeadersInit;
}
/** @aiPurpose Compose the selected public clients without network/startup side effects. @aiUsage No endpoint is inferred from another domain, and no daemon is automatically acquired. */
export function createAgentXClient(options: AgentXClientOptions) {
  const common = { fetch: options.fetch, headers: options.headers };
  return {
    management: options.managementUrl ? createManagementClient({ ...common, baseUrl: options.managementUrl }) : undefined,
    workbench: options.workbenchUrl ? createWorkbenchClient({ ...common, baseUrl: options.workbenchUrl }) : undefined,
    observation: options.observationUrl ? createObservationClient({ ...common, baseUrl: options.observationUrl }) : undefined,
  };
}

export * from './domain/index.js';
