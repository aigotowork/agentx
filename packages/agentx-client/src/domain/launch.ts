/** Explicit scope launch; an optional Workspace constrains the original selector. */
import type { ManagementClient } from '../management.js';
import { freezeInput } from '../transport.js';
import type { CallOptions } from '../transport.js';
import { DomainReferenceError } from './address.js';
import type { LaunchDraft, PreparedOperation, ScopeAddress, WorkspaceKey } from './contracts.js';
import { checkedReceipt, commandIdentity, launchResult, operationAddress, retainInput } from './operation.js';

export function bindLaunch(client: ManagementClient, scope: ScopeAddress, workspace?: WorkspaceKey) {
  return {
    prepareLaunch(draft: LaunchDraft, id: string): PreparedOperation<'launch'> {
      const operation = operationAddress(scope, id);
      const launch = workspace === undefined ? draft : { ...draft, workspace };
      const retained = retainInput('SubmitLaunch', { ...commandIdentity(operation, scope), launch });
      return freezeInput({ kind: 'launch', operation, launch: retained.launch });
    },
    async launch(prepared: PreparedOperation<'launch'>, options?: CallOptions) {
      if (prepared.kind !== 'launch') throw new TypeError('Expected a prepared launch');
      const identity = commandIdentity(prepared.operation, scope);
      if (workspace !== undefined && prepared.launch.workspace !== workspace) {
        throw new DomainReferenceError('ReferenceMismatch', 'Prepared launch belongs to a different Workspace');
      }
      const receipt = await client.agents.launch({ ...identity, launch: prepared.launch }, options);
      return checkedReceipt('launch', operationAddress(scope, identity.request_id), receipt, launchResult(scope));
    },
  };
}
