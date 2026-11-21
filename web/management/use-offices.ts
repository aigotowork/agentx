/** @aiPurpose Own generation-fenced office discovery and effective Role reads independently of Agent polling. */
import { useEffect, useState } from 'react';
import type { AgentView, CollaborationView, RoleCatalog, WorkspaceCatalog } from './api/native';
import { management, storageSuffix } from './api/client';
import { messageOf, readStored } from './model';

const workspaceKey = 'agentx.management.workspace.v1' + storageSuffix;
type WorkspaceSelection = { generation: string; workspace: string };

/** @aiPurpose Format saved membership without cwd inference. @aiUsage Use on current and stopped native projections. */
export function membershipLabel(facts: CollaborationView): string {
  if (facts.status === 'legacy') return '旧记录 · 未记录岗位';
  if (facts.status !== 'adopted') return '归属不可用';
  return `${facts.workspace ?? '全局'} · ${facts.role?.name ?? '无岗位'}`;
}

/** @aiPurpose Filter only exact adopted memberships or explicitly requested legacy/unavailable groups. @aiUsage Never use Agent directory as membership. */
export function matchesOffice(agent: AgentView, filter: string): boolean {
  if (filter === 'all') return true;
  const facts = agent.collaboration;
  if (filter === 'legacy') return facts.status === 'legacy';
  if (filter === 'unavailable') return facts.status !== 'adopted' && facts.status !== 'legacy';
  return facts.status === 'adopted' && (filter === 'global' ? facts.workspace === null : facts.workspace === filter);
}

/**
 * @aiPurpose Load finite known offices and native effective Role catalogs.
 * @aiUsage Supply current descriptor generation; refresh after explicit user requests.
 */
export function useOffices(generation: string | undefined, refreshVersion: number) {
  const [filter, setFilter] = useState('all');
  const [selection, setSelection] = useState(() => readStored<WorkspaceSelection | null>(workspaceKey, null));
  const workspace = selection && selection.generation === generation && typeof selection.workspace === 'string' ? selection.workspace : 'global';
  const [catalog, setCatalog] = useState<WorkspaceCatalog>();
  const [roleRead, setRoleRead] = useState<{ workspace: string; value: RoleCatalog }>();
  const [catalogError, setCatalogError] = useState('');
  const [roleError, setRoleError] = useState('');
  const [version, setVersion] = useState(0);
  function setWorkspace(value: string) {
    if (!generation || (value !== 'global' && !value.startsWith('/'))) return;
    const next = { generation, workspace: value };
    setSelection(next);
    try { localStorage.setItem(workspaceKey, JSON.stringify(next)); } catch { /* Navigation remains available in memory. */ }
  }
  useEffect(() => {
    setCatalog(undefined); setCatalogError('');
    if (!generation) return;
    const controller = new AbortController();
    void management.workspaces.list(generation, { signal: controller.signal }).then(value => {
      if (value.generation !== generation) throw new Error('办公室数据代次已变化');
      if (!controller.signal.aborted) setCatalog(value);
    }).catch(error => { if (!controller.signal.aborted) setCatalogError(messageOf(error)); });
    return () => controller.abort();
  }, [generation, refreshVersion, version]);
  useEffect(() => {
    setRoleRead(undefined); setRoleError('');
    if (!generation) return;
    const controller = new AbortController();
    void management.roles.list({ generation, workspace }, { signal: controller.signal }).then(value => {
      if (value.generation !== generation) throw new Error('岗位数据代次已变化');
      if (!controller.signal.aborted) setRoleRead({ workspace, value });
    }).catch(error => { if (!controller.signal.aborted) setRoleError(messageOf(error)); });
    return () => controller.abort();
  }, [generation, workspace, refreshVersion, version]);
  return { filter, choose: setFilter, workspace, setWorkspace, catalog: catalog?.generation === generation ? catalog : undefined, roles: roleRead && roleRead.value.generation === generation && roleRead.workspace === workspace ? roleRead.value : undefined, catalogError, roleError, refresh: () => setVersion(value => value + 1) };
}
export type Offices = ReturnType<typeof useOffices>;
