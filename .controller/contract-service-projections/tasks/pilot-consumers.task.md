---
schema: task
status: in_progress
description: Deliver a usable TypeScript SDK and small HTTP CLI for the pilot.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"deru","incarnation_bits":4745315140615472573}}]
---

# Deliver a usable TypeScript SDK and small HTTP CLI for the pilot

Read [feature entry](../AGENTS.md), [Delivery](../implementation.delivery.md), [accepted proposal](../../../.docs/proposals/contract-service-projections.md), [contract design](../../../.docs/api-panorama/service-contract-design.md) and [panorama](../../../.docs/api-panorama/README.md). Read relevant source/module AGENTS before changing code. User authorized implementation; Controller lezi owns assignment and shared resource coordination.

## Responsibility and authority

Profession: SDK/CLI contract-consumer specialist. This Task is initially unassigned; lezi must bind an actual suitable Agent before dispatch. Implement coherent native/domain/transport/consumer responsibilities and update their owners. No further delegation authority is granted by this Task alone.

Write/resource scope: New client package(s), generation config and HTTP CLI entry; package/build integration coordinated with integration owner. Do not change native CLI grammar.

## Start and collaboration

Start consumer skeleton on native shapes; binding/generation only from actual HTTP spec. Public imports, codec and generator trial must precede broader consumer implementation.

Use direct owner-to-owner contracts and handoffs where actionable. Preserve unrelated changes, current services, installed binaries and incident resources. Isolated scopes and named candidates only. Coordinate shared files, Cargo lock and build lanes; broad builds/package runs belong to integration. Resolve routine compatible choices autonomously, escalate only substantive incompatible promises.

## Required result

Consume actual emitted OpenAPI; choose one compatible fixed generator by a real client probe. Separate generated wire code from handwritten connection/codec/named operations/receipt waiting. Implement five SDK operations and CLI describe/list/show/send/operation show/wait with the panorama semantics. Preserve request identity before dispatch; uncertain submission can be reconciled. Implement lossless management-number decoding as a shared policy; no blind Number conversion or unsafe casts. Record final public package/bin, browser-safe exports and user usage.

## Completion and evidence

A consumer from the public package entry and CLI artifact walks the pilot with no private workspace imports; help is offline; JSON stdout, stderr, exits, timeout and cancellation are explicit. Relevant CS-01/04/05/06/10. Supply candidate and exact commands to independent acceptance; no publishing to registry is required.

Record here the actual source/entry/config/tool versions, changed guarantees, executed checks versus authored cases, remaining obligations and handoff consumers. Task completion does not establish full Delivery acceptance, installed/default support or live adoption. Keep ordinary progress in this Task; report actionable dependency releases, blockers and completed handoffs to lezi through AgentX.

## Current state

Assigned to deru under lezi; implementation in progress. No integrated SDK/CLI acceptance claimed.



## Working model and implementation decisions

Human clarification in [controller](../controller.md#human-clarification-outcome-over-inherited-plan) governs: preserve consumer usability and substantive guarantees rather than inherited scaffolding. Read domain-contracts/coder methods, API generation and CLI methods; reuse accepted native objects and boundaries. No oRPC/Zod layer is introduced into this Rust-owned service.

Identity confirmed by native `agentx list deru -v`: incarnation 4745315140615472573, domain-contracts role, bound here. Actual codex PID 2039582 argv contains historical `model_reasoning_effort="high"` followed by explicit `model_reasoning_effort="max"`; effective last override is max. Historical adopted profile remains high. Initial broad process search was overly noisy; narrowed evidence to own ancestor argv. No default/runtime mutation occurred.

Chosen package home: `packages/agentx-client`, public package `@agentx/client`, separate Node bin `agentx-http`. Ruzu confirmed no placement/name conflict (#46647). Standalone package scripts/lock avoid concurrent root npm edits. Public SDK entry imports no Node modules. Skeleton package, tsconfig and module guide exist; generated DTOs intentionally await hini actual emitted spec. Candidate fixed tool trial: @hey-api/openapi-ts 0.99.0 and lossless-json 4.3.3 (npm registry availability checked); selection is provisional until actual schema probe. Generator must retain actual integer formats and field-level conversion, not globally convert numeric strings.

Structure: native-emitted OpenAPI → generator-owned wire declarations/client → schema-directed lossless codec/transport → named SDK methods + observation helpers → Node CLI. `send` consumes a caller-retained full SubmitMessage; it never invents a new request ID during retry. Unknown submission carries original immutable input, generation and request ID. `waitForSent` checks operation identity/status and external Sent tag; failures retain original receipt. Abort/deadline stops local observation without canceling native work. Node CLI prints JSON to stdout, diagnostics to stderr; offline help/version require no endpoint.

Current dependency: actual five-operation spec/export command and integer metadata from hini. Native source shape supplied by naze #46640; no generated binding or real HTTP claims yet. Ruzu receives a public packed tarball plus usage/signatures for fresh external installation and owns composed real HTTP/native/worker acceptance. Author owns ordinary repairs after handoff.

## Implemented source and tool trial

Public boundary: [package manifest](../../../packages/agentx-client/package.json), [browser-safe SDK entry](../../../packages/agentx-client/src/index.ts), [named methods and receipt helpers](../../../packages/agentx-client/src/client.ts), [schema-directed codec](../../../packages/agentx-client/src/codec.ts), [separate Node CLI](../../../packages/agentx-client/src/cli.ts), [generation configuration](../../../packages/agentx-client/scripts/generate.mjs), [usage](../../../packages/agentx-client/README.md) and [module entry](../../../packages/agentx-client/AGENTS.md). Root AGENTS now routes this package. No root package.json/scripts/lock, native CLI, installed program or live service was changed.

Actual source chain is native DTO + annotated HTTP binding → emitted [.docs specification](../../../.docs/api-panorama/openapi/management.json) → [retained package spec](../../../packages/agentx-client/management.openapi.json) → generator-owned `src/generated` SDK/types and schema/type map → handwritten lossless wrapper → public entry/CLI. First emission was hini's actual isolated native-source/macro compilation, not fabricated JSON; actual frozen integration-candidate export equality is still ruzu's obligation. Five operationIds are managementDescribe, managementListAgents, managementGetAgent, managementSubmitMessage, managementGetOperation. No broader route coverage is claimed.

Fixed successful toolchain: @hey-api/openapi-ts 0.99.0, TypeScript 5.9.3, lossless-json 4.3.3; standalone package lock. The initial TypeScript 7.0.2 failed because generator needs TypeScript compiler AST APIs; fixed compatible TS 5.9.3 locally. Initial npm dependency report identified js-yaml transitive findings; pinned override 4.3.2 produced install result `found 0 vulnerabilities`. No account-wide package/tool change.

The actual generator trial found built-in transforms inadequate: u-int64 was emitted as number, external-tag unions were skipped, and generated fetch uses `JSON.parse(text)` instead of `Response.json()`. Selected supported generation resolver emits bigint for u-int64/uint64/int64; no generated output is hand-edited. Generated SDK is configured `parseAs: text`; the public wrapper receives untouched response text and converts by original emitted component schema with lossless-json before Number conversion. u-int64/int64 bounds return bigint; u-int32/int32 are bounded safe numbers; double remains finite number; opaque strings never convert. Decoder validates required fields, structural primitive/collection/union types and integer bounds before its localized schema-to-generated-type assertion. Native cross-field/authorization/state constraints remain native. Actual CommandResult children currently have no reachable u64 field; precision cases use AgentView/RequestError, and external-tag cases independently check actual result variants rather than inventing a numeric field.

CLI decisions refine the inherited panorama for this pilot: default JSON only, explicit exact AgentRef rather than ambiguous name-only resolution, matching `management show`, `agents list/show/messages send` and `operations show/wait` command groups. Environment/config names retain AGENTX_API_* for management URL/timeout; default config directory uses final bin name `agentx-http`. Local wait defaults 30 seconds/250 ms; per-attempt timeout 15 seconds. Exact options, precedence, constraints and exits are in the actual `--help` declaration and package README. Full `--input` and plaintext `--file` can each use stdin, never both. Pending full requests are exclusive-created mode 0600 and fsynced before POST; same-content replay is allowed without overwrite. Output is lossless JSON stdout plus diagnostics stderr. Unknown/timeout/interruption retain original Q/G/input and do not cancel native work.

## Executed author evidence

- `npm run generate --prefix packages/agentx-client -- ../../.docs/api-panorama/openapi/management.json`: successful actual five-method generation, custom integer type resolver applied, no postparse transformers. Later same-source `npm run generate --prefix packages/agentx-client` produced identical hashes for every generated TypeScript file.
- `npm run build --prefix packages/agentx-client`: successful public declarations/SDK/CLI compilation. Root ambient Bun types initially leaked into the standalone fetch type; explicit `types: [node]` fixed package isolation. Packaging uses `scripts/build.mjs` to clean only package dist before compile, excluding stale old transformer output.
- `npm test --prefix packages/agentx-client`: five grouped tests passed after final behavior correction; [cases](../../../packages/agentx-client/tests/consumer.test.mjs) use the actual generated transport and public SDK with finite fetch response injection. Cases cover u64 boundaries/int64 extrema and exact encoding, null/false/empty/digit-string preservation, full payload snapshot on response loss/no retry, structured refusal versus unknown, wrong receipt identity/non-Sent/multiple tags/Failed/Unknown, observation cancellation/deadline without mutation, repeated label/false query encoding and offline CLI help/unknown-option exit. These are finite consumer tests, not real HTTP/native/Provider evidence.
- `npm pack ./packages/agentx-client --pack-destination /ABS/.../consumer-02 --json`: successful clean prepack build and 48-file public package, 45024 compressed bytes. Earlier pack attempt used an incorrect cwd-relative setup path and failed before artifact creation; only own empty accidental directories were removed, then canonical root/workdir invocation succeeded. No installed/global package was changed.

## Immutable handoff and current remaining work

Released [consumer-02 tarball](../artifacts/consumer-02/agentx-client-0.1.0.tgz) with [source/spec/lock/generated SHA256 manifest](../artifacts/consumer-02/sha256.json). Tarball SHA256 `85d60ee9c076f1b600930e4e45e062abdae2cd152093e127b9ddb7dc18283a1e`; package spec SHA256 `f3f3dc8ab347f8e373519fd5c9679205b9b2430651685d7a7753f93d5f0e290a`; package-lock SHA256 `0988464e6768b97df42f9c6a3311b0152159ad8740cd75ccd345fcb2968e4418`. consumer-01 is an earlier pack with old maintainer-command text, not the released candidate. No immutable artifact is overwritten after handoff.

Direct supply to ruzu #47142 contains absolute artifact path/hashes, npm install/public import/bin, generator reproduction and full CLI stdin/pending/recovery usage. Lezi notified #47143. Public signatures were supplied #47013. In a fresh external project run `npm install /ABS/consumer-02/agentx-client-0.1.0.tgz`, import only `@agentx/client`, invoke `./node_modules/.bin/agentx-http`. README supplies the full browser/public SDK workflow and installed CLI commands; acceptance does not need workspace-private imports.

Current remaining work: ruzu's external tarball install/browser-safe import and actual composed HTTP/native/worker/response-loss integration; actual frozen native candidate export equality; any ordinary integration repairs remain deru-owned. No full Delivery acceptance, installed/default adoption, live service transition or actual Provider behavior is claimed. This Task remains in progress through that bounded integration boundary.

## Integration repair: CLI cancellation context

Ruzu #47242 independently found [installed consumer-02 CLI SIGINT evidence](../../../tests/contract_service_projections/evidence/package-r1/cli-cancel-r1.json): exit130 survived but `main`'s generic interruption catch masked the typed `ObservationStopped` and discarded its identity/lastOperation. The source correction keeps structured `ObservationStopped`, `ReceiptError`, `ManagementError` and `OutcomeUnknown` flowing to their existing error mappings; only unclassified interrupted reads use generic cancellation. Native work remains unchanged.

Added [targeted CLI cancellation regression](../../../packages/agentx-client/tests/cli-cancellation.test.mjs). Executed `node --test packages/agentx-client/tests/cli-cancellation.test.mjs`: 1 passed, 0 failed. It runs the actual compiled CLI subprocess against an owned finite loopback HTTP listener, observes Running once, interrupts during the second GET and asserts exit130, empty stdout, exact original identity and last Running receipt in structured stderr, and no mutation request. Its listener/child are cleaned; this is finite consumer/HTTP evidence, not native/Provider behavior. No whole-suite repetition was required.

New immutable [consumer-03](../artifacts/consumer-03/agentx-client-0.1.0.tgz), SHA256 `ae1927daffe35dd17cdf9c593b1480b807ee9fb77f5eabf8aadca308efb17cce`, with [manifest](../artifacts/consumer-03/sha256.json), supersedes consumer-02 for subsequent integration. Spec, generator, lock, public API and version remain the same; clean prepack build passed. Older candidates remain intact. Ruzu receives exact replacement artifact and scoped evidence; full integration remains pending and repairs stay owned here.

## Consolidated execution boundary

Human steering via lezi #47354 supersedes fragmented per-handoff packaging/validation: retain the current optimized native candidate, consumer-03 and ruzu's one owned integration workspace/scope with controlled reset. Do not create additional environments or package generations for ordinary handoffs or repeat passing checks without a material changed guarantee. Current consumer source is frozen only for the actual active run; deru owns no Cargo process, live service, integration scope or external-install workspace. Root npm files remain unclaimed.

Full consumer implementation can proceed as one cohesive SDK/CLI/generator/codec batch when lezi releases its consolidated assignment/write window and HTTP supplies actual remaining management/Workbench/Observation specs and integer-marker policy. Independent pilot acceptance is not a blanket readiness gate. Current real integration defects route directly to deru; package reproduction is needed only when changed public source must be concretely consumed at the single integration point. Existing external tarball installation establishes its bounded public-package boundary and is not a template to repeat for every method or local change. Remaining real dependencies/resource boundary acknowledged directly to lezi in reply to #47354.
