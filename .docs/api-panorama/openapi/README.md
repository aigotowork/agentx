---
schema: document
status: active
owner: pilot-http
last_updated: 2026-11-17
links: [../service-contract-design.md, ../../../src/management/AGENTS.md, ../../../.controller/contract-service-projections/tasks/pilot-http.task.md]
---

# Public HTTP OpenAPI projections

The retained native export covers three listener-qualified OpenAPI 3.1 projections: management (30 method/path combinations), fixed-project operator workbench (3), and passive observation (2). All three canonical JSON documents come from successful full-batch native offline emission and have been consumed by the public SDK/CLI and exercised through the retained independent Linux HTTP/native integration. Actual source, package, and integration evidence is linked from the [HTTP Task](../../../.controller/contract-service-projections/tasks/full-http.task.md), [consumer Task](../../../.controller/contract-service-projections/tasks/full-consumers.task.md), and [full acceptance evidence](../../../.controller/contract-service-projections/tasks/full-acceptance.task.md). Exact fields derive from Rust/serde ToSchema declarations in `src/management/contract.rs` and `agents/model_request.rs`; HTTP paths, inputs and response types derive from annotations on the real binding functions in `src/management/web.rs`. The document assembles in `src/management/openapi.rs`. Workbench and observation each assemble their own native closed roots in `src/collaboration/web/openapi.rs` and `src/observability/web/openapi.rs`; their actual binding functions own HTTP metadata. Internal Application requests and worker capabilities are excluded. For the native Workspace member route and exact launch receipt additions, use the [current contract and handoff](../service-contract-design.md#workspace-membership-and-launch-identity) for source and generation status.

The canonical artifacts come from actual compile-03 native exports, retained under `tests/contract_service_projections/evidence/full-system/compile-03`; its emission-result.json records binary/spec identities, resolved local references and operationId uniqueness. Management has 30 operations/70 schemas, workbench 3/47, observation 2/174. The [final public package artifact](../../../.controller/contract-service-projections/artifacts/full-consumers/agentx-client-0.1.0.tgz) and [57-file regeneration evidence](../../../tests/contract_service_projections/evidence/full-system/regeneration/result.json) establish consumer generation and repeatability. Initial tiny-crate and pilot candidate evidence remains historical. Offline emission itself is independent of listener startup, environment-file loading and native database access; the separate full-acceptance record supplies the composed listener/native/worker evidence and its explicit limits.

With an explicitly selected built candidate, regenerate using:

```sh
just management-openapi /absolute/path/to/candidate > .docs/api-panorama/openapi/management.json
```

For the other independent listeners, use `just workbench-openapi CANDIDATE` and `just observation-openapi CANDIDATE`, redirecting to workbench.json and observation.json respectively. Direct arguments are `__export-management-openapi`, `__export-workbench-openapi` and `__export-observation-openapi`. All dispatch before configuration/environment/native startup; an export cannot acquire a daemon. Ruzu owns the broad build/package lane and frozen candidate comparison for this Delivery; do not use the default installed binary or restart a running service to export a development schema.

## Wire and codec policy

Management retains ordinary v1 JSON number tokens. The pinned emitter's `non_strict_integers` feature emits unsigned widths as **u-int64** and **u-int32**, signed 64-bit as **int64**, and floating timestamps as **double**. `x-agentx-codec` is the machine-readable mapping: u-int64/int64 require lossless numeric token parsing plus schema-directed bigint conversion, u-int32/int32 remain safe JavaScript numbers, and double remains a number. Never call JSON.parse before converting an unsafe token, change v1 numbers into strings, or globally convert numeric-looking identity strings. Encoding bigint requires raw unquoted integer tokens with signed/unsigned bounds checked at declared fields.

Fields such as Agent incarnation/stopped_event, generation, request_id, message_id and revisions remain opaque strings even when they contain only decimal digits. Nullable fields retain null; omitted optional request fields use native serde defaults. Native checks own cross-field exact identity, text/metadata UTF-8 byte bounds, valid intents, current-only label filtering, existing attachments and durable journal semantics; schema descriptions record constraints JSON Schema cannot faithfully express. Native input objects reject unknown fields. All existing externally tagged CommandResult branches remain represented; a Succeeded receipt must not be assumed to contain Sent without checking the variant and original identity.

## Admission and recovery

All three listeners use the [open-origin browser policy](#browser-access-policy). JSON POSTs require exactly application/json, reject content encoding, and use existing size/deadline/strict duplicate-key decoding. This is a local/LAN listener contract, not a new bearer-auth protocol.

202 means durable acceptance. Persist request_id, generation and original payload before dispatch; after transport loss, query the original operation. Unknown and NotFound do not establish absence of an external effect. Cancelling client observation does not cancel the worker. The HTTP error envelope has stable typed codes and a diagnostic message; messages are not a programmatic discriminator.

## Workbench and observation marker schemas

Both existing listeners use `src/browser/wire.rs`, not ordinary management JSON. Every native integer, including small counts and revisions, is represented by a strict single-key object such as `{"$integer":"18446744073709551615"}`. Bare JSON numeric values are rejected as operands. Marker decimal text must be canonical: no leading zeroes, `-0`, plus sign, fractional or exponent forms; negative values parse as i64 and nonnegative values as u64 before native field validation.

`src/browser/openapi.rs` projects each source integer schema into that real object shape. The `x-agentx-native-integer` extension retains its exact native format (for example u-int64 or int64), and `x-agentx-native-schema` retains native constraints; nullable integers become marker/null alternatives. Schema keywords such as array maxItems remain ordinary schema metadata. Identity strings are never rewritten. SDK generated DTOs therefore describe actual marker objects; a derived Native<T> mapping exposes bigint operands/results with schema-directed range checks and marker encoding. See the standalone consumer package for its implementation; no second manually authored business DTO inventory is required.

Workbench retains Descriptor, ReadCall, ApplyCall, Read, OperatorCommand and Response component names. A 200 Rejected is a domain refusal and remains distinct from a transport error. Browser writes contain only the nine OperatorCommand variants; AgentReport is available exclusively through the existing authenticated native actor path. Observation retains ScopeRef, ReadCall, BrowserRead and ReadResult, including all thirteen reads, LocalFailure, stale references, event gaps and clocks. The same `/api/v1/read` text belongs to two independently configured URLs and specs and must not be merged into a generic proxy.

Management attachments retain application/octet-stream uploads and raw downloaded bytes (including the existing raster media types). Inspector path-specific schema tags reuse native payload types and exclude collection/execution internals. All three listeners retain syntactic HTTP framing, content-type, size/deadline and strict JSON admission; schema cannot grant actor authority or enforce durable/concurrent effects by itself.

## Browser access policy

The [Web SDK adoption Delivery](../../../.controller/web-sdk-adoption/delivery.delivery.md) authorizes cross-origin browser, extension and iframe access. Current source uses shared `src/browser/admission.rs` in Management, Workbench and Observation. The [native Task](../../../.controller/web-sdk-adoption/http.task.md) and [integration Task](../../../.controller/web-sdk-adoption/integration.task.md) distinguish implementation from actual isolated/browser acceptance; this does not establish installed/live adoption.

| Boundary | Current contract |
| --- | --- |
| Origin / Fetch Metadata | No Origin or Sec-Fetch-Site admission gate; absent, opaque `null`, cross-site and extension origins use the same routes. |
| Host / request target | One syntactically valid Host is required; DNS aliases, IPv4/IPv6 and alternate authorities are accepted. No Host-to-socket equality check. The already-bound listener owns fixed scope/project; Host never selects native authority. Absolute-form request targets remain rejected. Workbench/Observation still reject queries; Management retains its strict route-specific query parser. |
| Preflight | OPTIONS returns empty 204 before body parsing/native calls. All responses, including errors/assets/raw attachments, carry `Access-Control-Allow-Origin: *`, methods GET/POST/OPTIONS, allowed headers `*`, exposed Content-Disposition/Content-Type and legacy `Access-Control-Allow-Private-Network: true`. No credentials/cookies/handshake; use Fetch credentials `omit`. Actual method/route/input/native guards still apply. |
| Embedding | Served documents omit X-Frame-Options/frame-ancestors denial; CORP is cross-origin. Existing document self-contained script/style/connect CSP stays in effect. Manifest paths and bytes stay bounded. |
| Native guarantees | Scope/generation/exact references, revisions, operator/actor separation, original write/receipt recovery, lossless codecs, bounded bodies/headers/time/concurrency and passive Observation remain unchanged. |

Browser mixed-content rules, Local Network Access permission (or older PNA behavior), extension host permissions and the containing page's frame-src/sandbox/CSP are platform constraints. Server CORS cannot override a blocked insecure request or grant browser/extension permission. An embedded document's own same-origin fetch does not establish direct parent/extension cross-origin access; the integration Task records those separately. The listener bind host remains a numeric IP and independently controls network reachability; request Host aliases do not change that launch setting.

Domain SDK resource requests add the optional [expected Scope header](../service-contract-design.md). Same-origin calls go straight to the resource. Cross-origin browsers may issue an ordinary CORS OPTIONS preflight for this header; the existing wildcard permits it without credentials, and it makes no native call.
