/** @aiPurpose Deterministic local send outcomes for the canonical controlled Composer. */
import { useRef, useState } from "react";
import type { SceneProps } from "@longwork/uiux";
import type { Attachment } from "../../../../management/api/native";
import { ConversationComposer, Button } from "./space.web";
function Demo({ emit, outcome }: SceneProps & { outcome: string }) {
  const [draft, setDraft] = useState('请确认这个精确 Agent 的当前状态。');
  const [attachments, setAttachments] = useState<Attachment[]>([]), [status, setStatus] = useState('UIUX 本地模拟 · 尚未提交');
  const [waiting, setWaiting] = useState(false);
  const revision = useRef(0), sentRevision = useRef(0), sentIds = useRef<string[]>([]);
  const enabled = !waiting && (!!draft.trim() || attachments.length > 0);
  function submit() {
    sentRevision.current = revision.current; sentIds.current = attachments.map(item => item.id);
    emit('submit', { outcome });
    if (outcome === 'success') { setDraft(''); setAttachments([]); setStatus('模拟 Succeeded · 已清理发送草稿'); }
    else { setStatus(outcome === 'failure' ? '模拟 Failed · 草稿保留，可修正后重试' : outcome === 'pending' ? '模拟 Running · 等待原回执' : '模拟 Unknown · 草稿保留，仅查询原回执'); setWaiting(outcome !== 'failure'); }
  }
  function complete() { if (revision.current === sentRevision.current) setDraft(''); setAttachments(items => items.filter(item => !sentIds.current.includes(item.id))); setWaiting(false); setStatus('模拟回执恢复为 Succeeded · 已清理'); }
  function reset() { setDraft('请确认这个精确 Agent 的当前状态。'); setAttachments([]); setWaiting(false); setStatus('UIUX 本地模拟 · 尚未提交'); }
  return <div className="space-preview" data-short="true"><div className="space-preview-tools"><Button onClick={reset}>重置</Button>{waiting && <Button onClick={complete}>模拟查询原回执成功</Button>}</div><div className="hall-reader"><ConversationComposer draft={draft} onDraft={value => { revision.current++; setDraft(value); }} attachments={attachments} canAttach={!waiting} canSend={enabled} status={status} onSubmit={submit} onRemoveAttachment={id => setAttachments(items => items.filter(item => item.id !== id))} onAddFiles={files => setAttachments(items => [...items, ...files.map((file, index) => ({ id: `local-${items.length + index}`, name: file instanceof File ? file.name : 'local file', media_type: 'application/octet-stream', size: file.size, url: 'data:text/plain,UIUX-local-fixture' }))])} /></div></div>;
}
function Success(props: SceneProps) { return <Demo {...props} outcome="success" />; }
function Failure(props: SceneProps) { return <Demo {...props} outcome="failure" />; }
function Unknown(props: SceneProps) { return <Demo {...props} outcome="unknown" />; }
function Pending(props: SceneProps) { return <Demo {...props} outcome="pending" />; }
/** @aiPurpose Match manifest outcomes to real component scenes.
 * @aiUsage All effects are explicit local simulation; no native credentials or network are used.
 */
export const scenes = { default: Success, failure: Failure, unknown: Unknown, pending: Pending };
