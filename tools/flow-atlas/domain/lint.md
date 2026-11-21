---
schema: document
status: active
owner: flow-atlas-domain
last_updated: 2026-11-11
description: Implemented diagnostic ownership, finite checking semantics and independent verification entry.
links: [./AGENTS.md, ./contracts.ts, ./README.md, ../../../.controller/domain-daemon/tasks/flow-atlas-domain-acceptance.md]
---

# Domain checks and coverage

This is the implemented workbench subset. The [earlier authoring rules](../authoring/lint.md) retain their own accepted scope. Authoring handles and native conversion are reused, but this compiler does not call the legacy inspector compiler or change its behavior. Diagnostic strings and exact paths are owned by the modules below.

| Group | Representative diagnostics | Source / bounded meaning |
| --- | --- | --- |
| Portable identity/reference | `INVALID_ID`, `DUPLICATE_ID`, `BROKEN_REF`, `NONPORTABLE_DEFINITION`, `NONPORTABLE_SCENARIO`, `UNSAFE_DOCUMENT_URI`, `EXTENSION_NAMESPACE` | `compiler.ts` validates IDs/references and portable inert leaves; `portableCopy` rejects callbacks/accessors/custom serialization/nonfinite values. Source/document text is never fetched or executed. |
| Native Schema | `SCHEMA_CONVERSION_FAILED`, `DUPLICATE_SCHEMA_OWNERSHIP`, `SCHEMA_ID_MISMATCH`, adapter `UNSUPPORTED_SCHEMA` | Native handle + io is authoritative. Conversion is explicit; parse/refinements/methods are not invoked. |
| Native method correspondence | `FOREIGN_NATIVE_METHOD`, `NATIVE_INPUT_MISMATCH`, `NATIVE_METHOD_OWNERSHIP`, `DUPLICATE_METHOD_OWNERSHIP`, `DUPLICATE_NATIVE_METHOD_ID`; warning `UNBOUND_NATIVE_METHOD` | `binding.ts` retains actual typed method/input/output references outside JSON. `compiler.ts` rejects forged/copied handles, wrong input/owner and duplicate ownership. Source links without a method remain declared; signatures are checked by TypeScript, not inferred from method names. |
| Ownership/identity | `MISSING_BUSINESS_IDENTITY`, `BUSINESS_IDENTITY_MISMATCH`, `DUPLICATE_BUSINESS_IDENTITY`, `METHOD_OWNERSHIP`, `STATE_OWNERSHIP`, `RULE_OWNERSHIP`, `MEMBERSHIP_OWNERSHIP`, `INVALID_CONSISTENCY_ROOT`, `IDENTITY_WRITE` | Entity identity is projected from native state; methods/effects belong to the exact owner instance; optional roots bind actual instances. |
| Operations/state | `QUERY_WRITE`, `UNDECLARED_EFFECT`, `DERIVED_STATE_WRITE`, `TRANSITION_OWNERSHIP`, `INVALID_TRANSITION`, `INVALID_INITIAL_STATE`, `INVALID_STATE`, `RULE_VIOLATION` | Queries cannot write domain state; lifecycle changes need exact owner/operation/outcome/from/to transition witnesses; inert comparisons check only witnessed values. |
| Process/dataflow | `INVALID_START`, `INVALID_BRANCH`, `MODULE_DEPENDENCY`, `MISSING_INPUT_SOURCE`, `OVERLAPPING_BINDINGS`, `MISSING_FIELD`, `MISSING_SOURCE_FIELD`, `PRIVATE_STATE_SOURCE`, `INCOMPATIBLE_VALUE`, `INCOMPATIBLE_VALUE_MEANING`, `WRONG_BRANCH`, `UNAVAILABLE_RESULT`, `PARAMETER_MISMATCH`, `INVALID_PROCESS_PATH` | All required input leaves need an explicit source; branch-qualified results must dominate consumers on all paths, and actual visit input must match resolved provenance. No private-state reads are inferred. |
| Explicit unsupported work | `UNSUPPORTED_CYCLE`, `UNSUPPORTED_PARALLEL`, `UNSUPPORTED_INPUT_SOURCE`, `UNSUPPORTED_RULE`, `UNSUPPORTED_DERIVATION` | No repeated step visits, loop values, fork/join semantics, arbitrary map functions, arbitrary predicates or formula callbacks. |
| Schema/meaning coverage | `UNSUPPORTED_SCHEMA_KEYWORD`, `UNSUPPORTED_SCHEMA_REFERENCE`, `SCHEMA_COMPATIBILITY_UNSUPPORTED`, `VALUE_MEANING_UNDECLARED` | Warning + artifact coverage; unknown reasoning is not a universal pass. A source without declared meaning/unit remains unproved. |
| Calls/expectations | `INVALID_COMPLETION`, `UNKNOWN_AS_SUCCESS`, `EFFECT_OUTCOME_MISMATCH`, `MISSING_RESULT`, `INVALID_RESULT`, `UNEXPECTED_RESULT`, `SCENARIO_EXPECTATION` | Received replies and local uncertainty are explicit. Owner effects survive caller uncertainty; expected values remain separate from actual witnesses. |

## Finite Schema semantics

`schema.ts` checks primitive type/integer/null, enum/const, numeric and string bounds/patterns, arrays/items/tuples, object required/properties/additionalProperties, finite local references, anyOf/oneOf/allOf and the empty-never `not` form. Finite depth bounds stop unsupported recursive expansion. Compatibility is deliberately conservative: direct type/bounds/enum and required object/array subsets are compared; complex unions/references and nonidentical pattern/multiple constraints remain unproved. This is not a replacement for explicit native parse during actual invocation, nor a complete JSON Schema validator.

Read-only derived projection currently supports only `subtract-sum` over explicit numeric source paths. It produces `Checkpoint.derived`, never writable domain state. No arbitrary JS expression or callback is stored in the artifact. General source inference, oRPC adaptation, event routing, parallel dataflow, arbitrary loops, recovery/idempotency, graph-to-source editing and theorem proving remain outside this implementation.

## Rule evidence and authored cases

A source-bound rule/source URL is a correspondence claim. Without `check`, assertions remain `declared`. A bounded comparison produces per-witness `passed`/`failed` with authored/observed basis; unavailable operands stay `unsupported`. Compiler inspection never executes business predicates. A native test proves only its actual selected cases; attach independent evidence through the owning acceptance record instead of changing every rule to a global tested state.

For operations, source-bound status additionally requires an actual `bindNativeMethod` handle with an exact input SchemaHandle and owner. Its method signature is checked against imported input/output schemas through `NoInfer`; a removed member or incompatible signature fails scoped TypeScript checking. The method itself stays in a WeakMap and is never invoked/serialized. Portable `nativeMethodId` and `nativeOutputSchemaId` describe that correspondence; outcome projections and arbitrary method internals are not thereby proved. Native metadata is not an execution trace or a runtime dispatch table.

`examples/order-stock.ts` provides success, stock refusal, committed/lost reply, wrong-order receipt, stale intent and quantity mismatch. `examples/cases.ts` provides four invalid authored inputs: wrong quantity as success, wrong order receipt as success, unknown as success and wrong branch. The separate technical consumer uses no product IDs or aggregate requirement. Independent acceptance authors and executes its own cases; source-authored case presence is not execution evidence.

For this Delivery, execution belongs only to [independent acceptance](../../../.controller/domain-daemon/tasks/flow-atlas-domain-acceptance.md). Its scoped type command is `node_modules/.bin/tsc -p tools/flow-atlas/domain/tsconfig.json`; its runner/tool path and actual attempts/results remain in that Task. Source owners repair concrete returned defects and do not edit acceptance tests or rerun checks.
