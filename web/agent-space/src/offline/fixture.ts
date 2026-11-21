/** @aiPurpose Explicit synthetic directory offices and exact Agent incarnations; no transport or simulated reply. */
import { exactAgentKey, type AgentResource, type AgentSpaceSupply, type AdoptedRole, type HistoryRecord, type Membership, type Post, type SuppliedRecords } from "./model";
const scope = { id: "/example/agentx-data", generation: "sample-generation-7", coverage: "合成样例 · 已知目录集，非全盘发现" };
const atlas = "/projects/atlas", web = "/projects/atlas/web", empty = "/projects/empty";
const fronterSource = `${atlas}/.agentx/roles/fronter.toml`, reviewerSource = `${scope.id}/roles/reviewer.toml`;
const inheritedRole: AdoptedRole = { kind: "adopted", name: "fronter", source: fronterSource, profile: "codex-deep-at-launch", revision: "snapshot-1" };
const reviewerRole: AdoptedRole = { kind: "adopted", name: "reviewer", source: reviewerSource, profile: "review-deep", revision: "snapshot-1" };
const directory = (path: string): Membership => ({ kind: "directory", path });
function member(name: string, membership: Membership, role: AdoptedRole, state: AgentResource["state"], options: Partial<Pick<AgentResource, "cwd" | "model" | "description" | "profile">> = {}, stopped = false): AgentResource {
  const ref: AgentResource["ref"] = { scope: scope.id, generation: stopped ? "sample-generation-6" : scope.generation, name, incarnation: stopped ? "incarnation-old-1" : `incarnation-${name}-2`, record: stopped ? "stopped" : "current" };
  return { id: exactAgentKey(ref), ref, name, title: `@${name}`, membership, role, state, attention: false,
    description: stopped ? "停止的旧实例 · 任务结果未提供" : "当前实例 · 合成样例", cwd: membership.kind === "directory" ? membership.path : null,
    model: null, tool: "codex", profile: role.kind === "adopted" ? role.profile : null,
    source: { id: scope.id, title: "合成样例" }, createdAt: stopped ? "2026-01-01T08:00:00Z" : "2026-01-02T09:00:00Z", ...options };
}
const agents = [
  member("luna", directory(web), inheritedRole, "running", { model: "sample-observed-model", description: "继承岗位 · 保留启动快照" }),
  member("mora", directory(web), inheritedRole, "idle", { cwd: `${web}/src`, description: "同一继承岗位 · 独立实例" }),
  member("sira", directory(web), reviewerRole, "ready", { description: "全局来源岗位 · 本地目录成员" }),
  member("nora", directory(web), { kind: "none" }, "idle", { profile: "codex-basic", description: "已确认未采用 Role" }),
  member("riku", directory(web), { kind: "unknown" }, "failed", { description: "岗位事实缺失 · 失败不等于停止" }),
  member("tobi", directory(atlas), { kind: "none" }, "running", { cwd: web, description: "父目录成员 · cwd 位于子目录" }),
  member("gavo", { kind: "global" }, reviewerRole, "idle", { cwd: web, description: "全局成员 · cwd 不决定归属" }),
  member("keni", { kind: "unknown" }, { kind: "unknown" }, "unknown", { cwd: web, description: "成员归属未知 · 不归入全局" }),
  member("mora", directory(web), inheritedRole, "closed", {}, true),
];
function postsFor(office: string): Post[] {
  const local = office === atlas;
  return [
    { id: `${office}:fronter`, office, name: "fronter", origin: local ? "本地" : "继承", source: fronterSource, description: "当前岗位定义；现有成员仍保留各自启动快照。", profile: "codex-deep-current", revision: "definition-2" },
    { id: `${office}:controller`, office, name: "controller", origin: local ? "本地" : "继承", source: `${atlas}/.agentx/roles/controller.toml`, description: "可用岗位；未提供目录入口绑定。", profile: "codex-controller", revision: "definition-1" },
    { id: `${office}:reviewer`, office, name: "reviewer", origin: "全局", source: reviewerSource, description: "全局来源的可用岗位；来源不决定成员归属。", profile: "review-deep", revision: "definition-1" },
  ];
}
const posts: Post[] = [...postsFor(atlas), ...postsFor(web), { id: `${empty}:reviewer`, office: empty, name: "reviewer", origin: "全局", source: reviewerSource, description: "空办公室仍有可用岗位。", profile: "review-deep", revision: "definition-1" }];
function messages(agent: AgentResource): SuppliedRecords {
  if (agent.name === "keni") return { status: "unavailable", reason: "此精确实例的 AgentX 消息未提供" };
  if (agent.name !== "luna" && agent.name !== "mora") return { status: "available", provenance: "合成 AgentX 消息 · 当前精确实例区间", items: [] };
  const prefix = agent.ref.record === "stopped" ? "旧实例" : "当前实例";
  const items: HistoryRecord[] = [
    { id: `${agent.id}:msg-1`, kind: "message", role: "user", author: "@tobi", recipients: [agent.title], intent: "request", thread: "sample-layout", text: `${prefix}：请记录目录办公室的阅读场景。`, at: "2026-01-02T09:10:00Z" },
    { id: `${agent.id}:msg-2`, kind: "message", role: "assistant", author: agent.title, recipients: ["@tobi"], intent: "ack", thread: "sample-layout", replyTo: `${agent.id}:msg-1`, text: `${prefix}的静态示例记录：将核对成员、岗位与独立草稿。此内容未从真实 Agent 读取。`, at: "2026-01-02T09:12:00Z" },
  ];
  return { status: "available", provenance: `合成 AgentX 消息 · ${prefix} · 非邮箱消费`, items };
}
function modelRecords(agent: AgentResource): SuppliedRecords {
  if (agent.name === "riku") return { status: "error", reason: "示例读取失败：Provider 记录不可读；未重试或执行真实请求" };
  if (agent.name === "sira") return { status: "available", provenance: "codex · sample-provider-scope · session-sira · 空记录样例", items: [] };
  if (agent.name !== "luna") return { status: "unavailable", reason: "Provider Transcript 未提供；不能由 AgentX 消息推断" };
  return { status: "available", provenance: "codex · sample-provider-scope · session-luna · 合成记录，可含更早执行", items: [
    { id: "model-u1", kind: "message", role: "user", author: "Provider · user", text: "查看离线办公室的紧凑阅读布局。", at: "2026-01-02T09:10:00Z" },
    { id: "model-r1", kind: "reasoning", text: "合成思考块：分别检查目录成员归属与工作目录。" },
    { id: "model-t1", kind: "tool", name: "示例布局检查", summary: "静态工具块", status: "completed", text: "synthetic: true\nnetwork_calls: 0\n\n工具完成不代表任务验收。" },
    { id: "model-a1", kind: "message", role: "assistant", author: "Provider · assistant", text: "### 合成 Transcript\n\n- 目录成员与 cwd 分开。\n- 当前岗位定义与启动快照分开。\n\n`sample only`", at: "2026-01-02T09:12:00Z" },
  ] };
}
/** @aiPurpose Supply all offline semantic cases without inspecting user files or making requests.
 * @aiUsage Create at App composition; replace this adapter only after the authoritative API supply is available.
 */
export function createOfflineSupply(): AgentSpaceSupply {
  return { mode: "offline", scope, workspaces: [{ id: atlas, title: "atlas", origin: "role-marker" }, { id: web, title: "web", origin: "role-marker" }, { id: empty, title: "empty", origin: "role-marker" }], posts, agents,
    records: (id, channel) => { const agent = agents.find(item => item.id === id); return agent ? channel === "messages" ? messages(agent) : modelRecords(agent) : { status: "unavailable", reason: "精确实例未提供" }; } };
}
