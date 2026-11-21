/** Browser projections retain native identity; no display label is a command target. */
import type { AgentRef, AgentView, Descriptor } from '../../../management/api/native';

export type Dataset = Pick<Descriptor, 'scope' | 'generation'>;
export function referenceKey(agent: AgentRef): string {
  return JSON.stringify([agent.name, agent.incarnation, agent.stopped_event]);
}
export function datasetKey(dataset: Dataset): string {
  return JSON.stringify([dataset.scope, dataset.generation]);
}
export function selectionKey(dataset: Dataset, agent: AgentRef): string {
  return JSON.stringify([dataset.scope, dataset.generation, agent.name, agent.incarnation, agent.stopped_event]);
}
export function currentCounts(agents: AgentView[]): Array<{ status: string; count: number }> {
  const counts = new Map<string, number>();
  for (const row of agents) counts.set(row.status, (counts.get(row.status) ?? 0) + 1);
  return [...counts].map(([status, count]) => ({ status, count }));
}
export const isPending = (status: string) => ['Accepted', 'Running', 'Unknown'].includes(status);
export const unknown = (value: unknown): string => value === null || value === undefined ? '未知' : String(value);
