/** Workspace discovery, effective Roles and separately bound native membership pages. */
import type { ManagementClient } from '../management.js';
import { freezeInput } from '../transport.js';
import type { CallOptions } from '../transport.js';
import { DomainReferenceError, requireGeneration, requireWorkspace } from './address.js';
import { bindAgent, projectSnapshot } from './agent.js';
import type { NativeMemberPage, NativeMemberRequest, RoleCatalog, ScopeAddress, Workspace, WorkspaceCatalog, WorkspaceKey } from './contracts.js';
import { bindLaunch } from './launch.js';
import { readWhenSupported, type ReadCapabilities } from './read-capabilities.js';

export async function discoverWorkspaces(client: ManagementClient, scope: ScopeAddress, capabilities: ReadCapabilities, options?: CallOptions): Promise<WorkspaceCatalog> {
  const result = await client.workspaces.list(scope.generation, options);
  requireGeneration(result.generation, scope);
  return freezeInput({
    workspaces: result.workspaces.map(row => ({ workspace: bindWorkspace(client, scope, capabilities, row.path), sources: row.sources })),
    coverage: result.coverage,
  });
}

export function bindWorkspace(client: ManagementClient, scope: ScopeAddress, capabilities: ReadCapabilities, key: WorkspaceKey): Workspace {
  if (typeof key !== 'string' || !key || key.includes('\0')) {
    throw new DomainReferenceError('InvalidReference', 'Supply a canonical host path or the explicit global Workspace key');
  }
  const address = Object.freeze({ scope, key });
  const workspace: Workspace = {
    address,
    ...bindLaunch(client, scope, key),
    async roles(options?: CallOptions): Promise<RoleCatalog> {
      const result = await client.roles.list({ generation: scope.generation, workspace: key }, options);
      requireGeneration(result.generation, scope);
      if (key === 'global' ? result.workspace !== null : !result.workspace || result.workspace === 'global') {
        throw new DomainReferenceError('InconsistentResponse', 'Role catalog did not identify the selected workspace');
      }
      return freezeInput({
        workspace: result.workspace == null || result.workspace === key ? workspace : bindWorkspace(client, scope, capabilities, result.workspace),
        roles: result.roles.map(definition => ({ definition })),
      });
    },
    async memberPage(page: NativeMemberRequest = {}, options?: CallOptions): Promise<NativeMemberPage> {
      const currentOnly = page.currentOnly ?? false;
      if (page.after) {
        if (page.after.kind !== 'workspace-members') throw new DomainReferenceError('InvalidReference', 'Expected a native Workspace membership cursor');
        requireWorkspace(page.after.workspace, address);
        if (page.after.currentOnly !== currentOnly) throw new DomainReferenceError('ReferenceMismatch', 'Cursor belongs to a different member selector');
      }
      const result = await readWhenSupported(capabilities, 'workspace-members',
        () => client.workspaces.members({ generation: scope.generation, workspace: key, current_only: currentOnly, after: page.after?.position, limit: page.limit }, options));
      requireGeneration(result.generation, scope);
      const members = result.agents.map(view => {
        const snapshot = projectSnapshot(scope, view);
        if (snapshot.membership.kind !== 'adopted' || snapshot.membership.workspace !== key || currentOnly && snapshot.address.reference.kind !== 'current') {
          throw new DomainReferenceError('InconsistentResponse', 'Native member does not match the selected Workspace and member selector');
        }
        return { agent: bindAgent(client, scope, capabilities, snapshot.address.reference), snapshot };
      });
      return freezeInput({ members, coverage: { kind: 'native-workspace-members', currentOnly, currentComplete: result.current_complete,
        next: result.next == null ? null : { kind: 'workspace-members', workspace: address, currentOnly, position: result.next } } });
    },
  };
  return Object.freeze(workspace);
}
