---
schema: task
status: completed
owner: inspector-ui-beta
description: Transplant the complete Inspector UI and core presentation behavior into an isolated AgentX Web module.
---

# Frontend migration assignment

Read [Delivery](delivery.md), source `AGENTS.md`, `web/AGENTS.md`, `uiux/AGENTS.md`, source `.docs/domain-model.md` and `.docs/design-system.md` as relevant, and target local coder/interface guidance. Source remains read-only. You own the cohesive UI migration into `web/inspector/**`, plus `frontend-implementation.md` beside this task. No other writes, no subagents, no real API/native changes, no live services or existing previews affected. You may use a local package/dependencies; do not change root manifests without reporting the concrete need.

## Outcome

Preserve actual source modules, recognizable visual composition, all eight principal tabs and supporting routes, original/pixel skins, light/dark appearance, charts, localized copy, overlays, motion/preferences, and the shared design system/resources the product consumes. Inspect source reachability before choosing the complete finite transplant. Keep the alternate recovery experience and component preview where source supports them. Do not replace the product with a smaller newly invented dashboard. Remove executable Paseo SDK/packages/network/storage assumptions and old product-facing terminology; factual license/provenance can retain names.

Use an AgentX-oriented host/provider boundary with typed finite operations and a runnable in-memory/local mock default. Supply representative correlated data so chart/list/detail navigation, comparison, event views, insights/jobs, recovery/settings and applicable actions can be exercised. Existing domain algorithms that are portable may be retained; do not claim mock counters or recommendations as native AgentX facts. Real mutations must not occur. Local simulated effects must be explicit and coherent, including idempotence/unknown states where relevant. No synthetic success pretending to reach real agents. Keep source-health/unknown, partial history and delivery-vs-execution semantics.

## Method and handoff

Before substantial implementation, record a proportionate structure/model and source inventory in `frontend-implementation.md`: source surfaces/resources retained, host adapters replaced, directory and dependency ownership, actual local provider contract, scenario coverage and intended tests. Materialize the structure before filling behavior. Preserve MIT LICENSE/NOTICE. Add module README and AGENTS with run commands, design entry, migration inventory, deferred features and future API seam. Every exported `.tsx` component/module needs the project's `@aiPurpose`/`@aiUsage` JSDoc contract.

You own implementation and routine repairs. Run static/type checks needed to develop; one separately assigned executor will own final build/browser execution, so do not duplicate those without a concrete runtime uncertainty. Inform lube when buildable source and exact commands exist, then remain responsible for defects from that finite execution. Browser development uncertainty probes are allowed if necessary; record them distinctly. Use a non-default isolated port and clean disposable resources you create. Finish only after final defects assigned back to you are repaired and your scope is handed off.

The parallel data assessment will arrive at `.docs/inspector/data-foundation.md`; it should inform future mapping, but does not block preserving source UI/mock supply. Do not wait for production API design. Record source ambiguity and intentionally deferred provider capabilities in your implementation note, not hidden compatibility hacks.

## Accepted result

Lube accepts the source/core transplant and ordinary verification repairs under the [completed Delivery](delivery.md#accepted-local-outcome). The [implementation record](frontend-implementation.md) and module entries identify preserved source, local provider, algorithms and future seams; [independent execution](verification-result.md) establishes the documented finite offline behavior. Source and repair ownership are handed off; no further implementation or broad checks are required for this phase.
