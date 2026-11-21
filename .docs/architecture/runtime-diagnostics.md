---
scope: Native operational logs, management HTTP and execution-host diagnostics
status: verified-and-adopted-on-selected-linux-scope
owner: runtime-read-repair
related: ["../../.controller/runtime-read-repair/AGENTS.md", "daemon.md", "../observability/README.md"]
---

# Operational diagnostics

The existing scope-local JSONL log is `<scope>/.tmp/logs/agentx.log`. Operational diagnostics run independently of optional Observation collection. [Implementation and evidence](../../.controller/runtime-read-repair/implementation.task.md) own actual candidate/live support; source presence alone does not establish deployment.

| Concern | Record / correlation | Meaning |
| --- | --- | --- |
| HTTP failure | `management.http.failed`; `http_request_id`, normalized `route`, `method`, `status`, `error_code`, `duration_ms`, daemon instance | `x-request-id` in the actual response identifies this request. `http_admission` means the HTTP budget refused it; `native_admission` means a native busy response; `native_unavailable` and `outcome_unknown` remain distinct. |
| Slow HTTP response | `management.http.slow`, same request/route/time fields | Successful API responses taking at least500ms. This measures the HTTP route before diagnostic log persistence, not model execution. |
| Native failure/slow dispatch | `daemon.dispatch.failed` / `daemon.dispatch.slow`; native request/attempt/trace IDs when available, static operation label, error class, generation, application duration and queue wait | The slow threshold covers the sum of queue wait and application duration. `duration_ms` remains application time; `queue_wait_ms` is separate. One worker appends these diagnostics from a 128-record best-effort queue; a full queue, contended log lock, or shutdown can omit records. Application replies do not wait for log filesystem calls. |
| Native Busy refusal | Subsystem `daemon`, event `admission.busy`, daemon instance, `stage` and, for queued calls, `dispatch_lane` and `operation` | `connection` means the daemon refused a connection before decoding a call; `queue` means the selected read or write channel was full. These refusals use the same best-effort logger. Native counters remain the aggregate evidence. |
| Hook trace | `hooks.hook.trace`; safe phase durations and attempt outcomes; list with `agentx observe trace` | [Sampling and coverage](../observability/telemetry-coverage.md). No observation RPC is sent at Hook exit. |
| Listener/bootstrap | `management.runtime.failed` / `.listener.failed`; daemon instance and explicit scope | Records failures that previously existed only on daemon stderr. Repeated identical listener failures are deduplicated by the existing loop. |
| Execution host | `management.worker.started`, `.failed`, `.stderr`, `.abandon_failed`, `.image_retained`; operation ID, generation and worker PID where available | Bootstrap/spawn/wait/input errors retain their stage. stderr is a private bounded tail; the public operation receipt stays authoritative for domain outcome. Failure to record Abandon is itself logged; do not assume an operation was never executed. |
| Domain operation | `management.operation.finished` / `.failed`; original operation ID and generation | Host execution completion/failure. A successful worker process alone is not a successful domain operation; read the original receipt for its exact result. |

All new scoped events also include process PID and loaded native build. Scope and database generation are different: scope selects files, generation fences retained data. HTTP request ID, native request ID and durable operation ID are separate identities; do not equate them or infer that an HTTP request completed merely from acceptance of a durable operation.

New HTTP/native diagnostic events record static routes/classes, not request bodies, query values, prompts, environment maps or worker tickets. Worker stderr can contain subprocess diagnostics and stays in the private rotated log; known sensitive environment values are redacted, but arbitrary subprocess output is not guaranteed secret-free. Share only the relevant sanitized entries. Public errors contain supervision stage and executable/cwd context; raw stderr is not returned to browser clients.

The native writer has 16 queued slots; selected Management views, file bindings, and PTY delivery snapshots share four separate queued read slots and one read worker. A full read lane returns Busy independently of the writer. Runtime counters aggregate both lanes; the activity field shows one active request, and per-request observations identify each executing call. See [daemon I/O isolation](../../.controller/daemon-io-isolation.task.md) for the hot paths and [residual Busy](../../.controller/daemon-performance-audit/residual-busy.task.md) for the refusal diagnostic's verification and rollout status.

For design-level workload triage across browser polling, native listing/routing, and SQLite access, see [performance audit](../../.controller/daemon-performance-audit/audit.md). Its executable SQL probe uses in-memory fixtures and records query-plan and work evidence separately from live latency.

For managed startup, distinguish `ResolveResume`, `ReserveResume` and `TerminalLaunched` before attributing a timeout to the Provider. Reservation can commit before the CLI gets a reply; it does not itself spawn a Provider. See the [dancingai incident](../../.controller/daemon-performance-audit/codex-resume.task.md).

The [foreground follow-up](../../.controller/daemon-performance-audit/foreground.task.md) shares bounded database projections across repeated roster, Agent detail, human mailbox, and office activity reads. The read connection checks SQLite `data_version` before reuse and still fences every native request by generation. `OperationReceipt` observes journal state on the read lane; submit, claim, finish, and abandonment own durable effects.

# Retention and reads

The current file rolls after approximately8MB. Default retention is three numbered backups; `AGENTX_LOG_BACKUPS` selects0..32, with0 discarding the rolled file. Cross-process writers coordinate rotate+append through `agentx.log.lock`; never delete a live lock file to recover a logger. Unix log files/lock are mode0600 and the log directory is0700. Windows uses existing filesystem ACL behavior and needs its own platform acceptance; Unix mode bits are not an ACL promise.

`log::get_recent_logs` uses the caller's selected scope, and `get_recent_logs_in_scope` supports an explicitly selected host scope. Both include retained rotations, filter time/levels and return newest-first up to the requested limit. Reads are best-effort observations rather than an atomic forensic snapshot. Existing log-summary counts describe the retained latest100 warning/error entries, not an all-time error total. Log persistence failures do not recursively log or change domain command outcomes.

The existing `agentx status --logs` command displays recent warnings/errors for its selected `AGENTX_DIR`; select the intended scope explicitly when investigating a different daemon. For the selected home service after this repair is adopted, use `AGENTX_DIR=/home/long/.agentx /home/long/works/longwork/agentx/.bin/agentx-home-runtime-read-r1 status --logs`. Direct JSONL inspection retains the structured correlation fields that a short status display may omit. Recent-log reads stream rotations and retain only the requested result count in memory.

Linux worker executable retention under `<scope>/.tmp/management-hosts/` is separate from log rotation. A running image whose installed pathname was replaced is copied atomically once per native build so later worker/PTY launch steps retain an executable path. Do not remove a retained host image while live processes or launch references may still use it. Normal selected deployment uses a persistent versioned artifact and does not need this fallback cache.
