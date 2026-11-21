/** @aiPurpose Edit scalar labels with exact-ref drafts and the shared durable operation recovery. */
import { useEffect, useState } from 'react';
import type { LabelsSnapshot } from '../api/native';
import type { Management } from '../use-management';
import { management } from '../api/client';
import { activeOperation, messageOf, readStored, refKey } from '../model';
import { labelPatch, labelsOf, type LabelDraft } from './model';

interface SavedDraft extends LabelDraft { receiptId?: string }

/**
 * @aiPurpose Change only edited label keys while retaining drafts on conflicts and uncertain outcomes.
 * @aiUsage Mount for a selected exact Agent; native labels support and a current revision are required to write.
 * @aiBehavior Persists drafts by generation and exact reference; operation recovery retains the original body and ID.
 */
export function LabelEditor({ model, onOperation, onOpenOperation }: { model: Management; onOperation: (id: string) => void; onOpenOperation: (id: string) => void }) {
  const agent = model.detail;
  const target = model.selected;
  const storageKey = `agentx.management.labels.v1:${model.descriptor?.generation}:${target ? refKey(target) : ''}${model.storageSuffix}`;
  const [draft, setDraft] = useState<SavedDraft | null>(() => readStored(storageKey, null));
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const supported = model.descriptor && 'supports_labels' in model.descriptor && model.descriptor.supports_labels === true;
  const labels = agent ? labelsOf(agent) : null;
  const revision = agent && 'labels_revision' in agent && typeof agent.labels_revision === 'string' ? agent.labels_revision : null;
  const canEdit = supported && labels !== null && revision !== null && target?.incarnation !== null;
  const receipt = draft ? model.receipts.find(item => item.id === draft.receiptId || (!draft.receiptId && item.kind === 'labels' && item.body.generation === model.descriptor?.generation && 'agent' in item.body && target && refKey(item.body.agent) === refKey(target) && (activeOperation(item) || item.operation?.status === 'Unknown'))) : undefined;
  const uncertain = !!receipt && (activeOperation(receipt) || receipt.operation?.status === 'Unknown');
  const changedRevision = !!draft && revision !== null && draft.revision !== revision;
  const failed = receipt?.rejected || receipt?.operation?.status === 'Failed';

  useEffect(() => {
    try { if (draft) localStorage.setItem(storageKey, JSON.stringify(draft)); else localStorage.removeItem(storageKey); }
    catch { setError('无法保存标签草稿；请保留此页面。'); }
  }, [draft, storageKey]);
  useEffect(() => {
    if (receipt?.operation?.status === 'Succeeded') { setDraft(null); setNotice('标签已保存。'); model.refresh(); }
  }, [receipt?.operation?.status]);

  function begin() {
    if (labels === null || revision === null) return;
    setNotice(''); setError('');
    setDraft({ baseline: labels, revision, rows: Object.entries(labels).sort(([a], [b]) => a.localeCompare(b)).map(([key, value]) => ({ key, value })) });
  }
  function change(index: number, field: 'key' | 'value', value: string) {
    setDraft(previous => previous ? { ...previous, rows: previous.rows.map((row, at) => at === index ? { ...row, [field]: value } : row) } : null);
  }
  async function rebase() {
    if (!draft || !target || !model.descriptor || uncertain) return;
    setBusy(true); setError('');
    try {
      const snapshot = await management.agents.labels.get({ generation: model.descriptor.generation, agent: target });
      // Preserve the user's intended partial changes, including explicit removals.
      const intended = labelPatch(draft);
      const merged = { ...snapshot.labels, ...intended.set };
      intended.remove.forEach(key => delete merged[key]);
      setDraft({ baseline: snapshot.labels, revision: snapshot.revision, rows: Object.entries(merged).map(([key, value]) => ({ key, value })) });
      setNotice('已刷新并保留草稿；请确认后保存。');
    } catch (error) { setError(messageOf(error)); }
    finally { setBusy(false); }
  }
  async function save() {
    if (!draft || busy || uncertain) return;
    setBusy(true); setError(''); setNotice('');
    try {
      const patch = labelPatch(draft);
      if (!Object.keys(patch.set).length && !patch.remove.length) { setNotice('没有标签变更。'); return; }
      await model.submit('labels', patch, id => {
        const saved = { ...draft, receiptId: id };
        localStorage.setItem(storageKey, JSON.stringify(saved));
        setDraft(saved);
        onOperation(id);
      });
    } catch (error) { setError(messageOf(error)); }
    finally { setBusy(false); }
  }
  const facts = labels === null ? [] : Object.entries(labels).sort(([a], [b]) => a.localeCompare(b));
  const disabled = busy || uncertain;
  return <details className="labels-disclosure"><summary>标签{labels === null ? ' · 未记录' : ` · ${facts.length}`}</summary>
    {!supported && <p className="muted">当前服务不支持标签</p>}
    {supported && labels === null && <p className="muted">此记录未记录标签，不能推断为空。</p>}
    {!draft && <div className="label-editor"><div className="label-facts">{facts.map(([key, value]) => <span className="label-fact" key={key}>{key}={value}</span>)}{labels !== null && !facts.length && <span className="muted">无标签</span>}</div><div><button disabled={!canEdit} onClick={begin}>编辑标签</button></div></div>}
    {draft && <div className="label-editor"><fieldset disabled={disabled || !canEdit}><legend className="muted">标签草稿</legend>{draft.rows.map((row, index) => <div className="label-row" key={index}><label>key<input aria-label={`标签 key ${index + 1}`} value={row.key} onChange={event => change(index, 'key', event.target.value)} /></label><label>value<input aria-label={`标签 value ${index + 1}`} value={row.value} onChange={event => change(index, 'value', event.target.value)} /></label><button aria-label={`删除标签 ${index + 1}`} onClick={() => setDraft(previous => previous ? { ...previous, rows: previous.rows.filter((_, at) => at !== index) } : null)}>×</button></div>)}<button onClick={() => setDraft(previous => previous ? { ...previous, rows: [...previous.rows, { key: '', value: '' }] } : null)}>添加标签</button></fieldset>
      {(changedRevision || failed) && <p className="error" role="alert">{receipt?.operation?.error || receipt?.issue || '标签版本已变化。'} 草稿已保留。</p>}
      {uncertain && <p className="receipt-note">结果尚未确认；保留原请求，先查询操作。</p>}
      <div className="label-actions"><button className="primary" disabled={disabled || !canEdit || changedRevision || failed} onClick={() => void save()}>保存标签</button><button disabled={disabled || !canEdit} onClick={() => void rebase()}>刷新并保留草稿</button><button disabled={disabled} onClick={() => { setDraft(null); setError(''); }}>放弃草稿</button>{receipt && <button onClick={() => onOpenOperation(receipt.id)}>查看标签操作</button>}</div>
    </div>}
    {notice && <p className="receipt-note" role="status">{notice}</p>}{error && <p className="error" role="alert">{error}</p>}
  </details>;
}
