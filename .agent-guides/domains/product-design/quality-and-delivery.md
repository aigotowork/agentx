---
schema: scenario
description: Connect quality promises, trust boundaries, operating signals and implementation evidence to a complete delivery.
tags: [quality, delivery, operations]
---

# Carry promises into engineering and operation

Use when performance, correctness, accessibility, resource lifetime, security or recovery can invalidate the selected product outcome. Work from real constraints and failure consequences; a small local tool needs only the relevant boundaries. The [engineering contract method](../../scenarios/engineering-design.md) owns architecture and the [release method](../../scenarios/release-and-recovery.md) owns publication/deployment.

Write a quality scenario as source/stimulus + environment + affected object + required response + measurement or invariant, with owner and evidence. Set thresholds from actual tolerance, a baseline or a declared requirement. “Fast and reliable” cannot guide implementation; an invented p95 target does not become a requirement because it has a number.

| Concern | Decision and evidence |
| --- | --- |
| Latency and capacity | Separate input feedback from authoritative completion; name hardware/network/data/load/window and failed-request treatment |
| Concurrency and correctness | Find the actual atomicity or revision owner; prove the invariant under real contention where claimed |
| Accessibility | Follow the whole selected task including focus, naming, changes and recovery in the promised environment |
| Replaceable implementation | Exercise the public guarantee through the selected adapter; an interface alone does not prove substitutability |
| Recovery | Name acceptable loss/time only with basis, backup consistency and actual restore verification |

## Preserve boundaries in the handoff

Connect each consequential scenario criterion to the operation that can uphold it, its original facts/authority, the caller's observable result and the evidence still needed. Generated types or a compiled client can unblock integration while persistence, authorization, atomicity or external effects remain unproved. Return infeasible guarantees and unknown outcomes to the product/experience owner.

Where the product handles untrusted input or data, map the applicable trust/data flow: who may supply what, who authenticates identity, who authorizes each resource/action, and where an effect becomes durable. UI visibility and CORS do not enforce resource authorization. Keep recovery keys, deduplication and receipt lifetimes with the responsible domain; avoid unbounded retry amplification.

For collected data, identify purpose, necessary fields, readers, retention/deletion, backups and telemetry copies. Avoid copying credentials or whole payloads into logs. Regulatory applicability and periods require real context, not a universal checklist. Select the relevant [OWASP ASVS](https://owasp.org/projects/asvs) or [WCAG](https://www.w3.org/TR/WCAG22/) requirements and version when applicable; one component or automated scan does not prove complete conformance.

## Observe decisions, not merely events

Choose signals by the action they enable. Logs explain discrete events; traces relate paths; metrics expose trends. Authoritative business state still owns whether the effect happened. A span ending or an HTTP 200 is not a commit receipt. Keep original operation/scope identity in bounded records; avoid unbounded identity labels in metrics.

For a service symptom, distinguish normal domain rejection, technical failure and unknown outcome. A useful signal can say who is affected, which version/target, what threshold or invariant is violated and what its owner can do next. Trace sampling absence does not establish no execution. Define denominator, time window and exclusions before a reliability objective; do not improve a metric by hiding rejected or failed work.

## Verify at the promised boundary

Keep contract, implementation integration, host experience and operating evidence distinct. Choose a vertical scenario and its decisive failure, compare real effects and visible results, retain limitations and close owned resources. Update only impacted evidence when a local change leaves other guarantees intact. Mock success, packaging success and production readiness answer different questions.

Hand the next owner fixed source/artifact identity, current contracts and chosen scope, satisfied/unsatisfied obligations, actual run references and resource ownership. After release, compare real observations to the original outcome and constraints; create a bounded correction where they diverge. Do not maintain delivery status in a second design ledger.

Further sources: [arc42 quality scenarios](https://docs.arc42.org/section-10/), [OpenTelemetry signals](https://opentelemetry.io/docs/concepts/signals/), [SRE monitoring](https://sre.google/sre-book/monitoring-distributed-systems/). These support method selection; actual implementation and operating claims require consumer evidence.
