// Derived by derive.py from native serde declarations. Do not edit.
// Bounded numeric fields use numbers; identities and cursors use native string fields.

// Source: src/management/contract.rs
export type AgentRef = { name: string; incarnation: (string | null); stopped_event: (string | null) };

// Source: src/inspection/contract.rs
export type AnalysisJob = { id: string; input_revision: number; input_coverage: Coverage; created_at: number; status: string; observation_ids: Array<string>; report: (AnalysisReport | null) };

// Source: src/inspection/contract.rs
export type AnalysisReport = { algorithm: string; observation_ids: Array<string>; observation_count: number; native_error_count: number; distinct_agents: number; usage_sources: number; unknown_usage_count: number; caveats: Array<string> };

// Source: src/inspection/contract.rs
export type Coverage = { observed_at: (number | null); retained_from: (number | null); discovery_complete: boolean; dropped_observations: number; gaps: Array<Gap>; basis: string };

// Source: src/inspection/contract.rs
export type Episode = { agent: AgentRef; session_id: string; source: string; failing_turn: string; error_since: number };

// Source: src/inspection/contract.rs
export type Gap = { from: number; through: (number | null); reason: string };

// Source: src/agents/model_request.rs
export type ModelRequestState = { session_id: string; turn_id: string; model: string; phase: RequestPhase; last_progress_at: (number | null); error: (RequestError | null); retired_turns: Array<string> };

// Source: src/inspection/contract.rs
export type Observation = { id: string; at: number; agent: AgentRef; session_id: (string | null); status: string; model_request: (ModelRequestState | null); sample: (UsageSample | null) };

// Source: src/inspection/contract.rs
export type RecoveryAttempt = { id: string; episode: Episode; at: number; status: string; message_id: (string | null); reason: (string | null) };

// Source: src/inspection/contract.rs
export type RecoveryPolicy = { enabled: boolean; error_duration_seconds: number };

// Source: src/inspection/contract.rs
export type Request = { Read: { generation: string } } | { Configure: { generation: string; expected_revision: number; enabled: boolean; recovery: RecoveryPolicy } } | { Collect: { generation: string; expected_revision: number; samples: Array<UsageSample>; discovery_complete: boolean } } | { Analyze: { generation: string; request_id: string; observation_ids: Array<string> } } | { RunAnalysis: { generation: string; id: string } } | { CancelAnalysis: { generation: string; id: string } } | { RequestContinuation: { generation: string; episode: Episode } };

// Source: src/agents/model_request.rs
export type RequestError = { turn_id: string; source: string; reason: string; error_since: number; last_error_at: number; consecutive_failures: number };

// Source: src/agents/model_request.rs
export type RequestPhase = "started" | "completed" | "failed" | "cancelled";

// Source: src/inspection/contract.rs
export type Response = { Snapshot: Snapshot } | { Job: AnalysisJob } | { Attempt: RecoveryAttempt };

// Source: src/inspection/contract.rs
export type Settings = { enabled: boolean; collected_from: (number | null); revision: number; recovery: RecoveryPolicy };

// Source: src/inspection/contract.rs
export type Snapshot = { generation: string; settings: Settings; coverage: Coverage; observations: Array<Observation>; jobs: Array<AnalysisJob>; attempts: Array<RecoveryAttempt>; dropped_attempts: number; observation_revision: number };

// Source: src/inspection/contract.rs
export type Usage = { input: string; output: string; cache_read: string; cache_write: string; reasoning: (string | null); context_tokens: (string | null); context_window: (string | null); context_usage_ratio: (number | null); model: (string | null); unit: string; basis: string };

// Source: src/inspection/contract.rs
export type UsageSample = { agent: AgentRef; source_key: string; session_id: (string | null); usage: (Usage | null); availability: string };
