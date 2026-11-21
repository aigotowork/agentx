---
schema: task
status: complete
agent: zomi
description: Design autonomous agentx activation across PTY and native Codex app-server hosting.
links: [../../.docs/codex-activation/README.md, ../../.docs/codex-activation/protocol-feasibility.md, ../../.docs/codex-activation/session-environment.md, ./protocol-check.md]
---

# Scope and required outcome

Produce a source-grounded contract, process and structural design for preserving autonomous agentx parent/subagent collaboration when Codex is hosted through app-server. An outer client such as Paseo retains its own agent model and native Codex protocol; agentx retains its identity, message routing and activation policy. Agentx must wake an idle parent on a subagent reply without user input, client polling of agentx or a Paseo-specific plugin.

This task authorizes design, native declarations outside production source, focused isolated feasibility experiments and documentation. It does not implement the production bridge, replace installed binaries, modify Paseo or its configuration, contact unrelated agents, alter credentials, or submit protocol probes to a paid model endpoint.

# Working method

1. Recover current contracts from identity binding, delivery, hook acknowledgment, process hosting and native protocol declarations.
2. Define identity/lifetime ownership, activation and delivery semantics, control-flow arbitration, protocol routing and failure/recovery invariants.
3. Compare native side-channel access, a protocol bridge and outer-client integration against the stated independence requirement.
4. Use an independent protocol feasibility investigation for the uncertain native behavior; perform non-overlapping design work in the parent.
5. Publish one design source, native contract sketch, scenario/acceptance mapping, evidence and unresolved limits. Make them reachable from root AGENTS.md.

The existing design guides are linked through root AGENTS.md. Relevant methods are architecture-quality review, engineering design, collaborative modeling and data/state modeling. Keep current facts, proposed contracts, observed evidence and implementation requirements distinct.

# Current findings

- `src/runtime/codex_stdio.rs` inherits standard streams and never parses protocol bytes; it has no independent activation path.
- `src/messaging/delivery/mod.rs` mixes pending-message policy with PTY input/screen state. Its cursor check is hook delivery evidence, not completed work.
- `src/integrations/hooks/common.rs::commit_delivery_ack` advances the existing cursor after successful hook output, guarded against replaced instance rows. This does not prove model reasoning or business completion.
- Local Codex-generated schemas distinguish client `turn/start` and `turn/steer` requests from server `turn/started` notifications. `turn/steer` requires the expected active turn id.
- Earlier suggestions that agentx activation should be delegated to Paseo have been corrected in `src/README.md`.
- Independent native process experiments are recorded in the linked protocol report. Plain app-server starts a backend per launch; daemon/proxy is a separate topology. Native Unix WebSocket supports two connections on one thread, but no direct side channel was found for an inherited-stdio child.
- A successful native start response can be followed by blocked input and an empty completed turn. Blocking a busy empty marker can terminate the preexisting active turn. The proposed native adapter therefore carries real prepared messages and confirms a correlated userMessage item; it does not use hook rejection as an idle lock.
- Root binding, bootstrap and command identity must be thread-scoped for multiple external roots in one backend. Current single-process fallback is insufficient.

# Design decisions and evidence state

- Selected target after the user's daemon clarification: one compatible native managed backend per service scope, with agentx connection adapters and an agentx-owned activation/control lifetime. A per-launch private backend was the earlier compatibility proposal; it is no longer the default target. Shared-service management and integration remain unimplemented.
- All supported app-server launch entries must resolve the same selected backend instead of starting another backend per proxy. Native daemon start/proxy declarations support this direction, but the earlier live multi-client probe used a private Unix/WebSocket listener; the managed daemon end-to-end path is still unverified. CLI/TUI paths that do not enter the adapter are not covered by a universal interception promise.
- Contracts distinguish participant incarnation, launcher execution, native home/thread and attachment fences. Message claims coordinate hook, explicit listen and native-input consumers.
- State and routing designs cover input ordering, server requests, notifications, user approvals, cancellation, rebind and unknown effects. Server-request IDs stay native because agentx introduces no requests toward the outer client; only client-to-native request IDs require a shared allocator.
- Reuse existing message storage, formatting and the declared hook-output receipt boundary. Do not promise exactly-once model execution or business completion.
- Independent evidence covers native sequential and busy behavior, hooks, multiple threads/connections, dynamic-tool requests, blocked input and valid active steering with batch correlation. The investigator owns the report and reproducible probe.
- Parent verification is limited to native declaration compilation, document links and requirement/contract consistency. The independent passed probe suite was not rerun by the parent, and these checks are not production integration proof.
- Provider layering is explicit: generic agentx services own collaboration semantics and consumer-required ports; Codex owns its daemon, native protocol and correlation logic. IntegrationSpec/launcher select the implementation. Non-Codex providers retain existing paths and do not have to implement the new activation port. R9/A11 now require affected-provider compatibility verification before production acceptance.

# Remaining implementation work

The acceptance inventory and implementation order live once in the design README. Root attachment/claim transactions, production bridge, native-input receipt handling, PTY integration and complete unmodified-client parent/subagent acceptance are not implemented by this design task. Native input APIs are experimental and the empirical profile is currently only Codex 0.167.0. Unknown-outcome reconciliation may require explicit recovery when reliable history evidence is unavailable.

Provider compatibility is part of that implementation acceptance: changes to shared identity, claims, hooks or launcher wiring require verification of affected existing provider paths. Codex daemon policy and native protocol correlation must stay in the Codex adapter; other providers retain their current launch, binding, delivery and lifecycle mechanisms.

# Completion criteria

The design specifies the complete parent/subagent round trip, public contracts and proposed module boundaries, protocol message routing including server requests, state transitions, failure semantics, compatibility limits, acceptance cases, evidence confidence and a proportionate implementation sequence. No runtime implementation is implied by design completion.

# Completion receipt

Completed on 2026-11-06. The design entry links topology selection, detailed protocol/state flows, native Rust declaration sketch and independent real-native/loopback-only evidence. Root AGENTS.md, the resource catalog and src/README.md route fresh readers to the canonical design without duplicating it.

The final Rust declaration sketch compiled with `rustc --edition=2024 --crate-type=lib --emit=metadata`; its output is `/tmp/zomi-activation-contract.rmeta`. A scoped structural check passed for eight Markdown files, 61 local links/anchors, the artifact catalog, R1–R8 requirements and A1–A10 acceptance inventory. The two later completion-receipt references were checked separately; the requirement/case inventory is unchanged. No production binary or outer-client configuration was changed. Production implementation and integration remain the work identified by the design, outside this completed design assignment.

# Follow-up: session environment boundary

The user's next question identifies a concrete shared-service prerequisite: every session must expose its own agentx identity variables to native tool commands and hook processes. Client process environment alone cannot change an already running server. The local thread/start schema offers a config overlay, but this does not establish which execution paths inherit `shell_environment_policy.set`.

Bounded investigation, inline: reuse the existing isolated native protocol fixture; create two threads in one private backend with distinct fake identity overrides, alternate real native exec_command calls, and inspect selected tool/hook environment fields. The fixed loopback fixture runs only an absolute temporary Python reader and never calls a paid endpoint. Preserve tool environment, hook environment and backend environment as separate facts. The targeted probe is `.docs/codex-activation/.tools/probe_session_environment.py`. Production source, service configuration and installed binaries remain out of scope. Record observations and update the design's shared-service prerequisite before closing this follow-up.

Completed on 2026-11-07. The first collector tried to write a file inside the read-only sandbox and failed; this was a fixture defect, not an environment-isolation result. The collector was changed to parse native commandExecution stdout, retaining the read-only sandbox. The focused A1/B1/A2 run then completed: all three commands got their expected fake identity variables and native thread id, while eleven hooks retained the selected backend environment values and carried the correct session id in their payloads. The canonical receipt and coverage limits are in [session-environment.md](../../.docs/codex-activation/session-environment.md).

Design resolution: shared native service remains feasible; add explicit thread tool-environment provisioning and hook payload-to-binding lookup. Environment values do not replace participant/attachment authority, and backend execution identity must not collapse multiple roots into one participant. Updated the design entry, topology, protocol flow and A4 evidence; no production behavior changed. Parallel turns, resume/fork environment behavior, long-lived tools, MCP environment and the production identity adaptation remain unverified implementation work under the original acceptance inventory.

Follow-up structural verification passed for the five affected Markdown files: balanced code fences and thirty local file/heading references. The native experiment was not repeated after its successful collector correction; the unchanged Rust contract compilation and earlier protocol probes were not rerun.

# Follow-up: selected shared daemon target

The user now selects shared native daemon hosting as the final app-server topology. Update the canonical topology, design entry and protocol lifecycle rules so they no longer recommend per-launch private backends or equate client EOF with backend shutdown. Reuse the existing identity, delivery and native routing contracts. Separate per-client transport state, shared service ownership and per-thread agentx ownership; ensure unknown or incompatible service configuration cannot silently select a second backend.

Bounded source check: native 0.167.0 help declares daemon start as start-if-not-running and proxy as a stdio client of the running control socket. No managed service was started or changed. Existing multi-client, thread-environment and hook evidence is sufficient to explain the target; a real managed-daemon plus unmodified-client integration run remains required before claiming production compatibility. No additional native probes, production implementation or configuration changes are included in this clarification.

Completed on 2026-11-07. The canonical design now selects shared native daemon hosting, distinguishes per-client connections from backend and thread lifetimes, rejects incompatible reuse rather than silently spawning another backend, and retains per-connection native routing semantics. Client EOF stops that connection's driver while preserving the shared backend and durable mail. The runtime ownership sketch and A9 acceptance criterion now include discovery/reuse and exit isolation. The ExecutionId declaration received a comment clarifying backend-generation scope; no declaration signature or production code changed.

Scoped verification passed for four affected Markdown files, twenty-eight local file/heading references, code fences, the unchanged R1–R8/A1–A10 inventories, and removal of superseded private-backend selection statements. The prior native probes and Rust compilation were not repeated. Actual managed-daemon transport, concurrent startup, configuration compatibility, service generation and complete unmodified-client behavior remain explicit production integration obligations.

# Follow-up: provider layering and compatibility

The user explicitly requires provider-neutral agentx services and provider-specific implementation boundaries. This clarification remains design-only and reuses the current registry, launcher and owned ports rather than introducing a mandatory service framework for every provider. Preserve the ability to launch and collaborate across heterogeneous agents without making non-Codex tools implement app-server, native thread semantics or daemon reuse.

Source basis: integrations/registry.rs owns IntegrationSpec configuration, runtime/launcher.rs selects existing hosting mechanisms, and messaging/delivery still contains shared PTY behavior with per-tool gates. These are current implementation facts, not proof that dependency direction is already enforced. The draft exposes two structural issues to resolve: Codex-specific backend reuse policy was placed under generic runtime, and common delivery receipts carried native client/item identifiers. Move proposed service policy into the Codex integration, normalize verified provider input into common receipt semantics, and retain native correlation in the Codex adapter. Add an explicit provider-compatibility requirement and acceptance case. No existing provider implementation or live service is modified.

Completed on 2026-11-07. The canonical design now includes a provider-layer ownership table and runtime-call view, locates proposed Codex daemon policy at integrations/codex/app_server/service.rs, and preserves generic process/I/O mechanisms in runtime. Common DeliveryEvidence now uses ProviderInputRecorded without raw Codex client/item ids; the Codex protocol flow owns their verification before normalization. NoInputEvidence uses provider-neutral rejection terminology. Other providers keep their existing interfaces, defaults and execution scopes; they are not required to migrate to ActivationDriver for this feature.

Verification passed: the changed standalone Rust declarations compiled to /tmp/zomi-activation-provider-contract.rmeta; three Markdown files passed code-fence and twenty-five local link/heading checks; the R1–R9/A1–A11 inventories and receipt/ownership boundaries were checked. No native probes, live agents or provider runtime regressions were executed. Production implementation must verify affected shared paths and heterogeneous parent/subagent communication under A11; this design check does not certify existing providers after future code changes.
