/** Provider-independent, read-only management projections. A snapshot belongs to one authority. */
export type SourceStatus = 'available' | 'partial' | 'unavailable' | 'forbidden' | 'unsupported' | 'unconfigured';
export type SourceIssue = 'read_failed' | 'timeout' | 'denied' | 'unsupported' | 'identity_mismatch' | 'invalid_config' | null;
export type Observed<T> = { status: SourceStatus; items: T[]; observedAt: string | null; truncated: boolean; gap: boolean; issue: SourceIssue };
export type BotObservation = { id: string; name: string; title: string; provider: string; model: string | null; archived: boolean };
export type TeamObservation = { id: string; name: string; leadId: string | null; memberIds: string[] };
export type ConversationState = 'pending' | 'running' | 'idle' | 'succeeded' | 'failed' | 'cancelled' | 'unknown';
export type ConversationObservation = { id: string; botId: string; teamId: string | null; title: string; status: ConversationState; updatedAt: string; attention: boolean | null };
export type RuntimeObservation = { id: string; serverId: string; generation: string; title: string; mode: 'managed' | 'attached'; health: 'ready' | 'disconnected' | 'unavailable' | 'identity-mismatch'; partial: boolean; observedAt: string };
export type AgentObservation = { id: string; runtimeId: string; serverId: string; generation: string; title: string; provider: string; model: string | null; ownership: 'ordinary' | 'foreign'; state: 'running' | 'idle' | 'ready' | 'closed' | 'failed' | 'unknown'; attention: boolean; updatedAt: string };
export type BotsSnapshot = { observedAt: string; authorityId: string | null; connection: SourceStatus; issue: SourceIssue; bots: Observed<BotObservation>; teams: Observed<TeamObservation>; conversations: Observed<ConversationObservation>; runtimes: Observed<RuntimeObservation>; agents: Observed<AgentObservation> };
export type RoutineObservation = { id: string; name: string; enabled: boolean; schedule: string };
export type RunObservation = { id: string; routineId: string; status: 'pending' | 'accepted' | 'running' | 'unknown' | 'succeeded' | 'failed' | 'cancelled' | 'rejected' | 'skipped-busy' | 'skipped-missed'; trigger: 'manual' | 'schedule' | 'webhook'; createdAt: string; settledAt: string | null; conversationId: string | null; attemptId: string | null };
export type AutomationSnapshot = { authorityId: string | null; botId: string; observedAt: string; scheduler: boolean | null; routines: Observed<RoutineObservation>; runs: Observed<RunObservation> };

export function readable(source: Observed<unknown>): boolean { return source.status === 'available' || source.status === 'partial'; }
export function exact(source: Observed<unknown>): boolean { return source.status === 'available' && !source.truncated && !source.gap; }
/** Counts cover loaded records only; unavailable facts remain unknown, never zero. */
export function loadedCount<T>(source: Observed<T>, predicate: (item: T) => boolean = () => true): number | null {
  return readable(source) ? source.items.filter(predicate).length : null;
}
export function botLoad(snapshot: BotsSnapshot, botId: string) {
  return { running: loadedCount(snapshot.conversations, c => c.botId === botId && c.status === 'running'), failed: loadedCount(snapshot.conversations, c => c.botId === botId && c.status === 'failed'), total: loadedCount(snapshot.conversations, c => c.botId === botId) };
}
/** A stored teamId records Conversation attribution; current membership is a different relationship. */
export function teamLoad(snapshot: BotsSnapshot, teamId: string) {
  return { running: loadedCount(snapshot.conversations, c => c.teamId === teamId && c.status === 'running'), total: loadedCount(snapshot.conversations, c => c.teamId === teamId) };
}
export function unassignedBots(snapshot: BotsSnapshot): BotObservation[] | null {
  if (!exact(snapshot.teams) || !readable(snapshot.bots)) return null;
  const members = new Set(snapshot.teams.items.flatMap(t => t.memberIds));
  return snapshot.bots.items.filter(b => !b.archived && !members.has(b.id));
}
export function emptyObservation<T>(status: SourceStatus = 'unconfigured', issue: SourceIssue = null): Observed<T> {
  return { status, issue, items: [], observedAt: null, truncated: false, gap: false };
}
export function emptyBotsSnapshot(at: string, status: SourceStatus = 'unconfigured', issue: SourceIssue = null): BotsSnapshot {
  return { observedAt: at, authorityId: null, connection: status, issue, bots: emptyObservation(status, issue), teams: emptyObservation(status, issue), conversations: emptyObservation(status, issue), runtimes: emptyObservation(status, issue), agents: emptyObservation(status, issue) };
}
/** Consumer-owned port. SDK transport and lifecycle are supplied at the composition root. */
export interface BotsObservationProvider {
  snapshot(): Promise<BotsSnapshot>;
  automation(botId: string, authorityId: string): Promise<AutomationSnapshot>;
  close(): void;
}
