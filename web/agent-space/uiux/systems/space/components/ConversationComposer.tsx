/** @aiPurpose Controlled composer mechanics shared by Space and UIUX scenes. */
import { useId, useLayoutEffect, useRef, useState, type ClipboardEvent, type CSSProperties, type DragEvent, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import type { Attachment } from "../../../../../management/api/native";
import { Attachments } from "./Attachments";
import { Button, Feedback } from "./Controls";

type EditorSize = "compact" | "auto" | "expanded" | number;
interface EditorPosition { top: number; start: number; end: number }
const editorChoices: readonly { size: EditorSize; label: string }[] = [
  { size: "compact", label: "收紧" }, { size: "auto", label: "随内容调整" }, { size: "expanded", label: "展开" },
];

export interface ConversationComposerProps {
  draft: string;
  onDraft(text: string): void;
  attachments: readonly Attachment[];
  draftOnly?: boolean;
  uploading?: boolean;
  canAttach?: boolean;
  readOnly?: boolean;
  canSend: boolean;
  maxText?: number;
  status: string;
  contextError?: string;
  onSubmit(): Promise<void> | void;
  onAddFiles?(files: readonly Blob[]): Promise<void> | void;
  onRemoveAttachment?(id: string): void;
  options?: ReactNode;
  editorSize?: EditorSize;
  onEditorSize?(size: EditorSize): void;
  editorMemory?: { key: string; positions: Map<string, EditorPosition> };
  receiptAction?: ReactNode;
}

/** @aiPurpose Provide the canonical controlled Space message composer.
 * @aiUsage Keep draft, attachments, capabilities and delivery result in the page/controller owner; this component owns only input mechanics and accessible feedback.
 * @aiBehavior onSubmit reports deliberate intent; the product receipt adapter alone decides when the exact sent revision may be cleared.
 */
export function ConversationComposer({ draft, onDraft, attachments, draftOnly = false, uploading = false, canSend, maxText = 200_000, status, contextError, onSubmit, onAddFiles, onRemoveAttachment, options, canAttach = false, readOnly = false, editorSize, onEditorSize, editorMemory, receiptAction }: ConversationComposerProps) {
  const feedbackId = useId();
  const disclosure = useId();
  const lock = useRef(false);
  const composing = useRef(false);
  const sizeMenu = useRef<HTMLDetailsElement>(null);
  const sizeTrigger = useRef<HTMLElement>(null);
  const [localSize, setLocalSize] = useState<EditorSize>("auto");
  const input = useRef<HTMLTextAreaElement>(null);
  const editorId = useId();
  const drag = useRef<{ pointerId: number; origin: number; height: number; min: number; target?: number } | undefined>(undefined);
  const [dragHeight, setDragHeight] = useState<number>();
  const [submitting, setSubmitting] = useState(false);
  const picker = useRef<HTMLInputElement>(null);
  const validSize = new TextEncoder().encode(draft).length <= maxText;
  const enabled = !draftOnly && canSend && validSize && !uploading && !submitting && !readOnly;
  const attachmentEnabled = !readOnly && !draftOnly && canAttach && !uploading && !submitting;
  const placeholder = readOnly ? "只读" : draftOnly ? "写下本页草稿…" : "发送 AgentX 消息…";
  const size = editorSize ?? localSize;
  const requested = dragHeight ?? (typeof size === "number" ? size : undefined);
  const editorStyle = requested === undefined ? undefined : { "--composer-user-height": `${requested}px` } as CSSProperties;
  const { key: editorKey, positions: editorPositions } = editorMemory ?? {};
  useLayoutEffect(() => {
    const node = input.current;
    if (!node || !editorKey || !editorPositions) return;
    const position = editorPositions.get(editorKey);
    if (position) {
      node.setSelectionRange(position.start, position.end);
      node.scrollTop = position.top;
    }
    return () => { editorPositions.set(editorKey, editorPosition(node)); };
  }, [editorKey, editorPositions]);
  function storePosition(node: HTMLTextAreaElement) {
    if (editorKey) editorPositions?.set(editorKey, editorPosition(node));
  }
  function setSize(mode: EditorSize) {
    if (onEditorSize) onEditorSize(mode); else setLocalSize(mode);
  }
  function chooseSize(mode: EditorSize) {
    setSize(mode);
    if (sizeMenu.current) sizeMenu.current.open = false;
    sizeTrigger.current?.focus({ preventScroll: true });
  }
  function startResize(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0 || !event.isPrimary || !input.current) return;
    const node = input.current, style = getComputedStyle(node);
    const height = node.getBoundingClientRect().height;
    drag.current = { pointerId: event.pointerId, origin: event.clientY, height, min: parseFloat(style.minBlockSize) };
    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
  }
  function moveResize(event: PointerEvent<HTMLButtonElement>) {
    const current = drag.current;
    if (!current || current.pointerId !== event.pointerId) return;
    const difference = current.origin - event.clientY;
    if (current.target === undefined && Math.abs(difference) < 3) return;
    current.target = Math.round(Math.max(current.min, current.height + difference));
    setDragHeight(current.target);
  }
  function finishResize(event: PointerEvent<HTMLButtonElement>, commit: boolean) {
    const current = drag.current;
    if (!current || current.pointerId !== event.pointerId) return;
    if (commit && current.target !== undefined) setSize(current.target);
    drag.current = undefined;
    setDragHeight(undefined);
  }
  async function submit() {
    if (!enabled || lock.current || composing.current) return;
    lock.current = true; setSubmitting(true);
    try { await onSubmit(); } finally { lock.current = false; setSubmitting(false); }
  }
  function keyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey) && !event.repeat && !composing.current && !event.nativeEvent.isComposing && event.nativeEvent.keyCode !== 229 && enabled) { event.preventDefault(); void submit(); }
  }
  async function addFiles(files: readonly Blob[]) { if (attachmentEnabled && files.length) await onAddFiles?.(files); }
  function drop(event: DragEvent<HTMLFormElement>) { if (!event.dataTransfer.types.includes("Files")) return; event.preventDefault(); void addFiles(Array.from(event.dataTransfer.files)); }
  function paste(event: ClipboardEvent<HTMLTextAreaElement>) {
    const files = Array.from(event.clipboardData.items).filter(item => item.kind === "file").map(item => item.getAsFile()).filter((file): file is File => !!file);
    if (files.length) {
      event.preventDefault();
      if (!attachmentEnabled) return;
      const text = event.clipboardData.getData("text/plain"), input = event.currentTarget;
      if (text) onDraft(draft.slice(0, input.selectionStart) + text + draft.slice(input.selectionEnd));
      void addFiles(files);
    }
  }
  return <form className="hall-composer conversation-input" data-editor-size={requested !== undefined ? "custom" : size} style={editorStyle} onDrop={drop} onDragOver={event => { if (event.dataTransfer.types.includes("Files")) event.preventDefault(); }} onSubmit={event => event.preventDefault()}>
    {!readOnly && <button type="button" className="composer-resize-grip" aria-label="拖动调整草稿高度" title="拖动调整高度；回车打开高度选项" aria-controls={editorId}
      onPointerDown={startResize} onPointerMove={moveResize} onPointerUp={event => finishResize(event, true)} onPointerCancel={event => finishResize(event, false)} onLostPointerCapture={event => finishResize(event, false)}
      onClick={event => { if (event.detail === 0 && sizeMenu.current) { sizeMenu.current.open = true; sizeTrigger.current?.focus({ preventScroll: true }); } }} />}
    {attachments.length > 0 && <details className="composer-attachments"><summary>附件 {attachments.length}</summary><div className="composer-attachment-preview"><Attachments items={attachments} onRemove={uploading ? undefined : onRemoveAttachment} /></div></details>}
    {contextError && <div className="composer-feedback"><Feedback error>{contextError}</Feedback></div>}
    <label className="composer-editor" data-draft={size === "auto" ? draft : undefined}><span className="visually-hidden">当前精确 Agent 的本页草稿</span><textarea id={editorId} ref={input} rows={2} readOnly={readOnly} aria-label="Agent 消息草稿" aria-describedby={feedbackId} value={draft} maxLength={maxText} onChange={event => onDraft(event.target.value)} placeholder={placeholder} onScroll={event => storePosition(event.currentTarget)} onSelect={event => storePosition(event.currentTarget)} onBlur={event => storePosition(event.currentTarget)} onKeyDown={keyDown} onCompositionStart={() => { composing.current = true; }} onCompositionEnd={() => { composing.current = false; }} onPaste={paste} /></label>
    <div className="composer-toolbar">
      {!draftOnly && <><input ref={picker} type="file" multiple hidden aria-label="选择附件" onChange={event => { void addFiles(Array.from(event.target.files ?? [])); event.target.value = ""; }} /><Button variant="icon" className="conversation-icon" aria-label="添加附件" title="添加图片或文件" disabled={!attachmentEnabled} onClick={() => picker.current?.click()}><svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round"><path d="m8 12 5.5-5.5a3 3 0 0 1 4.3 4.3l-7.3 7.3a4.3 4.3 0 0 1-6.1-6.1l7.3-7.3m-4 8.4 5.5-5.5" /></svg></Button></>}
      <div className="composer-status"><Feedback id={feedbackId}>{submitting ? "提交中…" : !validSize ? "消息超过原生字节上限" : status}</Feedback>{receiptAction}</div>
      {!readOnly && <details className="composer-size" name={disclosure} ref={sizeMenu}>
        <summary ref={sizeTrigger} aria-label="调整草稿高度" title="调整草稿高度"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M5 4h14M5 20h14m-7-13v10m-3-7 3-3 3 3m-6 4 3 3 3-3" /></svg></summary>
        <div className="composer-size-options" aria-label="草稿高度">{editorChoices.map(choice => <button type="button" key={choice.size} aria-pressed={size === choice.size} onClick={() => chooseSize(choice.size)}>{choice.label}</button>)}</div>
      </details>}
      {!draftOnly && options}
      {!readOnly && <Button variant="primary" className="hall-local-append" aria-label="发送 AgentX 消息" title="发送 · Ctrl / ⌘ + Enter" disabled={!enabled} onClick={() => void submit()}><svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round"><path d="M12 19V5m-6 6 6-6 6 6" /></svg></Button>}
    </div>
  </form>;
}

function editorPosition(node: HTMLTextAreaElement): EditorPosition {
  return { top: node.scrollTop, start: node.selectionStart, end: node.selectionEnd };
}
