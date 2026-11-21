/** @aiPurpose Office-local post and unavailable Skill readers without launch or fabricated people. */
import { FactButton } from "./FactButton";
import type { SpaceController } from "./offline/controller";
import { matchingPostMembers } from "./offline/projection";
import { PostLaunch } from "./live/Actions";
import type { LiveSpace } from "./live/controller";
import type { Post } from "./offline/model";
/** @aiPurpose Show effective supplied posts and their actual adopted-source members, or honest unavailable Skills.
 * @aiUsage Mount for posts/Skills only; selection only reads; live post launch requires explicit submission.
 */
export function OfficeResources({ controller, live }: { controller: SpaceController; live?: LiveSpace }) {
  const state = controller.getSnapshot();
  const query = state.query.trim().toLocaleLowerCase();
  const posts = controller.supply.posts.filter(post => (!state.workspaceId || post.office === state.workspaceId) && (!query || [post.name, post.office, post.description, post.source, post.profile].join(" ").toLocaleLowerCase().includes(query)));
  const selected = posts.find(post => post.id === state.postId);
  if (state.officeView === "skills") return <section className="resource-unavailable" aria-label="技能"><strong>技能列表未提供</strong><code>{state.workspaceId || controller.supply.scope.id}</code><small>此环境没有提供可用技能目录或使用事件。</small></section>;
  return <div className="post-workspace"><section className="post-list" aria-label="可用岗位"><header><strong>可用岗位</strong><small>{posts.length}</small></header>{posts.map(post => <FactButton facts={<><strong>{post.name}</strong><span>{post.description || "岗位说明未提供"}</span><span>{post.origin}</span></>} type="button" key={post.id} aria-pressed={selected?.id === post.id} onClick={() => controller.post(post.id)}><strong>{post.name}</strong><small>{post.origin} · {matchingPostMembers(controller.supply, post).length} 成员</small>{!state.workspaceId && <small>{post.office}</small>}</FactButton>)}{!posts.length && <p>可用岗位未提供或无匹配</p>}</section>{selected && <PostDetail key={selected.id} post={selected} controller={controller} live={live} />}</div>;
}
function PostDetail({ post, controller, live }: { post: Post; controller: SpaceController; live?: LiveSpace }) {
  const members = matchingPostMembers(controller.supply, post);
  return <section className="post-detail" aria-label={`${post.name} 岗位详情`}><header><strong>{post.name}</strong><small>{post.origin}</small></header><p>{post.description}</p><dl><dt>目录</dt><dd>{post.office}</dd><dt>定义来源</dt><dd><code>{post.source}</code></dd><dt>当前配置引用</dt><dd>{post.profile}</dd><dt>当前定义版本</dt><dd>{post.revision}</dd></dl><strong>采用此来源的当前成员 · {members.length}</strong><div className="post-members">{members.map(agent => <button type="button" key={agent.id} onClick={() => { controller.openExact(agent.id); }}>{agent.title}<small>{agent.role.kind === "adopted" ? `启动快照 ${agent.role.revision} · ${agent.role.profile}` : ""}</small></button>)}</div>{!members.length && <p>暂无成员使用此岗位</p>}{live && <PostLaunch live={live} post={post} />}<details><summary>来源与快照 ⓘ</summary><small>成员按采用的定义来源匹配；当前定义和启动快照分别显示。启动与目录入口以当前供给能力为准。</small></details></section>;
}
