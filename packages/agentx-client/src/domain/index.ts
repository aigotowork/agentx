/** Browser-safe scoped domain object entry. */
import { createManagementClient } from '../management.js';
import { freezeInput } from '../transport.js';
import { DomainReferenceError, normalizeListener, requestId, requireGeneration, requireScope } from './address.js';
import { bindAgent, projectSnapshot } from './agent.js';
import type { Agent, AgentAddress, AgentXScope, CurrentAgent, CurrentReference, OperationAddress, OperationHandle, OperationKind, PreparedMessage, PreparedOperation, ScopeAddress, SelectAgentXScope, StoppedAgent, StoppedReference, TypedOperationHandle } from './contracts.js';
import { bindPrepared } from './operation.js';
import { bindLaunch } from './launch.js';
import { bindAttachments, bindDirectory, bindInbox, readOfficeActivity } from './resources.js';
import { bindWorkspace, discoverWorkspaces } from './workspace.js';

export type * from './contracts.js';
export { DomainReferenceError } from './address.js';

/** Pin one listener, expected native scope and observed generation without later implicit refreshes. */
export const selectAgentXScope: SelectAgentXScope = async (selection, options) => {
  const { baseUrl, expectedScope, fetch, headers } = selection;
  if (typeof expectedScope !== 'string' || !expectedScope) throw new TypeError('Supply the expected native management scope');
  const listener = normalizeListener(baseUrl);
  const client = createManagementClient({ baseUrl: listener, fetch, headers });
  async function describe(opts?: Parameters<typeof client.describe>[0]) {
    const descriptor = await client.describe(opts);
    if (descriptor.scope !== expectedScope) {
      throw new DomainReferenceError('ScopeMismatch', 'Management listener does not describe the expected native scope');
    }
    return freezeInput(descriptor);
  }
  const descriptor = await describe(options);
  const address: ScopeAddress = Object.freeze({ baseUrl: listener, scope: descriptor.scope, generation: descriptor.generation });
  const supportsWorkspaceMembers = descriptor.supports_workspace_members === true;

  function agent(target: AgentAddress<CurrentReference>): CurrentAgent;
  function agent(target: AgentAddress<StoppedReference>): StoppedAgent;
  function agent(target: AgentAddress): Agent;
  function agent(target: AgentAddress): Agent {
    requireScope(target.scope, address);
    return bindAgent(client, address, target.reference);
  }

  function operation<K extends OperationKind>(prepared: PreparedOperation<K> & { readonly kind: K }): TypedOperationHandle<K>;
  function operation(prepared: PreparedMessage): TypedOperationHandle<'send'>;
  function operation(prepared: PreparedOperation | PreparedMessage): TypedOperationHandle<OperationKind>;
  function operation(target: OperationAddress): OperationHandle;
  function operation(target: OperationAddress | PreparedOperation | PreparedMessage): OperationHandle | TypedOperationHandle<OperationKind> {
    if ('operation' in target) return bindPrepared(client, address, target);
    requireScope(target.scope, address);
    const id = requestId(target.requestId);
    const original = { generation: address.generation, request_id: id };
    return Object.freeze({
      address: Object.freeze({ scope: address, requestId: id }),
      observe: async (opts?: Parameters<typeof client.getOperation>[1]) => freezeInput(await client.getOperation(original, opts)),
      wait: async (opts?: Parameters<typeof client.waitForOperation>[1]) => freezeInput(await client.waitForOperation(original, opts)),
    });
  }

  const selected: AgentXScope = {
    address,
    descriptor,
    supportsWorkspaceMembers,
    async verify(opts) {
      const current = await describe(opts);
      requireGeneration(current.generation, address);
      return current;
    },
    ...bindLaunch(client, address),
    attachments: bindAttachments(client, address),
    inbox: bindInbox(client, address),
    directory: bindDirectory(client, address),
    officeActivity: (page, opts) => readOfficeActivity(client, address, page, opts),
    workspaces: opts => discoverWorkspaces(client, address, supportsWorkspaceMembers, opts),
    workspace: key => bindWorkspace(client, address, key, supportsWorkspaceMembers),
    async agents(page = {}, options) {
      if (page.after) requireScope(page.after.scope, address);
      const result = await client.agents.list({ generation: address.generation, after: page.after?.position, limit: page.limit }, options);
      requireGeneration(result.generation, address);
      return freezeInput({ agents: result.agents.map(view => projectSnapshot(address, view)), currentComplete: result.current_complete,
        next: result.next == null ? null : { scope: address, position: result.next } });
    },
    agent,
    operation,
  };
  return Object.freeze(selected);
};
