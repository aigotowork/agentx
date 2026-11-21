/** Live Inspector types derive from public management schemas; demo supply remains independent. */
import type { ManagementTypes as M } from '@agentx/client';
export type Snapshot = M["Snapshot"];
export type Observation = M["Observation"];
export type Episode = M["Episode"];
export type AnalysisJob = M["AnalysisJob"];
export type RecoveryAttempt = M["RecoveryAttempt"];
export type Usage = M["Usage"];
export type Settings = M["Settings"];
export type ConfigureInput = M["ConfigureInput"];
export type AnalyzeInput = M["AnalyzeInput"];
export type CancelAnalysisInput = M["CancelAnalysisInput"];
export type RequestContinuationInput = M["RequestContinuationInput"];
export type PublicMutation = { Configure: M['ConfigureInput'] } | { Analyze: M['AnalyzeInput'] } | { CancelAnalysis: M['CancelAnalysisInput'] } | { RequestContinuation: M['RequestContinuationInput'] };
