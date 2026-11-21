---
schema: task
status: complete
owner: sake
description: Implement native Codex shared-daemon discovery, compatibility, lifecycle and transport.
links: [../AGENTS.md, ../delivery.md, ../contracts-handoff.md, ../../../.docs/proposals/codex-shared-runtime.md, ../../../.docs/codex-activation/topology.md, ../../../.docs/codex-activation/managed-service-evidence.md, ../../../.docs/codex-activation/protocol-feasibility.md]
---

# Assignment

## Working record

- Runtime binding: `sake`; controller `zomi`; architecture/contracts `kana`.
- Integration correction #1503 resolved: independent production evidence confirmed native shell provenance caused a false environment conflict; the conservative normalization/dependency guard is implemented and validated. No user resources or Cargo execution were involved in this worker's correction.
- Correction model: `launch baseline → official daemon → native tool environment → child launch` adds native tool/terminal/pager markers and prepends the selected package's `codex-path`; full-environment equality incorrectly treats this native provenance as changed backend requirements. Independent snapshots are `production-environment/csr-t6cmjnbd/{fixture-environment,shell-environment-A1,shell-environment-B,shell-environment-A2}.json`. The A/B probe also adds its own `ACCEPTANCE_ROOT`; the original parent-child helper did not, and this fixture key is not excluded.
- Released correction after kana #1644/#1659: strip only explicit native shell/tool/presentation provenance, normalize redundant LC_CTYPE and the exact leading helper directory from the selected managed package, retain user PATH components/order and unknown env inputs. Known loaded config env_key/bearer_token_env_var/env_vars/env_http_headers/env references opt out, including absent->present; client-owned AgentX config dependencies fail explicitly. `service::environment_input_is_normalized` is the shared pure predicate for vume's per-thread rejection guard. Receipt now carries per-key digests and reports differing fields/key names without values. Authored cases protect native child reuse versus credential/proxy/PATH/locale/unknown-input/dependency conflicts; no test execution by sake. Source writes are released.
- Result: implemented the released [production contracts](../contracts-handoff.md), with `ManagedCodexService`, managed acquisition in `src/integrations/codex/service/managed.rs` and a direct Unix WebSocket transport in `service/transport.rs`. Local assignment is handed off; independent production validation and Delivery acceptance remain with integration.
- Evidence boundary: no user daemon management, user configuration/credentials, installed executables or broad checks. Native probes require demonstrably disposable service paths.
- Native evidence: namespace-isolated managed concurrent start converges; control socket is WebSocket; raw proxy does not translate JSONL; complete native package required; backend `-c` arguments are ignored. `thread/start.config`, `thread/resume.config` and `thread/fork.config` successfully enable, trust and execute UserPromptSubmit without process hook flags, preserving the official daemon path. Their production composition remains integration work.
- Connection lifetime: another client's active turn survives an idle connection closing, with the same native generation and a later completed turn. Closing the creating connection can unload its own root; do not promise its active turn continues merely because the daemon lives.
- Working artifacts: `.agentx/.tmp/deliveries/codex-shared-runtime/native-service-probe/{connection-summary,thread-hook-summary,thread-hook-lifecycle-summary}.json`. Exact native hash is `cdcb45aaa01d0f14e298985e09465aebccd47f64ecd298aea44814af7b62bbda`.
- Official source: OpenAI Docs app-server page fetched through `proxy curl`; installed CLI remains the authoritative managed-daemon version basis.
- Follow-up #1718: service dependency guard now also retains MCP `bearer_token_env_var`; extended the configured-dependency case with a marker-looking bearer variable and absent->present assertion. This aligns with vume's final merged thread-config guard. Source released; execution remains lobo-owned.

## Independent correction acceptance

- lobo #1754: 9 focused service/configuration cases passed with no compiler warnings, including normalized absent-key dependencies, opaque-field preservation and final merged thread-config guards; evidence [service-compatibility.log](../../../.agentx/.tmp/deliveries/codex-shared-runtime/service-compatibility.log).
- lobo #1800: real generic native and installed unmodified Paseo `0.15.3` provider API parent-child scenarios both passed; native parent tool launches an actual AgentX/native child, parent becomes idle, child message causes autonomous continuation without a new outer prompt, parent sends a business reply and child records it. Exact identities, delivered cursors and confirmed claims were checked in real SQLite. Evidence [production-parent-envfix/summary.json](../../../.agentx/.tmp/deliveries/codex-shared-runtime/production-parent-envfix/summary.json); all test namespaces cleaned by integration.
- Integration also reports 402 affected shared-layer regressions passed. Final fmt/Clippy/package and complete Delivery acceptance remain integration/controller work. This local Task closes the owned service correction and releases all source/resource claims; it does not close the Delivery or extend supported native versions/platforms.

## Handoff and evidence boundary

- Callable result: `service::ManagedCodexService` is a stateless unit struct implementing `CodexService::connect`. `ServiceRequest.args` contains app-server and only its stdio selection after the bridge has translated thread overlays. The returned transport has not performed initialize, preserving the outer client's handshake.
- Service support: Linux and native `0.167.0`; first native `daemon start` only, native start-report/PID/peer checks, fresh execution token bound to OS incarnation, credential-free compatibility receipts under AgentX storage, conservative conflict for unknown/incompatible services, bounded frame/timeout handling and connection-only close/drop. No automatic restart/adoption or second backend path exists.
- A9.4 disposition (vume request #1264): unsupported service versions are explicit acquisition errors. Degraded forwarding is meaningful only after compatible shared acquisition when missing observation capabilities affect autonomous input; no supported `0.155` shared transport has been established. The topology records this boundary; architecture/integration own the corresponding acceptance wording.
- Integration warning disposition (lobo request #1432): removed unused `ServiceRequest.cwd`; the bridge owns session cwd and native service acquisition runs from canonical home. Coordinated initializer removal with vume. `UnsupportedPlatform` is compiled only on unsupported platforms. No Cargo execution by this worker.
- Authored cases: client identity/cwd environment normalization, rejection of ignored startup flags, stale generation/configuration receipt fences, real Unix WebSocket JSON and connection lifetime. Integration's separate transport cases are registered in `service/acceptance_tests.rs`; RawValue validation preserves opaque numeric fields. No normal tests, Cargo checks, formatting jobs or builds were run by this worker.
- Model updates merged: [topology](../../../.docs/codex-activation/topology.md) and [managed native evidence](../../../.docs/codex-activation/managed-service-evidence.md). The [reusable native probe](../../../.docs/codex-activation/.tools/probe_managed_service.py) consolidates executed experiment procedures; the consolidated entry itself is newly authored and left to integration to execute.
- Receivers: vume has the callable service/composition contract and start/resume/fork native evidence; lobo owns compile, lock resolution, cases, production connector/native composition and user-outcome verification. Unmodified Paseo, other native versions/platforms and indirect plugin/config hot updates have not been established here.
- Resources released: all native experiments used hidden user home/runtime mounts and private PID/network namespaces, stopped their disposable daemon in finally, then exited the namespace. Removed copied native packages and disposable homes/state (about 445 MiB); retained compact evidence and `cleanup.json`. Installed executables, user daemons, user config and credentials remain unchanged. Service source and Cargo.toml writes are released; Cargo.lock resolution remains lobo's.

You are the native Codex service/runtime professional. Establish the real managed daemon/proxy contract for the installed native version and implement the service/transport capability consumed by the protocol bridge. Own the technical choice within the user's shared-native-daemon goal; escalate a substantive incompatibility with evidence rather than silently reverting to one backend per launch.

Immediately investigate the installed API/CLI, endpoint discovery, repeated/concurrent start, backend generation, hooks/config compatibility, resource cleanup and client disconnect semantics. The earlier multi-client experiment used a private Unix/WebSocket listener; native managed daemon end-to-end behavior remains unverified. Never assume those socket protocols are identical.

## Outputs and ownership

Own the service/transport design at `.docs/codex-activation/topology.md` and necessary service-specific evidence, plus the implementation leaves assigned by architecture under the Codex integration. The architecture owner publishes public signatures and shared exports in `../contracts-handoff.md`; you may propose required refinements, but do not guess sibling APIs or rewrite their modules.

Return a real reusable backend connection and sufficient service identity/compatibility information to avoid selecting a second private backend or killing shared work on proxy exit. Existing incompatible user services must produce a useful conflict, not an automatic restart or environment rewrite. Author focused cases for the integration owner.

## Verification and resource boundary

Use official documentation and installed declarations. Read the openai-docs skill when using it. Bounded external-runtime feasibility observations are permitted only with disposable home/project/service paths and fixed loopback model responses; establish that daemon-management effects stay isolated before invoking them. Do not touch current user daemon, systemd units, credentials, configuration or installed executables. If isolation cannot be established, provide the exact unsupported condition and continue static implementation work; seek a concrete architecture resolution.

The integration owner runs normal tests/builds. Preserve discovery evidence, selected assumptions, supported versions and exact cleanup. Publish usable service interfaces/results to the bridge owner and controller when they release dependent work. No further delegation; claim the actual runtime name in metadata and complete with model updates and resource release.
