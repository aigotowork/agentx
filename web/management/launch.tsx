import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import type { RoleLaunch } from './offices';
import type { Management } from './use-management';
import type { LaunchInput } from './api/native';
import { messageOf, readStored } from './model';
import { Icon } from './presentation';
import { InteractionDialog } from './dialog';

type LaunchMode = 'profile' | 'role';
interface LaunchDraft { title?: string; mode?: LaunchMode; role: string; workspace: string; source: string; directory: string; prompt: string }
const emptyDraft: LaunchDraft = { role: '', workspace: '', source: '', directory: '', prompt: '' };

/**
 * @aiPurpose Make launch source, working location and initial task explicit in one saved draft.
 * @aiUsage Mount once; open with an optional exact Role and independent workspace prefill.
 * @aiBehavior Closing preserves the draft; submission hands off to the exact journaled receipt.
 */
export function LaunchDialog({ model, open, onClose, onOperation, roleLaunch }: { roleLaunch?: RoleLaunch; model: Management; open: boolean; onClose: () => void; onOperation: (id: string) => void }) {
  const draftKey = 'agentx.management.launch.v1' + model.storageSuffix;
  const [draft, setDraft] = useState<LaunchDraft>(() => readStored(draftKey, emptyDraft));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [storageIssue, setStorageIssue] = useState(false);
  const [workspaceTouched, setWorkspaceTouched] = useState(false);
  const [attempted, setAttempted] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [invalidFocus, setInvalidFocus] = useState<{ selector: string }>();
  const formId = useId();
  const form = useRef<HTMLFormElement>(null);
  const mode = draft.mode ?? (draft.role?.trim() ? 'role' : 'profile');
  const descriptor = model.descriptor;
  const sources = [
    ...(descriptor?.profiles ?? []).map(profile => ({ value: `profile:${profile.name}`, label: `${profile.name} · ${profile.tool}`, description: profile.description, kind: 'profile' })),
    ...(descriptor?.tools ?? []).map(tool => ({ value: `tool:${tool}`, label: tool, description: '', kind: 'tool' })),
  ];
  const source = draft.source || sources[0]?.value || '';
  const selectedSource = sources.find(item => item.value === source);
  const role = draft.role?.trim() || '';
  const validSource = mode === 'role' ? !!role : !!selectedSource;
  const workspace = draft.workspace?.trim() || '';
  const validWorkspace = !workspace || workspace === 'global' || workspace.startsWith('/');
  const promptBytes = new TextEncoder().encode(draft.prompt).length;
  const maxBytes = descriptor?.max_text ?? 65536;
  const promptTooLong = promptBytes > maxBytes;
  const sourceInvalid = attempted && !validSource;
  const workspaceInvalid = (workspaceTouched || attempted) && !validWorkspace;
  const draftPresent = !!(draft.title || draft.role || draft.workspace || draft.directory || draft.prompt || draft.source);
  const overrides = [workspace && `工作区 ${workspace}`, draft.directory.trim() && `目录 ${draft.directory.trim()}`, draft.title?.trim() && `标题 ${draft.title.trim()}`].filter(Boolean).join(' · ');

  useLayoutEffect(() => {
    if (open && roleLaunch) setDraft(previous => ({ ...previous, mode: 'role', role: roleLaunch.role, workspace: roleLaunch.workspace }));
    if (!open) { setSettingsOpen(false); setWorkspaceTouched(false); setAttempted(false); setError(''); }
  }, [open, roleLaunch]);
  useLayoutEffect(() => {
    if (invalidFocus) form.current?.querySelector<HTMLElement>(invalidFocus.selector)?.focus();
  }, [invalidFocus]);
  useEffect(() => {
    try { localStorage.setItem(draftKey, JSON.stringify(draft)); setStorageIssue(false); }
    catch { setStorageIssue(true); }
  }, [draft, draftKey]);

  function update<K extends keyof LaunchDraft>(key: K, value: LaunchDraft[K]) {
    setDraft(previous => ({ ...previous, [key]: value }));
  }
  async function launch() {
    if (busy || !descriptor) return;
    setAttempted(true); setError('');
    if (!validSource || !validWorkspace || promptTooLong) {
      if (validSource && !validWorkspace) setSettingsOpen(true);
      setInvalidFocus({ selector: !validSource ? '[data-launch-source]' : !validWorkspace ? '[data-launch-workspace]' : '[data-launch-prompt]' });
      return;
    }
    const input: LaunchInput = {
      labels: {}, title: draft.title?.trim() || null, role: mode === 'role' ? role : null, workspace: workspace || null,
      profile: mode === 'profile' && selectedSource?.kind === 'profile' ? source.slice(8) : null,
      tool: mode === 'profile' && selectedSource?.kind === 'tool' ? source.slice(5) : null,
      directory: draft.directory.trim() || null, prompt: draft.prompt || null,
    };
    setBusy(true);
    try {
      await model.submit('launch', input, id => { onOperation(id); onClose(); });
    } catch (error) { setError(messageOf(error)); }
    finally { setBusy(false); }
  }

  const footer = <>
    <div className="dialog-footer-meta">{storageIssue ? '草稿仅保留在当前页面' : draftPresent ? '草稿已保留' : null}</div>
    <div className="dialog-buttons"><button type="button" className="quiet dialog-secondary" onClick={onClose}>取消</button><button form={formId} className="primary" aria-label="启动" disabled={!descriptor || busy}>{busy ? '提交中…' : '启动 Agent'}</button></div>
  </>;
  return <InteractionDialog open={open} variant="launch" title="启动 Agent" description="创建一个后台执行实例。" closeLabel="关闭启动面板" onDismiss={onClose} footer={footer}>
    <form ref={form} id={formId} className="launch-fields" onSubmit={event => { event.preventDefault(); void launch(); }}>
      <div className="launch-source">
        <div className="launch-modes" role="group" aria-label="选择启动方式">
          <button type="button" aria-pressed={mode === 'profile'} onClick={() => update('mode', 'profile')}>按配置或工具</button>
          <button type="button" aria-pressed={mode === 'role'} onClick={() => update('mode', 'role')}>按岗位</button>
        </div>
        <div className="launch-mode-fields" key={mode}>
          {mode === 'role' ? <label>岗位名称<input data-launch-source aria-label="启动岗位" aria-invalid={sourceInvalid || undefined} aria-describedby={sourceInvalid ? `${formId}-source-error` : undefined} placeholder="例如 reviewer 或 global:reviewer" value={draft.role ?? ''} onChange={event => update('role', event.target.value)} /></label> :
            <label>运行配置<select data-launch-source aria-invalid={sourceInvalid || undefined} aria-describedby={sourceInvalid ? `${formId}-source-error` : undefined} value={selectedSource ? source : ''} onChange={event => update('source', event.target.value)}>{!selectedSource && <option value="">选择一项配置或工具</option>}{sources.map(item => <option key={item.value} value={item.value}>{item.label}</option>)}</select></label>}
          {sourceInvalid && <p id={`${formId}-source-error`} className="field-error"><Icon name="warning" />{mode === 'role' ? '输入要使用的岗位名称。' : '选择可用的配置或工具。'}</p>}
          {mode === 'profile' && selectedSource?.description && <p className="muted">{selectedSource.description}</p>}
        </div>
      </div>
      <div className="launch-task">
        <label htmlFor={`${formId}-prompt`}><span>初始任务 <small className="muted">可选</small></span><textarea id={`${formId}-prompt`} data-launch-prompt rows={3} aria-invalid={promptTooLong || undefined} aria-describedby={promptTooLong ? `${formId}-prompt-error` : undefined} placeholder="描述要交给 Agent 的第一件事…" value={draft.prompt} onChange={event => update('prompt', event.target.value)} /></label>
        {promptTooLong && <p id={`${formId}-prompt-error`} className="field-error"><Icon name="warning" />初始任务超出 {maxBytes.toLocaleString('zh-CN')} 字节上限。</p>}
      </div>
      <div className="launch-settings">
        <button type="button" className="quiet launch-settings-toggle" aria-expanded={settingsOpen} aria-controls={`${formId}-settings`} onClick={() => setSettingsOpen(value => !value)}><Icon name="chevron" /><span>位置与标题</span><span className="setting-value ellipsis" title={overrides || undefined}>{overrides || '使用默认值'}</span></button>
        <div className="launch-settings-fields" id={`${formId}-settings`} hidden={!settingsOpen}>
          <div className="launch-field-pair">
            <label>归属工作区<input data-launch-workspace aria-label="启动办公室" aria-invalid={workspaceInvalid || undefined} aria-describedby={workspaceInvalid ? `${formId}-workspace-error` : undefined} placeholder="自动选择工作区" value={draft.workspace ?? ''} onChange={event => update('workspace', event.target.value)} onBlur={() => setWorkspaceTouched(true)} /></label>
            <label>工作目录<input aria-label="启动目录" placeholder="使用项目默认目录" value={draft.directory} onChange={event => update('directory', event.target.value)} /></label>
          </div>
          {workspaceInvalid && <p id={`${formId}-workspace-error`} className="field-error"><Icon name="warning" />使用绝对路径，或输入 global。</p>}
          <label>职责标题<input aria-label="职责标题" placeholder="例如：核对发布结果" value={draft.title ?? ''} onChange={event => update('title', event.target.value)} maxLength={160} /></label>
        </div>
      </div>
      {error && <p className="field-error" role="alert"><Icon name="warning" />{error}</p>}
    </form>
  </InteractionDialog>;
}
