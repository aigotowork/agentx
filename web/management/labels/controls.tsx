/** @aiPurpose Own compact server label selection and explicitly local grouping controls. */
import { useEffect, useState } from 'react';
import { Icon } from '../presentation';

interface SelectionProps { supported: boolean; selectors: string[]; onApply: (selectors: string[]) => void; groupKey: string; onGroup: (key: string) => void }
/**
 * @aiPurpose Select native label predicates before pagination and group the loaded result only.
 * @aiUsage Mount in the resource rail; the parent owns applied selectors and grouping.
 */
export function LabelSelection({ supported, selectors, onApply, groupKey, onGroup }: SelectionProps) {
  const [draft, setDraft] = useState(selectors.join('\n'));
  const [error, setError] = useState('');
  useEffect(() => setDraft(selectors.join('\n')), [selectors]);
  function apply() {
    const values = draft.split('\n').filter(value => value.length > 0);
    if (values.length > 16) { setError('最多 16 个条件。'); return; }
    setError(''); onApply(values);
  }
  return <details className="label-selection"><summary>标签筛选{selectors.length ? ` · ${selectors.length}` : ''} / 分组</summary><div className="label-controls">{!supported && <p className="muted">当前服务不支持标签</p>}<fieldset disabled={!supported}>
    <label>服务端条件（每行一个，AND）<textarea aria-label="标签筛选条件" placeholder={'project=atlas\n!review/state'} value={draft} onChange={event => setDraft(event.target.value)} /></label>
    <div className="label-actions"><button onClick={apply}>应用标签筛选</button><details className="label-syntax"><summary><Icon name="info" />条件语法</summary><p className="muted"><code>key=value</code>、<code>key!=value</code>、<code>key</code>、<code>!key</code>。全部满足才会匹配，筛选在服务端分页前执行。</p></details></div>
    <label>按 key 分组 · 仅已加载<input aria-label="标签分组 key" placeholder="例如 project" value={groupKey} onChange={event => onGroup(event.target.value)} /></label>
    </fieldset>{selectors.length > 0 && <button onClick={() => { setDraft(''); onApply([]); }}>清除标签筛选</button>}{error && <p className="error" role="alert">{error}</p>}
  </div></details>;
}
