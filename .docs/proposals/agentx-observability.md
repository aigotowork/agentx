---
schema: proposal
description: Establish a coherent AgentX observability and management foundation before Web presentation.
status: accepted
models: [../model/README.md, ../observability/README.md]
deliveries: [../../.controller/observability/delivery.md, ../../.controller/observability/implementation.md]
---

# AgentX observability foundation

## Decision and need

The user selected a complete model/contract/engineering design and Delivery-led professional team. Management must make runtime structure, daemon health, requests, client/host relationships, effective configuration/environment and correlated events understandable from both client and daemon viewpoints. Data structures, domain ownership, engineering/context governance and state maintenance precede Web implementation.

The current [local daemon implementation](../../.controller/domain-daemon/delivery.md#accepted-local-outcome) provides typed operations, scoped singleton storage ownership, caller context, generation fences and domain events. The [discovery record](../../.controller/domain-daemon/feedback.md#management-observability-discovery) identifies the management gap. Existing status/event views do not constitute the proposed complete observation system.

## Selected change

Design a cohesive observation capability that composes existing domain facts and explicit new runtime evidence. Define data identities, provenance, fields/types/units, relationships, unknown/freshness meaning, state transitions, queries/events/errors, bounded capture and retention, reset/restart rules and consumer contracts. Establish a maintainable source/context map and a downstream implementation task system. The [current design entry](../observability/README.md) will link authoritative domain decisions; execution state stays in the [Delivery](../../.controller/observability/delivery.md).

Retain one executable, one live managed-storage owner per canonical scope, native Rust contract authority, explicit invocation context, observed process identity, domain-owned business invariants and accepted/committed/external-effect distinctions. Runtime observation does not grant new mutation or process-control authority. Web will be a consumer of these capabilities, with its actual transport boundary designed explicitly rather than bypassing native services or reading live SQLite.

## Scope and decision limits

Acceptance here selects the capability direction and design batch. It does not claim instrumentation, browser hosting, a particular event transport, persistent telemetry storage or implemented management actions. Owners must resolve proportional mechanism choices through concrete scenarios and source evidence. Cross-scope discovery, state mutation, disclosure and consumer hosting require explicit contracts in the design rather than inferred global visibility. Design integration must produce executable implementation work and retain any justified deferred capabilities separately.

The linked [implementation Delivery](../../.controller/observability/implementation.md) was the planned execution handoff from the completed design batch. The user has subsequently authorized its backend-first implementation and later browser consumption. The [accepted implementation outcome](../../.controller/observability/implementation.md#accepted-local-implementation-outcome) now closes the selected local scope; its native and consumer receipts own actual support and limits. Original Tasks and controller preserve completed assignments and execution history.
