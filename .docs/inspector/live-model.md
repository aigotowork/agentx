---
schema: document
status: active
owner: live-native-sumo
last_updated: 2026-11-15
links: [../../.controller/live-ui-integration/tasks/native.md, ../../src/inspection/AGENTS.md, ./data-foundation.md]
---

# Native Inspector collection and governance

## Selected slice and source

Inspector observes exact management Agent references. The daemon owns generation-scoped collection settings, bounded retained observations, deterministic analysis jobs/reports and recovery audit. Originating hosts read Provider transcripts outside database transactions. The public native contract is [contract.rs](../../src/inspection/contract.rs); shared service/HTTP/generated binding/worker integration belongs to gabi. This model supersedes historical offline restrictions only for this delivery. Implementation and executed evidence remain separately tracked in the [Task](../../.controller/live-ui-integration/tasks/native.md).

Source evidence: [model request state](../../src/agents/model_request.rs), [transactional observation](../../src/agents/model_request_store.rs), [transcript counters](../../src/transcript/shared.rs), [management one-shot operations](../../src/management/store.rs) and [originating worker](../../src/management/worker.rs). `Started` retains an unresolved error; `Failed` can replace its failing turn while retaining error_since. Error duration measures unresolved episode age, not uninterrupted UI status. Current native support is Hook-only; absent observations remain unknown.

## Structure and integration signatures

| File | Responsibility and intended signatures |
| --- | --- |
| `contract.rs` | Native serializable settings, coverage, observation, usage, analysis/report, recovery policy/attempt and finite Request/Response |
| `collection.rs` | Host-side `usage_sample(agent, source, transcript)` projection, no managed storage; session cumulative counters with explicit parser-default caveat |
| `analysis.rs` | Pure `summarize(job, observations)` versioned deterministic evidence summary; no model calls or invented AI findings |
| `recovery.rs` | Exact `episode(row)` extraction and `eligible(row, policy, expected, now)` guard; continuation admission remains daemon transaction |
| `store.rs` | `handle(db, context, Request) -> ServiceResult<Response>`; generation-scoped KV, bounded collection and durable job/report transitions |
| `mod.rs` / `AGENTS.md` | Deliberate module boundary and integration instructions |

Integration calls `store::handle`; `Request::Collect` is internal host supply and must never be exposed as arbitrary browser mutation. The collector obtains exact native transcript source refs, reads them locally, then supplies optional parsed cumulative usage and discovery completeness. No Provider file reads under a SQLite write scope. Collection starts explicitly disabled; enabling records daemon time and an enable revision. Stale in-flight collection requests must carry this revision and are rejected after disable/re-enable. Polling collection is not scheduled tasks support.

## Collection, units and coverage

A retained observation is a current snapshot at collection time with exact AgentRef, session, native status and optional ModelRequestState. The first enable time is `collected_from`; disabling, collector outages and retention eviction are explicit gaps. Re-enabling creates a new admission revision. Prior Provider content may supply a present cumulative counter baseline but is never reconstructed as pre-enable history. Bounded sampled history cannot establish every transition, continuous busy durations, complete turn counts or success rates. Empty samples differ from complete empty discovery.

Usage identity is Provider tool + native source correlation (path/session internally, opaque source key publicly), independent of Agent names. Resumed/shared sources replace one source checkpoint; snapshots never sum across polls or across attached Agents. Input/cache-read/cache-write/output are disjoint; reasoning is a subset of output and must not be added again. Parser counters can default missing fields to zero: any known aggregate retains `parser_normalized` provenance, and no field-level completeness is asserted. No usage is `None`, not zero. Context/model remain optional. No money fields. Snapshot decreases/corrections invalidate delta inference; this slice exposes checkpoints and observed snapshots, not fictitious per-call IDs, exact interval spend or rates.

## Deterministic analysis

Submit pins exact retained observation IDs and an input revision in the same transaction as the durable queued job. Run consumes only that frozen input and records a report with algorithm `native-evidence-v1`, counts, native-error observations and source coverage caveats. Re-running the same job returns its retained report; crashes leave queued work safely restartable because computation has no external effects. Jobs/reports survive daemon restart. Caller request IDs bind their selected input; conflicting reuse is rejected. This is an evidence summary, never AI analysis, recommendation, proof of task completion or paid Provider execution.

## Causal recovery semantics

Selected action, agreed by integration in AgentX #24597, is native durable continuation mailbox acceptance. It is not PTY injection and does not claim that an errored Provider actually reads the message or recovers. Existing management Resume applies to stopped/inactive Agents and is not an errored-live-instance retry primitive. A different host action requires its own final guarded native primitive.

| Transition | Guard and durable effect |
| --- | --- |
| Configure | Scope-shared enabled flag and positive configurable threshold, default 600 seconds; independent of collection/UI preferences |
| Candidate | Same exact current incarnation/session; native model_request error source, failing turn and error_since; daemon time reaches threshold |
| Admit continuation | Re-read same instance and episode under daemon write transaction; effective native status is error, model request context, phase Failed, process not exited, no Started in-flight retry; one durable episode barrier |
| Accepted | Atomically persist episode barrier, native message acceptance and audit receipt; acceptance does not mean successful recovery |
| Refused | Changed episode/current instance/status/phase/policy means no message effect; exact reason returned |
| Unknown | If an external executor is selected, reservation must precede dispatch and survive restart; unknown blocks all repeats and cannot expire into retry |
| Observed success | Only matching native successful completion/episode clearance is evidence, linked to original instance/session; generic active/listening cannot prove success |

The barrier identity uses incarnation, native session, error source and original error_since. Failing turn is checked at execution but excluded from the barrier key, because repeated failures can update the turn inside one unresolved episode. This enforces once per episode even after failure detail changes. Second-resolution native error_since cannot distinguish clear/new-fail within one second with otherwise identical identity; conservatively suppressing a second attempt is safer than duplicate effects and is an explicit native source limit. Attempts are never trimmed while their once-only barrier is required. No fixed backoff, permission approval, scheduling, quota, price or backfill subsystem is introduced.

## Implemented module limits and handoff

The native module now implements the selected domain. All `at`, `from`, `through`, `collected_from`, `observed_at`, `retained_from`, `created_at` and native `error_since` fields are Unix seconds. Token quantities are exact decimal u64 strings; reasoning remains a subset of output, ratios remain optional floating values. A sample additionally carries native session identity; admission re-resolves the current native transcript source and matches source key/session/exact AgentRef, so a stale host read is dropped to unknown. Public Request and nested mutable policy/episode inputs reject unknown fields.

`Read` requires the current management generation. Configure uses settings revision CAS; after a lost response the caller reads settings and reconciles rather than assuming a repeat is safe. Observation revision changes on collection and analysis jobs pin that revision plus input coverage and exact observation IDs. Only collection and recovery settings are persisted; the consumer deliberately removed unsupported completion-read concepts, so no parallel read ledger exists.

Retained snapshots are bounded to 256 observations and 1.5 MiB of serialized observation facts within a 2 MiB ledger. Native discovery is at most 100 current local rows per collection; partial discovery is explicit, and stopped history is not backfilled. The read result includes the last 64 audit attempts and `dropped_attempts`; once-only barriers and full accepted receipts remain generation-scoped KV even after audit-window eviction. Up to 32 durable analysis jobs, each with 1..16 frozen inputs, are accepted; capacity exhaustion is an explicit conflict and never silently deletes a report or request dedup record. Gap detail is bounded to 64 intervals by conservatively merging older gaps. Collection silence beyond 120 seconds is explicitly unavailable coverage on read and a retained gap on the next collection. These are finite sampled observations, not a complete native event journal or complete Provider call history.

Recovery requires collection enabled. Both default off, and Configure rejects recovery enabled with collection disabled. Internal Collect evaluates automatic recovery against the fresh policy and current native state; it needs no browser timer. No host action is issued by this module. Reservation, native durable message acceptance and final queued audit commit together or roll back together; lost replies replay the retained episode receipt. A unique native message request ID per attempt allows one collection to handle multiple different eligible Agents. Later observed native Completed with no error on the same current incarnation/session marks `observed_error_cleared`, explicitly without asserting the continuation caused it. Existing retained unknown barriers remain non-replayable even though this atomic local queue action does not itself create external-execution unknown states.

The integration owner wires native service dispatch, finite HTTP routes, generated TypeScript and the originating collector. Browser mutation must never accept internal Collect or RunAnalysis. Production integration/HTTP/browser execution and executed native tests remain separately owned; module authorship and rustfmt parsing are not runtime acceptance. Nine finite tests in [store tests](../../src/inspection/store/tests.rs) cover collection fences/gaps, automatic multiple-Agent once-only admission/reopen, decisive recovery refusals, audit rollback, retained unknown barriers, frozen analysis/reopen/conflicts, full-width usage/unknowns/dedup identity, retention loss and stale-session/cancellation.
