/** Synthetic Catalog Studio facts for the actual Space App. No transport or Agent command ports. */
import { exactAgentKey, type AgentResource, type AgentSpaceSupply, type HistoryRecord, type Post } from "./model";

const catalog = "/studio/catalog", annex = "/studio/annex";
const scope = { id: "/example/agentx-studio", generation: "studio-review-1", coverage: "合成样例 · 工作室与空置附室" };
const roles = {
  maker: "制作版面与印刷样张",
  designer: "安排图录的版式和色样",
  keeper: "对照版片编号与出处",
  scout: "收集来稿，补齐来源",
};
type Role = keyof typeof roles;
interface CastMember {
  name: string; role: Role; duty: string; state: AgentResource["state"];
  topic: string; reply: string; attention?: boolean;
}
const cast: readonly CastMember[] = [
  { name: "mora", role: "scout", duty: "来稿联络", state: "idle", topic: "图录里「河岸」那一页还缺一条出处。你这边有收到新版署名吗？", reply: "收到。署名改成「北岸田野组」，版片编号是 R-14。我把对照页放在工作台左侧，下一轮可以一起核对。" },
  { name: "luna", role: "designer", duty: "封面版式", state: "running", topic: "封面先试两版：暖纸上用墨蓝，浅灰纸上用朱砂。", reply: "墨蓝那张的细线更清楚。我在补角标，再排一张不加边框的样张。" },
  { name: "riku", role: "keeper", duty: "版片编号", state: "failed", attention: true, topic: "请对照 A-06 的木刻版和收件簿。", reply: "两张标签用了同一个编号；年代还不能确认。我把冲突留在核对单上了。" },
  { name: "sira", role: "keeper", duty: "样张归档", state: "ready", topic: "本期退下来的三张试印需要保留一套。", reply: "已经按纸纹归成一组，随时可以核对。草图会单独装袋。" },
  { name: "tobi", role: "maker", duty: "试印木刻", state: "running", topic: "把细线图先在旧纸上印一轮，留一张带网纹的。", reply: "第一轮刚压完，网纹没有糊。我会等墨干了再补标号。" },
  { name: "vera", role: "maker", duty: "调配色墨", state: "running", topic: "先用半份灰蓝配一张色样。", reply: "色样在晾架上。下一张会再淡一点，方便对照纸色。" },
];

function roleSource(role: Role) { return `${catalog}/.agentx/roles/${role}.toml`; }
function agent(entry: CastMember, index: number, stopped = false): AgentResource {
  const ref: AgentResource["ref"] = {
    ...{ scope: scope.id, generation: scope.generation, name: entry.name },
    incarnation: stopped ? null : String(101 + index), stopped_event: stopped ? "80" : null, record: stopped ? "stopped" : "current",
  };
  const role = { kind: "adopted", name: entry.role, source: roleSource(entry.role), description: roles[entry.role], profile: "studio-sample", revision: "studio-role-1" } as const;
  return {
    id: exactAgentKey(ref), ref, name: entry.name, title: `@${entry.name}`, launchTitle: entry.duty,
    state: stopped ? "closed" : entry.state, attention: !stopped && !!entry.attention,
    membership: { kind: "directory", path: catalog }, description: roles[entry.role], role,
    cwd: catalog, model: null, tool: "codex", profile: role.profile, source: { id: scope.id, title: "工作室样例" },
    createdAt: stopped ? "2026-06-12T09:00:00Z" : `2026-06-12T10:0${index}:00Z`,
  };
}

const agents = [...cast.map((entry, index) => agent(entry, index)), agent(cast[0]!, 0, true)];
const posts: Post[] = (Object.keys(roles) as Role[]).map(name => ({
  id: `${catalog}:${name}`, office: catalog, name, origin: "本地", source: roleSource(name), description: roles[name], profile: "studio-sample", revision: "studio-role-1",
}));
const histories = new Map(agents.map(resource => {
  const entry = cast.find(row => row.name === resource.name)!;
  const messageId = `${resource.id}:1`, stopped = resource.ref.record === "stopped";
  const items: HistoryRecord[] = [
    { id: messageId, kind: "message", role: "user", author: "@bigboss", recipients: [resource.title], intent: "request", thread: "图录校样",
      text: stopped ? "上一期寄错的校样是否已经收回？" : entry.topic, at: "2026-06-12T10:10:00Z" },
    { id: `${resource.id}:2`, kind: "message", role: "assistant", author: resource.title, recipients: ["@bigboss"], intent: "inform", replyTo: messageId, thread: "图录校样",
      text: stopped ? "旧校样已收回；这一实例到此结束。" : entry.reply, at: "2026-06-12T10:14:00Z" },
  ];
  return [resource.id, items] as const;
}));

/** Explicit review data: six current actors, an exact stopped namesake and one empty office. */
export function createStudioSupply(): AgentSpaceSupply {
  return {
    mode: "offline", scope, agents, posts, population: { revision: 1, complete: true },
    workspaces: [
      { id: catalog, title: "图录工作室", origin: "explicit-membership" },
      { id: annex, title: "空置附室", origin: "explicit-membership" },
    ],
    human: { unhandledCount: 0, hasMore: false, messages: [
      { id: "studio-readonly-letter", at: "2026-06-12T10:14:00Z", from: "mora", handled_at: "2026-06-12T10:20:00Z", text: "新版署名和版片编号已放在工作台，供下一轮校样核对。" },
    ] },
    records: (key, channel) => {
      const items = histories.get(key);
      return channel === "messages" && items
        ? { status: "available", provenance: "图录工作室 · 合成消息", items }
        : { status: "unavailable", reason: items ? "样例未提供 Transcript" : "精确实例未提供" };
    },
  };
}
