---
schema: task
status: active
description: Implement source-linked TS contract authoring, reusable schema bindings, metadata and process conformance lint.
links: [../flow-atlas-implementation.md, ../../../tools/flow-atlas/design/README.md]
---

# TS design authoring

Read root/tool/design AGENTS and the parent Delivery. This is an implementation assignment, not another research-only proposal. Extend the existing design only where needed, publish public skeleton/signatures early, then implement an independently usable bounded TS integration in tools/flow-atlas/authoring/. You own that new directory, design/contracts.ts + api.ts + README.md refinements, and this Task. Controller owns other routing/docs/tests/browser/generation. No viewer/build.py/shared manifest/lock edits.

## Required outcome

A TS consumer imports its existing runtime schemas and native method handles, declares domain modules/objects/ports plus metadata/docs, and defines a process using those exact handles. A reusable lint/compiler reports broken refs, duplicate ownership, forbidden module calls/dependency cycles, unreachable/dead-end process steps, missing handled outcomes and unbound declared steps. Implementation wrappers use stable process-step handles and record/validate observed traces so design/code drift is testable; do not claim arbitrary source inference or build a workflow scheduler.

Prefer a small cohesive API over a universal modeling framework. Reuse existing design types where sensible; schema-derived types replace duplicated shape sources when implementing a particular contract. Clearly label the larger target's unimplemented portions. Reuse installed Zod4.6.7 through a documented adapter; no install required. Any package dependency declaration belongs to this new module, never silently rely on an undeclared transitive dep for a distributable package. Generic schema adapter/core should stay decoupled from Zod. No oRPC claims without real pinned upstream use; preserve adapter boundary for it.

Metadata: stable identity, summary/responsibility, tags, source links and design document refs; namespaced extension values inert and portable. Runtime handlers must not be invoked by inspect/export/lint; a wrapper invokes only at explicit consumer runtime request. Conformance distinguishes failed/unknown/no response from success and documents coverage limits.

Provide an actual order/inventory TS example that imports schemas from a separate native-contract source (not copied into visualization), uses module/port/process bindings and emits a portable inspector artifact. Include authored case matrix in this Task and callable examples. Native Node22 strip-types is available through retained task helper, system Node24 exists; no Node/tsc/tests/build/browser run by you. Controller executes after coherent handoff. You can statically inspect source. No new subagents/services/product or environment changes.

Publish usable API shape and exact example invocation via Paseo promptly. Final release includes all owned paths, limitations and no remaining source claims.

## Working model and selected structure

Implementation worker is the delegated TS authoring specialist in the controller's current Paseo session. Ownership remains exactly the paths above; no nested agents or execution. Public inspector skeleton was published to controller on 2026-11-10 at `authoring/contracts.ts::InspectorArtifact` (`flow-atlas/inspector-1`).

| Layer | Responsibility / contract |
| --- | --- |
| Native consumer contracts | Existing runtime schemas and actual method functions; never copied into visualization |
| Handles | Exact shared identities for modules, objects, operations, process steps and bindings; native method storage stays outside portable data |
| Compiler | Check ownership, refs, module policy, process reachability/outcome coverage and bound steps; export plain JSON with diagnostics |
| Conformance | Explicit sequential invocation wrappers and offline observed-path validation; no scheduler, retries, arbitrary source inference or exhaustive proof |
| Adapter | Zod 4.6.7 conversion to JSON Schema; generic core imports no Zod |
| Inspector consumer | Controller-owned offline view consuming the published artifact |

The existing target model supplies Identity/metadata, Domain, Module, object kinds, JSON Schema and diagnostic semantics. This bounded slice does not pretend to implement the full Definition/Scenario/event/state/layout compiler. The operation result classifier belongs to explicit consumer runtime only. Inspect/lint do not parse inputs or invoke handlers/classifiers. Unknown/no response/failed observations are distinct from returned success.

Rejected alternatives: source inference cannot recover domain intent; a workflow engine exceeds the outcome; a second schema DSL would duplicate native contracts. Generic adapter plus explicitly pinned optional Zod peer avoids relying on an undeclared transitive dependency.

Friction recorded: root dependencies contain Zod transitively, so the new private module declares its own optional exact peer rather than editing the shared manifest/lock. Node execution is assigned exclusively to the controller; specialist evidence is source inspection only.

Controller clarified the dependency-cycle seam: module dependency cycles default to a quality warning; `policy.dependencyCycles` can explicitly choose error/allow. Broken references, duplicate ownership and undeclared direct module calls remain structural errors. Parent-domain/module containment cycles remain invalid structure. No purity is inferred from command/query labels.

## Authored acceptance cases

These are authored inputs and expected behavior, not specialist execution evidence. Controller owns type/runtime/consumer checks. Callable inputs are `authoring/examples/cases.ts::{designCases,traceCases,schemaCases}`; actual native invocation example is `order-inventory.ts::observeOrder`.

| Case | Expected observation / diagnostic |
| --- | --- |
| Imported native request/result and unchanged method handles | Two-module design compiles, metadata/docs/extensions and actual JSON Schema fields survive export |
| Compile-only invalid handler input/output or invocation payload | `type-cases.ts` has expected type errors; NoInfer prevents handler/input arguments from widening native schema types |
| Inspect/export/lint | Native call count unchanged; classifier and parse not invoked; JSON has no functions/schema instances |
| Native successful order | Explicit prepare + reserve invokes actual methods once each, stock decreases natively, valid completed trace |
| Insufficient inventory / missing SKU | Returned rejection selects rejected terminal; complete trace is not business success |
| Lost connection / explicit no-response observation | Separate unknown / no-response status selects unknown terminal; no fabricated success |
| Native method throws / input-output parsing fails | Failed observation retained, original error rethrown; declared failure edge or explicit unresolved diagnostic |
| Pending promise / concurrent invoke or record | Pending check reports TRACE_PENDING; overlapping observations refused |
| Removed or replaced operation / wrong native ownership | BROKEN_REF / WRONG_OPERATION_BINDING / DUPLICATE_OPERATION_OWNERSHIP |
| Missing implementation binding / orphan binding | UNBOUND_STEP / ORPHAN_BINDING |
| Undeclared cross-module call | FORBIDDEN_MODULE_CALL error |
| Dependency cycle | DEPENDENCY_CYCLE warning by default, error in strict policy, absent when explicitly allowed |
| Unreachable or no-end path / removed result branch | UNREACHABLE_STEP / DEAD_END_STEP / UNHANDLED_OUTCOME |
| Trace skips or reorders step / operation swapped | TRACE_ORDER / TRACE_OPERATION |
| Unknown reported as returned success / missing outcome / undeclared branch | TRACE_STATUS / TRACE_OUTCOME / TRACE_BRANCH |
| Missing final step / sequence gap | TRACE_INCOMPLETE / TRACE_SEQUENCE |
| Callback schemas in either input/output conversion | UNSUPPORTED_SCHEMA and callback counter unchanged for transform/refinement/default/catch/lazy/shape getter |
| Date / BigInt and other unsupported schema nodes | Explicit UNSUPPORTED_SCHEMA; no permissive fallback |
| Unsafe document URI / missing document identity | UNSAFE_DOCUMENT_URI / INVALID_DOCUMENT_REF; no document reads or remote fetch |
| Nonportable metadata | NONPORTABLE_METADATA; functions/cycles/Date/nonfinite numbers/accessors cannot enter portable output |

## Callable handoff surface

Public APIs are `authoring/index.ts`; exact artifact structure is `authoring/contracts.ts::InspectorArtifact` (`flow-atlas/inspector-1`). The generic schema context and metadata/module-policy vocabulary reuse target contracts; business payload types are inferred from imported runtime schemas. Core execution/storage functions are not exported from the root entry.

Controller invocations after source release:

```sh
node_modules/.bin/tsc -p tools/flow-atlas/authoring/tsconfig.json
node_modules/.bin/tsc -p tools/flow-atlas/design/tsconfig.json
node --experimental-strip-types tools/flow-atlas/authoring/examples/order-inventory.ts
node --experimental-strip-types tools/flow-atlas/authoring/examples/order-inventory.ts /tmp/order-inspector.json
```

The optional destination is exclusive-create and must not already exist. Example stdout contains portable JSON only. No Node/tsc/tests/build/browser/service or environment mutation has been performed by this specialist; independent acceptance remains controller-owned.

## Source release to controller

All assigned source implementation is filled; public source is released for independent consumption and acceptance. No owned file remains claimed for an in-progress edit. The controller owns integration/tests/browser/generation and final acceptance; any concrete defect is returned to this Task's owner for a bounded fix.

| Released paths | Content |
| --- | --- |
| `tools/flow-atlas/authoring/{contracts,index,handles,compiler,conformance,portable}.ts` | Shared-handle authoring, JSON inspector projection, lint/compile and explicit observed-path checks |
| `tools/flow-atlas/authoring/adapters/zod.ts` | Installed/pinned 4.6.7 adapter, explicit schema rejection, preflight before conversion callbacks |
| `tools/flow-atlas/authoring/examples/{native-contracts,order-inventory,cases}.ts` | Separate actual business schemas/methods, complete composition/export, authored structural/schema/trace cases |
| `tools/flow-atlas/authoring/{type-cases.ts,tsconfig.json,package.json,AGENTS.md,README.md}` | Negative type cases, local exact peer, source-module entry and usage/limitations |
| `tools/flow-atlas/design/{contracts.ts,api.ts,README.md}` | Shared metadata/dependency-policy/schema-context vocabulary and target/current routing refinement |
| This Task | Model choices, friction, authored matrix and exact receiving responsibility |

Specialist verification was static source inspection against native Zod declarations/source and existing target contracts. No runtime, compiler or test result is claimed. No service, listener, child agent or temporary resource was opened. Explicitly unsupported: arbitrary TS source inference, uninstrumented-call proof, automatic handler scheduling/retries, parallel paths, full Definition/Scenario/state/event/layout compilation, generic SDK/oRPC import, watch/live observation, persistence or package publication. Schema adapters and native contract-only TS inputs are trusted build-time code, not a security sandbox.

## Controller-reported adapter correction

Independent controller execution on Node 22.23.3 found the native success design blocked by four UNSUPPORTED_SCHEMA diagnostics: `Hidden/accessor member at $.~standard` ([retained stderr](/tmp/flow-atlas-implementation-kfc9sslh/authoring-stderr.log)). Controller also reports both TypeScript configurations passed. These are receiving-controller observations, not specialist runs.

Pinned source `node_modules/zod/v4/core/to-json-schema.js::finalize` first JSON-round-trips the emitted schema and then attaches a nonenumerable, nonwritable, nonconfigurable root `~standard` data descriptor carrying Standard Schema functions. This integration member is not portable JSON Schema. The adapter now copies the output by descriptors, excludes only that exact root descriptor shape, then applies unchanged strict portableCopy. It never reads the member or invokes any of its functions; nested schema properties named `~standard` remain ordinary JSON and are preserved. All other hidden/accessor/nonportable members still fail, and callback-bearing Schema preflight remains unchanged.

Only `authoring/adapters/zod.ts` and this Task were reopened for the bounded overlay and are now released again. Controller should rerun the failed native authoring acceptance and check nested literal `~standard` fields survive while unsupported/callback Schema cases retain diagnostics and a zero callback count. No specialist Node/tsc/tests/build/browser execution was performed for this correction; all other source stays released.

## Independent acceptance received

The controller completed affected executable/browser acceptance and integrated this source; see the singular [accepted result](../flow-atlas-implementation.md#accepted-local-slice). Original author-only evidence above remains historical, and no source/resource claim is active.
