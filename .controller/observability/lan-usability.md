---
schema: task
status: completed
owner: controller
agent: nano
---

# Direct LAN console and standalone packaging

## Accepted running outcome

Direct URL: **http://192.168.2.178:18731/**. No launch token, bearer, login, required external asset directory or browser storage. The selected daemon, HTTP console and real managed Codex agent `lan-demo-zori` remain running at the user's request. The agent completed a real arithmetic/verification task and is listening. The isolated persistent root and operator instructions are [the demo README](/home/long/.local/state/agentx-observability-demo/README.md); exact live process/artifact identities are [runtime.json](/home/long/.local/state/agentx-observability-demo/runtime.json). No required implementation or validation work remains. Installed/PATH binaries and other data scopes were not replaced.

The user subsequently requested retirement of completed engineering subagents. All nineteen direct Paseo children from the daemon, observability and LAN assignments were archived, preserving their original Tasks, source and evidence. The post-operation list contains only controller `7c3af077-454b-4d39-9c3d-71de3fbc9598`; [exact retirement receipts](implementation-evidence/team-retirement.json) own the IDs and successful outcomes. This does not retire the user-requested AgentX demonstration participant or services. The exact demo HTTP/daemon PIDs were still running in their expected scope, and the LAN page returned HTTP200 after cleanup. [Roadmap candidates](../../.docs/roadmap.md) now distinguish finished scope, remaining release/support work and proposed new user outcomes; none creates a new implementation assignment.

Current executable `.bin/agentx` and the byte-identical persistent `bin/agentx` are the corrected standalone **development build**, SHA-256 `cb3d2737915204c0870edc665d8f2502a2d9965077846290d101e3e7d254c538`, 278628392 bytes. The first optimized `just package` result is preserved separately and exposed the real-use sizing defect; it is not relabeled corrected or used for the final running service. A second optimized release after the correction was not selected: the user's requested running local tool is fulfilled by the directly verified corrected image. Both build profiles embed freshly compiled real page resources; runtime needs no Node or asset directory. The obsolete byte-identical `.bin/observability-assets` output was removed only after its recovery copy was verified.

## User outcome and selected model

The user requests direct credential-free access, LAN reachability, a running daemon with real AgentX activity, and a standalone executable without a required asset directory. This explicit follow-up supersedes the old bootstrap/loopback/external-assets restrictions, preserving the accepted implementation evidence as history. Native finite read-only operations, safe disclosure and bounded transport remain unchanged.

Use `agentx observe-web --scope PATH --host 0.0.0.0 --port 18731`; localhost remains the default host. All browser visits and reloads work without credentials. Native inspection still selects one scope. Match browser Origin to the actual numeric interface Host and bound port; reject cross-origin requests and unexpected hosts. Ship the real compiled HTML/CSS/JS inside the executable; optional --assets is a development override. Cargo/package builds must regenerate resources from source rather than silently embed stale output.

## Ownership and next actions

- HTTP/client owner (new dedicated agent): owns src/observability/web except assets.rs; web/observability/api/client.ts and entry.ts; relevant tests and browser-api.md. Remove credential/session complexity, implement LAN bind and independent visits. Coordinate embedded() API with packaging owner. No Cargo/Node/browser execution; author targeted cases.
- Packaging owner (new dedicated agent): owns build.rs, assets.rs, Web build.mjs if necessary, justfile/package integration and packaging docs. Embed real freshly compiled resources at Cargo build time, with coherent source invalidation. No server/client edits, dependency upgrades or execution. Coordinate optional assets and embedded() return with HTTP owner.
- Controller: task/navigation state, independent focused checks/build/browser/startup; persistent isolated demo scope and real AgentX activity. Never reset existing work or alter installed programs. Preserve previous .bin artifact before replacement.

The split isolates HTTP authority from build/resource tooling and enables independent parallel implementation. No review committee or repeated prior acceptance campaign is selected.

## Acceptance

Direct HTTP and browser refresh via LAN numeric address, ordinary multi-client reads without bootstrap, cross-origin rejection, binary-only startup from another cwd without --assets, real daemon/configuration/request/event data, persistent process ownership and reachable URL. Run focused adapter/client/build checks and required package checks once; retain exact failures if any. Controller records startup command, scope, PID/log paths and evidence here. Stop only temporary validation services, leave the requested demonstration running.

## Friction

The former external-assets/bootstrap interface exposed development deployment choices in routine use; user explicitly selected a simpler direct-access product. Old task-specific exclusive executor and local-only restrictions belong to completed delivery and do not prohibit this authorized follow-up.

## Packaging source release

Packaging source authored: `build.rs` now runs the existing Node/TypeScript builder into a fresh Cargo-owned `OUT_DIR/observability-web`, generates a sorted finite `include_bytes!` table with per-file/count/total bounds, and fails rather than embedding previous output when compilation fails. Browser source/configuration/npm manifest changes invalidate the build; installed local compiler files and PATH are also watched. `assets::embedded() -> Assets` supplies the default bundle with the existing byte/content-type representation; `load(&Path)` remains unchanged for optional development overrides. No server/client files or dependency versions were changed. Existing `just package` already invokes Cargo and needs no extra build step.

The [owning browser guide](../../web/observability/AGENTS.md) links the [packaging prerequisites and source/archive contract](../../CONTRIBUTING.md#standalone-binary-packaging). Source archives build from included browser sources with Node plus an existing PATH `tsc` when `node_modules` is absent; builds perform no implicit dependency installation. This distinguishes build-time prerequisites from the standalone runtime artifact.

No Cargo, Node, formatter or tests were executed by this owner. Controller acceptance remains: compile and package once with current sibling sources; launch only the executable from a separate cwd without `--assets` and retrieve HTML/CSS/JavaScript; change a browser source and verify Cargo regenerates the embedded response; verify a compiler failure fails the build rather than selecting stale output. Inspect Cargo package contents/source-archive compiler availability if validating the archive distribution path. Source authoring alone establishes none of these runtime results.


## HTTP source release

HTTP/client ownership released on 2026-11-08. Current contract and model: [browser-api.md](../../.docs/observability/browser-api.md), [consumer boundary](../../.docs/observability/consumer-boundary.md), [Web design](../../web/observability/design.md). Sources: [HTTP adapter](../../src/observability/web/mod.rs), [finite mapping](../../src/observability/web/api.rs), [client](../../web/observability/api/client.ts), [entry](../../web/observability/entry.ts), plus state/app and authored state tests after the controller's bounded transfer. Removed session.rs and all server token/session machinery. CLI help now describes read-only diagnostics.

Connection contract: `POST /api/v1/scope` with `{}` returns the selected `ScopeRef` directly; `POST /api/v1/read` retains native ReadCall/ReadResult without credentials. `ObservationApi.connect()` initializes each independent page/reload. `--scope` remains required; `--host` accepts numeric IPv4/IPv6 and defaults to127.0.0.1, optional `--assets` overrides infallible `assets::embedded()`, and ignored `--name nano` is accepted without identity setup. A wildcard bind admits each actual local numeric interface. Host must equal the TCP connection's canonical local SocketAddr; Origin must equal `http://Host`. No DNS Host trust, cross-origin allowance or global event poll exclusivity exists. Eight native-read permits remain owned by actual work after HTTP cancellation; page-local event overlap prevention remains in the client. Close cancels only this page and retains historical evidence; stale401/session-expiry/restart instructions are removed.

Authored coverage: [adapter cases](../../src/observability/web/cases.md), [Rust tests](../../src/observability/web/tests.rs), [Web cases](../../web/observability/cases.md), and [state tests](../../web/observability/tests/state.test.mjs). HTTP author performed static source inspection only; no Cargo, Node, browser, formatter or installed/live operations. Controller owns independent execution and acceptance, including real numeric LAN reload/multiple pages, cross-origin rejection, concurrent native reads, packaged embedded resources and persistent demo. No owned processes or resources require cleanup.

Friction resolved: original bootstrap/loopback contract prevented ordinary refresh and independent clients; removed the owning authority state rather than adding token renewal. Remaining historical authenticated process tests describe prior delivery and must not be used as current acceptance without adaptation.

## Actual bindings and preservation

HTTP/client: `{serverId: srv_yDbm9zuqIMiM, agentId: 317b238a-5733-4519-b6a0-c1080df2e235}`. Packaging: `{serverId: srv_yDbm9zuqIMiM, agentId: 92425464-8395-4777-b812-0f9d5826dbad}`. Both released source after bounded authoring, with no executable jobs. Controller owns independent execution. Prior executable and external bundle preserved under `/tmp/agentx-lan-9jf25e31`; prior executable SHA-256 `7f5b79bf3bbc7d94663d2b8cfadff8f3d0ecd36cc7588ea76dc9d45753cf848b`. Actual LAN interface is `192.168.2.178`, selected port `18731` was available before startup. Focused adapter test/build is running against released source; no product service has been started yet.

## Independent checks before startup

Released source passed the targeted adapter filter: 11 tests, zero failures (`adapter-tests.log`). The Cargo-generated browser bundle passed all 9 state cases (`web-state.log`), including updated direct-access rejection/close behavior. Strict `cargo clippy --locked --all-targets -- -D warnings` passed (`clippy.log`). `just package` is now building the standalone optimized artifact (`package.log`); it regenerates the real bundle during Cargo build. The 433 compiler/build input hashes are retained in `compiler-inputs.json`. Focused navigation check resolved 168 paths with zero missing targets (`navigation.json`). No prior backend case suite was repeated.

The user manager bus is unavailable on this host, so the demonstration will use the product's detached daemon lifecycle and an explicitly recorded detached HTTP process rather than claim a systemd service. The persistent root is `/home/long/.local/state/agentx-observability-demo` (data, workspace, logs, bin); it was newly created and contains sample orders, not existing user data. No daemon or HTTP process is started until the packaged artifact is ready.

## Real-use blocker: ordinary request rows exceed detail cap

The optimized first artifact (`1f3001600275ad1edace8d7583be6562674fc76eae322a6d5f536b56ba209609`) passed 12 independent direct-LAN/embedded checks. Real product startup succeeded: daemon and Codex `lan-demo-rune` ran, and the agent authored `workspace/summary.md` with the correct total2800. However actual browser acceptance failed at the substantive requirement of nonempty request data. Runtime snapshot shows66 ordinary decoded/completed calls and66 dropped request details, despite no retained-row evictions. This is actual product-input sizing, not a UI mock or provider failure. The source cap of2048 bytes is applied at decode, lifecycle updates and completion; the actual scope/refs/chronology/configuration growth requires a coherent native sizing correction, preserving total history bounds and explicit losses. Controller owns actual product rerun; a focused runtime owner will repair only this diagnosed native issue. The requested demo remains isolated and first-stage web is available while correction proceeds; acceptance is not complete.

CLI friction: the old passive `observe` command rejects the global --name argument required by this agent session (exit2 retained in demo logs). This unrelated preexisting compatibility issue is not used as evidence of native failure; actual HTTP reads provide the selected runtime/data evidence.


## Runtime request-sizing source release

Runtime sizing owner released `src/observability/registry.rs`, `src/observability/registry_tests.rs` and the matching [runtime budget guide](../../.docs/observability/runtime.md#bounded-state-maintenance-and-concurrency) on 2026-11-08. Diagnosed cause: ordinary scoped request identity, chronology and configuration references exceed the former 2 KiB encoded-detail budget, causing whole-row loss at decode/update/completion. Those three request checks now share `REQUEST_DETAIL_BYTES = 8 * 1024`; the closed-connection check independently uses `CONNECTION_DETAIL_BYTES = 2 * 1024`. No request fields, native types, serialized schemas or protocol/business code changed. Shared completed-history limits remain 2 MiB, 1,024 rows and five minutes; active request slots remain64, bounding their retained payload to512 KiB. Larger history rows consume the existing byte budget and retain explicit drop/eviction behavior.

Authored regression: `observability::registry::tests::ordinary_long_scope_request_retains_full_lifecycle_and_configuration` (exact filter). It creates the demonstration-shaped long scope without disk access, UUID daemon/generation/request identities, observed peer and invocation/attempt origin; traverses queue, dispatch, Application return, response availability, write and worker completion; records all eight supported configuration refs; and requires the original identities/evidence, nonempty completed list/detail, zero detail drops and correct completion counters. Its naturally encoded completed row must exceed the former2048-byte limit; no padded strings, truncation or fabricated unknowns make it fit. Existing oversized-field/drop coverage remains unchanged.

Static source inspection only; this owner ran no Cargo, Node, formatter, tests or product probes. All owned source is released; no processes/resources were created. Controller alone formats and executes the exact regression, rebuilds the coherent artifact and verifies actual live request retention. The prior66/66 detail-drop evidence remains the historical pre-correction blocker; this source release does not claim runtime acceptance.

## Final executable evidence and operating disposition

Evidence root: `/tmp/agentx-lan-9jf25e31`. Selected independent checks: adapter11 and Web state9 passed; after the substantive native fix all14 registry cases passed, including the new full-lifecycle/long-scope/eight-config-ref regression and existing oversize/gauge cases. Corrected strict all-target Clippy passed30.12s; final affected-source format check passed. The first optimized packaging build passed9m56s and first actual12-case LAN/embedded checks passed. Failed first browser/no-row evidence and old artifact remain preserved, not converted into successes.

The corrected standalone image passed12 actual numeric-LAN/embedded checks from an otherwise empty unrelated cwd, with no external assets or credentials and exact test-service cleanup. Real Chrome147 passed5 changed end-to-end cases: direct page, ordinary reload, independent browser context, actual request/event investigation, and closing one page without revoking the other. It displayed16 real request rows and128 real event rows, with zero browser errors. Subsequent direct native HTTP receipt retained62 real request rows and17 captured configuration traces with **zero request-detail drops**; these are time-bounded observations, not permanent or complete counts. Runtime, events and configuration preserve their original retention/loss semantics.

`corrected-live-data.json`, `actual-browser-02/browser.json`, `corrected-lan/{acceptance,cleanup}.json`, `live-receipt.json`, `correction-builds.json`, `corrected-clippy.log` and `final-format.log` own exact execution results. `corrected-source.json` records433 build/compiler inputs; only `registry.rs` and `registry_tests.rs` differ from the first packaged input set. The14-case regression ran53.593s and corrected executable build87.053s. No unchanged backend campaign was repeated.

Actual request-size repair binding: `{serverId: srv_yDbm9zuqIMiM, agentId: f846b723-1900-4a2e-94fc-d8def0094640}`. Its exact native/source handoff is above; controller independently formatted, built, tested and exercised real product inputs. No new schemas, domain effects, larger total history budget or invented observations were introduced.

The first real Codex agent `lan-demo-rune` authored `workspace/summary.md` (3 orders,6 units,total2800). It was cleanly retired before replacing only this demonstration's daemon/image; original SQLite generation/data persisted. The corrected managed Codex `lan-demo-zori` authored `workspace/verification.md` and remains listening. `logs/live-launch.stdout` and `logs/latest-list.stdout` retain actual launch/readiness/managed-state evidence. Actual originating commands captured timeout17 and29. Demo startup/stop/restart instructions, scope and logs are durable in its README.

The running console PID1497500 and daemon PID1497466 were checked against their exact demo executable/scope; managed agent readiness and process binding were verified through native product listing. Temporary acceptance adapters were reaped, browser contexts closed, and disposable roots removed. No compiler/check job remains. Persistent demo processes/resources are intentionally retained for the user; no reboot startup or installed/live-default cutover was configured. LAN address was tested through the host's real numeric interface and actual browser; no second physical device or other-platform/toolchain claim is made.
