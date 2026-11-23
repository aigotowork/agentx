import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState, type FormEvent } from 'react';
import { CloudAdminError, CloudAdminOutcomeUnknown, type CloudControlInput, type CloudControlReceipt, type CloudHostCapabilities } from '@agentx/client';
import { InteractionDialog } from '../dialog';
import { Timestamp } from '../presentation';
import { explainError, useRead, type AdminClient } from './model';

export type ControlTarget = { scope: string; generation: string; host: string } & (
  { kind: 'launch' } | { kind: 'stop'; name: string; incarnation: string }
);

const statusLabel: Record<CloudControlReceipt['status'], string> = {
  queued: '等待主机接收', running: '主机正在执行', succeeded: '已完成', failed: '执行失败', uncertain: '执行结果未确认',
};

function Receipt({ value }: { value: CloudControlReceipt }) {
  const outcome = value.outcome;
  const command = value.command;
  const agent = outcome?.status === 'succeeded' ? outcome.agent : null;
  return <dl className="deployment-control-receipt" role="status">
    <div><dt>状态</dt><dd><strong>{statusLabel[value.status]}</strong>{outcome && 'code' in outcome && <> · <code>{outcome.code}</code></>}</dd></div>
    <div><dt>请求 ID</dt><dd><code>{value.request_id}</code></dd></div>
    <div><dt>主机</dt><dd><code>{value.host}</code> · <code>{value.installation_id}</code></dd></div>
    <div><dt>{({ launch: '启动', stop: '停止', add_workspace: '添加工作区' })[command.kind]}</dt><dd>{command.kind === 'launch'
      ? <>{command.workspace} · {command.role ?? '默认 Codex'}{command.name && <> · <code>@{command.name}</code></>}</>
      : command.kind === 'add_workspace' ? <code>{command.alias} · {command.parent}/{command.directory}</code>
      : <code>@{command.name} · {command.incarnation}</code>}</dd></div>
    {agent && <div><dt>Agent 实例</dt><dd><code>@{agent.name} · {agent.incarnation}</code></dd></div>}
  </dl>;
}

const isMissing = (error: unknown) =>
  error instanceof CloudAdminError && error.status === 404 && error.code !== 'InvalidErrorResponse';

function controlError(error: unknown, fallback: string) {
  return error instanceof CloudAdminError && error.status === 409
    ? '主机、Scope 代次、Agent 实例或角色可用性已变化。请核对后重新选择。'
    : explainError(error, fallback);
}

type Draft = { workspace: string; role: string; name: string; title: string; task: string };
type Phase =
  | { kind: 'draft' }
  | { kind: 'unknown'; input: CloudControlInput; canReplay: boolean }
  | { kind: 'saved'; input: CloudControlInput; receipt: CloudControlReceipt }
  | { kind: 'rejected' };
const emptyDraft: Draft = { workspace: '', role: '', name: '', title: '', task: '' };

function Catalogue({ value }: { value: CloudHostCapabilities }) {
  const date = value.observed_at == null ? undefined : new Date(Number(value.observed_at) * 1000);
  const timestamp = date && !Number.isNaN(date.valueOf()) ? date.toISOString() : undefined;
  return <div className="deployment-control-catalogue">
    <span>{value.online ? '管理在线' : '管理离线'}{timestamp && <> · 目录更新于 <Timestamp value={timestamp} /></>}</span>
    {value.installation_id && <span>安装标识 <code>{value.installation_id}</code></span>}
  </div>;
}

/**
 * @aiPurpose Confirm one cloud launch or exact-instance stop and retain its original receipt.
 * @aiUsage Keep mounted while the scope workspace is shown and clear the captured target on dismiss.
 * @aiBehavior Read the live installation catalogue, then permit exact-input replay only after an explicit receipt lookup finds no result.
 */
export function DeploymentControlDialog({ client, target, onClose }: {
  client: AdminClient; target: ControlTarget | null; onClose: () => void;
}) {
  const formId = useId();
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [phase, setPhase] = useState<Phase>({ kind: 'draft' });
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const pending = useRef<AbortController | null>(null);
  useEffect(() => () => pending.current?.abort(), []);
  useLayoutEffect(() => {
    if (!target) return;
    setDraft(emptyDraft);
    setPhase({ kind: 'draft' });
    setError('');
  }, [target]);

  const load = useCallback(async (signal: AbortSignal) => {
    if (!target) return null;
    const value = await client.getHostCapabilities(target.scope, target.host, { signal });
    if (value.generation !== target.generation) throw new Error('Scope changed');
    return value;
  }, [client, target]);
  const { read, retry } = useRead(load);
  const capabilities = read.kind === 'ready' ? read.data : null;
  const workspaces = capabilities?.workspaces ?? [];
  const workspace = workspaces.find(item => item.alias === draft.workspace);
  const roleAvailable = draft.role ? workspace?.roles.some(role => role.name === draft.role && role.available) : workspace?.default_available;
  const taskTooLong = new TextEncoder().encode(draft.task).length > 16 * 1024;
  const canSubmit = !!capabilities?.online && !!capabilities.installation_id && phase.kind === 'draft'
    && (target?.kind === 'stop' || (!!roleAvailable && !taskTooLong));
  const stopping = target?.kind === 'stop';
  const verb = stopping ? '停止 Agent' : '启动 Agent';
  const targetText = !target ? '' : stopping
    ? `${target.scope} / ${target.host} · @${target.name} · ${target.incarnation}`
    : `${target.scope} / ${target.host}`;

  function edit(patch: Partial<Draft>) {
    setDraft(value => ({ ...value, ...patch }));
    setError('');
  }

  async function execute(input: CloudControlInput, mode: 'submit' | 'lookup' | 'replay') {
    if (!target || pending.current) return;
    const controller = new AbortController();
    const recovering = phase.kind === 'unknown';
    pending.current = controller;
    setBusy(true);
    setError('');
    try {
      const opts = { signal: controller.signal };
      const receipt = await (mode === 'lookup' ? client.recoverControlOperation(target.scope, input, opts) : client.submitControlOperation(target.scope, input, opts));
      if (!controller.signal.aborted) setPhase({ kind: 'saved', input, receipt });
    } catch (failure) {
      if (controller.signal.aborted) return;
      if (mode === 'lookup' && recovering && isMissing(failure)) {
        setPhase({ kind: 'unknown', input, canReplay: true });
        setError('暂未找到原回执，可以重发原请求。');
      } else if (failure instanceof CloudAdminOutcomeUnknown || recovering) {
        setPhase({ kind: 'unknown', input, canReplay: false });
        if (!(failure instanceof CloudAdminOutcomeUnknown)) setError(controlError(failure, '无法确认原请求。请稍后查询。'));
      } else {
        if (mode === 'submit') {
          if (failure instanceof CloudAdminError) setPhase({ kind: 'rejected' });
          else return setError('请求格式无效。请检查表单。');
        }
        setError(controlError(failure, mode === 'lookup' ? '无法刷新回执。' : '操作被拒绝。'));
      }
    } finally {
      if (!controller.signal.aborted) setBusy(false);
      pending.current = null;
    }
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!target || !canSubmit || !capabilities) return;
    const command: CloudControlInput['command'] = target.kind === 'stop'
      ? { kind: 'stop', name: target.name, incarnation: target.incarnation }
      : { kind: 'launch', workspace: draft.workspace, role: draft.role || undefined, name: draft.name || undefined,
        title: draft.title || undefined, initial_prompt: draft.task || undefined };
    void execute({ generation: target.generation, host: target.host, installation_id: capabilities.installation_id,
      request_id: crypto.randomUUID(), command }, 'submit');
  }

  const dismiss = () => { if (!pending.current) onClose(); };
  const footer = <div className="dialog-buttons deployment-write-actions">
    <button type="button" disabled={busy} data-dialog-focus={stopping || undefined} onClick={dismiss}>
      {phase.kind === 'draft' ? '取消' : '关闭'}
    </button>
    {phase.kind === 'draft' && <button className={stopping ? 'danger' : 'primary'} type="submit" form={formId} disabled={busy || !canSubmit}>{busy ? '正在提交…' : `确认${verb}`}</button>}
    {(phase.kind === 'unknown' || phase.kind === 'saved') && <button type="button" disabled={busy} onClick={() => void execute(phase.input, 'lookup')}>{busy ? '正在查询…' : '查询原回执'}</button>}
    {phase.kind === 'unknown' && phase.canReplay && <button type="button" className="primary" disabled={busy} onClick={() => void execute(phase.input, 'replay')}>重发原请求</button>}
  </div>;

  return <InteractionDialog open={!!target} variant={stopping ? 'stop' : 'launch'} icon={stopping ? undefined : 'plus'}
    title={verb} description={targetText} closeLabel="关闭主机操作" onDismiss={dismiss} footer={footer}>
    {target && <form id={formId} className="launch-fields deployment-control-form" onSubmit={submit} autoComplete="off">
      {phase.kind === 'draft' && <>
        {read.kind === 'loading' && <p role="status">正在读取主机目录…</p>}
        {read.kind === 'error' && <div className="deployment-control-catalogue"><p role="alert">无法读取当前 Scope 的主机目录。</p><button type="button" onClick={retry}>重新读取</button></div>}
        {capabilities && <Catalogue value={capabilities} />}
        {capabilities && !capabilities.online && <div className="deployment-control-catalogue"><p role="status">主机当前无法接收管理操作。</p><button type="button" onClick={retry}>重新读取</button></div>}
        {!stopping && capabilities && <>
          {workspaces.length ? <>
            <label>工作区<select required value={draft.workspace} onChange={event => edit({ workspace: event.target.value, role: '' })}>
              <option value="">选择工作区</option>{workspaces.map(item => <option key={item.alias}>{item.alias}</option>)}
            </select></label>
            {workspace && <label>角色<select value={draft.role} onChange={event => edit({ role: event.target.value })}>
              <option value="" disabled={!workspace.default_available}>默认 Codex{!workspace.default_available && `（${workspace.default_reason ?? '不可用'}）`}</option>
              {workspace.roles.map(role => <option key={role.name} value={role.name} disabled={!role.available}>{role.name}{!role.available && `（${role.reason ?? '不可用'}）`}</option>)}
            </select></label>}
            <div className="deployment-control-identity">
              <label>Agent 短名（可选）<input value={draft.name} onChange={event => edit({ name: event.target.value })} spellCheck={false} /></label>
              <label>标题（可选）<input value={draft.title} onChange={event => edit({ title: event.target.value })} maxLength={160} /></label>
            </div>
            <label>初始任务（可选）<textarea rows={3} value={draft.task} onChange={event => edit({ task: event.target.value })} aria-invalid={taskTooLong} /></label>
            {taskTooLong && <p className="deployment-error" role="alert">初始任务最多 16 KiB。</p>}
          </> : <p role="status">该主机尚未公开工作区。</p>}
        </>}
      </>}
      {phase.kind === 'unknown' && <div className="deployment-write-note" role="status">
        <strong>提交结果未确认</strong>
        <code>{phase.input.request_id}</code>
        <span>关闭后仍可用此请求 ID 查询回执。</span>
      </div>}
      {phase.kind === 'saved' && <Receipt value={phase.receipt} />}
      {error && <p className="deployment-error" role="alert">{error}</p>}
    </form>}
  </InteractionDialog>;
}

/**
 * @aiPurpose Look up a cloud operation receipt by its original scope and request ID.
 * @aiUsage Mount inside the selected Scope and key by scope identity; form submission always reads without dispatching.
 */
export function DeploymentReceiptLookup({ client, scope, generation }: { client: AdminClient; scope: string; generation: string }) {
  const [draft, setDraft] = useState('');
  const [lookup, setLookup] = useState('');
  const load = useCallback(async (signal: AbortSignal) => {
    if (!lookup) return null;
    const receipt = await client.getControlOperation(scope, lookup, { signal });
    if (receipt.generation !== generation) throw new Error('Scope changed');
    return receipt;
  }, [client, scope, generation, lookup]);
  const { read, retry } = useRead(load);
  function submit(event: FormEvent) {
    event.preventDefault();
    const id = draft.trim();
    if (!/^[A-Za-z0-9_.:-]{1,128}$/.test(id) || id === '.' || id === '..') return;
    if (id === lookup) retry();
    else setLookup(id);
  }
  return <section className="deployment-receipt-search" aria-label="查询操作回执">
    <form onSubmit={submit}>
      <label>请求 ID<input value={draft} onChange={event => setDraft(event.target.value)} required maxLength={128}
        pattern="(?!\.{1,2}$)[A-Za-z0-9_.:\-]+" spellCheck={false} autoComplete="off" /></label>
      <button type="submit" disabled={!draft.trim()}>查询回执</button>
    </form>
    {lookup && read.kind === 'loading' && <p role="status">正在读取回执…</p>}
    {read.kind === 'error' && <p className="deployment-error" role="alert">{isMissing(read.error) ? '此 Scope 内未找到该请求。' : controlError(read.error, '无法读取回执。请重试。')}</p>}
    {read.kind === 'ready' && read.data && <Receipt value={read.data} />}
  </section>;
}
