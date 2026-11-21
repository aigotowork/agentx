# Source ownership

`main.rs` composes the executable. `config.rs`, `paths.rs` and `log.rs` provide application-wide configuration, path resolution and logging.

For the accepted client/daemon direction, read the [domain model](../.docs/model/README.md), [architecture](../.docs/architecture/README.md), [native contract release](../.controller/domain-daemon/contracts-handoff.md) and [current Delivery](../.controller/domain-daemon/delivery.md). The [controller](../.controller/domain-daemon/controller.md) owns current Paseo worker bindings. Earlier Codex assignment/installation receipts below remain historical evidence rather than current write authority or daemon acceptance.

| Home | Responsibility | Entry or local detail |
| --- | --- | --- |
| `service/` | Native operation aggregate, caller context, injected application and typed local client; product read-model and maintenance boundaries | [Owning context](service/AGENTS.md); domain invariants stay in their capability modules; only daemon composition constructs managed storage |
| `daemon/` | Canonical data scope, singleton/readiness, authenticated local IPC and bounded request dispatch | [Lifecycle contracts](daemon/mod.rs), [design](../.docs/architecture/daemon.md); serialize native operations, never SQL/argv |
| `cli/` | CLI routing, command handlers and command feedback | `router.rs`, `context.rs`, `commands/`; reset details stay in `commands/reset/` |
| `tui/` | Interactive terminal interface | `mod.rs`; `render/`, `input/` and `rpc/` own their local complexity |
| `management/` | Daemon-managed HTTP API and embedded Agent management console | [Owning context](management/AGENTS.md); exact live/stopped references, operation receipts, isolated host execution and HTTP projection |
| `collaboration/` | Bounded work brief/assignment/question/result/check/acceptance and explicit project workbench | [Owning context](collaboration/AGENTS.md); native declarations, pure aggregate, daemon transaction composition and closed browser adapter |
| `browser/` | Shared finite asset/precision/numeric-origin mechanics | [Owning context](browser/AGENTS.md); diagnostics, collaboration and management retain their own routes, policy and embedding selection |
| `agents/` | Participant records, identity, binding, lifecycle and names | [Owning context](agents/AGENTS.md); `attachments.rs`, `records.rs`, `identity.rs`, `binding.rs`, `lifecycle.rs`, `names.rs` |
| `messaging/` | Message routing, validation, bundles and delivery | [Owning context](messaging/AGENTS.md); `inbox.rs`, `activation.rs`, `routing.rs`, `validation.rs`, `bundles.rs`, `delivery/` |
| `events/` | Query and subscription filtering | `filters.rs`; includes current SQL generation |
| `integrations/` | External AI tool identities, metadata and adapters | `tool.rs`, `registry.rs`, `detection.rs`, `nested_env.rs` |
| `integrations/codex/` | Shared native service, qualified root attachments, protocol routing and actual-mail input | [Owning context](integrations/codex/AGENTS.md); `service.rs`, `roots.rs`, `protocol.rs`, `configuration.rs`, `bridge.rs` |
| `integrations/hooks/` | Tool hook handling and integration installation | Shared hook plumbing stays here; multi-file tool details stay under that tool |
| `integrations/launch_args/` | Upstream CLI argument adaptation | `shell.rs`, `overrides.rs`, `validation.rs`, and tool-named preprocessors |
| `integrations/plugins/` | Embedded source loaded by upstream tool hosts | Tool-named directories; preserve host-required filenames and manifests |
| `runtime/` | Launch, bootstrap, process environment and execution mechanisms | `launcher.rs`, `launch_status.rs`, `context.rs`, `process_tracking.rs` |
| `runtime/terminal/` | Terminal launch and terminal presets | `mod.rs`, `presets.rs` |
| `runtime/pty/` | Pseudoterminal forwarding, screen state and injection | `mod.rs`; OS backends remain private implementation details |
| `runtime/notify/` | Local process wake notifications | `mod.rs`, `server.rs`, `wake.rs` |
| `transcript/` | Unified transcript reading, per-format parsers and detail levels | `mod.rs`, `detail_levels.rs`, tool-named parser files |
| `relay/` | Cross-device transport, replication and remote control | `mod.rs` |
| `db/` | SQLite storage and stored entities | [Owning context](db/AGENTS.md); `mod.rs`, storage-specific implementations stay here |
| `workflows/` | Bundled product workflows | `mod.rs`, `bundled/`; repository development scripts live in root `scripts/` |
| `sys/` | OS process, filesystem, I/O, networking and signals | `mod.rs` |
| `shared/` | Existing cross-capability constants, errors, value types, text and time helpers | `mod.rs`; do not put a capability-owned implementation here just because it has multiple callers |

Use the nearest capability as the maintenance owner. Directory nesting expresses responsibility and limits the detail a reader must load; it does not claim that existing calls already form a strict one-way layer graph. Rust declarations and visibility define the actual interface. This binary has no newly introduced library or crate boundary.

Keep coherent single-file modules shallow. Add a subdirectory when a capability has meaningful cooperating parts; prefer `mod.rs` for that entry. Remove redundant subject prefixes once the parent supplies the context. Preserve external tool identifiers, config keys, CLI spellings, runtime installation paths and host-required plugin filenames.

The [migration design and complete file map](../.docs/source-layout/README.md) records the old-to-new layout, execution order and verification evidence.

## Originating environment

[`runtime/environment.rs`](runtime/environment.rs) owns frozen CLI/ENV/control-scope source selection, value-free receipts and local dotenv composition. [`config::EnvironmentSettings`](config.rs) owns strict `[environment]` parsing and [`config/file.rs`](config/file.rs) owns locked JSON-array/boolean CLI edits. [`InvocationContext`](service/context.rs) and [`AgentxContext`](runtime/context.rs) carry the effective snapshot plus selected variable names; launcher restores those names after clean-shell reconstruction. [`environment/tests.rs`](runtime/environment/tests.rs) owns authored finite cases. Follow the [detailed usage/order contract](../.docs/architecture/environment.md) and [original Task](../.controller/environment-loading/tasks/native.md); implementation release is distinct from executable support.

Independent execution passed 52 original scoped cases plus two causal routing regressions, the corrected package and Linux local finite CLI/child/isolation/direnv outcomes. Workbench evidence covers environment-selected scope/project startup and HTTP availability only. The [support boundary](../.docs/architecture/environment.md#independently-exercised-support) links canonical evidence and retains Provider/OS/browser limits; [Delivery](../.controller/environment-loading/delivery.md) owns artifact promotion/live adoption.

## Native Codex stdio

### Project environment boundary

Before configuration or identity reads, `main.rs` passes argv through `runtime/codex_project_env.rs::prepare`, then the [originating dotenv pipeline](../.docs/architecture/environment.md). The CLI's existing parser supplies the native stdio request and inserts a canonical agentx `--dir` before the opaque native separator. Project selection is explicit `--dir` > nonempty `PASEO_AGENT_CWD` > process cwd. Native version probes without either explicit project input bypass direnv, even when the process cwd has an environment file. Current adaptation ownership follows the domain-daemon handoff above; the original project-environment follow-up receipt records its earlier acceptance.

For a project or ancestor `.envrc`, preparation replaces the process with `direnv exec <project> <absolute-current-agentx> ...` from that project, preserving direnv's authorization, deletions and stderr diagnostics. An internal argv marker survives arbitrary environment deletions and is stripped before dispatch; the second entry validates the marker against the selected directory, cwd and direnv's loaded file. No automatic authorization or fallback follows failure. Projects without `.envrc` need no direnv executable. After dotenv prepares the complete effective environment, main pins the resolved native Codex command, rejecting the agentx executable itself; hook version/preflight and the final child share that command. Ordinary launches bypass direnv preparation and consume the shared explicit dotenv pipeline.

The existing effective-environment home resolver owns absolute `CODEX_HOME`, including explicit relative homes resolved against the canonical project; repeated launches with unchanged environment select the same home. Agentx configuration overrides retain their existing precedence after the imported project environment. Native thread history belongs to that home: changing it later does not migrate or expose threads from a different home.

Follow-up cases extend `tests/codex_stdio.rs`: project selection and explicit-relative priority, environment/defaults/identity before configuration, deletions and PATH, exact bytes after re-exec, blocked/failing environments before state creation, no-env fallback, stable relative home on reopen, guard validation/non-leakage and executable self-recursion, daemon/project version behavior, and termination after re-exec. Real direnv's blocked/failing/stdout contracts were checked in disposable isolated authorized fixtures; parent-owned probes cover real registered projects and native reopen. The pre-follow-up source snapshot is `/tmp/agentx-project-env-nano-before`. Follow-up implementation, focused checks and release packaging completed on 2026-11-06; the [handoff receipt](/tmp/agentx-project-env-result.md) records exact files, verification, artifact hash and known limits. Parent verification and installation remain independent.

The bounded [Paseo assignment](/home/long/works/paseo/deploy/tasks/agentx-stdio.md) originally established the direct stdio host. Ordinary Codex launches retain their PTY backend. `agentx launch --tool codex --stdio --go -- <codex arguments>` selects one local foreground connection. Supported `app-server` calls select the shared native integration described below; other native commands retain inherited streams. Agentx writes no launch preview, summary, title or banner to protocol stdout; errors use stderr. `--headless`, `--json`, remote launch, terminal selection, multiple instances and `--no-run-here` are incompatible. Resume/fork through agentx does not accept this launch-only flag; native root resume/fork remains available through app-server protocol requests.

The CLI owns flag validation, the preview gate and returning the native status. `runtime/launcher.rs` owns effective environment and backend selection. Its non-app-server stdio branch calls `runtime/codex_stdio.rs`, which owns direct process execution, signal forwarding and exit status conversion without protocol parsing. These direct-child exits retain existing session cleanup; Unix signals reach that child process group and Linux also terminates the direct child if its supervisor dies. The shared app-server branch calls `integrations/codex/bridge.rs::run` and has connection-only cleanup. An isolated `--version` or `-V` probe uses the effective environment but skips identity, hooks, bootstrap, service acquisition and conversational defaults, keeping native version output parseable.

The direct child retains caller arguments in order, with the existing Codex hook, trust, writable-root and developer-instruction overrides. Effective environment follows current-terminal launch rules; parent tool/session identity, including the shared-service marker, is removed for fresh children. Direct non-app-server stdio has no PTY idle wake, terminal screen or injection endpoint. The shared path uses native input and protocol approval state rather than terminal injection. Other providers retain their registry-selected launch and delivery paths.

| Focused case | Expected guarantee |
| --- | --- |
| Fake Codex receives binary input larger than a PTY line limit | Exact stdin/stdout bytes; stderr separate; no terminal or runner artifacts |
| Caller argv, cwd and effective environment | Literal argument boundaries, preserved ambient values and configured overrides, fresh identity, hooks and bootstrap |
| Child success, nonzero exit, spawn failure and Unix termination | Status propagated, child stopped, identity/process binding cleaned |
| Version through the same command prefix | Only native version output; no conversation defaults or session registration |
| Invalid stdio combinations and ordinary launch | Stderr-only rejection; existing interactive/default PTY selection retained |

The original stdio follow-up used `/tmp/agentx-stdio-nano-before` for its bounded comparison; the source-layout record above documents its unrelated baseline test failures. That receipt does not grant current installation or live-service authority. Current integration, builds and product-support claims follow the domain-daemon Delivery and independent acceptance.

### Shared app-server composition

The [shared runtime Delivery](../.controller/codex-shared-runtime/delivery.md) implements the selected app-server path through [production contracts](../.controller/codex-shared-runtime/contracts-handoff.md). [`configuration.rs`](integrations/codex/configuration.rs) selects only the app-server command position with stdio transport; version/help, daemon/proxy/schema subcommands, other listeners and ordinary commands retain direct handling. Unknown options in the selected shared path fail explicitly. Native `-c` and feature flags become thread configuration because official daemon start ignores them; they are never silently treated as daemon settings.

[`bridge.rs`](integrations/codex/bridge.rs) composes the injected native connection, attachment store and pending inbox. Its stdio host has bounded queues, preserves outer controls before internal offers, and closes only this connection on EOF/signal/output failure. The native service owns managed-daemon acquisition, compatibility and a stable observed backend generation; the bridge adds a fresh controller token. A proxy is never registered as a process-default participant. Successful correlated outer create/resume/fork results attach the correct canonical home/thread and select NativeInput before the original response reaches the client. Native internal threads and read/list/history events do not create identities.

[`protocol.rs`](integrations/codex/protocol.rs) preserves opaque JSON fields using raw values, including large numbers, and separates both request directions. Every outer/internal downstream request uses one allocator; only outer responses restore the original string/int64 ID. Native server request IDs and embedded resolved IDs stay unchanged. `ConnectionHandshake` preserves clientInfo and all capabilities except the internal observation union for notification opt-outs; it still filters the original opt-outs on outer output. Internal input requires successful initialize followed by initialized.

Session configuration appends trusted hooks and reserved identity exports while retaining caller config. Shared hooks resolve their payload session through home/generation attachments and never consume native mail a second time. The first prompt hook supplies the root's full bootstrap and commits its announcement only after flush under the current attachment/row fence. Known shared command identity resolves the same attachment, including child-launch ancestry; fresh children scrub the parent's shared marker.

Service compatibility remains conservative about environment changes. Arbitrary newly added variables in a child tool environment can produce an explicit compatibility conflict; general child reuse under such additions is not established. Thread provider/MCP configuration that explicitly depends on normalized environment inputs is rejected before native submission when the backend value cannot be verified. The [service topology](../.docs/codex-activation/topology.md) owns the normalized native provenance and material dependency rules.

The generic [`ActivationPolicy`](messaging/activation.rs) owns frozen-mail preparation and exclusive claims. The native driver durably marks Issuing before a possible write, chooses start while idle or steer with the observed active turn ID, and defers approval/input/unknown/mutating states. Only a completed native userMessage matching thread, batch clientId and exact frozen text confirms delivery. RPC success, assistant output and turn completion do not move the cursor. Lost outcomes stay uncertain; matching native history can recover positive delivery, while incomplete/empty history never authorizes replay. Blocked hooks retain their batch without an automatic retry loop. A predecessor claim continues to fence later bindings.

The implemented native profile is Linux Codex `0.167.0`. Unsupported service versions/transports fail acquisition explicitly; no private backend fallback is provided. After a compatible shared connection exists, usable native traffic may continue with autonomous input disabled when required observation capabilities are unavailable. Independent evidence includes real native parent-child autonomous round trips and the installed unmodified Paseo `0.15.3` Provider API, plus approval/cancel controls, root lifecycle, per-thread environment and connection-exit isolation. Exact execution, regression and final static/package status belong to [acceptance.md](../.docs/codex-activation/acceptance.md). Desktop/mobile rendering, paid-provider behavior, live user hosts and other platforms/native versions remain unverified. Read [protocol flows](../.docs/codex-activation/protocol.md) and [session environment](../.docs/codex-activation/session-environment.md) before changing effect or identity boundaries.

## File-backed Task/Delivery bindings

[`file_bindings/AGENTS.md`](file_bindings/AGENTS.md) owns source-file assignments and disposable observations, independent of `collaboration`/`work`. [`runtime/launcher.rs`](runtime/launcher.rs) binds the native reserved participant before spawning; [`service/application.rs`](service/application.rs) owns the memory index outside DB handlers. Use [`agentx bindings`](../.docs/file-task-bindings.md) for discovery, explicit refresh and exact-assignment queries.
