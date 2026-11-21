---
schema: task
status: active
description: Derive and serve a reproducible five-operation HTTP/OpenAPI boundary.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"hini","incarnation_bits":4745315139478759651}}]
---

# Derive and serve a reproducible five-operation HTTP/OpenAPI boundary

Read [feature entry](../AGENTS.md), [Delivery](../implementation.delivery.md), [accepted proposal](../../../.docs/proposals/contract-service-projections.md), [contract design](../../../.docs/api-panorama/service-contract-design.md) and [panorama](../../../.docs/api-panorama/README.md). Read relevant source/module AGENTS before changing code. User authorized implementation; Controller lezi owns assignment and shared resource coordination.

## Responsibility and authority

Profession: Rust HTTP/contract-projection specialist. Bound to hini (domain-contracts, historical adopted profile codex-gpt-6-1-sol-high; resumed actual launch overrides reasoning to max). Implement coherent native/domain/transport/consumer responsibilities and update their owners. No further delegation authority is granted by this Task alone.

Write/resource scope: src/management/web.rs and narrow HTTP projection/emission integration; relevant Cargo/script changes coordinated with native/integration owners.

## Start and collaboration

Start when pilot-native declaration shape is usable. Do not wait for all implementation to run schema/tool trial. Native runtime calls and final listener evidence depend on provider availability.

Use direct owner-to-owner contracts and handoffs where actionable. Preserve unrelated changes, current services, installed binaries and incident resources. Isolated scopes and named candidates only. Coordinate shared files, Cargo lock and build lanes; broad builds/package runs belong to integration. Resolve routine compatible choices autonomously, escalate only substantive incompatible promises.

## Required result

Consume the native five-operation contract. Trial maintained Rust/axum/serde OpenAPI integration against actual versions; record a compatible choice. Produce real OpenAPI with operationIds, input positions, strictness, response/error schemas and exact semantics. Connect handler and metadata through types, or explicitly verify their correspondence. Probe u64 precision, tagged union/nullability and runtime validation constraints before choosing SDK tooling.

## Completion and evidence

Real isolated HTTP agrees with emitted schema for success and decisive failure; repeated export produces identical contract. Keep Host/Origin behavior and existing consumers. Supply actual specification plus tool/config/version and mapping policy to consumers; cover relevant CS-02/03/06/10.

Record here the actual source/entry/config/tool versions, changed guarantees, executed checks versus authored cases, remaining obligations and handoff consumers. Task completion does not establish full Delivery acceptance, installed/default support or live adoption. Keep ordinary progress in this Task; report actionable dependency releases, blockers and completed handoffs to lezi through AgentX.

## Initial safe boundary

hini confirmed the exact Task participant binding and actual adopted role/profile using `agentx list hini -v --json`. Read feature entry, accepted proposal/design, native management contracts and HTTP paths. No source/Cargo files changed and no build started. Integration owner ruzu reserved broad build/package lane; naze owns native declaration/service/module-registration changes and a future focused native check window.

### Working structure and dependency decisions

- Keep native `contract.rs` DTOs and serde behavior authoritative. Add schema derives only to the public five-operation reachable closure; never derive/export internal Request/Response, WorkerInput, WorkGrant or InvocationContext.
- Candidate mechanism is pinned utoipa 5.4.0 ToSchema + typed HTTP metadata in `management::openapi`; compatibility is **not yet established**. Axum is 0.8; Rust manifest MSRV is 1.89 and actual compiler is 1.99.0. Avoid adding utoipa-axum unless the existing manual-router integration actually benefits.
- Naze supplies `ManagementService::new(scope, project, invocation, workers)` with describe/list_agents/get_agent/submit_message/get_operation, and reusable client/call/submit_command for nonpilot adapters. HTTP will bind those methods and remove duplicate worker/admission policy from Adapter.
- Naze was sent the reachable ToSchema closure (Descriptor, AgentPage/AgentView and safe child views, SubmitMessage/MessageInput, Operation/OperationStatus/CommandResult and all result children). RequestError in `src/agents/model_request.rs` needs a coordinated narrow schema derive; no edit yet. Naze will register openapi when created.
- Preserve ordinary v1 JSON numbers. SDK must decode u64/i64 losslessly with field-specific metadata (pending_messages, attachment size, RequestError counters/times); do not change wire numbers into strings or global-convert digit strings. Null output and optional input need distinct schema handling; externally tagged CommandResult is part of the trial.

### Probe and friction

`cargo info utoipa@5.4.0` returned registry configuration error: crates-io is replaced by `rsproxy-sparse`; retry with explicit `--registry crates-io` or use the configured source in an isolated tiny crate. No dependencies downloaded or selected yet. No project lock mutation, schema export, runtime acceptance or compatibility claim.

### Safe continuation boundary

Controller request #46293 requires a durable handoff for high→max reasoning reconciliation. This is that boundary: no running tool sessions, no temporary probe crate, no source edits, and no held build/Cargo lock. Resume by probing the candidate in a separate manifest/target, coordinate Cargo mutation with naze before his focused check, emit actual spec/codec supply early, then bind handlers. Ruzu requires pinned dependency/lock, actual export command/output, five stable operationIds, u64/null/union mapping and affected Web check command before freezing an integration candidate.


### Resumed implementation and current supply

Actual `/proc` ancestor argv on resume: codex PID 1970916, model `gpt-6.1-sol`, `model_reasoning_effort="high"` followed by explicit `model_reasoning_effort="max"`. Last override is max; resumed incarnation is 4745315139478759651. Historical adopted profile provenance is unchanged.

Pinned `utoipa =5.4.0`, feature `non_strict_integers`, locked transitive `utoipa-gen 5.5.0`. Crate MSRV 1.75 fits project 1.89. `cargo info ... --registry crates-io` resolved through configured rsproxy and succeeded. Tiny isolated crate `/tmp/agentx-hini-schema-trial` compiled exact selected native DTO definitions with ToSchema and emitted real component schemas. No utoipa-axum: existing fallback router does not need a second router layer.

Native DTO reachable closure now derives ToSchema in contract.rs and safe RequestError. `management/openapi.rs` owns public HTTP-only ErrorEnvelope/code mapping (exhaustive native ServiceError match), document assembly and offline export. `web.rs` real five HTTP binding functions carry path/parameter/response annotations and consume ManagementService; old worker/client/admission implementations now delegate to native service. `main.rs` dispatches `__export-management-openapi` before environment/config/runtime setup. Module registration is complete. Ordinary v1 serde representations and Host/Origin admission are retained.

Cargo.toml/lock mutation released to naze in #46661; naze started focused `cargo test --locked --bin agentx management::`. No broad build/package from hini. Trial of the real path annotations plus native DTOs is in progress in the isolated manifest. Candidate export command is `CANDIDATE __export-management-openapi > management.openapi.json`; frozen candidate/full native export and actual HTTP conformance belong to ruzu integration lane. Deru owns consumer generator/codec trial and receives actual spec when emitted. Proposed stable IDs: managementDescribe, managementListAgents, managementGetAgent, managementSubmitMessage, managementGetOperation.

Integer formats encode native sign/width: uint64/int64 require token-preserving bigint conversion/encoding; uint32/int32 remain safe numbers, doubles remain numbers. Opaque strings are untouched. `x-agentx-codec` records this mapping beside emitted schemas. CommandResult retains every existing externally tagged result variant; Option schemas remain nullable and input omission follows native serde. No new service error hierarchy or public Application/worker data was added.

## Current source handoff

Actual OpenAPI 3.1 specification is available at [management.json](../../../.docs/api-panorama/openapi/management.json), with [usage/provenance/codec guide](../../../.docs/api-panorama/openapi/README.md). Consumer deru and independent acceptance ruzu received it in #46718 and updated unsigned-format details in #46734. The isolated real native-source/macro trial compiled and emitted successfully after correcting one unsupported `schema(description=...)` attribute to native Rust field documentation. This friction is resolved in the owning source. Exact unsigned format names are **u-int64/u-int32**, superseding preliminary uint64/uint32 shorthand above.

Registered `just management-openapi CANDIDATE` and offline `CANDIDATE __export-management-openapi` exports; export dispatch precedes environment/config/runtime initialization. `just --list` confirmed recipe parsing. Source layout remains proportional: one HTTP projection module and real annotated web bindings, existing native service, no extra router layer/compiler/DTO registry. Public ErrorCode/ErrorEnvelope model only the existing HTTP representation and exhaustively map ServiceError; they are not a second Service error hierarchy.

Authored projection regression `management::openapi::tests::native_wire_and_schema_preserve_precision_tags_null_and_omission` covers native u64/i64 extremes, string identity preservation, serde tagged result, null/omission, strict unknown-field refusal and private-schema exclusion. Execution depends on naze's current focused Cargo invocation; do not claim it passed before that owner reports the test list. HTTP Host/Origin code was not changed. All five routes now invoke native ManagementService; nonpilot client/call/submit reuse the extracted service host policy.

Remaining evidence receivers: naze reports focused management compile/tests, including whether the new projection regression was present in its compile snapshot; ruzu owns candidate package/frozen native export byte comparison and isolated actual HTTP/schema conformance; deru owns real generator/codec/public package checks. Web compatibility check is `node node_modules/typescript/bin/tsc -p web/management/tsconfig.json --noEmit`, already part of build.rs candidate asset production; avoid rerunning separately if that build covers the same source. No installed program or running user service was changed. No broad Cargo/build/package run by hini.

### Focused verification window

Naze released its focused native window in #46907: `cargo test --locked --bin agentx management::` passed 34/34 (the new projection test landed after its compiler snapshot). Hini then notified ruzu and started exactly the missing `cargo test --locked --bin agentx management::openapi::tests::` scope. Ruzu acknowledged the serialized window in #46950 and will not duplicate that test. Source is frozen for its candidate package. Projection result and Cargo release are pending, not claimed passed.

The isolated emission extractor is being adjusted to retain native struct and real binding doc comments as well as derive/path tokens, so early artifact includes the same summaries/descriptions as full candidate export. This is artifact/probe correction only; frozen production source is unchanged. Exact generation equality is ultimately checked against actual candidate export by ruzu.

### Consumer interoperability result

Deru's actual HeyAPI 0.99.0 trial established that its built-in bigint transformer ignores u-int64 and complex externally tagged unions, and generated fetch parses text with JSON.parse. The chosen supported path is generated `parseAs:text`, then thin schema-directed lossless decoding before any numeric conversion; a supported number resolver derives bigint types for exact formats. Fixed lossless-json 4.3.3 and local TypeScript 5.9.3 (generator incompatible with root TS7) remain in the standalone consumer package; no root JS dependency changes. See [consumer Task](pilot-consumers.task.md) for the authoritative package, commands and checks.

Doc-preserving first spec SHA-256 is `f3f3dc8ab347f8e373519fd5c9679205b9b2430651685d7a7753f93d5f0e290a` (five paths, 27 safe schemas). This supersedes the earlier artifact hash without preceding doc comments; types, routes and codecs are unchanged. Actual candidate byte equality remains the integration check, not assumed from trial extraction.

### Author verification completed and integration lane released

`cargo test --locked --bin agentx management::openapi::tests::` passed **1/1**, zero failures/ignored, 3349 unrelated tests filtered out, test runtime 0.01s after 5m44s compile. It executed `native_wire_and_schema_preserve_precision_tags_null_and_omission` on the final frozen source. Only the same three pre-existing labels warnings identified by naze were emitted. No repetition of its 34 passed management tests.

Hini released the Cargo lane directly to ruzu and confirmed source freeze. Ruzu now owns named candidate package, offline deterministic export/native-byte comparison and real isolated HTTP/schema conformance; deru owns public consumer package generation/codec and tarball supply. HTTP source remains available for ordinary integration repairs. Task stays active until the declared HTTP evidence is received or Controller accepts a narrower explicit handoff; author checks do not establish live runtime adoption.

### Actual candidate export and consolidated integration

Ruzu built the immutable candidate `/tmp/agentx-contract-candidate-r1-99xo9tn2/agentx-pilot-r1`; repeated offline exports and the early artifact comparison are retained in [candidate-r1 evidence](../../../tests/contract_service_projections/evidence/candidate-r1). [spec-diff.json](../../../tests/contract_service_projections/evidence/candidate-r1/spec-diff.json) found only Cargo-inferred contact/license metadata: actual candidate includes contact name `agentx` and MIT name/identifier. All paths, types and codecs matched. Canonical management.json now copies actual `export-1.json` exactly (SHA-256 `793e0fa0abc1e37b051f3ceed57de1519ee7ce243eb0917a95663137738438d5`). No source change, rebuild, package generation or consumer reinstallation was needed for this metadata correction; deru keeps the existing consumer-03 package.

Human/controller #47354 consolidates full system implementation and concentrated integration: pilot runtime acceptance is not a blanket source gate. Ruzu released Cargo/source in #47416 and preserves its immutable candidate plus one reusable integration workspace/scope. This Task's source/projection handoff and focused checks are complete; actual full-chain/runtime evidence is retained by the consolidated acceptance owner. No full-stage source assignment is inferred from earlier ownership; next coherent HTTP batch requires explicit Controller binding. Current HTTP/Cargo sources are released, no owned process/lock/live resource remains. Ordinary pilot compatibility repairs stay available.
