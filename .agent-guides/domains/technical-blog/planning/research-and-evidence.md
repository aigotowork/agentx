---
schema: document
description: Support technical claims with primary sources, discriminating cases, controlled measurements and explicit limits
---

# Research and evidence

Use before making technical claims, depicting actual behavior or reporting comparisons and measurements. Investigate to support the reader's decision. Stop when consequential claims have adequate support and material unknowns are explicit.

## Preserve the kind of statement

| Statement | Required basis | Appropriate expression |
| --- | --- | --- |
| Current implementation | Native contract plus the relevant implementation and composition path | “This implementation…” with its actual source state |
| Platform/protocol guarantee | Primary specification or documentation for the relevant version and assumptions | “Under these conditions…” |
| Observed behavior | Actual inputs, procedure, environment and result | “This run observed…” with limited scope |
| Proposed design | Explicit assumptions, responsibilities and reasoned consequences | “This design would require…” |
| Simplified teaching model | Declared rules and omissions; consistent derivation | “In this model…” |

Do not infer production behavior from a teaching model, an implemented feature from a schema, authorization from a generated client or a current guarantee from historical acceptance. A target design can differ from the current system. The same discipline applies to titles, diagram labels, captions, subtitles, code output and demonstration scenes.

## Trace the claim to the actual owner

For repository claims, start with the owner's entry and native contract. Follow only the relevant adapter, policy, service, durable operation or external effect. Inspect composition when injected dependencies determine behavior. A folder name, import graph or test title does not establish runtime order or semantics.

For framework/platform behavior, use current primary references appropriate to the installed version. Record the source state and access date where behavior changes. A result in one compatibility layer or development runtime does not automatically describe another runtime or production.

Keep a compact ledger in the article Task:

| Claim | Basis and location | Needed challenge | Allowed conclusion |
| --- | --- | --- | --- |
| Selected adapters retain an admission rule | Operation policy, adapter paths and focused rejection checks | A valid caller bypassing the usual UI | The covered paths preserve the rule under the checked conditions |
| One durable operation protects capacity | Real predicate and storage semantics | Two requests competing for the same final slot | The inspected operation protects the stated invariant; no claim about external mail |
| A response cache separates identities | Key dimensions, admission and response policy | Same key candidates differing in identity, locale or representation | Safety within the inspected cache domain |
| An optimization lowers latency | Comparable workloads and actual measurements | Cold/warm behavior, variation and added costs | Improvement in the measured environment |

Publish the sources and assumptions needed to assess the conclusion. Keep internal progress logs in their existing owner.

## Choose evidence that can change the conclusion

Use the smallest meaningful counterexample distinguishing plausible approaches. For contention, two requests reading the last slot can be decisive. For authorization, a valid identity targeting an unauthorized object is stronger than malformed input alone. For failure recovery, inspect durable state after the consequential failure cut.

Before settling the conclusion, state what result would force it to change. If no observation or logical challenge could change it, distinguish a preference, value judgment or recommendation from an empirical technical claim.

Match evidence to the question:

- Pure decision cases can check a selector's boundaries; they do not establish database isolation.
- Real integration behavior can support a durable-write claim within its environment; a mocked repository call cannot.
- A simulator can reveal the consequences of its stated rules; animation cannot validate a production algorithm.
- An actual compiled-source/browser flow can check rendering and interaction; valid Markdown or one screenshot cannot establish hydration or all states.
- Rejecting an invalid example and accepting a valid one can check a detection rule; a green lint result cannot establish overall architecture quality.
- A meaningful end result can support a user-path claim; a click or HTTP success alone cannot establish the promised business effect.

Label designed but unrun procedures as proposed. Preserve failed observations and narrow the conclusion. If evidence contradicts the thesis, change the thesis or investigate its assumptions before polishing; do not rewrite expected outcomes to obtain a pass.

## Measurements and comparisons

Define metric, units, population, workload, environment, relevant versions, cache state and sampling method. Retain inputs and sufficient procedure for reproduction. Report sample count and spread where useful; one sample is not a distribution or a percentile.

Keep relevant conditions comparable across alternatives. Change one explanatory factor at a time unless the question explicitly studies a combined intervention. Report costs alongside benefits: memory, payload, writes, operational complexity, delayed freshness or other affected quantities. A faster edited recording is not a latency measurement.

Label synthetic data and simulated time near the result. A deterministic simulator does not measure network delay, platform latency or real contention. Use known units and consistent denominators; distinguish prediction, estimate and actual measurement. A visually polished chart cannot repair invalid samples or incomparable workloads.

## Sources, uncertainty and privacy

Prefer primary evidence over summaries. Trace important citations to the claim they support and verify the source actually says it. Several pages repeating one source are not independent confirmation. Do not invent citations, incidents, customer experience or measured history.

Keep uncertainty specific: which assumption is unresolved, how it limits the conclusion and what observation would resolve it. An unsupported central guarantee blocks publication; an honest, bounded proposed model can still be a useful deliverable.

Use isolated, publishable examples and appropriate source rights. Omit credentials, private tenant/customer data and identifying traces. Keep illustrative redactions distinct from original evidence. Educational intent grants no authority to send messages, charge accounts or mutate production resources; real-effect procedures require the actual task's authorization and owned resources.
