import { t } from "./text";
import { completionReadState, type AgentRecord } from "./model";
export let ARCHIVE_PARTIAL_LIST = t("sharedSchedules.m006");
/** `truncated`: `agents` is not every unarchived Agent (the directory read was cut short, its `truncated`). */
export interface ArchiveListState { truncated?: boolean }
export interface ArchiveOffer {
  candidates: AgentRecord[];
  
  withheld: number;
  /** Why the `withheld` sessions wait, in one Chinese sentence; absent when none do. */
  reason?: string;
  
  sparedForChildren: number;
}
const LIVE_STATUSES: ReadonlySet<string> = new Set(["running", "waiting", "initializing"]);
/** A session 归档已读完成 may take on its own account: completed, its result read, no stop to come back for. */
const settledAlone = (agent: AgentRecord) => !agent.archivedAt && agent.status === "completed" && completionReadState(agent) === "read" && !agent.stop;

function archiveReview(agents: readonly AgentRecord[], scope: readonly AgentRecord[]) {
  const byId = new Map(agents.map(agent => [agent.id, agent])), children = new Map<string, AgentRecord[]>();
  for (const agent of agents) if (agent.parentAgentId && !agent.archivedAt) { const list = children.get(agent.parentAgentId); if (list) list.push(agent); else children.set(agent.parentAgentId, [agent]); }
  const ancestors = (agent: AgentRecord) => {
    const ids: string[] = [];
    for (let parent = agent.parentAgentId; parent && !ids.includes(parent); parent = byId.get(parent)?.parentAgentId ?? null) ids.push(parent);
    return ids;
  };
  const live = new Set(agents.filter(agent => LIVE_STATUSES.has(agent.status)).map(agent => agent.id));
  // Anything running anywhere below keeps its whole line of ancestors; anything running above keeps its descendants.
  const busy = new Set([...live].flatMap(id => [id, ...ancestors(byId.get(id)!)]));
  const settled = scope.filter(agent => settledAlone(agent) && !ancestors(agent).some(id => live.has(id)));
  /** Breadth first through unarchived children (a cycle of parent labels ends where it began): one not settled, or woken by `targets`, stays. */
  const keptBelow = (agent: AgentRecord, targets: ReadonlySet<string>) => {
    if (busy.has(agent.id)) return true;
    const seen = new Set([agent.id]), queue = [agent.id];
    for (let index = 0; index < queue.length; index++) for (const child of children.get(queue[index]!) ?? []) {
      if (seen.has(child.id)) continue;
      if (!settledAlone(child) || targets.has(child.id)) return true;
      seen.add(child.id); queue.push(child.id);
    }
    return false;
  };
  return { settled, keptBelow };
}

export function archiveReadCandidates(agents: readonly AgentRecord[], scope: readonly AgentRecord[], list: ArchiveListState = {}): ArchiveOffer {
  const { settled, keptBelow } = archiveReview(agents, scope), targets = new Set<string>();
  const own = settled.filter(agent => !targets.has(agent.id)), clear = own.filter(agent => !keptBelow(agent, targets));
  const sparedForChildren = own.length - clear.length, reason = list.truncated ? ARCHIVE_PARTIAL_LIST : null;
  if (reason !== null) return { candidates: [], withheld: clear.length, ...(clear.length ? { reason } : {}), sparedForChildren };
  return { candidates: clear, withheld: 0, sparedForChildren };
}

export function confirmArchive(marked: ReadonlySet<string>, agents: readonly AgentRecord[], scope: readonly AgentRecord[], list: ArchiveListState = {}):
  { ok: false; reason: string } | { ok: true; ids: string[]; spared: number; sparedForChildren: number; changed: number } {
  if (list.truncated) return { ok: false, reason: ARCHIVE_PARTIAL_LIST };
  const { settled, keptBelow } = archiveReview(agents, scope), ids: string[] = [];
  let still = 0, spared = 0, sparedForChildren = 0;
  for (const agent of settled) {
    if (!marked.has(agent.id)) continue;
    still++;
    if (keptBelow(agent, new Set())) sparedForChildren++;
    else ids.push(agent.id);
  }
  return { ok: true, ids, spared, sparedForChildren, changed: marked.size - still };
}

