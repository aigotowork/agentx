/** @aiPurpose Derive local label drafts and current-page groups without owning native validation. */
import type { AgentView } from '../api/native';

export interface LabelRow { key: string; value: string }
export interface LabelDraft { rows: LabelRow[]; baseline: Record<string, string>; revision: string }

/** @aiPurpose Read known labels without converting missing historical facts into an empty map. @aiUsage Use null to display unrecorded or unsupported labels distinctly. */
export function labelsOf(agent: AgentView): Record<string, string> | null {
  return agent.labels ?? null;
}

/** @aiPurpose Compute a bounded partial update and reject ambiguous duplicate inputs. @aiUsage Compare with the draft baseline so unrelated remote keys are never replaced wholesale. */
export function labelPatch(draft: LabelDraft) {
  const set: Record<string, string> = Object.create(null);
  const keys = new Set<string>();
  for (const row of draft.rows) {
    if (!row.key && !row.value) continue;
    if (!row.key || !row.value) throw new Error('每个标签需要 key 和非空 value。');
    if (keys.has(row.key)) throw new Error(`重复 key：${row.key}`);
    keys.add(row.key);
    if (draft.baseline[row.key] !== row.value) set[row.key] = row.value;
  }
  const remove = Object.keys(draft.baseline).filter(key => !keys.has(key));
  return { set, remove, if_revision: draft.revision };
}

/** @aiPurpose Partition loaded resources by one opaque scalar value with explicit missing facts. @aiUsage This grouping never implies server-side completeness or a total count. */
export function groupAgents(agents: AgentView[], key: string): { id: string; title: string; agents: AgentView[] }[] {
  if (!key) return [{ id: 'all', title: '', agents }];
  const groups = new Map<string, { id: string; title: string; agents: AgentView[] }>();
  for (const agent of agents) {
    const labels = labelsOf(agent);
    const known = labels !== null && Object.hasOwn(labels, key);
    const id = labels === null ? 'unknown' : known ? `value:${labels[key]}` : 'missing';
    const title = labels === null ? '未记录 / 服务不支持' : known ? labels[key] : '缺失';
    if (!groups.has(id)) groups.set(id, { id, title, agents: [] });
    groups.get(id)!.agents.push(agent);
  }
  return [...groups.values()];
}
