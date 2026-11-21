/** @aiPurpose Presentation-local supplied facts; these declarations are not AgentX HTTP contracts. */
import type { AgentRef, Attachment, FileBindingView } from "../../../management/api/native";
export type AgentState = "running" | "idle" | "ready" | "closed" | "failed" | "unknown";
export type Membership = { kind: "directory"; path: string } | { kind: "global" } | { kind: "unknown" } | { kind: "legacy" };
export interface Office { id: string; title: string; origin: "role-marker" | "explicit-membership"; sources?: readonly string[]; }
export interface Post { id: string; office: string; name: string; origin: "本地" | "继承" | "全局"; source: string; description: string; profile: string; revision: string; }
export type AdoptedRole = { kind: "adopted"; name: string; source: string; profile: string; revision: string; description?: string | null } | { kind: "none" } | { kind: "unknown" };
export interface AgentResource {
  fileBindings?: readonly FileBindingView[];
  id: string; name: string; title: string; launchTitle?: string | null; nativeStatus?: string; statusContext?: string; storedStatus?: string; requestError?: { reason: string; source: string } | null; pendingMessages?: number | bigint | null; arrival?: string; description: string; state: AgentState; attention: boolean;
  ref: { scope: string; generation: string; name: string; incarnation: string | null; stopped_event?: string | null; record: "current" | "stopped" };
  membership: Membership; role: AdoptedRole; cwd: string | null; model: string | null; tool: string; profile: string | null;
  source: { id: string; title: string }; createdAt: string;
}
export type HistoryRecord =
  | { id: string; kind: "message"; role: "user" | "assistant"; text: string; at: string; author: string; attachments?: readonly Attachment[]; recipients?: readonly string[]; thread?: string; replyTo?: string; intent?: string; native?: boolean }
  | { id: string; kind: "reasoning"; text: string }
  | { id: string; kind: "tool"; name: string; summary: string; text: string; status: "completed" | "failed" };
export type RecordChannel = "messages" | "model";
export type OfficeView = "members" | "posts" | "skills" | "history";
export type SuppliedRecords = { status: "loading" } | { status: "available"; provenance: string; items: readonly HistoryRecord[] } | { status: "unavailable" | "error"; reason: string };
export interface HumanMessage { id: string; at: string; from: string; text: string; handled_at?: string | null; sender?: AgentRef | null }
export interface HumanFeed {
  messages: readonly HumanMessage[]; hasMore: boolean; error?: string; actionError?: string;
  nextBefore?: string; before?: string; unhandledCount?: number | bigint; pending?: boolean; older?: boolean; busy?: boolean;
  openSender?(reference: AgentRef): Promise<AgentResource>;
  handle?(ids: readonly string[]): void;
  history?(pending: boolean): void;
  page?(older: boolean): void;
}
export const HUMAN_ENDPOINT = "$human:bigboss";
export interface SceneMessage { id: string; from: string; to: string; receivedAt: number }
export type RosterCoverage = {
  office: string; phase: "loading" | "ready" | "error"; error?: string;
  more: boolean; exhausted: boolean; emptyPage: boolean;
} & ({ kind: "native-workspace-members" } | { kind: "global-agent-page"; scanned: number; legacy: number; unavailable: number });
export interface AgentSpaceSupply {
  readonly roster?: RosterCoverage;
  /** Exact reader-only subjects never contribute to roster counts, filters or seats. */
  readonly subjects?: readonly AgentResource[];
  readonly human?: HumanFeed;
  readonly population?: { revision: number; complete: boolean };
  readonly activity?: readonly SceneMessage[];
  readonly mode: "offline" | "live";
  readonly scope: { id: string; generation: string; listener?: string; coverage: string };
  readonly workspaces: readonly Office[];
  readonly posts: readonly Post[];
  readonly agents: readonly AgentResource[];
  records(agentId: string, channel: RecordChannel): SuppliedRecords;
}
export const GLOBAL_OFFICE = "$global", UNKNOWN_OFFICE = "$unknown", LEGACY_OFFICE = "$legacy";
/** Exact supplied membership; unknown never inherits the global bucket. */
export function membershipKey(agent: AgentResource): string {
  return agent.membership.kind === "directory" ? agent.membership.path : agent.membership.kind === "global" ? GLOBAL_OFFICE : agent.membership.kind === "legacy" ? LEGACY_OFFICE : UNKNOWN_OFFICE;
}
export function adoptedRoleLabel(role: AdoptedRole): string { return role.kind === "adopted" ? role.name : role.kind === "none" ? "未采用 Role" : "岗位未知"; }
export function exactAgentKey(ref: AgentResource["ref"]): string { return JSON.stringify([ref.scope, ref.generation, ref.name, ref.incarnation, ref.stopped_event ?? null, ref.record]); }

export function findConversation(supply: AgentSpaceSupply, id: string | null) {
  return supply.subjects?.find(agent => agent.id === id) ?? supply.agents.find(agent => agent.id === id);
}
export function conversationSubjects(supply: AgentSpaceSupply): readonly AgentResource[] {
  return supply.subjects?.length ? [...new Map([...supply.agents, ...supply.subjects].map(agent => [agent.id, agent])).values()] : supply.agents;
}
