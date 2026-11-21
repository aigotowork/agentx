import { parseJson, stringifyJson, parseManagementJson } from '@agentx/client';
/** Browser uncertainty is separate from durable native state; only reads reconcile a lost mutation reply. */
import { useEffect, useState } from 'react';
import type { PublicMutation, Snapshot } from '../../../host/live/native';
import type { Dataset } from '../../../host/live/model';
import { datasetKey } from '../../../host/live/model';
import { NativeError } from '../../../host/live/client';

interface PendingMutation { input: PublicMutation; at: number }
const key = (dataset: Dataset) => `agentx-inspector:pending:v1:${datasetKey(dataset)}`;
function restoreInput(input: PublicMutation): PublicMutation {
  if ('Configure' in input) return { Configure: parseManagementJson('ConfigureInput', stringifyJson(input.Configure)) };
  if ('Analyze' in input) return { Analyze: parseManagementJson('AnalyzeInput', stringifyJson(input.Analyze)) };
  if ('CancelAnalysis' in input) return { CancelAnalysis: parseManagementJson('CancelAnalysisInput', stringifyJson(input.CancelAnalysis)) };
  if ('RequestContinuation' in input) return { RequestContinuation: parseManagementJson('RequestContinuationInput', stringifyJson(input.RequestContinuation)) };
  throw new TypeError('Unknown retained inspection intent');
}
const load = (dataset: Dataset): PendingMutation[] => {
  try { const value: unknown = parseJson(localStorage.getItem(key(dataset)) ?? '[]'); return Array.isArray(value) ? value.map(entry => ({ ...entry, input: restoreInput(entry.input) })) : []; } catch { return []; }
};
function resolved(entry: PendingMutation, snapshot: Snapshot): boolean {
  const input = entry.input;
  if ('Configure' in input) return snapshot.settings.revision > input.Configure.expected_revision;
  if ('Analyze' in input) return snapshot.jobs.some(job => job.id === input.Analyze.request_id);
  if ('CancelAnalysis' in input) return snapshot.jobs.some(job => job.id === input.CancelAnalysis.id && job.status !== 'queued');
  if ('RequestContinuation' in input) {
    const expected = input.RequestContinuation.episode;
    return snapshot.attempts.some(attempt => JSON.stringify(attempt.episode.agent) === JSON.stringify(expected.agent) && attempt.episode.session_id === expected.session_id && attempt.episode.source === expected.source && attempt.episode.error_since === expected.error_since);
  }
  return false;
}
export function useInspectionMutations(dataset: Dataset, snapshot: Snapshot | undefined, refresh: () => Promise<unknown>) {
  const [pending, setPending] = useState(() => load(dataset));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = (value: PendingMutation[]) => { localStorage.setItem(key(dataset), stringifyJson(value)); setPending(value); };
  useEffect(() => {
    if (!snapshot) return;
    const current = load(dataset);
    const next = current.filter(entry => !resolved(entry, snapshot));
    if (next.length !== current.length) {
      try { save(next); } catch (failure) { setError(String(failure)); }
    }
  }, [snapshot]);
  const run = async <T>(input: PublicMutation, effect: () => Promise<T>): Promise<T | undefined> => {
    if (busy || load(dataset).length) { setError('尚有结果未知的请求，请先读取原状态对账。'); return; }
    const entry = { input, at: Date.now() };
    setError(null); setBusy(true);
    try { save([entry]); } catch (failure) { setError(`无法保存请求身份，未提交：${String(failure)}`); setBusy(false); return; }
    try {
      const value = await effect();
      save([]);
      void refresh().catch(failure => setError(`已收到请求回执；刷新状态失败：${String(failure)}`));
      return value;
    } catch (failure) {
      if (failure instanceof NativeError && failure.status >= 400 && failure.status < 500) {
        save([]); setError(`请求被拒绝：${failure.message}`); void refresh();
      } else setError(`结果未知，请读取原状态；不会自动重发。${String(failure)}`);
    } finally { setBusy(false); }
  };
  return { run, busy, pending, error, blocked: busy || pending.length > 0 };
}
