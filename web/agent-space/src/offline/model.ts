/** @aiPurpose Presentation-local supplied facts; these declarations are not AgentX HTTP contracts. */
import type { AgentRef, Attachment, FileBindingView } from "../../../management/api/native";
export type AgentState = "running" | "idle" | "ready" | "closed" | "failed" | "unknown";
export type Membership = { kind: "directory"; path: string } | { kind: "project"; host: string; workspaceId: string } | { kind: "global" } | { kind: "unexposed" } | { kind: "unknown" } | { kind: "legacy" };
export interface Office { id: string; title: string; origin: "role-marker" | "explicit-membership" | "cloud-project"; sources?: readonly string[]; }
export type CloudRoleOption = { name: string; available: boolean; reason: string | null };
export type CloudOffice = Office & { origin: "cloud-project"; host: string; workspaceId: string; online: boolean; roles: readonly CloudRoleOption[] };
export type CloudOfficeDirectory = { phase: "loading" | "ready" | "error"; hosts: readonly { name: string; online: boolean }[]; projects: readonly CloudOffice[] };
export interface Post { id: string; office: string; name: string; origin: "本地" | "继承" | "全局"; source: string; description: string; profile: string; revision: string; }
export type AdoptedRole = { kind: "adopted"; name: string; source: string; profile: string; revision: string; description?: string | null } | { kind: "none" } | { kind: "unknown" };
export interface AgentResource {
  fileBindings?: readonly FileBindingView[];
  id: string; name: string; title: string; launchTitle?: string | null; nativeStatus?: string; statusContext?: string; storedStatus?: string; requestError?: { reason: string; source: string } | null; pendingMessages?: number | bigint | null; arrival?: string; description: string; state: AgentState; attention: boolean;
  ref: { scope: string; generation: string; host?: string; name: string; incarnation: string | null; stopped_event?: string | null; record: "current" | "stopped" };
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
  office: string; phase: "loading" | "ready" | "error" | "unavailable"; error?: string;
  more: boolean;
};
export interface AgentSpaceSupply {
  readonly roster?: RosterCoverage;
  /** Exact reader-only subjects never contribute to roster counts, filters or seats. */
  readonly subjects?: readonly AgentResource[];
  readonly human?: HumanFeed;
  readonly population?: { revision: number; complete: boolean };
  readonly activity?: readonly SceneMessage[];
  readonly cloudDirectory?: CloudOfficeDirectory;
  readonly mode: "offline" | "live";
  readonly scope: { cloud?: true; id: string; generation: string; listener?: string; coverage: string };
  readonly workspaces: readonly Office[];
  readonly posts: readonly Post[];
  readonly agents: readonly AgentResource[];
  records(agentId: string, channel: RecordChannel): SuppliedRecords;
}
export const hostGroup = (host: string) => `host:${host}`;
export const projectOffice = (host: string, workspaceId: string) => `project:${host}:${workspaceId}`;
/** Disambiguate equal display names on one host with only as much of the opaque ID as needed. */
export function projectDisplayName(name: string, id: string, sameNameIds: readonly string[]): string {
  if (sameNameIds.length < 2) return name;
  let length = 8;
  while (length < id.length && sameNameIds.some(other => other !== id && other.startsWith(id.slice(0, length)))) length++;
  return `${name} · ${id.slice(0, length)}`;
}
export const UNEXPOSED_OFFICE = "$unexposed";
export const GLOBAL_OFFICE = "$global", UNKNOWN_OFFICE = "$unknown", LEGACY_OFFICE = "$legacy";
/** Exact supplied membership; unknown never inherits the global bucket. */
export function membershipKey(agent: AgentResource): string {
  const membership = agent.membership;
  return membership.kind === "directory" ? membership.path : membership.kind === "project" ? projectOffice(membership.host, membership.workspaceId)
    : membership.kind === "global" ? GLOBAL_OFFICE : membership.kind === "unexposed" ? UNEXPOSED_OFFICE : membership.kind === "legacy" ? LEGACY_OFFICE : UNKNOWN_OFFICE;
}

/** Host filters can include every membership kind but never assign an Agent to a project. */
export function matchesOffice(agent: AgentResource, office: string): boolean {
  return !office || (office.startsWith("host:") ? !!agent.ref.host && hostGroup(agent.ref.host) === office : membershipKey(agent) === office);
}
export function adoptedRoleLabel(role: AdoptedRole): string { return role.kind === "adopted" ? role.name : role.kind === "none" ? "未采用 Role" : "岗位未知"; }
export function exactAgentKey(ref: AgentResource["ref"]): string { return JSON.stringify([ref.scope, ref.generation, ref.name, ref.incarnation, ref.stopped_event ?? null, ref.record, ...(ref.host === undefined ? [] : [ref.host])]); }

export function findConversation(supply: AgentSpaceSupply, id: string | null) {
  return supply.subjects?.find(agent => agent.id === id) ?? supply.agents.find(agent => agent.id === id);
}
export function conversationSubjects(supply: AgentSpaceSupply): readonly AgentResource[] {
  return supply.subjects?.length ? [...new Map([...supply.agents, ...supply.subjects].map(agent => [agent.id, agent])).values()] : supply.agents;
}
