---
schema: document
status: implemented_pending_verification
owner: agentx
last_updated: 2026-11-10
description: Scope ownership, local transport and lifecycle for the single-binary AgentX daemon.
links: [../proposals/domain-driven-agentx-daemon.md, ../../.controller/domain-daemon/tasks/daemon.md]
---

# Daemon lifecycle and local IPC

The [proposal](../proposals/domain-driven-agentx-daemon.md) owns the selected product direction. The [Task](../../.controller/domain-daemon/tasks/daemon.md) owns execution state and verification handoff. Native application declarations under `src/service` own operations; transport serializes those declarations and never dispatches SQL, command argv, arbitrary method names or untyped operation payloads.

## Responsibilities and mechanism choice

| Owner | Responsibility | Source |
| --- | --- | --- |
| Lifecycle | Canonical data scope, singleton ownership, endpoint discovery, startup/readiness, compatible process identity, shutdown | `src/daemon/` |
| IPC boundary | Bounded frames, absolute deadlines, authenticated OS peer, native operation encoding | `src/daemon/` |
| Client binding | Explicit invocation context and native application call over IPC; no persistence fallback | `src/service/client.rs` |
| Application composition | Construct domain services only after scope ownership; serialize SQLite-backed use cases | Native `src/service` composition |
| Product entry | Enter daemon mode before client environment preparation; preserve pure/local commands | `src/main.rs` and CLI router |

Existing [notification sockets](../../src/runtime/notify/mod.rs) carry connect-and-close wakeups and cannot become a service API without changing their purpose. Standard Unix-domain streams provide a maintained local byte transport and OS caller identity with the existing Rust/libc dependencies. Serde derives on the native service contract are the single encoding definition. Local IPC retains that transport; the [management HTTP adapter](../agent-management/design.md) separately serves the browser and maps bounded product operations to the same native services. WebSocket has no current bidirectional event-stream requirement. Only the narrow local length framing, lifecycle handshake and bounds are application-owned.

Source references: [UnixStream](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html), [File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock), [Linux socket credentials](https://man7.org/linux/man-pages/man7/unix.7.html), and the existing [absolute-deadline poll adapter](../../src/integrations/codex/service/transport.rs). Platform claims require integration evidence. Linux peer identity uses `SO_PEERCRED`; macOS uses local peer credentials/PID. Windows uses [native named pipes](../../src/daemon/windows.rs), a protected current-user DACL, network-client rejection and kernel peer PID plus token-user SID checks. Lifecycle directories receive explicit inheritable current-user ACLs, including redirected scopes; ACL failures stop startup. Lifetime/startup lock handles deny delete-sharing. Windows frame I/O uses overlapped operations with absolute deadlines; cancellation is reaped before buffers or OVERLAPPED storage are released. This uses the existing windows-sys dependency, not a second wire protocol. Linux/macOS/Windows runtime support must be established separately by integration; source implementation is not runtime evidence.

## Ownership and state

The canonical data directory is the singleton key. Clients resolve it from their own invocation environment through [path resolution](../../src/paths.rs); the daemon receives an explicit path. Relative paths become absolute before acquisition, and existing symlink aliases converge through filesystem canonicalization. A scope-owned `.daemon` directory is separate from resettable `.tmp` data. An OS lock is held for the complete database/application lifetime, acquired before database open or migration. Lock files are never unlinked for normal shutdown. Reset must retain this directory and coordinate its database-generation fence with the application owner.

```mermaid
stateDiagram-v2
    [*] --> Discover
    Discover --> Ready: authenticated compatible handshake
    Discover --> Starting: missing endpoint and startup permitted
    Starting --> Owner: OS singleton lock acquired
    Starting --> Discover: another daemon owns the lock
    Owner --> Initializing: construct application under lock
    Initializing --> Ready: bind and answer readiness handshake
    Ready --> Draining: explicit shutdown or termination signal
    Draining --> Stopped: finish admitted operation and close endpoints
    Stopped --> [*]
    Ready --> Discover: connection lost before new request
```

A spawned child is not ready. Readiness requires an authenticated compatible handshake after application construction succeeds. A stale filesystem socket is removed only by the process holding the scope lock. The endpoint is owner-private and may use a fixed short runtime directory keyed by a digest of the canonical scope, avoiding Unix socket pathname limits and caller-specific `TMPDIR` selection. The lock, not a PID file or endpoint existence, establishes ownership. Removing/replacing live ownership files outside the managed reset contract is unsupported; requests check the live ownership handle before dispatch.

## Wire and effects

Each connection performs compatibility/readiness before at most one native operation. Both ordinary and inspection IPC compare the explicit protocol epoch, not package version or build identity. [`wire::PROTOCOL_VERSION`](../../src/daemon/wire.rs) is the shared authority: current epochs establishes this baseline, and incompatible native request/response shapes or operation semantics require a new epoch. Compatible changes must preserve existing readers and callers. Package version and embedded build ID remain diagnostic fields; different builds at the same epoch reuse the running daemon without an automatic restart. Epoch 1 used exact-build matching and requires one explicit transition restart. No capability negotiation or strict-mode configuration is implemented in this slice. See the [compatibility Task](../../.controller/domain-daemon/protocol-compatibility.md) for evidence.

The handshake also carries canonical scope, daemon incarnation and database generation. Database schema remains owned and checked by application construction, independently of transport compatibility. The receiver validates scope and instance fences before application dispatch. Linux/macOS peer credentials establish the actual process; participant, session and provider environment are explicit native caller context and remain subject to domain validation. The daemon's inherited environment never supplies another client's identity. Build identity survives executable pathname replacement and has no runtime image I/O; it is not authorization or proof of protocol compatibility.

All frames have an 8 MiB size bound. A deadline covers an entire read/write, including partial fragments and interrupted syscalls. At most 32 connection workers, 16 queued writer requests, and 4 queued read-lane requests protect the process from idle or slow clients. I/O and operation waiting use one 30-second maximum deadline, capped by any earlier caller deadline. Compatibility adds no eager full-image work before acquisition or calls. Readiness probes use two seconds; startup lock contention plus readiness has a 15-second bound. Automatic start sets explicit canonical `AGENTX_DIR` and cwd on the same executable; no readiness follows from spawn success. Scope startup logs are private and no operation is replayed by acquisition. One application dispatcher owns SQLite writes and generation changes. A separate daemon worker uses a read-only WAL connection for selected Management and file-binding reads. Each result is fenced against the retained generation; slow reads and file enrichment do not occupy the writer queue. An unavailable or full read lane does not fall back to the writer. A scoped admission thread constructs/owns its platform Listener and bounded connection workers; Unix listener poll or the Windows pending-connect event wakes it immediately without waiting for an application Job. Both loops retain the five-millisecond control/stop/generation observation interval. No wire worker or admission thread opens the database or borrows application state. An expired queued request is rejected before dispatch. Once admitted, a timeout or dropped reply does not cancel a committed mutation or prove that its effect did not occur. The client reports an unknown outcome and never silently replays it; idempotency/reconciliation belongs to the operation contract.

Domain responses can carry explicit work for the originating host without extending the IPC transport. Relay's native [`ReceiveResult`](../../src/relay/control_service.rs) carries admitted control effects and frozen wake ports; [runtime notification](../../src/runtime/notify/wake.rs) owns allowed-port snapshots and connect-and-close emission, while the [relay client](../../src/relay/client.rs) emits hints through its bounded notifier after dispatch. The [relay model](relay.md) owns wake filtering, dropped-hint recovery and control-effect grant/completion semantics. A local wake hint grants no process ownership; mutating host effects still require a stable observed `CallerProcess.incarnation`, and two missing incarnations never establish ownership. This response path needs no new IPC channel and does not change other domains' notification policy.

Shutdown stops new operation admission, finishes the current application call, rejects remaining queued work, closes the listener and releases database resources before the singleton guard. Listener construction/readiness/runtime failures signal stop and return to the dispatcher; an admission panic is converted to an unavailable failure after cleanup. Dispatcher exit or unwinding likewise signals admission to stop. Admission's drop guard closes the Listener and joins every tracked connection worker, including error/panic paths, before the scoped thread joins and application/lease can drop. A second queue drain handles workers racing with stop; dispatch itself refuses a stopped request. Connection workers finish within their fixed I/O deadline, so an idle pre-handshake client may delay shutdown by up to 30 seconds. Windows pending accepts cancel and reap their OVERLAPPED state before storage release. Unix endpoint cleanup checks the inode and never removes a replacement endpoint. Process restart changes the daemon incarnation. SQLite `service:database_generation` is the sole durable generation. The transport mirror is published from committed application state before readiness and after replacement; invalid publication clears it, and an invalidated/poisoned mirror stops admission. Expected generation is rechecked on the application thread after queueing. Database generation changes on managed replacement/reset according to the maintenance contract; ordinary reconnect must not silently attach a stale caller to a replaced database. Compatibility failure is explicit and does not kill an existing process or fall back to SQLite.

## Operator status and recovery

普通命令会自动获取缺失的守护进程。查看状态用 `agentx daemon status`；协议不兼容时，用 `agentx daemon restart` 显式恢复。两者均在参与者身份、项目环境和 dev-root 设置之前执行，默认遵循 `AGENTX_DIR`，未设置时使用正常的 home scope；`--scope PATH` 可覆盖环境，相对路径以调用目录为基准。不读取配置文件选择另一个 scope，不要求 `--iam`。

| 场景 | 调用形式 | 成功的含义 |
| --- | --- | --- |
| 查看当前环境选择的 scope | `agentx daemon status` | 输出 scope、状态、当前客户端版本/cohort，以及可取得的内核 peer 与 Ready 信息；不创建目录、修复权限、启动进程或打开 SQLite。 |
| 单独诊断另一 scope | `agentx daemon status --scope PATH` | 显式路径优先于环境；缺失 scope/无持有者的缺失端点显示 stopped。仍持锁但未提供端点时显示 unavailable。 |
| 协议升级后恢复选定 scope | `agentx daemon restart --scope PATH` | 已核实的旧 owner 完成优雅停止并释放 singleton，当前可执行文件提供兼容 Ready；保留数据及 owner.lock inode。 |

`status` 对 stopped、running 和 running-incompatible 返回 0，表示诊断成功；unavailable 或读取失败返回 1。CLI 用法错误返回 2，帮助返回 0。`restart` 只有获取到当前兼容 Ready 才返回 0，失败返回 1；没有交互提示。状态和成功结果写 stdout，错误写 stderr。兼容错误包含选定 scope、当前客户端 protocol/version/cohort、内核认证 PID 和可复制的绝对路径重启命令。旧版本在 Hello 阶段只返回笼统拒绝时，其加载版本/cohort 明确不可取得；当前服务端会在拒绝中提供自己的加载信息。磁盘上已被替换的可执行文件不代表旧进程加载的 cohort。

源码契约由 [`lifecycle.rs`](../../src/daemon/lifecycle.rs) 和 [`transport.rs`](../../src/daemon/transport.rs) 所有。重启先取得与冷启动共用的 startup lock，兼容 peer 使用同一认证连接和精确 daemon instance 的原生 Shutdown；Linux 上不兼容的旧 peer 只有在核对 PID/incarnation、精确 `daemon serve --scope` argv、加载映像、`/proc/locks` 中由该 PID 持有的 scope owner.lock device/inode，并用 pidfd 固定进程后，才收到一次 SIGTERM。当前调用完成后释放 Application/SQLite，再释放 singleton。没有数据删除、锁删除、PID-only kill 或强制终止。

startup-lock 等待上限为 15 秒，停止及持锁释放观察上限为 35 秒，随后获取当前 Ready 另有 15 秒上限。长时间应用调用可能继续运行；停止失败、回复不确定、证据不足、超时或端点异常都会拒绝替换，不启动新进程。可能已应用的业务调用不会重试。受阻后先查看状态与 scope 的 `.daemon/stderr.log`，排除仍在初始化/排空的进程，再显式重试。其他平台保留兼容 peer 的原生 Shutdown；没有 Linux pidfd/锁所有权证据时，不兼容恢复明确返回 unsupported，可用其原始兼容程序执行停止。平台源码分支不等于实机支持。

这次修订的[作者用例与契约](../../.controller/domain-daemon/tasks/startup-native.md)和[独立启动证据](../../.controller/domain-daemon/startup-evidence.md)分别记录源码交付与实际执行结果；历史 daemon 验收不自动覆盖新命令。

## Cases for independent integration

| Case | Required observation |
| --- | --- |
| Two clients start simultaneously, including symlink scope aliases | Only one application constructor opens the scope; both clients observe the same ready daemon |
| Child starts but application initialization fails | Acquisition reports unavailable; no readiness claimed and no DB fallback |
| Stale socket after crash | New lock owner can replace it and answer a fresh handshake |
| Live incompatible daemon | Client receives incompatible; no replacement, kill or database access |
| Changed native source/assets/build inputs | Build ID can change independently of compatibility; equal protocol epochs reuse the live daemon |
| Same-copy identity / executable path replacement | Running process keeps loaded diagnostic identity; different builds at the same epoch connect; incompatible epochs reject before dispatch |
| Deadline during partial frame / slow stream | Absolute deadline expires without unbounded allocation or extension per fragment |
| Frame exceeds bound / malformed native enum | Reject before dispatch, preserve server availability |
| Worker/queue bound exhausted | Explicit busy/unavailable within bounded time; no unbounded threads or allocation |
| Application held at a test barrier | Another Hello completes before releasing the application, proving independent listener/Job readiness |
| Stop with one in-flight and another queued call | In-flight call completes; queued call rejects without application dispatch; admission/workers join before lease reuse |
| Listener construction/readiness failure | Dispatcher observes stop/error, no application call executes, and scoped cleanup permits later scope acquisition |
| Client sends forged PID/identity | Application receives OS-authenticated process evidence or a rejection |
| Reply lost after mutation | Caller gets unknown outcome; transport performs no automatic retry |
| Graceful shutdown then reconnect | New admission stops, existing application call drains, ownership is released only after application drop |
| Managed reset / generation replacement | Old generation is rejected before a new mutation |
| Long canonical scope and differing `TMPDIR` | Clients discover the same short endpoint and the same scope owner |

All cases use disposable directories. Cargo execution, executable build and fixture ownership belong to integration; Linux process evidence does not establish macOS/Windows runtime support.

## Implementation and receiving verification

[Scope](../../src/daemon/scope.rs), [frame I/O](../../src/daemon/frame.rs), [platform peers/listener](../../src/daemon/platform.rs), [Windows binding](../../src/daemon/windows.rs), [transport](../../src/daemon/transport.rs), [server](../../src/daemon/server.rs), [wire](../../src/daemon/wire.rs) and [typed client](../../src/service/client.rs) implement the selected lifecycle contract. [Module guide](../../src/daemon/AGENTS.md) routes source ownership. The implementation owner authored frame size/fragment deadline, scope singleton/replacement, invalid generation publication, queued deadline/reset and Windows pipe peer/frame cases. [Integration](../../.controller/domain-daemon/tasks/integration.md) owns the historical execution and independent real-process fixtures, including epoch-1 same-version different-cohort rejection. The [protocol compatibility Task](../../.controller/domain-daemon/protocol-compatibility.md) supersedes that rejection policy for current epochs. Integration s1 compiled a fixed test harness and passed all six authored Unix/source boundary cases; see the [Task receipt](../../.controller/domain-daemon/tasks/daemon.md#independent-verification-receipt-s1). The later [s2 receipt](../../.controller/domain-daemon/tasks/daemon.md#independent-verification-receipt-s2) compiled and passed the formerly failing Claude real IPC fixture after hash-readiness/cancellation fixes. Subsequent full-digest product/process evidence remains in integration's Task. The new [embedded cohort source handoff](../../.controller/domain-daemon/tasks/daemon.md#selected-embedded-native-build-cohort) requires its own real Cargo invalidation, compatibility and DD12 evidence; prior fixed artifacts do not verify it. Native Windows/macOS execution and complete Delivery acceptance remain separate unverified outcomes.


### Historical full-digest startup evidence

The earlier full-digest implementation produced a 217 MB debug executable; eight cold clients consumed their fixture's 12-second observation at 100% CPU and explicit daemon readiness exceeded its bound. The [s5 receipt](../../.controller/domain-daemon/tasks/daemon.md#independent-verification-receipt-s5) preserves that observation. Matched release DD12 then retained a material regression even after AWS-LC backend correction. The [corrected receipt and fixed-release diagnostic](../../.controller/domain-daemon/tasks/daemon.md#corrected-digest-evidence-and-residual-dd12-disposition) distinguish exact artifact audit from product latency and trace overhead. Architecture consequently selected the embedded native cohort contract above. Its real-Cargo invalidation and native compatibility/copy/path-replacement cases subsequently passed within integration's documented single-toolchain/Linux scope. Full-file artifact audit remains independent.

Integration's [cohort receipt](../../.controller/domain-daemon/tasks/daemon.md#cohort-result-and-independent-readiness-correction) measured zero errors and substantially reduced full-image cost, but serial/concurrent list and start still regressed against the matched baseline. Source and traced Hello/accept waits identified independent listener and Job readiness coupled to one timed loop. The scoped admission correction above removes that coupling while preserving the existing control tick and native call counts. Its eight affected server/scope cases passed, including independence, stop/queued rejection and panic/listener-failure cleanup. [Actual admission DD12 evidence](../../.controller/domain-daemon/tasks/daemon.md#verified-admission-result-and-remaining-performance-disposition) now places warm sequential/concurrent list and events at or below the retained matched baseline's p50/p95 with zero candidate errors. Cold acquisition and manual start retain measurable overhead; controller/integration own final performance disposition. This is one local Linux run with no Provider/network workload, not universal parity or final Delivery/platform acceptance. No further daemon tuning is selected from these results.

The corrected release also passed three affected actual process cases: concurrent canonical/alias/independent-scope acquisition with readable descriptor ownership, compatibility-family rejection, and restart/reset/follower-no-fallback behavior. The [controller-accepted bounded Task](../../.controller/domain-daemon/tasks/daemon.md#completed-scope-and-receiving-boundary) releases source/resources to final integration. Cold/start absolute overhead remains disclosed: cold includes another process and initialized/published generation before ready; start keeps durable creation, snapshot rendering and confirmation after actual output flush. Current readiness-only acquisition plus a repeated first-call Hello is an explicit possible future composition improvement, with no measured attribution or implementation selected. Final combined-source performance/cutover/package and native Windows/macOS execution remain Delivery/integration outcomes.


### Peer identity and compatibility boundaries

`CallerProcess.incarnation` is an observed value, not a caller-supplied claim. Linux/Android derive it from boot ID plus `/proc` start ticks; macOS/iOS from `proc_pidinfo` start time; Windows from process creation FILETIME. Observation failure is `None`. Linux/macOS peer authentication can therefore establish same-user PID/UID while leaving incarnation unknown; Windows named-pipe validation rejects an unknown incarnation. Any domain that grants a durable host or external process effect must require a nonempty incarnation and must not treat two missing values as equal. Positive `has_exited` evidence may retire an owner; failed observation or `is_alive` uncertainty does not authorize takeover.

The build cohort is a nonsecret diagnostic value, not process authorization or byte attestation. [Generator](../../build.rs) recursively watches current source/embedded assets plus manifests, lockfile, generator, existing toolchain/configuration and relevant user build environment; Cargo owns target/host/profile/features/compiler unit separation. New inputs/local dependencies outside that closure require extending it. Clean reruns can conservatively mint new cohorts, while a successful unchanged Cargo cache/relink can reuse one. Arbitrary final-crate compiler overrides and post-link native edits are not certified by the stamp; see the [model](../model/state-ownership.md#native-build-compatibility) and [Task boundary](../../.controller/domain-daemon/tasks/daemon.md#selected-embedded-native-build-cohort). Full-digest helpers are retained under cfg(test) for artifact audit only; production uses neither a runtime image cache nor pathname/mtime identity substitution.
