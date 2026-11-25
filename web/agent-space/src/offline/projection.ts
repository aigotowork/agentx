/** @aiPurpose Exact membership and current/stopped projections shared by scene, navigation and resource readers. */
import { adoptedRoleLabel, matchesOffice, membershipKey, type AgentResource, type AgentSpaceSupply, type Post } from "./model";
import type { SpaceSnapshot } from "./controller";
export function officeMembers(supply: AgentSpaceSupply, office: string): readonly AgentResource[] {
  return supply.agents.filter(agent => matchesOffice(agent, office));
}
export function currentMembers(agents: readonly AgentResource[]): readonly AgentResource[] { return agents.filter(agent => agent.ref.record === "current"); }
export function matchingPostMembers(supply: AgentSpaceSupply, post: Post): readonly AgentResource[] {
  return currentMembers(officeMembers(supply, post.office)).filter(agent => agent.role.kind === "adopted" && agent.role.source === post.source);
}
export function visibleMembers(supply: AgentSpaceSupply, state: SpaceSnapshot): readonly AgentResource[] {
  const query = state.query.trim().toLocaleLowerCase();
  return officeMembers(supply, state.workspaceId).filter(agent => {
    const correctRecord = agent.ref.record === (state.officeView === "history" ? "stopped" : "current");
    const text = [agent.title, agent.description, membershipKey(agent), adoptedRoleLabel(agent.role), agent.model].join(" ").toLocaleLowerCase();
    return correctRecord && (!state.state || agent.state === state.state) && (!query || text.includes(query));
  });
}
