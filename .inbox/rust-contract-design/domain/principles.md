---
schema: document
description: Principles and concrete decisions for Rust-owned contracts and generated interfaces
---

# Principles and decision basis

Apply these principles to the boundary being designed. They describe a method, not a fixed project template. They reflect the original design goals, refined through a bounded Rust/TS experiment and structural review. See the [AgentX contract guide](../../../.docs/architecture/contracts.md) for how the project applies them.

## Model the capability before its delivery mechanism

Define the domain vocabulary, responsibility, consumers and useful operations before choosing HTTP routes or CLI flags. A contract includes native types, Service/function relationships, inputs, results, errors, names, descriptions and relevant public values. Document observable meaning that a signature cannot express when callers need it.

A domain model, public contract and wire representation can share types when their meaning agrees. Use a mapping when ownership, semantics or evolution differ. A database row is not automatically a public entity; a query result may be a snapshot rather than a mutable domain entity. DDD does not require a Repository, aggregate or Service class for every small capability.

## Keep one authority for each fact

Rust declarations are the maintenance source for Rust-owned types and operation signatures. Use docs and structured attributes or Rust registration for descriptions, supported constraints and target metadata. Derive OpenAPI, data schemas, TS types and Client bindings as selected outputs; do not maintain a second handwritten definition of the same API.

One source does not mean one file. A target-specific Rust module may reference an operation identity and own its mapping. Each fact still has one owner, and default/override precedence must be explicit. Separate serialization, runtime validation and target mapping; do not assume multiple derives automatically share every rule.

This default applies to Rust-owned boundaries. If an external service or existing protocol owns its schema, consume that authority rather than recreating it in Rust and claiming ownership.

## Depend on contracts and inject implementations

Consumers depend on capabilities, not concrete service construction. An implementation conforms to the native trait or generated interface. Parameters, constructors and composition factories provide the selected implementation and manage its lifetime. Keep required ports with the policy that needs them; keep an offered Service contract with its owning capability.

Use the same Service implementation behind multiple selected entry points. Alternative implementations and fakes are possible, not mandatory. A factory is useful where construction or selection needs a home; do not add a factory wrapper or DI container for every object. Contract clarity lets consumers, providers and tools develop in parallel without starting the full application.

## Generate interfaces; implement outside them

Never hand-edit generated artifacts. Change their source declarations or generator and regenerate. Keep business implementation, controllers, custom transports and middleware configuration outside generated output. Generated interfaces and native traits provide the connection; regeneration must not overwrite handwritten implementation.

Generated server interfaces must refer to the authoritative operation types. Generated clients must derive operation names and input/output/error relationships from the same contract. A stale artifact must fail a check that starts from the current native source; checking only a downstream file against a stale snapshot is insufficient.

## Separate types, declarations and behavior guarantees

| Mechanism | Its responsibility | What it does not establish |
| --- | --- | --- |
| Native trait and compiler | Enforce declared method/type relationships | Business algorithm correctness |
| Metadata and generator | Preserve supported information and produce associated bindings | Automatic behavior for every custom label |
| Value construction and decoding | Enforce rules implemented at that boundary | Arbitrary state-dependent domain rules |
| Generation comparison | Detect differences from the current source chain | Backward compatibility or semantic equivalence |
| Generated TS types | Check statically typed callers | Validate arbitrary runtime payloads |
| Service implementation | Produce the intended business result and own its state | Correctness merely because it compiles |

Sorting, pagination, transactions and business concurrency belong to the implementation. Describe promises consumers need and test implementation behavior where useful. Do not make a universal cross-provider behavior-testing system a prerequisite for the contract tool. Multiple frontends bound to the same Service do not require independently reimplementing business policy.

## Isolate complexity only when total complexity falls

Reuse native language mechanisms, standard libraries and suitable existing dependencies first. Add custom infrastructure only for a demonstrated gap. A wrapper, layer or package needs a present responsibility, consumer or independent build boundary.

| Situation | Proportionate decision |
| --- | --- |
| A small in-process capability already has clear types | Use the native module/trait; skip generation |
| HTTP imports CLI and exporter dependencies just to reuse an implementation | Extract the shared implementation or correct dependency placement |
| Every layer forwards the same method without a distinct policy | Keep a simple binding; generate default forwarding only if repetition becomes a real burden |
| A second output needs existing metadata | Reuse the description; add only that target backend |
| A hypothetical future protocol could need a plugin runtime | Defer it; a script or ordinary module is sufficient until needed |

Measure simplicity by understanding, change locality and required concepts, not by the fewest files or the most isolated crates. Do not solve a small coupling problem with a larger framework.

## Keep names and ownership semantic

Name a Service by its capability: a read/count interface can be `MessageQueries`; an error shared by those operations can be `MessageQueryError`. Short names are fine inside a clear namespace; public flat exports need enough domain identity to avoid ambiguity.

Name fields to express identity, units and absence meaning. Distinguish a stable operation ID from a Rust method name, and a public error code from diagnostic prose. Internal renames should not accidentally rename a wire operation. Keep API version, exported-description format version and build identity separate.

Directories and filenames should reveal the owner and role. Do not put reusable runtime code behind an unrelated demo application or create empty layers for naming symmetry. A fresh reader should find the contract, implementation, binding and composition point through the module entry.

## Keep outputs optional and extensions interpretable

An internal contract remains useful without HTTP, CLI or an SDK. Select the operations and outputs needed by the consumer. Generating a Client does not publish a server route; route registration, deployment and access control determine actual exposure.

Preserve structured metadata at the levels that the tools need: Service, operation, type, field, error variant and public value where supported. Validate known namespaces and preserve unrelated project namespaces. Metadata such as `retryable` or `sensitivity` is information until a named consumer implements its meaning.

## Separate asynchronous calls from streams and business concurrency

A normal async operation takes one input and eventually returns one result. Use it to await database or network I/O when that boundary needs it; an in-memory implementation can complete immediately. A sync boundary is also valid for an appropriate capability. Do not hide asynchronous I/O through blocking bridges in every adapter.

Streams, subscriptions, dynamic dispatch and global scheduling are separate requirements. Introduce them only for an actual use case. `Send`, `Sync` and `Arc` support Rust's execution constraints; they do not define transactions, idempotency or write ordering. The implementation owns those decisions when relevant.

## Verify the responsibility you changed

Use fake injection for consumer branches, compile checks for type relationships, complete generation comparisons for artifact freshness, and selected boundary tests for encoding and routing. Use implementation tests for business rules. Record the evidence and its limits; no single check replaces the others.

Apply compatibility checks when real released consumers require them, not as a reason to build a version-governance platform for an unpublished example. Evolve the smallest complete capability needed by the task.
