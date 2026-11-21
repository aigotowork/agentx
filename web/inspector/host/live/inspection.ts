/** Typed SDK inspection operations; internal Collect/RunAnalysis are never browser operations. */
import type { Snapshot, ConfigureInput, AnalyzeInput, CancelAnalysisInput, RequestContinuationInput } from './native';
import { management } from '../../../management/api/client';
export type { Snapshot, Observation, Episode, AnalysisJob, RecoveryAttempt, Usage, Settings } from './native';
export type ConfigureRequest = { Configure: ConfigureInput };
export type AnalyzeRequest = { Analyze: AnalyzeInput };
export type CancelRequest = { CancelAnalysis: CancelAnalysisInput };
export type ContinueRequest = { RequestContinuation: RequestContinuationInput };
export const inspection = async (generation: string, signal?: AbortSignal): Promise<Snapshot> => {
  const snapshot = await management.inspection.get(generation, { signal });
  if (snapshot.generation !== generation) throw new Error('采集数据代次已改变，请重新连接');
  return snapshot;
};
export const configure = (input: ConfigureRequest) => management.inspection.configure(input.Configure);
export const analyze = (input: AnalyzeRequest) => management.inspection.analyze(input.Analyze);
export const cancelAnalysis = (input: CancelRequest) => management.inspection.cancelAnalysis(input.CancelAnalysis);
export const continuation = (input: ContinueRequest) => management.inspection.requestContinuation(input.RequestContinuation);
export function at(seconds: bigint | null | undefined): string {
  if (seconds === null || seconds === undefined) return '未知';
  // Date conversion is presentation only; out-of-Date-range evidence stays exact.
  if (seconds > 8640000000000n || seconds < -8640000000000n) return `${seconds} Unix 秒`;
  return new Date(Number(seconds * 1000n)).toLocaleString();
}
/** Latest checkpoint by exact ref. Missing recent samples never inherit an older known counter. */
export function latestObservations(snapshot: Snapshot): Snapshot['observations'] {
  const byRef = new Map<string, Snapshot['observations'][number]>();
  for (const observation of snapshot.observations) {
    const key = JSON.stringify(observation.agent);
    const previous = byRef.get(key);
    if (!previous || observation.at >= previous.at) byRef.set(key, observation);
  }
  return [...byRef.values()];
}
