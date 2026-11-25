import { useEffect, useId, useLayoutEffect, useRef, useState, type FormEvent } from 'react';
import { newCloudId, CloudAdminError, CloudAdminOutcomeUnknown } from '@agentx/client';
import { InteractionDialog } from '../dialog';
import { cloudTokenPattern, explainError, type AdminClient, type NamespaceReceipt, type NamespaceTarget, type NamespaceWriteAction } from './model';

type Fields = { id: string; token: string };
type Intent = Fields & { requestId: string; target?: NamespaceTarget };
type Phase =
  | { kind: 'draft'; fields: Fields; error?: string }
  | { kind: 'busy'; intent: Intent }
  | { kind: 'unknown'; intent: Intent; detail?: string }
  | { kind: 'rejected'; intent: Intent; error: string }
  | { kind: 'saved'; intent: Intent; receipt: NamespaceReceipt };

const namespaceIdPattern = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;

/**
 * @aiPurpose Create a Namespace or rotate the selected Namespace's communication bearer and present its original receipt.
 * @aiUsage Supply an exact action, then set it to null on dismissal; refresh does not invalidate the captured action.
 * @aiBehavior Once dispatched, an uncertain result retains a locked input in memory for explicit same-request replay.
 */
export function NamespaceWriteDialog({ client, action, onDismiss, onSaved, onRefresh }: {
  client: AdminClient; action: NamespaceWriteAction | null; onDismiss: () => void; onSaved: (id: string) => void; onRefresh: () => void;
}) {
  const formId = useId();
  const [phase, setPhase] = useState<Phase | null>(null);
  const [revealed, setRevealed] = useState(false);
  const [copied, setCopied] = useState('');
  const [confirmClose, setConfirmClose] = useState(false);
  const tokenInput = useRef<HTMLInputElement>(null);
  const pending = useRef<AbortController | null>(null);
  useEffect(() => () => pending.current?.abort(), []);
  useLayoutEffect(() => {
    setPhase(action ? { kind: 'draft', fields: { id: action.kind === 'rotate' ? action.namespace.namespace_id : '', token: newCloudId() } } : null);
    setRevealed(false);
    setCopied('');
    setConfirmClose(false);
  }, [action]);

  const target = action?.kind === 'rotate' ? action.namespace : undefined;
  const fields = phase && (phase.kind === 'draft' ? phase.fields : phase.intent);
  const error = phase?.kind === 'draft' || phase?.kind === 'rejected' ? phase.error : undefined;
  const busy = phase?.kind === 'busy';
  const saved = phase?.kind === 'saved';
  const unknown = phase?.kind === 'unknown';

  function dismiss() {
    if (pending.current) return;
    if (phase?.kind === 'unknown') { setConfirmClose(true); return; }
    onDismiss();
  }

  function edit(patch: Partial<Fields>) {
    setCopied('');
    setPhase(current => current?.kind === 'draft' ? { kind: 'draft', fields: { ...current.fields, ...patch } } : current);
  }

  async function copy() {
    if (!fields) return;
    try {
      await navigator.clipboard.writeText(fields.token);
      setCopied('已复制');
    } catch {
      setRevealed(true);
      setCopied('请手动复制');
      requestAnimationFrame(() => { tokenInput.current?.focus(); tokenInput.current?.select(); });
    }
  }

  async function execute(intent: Intent, recovering: boolean) {
    if (pending.current) return;
    const controller = new AbortController();
    pending.current = controller;
    setConfirmClose(false);
    setPhase({ kind: 'busy', intent });
    const body = { request_id: intent.requestId, token: intent.token };
    const options = { signal: controller.signal };
    try {
      const receipt = intent.target
        ? await client.rotateNamespaceToken(intent.id, { ...body, expected_version: intent.target.credential_version }, options)
        : await client.putNamespace(intent.id, body, options);
      if (controller.signal.aborted) return;
      setPhase({ kind: 'saved', intent, receipt });
      onSaved(receipt.namespace_id);
    } catch (failure) {
      if (controller.signal.aborted) return;
      const detail = explainError(failure, '提交失败。请检查输入后重试。');
      if (failure instanceof CloudAdminOutcomeUnknown || recovering) {
        setPhase({ kind: 'unknown', intent, detail: failure instanceof CloudAdminOutcomeUnknown ? undefined : detail });
      } else if (intent.target) {
        setPhase({ kind: 'rejected', intent, error: detail });
      } else {
        setPhase({ kind: 'draft', fields: { id: intent.id, token: intent.token }, error: detail });
      }
      if (failure instanceof CloudAdminError && failure.status === 409) onRefresh();
    } finally {
      pending.current = null;
    }
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (phase?.kind !== 'draft') return;
    const { id, token } = phase.fields;
    if (!namespaceIdPattern.test(id) || !cloudTokenPattern.test(token)) return;
    void execute({ id, token, requestId: newCloudId(), target }, false);
  }

  const title = target ? '轮换通信令牌' : '新建 Namespace';
  const description = target ? `${target.namespace_id} · 当前版本 v${String(target.credential_version)}。轮换生效后旧令牌失效。` : '请保存通信令牌。部署不会在回执中返回令牌。';
  const footer = <div className="dialog-buttons deployment-write-actions">
    {unknown ? <>
      <button type="button" onClick={dismiss}>关闭</button>
      <button type="button" className="primary" onClick={() => void execute(phase.intent, true)}>重试同一请求</button>
    </> : saved || phase?.kind === 'rejected' ? <button type="button" className={saved ? 'primary' : undefined} onClick={onDismiss}>{saved ? '完成' : '返回 Namespace'}</button> : <>
      <button type="button" onClick={dismiss} disabled={busy}>取消</button>
      <button className="primary" form={formId} type="submit" disabled={busy || !fields?.id || !fields.token}>{busy ? '正在提交…' : target ? '确认轮换' : '创建 Namespace'}</button>
    </>}
  </div>;

  return <InteractionDialog open={!!action && !!phase} variant="launch" icon={target ? 'refresh' : 'plus'} title={title} description={description} closeLabel="关闭令牌操作" onDismiss={dismiss} footer={footer}>
    {!!action && fields && <form id={formId} className="launch-fields deployment-credential-form" autoComplete="off" onSubmit={submit}>
      {!target && <label>Namespace ID<input value={fields.id} onChange={event => edit({ id: event.target.value })} readOnly={phase?.kind !== 'draft'} required pattern="[a-z0-9](?:[a-z0-9\-]{0,61}[a-z0-9])?" maxLength={63} placeholder="例如 studio" title="1–63 个小写字母、数字或连字符，首尾为字母或数字" spellCheck={false} /></label>}
      <label>通信令牌<input ref={tokenInput} type={revealed ? 'text' : 'password'} value={fields.token} onChange={event => edit({ token: event.target.value })} readOnly={phase?.kind !== 'draft'} autoComplete="off" required pattern={cloudTokenPattern.source} title="12 位字母数字" spellCheck={false} /></label>
      <div className="deployment-token-actions">
        <button type="button" className="quiet" aria-pressed={revealed} onClick={() => setRevealed(value => !value)}>{revealed ? '隐藏令牌' : '显示令牌'}</button>
        <button type="button" onClick={() => void copy()}>复制令牌</button>
        {phase?.kind === 'draft' && <button type="button" className="quiet" onClick={() => edit({ token: newCloudId() })}>重新生成</button>}
        <span role="status" className="muted">{copied}</span>
      </div>
      {error && <p className="deployment-error" role="alert">{error}</p>}
      {unknown && <div className="deployment-write-note" role="alert">
        <strong>尚未确认操作结果</strong>
        <p>可用原令牌和同一请求重试。关闭此窗口会清除本页用于重试的凭据。</p>
        {phase.detail && <p>{phase.detail}</p>}
        <code>{phase.intent.requestId}</code>
      </div>}
      {unknown && confirmClose && <button type="button" className="danger" onClick={onDismiss}>关闭并清除重试凭据</button>}
      {saved && <div className="deployment-write-note" data-saved="true" role="status">
        <strong>请求已确认 · {phase.receipt.namespace_id} · v{String(phase.receipt.credential_version)}</strong>
        <code>{phase.receipt.namespace_url}</code>
        <span className="muted">请保存通信令牌，关闭后无法从回执取回。</span>
      </div>}
    </form>}
  </InteractionDialog>;
}
