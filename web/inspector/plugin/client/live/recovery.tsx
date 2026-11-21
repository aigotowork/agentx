import { useEffect, useState } from 'react';
import type { PluginTheme } from '../../../host/contracts';
import type { Descriptor } from '../../../host/live/client';
import { at, configure, continuation, latestObservations, type Episode, type Snapshot } from '../../../host/live/inspection';
import { referenceKey } from '../../../host/live/model';
import { Button } from '../../../uiux/systems/inspector/inspector.native';
import { useInspectionMutations } from './use-mutations';

/** @aiPurpose Configure daemon-owned sustained-error continuation and show exact-episode mailbox acceptance/audit.
 * @aiUsage Supply an authoritative Snapshot; queued means a continuation request is accepted, never Provider recovery.
 * @aiBehavior Native guards recheck live episode/phase/process and once-only barrier atomically; browser never runs a recovery timer. */
export function RecoveryBlock({ theme, dataset, snapshot, refresh }: { theme: PluginTheme; dataset: Descriptor; snapshot?: Snapshot; refresh: () => Promise<unknown> }) {
  const [enabled, setEnabled] = useState(snapshot?.settings.recovery.enabled ?? false);
  const [seconds, setSeconds] = useState(String(snapshot?.settings.recovery.error_duration_seconds ?? 600));
  const [revision, setRevision] = useState(snapshot?.settings.revision ?? 0n);
  const [dirty, setDirty] = useState(false);
  const mutation = useInspectionMutations(dataset, snapshot, refresh);
  const load = () => {
    if (!snapshot) return;
    setEnabled(snapshot.settings.recovery.enabled); setSeconds(String(snapshot.settings.recovery.error_duration_seconds)); setRevision(snapshot.settings.revision); setDirty(false);
  };
  useEffect(() => { if (!dirty) load(); }, [snapshot?.settings.revision]);
  const duration = Number(seconds);
  const valid = /^\d+$/.test(seconds) && Number.isSafeInteger(duration) && duration >= 1 && duration <= 604800;
  const save = () => {
    if (!snapshot || !valid) return;
    const input = { Configure: { generation: dataset.generation, expected_revision: revision, enabled: snapshot.settings.enabled, recovery: { enabled, error_duration_seconds: duration } } };
    void mutation.run(input, () => configure(input)).then(result => { if (result) { setEnabled(result.settings.recovery.enabled); setSeconds(String(result.settings.recovery.error_duration_seconds)); setRevision(result.settings.revision); setDirty(false); } });
  };
  const candidates = snapshot ? latestObservations(snapshot).filter(observation => observation.model_request?.error) : [];
  const requestContinue = (episode: Episode) => { const input = { RequestContinuation: { generation: dataset.generation, episode } }; void mutation.run(input, () => continuation(input)); };
  return <section><h2>持续错误恢复</h2>{!snapshot ? <p className="native-muted">恢复策略未知</p> : <><div className="native-actions"><label><input type="checkbox" checked={enabled} disabled={!snapshot.settings.enabled} onChange={event => { setEnabled(event.target.checked); setDirty(true); }} /> 自动请求继续</label><label>未解除错误持续 <input aria-label="错误持续秒数" inputMode="numeric" value={seconds} onChange={event => { setSeconds(event.target.value); setDirty(true); }} style={{ width: 100 }} /> 秒</label><Button theme={theme} size="sm" label="保存策略" disabled={mutation.blocked || !dirty || !valid} onPress={save} /><Button theme={theme} size="sm" label="载入当前配置" disabled={mutation.busy} onPress={load} /></div>{!snapshot.settings.enabled && <p className="native-muted">先在「明细」启用采集。</p>}{dirty && revision !== snapshot.settings.revision && <p className="native-error" role="alert">配置已在别处改变，请载入当前配置后修改。</p>}{!valid && <p className="native-error" role="alert">请输入 1–604800 秒。</p>}<p className="native-muted">请求进入原生继续消息队列；模型是否执行、是否解除错误须看后续模型观测。</p></>}
    {mutation.error && <p className="native-error" role="alert">{mutation.error}</p>}{mutation.pending.length > 0 && <div className="native-actions"><span>原请求结果待核对</span><Button theme={theme} label="读取策略与尝试回执" size="sm" onPress={() => void refresh()} /></div>}
    <h3>未解除错误采样</h3>{!candidates.length && <p className="native-muted">当前保留样本没有错误证据；未观测的实例状态未知。</p>}{candidates.map(observation => {
      const state = observation.model_request!;
      const error = state.error!;
      const episode: Episode = { agent: observation.agent, session_id: state.session_id, source: error.source, failing_turn: error.turn_id, error_since: error.error_since };
      const attempted = snapshot?.attempts.some(attempt => referenceKey(attempt.episode.agent) === referenceKey(episode.agent) && attempt.episode.session_id === episode.session_id && attempt.episode.source === episode.source && attempt.episode.error_since === episode.error_since);
      const age = observation.at - error.error_since;
      const eligible = snapshot?.settings.enabled && snapshot.settings.recovery.enabled && observation.status === 'error' && state.phase === 'failed' && state.turn_id === error.turn_id && observation.session_id === state.session_id && observation.agent.incarnation !== null && observation.agent.stopped_event === null && age >= snapshot.settings.recovery.error_duration_seconds && !attempted;
      const label = attempted ? '本 episode 已请求' : '请求继续';
      return <details key={observation.id}><summary>{observation.agent.name} · {state.phase} · 错误起点 {at(error.error_since)}</summary><p>{error.reason}</p><dl className="native-facts"><dt>实例 / 会话</dt><dd>{referenceKey(episode.agent)} / {episode.session_id}</dd><dt>来源 / 失败轮次</dt><dd>{episode.source} / {episode.failing_turn}</dd><dt>采样时的未解除时长</dt><dd>{age >= 0 ? `${age} 秒` : '未知'}</dd><dt>采样时间</dt><dd>{at(observation.at)}</dd></dl><Button theme={theme} size="sm" label={label} disabled={mutation.blocked || !eligible} tooltip="daemon 会在动作时重新核对同实例错误 episode、阈值及一次性屏障" onPress={() => requestContinue(episode)} /></details>;
    })}
    <h3>继续请求审计</h3>{snapshot && !snapshot.attempts.length && <p className="native-muted">尚无保留尝试</p>}{snapshot && snapshot.dropped_attempts > 0 && <p className="native-muted">已超出保留范围的尝试 {snapshot.dropped_attempts}；原生一次性屏障仍保留。</p>}{snapshot?.attempts.map(attempt => <details key={attempt.id}><summary>{at(attempt.at)} · {attempt.episode.agent.name} · {attempt.status}</summary><p>{attempt.reason ?? '继续消息已接纳；执行结果未由此回执证明。'}</p><dl className="native-facts"><dt>尝试 ID</dt><dd>{attempt.id}</dd><dt>消息 ID</dt><dd>{attempt.message_id ?? '未知'}</dd><dt>实例 / episode</dt><dd>{referenceKey(attempt.episode.agent)} · {at(attempt.episode.error_since)}</dd></dl></details>)}
  </section>;
}
