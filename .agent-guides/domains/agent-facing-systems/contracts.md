---
schema: scenario
status: active
description: Preserve native contract meaning across CLI, API, SDK and MCP adapters
tags: [contracts, cli, sdk, mcp]
---

# Native contracts and owned projections

Use when defining or changing schemas, operations, errors, effect metadata or their public bindings. Begin with the [model](model.md); retain a sound existing domain instead of rebuilding it for a new transport.

## Native declarations before projections

For a TypeScript oRPC + Zod system, keep executable Zod inputs/outputs, a service derived from those types, and native oRPC procedures as the authoritative chain. Put stable errors and intentional presentation/effect metadata beside those declarations. Let `ContractRouterClient`, `implement(contract)` and supported generators derive bindings; do not maintain a second operation/schema registry.

| Concern | Declare in the owning source | Enforce or adapt at |
| --- | --- | --- |
| Value shape | Requiredness, bounds, units, variants, nullability and input/output transformations | Native validation; supported protocol schema projection |
| Creation versus patch | Defaults on creation; omitted patch fields retain; explicit clear/false/empty meanings | Domain update rules plus CLI/input mapper |
| Operation meaning | Identity/scope, preconditions, acceptance/completion/observation and lifecycle guarantees | Domain/application service |
| Public failures | Stable code, safe message/data and meaning for recovery | Domain owner; adapter preserves or deliberately maps codes |
| Effect classification | Reads, writes, destructive effects, external access and idempotence scope | Implementation enforces semantics; metadata supplies conservative hints |
| Surface presentation | Deliberate CLI command/options, HTTP route/operation ID, MCP name/description | Corresponding adapter and generated descriptors |
| Transport behavior | Serialization, envelope, streams, exit status, timeouts and disconnection | Authored transport adapter, linked from its contract |

Prefer a typed native metadata extension over a new DSL. Include a field only when a real projection consumes it. Keep one owner for each presentation choice; metadata can reference a surface-owned mapping rather than forcing all adapters into one giant declaration. Version generated artifact formats only when a real distributed consumer needs that compatibility promise.

## Construct the derivation and generation scheme

Use this method when establishing or materially changing how a native contract reaches consumers. Begin with the required caller surfaces, deployment boundaries and existing declarations. Choose mechanisms against those needs and the installed stack. A new framework, a fixed directory template or an external example checkout is not a prerequisite.

Distinguish three mechanisms when explaining the scheme: type inference derives compile-time caller/value types; runtime binding connects procedures to implementations and transports; build-time generation writes derived artifacts. Combine them as needed. An inferred client or a runtime Handler does not require generated source files, and a generated descriptor still needs an invocation host.

1. **Identify authoritative facts and boundaries.** Locate domain types, boundary schemas, operation declarations and their consumers. Reuse types where meaning, lifecycle and compatibility agree. When an internal Service uses entities, rich values or a different operation shape, own a mapping to public inputs/results. Keep schemas and operation definitions connected through inference, composition or explicit correspondence; avoid independently editable copies of the same facts.
2. **Connect Service behavior to procedures.** Decide where trusted context enters, where input normalization occurs, which operation enforces domain policy and how results/errors cross the boundary. Use supported contract implementation/binding APIs where adequate. Handwritten binding is appropriate when it performs meaningful mapping or orchestration. Keep provider construction and cleanup at composition; domain policy depends on its required capabilities.
3. **Select a derivation path per requested surface.** Check existing providers, official APIs and compatible integrations for the installed versions. Decide what can be inferred, bound or generated and which additional policies the surface must own. Use the first adequate mechanism; introduce a narrow adapter or generator only for a demonstrated gap. Consult the linked official references and existing project commands for exact usage. An example can inform a decision without becoming a required dependency or permanent tool choice.
4. **Keep generated and authored responsibilities distinct.** Derive mechanical schema, registration and transport glue where useful. Keep business behavior, authority establishment, domain-to-wire mappings and surface policies with their owners. A Handler or framework Controller may adapt requests to a Service; choose the layers required by the host and avoid repeating the same translation through pass-through layers. Neither should acquire business rules merely because it is a generation target.
5. **Make the chosen path maintainable.** Record the actual source → mechanism → output or binding → package/host → consumer relationship in the existing design. Include the relevant command/configuration and version constraints where generation exists, plus unsupported mappings and their enforcement owner. Regenerate from the source and detect drift for maintained artifacts; use type/build checks and actual boundary calls for inferred or runtime-bound surfaces.

| Requested surface | Mechanism to evaluate | Remaining authored responsibility |
| --- | --- | --- |
| In-process Client or Service port | Native type inference and local procedure binding, or a direct typed capability where no protocol is needed | Domain behavior, context and resource ownership; mapping when Service and public operation shapes differ |
| Native RPC Client and server | Contract-derived client type, protocol link and supported server Handler | Endpoint/authentication setup, request context, Service binding, failure and lifetime policy |
| HTTP Client and server | Native HTTP/OpenAPI projection with a compatible handler/client, or an SDK generated from the owned HTTP description | Routes, status/security and serialization policy, Service mapping, framework integration where needed |
| CLI | Derived schemas, command descriptors or registration consumed by an existing parser/host | Grammar, configuration, input mapping, output, exit status and interruption |
| MCP | Derived tool schemas/metadata with a compatible integration or protocol SDK adapter | Tool registration/invocation, result/error envelopes, transport and connection lifetime |

For a substantive new scheme, trace a representative complete caller task and its deciding rejection or failure through the selected boundaries. Check that every exposed operation has a binding or explicit availability policy, that inputs can be obtained, and that results permit continuation. Select evidence for the promises changed; no bundled reference application, all-surface implementation or additional test framework is required. Stop when the requested surfaces are usable at the assigned delivery boundary and remaining mapping or enforcement limits are explicit.

## Public effects, errors and lifetime

For each substantive operation, answer these questions in native declarations or nearby linked semantics. Shared rules may be defined once.

| Question | Required distinction |
| --- | --- |
| What can change? | Persistent state, external effects and owned versus borrowed resources; a connect/open action can mutate even if its result resembles a query |
| What does success establish? | Accepted, started, observed or completed; include identity/freshness needed for a later query |
| Is replay safe? | Define input/target scope and observable result; repeated state convergence alone does not prove request/result idempotence |
| What if it fails after dispatch? | Rejected before effects, partial completion or unknown outcome; timeout/disconnection does not prove rollback |
| How does a caller continue? | Query/reconcile before retry where necessary; preserve existing operation identity if the domain supplies one |
| Who owns lifetime? | What closing the CLI, SDK attachment, MCP connection or observation does to underlying work and persistent data |
| What can be cancelled? | Distinguish stopping observation, disconnecting a client and cancelling/terminating actual work |

Mark effects conservatively. `readOnly` can coexist with external access. An operation that reaches a desired state on repetition may still return a conflict or perform a second external action. Do not infer idempotence from HTTP method, verb spelling or output schema. Destructive and external-access hints help a client reason; they are neither admission checks nor permission grants.

Preserve stable domain failure codes across transports where possible. Explicitly map validation, usage, unavailable transport and unexpected internal failures without presenting a technical failure as a domain rejection. Keep sensitive diagnostics out of public messages and machine stdout. For stateful effects, use the shared [effect and unknown-outcome guidance](../../scenarios/data-state-modeling.md#effect-boundaries-and-unknown-outcomes).

## Give every projection an explicit policy

| Projection | Derive or reuse | Own and verify |
| --- | --- | --- |
| CLI | Public input schema and operation mapping; generated registration where useful | Grammar/positionals/options, config precedence, parsing, machine stdout, stderr, exit status and offline help |
| HTTP / OpenAPI | Native routes, request/response schemas, errors, stable operation IDs and security declarations | HTTP serialization/status and absence mapping; OpenAPI dialect/tool compatibility |
| Typed SDK / RPC | Native client types and supported protocol bindings | Endpoint/auth setup, transport errors, cancellation/lifetime and public package entry |
| MCP tools | Native input/output schemas, descriptions and conservative effect annotations | Supported JSON Schema dialect, object envelopes, error signalling and connection lifetime |
| Capability index | Stable operation-to-surface mappings and effect declarations | Public format only when an actual consumer needs it; no universal discovery claim |

Record losses and policies where a mapping cannot express native meaning. Examples include omitted versus explicit `null`, flags versus structured patches, transformed Zod input versus output, rich native values versus JSON, and a domain list result wrapped in a protocol object. Revalidate with native schemas when the projected schema cannot enforce the full contract. The client still needs declared semantics beyond JSON Schema.

Use the shared [CLI method](../../scenarios/command-line-design.md) and [API/SDK method](../../scenarios/api-contracts-and-sdks.md) for detailed interface design. HTTP and native RPC are different wire protocols even when they derive from the same procedure; a generic JSON HTTP caller must use the supported HTTP route.

For MCP, use a maintained protocol SDK. Return both a machine-consumable result and useful text where the selected client expects them. If declaring `outputSchema`, check how the actual SDK/client validates structured success and tool-error results. A success-only envelope may reject a correctly signalled structured error. Preserve the domain result under a deliberate object wrapper and include a validated error branch when that is the selected policy; keep protocol-level errors distinct from tool execution failures. SDK behavior must be checked for the selected version.

## Select compatible providers

Use existing accepted tooling unless a changed requirement exposes a gap. Before choosing an integration, compare its peer dependencies, runtime, schema dialect, transport and result/error behavior against the project's fixed stack. Consult official documentation, package metadata and a bounded actual-client probe for unresolved behavior.

Choose the first adequate option: an existing compatible provider; a maintained compatible integration; a narrow adapter over an official protocol SDK; custom infrastructure only for a demonstrated remaining gap. Do not force a stack-wide beta upgrade just to obtain a convenience adapter. Record the selected versions, rejected incompatible options and reconsideration trigger in the owning design, linking sources. [Technology selection](../../scenarios/engineering-design.md#technology-selection-design) owns the general method.

Authoritative references: [oRPC](https://orpc.dev/), [Zod](https://zod.dev/), [OpenAPI](https://spec.openapis.org/oas/latest.html), [MCP tools](https://modelcontextprotocol.io/specification/latest/server/tools) and [MCP TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk). Follow the release/dialect supported by the selected provider rather than assuming the latest specification is implemented.
