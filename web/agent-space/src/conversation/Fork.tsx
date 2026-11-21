import { useEffect, useRef, useState } from "react";
import { Button } from "../../uiux/systems/space/space.web";
import { useAgentCommands } from "../interactions/AgentCommands";
import { operationIssue, type LiveSpace } from "../live/controller";
import type { AgentResource } from "../offline/model";

const label = "Fork 新 Agent";

/** @aiPurpose Request a deliberate Fork confirmation for the exact reader.
 * @aiUsage Mount in the conversation header; native can_fork alone admits the action.
 */
export function ForkControl({ agent, live }: { agent: AgentResource; live: LiveSpace }) {
  const commands = useAgentCommands();
  if (!live.capabilities(agent.id)?.can_fork) return null;
  return <Button variant="icon" className="conversation-icon" aria-label={label} aria-haspopup="dialog" title={label} disabled={live.blocked(agent.id)} onClick={event => commands?.request("fork", agent, event.currentTarget)}>
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" aria-hidden="true"><path d="M6 7v10m0-5h6a6 6 0 0 0 6-6v1" /><circle cx="6" cy="5" r="2" /><circle cx="6" cy="19" r="2" /><circle cx="18" cy="5" r="2" /></svg>
  </Button>;
}

/** @aiPurpose Present the existing exact operation receipt and allow deliberate result navigation.
 * @aiUsage Pass a navigator that preserves the source window. No name-based fallback or automatic open.
 */
export function ForkFeedback({ agent, live, onOpen }: { agent: AgentResource; live: LiveSpace; onOpen(id: string): void }) {
  const [opened, setOpened] = useState<string>();
  const [issue, setIssue] = useState("");
  const [busy, setBusy] = useState(false);
  const navigation = useRef<AbortController | undefined>(undefined);
  const receipt = live.latestReceipt("fork", agent.id);
  useEffect(() => () => navigation.current?.abort(), [receipt?.id]);
  if (!receipt || opened === receipt.id) return null;
  const status = receipt.operation?.status;
  const ref = live.forkReference(receipt);
  const lookup = status !== 'Succeeded' && status !== 'Failed';
  async function navigate() {
    if (!ref || !receipt || navigation.current) return;
    const request = navigation.current = new AbortController();
    setBusy(true); setIssue("");
    try {
      const target = await live.openReference(ref, { scope: receipt.scope, generation: receipt.body.generation }, request.signal);
      if (!request.signal.aborted) { setOpened(receipt.id); onOpen(target.id); }
    } catch (error) { if (!request.signal.aborted) setIssue(error instanceof Error ? error.message : String(error)); }
    finally {
      if (navigation.current === request) {
        navigation.current = undefined;
        if (!request.signal.aborted) setBusy(false);
      }
    }
  }
  const result = status === 'Succeeded' ? (ref ? 'Fork 已完成' : 'Fork 已完成 · 回执未提供精确会话')
    : status === 'Failed' ? 'Fork 失败' : status === 'Accepted' || status === 'Running' ? '等待 Fork 回执…' : 'Fork 结果未确定';
  const detail = issue || operationIssue(receipt);
  return <div className="conversation-fork-result" data-issue={!!detail || undefined}>
    <small role="status" title={detail}>{result}{issue && ` · ${issue}`}</small>
    {ref && <Button onClick={() => void navigate()} disabled={busy}>打开分叉会话</Button>}
    {lookup && <Button disabled={!live.canLookup(receipt)} onClick={() => void live.lookup(receipt)}>查询原回执</Button>}
  </div>;
}
