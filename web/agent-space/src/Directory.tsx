/** Compact two-line contact rows adapted from web/hall/HallHome.tsx; selection is presentation-local. */
import { useContext, useLayoutEffect, useMemo, useRef } from "react";
import { SpaceWorldContext } from "../uiux/systems/space/components/SpaceTheme";
import { adoptedRoleLabel, membershipKey, type AgentResource, type CloudOfficeDirectory } from "./offline/model";
import { character, hallSpriteUri, lookFor, officePalette } from "./scene/art/art";
import { HALL_STATE_LABEL } from "./scene/model";
import { studioCast, studioSignal, studioSignalLabel } from "./scene/studio-art";
import { studioAssetUrl } from "./scene/studio-assets";
const avatarPalette = officePalette({ colors: { surface0: "#f3f2ed", surface1: "#efefe9", surface2: "#cbd1cb", foreground: "#454d49", foregroundMuted: "#7a847d", accent: "#628f7e", statusWarning: "#b98643", statusDanger: "#b45b50", statusSuccess: "#508a63" } });
/** @aiPurpose Reuse the actual source pixel character as an offline contact portrait.
 * @aiUsage Identity drives a stable appearance shared by roster and conversation; no external image is loaded.
 */
export function PixelAvatar({ identity }: { identity: string }) {
  const uri = useMemo(() => hallSpriteUri(character({ role: "agent", look: lookFor(identity), pose: "idle", facing: "down", frame: 0 }), avatarPalette, 2), [identity]);
  return <img className="avatar pixel-avatar" src={uri} alt="" />;
}
/** @aiPurpose Match a Studio contact portrait to the cast drawn on Canvas.
 * @aiUsage Pass the supplied record and the explicit Studio sample flag.
 */
export function AgentAvatar({ agent, studio }: { agent: AgentResource; studio?: boolean }) {
  const world = useContext(SpaceWorldContext);
  return studio ? <img className="avatar studio-avatar" src={studioAssetUrl(studioCast(agent), world)} alt="" /> : <PixelAvatar identity={agent.id} />;
}
/** @aiPurpose Preserve the source's compact contact directory, state and per-agent draft cues.
 * @aiUsage Supply the same filtered agents as the scene; onSelect opens the exact fixture identity.
 */
export function Directory({ agents, selectedId, drafts, onSelect, onReset, scrollPositions, scrollKey, showEmpty = true, studio, cloudDirectory, groupByHost }: { agents: readonly AgentResource[]; selectedId: string | null; drafts: Readonly<Record<string, string>>; onSelect(id: string): void; onReset(): void; scrollPositions?: Map<string, number>; scrollKey?: string; showEmpty?: boolean; studio?: boolean; cloudDirectory?: CloudOfficeDirectory; groupByHost?: boolean }) {
  const scroll = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (scroll.current && scrollKey) scroll.current.scrollTop = scrollPositions?.get(scrollKey) ?? 0;
  }, [scrollPositions, scrollKey]);
  const projects = new Map(cloudDirectory?.projects.map(project => [project.id, project.title]));
  function row(agent: AgentResource) {
    const state = HALL_STATE_LABEL[agent.state], signal = studio ? studioSignal(agent) : null;
    const label = studio ? studioSignalLabel(agent) : agent.attention ? "未读" : null;
    const status = label && label !== state ? `${state} · ${label}` : state;
    const membership = agent.membership;
    const location = membership.kind === "project" ? projects.get(membershipKey(agent)) ?? "项目目录未提供"
      : membership.kind === "global" ? "全局成员" : membership.kind === "unexposed" ? "项目未开放" : "归属未知";
    const role = agent.ref.host ? groupByHost ? location : `${agent.ref.host} · ${location}` : studio && agent.launchTitle ? agent.launchTitle : adoptedRoleLabel(agent.role);
    return <button type="button" className="office-conversation-link hall-directory-person" key={agent.id} data-agent-key={agent.id} data-studio-signal={signal ?? undefined} aria-current={selectedId === agent.id ? "true" : undefined} aria-label={`${agent.title} · ${agent.ref.host ? `${agent.ref.host} · ${agent.ref.incarnation}` : studio ? status : state}`} onClick={() => onSelect(agent.id)}>
      <AgentAvatar agent={agent} studio={studio} /><span className="hall-person-copy"><strong>{agent.title}</strong>
        <span className="hall-person-preview"><small>{drafts[agent.id]?.trim() ? "[草稿] " : ""}{role} · {agent.ref.host ? agent.ref.incarnation : agent.ref.record === "stopped" ? "停止历史" : "当前实例"}</small><span className="hall-person-state" data-attention={signal === "attention" || agent.attention}>{status}</span></span>
      </span>
    </button>;
  }
  const groups = new Map<string, AgentResource[]>();
  if (groupByHost) for (const agent of agents) {
    const host = agent.ref.host ?? "主机未知";
    if (!groups.has(host)) groups.set(host, []);
    groups.get(host)!.push(agent);
  }
  const entries = groupByHost ? [...groups].sort(([a], [b]) => a.localeCompare(b)).map(([host, members]) =>
    <section key={host} aria-label={host}><small className="cloud-directory-host">{host}</small>{members.map(row)}</section>) : agents.map(row);
  return <section className="hall-directory" aria-label="Agent 名册"><header className="directory-title"><strong>{studio ? "工作室名册" : "精确 Agent 实例"}</strong><small>{agents.length}</small></header><div className="hall-directory-scroll" ref={scroll} onScroll={event => { if (scrollKey && event.currentTarget.clientHeight) scrollPositions?.set(scrollKey, event.currentTarget.scrollTop); }}><div className="hall-directory-items">{entries}</div>{showEmpty && !agents.length && <div className="directory-empty"><span>暂无成员或无匹配</span><button type="button" onClick={onReset}>清除筛选</button></div>}</div></section>;
}
