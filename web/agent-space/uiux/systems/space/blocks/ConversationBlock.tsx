/** @aiPurpose Compose exact conversation identity, record channels and controlled reading/input regions. */
import type { ReactNode } from "react";
import { ConversationLayout, type ConversationScroll } from "../layouts/WorkspaceLayout";
import { Button } from "../components/Controls";
export type ConversationChannel = "messages" | "model";
interface Props {
  identity: { title: string; role: string; kind: string; state: string; source: string; avatar?: ReactNode };
  channel: ConversationChannel; onChannel(channel: ConversationChannel): void; onBack?(): void;
  backLabel?: string;
  transcriptUnavailable?: string;
  workspace?: boolean;
  workspaceControls?: ReactNode;
  relations?: ReactNode;
  primaryAction?: ReactNode; result?: ReactNode; liveIssue?: ReactNode; olderStatus?: ReactNode;
  facts?: ReactNode; actions?: ReactNode; presentationControl?: ReactNode; history: ReactNode; context?: ReactNode; composer?: ReactNode;
  historySearch?: ReactNode; searchTrigger?: ReactNode; exactLabel?: string;
  historyScroll?: ConversationScroll;
}
const channels: { id: ConversationChannel; label: string }[] = [{ id: "messages", label: "消息" }, { id: "model", label: "Transcript" }];
/** @aiPurpose Present one exact conversation without acquiring records or performing native actions.
 * @aiUsage Supply identity and record/input slots from the product adapter; drafts and channel choice remain controlled.
 * @aiLayout History scrolls independently; in workspace mode the caller supplies window actions beside the channels.
 */
export function ConversationBlock({ identity, channel, onChannel, onBack, backLabel = "返回总览", transcriptUnavailable, workspace, workspaceControls, relations, facts, actions, presentationControl, history, context, composer, historyScroll, historySearch, searchTrigger, exactLabel, primaryAction, result, liveIssue, olderStatus }: Props) {
  const heading = workspace
    ? <div className="conversation-heading-copy"><h1 title={exactLabel || identity.title}>{identity.title}</h1><small className="workspace-reader-meta"><span className="workspace-reader-kind" title={exactLabel}>{identity.kind}</span><span className="reader-state">{identity.state}</span><span className="workspace-reader-role" title={identity.role}>{identity.role}</span></small></div>
    : <div className="conversation-heading-copy"><h1><span>{identity.title}</span><span className="reader-heading-role"><span className="reader-divider"> · </span>{identity.role}</span></h1><small><span className="reader-state">{identity.state}</span><span className="reader-divider"> · </span><span className="reader-origin">{identity.kind} · {identity.source}</span></small></div>;
  const back = !workspace && onBack && <Button variant="icon" className="reader-back" title={backLabel} aria-label={backLabel} onClick={onBack}>‹</Button>;
  const headerTools = <div className="workspace-reader-tools">{workspace && searchTrigger}{relations}{primaryAction}{facts}</div>;
  const header = <div className="hall-conversation-heading"><div className="reader-identity">{back}{identity.avatar}{heading}</div>{headerTools}</div>;
  const channelTabs = <nav className="record-channels" aria-label="记录类型">{channels.map(entry => <Button key={entry.id} disabled={entry.id === "model" && !!transcriptUnavailable} title={entry.id === "model" ? transcriptUnavailable : undefined} aria-pressed={channel === entry.id} onClick={() => onChannel(entry.id)}>{entry.label}</Button>)}{!workspace && searchTrigger}</nav>;
  const channelBar = workspace ? <div className="workspace-reader-channelbar">{channelTabs}{workspaceControls && <div className="workspace-reader-window-actions">{workspaceControls}</div>}</div> : channelTabs;
  const feedback = <>{channelBar}{result}{historySearch}{presentationControl && <div className="reader-presentation-control">{presentationControl}</div>}{actions}</>;
  return <div className={workspace ? "hall-reader workspace-reader" : "hall-reader"} data-history-search={!!historySearch || undefined}><ConversationLayout header={header} feedback={feedback} history={history} context={context} composer={composer} historyScroll={historyScroll} liveIssue={liveIssue} olderStatus={olderStatus} /></div>;
}
