import { findConversation } from "../offline/model";
import { createContext, useContext, useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import type { AgentResource } from "../offline/model";
import type { SpaceController } from "../offline/controller";
import { operationIssue, type LiveSpace } from "../live/controller";
import { useOverlay, useSpaceShortcuts } from "./SpaceInteractions";
import { CommandSurface } from "../../uiux/systems/space/space.web";
import "../conversation/compact-surfaces.css";

type Action = "new" | "stop-participation" | "fork" | "kill";
interface Commands { request(action: Action, agent?: AgentResource, invoker?: HTMLElement): void; menu(agent: AgentResource, anchor: HTMLElement): void }
const Context = createContext<Commands | null>(null);
const LABELS: Record<Action, string> = { new: "新建 Agent", "stop-participation": "停止参与", fork: "Fork 新 Agent", kill: "终止进程" };

/** @aiPurpose Compose deliberate exact-Agent dialogs and compact pointer/keyboard context actions.
 * @aiUsage Wrap Space content once with its LiveSpace/controller; offline supply offers no native mutations.
 * @aiBehavior Menu/dialog opening is inert. Existing native capabilities, confirmations and receipt journal own effects.
 */
export function AgentCommands({ live, controller, children, onSearch }: { live?: LiveSpace; controller: SpaceController; children: ReactNode; onSearch?(): void }) {
  const [dialog, setDialog] = useState<{ action: Action; agent?: AgentResource; opener: HTMLElement | null }>();
  const [menu, setMenu] = useState<{ agent: AgentResource; x: number; y: number; anchor: HTMLElement }>();
  const commandInvoker = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (dialog) commandInvoker.current = dialog.opener;
    else {
      // Reconciled receipts can close from an effect; return focus after the modal has unmounted.
      const invoker = commandInvoker.current;
      commandInvoker.current = null;
      if (invoker?.isConnected) invoker.focus({ preventScroll: true });
    }
  }, [dialog]);
  function request(action: Action, agent?: AgentResource, invoker?: HTMLElement) {
    if (!live || (action !== "new" && !agent)) return;
    const opener = invoker ?? menu?.anchor ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    setMenu(undefined); setDialog({ action, agent, opener });
  }
  const commands: Commands = { request, menu(agent, anchor) {
    if (!live) return;
    const box = anchor.getBoundingClientRect();
    setMenu({ agent, anchor, x: Math.min(innerWidth - 224, Math.max(8, box.right)), y: Math.min(innerHeight - 184, Math.max(8, box.top)) });
  } };
  useSpaceShortcuts({ newAgent: () => request("new"), killAgent: () => {
    const agent = findConversation(controller.supply, controller.getSnapshot().selectedId);
    if (agent && live?.capabilities(agent.id)?.can_stop) request("kill", agent);
  }, search: onSearch ?? (() => document.querySelector<HTMLInputElement>('[aria-label="搜索 Agent 或工作区"]')?.focus()) });
  return <Context.Provider value={commands}>{children}
    {menu && live && <ActorMenu anchor={menu.anchor} live={live} agent={menu.agent} x={menu.x} y={menu.y} onAction={action => request(action, menu.agent)} onClose={() => setMenu(undefined)} />}
    {dialog && live && <AgentCommandDialog opener={dialog.opener} key={`${dialog.action}:${dialog.agent?.id ?? "new"}`} live={live} controller={controller} action={dialog.action} agent={dialog.agent} onClose={() => setDialog(current => current === dialog ? undefined : current)} />}
  </Context.Provider>;
}

/** @aiPurpose Request existing command dialogs from a reader or scene without performing effects.
 * @aiUsage Use under AgentCommands; outside/offline absence remains an unavailable capability.
 */
export function useAgentCommands(): Commands | null { return useContext(Context); }

/** @aiPurpose Expose the native new-Agent dialog from a compact labeled action.
 * @aiUsage Place under AgentCommands on a live Space toolbar; the command owns submission and focus return.
 */
export function NewAgentButton() {
  const commands = useAgentCommands();
  return <button type="button" className="space-new-agent" aria-label="新建 Agent" aria-haspopup="dialog" title="新建 Agent · Ctrl / ⌘ + Shift + N" onClick={event => commands?.request("new", undefined, event.currentTarget)}>
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg><span>新建<span className="space-new-agent-suffix"> Agent</span></span>
  </button>;
}

function ActorMenu({ anchor, live, agent, x, y, onAction, onClose }: { anchor: HTMLElement; live: LiveSpace; agent: AgentResource; x: number; y: number; onAction(action: Action): void; onClose(): void }) {
  const root = useRef<HTMLDivElement>(null), close = useOverlay(true, onClose, 60, root, false, anchor);
  const native = live.capabilities(agent.id), blocked = live.blocked(agent.id);
  const style = { "--command-menu-x": `${x}px`, "--command-menu-y": `${y}px` } as CSSProperties;
  const items: { action: Action; available: boolean; reason: string }[] = [
    { action: "stop-participation", available: !!native?.can_stop_participation, reason: "停止 AgentX 参与；具体进程处置由原生生命周期决定" },
    { action: "fork", available: !!native?.can_fork, reason: native?.can_fork ? "从此精确会话分叉新 Agent" : "原生工具或会话不支持分叉" },
    { action: "kill", available: !!native?.can_stop, reason: "终止此精确实例的进程，继续前需要确认" },
  ];
  useEffect(() => {
    root.current?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    const outside = (event: PointerEvent) => { if (event.target instanceof Node && !root.current?.contains(event.target)) close(); };
    document.addEventListener("pointerdown", outside); return () => document.removeEventListener("pointerdown", outside);
  }, [close]);
  function keys(event: React.KeyboardEvent) {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    const buttons = [...(root.current?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])];
    const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const index = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (current + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
    event.preventDefault(); buttons[index]?.focus();
  }
  return <div ref={root} className="actor-action-menu" role="menu" aria-label={`${agent.title} 操作`} style={style} onKeyDown={keys}>
    <strong>{agent.title}</strong>{items.map(item => <button role="menuitem" type="button" key={item.action} disabled={blocked || !item.available} title={item.reason} onClick={() => onAction(item.action)}>{LABELS[item.action]}</button>)}
    <button role="menuitem" type="button" onClick={close}>关闭</button>
  </div>;
}

function AgentCommandDialog({ opener, live, controller, action, agent, onClose }: { opener: HTMLElement | null; live: LiveSpace; controller: SpaceController; action: Action; agent?: AgentResource; onClose(): void }) {
  const scope = useRef(live.descriptor), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const mounted = useRef(false), profileInput = useRef<HTMLSelectElement>(null), errorTarget = useRef<HTMLDivElement>(null);
  const closeAfterReceipt = useRef<(() => void) | undefined>(undefined);
  const [receiptId, setReceiptId] = useState<string>();
  const [title, setTitle] = useState(""), [prompt, setPrompt] = useState("");
  const [directory, setDirectory] = useState(() => controller.getSnapshot().workspaceId.startsWith("/") ? controller.getSnapshot().workspaceId : live.descriptor?.project ?? "");
  const [profile, setProfile] = useState(live.descriptor?.profiles[0]?.name ?? "");
  const isNew = action === "new", profiles = live.descriptor?.profiles ?? [];
  const native = agent && live.capabilities(agent.id);
  const supported = isNew ? profiles.some(item => item.name === profile) : action === "fork" ? !!native?.can_fork : action === "kill" ? !!native?.can_stop : !!native?.can_stop_participation;
  const blocked = agent ? live.blocked(agent.id) : live.blocked("launch:new");
  const stale = !scope.current || live.descriptor?.scope !== scope.current.scope || live.descriptor.generation !== scope.current.generation;
  const latestReceipt = isNew ? live.latestLaunch("launch:new")
    : action === "fork" && agent ? live.latestReceipt("fork", agent.id) : undefined;
  const receipt = latestReceipt && (latestReceipt.id === receiptId || blocked) ? latestReceipt : undefined;
  const receiptIssue = receipt && operationIssue(receipt);
  const confirmed = receipt?.id === receiptId && receipt?.operation?.status === "Succeeded";
  const enabled = supported && !blocked && !stale && !busy && !confirmed;
  const feedback = stale ? "数据范围已变化，请关闭后重新选择。" : receiptIssue || error;
  useEffect(() => {
    mounted.current = true;
    profileInput.current?.focus({ preventScroll: true });
    return () => { mounted.current = false; };
  }, []);
  useEffect(() => { if (feedback) errorTarget.current?.focus({ preventScroll: false }); }, [feedback]);
  useEffect(() => { if (confirmed && !stale) closeAfterReceipt.current?.(); }, [confirmed, stale]);
  async function submit(event: React.FormEvent, close: () => void) {
    event.preventDefault(); if (!enabled) return;
    const previous = latestReceipt?.id;
    setBusy(true); setError(""); setReceiptId(undefined);
    try {
      if (isNew) await live.launchNew({ title: title || null, prompt: prompt || null, profile, directory: directory || null, role: null, workspace: null, tool: null }, scope.current!);
      else if (agent) {
        const succeeded = await live.target(action, agent.id);
        if (action === "fork" && !succeeded) {
          if (!mounted.current) return;
          const original = live.latestReceipt("fork", agent.id);
          if (original && original.id !== previous) {
            closeAfterReceipt.current = close;
            setReceiptId(original.id);
            return;
          }
          throw new Error("未确认 Fork 结果");
        }
      }
      if (mounted.current) close();
    } catch (failure) {
      if (!mounted.current) return;
      const current = isNew ? live.latestLaunch("launch:new") : undefined;
      if (current && current.id !== previous && operationIssue(current)) {
        closeAfterReceipt.current = close;
        setReceiptId(current.id);
      }
      else setError(failure instanceof Error ? failure.message : String(failure));
    } finally { if (mounted.current) setBusy(false); }
  }
  const consequence = action === "kill" ? "终止此精确实例的进程。历史记录会保留。" : action === "stop-participation" ? "停止此实例参与 AgentX；原生生命周期按会话类型处置进程。" : action === "fork" ? "使用原生工具从此精确会话创建分叉，原会话保持独立。" : "";
  const submitLabel = busy ? "提交中…" : isNew ? "启动" : `确认${LABELS[action]}`;
  const availability = confirmed ? "原操作已成功。" : !supported ? (isNew ? "请选择可用的启动配置。" : "此实例当前不支持该操作。") : (isNew || action === "fork") && blocked && !busy && !stale && !feedback
    ? (receipt?.operation ? `原操作 · ${receipt.operation.status}` : "操作暂不可用 · 查看连接与回执") : "";
  return <CommandSurface title={LABELS[action]} opener={opener} onClose={onClose} onSubmit={submit} submitLabel={submitLabel} disabled={!enabled} danger={action === "kill" || action === "stop-participation"}>
      {agent ? <div className="command-identity"><strong>{action === "fork" ? "来源 · " : ""}{agent.title}</strong><small className="command-exact-ref">{agent.ref.stopped_event != null ? `停止历史 #${agent.ref.stopped_event}` : `当前实例 · ${agent.ref.incarnation}`}</small><p>{consequence}</p></div> : <div className="launch-form">
        <div className="launch-field-pair">
          <label>启动配置<select ref={profileInput} required aria-label="新 Agent 启动配置" value={supported ? profile : ""} onChange={event => setProfile(event.target.value)}>{!supported && <option value="">{profiles.length ? "请选择" : "无可用配置"}</option>}{profiles.map(item => <option key={item.name} value={item.name}>{item.name} · {item.tool}</option>)}</select></label>
          <label><span>职责标题 <small>可选</small></span><input aria-label="新 Agent 职责标题" maxLength={160} value={title} onChange={event => setTitle(event.target.value)} /></label>
        </div>
        <label><span>启动任务 <small>可选</small></span><textarea aria-label="新 Agent 任务" value={prompt} onChange={event => setPrompt(event.target.value)} /></label>
        <details className="launch-directory">
          <summary><span>工作目录</span><code title={directory}>{directory || "使用默认目录"}</code></summary>
          <label><span className="visually-hidden">新 Agent 工作目录</span><input aria-label="新 Agent 目录" value={directory} onChange={event => setDirectory(event.target.value)} /></label>
        </details>
      </div>}
      {feedback && <div className="command-result" role="alert" ref={errorTarget} tabIndex={-1}>{feedback}</div>}
      {availability && <div className="command-result" role="status">{availability}</div>}
      {receipt && !busy && !stale && (blocked || receiptIssue) && <div className="command-receipt"><button type="button" disabled={!live.canLookup(receipt)} onClick={() => { setReceiptId(receipt.id); void live.lookup(receipt); }}>查询原回执</button></div>}
  </CommandSurface>;
}
