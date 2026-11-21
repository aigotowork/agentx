import { stringifyJson } from '@agentx/client';
import { useState } from 'react';
import type { PluginTheme } from '../../../host/contracts';
import type { Descriptor } from '../../../host/live/client';
import { at, configure, latestObservations, type Snapshot } from '../../../host/live/inspection';
import { unknown, referenceKey } from '../../../host/live/model';
import { Button } from '../../../uiux/systems/inspector/inspector.native';
import { useInspectionMutations } from './use-mutations';

/** @aiPurpose Show authoritative collection start/coverage and explicitly enable or pause durable sampling.
 * @aiUsage Pass a same-generation Snapshot and refresh; disabling also disables recovery atomically. */
export function CollectionBlock({ theme, dataset, snapshot, refresh }: { theme: PluginTheme; dataset: Descriptor; snapshot?: Snapshot; refresh: () => Promise<unknown> }) {
  const mutation = useInspectionMutations(dataset, snapshot, refresh);
  const toggle = () => {
    if (!snapshot) return;
    const input = { Configure: { generation: dataset.generation, expected_revision: snapshot.settings.revision, enabled: !snapshot.settings.enabled, recovery: { ...snapshot.settings.recovery, enabled: snapshot.settings.enabled ? false : snapshot.settings.recovery.enabled } } };
    void mutation.run(input, () => configure(input));
  };
  if (!snapshot) return <p className="native-muted">采集状态未知</p>;
  return <section aria-label="采集控制"><div className="native-actions"><strong>{snapshot.settings.enabled ? '采集中' : '采集已关闭'}</strong><Button theme={theme} size="sm" label={snapshot.settings.enabled ? '暂停采集与自动继续' : '启用采集'} disabled={mutation.blocked} onPress={toggle} /><span className="native-muted">历史起点 {at(snapshot.settings.collected_from)}</span></div><dl className="native-facts"><dt>最近观测</dt><dd>{at(snapshot.coverage.observed_at)}</dd><dt>保留起点</dt><dd>{at(snapshot.coverage.retained_from)}</dd><dt>发现覆盖</dt><dd>{snapshot.coverage.observed_at === null ? '未知' : snapshot.coverage.discovery_complete ? '本次发现完整' : '部分 / 未完成'}</dd><dt>观测丢弃</dt><dd>{snapshot.coverage.dropped_observations}</dd></dl><details><summary>来源与覆盖边界</summary><p>{snapshot.coverage.basis}</p><p>历史从采集启用开始；当前累计用量可以包含更早的 Provider 用量，不作历史回填。</p>{snapshot.coverage.gaps.map((gap, index) => <p key={index}>{at(gap.from)} → {gap.through === null ? '至今' : at(gap.through)} · {gap.reason}</p>)}</details>{mutation.error && <p className="native-error" role="alert">{mutation.error}</p>}{mutation.pending.length > 0 && <div className="native-actions"><span>请求结果待核对</span><Button theme={theme} size="sm" label="读取原状态" onPress={() => void refresh()} /></div>}</section>;
}

/** @aiPurpose Display sampled native status/model evidence with exact references and bounded visible rows.
 * @aiUsage These are collected snapshots, never complete lifecycle events or task completion facts. */
export function ObservationTable({ snapshot }: { snapshot?: Snapshot }) {
  const [limit, setLimit] = useState(50);
  if (!snapshot) return <p className="native-muted">观测数据未知</p>;
  const observations = [...snapshot.observations].reverse();
  return <section><p className="native-muted">状态采样 · {observations.length} 条保留观测 · {at(snapshot.settings.collected_from)} 起</p>{!observations.length && <p>尚无采集样本</p>}{observations.slice(0, limit).map(observation => <details key={observation.id}><summary>{at(observation.at)} · {observation.agent.name} · {observation.status} · {observation.model_request?.phase ?? '模型状态未知'}</summary><dl className="native-facts"><dt>观测 ID</dt><dd>{observation.id}</dd><dt>精确引用</dt><dd>{referenceKey(observation.agent)}</dd><dt>Provider 会话</dt><dd>{observation.session_id ?? '未知'}</dd><dt>用量来源</dt><dd>{observation.sample?.availability ?? '未知'}</dd><dt>模型</dt><dd>{observation.model_request?.model ?? '未知'}</dd></dl>{observation.model_request && <pre>{stringifyJson(observation.model_request)}</pre>}</details>)}{observations.length > limit && <button className="native-resource" onClick={() => setLimit(value => value + 50)}>继续显示采集样本</button>}</section>;
}

/** @aiPurpose Display exact decimal native usage checkpoints and unknown provenance without summing repeated polls/sources.
 * @aiUsage Input and cache counters are disjoint; reasoning is an output subset; missing parser fields may normalize to zero. */
export function TokenTable({ snapshot }: { snapshot?: Snapshot }) {
  if (!snapshot) return <p className="native-muted">用量数据未知</p>;
  const rows = latestObservations(snapshot);
  return <section><p className="native-muted">最新采样的会话累计值 · 不跨采样或共享来源相加 · 缺失字段保持未知</p>{!rows.length ? <p>尚无用量样本</p> : <div style={{ overflowX: 'auto' }}><table><thead><tr><th>实例 / 模型</th><th>来源 / 时间</th><th>输入</th><th>输出</th><th>缓存读 / 写</th><th>推理（输出子集）</th><th>上下文</th></tr></thead><tbody>{rows.map(observation => {
    const usage = observation.sample?.availability === 'available' ? observation.sample.usage : null;
    return <tr key={referenceKey(observation.agent)}><td>{observation.agent.name}<br /><span className="native-muted">{usage?.model ?? observation.model_request?.model ?? '模型未知'}</span></td><td><details><summary>{observation.sample?.availability ?? '未知'} · {at(observation.at)}</summary><p>{referenceKey(observation.agent)}</p><p>{observation.sample?.source_key ?? '来源未知'}</p><p>{usage?.unit ?? '单位未知'} · {usage?.basis ?? 'Provider 未提供用量'}</p></details></td><td>{unknown(usage?.input)}</td><td>{unknown(usage?.output)}</td><td>{unknown(usage?.cache_read)} / {unknown(usage?.cache_write)}</td><td>{unknown(usage?.reasoning)}</td><td>{unknown(usage?.context_tokens)} / {unknown(usage?.context_window)}{usage?.context_usage_ratio != null && <small> · {(usage.context_usage_ratio * 100).toFixed(1)}%</small>}</td></tr>;
  })}</tbody></table></div>}<details><summary>计数含义</summary><p>十进制原生计数保持精度。input、cache_read、cache_write、output 为互斥计数；reasoning 属于 output。parser_normalized 的 0 不证明 Provider 明确报告了该字段为 0。这里没有速率、调用级账本或历史增量推断。</p></details></section>;
}
