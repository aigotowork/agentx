/** @aiPurpose Compose office facts, effective posts and directory actions inside the existing console. */
import { useEffect, useState } from 'react';
import type { AgentRef, CollaborationView, DirectoryEntry, RoleDefinitionView } from './api/native';
import type { Management } from './use-management';
import { management } from './api/client';
import { messageOf, needsAttention, readStored, refKey } from './model';
import { matchesOffice, membershipLabel } from './use-offices';
import { EmptyState, Icon, State } from './presentation';

/** @aiPurpose Prefill an explicit native Role launch. @aiUsage Pass the effective catalog workspace independently of cwd. */
export interface RoleLaunch { role: string; workspace: string }

/**
 * @aiPurpose Show saved collaboration provenance for an exact Agent projection.
 * @aiUsage Render native current or stopped facts; current catalogs never replace adoption.
 */
export function MembershipFacts({ facts }: { facts: CollaborationView }) {
  const role = facts.role;
  return <details className="membership-facts"><summary title={membershipLabel(facts)}><Icon name="workspace" />{facts.status === 'adopted' ? facts.role?.name ?? '无岗位' : facts.status === 'legacy' ? '旧记录' : '归属不可用'}</summary><dl><dt>归属状态</dt><dd>{facts.status}</dd><dt>工作区</dt><dd>{facts.status === 'adopted' ? facts.workspace ?? '全局（明确）' : facts.status === 'legacy' ? '旧记录（原生全局行为）' : '不可用'}</dd>
    <dt>岗位</dt><dd>{facts.status === 'adopted' ? role?.name ?? '无岗位（明确）' : '未确认'}</dd>{facts.reason && <><dt>原因</dt><dd>{facts.reason}</dd></>}
    {role && <><dt>采纳来源</dt><dd>{role.path}</dd><dt>命名空间</dt><dd>{role.namespace}</dd><dt>采纳配置</dt><dd>{role.profile}</dd><dt>默认声明</dt><dd>{role.default ? '是' : '否'}</dd><dt>描述</dt><dd>{role.description ?? '—'}</dd></>}
  </dl></details>;
}

function RoleRow({ role, onLaunch }: { role: RoleDefinitionView; onLaunch: () => void }) {
  return <article className="role-row"><div><strong>{role.name}</strong>{role.default && <span className="state">默认岗位</span>}<span className="agent-meta">{role.profile ?? '默认启动配置'} · {role.namespace}</span><details><summary>岗位来源</summary><p><code>{role.path}</code></p>{role.description && <p>{role.description}</p>}</details></div><button className="icon-button" onClick={onLaunch} aria-label={`启动岗位 ${role.name}`}><Icon name="plus" />启动</button></article>;
}

interface DirectoryDraft { directory: string; text: string; intent: string; thread: string; replyTo: string }
function DirectoryActions({ model, onOperation, onOpenOperation }: { model: Management; onOperation: (id: string) => void; onOpenOperation: (id: string) => void }) {
  const directoryDraftKey = 'agentx.management.directory-draft.v1' + model.storageSuffix;
  const [draft, setDraft] = useState<DirectoryDraft>(() => readStored(directoryDraftKey, { directory: '', text: '', intent: 'request', thread: '', replyTo: '' }));
  const [preview, setPreview] = useState<DirectoryEntry>();
  const [previewKey, setPreviewKey] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [previewBusy, setPreviewBusy] = useState(false);
  const generation = model.descriptor?.generation;
  const directory = draft.directory.trim();
  const key = JSON.stringify([generation, directory]);
  const currentPreview = previewKey === key ? preview : undefined;
  const pending = model.receipts.find(receipt => receipt.kind === 'directory-send' && 'directory' in receipt.body && receipt.body.directory === directory && receipt.body.generation === generation && needsAttention(receipt));
  const currentPath = model.offices.workspace.startsWith('/') ? model.offices.workspace : undefined;
  const tooLong = new TextEncoder().encode(draft.text).length > (model.descriptor?.max_text ?? 65536);
  const valid = !!generation && directory.startsWith('/');
  useEffect(() => { try { localStorage.setItem(directoryDraftKey, JSON.stringify(draft)); } catch { /* Preserve in memory. */ } }, [draft, directoryDraftKey]);
  async function inspect() {
    if (!generation || !valid) return;
    setPreviewBusy(true); setError(''); setPreview(undefined);
    try {
      const entry = await management.directory.inspect({ generation, directory });
      if (entry.generation !== generation) throw new Error('路由数据代次已变化');
      setPreview(entry); setPreviewKey(key);
    } catch (error) { setError(messageOf(error)); }
    finally { setPreviewBusy(false); }
  }
  async function send() {
    if (!valid || !currentPreview || !draft.text.trim() || tooLong || busy || pending) return;
    setBusy(true); setError('');
    try { await model.submit('directory-send', { directory, message: { attachments: [], text: draft.text, intent: draft.intent || null, thread: draft.thread || null, reply_to: draft.replyTo || null } }, onOperation); }
    catch (error) { setError(messageOf(error)); }
    finally { setBusy(false); }
  }
  return <section className="directory-actions" aria-label="目录路由与发送"><h2 className="icon-heading"><Icon name="route" />目录路由</h2><form className="directory-preview-control" onSubmit={event => { event.preventDefault(); void inspect(); }}><label>目标目录<input aria-label="路由目标目录" placeholder={currentPath ?? '输入绝对路径'} value={draft.directory} onChange={event => { setError(''); setDraft(previous => ({ ...previous, directory: event.target.value })); }} /></label><button disabled={!valid || previewBusy}>{previewBusy ? '预览中…' : '预览路由'}</button></form>
    {currentPreview && <div className="route-facts"><div className="route-chain"><span>路由预览</span><Icon name="chevron" /><code>{currentPreview.resolved_workspace ?? '全局'}</code><Icon name="chevron" /><strong>{currentPreview.role.name}</strong></div><details><summary>解析依据 · {currentPreview.endpoint.status}</summary><dl><dt>目录</dt><dd>{currentPreview.directory}</dd><dt>所选工作区</dt><dd>{currentPreview.requested_workspace ?? '全局'}</dd><dt>默认声明工作区</dt><dd>{currentPreview.resolved_workspace ?? '全局'}</dd><dt>岗位</dt><dd>{currentPreview.role.name} · {currentPreview.role.profile}</dd><dt>岗位来源</dt><dd>{currentPreview.role.path}</dd><dt>端点</dt><dd>{currentPreview.endpoint.status} · 候选 {currentPreview.endpoint.candidates}</dd><dt>精确成员</dt><dd>{currentPreview.endpoint.participant ? JSON.stringify(currentPreview.endpoint.participant) : '—'}</dd>{currentPreview.endpoint.reason && <><dt>原因</dt><dd>{currentPreview.endpoint.reason}</dd></>}</dl></details></div>}
    <form className="directory-composer" onSubmit={event => { event.preventDefault(); void send(); }}><label>目录消息<textarea aria-label="目录消息内容" placeholder="写给此目录默认岗位的任务…" value={draft.text} onChange={event => setDraft(previous => ({ ...previous, text: event.target.value }))} /></label><div className="composer-toolbar"><label className="inline-field">意图<select value={draft.intent} onChange={event => setDraft(previous => ({ ...previous, intent: event.target.value }))}><option value="request">请求</option><option value="inform">告知</option><option value="ack">确认</option><option value="">未指定</option></select></label><button className="primary send-button icon-button" disabled={!currentPreview || !draft.text.trim() || tooLong || busy || !!pending}>{busy ? '提交中…' : '向目录发送'}<Icon name="send" /></button></div><details><summary>回复与话题</summary><div className="message-options"><label>回复 ID<input value={draft.replyTo} onChange={event => setDraft(previous => ({ ...previous, replyTo: event.target.value }))} /></label><label>话题<input value={draft.thread} onChange={event => setDraft(previous => ({ ...previous, thread: event.target.value }))} /></label></div></details>
    <p className="muted">{currentPreview ? '发送时将重新解析岗位，可能创建或采纳成员。实际收件人见回执。' : '发送前请先预览此目录的路由。'}</p></form>
    {(error || tooLong) && <p className="error" role="alert">{error || '目录消息超过允许长度'}</p>}
    {pending && <div className="warning-note">此目录有未完成或未知操作。<button className="quiet" onClick={() => onOpenOperation(pending.id)}>查看原请求</button></div>}
  </section>;
}

/**
 * @aiPurpose Browse known workspaces, effective roles, loaded members and directory routing.
 * @aiUsage Keep mounted across destinations to retain the input and advisory preview.
 */
export function OfficeWorkspace({ model, onLaunch, onMember, onOperation, onOpenOperation }: { model: Management; onLaunch: (launch: RoleLaunch) => void; onMember: (agent: AgentRef) => void; onOperation: (id: string) => void; onOpenOperation: (id: string) => void }) {
  const offices = model.offices;
  const [workspaceInput, setWorkspaceInput] = useState(offices.workspace);
  const [pickerOpen, setPickerOpen] = useState(false);
  useEffect(() => setWorkspaceInput(offices.workspace), [offices.workspace]);
  const coverage = offices.catalog?.coverage;
  const coverageLabel = coverage ? coverage.agent_limit_reached || coverage.scan_limit_reached || coverage.errors.length ? '发现不完整' : '有限发现完成' : '发现读取中';
  const chooseWorkspace = (workspace: string) => { offices.setWorkspace(workspace); setPickerOpen(false); };
  const canChoose = workspaceInput.trim() === 'global' || workspaceInput.trim().startsWith('/');
  const selectedWorkspace = offices.workspace === 'global' ? '全局岗位' : offices.workspace;
  const members = model.agents.filter(agent => matchesOffice(agent, offices.workspace));
  return <div className="workspace workspaces-workspace">
    <aside className={pickerOpen ? 'collection-rail workspace-index expanded' : 'collection-rail workspace-index'} aria-label="工作区索引">
      <button className="mobile-picker icon-button" onClick={() => setPickerOpen(value => !value)} aria-expanded={pickerOpen}><Icon name="workspace" />工作区索引<Icon name={pickerOpen ? 'down' : 'chevron'} /></button>
      <div className="workspace-index-content">
        <header className="collection-header section-heading"><h2>工作区索引</h2><button className="quiet icon-control" onClick={offices.refresh} aria-label="刷新工作区" title="刷新工作区"><Icon name="refresh" /></button></header>
        <details className="coverage"><summary>{coverageLabel} · {offices.catalog?.workspaces.length ?? '—'} 个已知工作区</summary>{coverage && <dl><dt>Agent 页</dt><dd>{coverage.agent_pages}{coverage.agent_limit_reached ? ' · 已达上限' : ''}</dd><dt>扫描目录</dt><dd>{coverage.directories_examined}{coverage.scan_limit_reached ? ' · 已达上限' : ''}</dd></dl>}{coverage?.errors.map((error, index) => <p className="error" key={index}>{error}</p>)}</details>
        {offices.catalogError && <p className="error" role="alert">{offices.catalogError}</p>}
        <div className="collection-list">
          <button className="collection-row workspace-row icon-button" aria-pressed={offices.workspace === 'global'} onClick={() => chooseWorkspace('global')}><Icon name="workspace" />全局岗位</button>
          {offices.catalog?.workspaces.map(office => <button className="collection-row workspace-row" key={office.path} aria-pressed={offices.workspace === office.path} onClick={() => chooseWorkspace(office.path)}><strong>{office.path}</strong><span className="muted">{office.sources.join(' · ')}</span></button>)}
        </div>
      </div>
    </aside>
    <div className="office-workspace">
      <header className="workspace-heading"><span className="context-label">工作区</span><h1>{selectedWorkspace}</h1></header>
      <div className="office-columns">
        <div className="office-catalog">
          <form className="directory-preview-control" onSubmit={event => { event.preventDefault(); if (canChoose) chooseWorkspace(workspaceInput.trim()); }}><label>查询其他工作区<input aria-label="岗位查询办公室" list="known-office-paths" value={workspaceInput} onChange={event => setWorkspaceInput(event.target.value)} /></label><button disabled={!canChoose || !model.descriptor}>查看岗位</button></form>
          <datalist id="known-office-paths"><option value="global" />{offices.catalog?.workspaces.map(office => <option key={office.path} value={office.path} />)}</datalist>
          <section className="office-section"><h2>有效岗位 <span className="count">{offices.roles?.roles.length ?? '—'}</span></h2>
          {offices.roleError ? <p className="error" role="alert">{offices.roleError}</p> : !offices.roles ? <p className="muted">读取岗位…</p> : !offices.roles.roles.length ? <p className="muted">此工作区无有效岗位</p> : offices.roles.roles.map(role => <RoleRow key={role.path} role={role} onLaunch={() => onLaunch({ role: role.name, workspace: offices.roles?.workspace ?? 'global' })} />)}</section>
          <section className="office-section"><h2>已加载成员 <span className="count">{members.length}</span></h2>
            {members.map(agent => <button className="member-row" key={refKey(agent.agent)} onClick={() => onMember(agent.agent)}><strong>@{agent.agent.name}</strong><State value={agent.status} /><Icon name="chevron" /></button>)}
            {!members.length && <EmptyState icon="agents" title="已加载列表无匹配成员" />}
            {(model.hasMore || model.labelSelectors.length > 0) && <p className="muted">仅显示 Agents 当前已加载的成员。</p>}
          </section>
        </div>
        <DirectoryActions model={model} onOperation={onOperation} onOpenOperation={onOpenOperation} />
      </div>
    </div>
  </div>;
}
