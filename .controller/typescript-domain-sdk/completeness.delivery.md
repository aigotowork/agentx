---
schema: delivery
status: completed
description: Complete the AgentX Management domain object SDK and adopt it in the Space business flows.
stages:
  object_and_native:
    description: Implement cohesive TypeScript contracts and any missing native facts needed to make their promises true.
    tasks: ["./tasks/contract-completeness.task.md", "./tasks/native-completeness.task.md"]
  composition:
    description: Integrate the object contracts into Space, publish their maintained model, and verify bounded full journeys.
    tasks: ["./tasks/space-completeness.task.md", "./tasks/canvas-completeness.task.md"]
---

# Complete the AgentX Domain SDK

The [selected target model](../../.docs/proposals/typescript-domain-sdk.md#complete-the-object-model) owns responsibilities and caller promises. The [baseline audit](../../.docs/model/typescript-domain-sdk-review.md) distinguishes missing object wrappers from missing server behavior. The goal is a complete object interface for Application/Management capabilities in scope. Space has one business path across visual styles. The generated HTTP SDK remains separately usable and native Service remains authoritative.

| Responsibility | Owner / boundary |
| --- | --- |
| Controller and integration | kula owns dependencies, change coordination, consumer/diagram acceptance and concentrated verification. |
| TypeScript objects | One domain-contracts owner designs and implements the complete package surface and adapters over existing native capabilities. Release the first usable contract (Transcript, launch relations, typed original receipt recovery) to Space, then continue through lifecycle and independent scope capabilities. Own package user documentation and the current SDK model. |
| Native promises | One domain-contracts owner handles member-filtered paging and precise launch outcome; this is separate from the TypeScript object adapter because the service owns filtering, identity and journal facts. Native owner writes native source and upstream contract docs. Coordinate the generated package handoff with the SDK owner; only one writer generates artifacts. |
| Space and bounded acceptance | Consume the object contracts while preserving exact navigation, restored journal entries, user-visible states and style continuity. An independent consumer exercises the SDK package across the real app flow; a finite fixture distinguishes shape inference from verified native support. The existing live listener is separate. |
| Contract Canvas projection | Maintain navigable contracts and source correspondence in the AgentX consumer's data, separate from the reusable renderer. |

Use one first handoff for Transcript, provenance and original recovery, a second for lifecycle plus the remaining supported capabilities, and a final composition with the two native prerequisites. These are integration points, not separate micro-assignments. Preserve independent Workbench/Observation/Inspector contracts, installed binaries, current services and unrelated changes. No load/large-data tests, production rollout, release builds, parallel native compilations, or test production Agents. Use tiny fixtures; Controller coordinates any single low-concurrency native check needed after source composes.

Done when the package declarations, modeled semantics, typed adapters and actual Space flow agree; the old public HTTP and SDK paths remain compatible; the native-backed member and launch promises are evidenced; the model and Contract Canvas describe what actually shipped in source. Link evidence and unverified bounds at the owning Tasks.

AgentX confirmed `fele` (domain-contracts, batch `20574a1e`) ready for the package Task and `hozira` (domain-contracts, batch `72c899ae`) ready for the native Task. Each owns its bound Task and has no further delegation. Next dependency: first usable package contracts and first native shapes; the Controller starts Space source integration from contract handoff and coordinates generated artifact ownership after the native shape is known.

The [native source handoff](tasks/native-completeness.task.md#source-handoff-and-evidence) built successfully as a debug candidate under a single-job CPU/memory-limited scope. Controller emitted the [actual Management OpenAPI](../../.docs/api-panorama/openapi/management.json), ran the existing generator in a disposable directory, and transferred the checked Management outputs. Diff: one added route, optional fields in Descriptor and LaunchOutcome, and no changed existing paths; Workbench and Observation inputs and generated outputs remained byte-identical. Handoff delivered to `fele` for final domain binding. The debug artifact predates the generated binding in its embedded browser assets; validate final Space source through the isolated host rather than claiming embedded adoption.

`fele` delivered the final emitted package with [finite public and compatibility evidence](tasks/contract-completeness.task.md). Controller integrated the [complete Space source path](tasks/space-completeness.task.md) and exercised the current browser source plus final public SDK through a private compatible native daemon. `ludevo` (domain-contracts, batch `66d44ad4`) owns the [Contract Canvas update](tasks/canvas-completeness.task.md) against that canonical source. No installed or live change was made.

## Accepted local outcome

The supported Management object surface is integrated through the actual native → emitted OpenAPI → generated SDK → Domain SDK → Space chain. Exact target addressing and recovery use the original scope and request ID across operation kinds; native member pages and bounded legacy projections preserve separate coverage. An exact launch identity is usable when the original native receipt contains it. The [current written model](../../.docs/model/typescript-domain-sdk.md), [public package examples](../../packages/agentx-client/README.md) and [offline Contract Canvas](../../.docs/object-atlas-v2/index.html) describe those contracts. The [Canvas handoff](tasks/canvas-completeness.task.md#completed-projection-and-evidence) passed source correspondence and all 54 directed navigation routes.

Accepted evidence: 22 finite package and compatibility cases, 22 focused Space controller cases, passing Space type check, successful bounded native debug build and isolated actual OpenAPI generation, followed by the [real SDK and five browser journeys](tasks/space-completeness.task.md#scope-and-lifecycle-composition) with one private Agent fixture and two explicit browser POSTs (send and Transcript). Private test resources were cleaned up. The dedicated Rust unit cases and full native suite were not executed. No load or large-data work, release build, registry package publication or live deployment is included.
