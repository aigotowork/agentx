---
schema: scenario
description: Select bounded contract projections and evolution checks while retaining native semantics and real implementation obligations.
tags: [contracts, modeling, generation]
---

# Use contract tools at their supported boundary

Begin with the original [public contract](../../scenarios/engineering-design.md#public-contracts-and-domain-meaning). Use the [API/SDK method](../../scenarios/api-contracts-and-sdks.md) when the selected boundary needs a protocol definition, client generation or API compatibility check; a same-process module can expose its native types directly. CLI and product interactions retain their [native forms and mappings](../../scenarios/collaborative-modeling.md#carry-a-promise-across-interfaces), without conversion to OpenAPI or a checker format as a prerequisite. Native types, domain errors, effect and recovery semantics own meaning. A generated protocol or SDK is a projection with explicit losses, not permission to shrink product requirements.

Choose an existing generator/provider toolchain before custom machinery. Read the installed component's Profile, declarations, examples and dependencies. A finite OpenAPI JSON generator can reject security, streams or parameter shapes that the full specification permits; a JSON Schema evolution analyzer may accept only a bounded vocabulary. Unsupported must remain unsupported, not be dropped or treated as compatible. An oRPC projection supports only its selected native schema/runtime/version, not every TypeScript API.

Record source declaration/version → projection/profile → generated artifact identity → consumer and handwritten obligations. Keep actual wire serialization, synchronous/asynchronous behavior, optional/null semantics, errors, resource scope, cancellation and result-unknown handling explicit. A local call changed to a Promise remote client may change promises even if nominal field types look identical.

For evolution, identify the direction of compatibility and actual old/new consumers, producers and stored or in-flight work. Compare accepted inputs, promised outputs/errors, permissions, effects, receipt lifetime and recovery. Use [changed-promise analysis](../../scenarios/collaborative-modeling.md#compare-changed-promises) when signatures remain unchanged. A structural diff cannot discover all consumers or prove behavioral compatibility.

Run the finite analyzer with explicit contracts and relationships. Preserve source freshness separately from the historical result. A matching basis proves matching bytes only. Keep unsupported relationships, incomplete checks, counterexamples and manual decisions visible; do not convert “no contradiction found in this vocabulary” into universal compatibility.

Generate reproducibly, inspect package exports and runtime dependency closure, then exercise the intended real consumer boundary. Compile-only checks establish type usability; a codec test establishes its codec; actual HTTP/storage/process tests establish their exercised effects. A sample User Service or SDK tarball illustrates one composition, not a reusable identity subsystem or a released generator.

Finish with the actual supported callable artifact, instructions and remaining implementation evidence. Preserve old fixed releases and data. The release owner controls version/publication; model/tool success alone cannot close consumer integration.
