import { stringifyJson } from '@agentx/client';
import { useState } from 'react';
import type { PluginTheme } from '../../../host/contracts';
import type { Descriptor } from '../../../host/live/client';
import { analyze, at, cancelAnalysis, type Snapshot } from '../../../host/live/inspection';
import { requestId } from '../../../host/live/operations';
import { Button } from '../../../uiux/systems/inspector/inspector.native';
import { useInspectionMutations } from './use-mutations';

/** @aiPurpose Submit pinned native evidence summaries and inspect durable jobs/reports in the retained analysis destination.
 * @aiUsage Select 1–16 retained observation IDs; server performs deterministic analysis, never an implied AI model run.
 * @aiBehavior Lost submit replies reconcile by persisted request/job identity; no automatic resubmission. */
export function AnalysisBlock({ theme, dataset, snapshot, refresh }: { theme: PluginTheme; dataset: Descriptor; snapshot?: Snapshot; refresh: () => Promise<unknown> }) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const mutation = useInspectionMutations(dataset, snapshot, refresh);
  const observations = snapshot ? [...snapshot.observations].reverse() : [];
  const staleSelection = [...selected].some(id => !observations.some(observation => observation.id === id));
  const toggle = (id: string) => setSelected(previous => { const next = new Set(previous); if (next.has(id)) next.delete(id); else if (next.size < 16) next.add(id); return next; });
  const submit = () => {
    const input = { Analyze: { generation: dataset.generation, request_id: requestId(), observation_ids: [...selected] } };
    void mutation.run(input, () => analyze(input));
  };
  const cancel = (id: string) => { const input = { CancelAnalysis: { generation: dataset.generation, id } }; void mutation.run(input, () => cancelAnalysis(input)); };
  return <section><h2>证据分析</h2><div className="native-actions"><span className="native-muted">确定性摘要 · 已选 {selected.size} / 16</span><Button theme={theme} label="生成证据摘要" size="sm" disabled={mutation.blocked || !selected.size || staleSelection || !snapshot} onPress={submit} /><Button theme={theme} label="清空选择" size="sm" disabled={!selected.size} onPress={() => setSelected(new Set())} /></div>{staleSelection && <p role="alert" className="native-error">所选样本已超出保留范围，请重新选择。</p>}{mutation.error && <p role="alert" className="native-error">{mutation.error}</p>}{mutation.pending.length > 0 && <div className="native-actions"><span>原请求结果待核对</span><Button theme={theme} label="读取作业回执" size="sm" onPress={() => void refresh()} /></div>}
    <details><summary>选择采集证据（{observations.length}）</summary>{!observations.length && <p>启用采集后才有可分析的样本。</p>}{observations.map(observation => <label key={observation.id} className="native-resource"><input type="checkbox" checked={selected.has(observation.id)} disabled={!selected.has(observation.id) && selected.size >= 16} onChange={() => toggle(observation.id)} /> {at(observation.at)} · {observation.agent.name} · {observation.status} <span className="native-muted">#{observation.id}</span></label>)}</details>
    <h3>分析作业 / 报告</h3>{snapshot && !snapshot.jobs.length && <p className="native-muted">尚无分析作业</p>}{snapshot?.jobs.map(job => <details key={job.id}><summary>{at(job.created_at)} · {job.status} · {job.observation_ids.length} 个样本 · {job.id}</summary><p className="native-muted">输入 revision {job.input_revision} · 固定样本 {job.observation_ids.join(', ')}</p><details><summary>提交时的覆盖范围</summary><pre>{stringifyJson(job.input_coverage)}</pre></details>{job.status === 'queued' && <Button theme={theme} label="取消待执行作业" size="sm" disabled={mutation.blocked} onPress={() => cancel(job.id)} />}{job.report && <><dl className="native-facts"><dt>算法</dt><dd>{job.report.algorithm}</dd><dt>观测数 / 实例数</dt><dd>{job.report.observation_count} / {job.report.distinct_agents}</dd><dt>原生错误采样</dt><dd>{job.report.native_error_count}</dd><dt>用量来源数</dt><dd>{job.report.usage_sources}</dd><dt>用量未知采样</dt><dd>{job.report.unknown_usage_count}</dd></dl>{job.report.caveats.map((caveat, index) => <p className="native-muted" key={index}>{caveat}</p>)}<details><summary>完整原生报告与覆盖</summary><pre>{stringifyJson(job.report)}</pre></details></>}</details>)}
  </section>;
}
